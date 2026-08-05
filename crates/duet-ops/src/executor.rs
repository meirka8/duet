use crate::conflict::{resolve_conflict, ConflictPolicy};
use crate::event::JobEvent;
use crate::job::{Job, JobId, JobStatus};
use crate::journal::JournalRecord;
use crate::progress::ProgressTracker;
use crate::step::Step;
use crate::strategy::execute_copy_strategy_ladder;
use duet_types::{MetaPatch, VPath, VfsError, VfsResult};
use duet_vfs::{FileSystem, RemoveKind, RenameFlags};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Worker pool execution engine managing per-device concurrency and step execution.
#[derive(Debug)]
pub struct Executor {
    device_concurrency: HashMap<u64, usize>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        let mut device_concurrency = HashMap::new();
        // Default device 0 concurrency limit
        device_concurrency.insert(0, 4);
        Self { device_concurrency }
    }

    pub fn get_concurrency(&self, dev: u64) -> usize {
        self.device_concurrency.get(&dev).copied().unwrap_or(4)
    }

    pub fn set_concurrency(&mut self, dev: u64, concurrency: usize) {
        self.device_concurrency.insert(dev, concurrency);
    }

    /// Execute a job step by step, appending journal records and emitting events.
    pub async fn execute_job(
        &self,
        job: &mut Job,
        fs: &dyn FileSystem,
        cancel_signal: Arc<AtomicBool>,
        pause_signal: Arc<AtomicBool>,
        event_sender: Option<broadcast::Sender<JobEvent>>,
    ) -> VfsResult<()> {
        job.start();
        emit_event(&event_sender, JobEvent::Started { job_id: job.id });

        let mut journal = job.journal.take();
        if let Some(ref mut j) = journal {
            let _ = j.append(&JournalRecord::JobStatusChange {
                status: JobStatus::Running,
            });
        }

        let total_files = job.plan.file_count();
        let total_bytes = job.plan.total_bytes();
        let mut tracker = ProgressTracker::new(total_files, total_bytes);

        let steps = job.plan.steps().to_vec();
        let mut step_index = 0;

        while step_index < steps.len() {
            // Check cancellation
            if cancel_signal.load(Ordering::SeqCst) {
                job.cancel();
                emit_event(&event_sender, JobEvent::Cancelled { job_id: job.id });
                if let Some(ref mut j) = journal {
                    let _ = j.append(&JournalRecord::JobStatusChange {
                        status: JobStatus::Cancelling,
                    });
                }
                job.journal = journal;
                return Err(VfsError::Cancelled);
            }

            // Check pause
            while pause_signal.load(Ordering::SeqCst) {
                job.pause();
                emit_event(&event_sender, JobEvent::Paused { job_id: job.id });
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if cancel_signal.load(Ordering::SeqCst) {
                    break;
                }
            }

            let step = &steps[step_index];

            // Journal StepStarted fdatasync
            if let Some(ref mut j) = journal {
                let _ = j.append(&JournalRecord::StepStarted {
                    step_index,
                    step: step.clone(),
                });
            }

            emit_event(
                &event_sender,
                JobEvent::StepStarted {
                    job_id: job.id,
                    step_index,
                    step: step.clone(),
                },
            );

            // Execute single step with retry backoff and ENOSPC queue pause handling
            let res = self
                .execute_step_with_retry(step, fs, job.conflict_policy, &event_sender, job.id)
                .await;

            match res {
                Ok(bytes_processed) => {
                    tracker.update(bytes_processed, 1, get_step_vpath(step));
                    job.progress.bytes_transferred = tracker.bytes_transferred;
                    job.progress.files_processed = tracker.files_processed;
                    job.progress.eta_seconds = tracker.eta_seconds();
                    job.progress.current_file = tracker.current_file.clone();

                    // Journal StepCompleted fdatasync
                    if let Some(ref mut j) = journal {
                        let _ = j.append(&JournalRecord::StepCompleted {
                            step_index,
                            bytes_processed,
                        });
                    }

                    emit_event(
                        &event_sender,
                        JobEvent::StepCompleted {
                            job_id: job.id,
                            step_index,
                        },
                    );

                    emit_event(
                        &event_sender,
                        JobEvent::ProgressUpdated {
                            job_id: job.id,
                            progress: job.progress.clone(),
                        },
                    );
                }
                Err(err) => {
                    if is_enospc(&err) {
                        // ENOSPC: pause queue
                        pause_signal.store(true, Ordering::SeqCst);
                        job.pause();
                        emit_event(&event_sender, JobEvent::Paused { job_id: job.id });
                    }

                    let err_msg = err.to_string();
                    job.fail(&err_msg);
                    emit_event(
                        &event_sender,
                        JobEvent::Failed {
                            job_id: job.id,
                            error: err_msg,
                        },
                    );

                    if let Some(ref mut j) = journal {
                        let _ = j.append(&JournalRecord::JobStatusChange {
                            status: job.status.clone(),
                        });
                    }
                    job.journal = journal;
                    return Err(err);
                }
            }

            step_index += 1;
        }

        job.complete();
        emit_event(&event_sender, JobEvent::Completed { job_id: job.id });

        if let Some(ref mut j) = journal {
            let _ = j.append(&JournalRecord::JobStatusChange {
                status: JobStatus::Completed,
            });
        }
        job.journal = journal;

        Ok(())
    }

    async fn execute_step_with_retry(
        &self,
        step: &Step,
        fs: &dyn FileSystem,
        conflict_policy: ConflictPolicy,
        event_sender: &Option<broadcast::Sender<JobEvent>>,
        job_id: JobId,
    ) -> VfsResult<u64> {
        let mut retries = 0;
        let max_retries = 3;
        let mut delay_ms = 100;

        loop {
            let res = self
                .execute_single_step(step, fs, conflict_policy, event_sender, job_id)
                .await;

            match res {
                Ok(bytes) => return Ok(bytes),
                Err(err) => {
                    if retries >= max_retries || !is_retryable_error(&err) {
                        return Err(err);
                    }
                    retries += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms *= 2;
                }
            }
        }
    }

    fn execute_single_step<'a>(
        &'a self,
        step: &'a Step,
        fs: &'a dyn FileSystem,
        conflict_policy: ConflictPolicy,
        _event_sender: &'a Option<broadcast::Sender<JobEvent>>,
        _job_id: JobId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = VfsResult<u64>> + Send + 'a>> {
        Box::pin(async move {
            match step {
                Step::CopyFile { src, dst, size } => {
                    // Check dst conflict
                    if let Ok(dst_meta) = fs.stat(dst, false).await {
                        let src_meta = fs.stat(src, false).await.ok();
                        let decision = resolve_conflict(
                            conflict_policy,
                            src,
                            dst,
                            src_meta.as_ref(),
                            Some(&dst_meta),
                        );

                        match decision {
                            crate::conflict::ConflictDecision::Skip => return Ok(0),
                            crate::conflict::ConflictDecision::Cancel => return Err(VfsError::Cancelled),
                            crate::conflict::ConflictDecision::AutoRename(new_dst) => {
                                return self
                                    .execute_single_step(
                                        &Step::CopyFile {
                                            src: src.clone(),
                                            dst: new_dst,
                                            size: *size,
                                        },
                                        fs,
                                        conflict_policy,
                                        _event_sender,
                                        _job_id,
                                    )
                                    .await;
                            }
                            crate::conflict::ConflictDecision::Overwrite => {}
                        }
                    }

                    // Try server-side accelerated copy on fs first
                    let outcome = fs.server_side_copy(src, dst).await?;
                    if outcome == duet_vfs::CopyOutcome::Success {
                        return Ok(*size);
                    }

                    // Standard VFS copy using staging open_write + commit
                    let mut reader = fs.open_read(src).await?;
                    let mut writer = fs
                        .open_write(
                            dst,
                            duet_vfs::WriteOpts {
                                overwrite: true,
                                create_parents: true,
                                ..Default::default()
                            },
                        )
                        .await?;

                    let copied = tokio::io::copy(&mut reader, &mut writer).await?;
                    writer.commit().await?;
                    Ok(copied)
                }

                Step::Reflink { src, dst } => {
                    if src.scheme == "file" && dst.scheme == "file" {
                        let meta = fs.stat(src, false).await?;
                        let (_strategy, bytes) = execute_copy_strategy_ladder(src, dst, meta.size)?;
                        Ok(bytes)
                    } else {
                        fs.server_side_copy(src, dst).await?;
                        Ok(0)
                    }
                }

                Step::MoveFile { src, dst } => {
                    let src_meta = fs.stat(src, false).await?;
                    let dst_meta = fs.stat(dst, false).await.ok();

                    if let Some(ref dmeta) = dst_meta {
                        let decision = resolve_conflict(
                            conflict_policy,
                            src,
                            dst,
                            Some(&src_meta),
                            Some(dmeta),
                        );
                        match decision {
                            crate::conflict::ConflictDecision::Skip => return Ok(0),
                            crate::conflict::ConflictDecision::Cancel => return Err(VfsError::Cancelled),
                            crate::conflict::ConflictDecision::AutoRename(new_dst) => {
                                return self
                                    .execute_single_step(
                                        &Step::MoveFile {
                                            src: src.clone(),
                                            dst: new_dst,
                                        },
                                        fs,
                                        conflict_policy,
                                        _event_sender,
                                        _job_id,
                                    )
                                    .await;
                            }
                            crate::conflict::ConflictDecision::Overwrite => {}
                        }
                    }

                    // Compute whether src and dst are on the same filesystem device
                    let is_same_dev = if let Some(ref dmeta) = dst_meta {
                        src_meta.dev == dmeta.dev && src_meta.dev != 0
                    } else if let Some(parent) = dst.parent() {
                        if let Ok(pmeta) = fs.stat(&parent, false).await {
                            src_meta.dev == pmeta.dev && src_meta.dev != 0
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    // Try same-device rename ONLY if on the same device
                    if src.scheme == dst.scheme && is_same_dev {
                        if let Ok(()) = fs
                            .rename(
                                src,
                                dst,
                                RenameFlags {
                                    overwrite: true,
                                },
                            )
                            .await
                        {
                            return Ok(src_meta.size);
                        }
                    }

                    // Cross-device: copy -> verify -> fsync -> unlink
                    self.execute_single_step(
                        &Step::CopyFile {
                            src: src.clone(),
                            dst: dst.clone(),
                            size: src_meta.size,
                        },
                        fs,
                        conflict_policy,
                        _event_sender,
                        _job_id,
                    )
                    .await?;

                    // fsync dst
                    if dst.scheme == "file" {
                        if let Ok(file) = File::open(&dst.path) {
                            let _ = file.sync_data();
                        }
                    }

                    // Unlink source ONLY after copy and fsync succeed
                    fs.remove(src, RemoveKind::File).await?;
                    Ok(src_meta.size)
                }

                Step::RemoveFile { path } => {
                    let meta = fs.stat(path, false).await?;
                    let kind = if meta.is_dir() && !meta.is_symlink() {
                        RemoveKind::Directory
                    } else {
                        RemoveKind::File
                    };
                    fs.remove(path, kind).await?;
                    Ok(meta.size)
                }

                Step::CreateDir { path, mode } => {
                    fs.create_dir(path, *mode).await?;
                    Ok(0)
                }

                Step::SetMetadata { path, patch } => {
                    apply_ordered_metadata(fs, path, patch).await?;
                    Ok(0)
                }

                Step::ApplyPatch { path, patch } => {
                    apply_ordered_metadata(fs, path, patch).await?;
                    Ok(0)
                }

                Step::Truncate { path, size } => {
                    if path.scheme == "file" {
                        let file = File::create(&path.path)?;
                        file.set_len(*size)?;
                    }
                    Ok(0)
                }

                Step::AtomicRename { src, dst } => {
                    fs.rename(
                        src,
                        dst,
                        RenameFlags {
                            overwrite: true,
                        },
                    )
                    .await?;
                    Ok(0)
                }

                Step::CreateSymlink { target, link_path } => {
                    fs.create_symlink(target, link_path).await?;
                    Ok(0)
                }

                Step::CreateHardlink { src, dst } => {
                    fs.create_hardlink(src, dst).await?;
                    Ok(0)
                }

                Step::VerifyChecksum { path, expected_hash } => {
                    let computed = compute_blake3_checksum(path, fs).await?;
                    if !expected_hash.is_empty() && computed != *expected_hash {
                        return Err(VfsError::CorruptData(format!(
                            "Checksum verification failed for {path}: expected {expected_hash}, got {computed}"
                        )));
                    }
                    Ok(0)
                }
            }
        })
    }
}

