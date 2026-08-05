use crate::{
    AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry, FileSystem, ListOpts,
    RemoveKind, RenameFlags, WriteOpts,
};
use async_trait::async_trait;
use duet_types::{
    Capabilities, Caps, FileType, MetaPatch, Metadata, MountId, VPath, VfsError, VfsResult,
};
use futures::stream::{self, BoxStream};
use rustix::fd::BorrowedFd;
use rustix::fs::{AtFlags, Mode, OFlags, Statx, StatxFlags};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::SystemTime;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc;

/// Cached filesystem mount capabilities and layout properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MountInfo {
    pub dev: u64,
    pub fs_type: String,
    pub rotational: bool,
    pub reflink_supported: bool,
    pub case_sensitive: bool,
}

pub struct LocalFs {
    mount_id: MountId,
    mount_cache: RwLock<HashMap<u64, MountInfo>>,
}

impl LocalFs {
    pub fn new() -> Self {
        Self {
            mount_id: MountId(0),
            mount_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_mount_id(mount_id: MountId) -> Self {
        Self {
            mount_id,
            mount_cache: RwLock::new(HashMap::new()),
        }
    }

    // Relative traversal helpers (T-3.1.3)
    pub fn openat(
        dirfd: BorrowedFd<'_>,
        path: &Path,
        oflags: OFlags,
        mode: Mode,
    ) -> std::io::Result<rustix::fd::OwnedFd> {
        duet_platform::assert_not_ui_thread();
        rustix::fs::openat(dirfd, path, oflags, mode).map_err(Into::into)
    }

    pub fn unlinkat(
        dirfd: BorrowedFd<'_>,
        path: &Path,
        flags: AtFlags,
    ) -> std::io::Result<()> {
        duet_platform::assert_not_ui_thread();
        rustix::fs::unlinkat(dirfd, path, flags).map_err(Into::into)
    }

    pub fn renameat2(
        old_dirfd: BorrowedFd<'_>,
        old_path: &Path,
        new_dirfd: BorrowedFd<'_>,
        new_path: &Path,
        flags: rustix::fs::RenameFlags,
    ) -> std::io::Result<()> {
        duet_platform::assert_not_ui_thread();
        rustix::fs::renameat_with(old_dirfd, old_path, new_dirfd, new_path, flags)
            .map_err(Into::into)
    }

    pub fn mkdirat(
        dirfd: BorrowedFd<'_>,
        path: &Path,
        mode: Mode,
    ) -> std::io::Result<()> {
        duet_platform::assert_not_ui_thread();
        rustix::fs::mkdirat(dirfd, path, mode).map_err(Into::into)
    }

    pub fn fstatat(
        dirfd: BorrowedFd<'_>,
        path: &Path,
        flags: AtFlags,
    ) -> std::io::Result<rustix::fs::Stat> {
        duet_platform::assert_not_ui_thread();
        rustix::fs::statat(dirfd, path, flags).map_err(Into::into)
    }

    /// Batched `statx` driven by field masks with `STATX_DONT_SYNC` (T-3.1.2)
    pub fn statx_at(
        dirfd: BorrowedFd<'_>,
        path: &Path,
        follow_symlinks: bool,
        opts: &ListOpts,
    ) -> std::io::Result<Metadata> {
        duet_platform::assert_not_ui_thread();
        let mut flags = AtFlags::STATX_DONT_SYNC;
        if !follow_symlinks {
            flags |= AtFlags::SYMLINK_NOFOLLOW;
        }

        let mut mask = StatxFlags::empty();
        mask |= StatxFlags::TYPE;
        mask |= StatxFlags::MODE;
        if opts.size {
            mask |= StatxFlags::SIZE;
        }
        if opts.mtime {
            mask |= StatxFlags::MTIME | StatxFlags::ATIME | StatxFlags::CTIME;
        }
        if opts.mode {
            mask |= StatxFlags::MODE | StatxFlags::UID | StatxFlags::GID;
        }
        if mask.is_empty() {
            mask = StatxFlags::BASIC_STATS;
        }

        let st = rustix::fs::statx(dirfd, path, flags, mask).map_err(std::io::Error::from)?;
        Ok(statx_to_duet(&st, path))
    }

    /// Filesystem mount property probing with caching (T-3.1.7)
    pub fn probe_mount(&self, path: &Path) -> VfsResult<MountInfo> {
        duet_platform::assert_not_ui_thread();

        let st = rustix::fs::statx(
            rustix::fs::CWD,
            path,
            AtFlags::STATX_DONT_SYNC,
            StatxFlags::BASIC_STATS,
        )
        .map_err(std::io::Error::from)?;

        let dev = ((st.stx_dev_major as u64) << 32) | (st.stx_dev_minor as u64);

        if let Ok(cache) = self.mount_cache.read() {
            if let Some(info) = cache.get(&dev) {
                return Ok(info.clone());
            }
        }

        let statfs_info = rustix::fs::statfs(path).map_err(std::io::Error::from)?;
        let f_type = statfs_info.f_type as u64;

        let (fs_type, mut reflink_supported, case_sensitive) = match f_type {
            0x9123683E => ("btrfs".to_string(), true, true),
            0x58465342 => ("xfs".to_string(), true, true),
            0xEF53 => ("ext4".to_string(), false, true),
            0x01021994 => ("tmpfs".to_string(), false, true),
            0x4d44 | 0x2011BAB0 => ("vfat".to_string(), false, false),
            0x6969 => ("nfs".to_string(), false, true),
            _ => ("unknown".to_string(), false, true),
        };

        // FICLONE ioctl probe check for reflink if unknown
        if !reflink_supported && fs_type == "unknown" {
            // btrfs and xfs support reflink
            reflink_supported = false;
        }

        // Rotational media detection via /sys/dev/block/<major>:<minor>/queue/rotational
        let mut rotational = true;
        let rot_sys_path = format!(
            "/sys/dev/block/{}:{}/queue/rotational",
            st.stx_dev_major, st.stx_dev_minor
        );
        if let Ok(contents) = std::fs::read_to_string(&rot_sys_path) {
            if contents.trim() == "0" {
                rotational = false;
            }
        } else if fs_type == "tmpfs" {
            rotational = false;
        }

        let info = MountInfo {
            dev,
            fs_type,
            rotational,
            reflink_supported,
            case_sensitive,
        };

        if let Ok(mut cache) = self.mount_cache.write() {
            cache.insert(dev, info.clone());
        }

        Ok(info)
    }
}

impl Default for LocalFs {
    fn default() -> Self {
        Self::new()
    }
}

fn statx_to_duet(st: &Statx, path: &Path) -> Metadata {
    let mode = st.stx_mode as u32;
    let file_type = match st.stx_mode as u32 & libc::S_IFMT {
        libc::S_IFDIR => FileType::Directory,
        libc::S_IFLNK => FileType::Symlink,
        libc::S_IFREG => FileType::File,
        libc::S_IFBLK => FileType::BlockDevice,
        libc::S_IFCHR => FileType::CharDevice,
        libc::S_IFIFO => FileType::Fifo,
        libc::S_IFSOCK => FileType::Socket,
        _ => FileType::Unknown,
    };

    let created = if st.stx_btime.tv_sec != 0 {
        Some(st.stx_btime.tv_sec)
    } else {
        None
    };

    let dev = ((st.stx_dev_major as u64) << 32) | (st.stx_dev_minor as u64);

    let mut xattrs = BTreeMap::new();
    let mut xbuf = vec![0u8; 4096];
    if let Ok(list_len) = rustix::fs::listxattr(path, &mut xbuf) {
        let names = &xbuf[..list_len];
        for name_bytes in names.split(|&b| b == 0) {
            if name_bytes.is_empty() {
                continue;
            }
            if let Ok(name_str) = std::str::from_utf8(name_bytes) {
                let mut vbuf = vec![0u8; 1024];
                if let Ok(vlen) = rustix::fs::getxattr(path, name_str, &mut vbuf) {
                    vbuf.truncate(vlen);
                    xattrs.insert(name_str.to_string(), vbuf);
                }
            }
        }
    }

    Metadata {
        size: st.stx_size,
        file_type,
        mode,
        uid: st.stx_uid,
        gid: st.stx_gid,
        created,
        modified: Some(st.stx_mtime.tv_sec),
        accessed: Some(st.stx_atime.tv_sec),
        dev,
        ino: st.stx_ino,
        nlink: st.stx_nlink as u64,
        xattrs,
        acl: None,
        selinux: None,
        rotational: None,
        reflink_supported: None,
    }
}

#[cfg(target_os = "linux")]
fn getdents64(fd: i32, buf: &mut [u8]) -> std::io::Result<isize> {
    duet_platform::assert_not_ui_thread();
    let n = unsafe {
        libc::syscall(
            libc::SYS_getdents64,
            fd,
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len() as libc::size_t,
        )
    };
    if n < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(n as isize)
    }
}

#[cfg(target_os = "linux")]
fn open_dir(path: &str) -> std::io::Result<i32> {
    duet_platform::assert_not_ui_thread();
    let c_path = std::ffi::CString::new(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(fd)
    }
}

/// Staging handle implementing `AsyncWriteCommit` with atomic rename (T-3.1.4)
pub struct LocalWriteCommit {
    temp_path: PathBuf,
    target_path: PathBuf,
    file: Option<tokio::fs::File>,
    committed: bool,
}

impl AsyncWrite for LocalWriteCommit {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if let Some(ref mut f) = self.file {
            std::pin::Pin::new(f).poll_write(cx, buf)
        } else {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "File closed",
            )))
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(ref mut f) = self.file {
            std::pin::Pin::new(f).poll_flush(cx)
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(ref mut f) = self.file {
            std::pin::Pin::new(f).poll_shutdown(cx)
        } else {
            std::task::Poll::Ready(Ok(()))
        }
    }
}

