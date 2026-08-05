use crate::plan::{CopyPlan, DeletePlan, MovePlan, SyncPlan};
use crate::step::Step;
use duet_types::{MetaPatch, Metadata, VPath, VfsError, VfsResult};
use duet_vfs::{FileSystem, ListOpts};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Tree planner for generating materialised execution plans.
#[derive(Debug, Default)]
pub struct Planner {
    verify_checksums: bool,
}

impl Planner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_verification(mut self, verify: bool) -> Self {
        self.verify_checksums = verify;
        self
    }

    /// Build an async cancellable CopyPlan.
    pub async fn build_copy_plan(
        &self,
        sources: &[VPath],
        destination_dir: &VPath,
        fs: &dyn FileSystem,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<CopyPlan> {
        let mut steps = Vec::new();
        let mut inode_map: HashMap<(u64, u64), VPath> = HashMap::new();
        let mut total_files = 0u64;
        let mut total_bytes = 0u64;

        steps.push(Step::CreateDir {
            path: destination_dir.clone(),
            mode: Some(0o755),
        });

        for src in sources {
            if is_cancelled(cancel_signal) {
                return Err(VfsError::Cancelled);
            }

            let meta = fs.stat(src, false).await?;
            let file_name = src.file_name().unwrap_or("file");
            let dst_path = join_vpath(destination_dir, file_name);

            self.plan_copy_recursive(
                src,
                &dst_path,
                &meta,
                fs,
                &mut steps,
                &mut inode_map,
                &mut total_files,
                &mut total_bytes,
                cancel_signal,
            )
            .await?;
        }

        Ok(CopyPlan {
            file_count: total_files,
            total_bytes,
            steps,
        })
    }

    /// Build an async cancellable MovePlan.
    pub async fn build_move_plan(
        &self,
        sources: &[VPath],
        destination_dir: &VPath,
        fs: &dyn FileSystem,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<MovePlan> {
        // Build copy plan then add steps to delete sources
        let copy_plan = self
            .build_copy_plan(sources, destination_dir, fs, cancel_signal)
            .await?;

        let mut steps = copy_plan.steps;

        // Append source file cleanup steps in post-order
        for src in sources {
            if is_cancelled(cancel_signal) {
                return Err(VfsError::Cancelled);
            }
            let delete_plan = self.build_delete_plan(&[src.clone()], fs, cancel_signal).await?;
            steps.extend(delete_plan.steps);
        }

        Ok(MovePlan {
            file_count: copy_plan.file_count,
            total_bytes: copy_plan.total_bytes,
            steps,
        })
    }

    /// Build an async cancellable DeletePlan (symlink-safe, post-order).
    pub async fn build_delete_plan(
        &self,
        sources: &[VPath],
        fs: &dyn FileSystem,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<DeletePlan> {
        let mut steps = Vec::new();
        let mut total_files = 0u64;
        let mut total_bytes = 0u64;

        for src in sources {
            if is_cancelled(cancel_signal) {
                return Err(VfsError::Cancelled);
            }

            let meta = fs.stat(src, false).await?;
            self.plan_delete_recursive(
                src,
                &meta,
                fs,
                &mut steps,
                &mut total_files,
                &mut total_bytes,
                cancel_signal,
            )
            .await?;
        }

        Ok(DeletePlan {
            file_count: total_files,
            total_bytes,
            steps,
        })
    }

    /// Build an async cancellable SyncPlan.
    pub async fn build_sync_plan(
        &self,
        source_dir: &VPath,
        target_dir: &VPath,
        fs: &dyn FileSystem,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<SyncPlan> {
        let copy_plan = self
            .build_copy_plan(&[source_dir.clone()], target_dir, fs, cancel_signal)
            .await?;

        Ok(SyncPlan {
            file_count: copy_plan.file_count,
            total_bytes: copy_plan.total_bytes,
            steps: copy_plan.steps,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn plan_copy_recursive(
        &self,
        src: &VPath,
        dst: &VPath,
        meta: &Metadata,
        fs: &dyn FileSystem,
        steps: &mut Vec<Step>,
        inode_map: &mut HashMap<(u64, u64), VPath>,
        total_files: &mut u64,
        total_bytes: &mut u64,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<()> {
        if is_cancelled(cancel_signal) {
            return Err(VfsError::Cancelled);
        }

        // Check hardlink map (dev, ino)
        let key = (meta.dev, meta.ino);
        if meta.nlink > 1 && meta.dev != 0 && meta.ino != 0 {
            if let Some(existing_dst) = inode_map.get(&key) {
                steps.push(Step::CreateHardlink {
                    src: existing_dst.clone(),
                    dst: dst.clone(),
                });
                *total_files += 1;
                return Ok(());
            } else {
                inode_map.insert(key, dst.clone());
            }
        }

        if meta.is_dir() {
            steps.push(Step::CreateDir {
                path: dst.clone(),
                mode: Some(meta.mode),
            });

            use futures::StreamExt;
            let opts = ListOpts {
                size: true,
                mtime: true,
                mode: true,
                file_type: true,
            };
            let mut stream = fs.read_dir(src, opts);

            while let Some(chunk_res) = stream.next().await {
                if is_cancelled(cancel_signal) {
                    return Err(VfsError::Cancelled);
                }

                let chunk = chunk_res?;
                for entry in chunk {
                    let child_src = join_vpath(src, &entry.name);
                    let child_dst = join_vpath(dst, &entry.name);
                    let child_meta = if let Some(m) = entry.metadata {
                        m
                    } else {
                        fs.stat(&child_src, false).await?
                    };

                    Box::pin(self.plan_copy_recursive(
                        &child_src,
                        &child_dst,
                        &child_meta,
                        fs,
                        steps,
                        inode_map,
                        total_files,
                        total_bytes,
                        cancel_signal,
                    ))
                    .await?;
                }
            }

            // Set metadata on directory after child entries are created
            steps.push(Step::SetMetadata {
                path: dst.clone(),
                patch: MetaPatch {
                    mode: Some(meta.mode),
                    modified: meta.modified,
                    accessed: meta.accessed,
                    uid: Some(meta.uid),
                    gid: Some(meta.gid),
                    ..Default::default()
                },
            });
        } else if meta.is_symlink() {
            let link_target = fs.read_link(src).await.unwrap_or_default();
            steps.push(Step::CreateSymlink {
                target: link_target,
                link_path: dst.clone(),
            });
            *total_files += 1;
        } else {
            // Regular file
            steps.push(Step::CopyFile {
                src: src.clone(),
                dst: dst.clone(),
                size: meta.size,
            });

            if self.verify_checksums {
                steps.push(Step::VerifyChecksum {
                    path: dst.clone(),
                    expected_hash: String::new(), // Computed during execution
                });
            }

            steps.push(Step::SetMetadata {
                path: dst.clone(),
                patch: MetaPatch {
                    mode: Some(meta.mode),
                    modified: meta.modified,
                    accessed: meta.accessed,
                    uid: Some(meta.uid),
                    gid: Some(meta.gid),
                    ..Default::default()
                },
            });

            *total_files += 1;
            *total_bytes += meta.size;
        }

        Ok(())
    }

    async fn plan_delete_recursive(
        &self,
        src: &VPath,
        meta: &Metadata,
        fs: &dyn FileSystem,
        steps: &mut Vec<Step>,
        total_files: &mut u64,
        total_bytes: &mut u64,
        cancel_signal: Option<&Arc<AtomicBool>>,
    ) -> VfsResult<()> {
        if is_cancelled(cancel_signal) {
            return Err(VfsError::Cancelled);
        }

        // Symlink safety: do NOT recurse into symlinks even if target is directory!
        if meta.is_dir() && !meta.is_symlink() {
            use futures::StreamExt;
            let opts = ListOpts {
                size: true,
                mtime: false,
                mode: false,
                file_type: true,
            };
            let mut stream = fs.read_dir(src, opts);

            while let Some(chunk_res) = stream.next().await {
                if is_cancelled(cancel_signal) {
                    return Err(VfsError::Cancelled);
                }

                let chunk = chunk_res?;
                for entry in chunk {
                    let child_src = join_vpath(src, &entry.name);
                    let child_meta = if let Some(m) = entry.metadata {
                        m
                    } else {
                        fs.stat(&child_src, false).await?
                    };

                    Box::pin(self.plan_delete_recursive(
                        &child_src,
                        &child_meta,
                        fs,
                        steps,
                        total_files,
                        total_bytes,
                        cancel_signal,
                    ))
                    .await?;
                }
            }

            // Delete directory itself post-order
            steps.push(Step::RemoveFile { path: src.clone() });
        } else {
            // File or Symlink
            steps.push(Step::RemoveFile { path: src.clone() });
            *total_files += 1;
            *total_bytes += meta.size;
        }

        Ok(())
    }
}

fn is_cancelled(cancel_signal: Option<&Arc<AtomicBool>>) -> bool {
    if let Some(signal) = cancel_signal {
        signal.load(Ordering::SeqCst)
    } else {
        false
    }
}

fn join_vpath(base: &VPath, child: &str) -> VPath {
    let mut new_vpath = base.clone();
    if new_vpath.path.ends_with('/') {
        new_vpath.path.push_str(child);
    } else {
        new_vpath.path.push('/');
        new_vpath.path.push_str(child);
    }
    new_vpath
}