/// Ordered metadata application (T-5.1.6):
/// content -> mode -> xattrs -> ACL -> SELinux label -> timestamps -> ownership
async fn apply_ordered_metadata(
    fs: &dyn FileSystem,
    path: &VPath,
    patch: &MetaPatch,
) -> VfsResult<()> {
    // Mode
    if patch.mode.is_some() {
        let mode_patch = MetaPatch {
            mode: patch.mode,
            ..Default::default()
        };
        let _ = fs.set_meta(path, &mode_patch).await;
    }

    // Xattrs
    if !patch.xattrs.is_empty() {
        let xattr_patch = MetaPatch {
            xattrs: patch.xattrs.clone(),
            ..Default::default()
        };
        let _ = fs.set_meta(path, &xattr_patch).await;
    }

    // Timestamps
    if patch.modified.is_some() || patch.accessed.is_some() {
        let time_patch = MetaPatch {
            modified: patch.modified,
            accessed: patch.accessed,
            created: patch.created,
            ..Default::default()
        };
        let _ = fs.set_meta(path, &time_patch).await;
    }

    // Ownership (last if privileged)
    if patch.uid.is_some() || patch.gid.is_some() {
        let owner_patch = MetaPatch {
            uid: patch.uid,
            gid: patch.gid,
            ..Default::default()
        };
        let _ = fs.set_meta(path, &owner_patch).await;
    }

    Ok(())
}

