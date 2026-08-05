use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Serialize, Deserialize};

pub use duet_types::{Caps, Error, MetaPatch, Metadata, Result, VPath};

pub mod local;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ListOpts {
    pub size: bool,
    pub mtime: bool,
    pub mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenameFlags {
    pub overwrite: bool,
}

#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Created(VPath),
    Deleted(VPath),
    Modified(VPath),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyOutcome {
    Success,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WriteOpts {
    pub overwrite: bool,
}

#[async_trait]
pub trait AsyncWriteCommit: tokio::io::AsyncWrite + Send + Unpin {
    async fn commit(self: Box<Self>) -> Result<()>;
}

pub trait AsyncReadSeek: tokio::io::AsyncRead + tokio::io::AsyncSeek + Send + Unpin {}

impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Send + Unpin> AsyncReadSeek for T {}

#[async_trait]
pub trait FileSystem: Send + Sync {
    fn scheme(&self) -> &'static str;
    fn caps(&self) -> Caps;
    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, Result<Vec<DirEntry>>>;
    async fn stat(&self, p: &VPath, follow: bool) -> Result<Metadata>;
    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncReadSeek>>;
    async fn open_write(&self, p: &VPath, o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>>;
    async fn create_dir(&self, p: &VPath, mode: Option<u32>) -> Result<()>;
    async fn remove(&self, p: &VPath, kind: RemoveKind) -> Result<()>;
    async fn rename(&self, from: &VPath, to: &VPath, flags: RenameFlags) -> Result<()>;
    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> Result<()>;
    fn watch(&self, p: &VPath) -> Result<BoxStream<'_, ChangeEvent>>;
    async fn server_side_copy(&self, from: &VPath, to: &VPath) -> Result<CopyOutcome>;
}
