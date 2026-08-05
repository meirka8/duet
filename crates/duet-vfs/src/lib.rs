use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

pub use duet_types::{
    Capabilities, Caps, EntryId, Error, FileType, MetaPatch, Metadata, MountId, Result, VPath,
    VfsError, VfsResult,
};

pub mod local;
pub mod null;

pub use local::LocalFs;
pub use null::NullFs;

/// Listing options to restrict requested metadata fields for efficient listing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ListOpts {
    pub size: bool,
    pub mtime: bool,
    pub mode: bool,
    pub file_type: bool,
}

/// Directory entry item returned by `read_dir`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub metadata: Option<Metadata>,
}

/// Category of item being removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveKind {
    File,
    Directory,
}

/// Options for file rename operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenameFlags {
    pub overwrite: bool,
}

/// Event notification for filesystem directory watch streams.
#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Created(VPath),
    Deleted(VPath),
    Modified(VPath),
}

/// Outcome of backend-accelerated server-side file copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyOutcome {
    Success,
    Unsupported,
}

/// Options for file creation and write operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteOpts {
    pub overwrite: bool,
    pub append: bool,
    pub create_parents: bool,
    pub mode: Option<u32>,
}

/// Handle abstraction for atomic file write operations.
/// Writing takes place into staging / temporary locations, and `commit()`
/// atomically publishes the change to the target destination upon completion.
#[async_trait]
pub trait AsyncWriteCommit: tokio::io::AsyncWrite + Send + Unpin {
    async fn commit(self: Box<Self>) -> Result<()>;
}

/// Handle abstraction for seekable read streams.
pub trait AsyncReadSeek: tokio::io::AsyncRead + tokio::io::AsyncSeek + Send + Unpin {}

impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Send + Unpin> AsyncReadSeek for T {}

/// Central FileSystem trait defining asynchronous VFS operational contracts.
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Return the unique MountId associated with this mounted filesystem.
    fn mount_id(&self) -> MountId;

    /// Return the URI scheme implemented by this filesystem (e.g. "file", "sftp", "zip").
    fn scheme(&self) -> &'static str;

    /// Return capability bitflags supported by this backend.
    fn capabilities(&self) -> Capabilities;

    /// Alias for `capabilities()`.
    fn caps(&self) -> Capabilities {
        self.capabilities()
    }

    /// Stream directory contents in chunks for responsive UI progressive loading.
    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, Result<Vec<DirEntry>>>;

    /// Fetch metadata for a given path.
    async fn stat(&self, p: &VPath, follow: bool) -> Result<Metadata>;

    /// Open a file for reading.
    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncReadSeek>>;

    /// Open a handle for writing with commit semantics.
    async fn open_write(&self, p: &VPath, o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>>;

    /// Alias for `open_write`.
    async fn create_write(&self, p: &VPath, o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>> {
        self.open_write(p, o).await
    }

    /// Create a directory.
    async fn create_dir(&self, p: &VPath, mode: Option<u32>) -> Result<()>;

    /// Alias for `create_dir`.
    async fn mkdir(&self, p: &VPath, mode: Option<u32>) -> Result<()> {
        self.create_dir(p, mode).await
    }

    /// Remove a file or empty directory.
    async fn remove(&self, p: &VPath, kind: RemoveKind) -> Result<()>;

    /// Rename or move a file/directory.
    async fn rename(&self, from: &VPath, to: &VPath, flags: RenameFlags) -> Result<()>;

    /// Update metadata attributes for a given path.
    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> Result<()>;

    /// Alias for `set_meta`.
    async fn set_metadata(&self, p: &VPath, m: &MetaPatch) -> Result<()> {
        self.set_meta(p, m).await
    }

    /// Watch path for change events.
    fn watch(&self, p: &VPath) -> Result<BoxStream<'_, ChangeEvent>>;

    /// Backend-accelerated copy. Returns `CopyOutcome::Unsupported` to request engine fallback.
    async fn server_side_copy(&self, from: &VPath, to: &VPath) -> Result<CopyOutcome>;

    /// Alias for `server_side_copy`.
    async fn copy_file(&self, from: &VPath, to: &VPath) -> Result<CopyOutcome> {
        self.server_side_copy(from, to).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_null_fs_contracts() {
        let null_fs = NullFs::new(MountId(1));
        let path = VPath::new_local("/test.txt");

        assert_eq!(null_fs.mount_id(), MountId(1));
        assert_eq!(null_fs.scheme(), "null");
        assert!(null_fs.capabilities().is_empty());

        assert!(null_fs.stat(&path, true).await.is_err());
        assert!(null_fs
            .open_write(&path, WriteOpts::default())
            .await
            .is_err());
        assert!(null_fs.create_dir(&path, None).await.is_err());
    }
}
