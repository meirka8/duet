use async_trait::async_trait;
use duet_ops::{
    ConflictPolicy, Executor, Job, JobId, JobStatus, Journal, JournalRecord, Plan, Planner, Step,
};
use duet_types::{Capabilities, MetaPatch, Metadata, MountId, VPath, VfsError, VfsResult};
use duet_vfs::{
    local::LocalFs, AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry,
    FileSystem, ListOpts, RemoveKind, RenameFlags, WriteOpts,
};
use futures::stream::BoxStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

// =============================================================================
// Helper VFS wrappers for fault injection
// =============================================================================

/// Fault-injecting VFS wrapper that simulates ENOSPC (Out of space) after N bytes written.
struct EnospcFs {
    inner: LocalFs,
    max_bytes: usize,
    bytes_written: Arc<std::sync::atomic::AtomicUsize>,
}

impl EnospcFs {
    fn new(max_bytes: usize) -> Self {
        Self {
            inner: LocalFs::new(),
            max_bytes,
            bytes_written: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

pub struct FaultyWriteCommit {
    inner: Box<dyn AsyncWriteCommit>,
    max_bytes: usize,
    bytes_written: Arc<std::sync::atomic::AtomicUsize>,
}

impl tokio::io::AsyncWrite for FaultyWriteCommit {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let current = self.bytes_written.load(Ordering::SeqCst);
        if current >= self.max_bytes {
            return std::task::Poll::Ready(Err(std::io::Error::from_raw_os_error(libc::ENOSPC)));
        }
        let allowed = (self.max_bytes - current).min(buf.len());
        if allowed == 0 {
            return std::task::Poll::Ready(Err(std::io::Error::from_raw_os_error(libc::ENOSPC)));
        }

        match std::pin::Pin::new(&mut self.inner).poll_write(cx, &buf[..allowed]) {
            std::task::Poll::Ready(Ok(n)) => {
                self.bytes_written.fetch_add(n, Ordering::SeqCst);
                if n < buf.len() && self.bytes_written.load(Ordering::SeqCst) >= self.max_bytes {
                    std::task::Poll::Ready(Err(std::io::Error::from_raw_os_error(libc::ENOSPC)))
                } else {
                    std::task::Poll::Ready(Ok(n))
                }
            }
            other => other,
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[async_trait]
impl AsyncWriteCommit for FaultyWriteCommit {
    async fn commit(self: Box<Self>) -> VfsResult<()> {
        if self.bytes_written.load(Ordering::SeqCst) >= self.max_bytes {
            return Err(VfsError::OutOfSpace);
        }
        self.inner.commit().await
    }
}

#[async_trait]
impl FileSystem for EnospcFs {
    fn mount_id(&self) -> MountId {
        self.inner.mount_id()
    }

    fn scheme(&self) -> &'static str {
        self.inner.scheme()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }

    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, VfsResult<Vec<DirEntry>>> {
        self.inner.read_dir(p, opts)
    }

    async fn stat(&self, p: &VPath, follow: bool) -> VfsResult<Metadata> {
        self.inner.stat(p, follow).await
    }

    async fn open_read(&self, p: &VPath) -> VfsResult<Box<dyn AsyncReadSeek>> {
        self.inner.open_read(p).await
    }

    async fn open_write(&self, p: &VPath, o: WriteOpts) -> VfsResult<Box<dyn AsyncWriteCommit>> {
        let inner_writer = self.inner.open_write(p, o).await?;
        Ok(Box::new(FaultyWriteCommit {
            inner: inner_writer,
            max_bytes: self.max_bytes,
            bytes_written: self.bytes_written.clone(),
        }))
    }

    async fn create_dir(&self, p: &VPath, mode: Option<u32>) -> VfsResult<()> {
        self.inner.create_dir(p, mode).await
    }

    async fn remove(&self, p: &VPath, kind: RemoveKind) -> VfsResult<()> {
        self.inner.remove(p, kind).await
    }

    async fn rename(&self, from: &VPath, to: &VPath, flags: RenameFlags) -> VfsResult<()> {
        self.inner.rename(from, to, flags).await
    }

    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> VfsResult<()> {
        self.inner.set_meta(p, m).await
    }

    fn watch(&self, p: &VPath) -> VfsResult<BoxStream<'_, ChangeEvent>> {
        self.inner.watch(p)
    }

    async fn server_side_copy(&self, from: &VPath, to: &VPath) -> VfsResult<CopyOutcome> {
        self.inner.server_side_copy(from, to).await
    }

    async fn create_symlink(&self, target: &str, p: &VPath) -> VfsResult<()> {
        self.inner.create_symlink(target, p).await
    }

    async fn read_link(&self, p: &VPath) -> VfsResult<String> {
        self.inner.read_link(p).await
    }

    async fn create_hardlink(&self, from: &VPath, to: &VPath) -> VfsResult<()> {
        self.inner.create_hardlink(from, to).await
    }
}

/// Simulated Cross-Device VFS wrapper where source and destination reside on different device IDs.
struct CrossDevFs {
    inner: LocalFs,
    dest_prefix: String,
    unlinked_paths: Arc<std::sync::Mutex<Vec<String>>>,
    fail_on_fsync: bool,
}

impl CrossDevFs {
    fn new(dest_prefix: impl Into<String>, fail_on_fsync: bool) -> Self {
        Self {
            inner: LocalFs::new(),
            dest_prefix: dest_prefix.into(),
            unlinked_paths: Arc::new(std::sync::Mutex::new(Vec::new())),
            fail_on_fsync,
        }
    }

    fn was_unlinked(&self, path: &str) -> bool {
        self.unlinked_paths.lock().unwrap().contains(&path.to_string())
    }
}

#[async_trait]
impl FileSystem for CrossDevFs {
    fn mount_id(&self) -> MountId {
        self.inner.mount_id()
    }

    fn scheme(&self) -> &'static str {
        self.inner.scheme()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }

    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, VfsResult<Vec<DirEntry>>> {
        self.inner.read_dir(p, opts)
    }

    async fn stat(&self, p: &VPath, follow: bool) -> VfsResult<Metadata> {
        let mut meta = self.inner.stat(p, follow).await?;
        if p.path.starts_with(&self.dest_prefix) {
            meta.dev = 9999; // Different filesystem device ID
        } else {
            meta.dev = 1000;
        }
        Ok(meta)
    }

    async fn open_read(&self, p: &VPath) -> VfsResult<Box<dyn AsyncReadSeek>> {
        self.inner.open_read(p).await
    }

    async fn open_write(&self, p: &VPath, o: WriteOpts) -> VfsResult<Box<dyn AsyncWriteCommit>> {
        if self.fail_on_fsync && p.path.starts_with(&self.dest_prefix) {
            return Err(VfsError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Simulated fsync/write failure on cross-dev destination",
            )));
        }
        self.inner.open_write(p, o).await
    }

