//! Branch View (Ctrl+B, Task T-6.1.13) flat directory flattener.

use duet_types::{EntryId, FileType, VPath, VfsError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::EntryInput;

static BRANCH_ENTRY_ID: AtomicU64 = AtomicU64::new(100_000);

pub struct BranchView;

impl BranchView {
    /// Flatten a directory tree recursively into a flat synthetic list of entries.
    pub async fn build_flat_tree(
        root_path: &VPath,
        vfs: Arc<dyn duet_vfs::FileSystem>,
    ) -> Result<Vec<EntryInput>, VfsError> {
        let mut results = Vec::new();
        Self::traverse_recursive(root_path, vfs.as_ref(), &mut results).await?;
        Ok(results)
    }

    fn traverse_recursive<'a>(
        dir_path: &'a VPath,
        vfs: &'a (dyn duet_vfs::FileSystem + 'static),
        out: &'a mut Vec<EntryInput>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), VfsError>> + Send + 'a>> {
        Box::pin(async move {
            use futures::StreamExt;

            let opts = duet_vfs::ListOpts {
                size: true,
                mtime: true,
                mode: true,
                file_type: true,
            };

            let mut stream = vfs.read_dir(dir_path, opts);
            let mut subdirs = Vec::new();

            while let Some(item_res) = stream.next().await {
                if let Ok(entries) = item_res {
                    for item in entries {
                        let id = EntryId(BRANCH_ENTRY_ID.fetch_add(1, Ordering::SeqCst));
                        let is_dir = item.is_dir;
                        let size = item.metadata.as_ref().map(|m| m.size).unwrap_or(0);
                        let mode = item.metadata.as_ref().map(|m| m.mode).unwrap_or(0o644);
                        let mtime = item.metadata.as_ref().and_then(|m| m.modified).unwrap_or(1);

                        let entry_input = EntryInput {
                            id,
                            name: format!("{}/{}", dir_path.path.trim_start_matches('/'), item.name),
                            file_type: if is_dir { FileType::Directory } else { FileType::File },
                            size,
                            mode,
                            uid: 1000,
                            gid: 1000,
                            mtime,
                            atime: mtime,
                            ctime: mtime,
                            dev: 1,
                            ino: id.0,
                            nlink: 1,
                            flags: 0,
                        };
                        out.push(entry_input);

                        if is_dir {
                            let sub_path = VPath::parse(&format!("{dir_path}/{}", item.name))?;
                            subdirs.push(sub_path);
                        }
                    }
                }
            }

            for sub in subdirs {
                let _ = Self::traverse_recursive(&sub, vfs, out).await;
            }

            Ok(())
        })
    }
}
