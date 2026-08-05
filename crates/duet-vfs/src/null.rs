//! Stub `FileSystem` implementation (`NullFs`).
//!
//! `NullFs` acts as a sentinel or null-object backend in the VFS mount table.
//! It implements `FileSystem` and explicitly defines error semantics and contracts
//! for backends when target items do not exist, operations are read-only, or features
//! are unsupported.

use crate::{
    AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry, FileSystem, ListOpts,
    RemoveKind, RenameFlags, WriteOpts,
};
use async_trait::async_trait;
use duet_types::{Capabilities, MetaPatch, Metadata, MountId, VPath, VfsError, VfsResult};
use futures::stream::{self, BoxStream};

/// A stub/null filesystem backend.
///
/// Error semantics:
/// - Read operations (`stat`, `open_read`, `read_dir`) return `VfsError::NotFound`.
/// - Mutation operations (`open_write`, `create_dir`, `remove`) return `VfsError::ReadOnlyFilesystem`.
/// - Advanced / backend-specific features (`rename`, `set_meta`, `server_side_copy`) return `VfsError::Unsupported`.
#[derive(Debug, Clone)]
pub struct NullFs {
    mount_id: MountId,
}

impl NullFs {
    /// Create a new `NullFs` with a given `MountId`.
    pub fn new(mount_id: MountId) -> Self {
        Self { mount_id }
    }
}

impl Default for NullFs {
    fn default() -> Self {
        Self::new(MountId(0))
    }
}

#[async_trait]
impl FileSystem for NullFs {
    fn mount_id(&self) -> MountId {
        self.mount_id
    }

    fn scheme(&self) -> &'static str {
        "null"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::empty()
    }

    fn read_dir(&self, p: &VPath, _opts: ListOpts) -> BoxStream<'_, VfsResult<Vec<DirEntry>>> {
        let err = VfsError::NotFound(format!("NullFs path not found: {p}"));
        Box::pin(stream::once(async move { Err(err) }))
    }

    async fn stat(&self, p: &VPath, _follow: bool) -> VfsResult<Metadata> {
        Err(VfsError::NotFound(format!("NullFs path not found: {p}")))
    }

    async fn open_read(&self, p: &VPath) -> VfsResult<Box<dyn AsyncReadSeek>> {
        Err(VfsError::NotFound(format!("NullFs path not found: {p}")))
    }

    async fn open_write(&self, _p: &VPath, _o: WriteOpts) -> VfsResult<Box<dyn AsyncWriteCommit>> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn create_dir(&self, _p: &VPath, _mode: Option<u32>) -> VfsResult<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn remove(&self, p: &VPath, _kind: RemoveKind) -> VfsResult<()> {
        Err(VfsError::NotFound(format!("NullFs path not found: {p}")))
    }

    async fn rename(&self, _from: &VPath, _to: &VPath, _flags: RenameFlags) -> VfsResult<()> {
        Err(VfsError::Unsupported(
            "NullFs does not support rename operations".to_string(),
        ))
    }

    async fn set_meta(&self, _p: &VPath, _m: &MetaPatch) -> VfsResult<()> {
        Err(VfsError::Unsupported(
            "NullFs does not support metadata mutations".to_string(),
        ))
    }

    fn watch(&self, _p: &VPath) -> VfsResult<BoxStream<'_, ChangeEvent>> {
        Ok(Box::pin(stream::empty()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> VfsResult<CopyOutcome> {
        Ok(CopyOutcome::Unsupported)
    }

    async fn create_symlink(&self, _target: &str, _p: &VPath) -> VfsResult<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn read_link(&self, p: &VPath) -> VfsResult<String> {
        Err(VfsError::NotFound(format!("NullFs path not found: {p}")))
    }

    async fn create_hardlink(&self, _from: &VPath, _to: &VPath) -> VfsResult<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }
}
