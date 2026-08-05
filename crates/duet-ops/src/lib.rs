pub mod conflict;
pub mod event;
pub mod executor;
pub mod job;
pub mod journal;
pub mod plan;
pub mod planner;
pub mod progress;
pub mod queue;
pub mod step;
pub mod strategy;

pub use conflict::{resolve_conflict, ConflictDecision, ConflictPolicy};
pub use event::JobEvent;
pub use executor::{compute_blake3_checksum, Executor};
pub use job::{Job, JobId, JobProgress, JobStatus};
pub use journal::{Journal, JournalRecord};
pub use plan::{CopyPlan, DeletePlan, MovePlan, Plan, SyncPlan};
pub use planner::Planner;
pub use progress::ProgressTracker;
pub use queue::QueueManager;
pub use step::Step;
pub use strategy::{execute_copy_strategy_ladder, CopyStrategyUsed};

#[cfg(test)]
mod tests {
    use super::*;
    use duet_types::VPath;
    use duet_vfs::LocalFs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_copy_3_file_directory_plan_and_execution() {
        let temp = tempdir().expect("tempdir failed");
        let src_dir = temp.path().join("source_dir");
        let dst_dir = temp.path().join("dest_dir");

        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        std::fs::write(src_dir.join("file1.txt"), "hello file 1").unwrap();
        std::fs::write(src_dir.join("file2.txt"), "hello file 2 data").unwrap();
        std::fs::write(src_dir.join("file3.txt"), "hello file 3 extra data").unwrap();

        let src_vpath = VPath::new_local(src_dir.to_str().unwrap());
        let dst_vpath = VPath::new_local(dst_dir.to_str().unwrap());

        let local_fs = LocalFs::new();
        let planner = Planner::new().with_verification(true);

        let plan = planner
            .build_copy_plan(&[src_vpath], &dst_vpath, &local_fs, None)
            .await
            .expect("build_copy_plan failed");

        assert_eq!(plan.file_count, 3);
        assert!(plan.total_bytes > 0);

        let mut job = Job::new(JobId(1), Plan::Copy(plan), ConflictPolicy::OverwriteAll);
        let executor = Executor::new();

        let cancel_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pause_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        executor
            .execute_job(&mut job, &local_fs, cancel_signal, pause_signal, None)
            .await
            .expect("execute_job failed");

        assert_eq!(job.status, JobStatus::Completed);
        assert!(dst_dir.join("source_dir").join("file1.txt").exists());
        assert!(dst_dir.join("source_dir").join("file2.txt").exists());
        assert!(dst_dir.join("source_dir").join("file3.txt").exists());
    }

    #[test]
    fn test_journal_append_and_recovery() {
        let temp_dir = tempdir().expect("tempdir failed");
        let journal_path = temp_dir.path().join("test_job.journal");

        let mut journal = Journal::open(&journal_path).expect("open journal failed");

        let job_id = JobId(42);
        let plan = Plan::Copy(CopyPlan {
            file_count: 1,
            total_bytes: 100,
            steps: vec![Step::CopyFile {
                src: VPath::new_local("/a"),
                dst: VPath::new_local("/b"),
                size: 100,
            }],
        });

        journal
            .append(&JournalRecord::JobInit {
                job_id,
                plan: plan.clone(),
                conflict_policy: ConflictPolicy::OverwriteAll,
            })
            .expect("append failed");

        journal
            .append(&JournalRecord::StepStarted {
                step_index: 0,
                step: plan.steps()[0].clone(),
            })
            .expect("append step started failed");

        journal
            .append(&JournalRecord::StepCompleted {
                step_index: 0,
                bytes_processed: 100,
            })
            .expect("append step completed failed");

        drop(journal);

        let records = Journal::read_all(&journal_path).expect("read_all failed");
        assert_eq!(records.len(), 3);

        let recovery = Journal::recover(&journal_path).expect("recover failed");
        assert!(recovery.is_some());
        let (recovered_id, recovered_plan, step_idx) = recovery.unwrap();
        assert_eq!(recovered_id, job_id);
        assert_eq!(recovered_plan, plan);
        assert_eq!(step_idx, 1);
    }

    #[tokio::test]
    async fn test_move_and_delete_plan_and_execution() {
        let temp = tempdir().expect("tempdir failed");
        let src_file = temp.path().join("move_me.txt");
        let dst_dir = temp.path().join("moved_dir");
        std::fs::write(&src_file, "data to move").unwrap();

        let local_fs = LocalFs::new();
        let planner = Planner::new();

        let src_vpath = VPath::new_local(src_file.to_str().unwrap());
        let dst_vpath = VPath::new_local(dst_dir.to_str().unwrap());

        let move_plan = planner
            .build_move_plan(&[src_vpath.clone()], &dst_vpath, &local_fs, None)
            .await
            .unwrap();

        let mut job = Job::new(JobId(2), Plan::Move(move_plan), ConflictPolicy::OverwriteAll);
        let executor = Executor::new();

        executor
            .execute_job(
                &mut job,
                &local_fs,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                None,
            )
            .await
            .unwrap();

        assert_eq!(job.status, JobStatus::Completed);
        assert!(!src_file.exists());
        assert!(dst_dir.join("move_me.txt").exists());

        // Test delete
        let del_vpath = VPath::new_local(dst_dir.join("move_me.txt").to_str().unwrap());
        let delete_plan = planner
            .build_delete_plan(&[del_vpath], &local_fs, None)
            .await
            .unwrap();

        let mut del_job = Job::new(JobId(3), Plan::Delete(delete_plan), ConflictPolicy::OverwriteAll);
        executor
            .execute_job(
                &mut del_job,
                &local_fs,
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                None,
            )
            .await
            .unwrap();

        assert_eq!(del_job.status, JobStatus::Completed);
        assert!(!dst_dir.join("move_me.txt").exists());
    }

    #[test]
    fn test_conflict_policy_resolution() {
        let src = VPath::new_local("/tmp/a.txt");
        let dst = VPath::new_local("/tmp/b.txt");

        let dec_skip = resolve_conflict(ConflictPolicy::SkipAll, &src, &dst, None, None);
        assert_eq!(dec_skip, ConflictDecision::Skip);

        let dec_overwrite = resolve_conflict(ConflictPolicy::OverwriteAll, &src, &dst, None, None);
        assert_eq!(dec_overwrite, ConflictDecision::Overwrite);

        let dec_cancel = resolve_conflict(ConflictPolicy::Cancel, &src, &dst, None, None);
        assert_eq!(dec_cancel, ConflictDecision::Cancel);
    }

    #[test]
    fn test_queue_manager_operations() {
        let qm = QueueManager::new(None);
        let plan = Plan::Copy(CopyPlan {
            file_count: 2,
            total_bytes: 500,
            steps: vec![],
        });

        let id1 = qm.enqueue(plan.clone(), ConflictPolicy::OverwriteAll).unwrap();
        let id2 = qm.enqueue(plan, ConflictPolicy::SkipAll).unwrap();

        assert_eq!(id1, JobId(1));
        assert_eq!(id2, JobId(2));

        qm.reorder_jobs(0, 1).unwrap();
        let status = qm.get_job_status(id1);
        assert_eq!(status, Some(JobStatus::Pending));

        let progress = qm.aggregate_progress();
        assert_eq!(progress.total_files, 4);
        assert_eq!(progress.total_bytes, 1000);
    }
}