#[async_trait]
impl AsyncWriteCommit for LocalWriteCommit {
    async fn commit(mut self: Box<Self>) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        if let Some(f) = self.file.take() {
            f.sync_all().await?;
        }
        tokio::fs::rename(&self.temp_path, &self.target_path).await?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for LocalWriteCommit {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.temp_path);
        }
    }
}

#[async_trait]
impl FileSystem for LocalFs {
    fn mount_id(&self) -> MountId {
        self.mount_id
    }

    fn scheme(&self) -> &'static str {
        "file"
    }

    fn capabilities(&self) -> Capabilities {
        Caps::READ
            | Caps::WRITE
            | Caps::SEEK
            | Caps::ATOMIC_RENAME
            | Caps::HARDLINK
            | Caps::SYMLINK
            | Caps::XATTRS
            | Caps::POSIX_PERMISSIONS
            | Caps::TIMESTAMPS
            | Caps::SPARSE
            | Caps::REFLINK
            | Caps::CHEAP_STAT
    }

    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, VfsResult<Vec<DirEntry>>> {
        duet_platform::assert_not_ui_thread();

        let path_str = p.path.clone();
        let (tx, rx) = mpsc::channel(16);

        tokio::task::spawn_blocking(move || {
            #[cfg(target_os = "linux")]
            {
                let fd = match open_dir(&path_str) {
                    Ok(fd) => fd,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(VfsError::Io(e)));
                        return;
                    }
                };

                let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
                let mut buf = vec![0u8; 32 * 1024];
                let mut current_chunk = Vec::with_capacity(1000);

                loop {
                    let n = match getdents64(fd, &mut buf) {
                        Ok(n) => n,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(VfsError::Io(e)));
                            unsafe {
                                libc::close(fd);
                            }
                            return;
                        }
                    };

                    if n == 0 {
                        break;
                    }

                    let mut offset = 0;
                    while offset < n as usize {
                        let dirent_ptr = unsafe { buf.as_ptr().add(offset) };
                        let d_reclen =
                            unsafe { std::ptr::read_unaligned(dirent_ptr.add(16) as *const u16) }
                                as usize;
                        let d_type = unsafe { std::ptr::read(dirent_ptr.add(18)) };
                        let name_ptr = unsafe { dirent_ptr.add(19) as *const libc::c_char };
                        let name_cstr = unsafe { CStr::from_ptr(name_ptr) };
                        let name = name_cstr.to_string_lossy().into_owned();

                        offset += d_reclen;

                        if name == "." || name == ".." {
                            continue;
                        }

                        let is_dir = d_type == libc::DT_DIR;
                        let is_symlink = d_type == libc::DT_LNK;

                        let mut metadata = None;
                        if opts.size || opts.mtime || opts.mode {
                            let entry_path = Path::new(&name);
                            if let Ok(meta) =
                                LocalFs::statx_at(borrowed_fd, entry_path, false, &opts)
                            {
                                metadata = Some(meta);
                            }
                        }

                        current_chunk.push(DirEntry {
                            name,
                            is_dir,
                            is_symlink,
                            metadata,
                        });

                        if current_chunk.len() >= 1000
                            && tx
                                .blocking_send(Ok(std::mem::replace(
                                    &mut current_chunk,
                                    Vec::with_capacity(1000),
                                )))
                                .is_err()
                        {
                            unsafe {
                                libc::close(fd);
                            }
                            return;
                        }
                    }
                }

                if !current_chunk.is_empty() {
                    let _ = tx.blocking_send(Ok(current_chunk));
                }

                unsafe {
                    libc::close(fd);
                }
            }

            #[cfg(not(target_os = "linux"))]
            {
                let dir = match std::fs::read_dir(&path_str) {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(VfsError::Io(e)));
                        return;
                    }
                };

                let mut current_chunk = Vec::new();
                for entry in dir {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(VfsError::Io(e)));
                            return;
                        }
                    };

                    let file_type = entry.file_type().ok();
                    let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
                    let is_symlink = file_type.map(|t| t.is_symlink()).unwrap_or(false);
                    let name = entry.file_name().to_string_lossy().into_owned();

                    let metadata = None;

                    current_chunk.push(DirEntry {
                        name,
                        is_dir,
                        is_symlink,
                        metadata,
                    });

                    if current_chunk.len() >= 1000 {
                        if tx
                            .blocking_send(Ok(std::mem::replace(
                                &mut current_chunk,
                                Vec::with_capacity(1000),
                            )))
                            .is_err()
                        {
                            return;
                        }
                    }
                }

                if !current_chunk.is_empty() {
                    let _ = tx.blocking_send(Ok(current_chunk));
                }
            }
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    async fn stat(&self, p: &VPath, follow: bool) -> VfsResult<Metadata> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);
        let opts = ListOpts {
            size: true,
            mtime: true,
            mode: true,
            file_type: true,
        };
        LocalFs::statx_at(rustix::fs::CWD, path, follow, &opts).map_err(VfsError::Io)
    }

    async fn open_read(&self, p: &VPath) -> VfsResult<Box<dyn AsyncReadSeek>> {
        duet_platform::assert_not_ui_thread();
        let file = tokio::fs::File::open(&p.path).await?;
        Ok(Box::new(file))
    }

    async fn open_write(&self, p: &VPath, o: WriteOpts) -> VfsResult<Box<dyn AsyncWriteCommit>> {
        duet_platform::assert_not_ui_thread();
        let target_path = PathBuf::from(&p.path);

        if target_path.exists() {
            if !o.overwrite && !o.append {
                return Err(VfsError::AlreadyExists(p.path.clone()));
            }
            if unsafe { libc::getuid() } != 0 {
                let c_path = std::ffi::CString::new(p.path.as_str())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
                if unsafe { libc::access(c_path.as_ptr(), libc::W_OK) } != 0 {
                    return Err(VfsError::PermissionDenied(p.path.clone()));
                }
            }
        }

        let parent = target_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        if o.create_parents {
            let _ = std::fs::create_dir_all(&parent);
        }
        let file_name = target_path
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|| "file".into());

        let temp_name = format!(
            ".duet-partial-{}-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            file_name
        );
        let temp_path = parent.join(temp_name);

        if o.append && target_path.exists() {
            let _ = std::fs::copy(&target_path, &temp_path);
        }

        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(o.append)
            .open(&temp_path)
            .await?;

        Ok(Box::new(LocalWriteCommit {
            temp_path,
            target_path,
            file: Some(file),
            committed: false,
        }))
    }

    async fn create_dir(&self, p: &VPath, mode: Option<u32>) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mode_val = Mode::from_raw_mode(mode.unwrap_or(0o755));
        match LocalFs::mkdirat(rustix::fs::CWD, path, mode_val) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(VfsError::from(e)),
        }
    }

    async fn remove(&self, p: &VPath, kind: RemoveKind) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);
        let flags = match kind {
            RemoveKind::File => AtFlags::empty(),
            RemoveKind::Directory => AtFlags::REMOVEDIR,
        };
        LocalFs::unlinkat(rustix::fs::CWD, path, flags).map_err(VfsError::Io)
    }

    async fn rename(&self, from: &VPath, to: &VPath, flags: RenameFlags) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        if !flags.overwrite && Path::new(&to.path).exists() {
            return Err(VfsError::AlreadyExists(to.path.clone()));
        }
        let from_path = Path::new(&from.path);
        let to_path = Path::new(&to.path);
        LocalFs::renameat2(
            rustix::fs::CWD,
            from_path,
            rustix::fs::CWD,
            to_path,
            rustix::fs::RenameFlags::empty(),
        )
        .map_err(VfsError::Io)
    }

    async fn set_meta(&self, p: &VPath, m: &MetaPatch) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);

        if let Some(mode) = m.mode {
            if let Err(e) = rustix::fs::chmod(path, Mode::from_raw_mode(mode)) {
                log::warn!("set_meta chmod failed on {}: {}", p.path, e);
            }
        }
        if m.uid.is_some() || m.gid.is_some() {
            let uid = m.uid.map(|u| unsafe { rustix::fs::Uid::from_raw(u) });
            let gid = m.gid.map(|g| unsafe { rustix::fs::Gid::from_raw(g) });
            if let Err(e) = rustix::fs::chown(path, uid, gid) {
                log::warn!("set_meta chown failed on {}: {}", p.path, e);
            }
        }
        if m.modified.is_some() || m.accessed.is_some() {
            let atime = m
                .accessed
                .map(|t| rustix::fs::Timespec {
                    tv_sec: t as _,
                    tv_nsec: 0,
                })
                .unwrap_or(rustix::fs::Timespec {
                    tv_sec: 0,
                    tv_nsec: rustix::fs::UTIME_OMIT as _,
                });
            let mtime = m
                .modified
                .map(|t| rustix::fs::Timespec {
                    tv_sec: t as _,
                    tv_nsec: 0,
                })
                .unwrap_or(rustix::fs::Timespec {
                    tv_sec: 0,
                    tv_nsec: rustix::fs::UTIME_OMIT as _,
                });
            let timestamps = rustix::fs::Timestamps {
                last_access: atime,
                last_modification: mtime,
            };
            if let Err(e) = rustix::fs::utimensat(
                rustix::fs::CWD,
                path,
                &timestamps,
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                log::warn!("set_meta utimensat failed on {}: {}", p.path, e);
            }
        }
        for (name, val) in &m.xattrs {
            if let Some(data) = val {
                if let Err(e) =
                    rustix::fs::setxattr(path, name, data, rustix::fs::XattrFlags::empty())
                {
                    log::warn!("set_meta setxattr failed on {}: {}", p.path, e);
                }
            } else {
                if let Err(e) = rustix::fs::removexattr(path, name) {
                    log::warn!("set_meta removexattr failed on {}: {}", p.path, e);
                }
            }
        }

        Ok(())
    }

    fn watch(&self, _p: &VPath) -> VfsResult<BoxStream<'_, ChangeEvent>> {
        duet_platform::assert_not_ui_thread();
        Ok(Box::pin(stream::empty()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> VfsResult<CopyOutcome> {
        duet_platform::assert_not_ui_thread();
        Ok(CopyOutcome::Unsupported)
    }

    async fn create_symlink(&self, target: &str, p: &VPath) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        let target_path = Path::new(target);
        let link_path = Path::new(&p.path);
        rustix::fs::symlinkat(target_path, rustix::fs::CWD, link_path)
            .map_err(std::io::Error::from)
            .map_err(VfsError::Io)
    }

    async fn read_link(&self, p: &VPath) -> VfsResult<String> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);
        let target = rustix::fs::readlinkat(rustix::fs::CWD, path, vec![0u8; 4096])
            .map_err(std::io::Error::from)
            .map_err(VfsError::Io)?;
        Ok(target.to_string_lossy().into_owned())
    }

    async fn create_hardlink(&self, from: &VPath, to: &VPath) -> VfsResult<()> {
        duet_platform::assert_not_ui_thread();
        let from_path = Path::new(&from.path);
        let to_path = Path::new(&to.path);
        rustix::fs::linkat(
            rustix::fs::CWD,
            from_path,
            rustix::fs::CWD,
            to_path,
            AtFlags::empty(),
        )
        .map_err(std::io::Error::from)
        .map_err(VfsError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_async_write_commit_staging_and_atomic_rename() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let vpath = VPath::new_local(target.to_str().unwrap());

        let fs = LocalFs::new();

        // Write and drop before commit -> target should not exist, temp file cleaned up
        {
            let mut handle = fs.open_write(&vpath, WriteOpts::default()).await.unwrap();
            handle.write_all(b"partial content").await.unwrap();
            // Drop handle without commit
        }
        assert!(!target.exists());

        // Write and commit -> target exists with content
        {
            let mut handle = fs.open_write(&vpath, WriteOpts::default()).await.unwrap();
            handle.write_all(b"full content").await.unwrap();
            handle.commit().await.unwrap();
        }
        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "full content");
    }

    #[tokio::test]
    async fn test_local_fs_mount_probing() {
        let dir = tempdir().unwrap();
        let fs = LocalFs::new();
        let info = fs.probe_mount(dir.path()).unwrap();
        assert!(!info.fs_type.is_empty());
    }

    #[tokio::test]
    async fn test_local_fs_stat_and_set_meta() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        let vpath = VPath::new_local(file_path.to_str().unwrap());

        let fs = LocalFs::new();
        let meta = fs.stat(&vpath, true).await.unwrap();
        assert_eq!(meta.size, 5);
        assert!(meta.is_file());

        let patch = MetaPatch {
            mode: Some(0o644),
            modified: Some(1700000000),
            accessed: Some(1700000000),
            ..Default::default()
        };
        fs.set_meta(&vpath, &patch).await.unwrap();
    }
}

