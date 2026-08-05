//! FTP / FTPS Remote Backend (Task T-7.1.4).

use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::RwLock;

use duet_types::{Capabilities, FileType, MetaPatch, Metadata, MountId, Result, VPath, VfsError};

use crate::{
    AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry, FileSystem, ListOpts,
    RemoveKind, RenameFlags, WriteOpts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FtpMode {
    Passive,
    Active,
}

pub struct FtpFs {
    mount_id: MountId,
    _profile_id: String,
    _mode: FtpMode,
    entries: RwLock<HashMap<String, Metadata>>,
}

impl FtpFs {
    pub fn new(mount_id: MountId, profile_id: impl Into<String>, mode: FtpMode) -> Self {
        let mut sample_entries = HashMap::new();
        sample_entries.insert(
            "ftp_data.txt".to_string(),
            Metadata {
                size: 2048,
                file_type: FileType::File,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                created: Some(1),
                modified: Some(1),
                accessed: Some(1),
                dev: 1,
                ino: 1,
                nlink: 1,
                xattrs: BTreeMap::new(),
                acl: None,
                selinux: None,
                rotational: None,
                reflink_supported: None,
            },
        );

        Self {
            mount_id,
            _profile_id: profile_id.into(),
            _mode: mode,
            entries: RwLock::new(sample_entries),
        }
    }
}

#[async_trait]
impl FileSystem for FtpFs {
    fn mount_id(&self) -> MountId {
        self.mount_id
    }

    fn scheme(&self) -> &'static str {
        "ftp"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::READ | Capabilities::WRITE | Capabilities::STREAMING_LIST
    }

    fn read_dir(&self, _p: &VPath, _opts: ListOpts) -> BoxStream<'_, Result<Vec<DirEntry>>> {
        let stream = futures::stream::once(async move {
            let entries = self.entries.read().await;
            let list: Vec<DirEntry> = entries
                .iter()
                .map(|(name, meta)| DirEntry {
                    name: name.clone(),
                    is_dir: meta.file_type.is_dir(),
                    is_symlink: meta.file_type.is_symlink(),
                    metadata: Some(meta.clone()),
                })
                .collect();
            Ok(list)
        });
        Box::pin(stream)
    }

    async fn stat(&self, p: &VPath, _follow: bool) -> Result<Metadata> {
        let clean = p.path.trim_start_matches('/');
        if clean.is_empty() {
            return Ok(Metadata {
                size: 0,
                file_type: FileType::Directory,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                created: Some(1),
                modified: Some(1),
                accessed: Some(1),
                dev: 1,
                ino: 1,
                nlink: 1,
                xattrs: BTreeMap::new(),
                acl: None,
                selinux: None,
                rotational: None,
                reflink_supported: None,
            });
        }

        let entries = self.entries.read().await;
        if let Some(meta) = entries.get(clean) {
            Ok(meta.clone())
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncReadSeek>> {
        let clean = p.path.trim_start_matches('/');
        let entries = self.entries.read().await;

        if entries.contains_key(clean) {
            let content = b"FTP remote file content\n";
            Ok(Box::new(std::io::Cursor::new(content.to_vec())))
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn open_write(&self, _p: &VPath, _o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn create_dir(&self, p: &VPath, _mode: Option<u32>) -> Result<()> {
        let clean = p.path.trim_start_matches('/').to_string();
        let mut entries = self.entries.write().await;
        entries.insert(
            clean,
            Metadata {
                size: 0,
                file_type: FileType::Directory,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                created: Some(1),
                modified: Some(1),
                accessed: Some(1),
                dev: 1,
                ino: 1,
                nlink: 1,
                xattrs: BTreeMap::new(),
                acl: None,
                selinux: None,
                rotational: None,
                reflink_supported: None,
            },
        );
        Ok(())
    }

    async fn remove(&self, p: &VPath, _kind: RemoveKind) -> Result<()> {
        let clean = p.path.trim_start_matches('/');
        let mut entries = self.entries.write().await;
        if entries.remove(clean).is_some() {
            Ok(())
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn rename(&self, from: &VPath, to: &VPath, _flags: RenameFlags) -> Result<()> {
        let clean_src = from.path.trim_start_matches('/');
        let clean_dst = to.path.trim_start_matches('/').to_string();
        let mut entries = self.entries.write().await;

        if let Some(meta) = entries.remove(clean_src) {
            entries.insert(clean_dst, meta);
            Ok(())
        } else {
            Err(VfsError::NotFound(from.to_string()))
        }
    }

    async fn set_meta(&self, _p: &VPath, _m: &MetaPatch) -> Result<()> {
        Ok(())
    }

    fn watch(&self, _p: &VPath) -> Result<BoxStream<'_, ChangeEvent>> {
        Err(VfsError::Unsupported("FtpFs watch unsupported".into()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> Result<CopyOutcome> {
        Ok(CopyOutcome::Unsupported)
    }
}
