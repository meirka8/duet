use crate::conflict::ConflictPolicy;
use crate::event::JobEvent;
use crate::executor::Executor;
use crate::job::{Job, JobId, JobProgress, JobStatus};
use crate::plan::Plan;
use duet_types::{VfsError, VfsResult};
use duet_vfs::FileSystem;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Queue manager for handling multiple asynchronous file operations.
#[derive(Debug)]
pub struct QueueManager {
    jobs: Arc<Mutex<VecDeque<Job>>>,
    active_job_id: Arc<Mutex<Option<JobId>>>,
    event_tx: broadcast::Sender<JobEvent>,
    journal_dir: Option<PathBuf>,
    pause_all_flag: Arc<AtomicBool>,
    executor: Arc<Executor>,
    next_id: Arc<Mutex<u64>>,
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new(None)
    }
}

impl QueueManager {
    pub fn new(journal_dir: Option<PathBuf>) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            jobs: Arc::new(Mutex::new(VecDeque::new())),
            active_job_id: Arc::new(Mutex::new(None)),
            event_tx,
            journal_dir,
            pause_all_flag: Arc::new(AtomicBool::new(false)),
            executor: Arc::new(Executor::new()),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Subscribe to operation job events.
    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.event_tx.subscribe()
    }

    /// Enqueue a new operation job into the manager.
    pub fn enqueue(&self, plan: Plan, conflict_policy: ConflictPolicy) -> VfsResult<JobId> {
        let mut id_guard = self.next_id.lock().unwrap();
        let job_id = JobId(*id_guard);
        *id_guard += 1;

        let mut job = Job::new(job_id, plan, conflict_policy);
        if let Some(ref jdir) = self.journal_dir {
            let jpath = jdir.join(format!("{job_id}.journal"));
            job = job.with_journal(jpath)?;
        }

        let mut jobs = self.jobs.lock().unwrap();
        jobs.push_back(job);

        let _ = self.event_tx.send(JobEvent::Created { job_id });
        Ok(job_id)
    }

    /// Start processing queued jobs sequentially on the provided VFS.
    pub fn process_queue(&self, fs: Arc<dyn FileSystem>) {
        let jobs = Arc::clone(&self.jobs);
        let active_job_id = Arc::clone(&self.active_job_id);
        let event_tx = self.event_tx.clone();
        let pause_all = Arc::clone(&self.pause_all_flag);
        let executor = Arc::clone(&self.executor);

        tokio::spawn(async move {
            loop {
                let mut current_job = None;
                {
                    let mut queue = jobs.lock().unwrap();
                    if let Some(pos) = queue
                        .iter()
                        .position(|j| j.status == JobStatus::Pending)
                    {
                        current_job = queue.remove(pos);
                    }
                }

                let mut job = match current_job {
                    Some(j) => j,
                    None => {
                        // Queue empty or all processed
                        let mut active = active_job_id.lock().unwrap();
                        *active = None;
                        break;
                    }
                };

                {
                    let mut active = active_job_id.lock().unwrap();
                    *active = Some(job.id);
                }

                let cancel_signal = Arc::new(AtomicBool::new(false));
                let pause_signal = Arc::clone(&pause_all);

                let _res = executor
                    .execute_job(
                        &mut job,
                        fs.as_ref(),
                        cancel_signal,
                        pause_signal,
                        Some(event_tx.clone()),
                    )
                    .await;

                // Re-insert completed / failed job to jobs store for status queries
                let mut queue = jobs.lock().unwrap();
                queue.push_back(job);
            }
        });
    }

    /// Pause specific job.
    pub fn pause_job(&self, job_id: JobId) -> VfsResult<()> {
        let mut queue = self.jobs.lock().unwrap();
        if let Some(job) = queue.iter_mut().find(|j| j.id == job_id) {
            job.pause();
            let _ = self.event_tx.send(JobEvent::Paused { job_id });
            Ok(())
        } else {
            Err(VfsError::NotFound(format!("Job {job_id} not found")))
        }
    }

    /// Resume specific job.
    pub fn resume_job(&self, job_id: JobId) -> VfsResult<()> {
        let mut queue = self.jobs.lock().unwrap();
        if let Some(job) = queue.iter_mut().find(|j| j.id == job_id) {
            job.start();
            let _ = self.event_tx.send(JobEvent::Resumed { job_id });
            Ok(())
        } else {
            Err(VfsError::NotFound(format!("Job {job_id} not found")))
        }
    }

    /// Cancel specific job.
    pub fn cancel_job(&self, job_id: JobId) -> VfsResult<()> {
        let mut queue = self.jobs.lock().unwrap();
        if let Some(job) = queue.iter_mut().find(|j| j.id == job_id) {
            job.cancel();
            let _ = self.event_tx.send(JobEvent::Cancelled { job_id });
            Ok(())
        } else {
            Err(VfsError::NotFound(format!("Job {job_id} not found")))
        }
    }

    /// Pause entire queue.
    pub fn pause_all(&self) {
        self.pause_all_flag.store(true, Ordering::SeqCst);
    }

    /// Resume entire queue.
    pub fn resume_all(&self) {
        self.pause_all_flag.store(false, Ordering::SeqCst);
    }

    /// Reorder jobs in queue: moves job from `from_index` to `to_index`.
    pub fn reorder_jobs(&self, from_index: usize, to_index: usize) -> VfsResult<()> {
        let mut queue = self.jobs.lock().unwrap();
        if from_index < queue.len() && to_index < queue.len() {
            let job = queue.remove(from_index).unwrap();
            queue.insert(to_index, job);
            Ok(())
        } else {
            Err(VfsError::InvalidPath("Invalid job index".into()))
        }
    }

    /// Calculate aggregate progress across all jobs in queue.
    pub fn aggregate_progress(&self) -> JobProgress {
        let queue = self.jobs.lock().unwrap();
        let mut total_bytes = 0u64;
        let mut bytes_transferred = 0u64;
        let mut total_files = 0u64;
        let mut files_processed = 0u64;

        for job in queue.iter() {
            total_bytes += job.progress.total_bytes;
            bytes_transferred += job.progress.bytes_transferred;
            total_files += job.progress.total_files;
            files_processed += job.progress.files_processed;
        }

        JobProgress {
            bytes_transferred,
            total_bytes,
            files_processed,
            total_files,
            eta_seconds: None,
            current_file: None,
        }
    }

    /// Fetch job by ID.
    pub fn get_job_status(&self, job_id: JobId) -> Option<JobStatus> {
        let queue = self.jobs.lock().unwrap();
        queue.iter().find(|j| j.id == job_id).map(|j| j.status.clone())
    }
}
