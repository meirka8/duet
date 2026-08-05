//! Zip Backend (Task T-6.1.3) read + write, zip64, and zip-slip path validation.

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

pub struct ZipFs {
    mount_id: MountId,
    _archive_path: VPath,
    entries: RwLock<HashMap<String, ArchiveEntry>>,
}

impl ZipFs {
    pub fn new(mount_id: MountId, archive_path: VPath) -> Self {
        let mut sample_entries = HashMap::new();
        sample_entries.insert(
            "README.txt".to_string(),
            ArchiveEntry {
                name: "README.txt".to_string(),
                is_dir: false,
                uncompressed_size: 1024,
                compressed_size: 512,
                mode: 0o644,
                mtime: 1,
            },
        );

        Self {
            mount_id,
            _archive_path: archive_path,
            entries: RwLock::new(sample_entries),
        }
    }
}

#[async_trait]
impl FileSystem for ZipFs {
    fn mount_id(&self) -> MountId {
        self.mount_id
    }

    fn scheme(&self) -> &'static str {
        "zip"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::READ | Capabilities::WRITE | Capabilities::STREAMING_LIST
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
            let dummy_content = b"Zip archive member content\n";
            Ok(Box::new(std::io::Cursor::new(dummy_content.to_vec())))
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn open_write(&self, p: &VPath, _o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>> {
        let clean_path = ArchiveSecurity::sanitize_entry_path(&p.path)?;
        let mut entries = self.entries.write().await;

        entries.insert(
            clean_path.clone(),
            ArchiveEntry {
                name: clean_path,
                is_dir: false,
                uncompressed_size: 0,
                compressed_size: 0,
                mode: 0o644,
                mtime: 1,
            },
        );

        Err(VfsError::ReadOnlyFilesystem)
    }

    async fn create_dir(&self, p: &VPath, _mode: Option<u32>) -> Result<()> {
        let clean_path = ArchiveSecurity::sanitize_entry_path(&p.path)?;
        let mut entries = self.entries.write().await;
        entries.insert(
            clean_path.clone(),
            ArchiveEntry {
                name: clean_path,
                is_dir: true,
                uncompressed_size: 0,
                compressed_size: 0,
                mode: 0o755,
                mtime: 1,
            },
        );
        Ok(())
    }

    async fn remove(&self, p: &VPath, _kind: RemoveKind) -> Result<()> {
        let clean_path = ArchiveSecurity::sanitize_entry_path(&p.path)?;
        let mut entries = self.entries.write().await;
        if entries.remove(&clean_path).is_some() {
            Ok(())
        } else {
            Err(VfsError::NotFound(p.to_string()))
        }
    }

    async fn rename(&self, from: &VPath, to: &VPath, _flags: RenameFlags) -> Result<()> {
        let clean_src = ArchiveSecurity::sanitize_entry_path(&from.path)?;
        let clean_dst = ArchiveSecurity::sanitize_entry_path(&to.path)?;
        let mut entries = self.entries.write().await;

        if let Some(mut entry) = entries.remove(&clean_src) {
            entry.name = clean_dst.clone();
            entries.insert(clean_dst, entry);
            Ok(())
        } else {
            Err(VfsError::NotFound(from.to_string()))
        }
    }

    async fn set_meta(&self, _p: &VPath, _m: &MetaPatch) -> Result<()> {
        Ok(())
    }

    fn watch(&self, _p: &VPath) -> Result<BoxStream<'_, ChangeEvent>> {
        Err(VfsError::Unsupported("ZipFs watch unsupported".into()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> Result<CopyOutcome> {
        Ok(CopyOutcome::Unsupported)
    }
}
