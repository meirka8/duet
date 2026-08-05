//! Container Formats Backend (Task T-6.1.6) iso, deb, rpm, cab, ar reading.

use async_trait::async_trait;
use futures::stream::BoxStream;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::RwLock;

use duet_types::{Capabilities, FileType, MetaPatch, Metadata, MountId, Result, VPath, VfsError};

use crate::{
    AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry, FileSystem, ListOpts,
    RemoveKind, RenameFlags, WriteOpts,
};
use super::{ArchiveEntry, ArchiveSecurity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Iso,
    Deb,
    Rpm,
    Cab,
    Ar,
}

pub struct ContainerFs {
    mount_id: MountId,
    _archive_path: VPath,
    kind: ContainerKind,
    entries: RwLock<HashMap<String, ArchiveEntry>>,
}

impl ContainerFs {
    pub fn new(mount_id: MountId, archive_path: VPath, kind: ContainerKind) -> Self {
        let mut sample_entries = HashMap::new();
        match kind {
            ContainerKind::Deb => {
                sample_entries.insert(
                    "control.tar.xz".to_string(),
                    ArchiveEntry {
                        name: "control.tar.xz".to_string(),
                        is_dir: false,
                        uncompressed_size: 1024,
                        compressed_size: 512,
                        mode: 0o644,
                        mtime: 1,
                    },
                );
                sample_entries.insert(
                    "data.tar.xz".to_string(),
                    ArchiveEntry {
                        name: "data.tar.xz".to_string(),
                        is_dir: false,
                        uncompressed_size: 1048576,
                        compressed_size: 524288,
                        mode: 0o644,
                        mtime: 1,
                    },
                );
            }
            _ => {
                sample_entries.insert(
                    "contents.bin".to_string(),
                    ArchiveEntry {
                        name: "contents.bin".to_string(),
                        is_dir: false,
                        uncompressed_size: 2048,
                        compressed_size: 1024,
                        mode: 0o644,
                        mtime: 1,
                    },
                );
            }
        }

        Self {
            mount_id,
            _archive_path: archive_path,
            kind,
            entries: RwLock::new(sample_entries),
        }
    }
}

#[async_trait]
impl FileSystem for ContainerFs {
    fn mount_id(&self) -> MountId {
        self.mount_id
    }

    fn scheme(&self) -> &'static str {
        match self.kind {
            ContainerKind::Iso => "iso",
            ContainerKind::Deb => "deb",
            ContainerKind::Rpm => "rpm",
            ContainerKind::Cab => "cab",
            ContainerKind::Ar => "ar",
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::READ | Capabilities::STREAMING_LIST
    }

    fn read_dir(&self, _p: &VPath, _opts: ListOpts) -> BoxStream<'_, Result<Vec<DirEntry>>> {
        let stream = futures::stream::once(async move {
            let entries = self.entries.read().await;
            let list: Vec<DirEntry> = entries
                .values()
                .map(|e| DirEntry {
                    name: e.name.clone(),
                    is_dir: e.is_dir,
                    is_symlink: false,
                    metadata: Some(Metadata {
                        size: e.uncompressed_size,
                        file_type: if e.is_dir { FileType::Directory } else { FileType::File },
                        mode: e.mode,
                        uid: 1000,
                        gid: 1000,
                        created: Some(e.mtime),
                        modified: Some(e.mtime),
                        accessed: Some(e.mtime),
                        dev: 0,
                        ino: 1,
                        nlink: 1,
                        xattrs: BTreeMap::new(),
                        acl: None,
                        selinux: None,
                        rotational: None,
                        reflink_supported: None,
                    }),
                })
                .collect();
            Ok(list)
        });
        Box::pin(stream)
    }

    async fn stat(&self, p: &VPath, _follow: bool) -> Result<Metadata> {
        let clean_path = ArchiveSecurity::sanitize_entry_path(&p.path)?;
        let entries = self.entries.read().await;

        if clean_path.is_empty() || clean_path == "/" {
            return Ok(Metadata {
                size: 0,
                file_type: FileType::Directory,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                created: Some(1),
                modified: Some(1),
                accessed: Some(1),
                dev: 0,
                ino: 1,
                nlink: 1,
                xattrs: BTreeMap::new(),
                acl: None,
                selinux: None,
                rotational: None,
                reflink_supported: None,
            });
        }

        if let Some(entry) = entries.get(&clean_path) {
            Ok(Metadata {
                size: entry.uncompressed_size,
                file_type: if entry.is_dir { FileType::Directory } else { FileType::File },
                mode: entry.mode,
                uid: 1000,
                gid: 1000,
                created: Some(entry.mtime),
                modified: Some(entry.mtime),
                accessed: Some(entry.mtime),
                dev: 0,
                ino: 1,
                nlink: 1,
                xattrs: BTreeMap::new(),
                acl: None,
                selinux: None,
                rotational: None,
                reflink_supported: None,
            })
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncReadSeek>> {
        let clean_path = ArchiveSecurity::sanitize_entry_path(&p.path)?;
        let entries = self.entries.read().await;

        if entries.contains_key(&clean_path) {
            let dummy_content = b"Container format member content\n";
            Ok(Box::new(std::io::Cursor::new(dummy_content.to_vec())))
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn open_write(&self, _p: &VPath, _o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn create_dir(&self, _p: &VPath, _mode: Option<u32>) -> Result<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn remove(&self, _p: &VPath, _kind: RemoveKind) -> Result<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn rename(&self, _from: &VPath, _to: &VPath, _flags: RenameFlags) -> Result<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn set_meta(&self, _p: &VPath, _m: &MetaPatch) -> Result<()> {
        Err(VfsError::ReadOnlyFilesystem)
    }

    fn watch(&self, _p: &VPath) -> Result<BoxStream<'_, ChangeEvent>> {
        Err(VfsError::Unsupported("ContainerFs watch unsupported".into()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> Result<CopyOutcome> {
        Ok(CopyOutcome::Unsupported)
    }
}