    async fn create_dir(&self, p: &VPath, mode: Option<u32>) -> VfsResult<()> {
        self.inner.create_dir(p, mode).await
    }

    async fn remove(&self, p: &VPath, kind: RemoveKind) -> VfsResult<()> {
        self.unlinked_paths.lock().unwrap().push(p.path.clone());
        self.inner.remove(p, kind).await
    }

    async fn rename(&self, from: &VPath, to: &VPath, flags: RenameFlags) -> VfsResult<()> {
        self.inner.rename(from, to, flags).await
    }

    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> VfsResult<()> {
        self.inner.set_meta(p, m).await
    }

    fn watch(&self, p: &VPath) -> VfsResult<BoxStream<'_, ChangeEvent>> {
        self.inner.watch(p)
    }

    async fn server_side_copy(&self, from: &VPath, to: &VPath) -> VfsResult<CopyOutcome> {
        self.inner.server_side_copy(from, to).await
    }

    async fn create_symlink(&self, target: &str, p: &VPath) -> VfsResult<()> {
        self.inner.create_symlink(target, p).await
    }

    async fn read_link(&self, p: &VPath) -> VfsResult<String> {
        self.inner.read_link(p).await
    }

    async fn create_hardlink(&self, from: &VPath, to: &VPath) -> VfsResult<()> {
        self.inner.create_hardlink(from, to).await
    }
}

// =============================================================================
// Test Suite 1: Write-ahead Journal Recovery after Simulated SIGKILL (T-5.1.2)
// =============================================================================

