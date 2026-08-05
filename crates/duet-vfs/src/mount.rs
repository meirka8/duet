//! VFS Mount Table (Task T-6.1.1) supporting nested mounts, reference counting, and lifecycle management.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use duet_types::{MountId, VPath, VfsError};
use tokio::sync::RwLock;

use crate::FileSystem;

static NEXT_MOUNT_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_mount_id() -> MountId {
    MountId(NEXT_MOUNT_ID.fetch_add(1, Ordering::SeqCst))
}

/// Record of an active mount in the VFS mount table.
pub struct MountRecord {
    pub id: MountId,
    pub path_prefix: String,
    pub fs: Arc<dyn FileSystem>,
    pub ref_count: usize,
    pub parent_mount_id: Option<MountId>,
}

/// Global/Workspace Mount Table manager.
#[derive(Default)]
pub struct MountTable {
    mounts: RwLock<HashMap<MountId, MountRecord>>,
    path_map: RwLock<HashMap<String, MountId>>,
}

impl MountTable {
    pub fn new() -> Self {
        Self {
            mounts: RwLock::new(HashMap::new()),
            path_map: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new mounted filesystem under a path prefix.
    pub async fn register_mount(
        &self,
        path_prefix: impl Into<String>,
        fs: Arc<dyn FileSystem>,
        parent_mount_id: Option<MountId>,
    ) -> Result<MountId, VfsError> {
        let prefix = path_prefix.into();
        let id = next_mount_id();

        let record = MountRecord {
            id,
            path_prefix: prefix.clone(),
            fs,
            ref_count: 1,
            parent_mount_id,
        };

        let mut mounts = self.mounts.write().await;
        let mut path_map = self.path_map.write().await;

        path_map.insert(prefix, id);
        mounts.insert(id, record);

        Ok(id)
    }

    /// Resolve a `VPath` to its active `FileSystem` instance and inner relative path.
    pub async fn resolve_path(&self, vpath: &VPath) -> Result<(Arc<dyn FileSystem>, String), VfsError> {
        let full_uri = vpath.to_string();
        let path_map = self.path_map.read().await;
        let mounts = self.mounts.read().await;

        // Longest prefix match
        let mut best_match: Option<(&String, &MountId)> = None;
        for (prefix, id) in path_map.iter() {
            if full_uri.starts_with(prefix) {
                if let Some((best_prefix, _)) = best_match {
                    if prefix.len() > best_prefix.len() {
                        best_match = Some((prefix, id));
                    }
                } else {
                    best_match = Some((prefix, id));
                }
            }
        }

        if let Some((prefix, id)) = best_match {
            if let Some(record) = mounts.get(id) {
                let inner_path = full_uri[prefix.len()..].to_string();
                let clean_inner = if inner_path.is_empty() {
                    "/".to_string()
                } else if !inner_path.starts_with('/') {
                    format!("/{}", inner_path)
                } else {
                    inner_path
                };
                return Ok((record.fs.clone(), clean_inner));
            }
        }

        Err(VfsError::NotFound(full_uri))
    }

    /// Increment reference count for a mount.
    pub async fn acquire(&self, id: MountId) -> Result<(), VfsError> {
        let mut mounts = self.mounts.write().await;
        if let Some(record) = mounts.get_mut(&id) {
            record.ref_count += 1;
            Ok(())
        } else {
            Err(VfsError::NotFound(format!("MountId({:?})", id)))
        }
    }

    /// Decrement reference count; unmount if ref_count reaches 0.
    pub async fn release(&self, id: MountId) -> Result<bool, VfsError> {
        let mut mounts = self.mounts.write().await;
        let mut path_map = self.path_map.write().await;

        if let Some(record) = mounts.get_mut(&id) {
            record.ref_count = record.ref_count.saturating_sub(1);
            if record.ref_count == 0 {
                let prefix = record.path_prefix.clone();
                path_map.remove(&prefix);
                mounts.remove(&id);
                Ok(true) // Unmounted
            } else {
                Ok(false) // Still referenced
            }
        } else {
            Err(VfsError::NotFound(format!("MountId({:?})", id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NullFs;

    #[tokio::test]
    async fn test_mount_table_registration_and_resolution() {
        let table = MountTable::new();
        let null_fs = Arc::new(NullFs::new(MountId(1)));

        let id = table
            .register_mount("file:///tmp", null_fs.clone(), None)
            .await
            .unwrap();

        let vpath = VPath::parse("file:///tmp/foo/bar.txt").unwrap();
        let (_fs, inner_path) = table.resolve_path(&vpath).await.unwrap();
        assert_eq!(inner_path, "/foo/bar.txt");

        let released = table.release(id).await.unwrap();
        assert!(released);
    }
}