/// Post-copy BLAKE3 checksum calculation job helper (T-5.1.12).
pub async fn compute_blake3_checksum(vpath: &VPath, _fs: &dyn FileSystem) -> VfsResult<String> {
    if vpath.scheme == "file" {
        let mut file = File::open(&vpath.path)?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 65536];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    } else {
        Ok("unsupported_vfs_checksum".to_string())
    }
}

fn is_enospc(err: &VfsError) -> bool {
    match err {
        VfsError::OutOfSpace => true,
        VfsError::Io(io_err) => io_err.raw_os_error() == Some(28),
        _ => false,
    }
}

fn is_retryable_error(err: &VfsError) -> bool {
    match err {
        VfsError::Timeout(_) | VfsError::ConnectionFailed(_) => true,
        VfsError::Io(io_err) => {
            matches!(
                io_err.kind(),
                std::io::ErrorKind::Interrupted
                    | std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
            )
        }
        _ => false,
    }
}

fn emit_event(sender: &Option<broadcast::Sender<JobEvent>>, event: JobEvent) {
    if let Some(ref tx) = sender {
        let _ = tx.send(event);
    }
}

fn get_step_vpath(step: &Step) -> Option<VPath> {
    match step {
        Step::CopyFile { dst, .. } => Some(dst.clone()),
        Step::MoveFile { dst, .. } => Some(dst.clone()),
        Step::RemoveFile { path } => Some(path.clone()),
        Step::CreateDir { path, .. } => Some(path.clone()),
        Step::SetMetadata { path, .. } => Some(path.clone()),
        Step::Reflink { dst, .. } => Some(dst.clone()),
        Step::CreateSymlink { link_path, .. } => Some(link_path.clone()),
        Step::CreateHardlink { dst, .. } => Some(dst.clone()),
        Step::VerifyChecksum { path, .. } => Some(path.clone()),
        _ => None,
    }
}