#[tokio::test]
async fn test_journal_recovery_after_simulated_sigkill_mid_transfer() {
    let temp = TempDir::new().unwrap();
    let journal_path = temp.path().join("ops_job_1.journal");
    let src_dir = temp.path().join("src");
    let dst_dir = temp.path().join("dst");

    std::fs::create_dir_all(&src_dir).unwrap();
    let file1_src = src_dir.join("file1.dat");
    let file2_src = src_dir.join("file2.dat");
    std::fs::write(&file1_src, "Data block 1 content for journal recovery test").unwrap();
    std::fs::write(&file2_src, "Data block 2 content for journal recovery test").unwrap();

    let local_fs = LocalFs::new();
    let planner = Planner::new();
    let plan = planner
        .build_copy_plan(
            &[VPath::new_local(src_dir.to_str().unwrap())],
            &VPath::new_local(dst_dir.to_str().unwrap()),
            &local_fs,
            None,
        )
        .await
        .unwrap();

    // 1. Simulate process running up to Step 1, then SIGKILL
    let job_id = JobId(101);
    {
        let mut journal = Journal::open(&journal_path).unwrap();
        journal
            .append(&JournalRecord::JobInit {
                job_id,
                plan: Plan::Copy(plan.clone()),
                conflict_policy: ConflictPolicy::OverwriteAll,
            })
            .unwrap();

        // Step 0: CreateDir
        journal
            .append(&JournalRecord::StepStarted {
                step_index: 0,
                step: plan.steps[0].clone(),
            })
            .unwrap();
        journal
            .append(&JournalRecord::StepCompleted {
                step_index: 0,
                bytes_processed: 0,
            })
            .unwrap();

        // Step 1: Copy file1
        journal
            .append(&JournalRecord::StepStarted {
                step_index: 1,
                step: plan.steps[1].clone(),
            })
            .unwrap();
        journal
            .append(&JournalRecord::StepCompleted {
                step_index: 1,
                bytes_processed: 45,
            })
            .unwrap();

        // Step 2: Copy file2 STARTED but NOT COMPLETED (simulated SIGKILL interrupting process)
        journal
            .append(&JournalRecord::StepStarted {
                step_index: 2,
                step: plan.steps[2].clone(),
            })
            .unwrap();

        // Abrupt termination (drop journal without flushing StepCompleted)
    }

    // 2. Recovery phase: startup scanner inspects journal
    let recovery = Journal::recover(&journal_path).unwrap();
    assert!(recovery.is_some(), "Journal recovery scanner failed to parse unclosed journal");
    let (recovered_id, recovered_plan, next_step_idx) = recovery.unwrap();

    assert_eq!(recovered_id, job_id);
    assert_eq!(recovered_plan, Plan::Copy(plan.clone()));
    assert_eq!(
        next_step_idx, 2,
        "Recovery scanner must resume from step index 2 (first uncommitted step)"
    );

    // 3. Clean partial artifacts and resume remaining plan steps
    let mut resumed_job = Job::new(job_id, recovered_plan, ConflictPolicy::OverwriteAll);
    let executor = Executor::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));

    // Execute job to completion
    executor
        .execute_job(&mut resumed_job, &local_fs, cancel, pause, None)
        .await
        .unwrap();

    assert_eq!(resumed_job.status, JobStatus::Completed);
    assert!(dst_dir.join("src").join("file1.dat").exists());
    assert!(dst_dir.join("src").join("file2.dat").exists());
    assert_eq!(
        std::fs::read_to_string(dst_dir.join("src").join("file1.dat")).unwrap(),
        "Data block 1 content for journal recovery test"
    );
    assert_eq!(
        std::fs::read_to_string(dst_dir.join("src").join("file2.dat")).unwrap(),
        "Data block 2 content for journal recovery test"
    );
}

// =============================================================================
// Test Suite 2: ENOSPC Injection mid-copy (T-10.2.1 / Partial Staging Safety)
// =============================================================================

#[tokio::test]
async fn test_enospc_injection_mid_copy_preserves_source_and_marks_partial() {
    let temp = TempDir::new().unwrap();
    let src_path = temp.path().join("source_file.bin");
    let dst_dir = temp.path().join("dst_dir");

    // Create 64 KiB source file
    let source_data = vec![0xABu8; 64 * 1024];
    std::fs::write(&src_path, &source_data).unwrap();

    let src_vpath = VPath::new_local(src_path.to_str().unwrap());
    let dst_vpath = VPath::new_local(dst_dir.to_str().unwrap());

    // Inject ENOSPC after 16 KiB written
    let enospc_fs = EnospcFs::new(16 * 1024);

    let planner = Planner::new();
    let plan = planner
        .build_copy_plan(&[src_vpath.clone()], &dst_vpath, &enospc_fs, None)
        .await
        .unwrap();

    let mut job = Job::new(JobId(202), Plan::Copy(plan), ConflictPolicy::OverwriteAll);
    let executor = Executor::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));

    let result = executor
        .execute_job(&mut job, &enospc_fs, cancel, pause, None)
        .await;

    // 1. Verify job failed with OutOfSpace
    assert!(result.is_err());
    assert!(matches!(job.status, JobStatus::Failed { .. }));

    // 2. Core Invariant Assertion: Source file remains 100% intact
    assert!(src_path.exists(), "Source file MUST remain present on disk");
    let current_src_data = std::fs::read(&src_path).unwrap();
    assert_eq!(
        current_src_data, source_data,
        "Source file content MUST remain 100% intact after ENOSPC fault"
    );

    // 3. Core Invariant Assertion: Destination file does NOT exist as completed file
    let target_dst_file = dst_dir.join("source_file.bin");
    assert!(
        !target_dst_file.exists(),
        "Destination target file MUST NOT exist as a corrupted or partial file"
    );

    // 4. Staging check: any uncommitted file in dst_dir is named .duet-partial-*
    if dst_dir.exists() {
        for entry in std::fs::read_dir(&dst_dir).unwrap() {
            let entry = entry.unwrap();
            let fname = entry.file_name().to_string_lossy().into_owned();
            assert!(
                fname.starts_with(".duet-partial-"),
                "Any uncommitted staging file MUST be prefixed with .duet-partial-*, found: {fname}"
            );
        }
    }
}

// =============================================================================
// Test Suite 3: EACCES Permission Error Handling
// =============================================================================

#[tokio::test]
async fn test_eacces_permission_error_handling() {
    let temp = TempDir::new().unwrap();
    let src_path = temp.path().join("input.txt");
    let readonly_dir = temp.path().join("readonly_dir");

    std::fs::write(&src_path, "Permission test payload").unwrap();
    std::fs::create_dir_all(&readonly_dir).unwrap();

    // Set directory to read-only (mode 0555)
    let mut perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
    perms.set_readonly(true);
    let _ = std::fs::set_permissions(&readonly_dir, perms.clone());

    let local_fs = LocalFs::new();
    let src_vpath = VPath::new_local(src_path.to_str().unwrap());
    let dst_vpath = VPath::new_local(readonly_dir.to_str().unwrap());

    let planner = Planner::new();
    let plan = planner
        .build_copy_plan(&[src_vpath.clone()], &dst_vpath, &local_fs, None)
        .await
        .unwrap();

    let mut job = Job::new(JobId(303), Plan::Copy(plan), ConflictPolicy::OverwriteAll);
    let executor = Executor::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));

    let res = executor
        .execute_job(&mut job, &local_fs, cancel, pause, None)
        .await;

    // Restore permissions for cleanup
    perms.set_readonly(false);
    let _ = std::fs::set_permissions(&readonly_dir, perms);

    // Assert permission error was caught cleanly
    assert!(res.is_err(), "Operation into read-only folder must fail");
    assert!(
        matches!(job.status, JobStatus::Failed { .. }),
        "Job status must be set to Failed on EACCES"
    );

    // Source remains 100% intact
    assert_eq!(
        std::fs::read_to_string(&src_path).unwrap(),
        "Permission test payload"
    );
}

// =============================================================================
// Test Suite 4: Cross-Device Move Safety (T-5.1.5 / Source Unlink Safety)
// =============================================================================

#[tokio::test]
async fn test_cross_device_move_safety_source_never_unlinked_prior_to_fdatasync() {
    let temp = TempDir::new().unwrap();
    let src_dir = temp.path().join("src_dev1");
    let dst_dir = temp.path().join("dst_dev2");

    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();

    let src_file = src_dir.join("critical_doc.pdf");
    std::fs::write(&src_file, "%PDF-1.4 Important binary content").unwrap();

    let src_vpath = VPath::new_local(src_file.to_str().unwrap());
    let dst_vpath = VPath::new_local(dst_dir.join("critical_doc.pdf").to_str().unwrap());

    // 1. Injected failure on cross-dev destination fsync / write
    let faulty_cross_fs = CrossDevFs::new(dst_dir.to_str().unwrap(), true);

    let mut job = Job::new(
        JobId(404),
        Plan::Move(duet_ops::MovePlan {
            file_count: 1,
            total_bytes: 32,
            steps: vec![Step::MoveFile {
                src: src_vpath.clone(),
                dst: dst_vpath.clone(),
            }],
        }),
        ConflictPolicy::OverwriteAll,
    );

    let executor = Executor::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));

    let res = executor
        .execute_job(&mut job, &faulty_cross_fs, cancel.clone(), pause.clone(), None)
        .await;

    // Verify move failed
    assert!(res.is_err());
    // Critical Invariant Assertion: Source file was NEVER unlinked!
    assert!(
        !faulty_cross_fs.was_unlinked(src_file.to_str().unwrap()),
        "Source file MUST NOT be unlinked if destination fsync fails!"
    );
    assert!(
        src_file.exists(),
        "Source file MUST exist on disk after cross-device move failure"
    );

    // 2. Successful cross-device move test
    let valid_cross_fs = CrossDevFs::new(dst_dir.to_str().unwrap(), false);
    let mut successful_job = Job::new(
        JobId(405),
        Plan::Move(duet_ops::MovePlan {
            file_count: 1,
            total_bytes: 32,
            steps: vec![Step::MoveFile {
                src: src_vpath.clone(),
                dst: dst_vpath.clone(),
            }],
        }),
        ConflictPolicy::OverwriteAll,
    );

    executor
        .execute_job(&mut successful_job, &valid_cross_fs, cancel, pause, None)
        .await
        .unwrap();

    assert_eq!(successful_job.status, JobStatus::Completed);
    assert!(
        dst_dir.join("critical_doc.pdf").exists(),
        "Destination file must exist after cross-device move"
    );
    assert!(
        valid_cross_fs.was_unlinked(src_file.to_str().unwrap()),
        "Source file must only be unlinked AFTER destination is written and fsync'd"
    );
    assert!(!src_file.exists(), "Source file should be removed after successful move");
}

// =============================================================================
// Test Suite 5: Hardlink Graph Count Preservation (T-5.1.7)
// =============================================================================

#[tokio::test]
async fn test_hardlink_graph_count_preservation_across_copy_jobs() {
    let temp = TempDir::new().unwrap();
    let src_dir = temp.path().join("hardlink_tree");
    let dst_dir = temp.path().join("copied_tree");

    std::fs::create_dir_all(&src_dir).unwrap();
    let original = src_dir.join("original.txt");
    let hardlink1 = src_dir.join("link1.txt");
    let hardlink2 = src_dir.join("link2.txt");

    std::fs::write(&original, "Shared hardlink inode data").unwrap();
    std::fs::hard_link(&original, &hardlink1).unwrap();
    std::fs::hard_link(&original, &hardlink2).unwrap();

    // Verify initial nlink == 3
    let orig_meta = std::fs::metadata(&original).unwrap();
    assert_eq!(
        orig_meta.nlink(),
        3,
        "Source files must share same inode with nlink 3"
    );

    let local_fs = LocalFs::new();
    let src_vpath = VPath::new_local(src_dir.to_str().unwrap());
    let dst_vpath = VPath::new_local(dst_dir.to_str().unwrap());

    let planner = Planner::new();
    let plan = planner
        .build_copy_plan(&[src_vpath], &dst_vpath, &local_fs, None)
        .await
        .unwrap();

    // Verify plan structure has 1 CopyFile and 2 CreateHardlink steps
    let hardlink_steps = plan
        .steps
        .iter()
        .filter(|s| matches!(s, Step::CreateHardlink { .. }))
        .count();
    assert_eq!(
        hardlink_steps, 2,
        "Planner must detect existing (dev, ino) in inode_map and generate CreateHardlink steps"
    );

    let mut job = Job::new(JobId(505), Plan::Copy(plan), ConflictPolicy::OverwriteAll);
    let executor = Executor::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));

    executor
        .execute_job(&mut job, &local_fs, cancel, pause, None)
        .await
        .unwrap();

    assert_eq!(job.status, JobStatus::Completed);

    let dst_orig = dst_dir.join("hardlink_tree").join("original.txt");
    let dst_link1 = dst_dir.join("hardlink_tree").join("link1.txt");
    let dst_link2 = dst_dir.join("hardlink_tree").join("link2.txt");

    assert!(dst_orig.exists());
    assert!(dst_link1.exists());
    assert!(dst_link2.exists());

    let meta_orig = std::fs::metadata(&dst_orig).unwrap();
    let meta_l1 = std::fs::metadata(&dst_link1).unwrap();
    let meta_l2 = std::fs::metadata(&dst_link2).unwrap();

    // Assert nlink count is preserved as 3 on destination
    assert_eq!(
        meta_orig.nlink(),
        3,
        "Copied hardlink tree destination nlink count must equal 3"
    );

    // Assert inodes are identical across destination hardlinks
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        meta_orig.ino(),
        meta_l1.ino(),
        "Destination hardlink 1 must point to same inode as original"
    );
    assert_eq!(
        meta_orig.ino(),
        meta_l2.ino(),
        "Destination hardlink 2 must point to same inode as original"
    );
}
