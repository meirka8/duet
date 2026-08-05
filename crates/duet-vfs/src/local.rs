use crate::{
    AsyncReadSeek, AsyncWriteCommit, ChangeEvent, CopyOutcome, DirEntry, FileSystem, ListOpts,
    RemoveKind, RenameFlags, WriteOpts,
};
use async_trait::async_trait;
use duet_types::{Caps, Error, MetaPatch, Metadata, Result, VPath};
use futures::stream::{self, BoxStream};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use tokio::sync::mpsc;

pub struct LocalFs;

impl LocalFs {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFs {
    fn default() -> Self {
        Self::new()
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

#[async_trait]
impl FileSystem for LocalFs {
    fn scheme(&self) -> &'static str {
        "file"
    }

    fn caps(&self) -> Caps {
        Caps::RANDOM_READ
            | Caps::RANDOM_WRITE
            | Caps::RENAME
            | Caps::ATOMIC_REPLACE
            | Caps::HARDLINK
            | Caps::SYMLINK
            | Caps::XATTR
            | Caps::PERMISSIONS
            | Caps::TIMESTAMPS
            | Caps::SPARSE
            | Caps::REFLINK
            | Caps::CHEAP_STAT
    }

    fn read_dir(&self, p: &VPath, opts: ListOpts) -> BoxStream<'_, Result<Vec<DirEntry>>> {
        duet_platform::assert_not_ui_thread();

        let path_str = p.path.clone();
        let (tx, rx) = mpsc::channel(16);

        // Run the blocking dir listing in spawn_blocking
        tokio::task::spawn_blocking(move || {
            #[cfg(target_os = "linux")]
            {
                let fd = match open_dir(&path_str) {
                    Ok(fd) => fd,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Error::Io(e)));
                        return;
                    }
                };

                let mut buf = vec![0u8; 32 * 1024];
                let mut current_chunk = Vec::with_capacity(1000);

                loop {
                    let n = match getdents64(fd, &mut buf) {
                        Ok(n) => n,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(Error::Io(e)));
                            unsafe {
                                libc::close(fd);
                            }
                            return;
                        }
                    };

                    if n == 0 {
                        break; // EOF
                    }

                    let mut offset = 0;
                    while offset < n as usize {
                        let dirent_ptr = unsafe { buf.as_ptr().add(offset) };

                        let d_reclen =
                            unsafe { std::ptr::read_unaligned(dirent_ptr.add(16) as *const u16) }
                                as usize;

                        let d_type = unsafe { std::ptr::read(dirent_ptr.add(18)) };

                        let name_ptr = unsafe { dirent_ptr.add(19) as *const libc::c_char };
                        let name_cstr = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                        let name = name_cstr.to_string_lossy().into_owned();

                        offset += d_reclen;

                        if name == "." || name == ".." {
                            continue;
                        }

                        let is_dir = d_type == libc::DT_DIR;
                        let is_symlink = d_type == libc::DT_LNK;

                        let mut metadata = None;
                        if opts.size || opts.mtime || opts.mode {
                            // If any specific field is requested, fetch full metadata
                            let entry_path = Path::new(&path_str).join(&name);
                            if let Ok(meta) = std::fs::metadata(&entry_path) {
                                metadata = Some(Metadata {
                                    is_dir: meta.is_dir(),
                                    is_symlink: meta.file_type().is_symlink(),
                                    size: meta.len(),
                                    mtime: meta.mtime(),
                                    atime: meta.atime(),
                                    mode: meta.mode(),
                                    nlink: meta.nlink(),
                                    dev: meta.dev(),
                                    ino: meta.ino(),
                                });
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
                // Fallback for non-Linux platforms
                let dir = match std::fs::read_dir(&path_str) {
                    Ok(d) => d,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Error::Io(e)));
                        return;
                    }
                };

                let mut current_chunk = Vec::new();
                for entry in dir {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            let _ = tx.blocking_send(Err(Error::Io(e)));
                            return;
                        }
                    };

                    let file_type = entry.file_type().ok();
                    let is_dir = file_type.map(|t| t.is_dir()).unwrap_or(false);
                    let is_symlink = file_type.map(|t| t.is_symlink()).unwrap_or(false);
                    let name = entry.file_name().to_string_lossy().into_owned();

                    let mut metadata = None;
                    if opts.size || opts.mtime || opts.mode {
                        if let Ok(meta) = entry.metadata() {
                            metadata = Some(Metadata {
                                is_dir: meta.is_dir(),
                                is_symlink: meta.file_type().is_symlink(),
                                size: meta.len(),
                                mtime: 0, // stub or OS specific
                                atime: 0,
                                mode: 0,
                                nlink: 1,
                                dev: 0,
                                ino: 0,
                            });
                        }
                    }

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

        // Convert the receiver into a stream
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    async fn stat(&self, p: &VPath, _follow: bool) -> Result<Metadata> {
        duet_platform::assert_not_ui_thread();
        let path = Path::new(&p.path);
        let meta = std::fs::metadata(path)?;
        Ok(Metadata {
            is_dir: meta.is_dir(),
            is_symlink: meta.file_type().is_symlink(),
            size: meta.len(),
            mtime: meta.mtime(),
            atime: meta.atime(),
            mode: meta.mode(),
            nlink: meta.nlink(),
            dev: meta.dev(),
            ino: meta.ino(),
        })
    }

    async fn open_read(&self, p: &VPath) -> Result<Box<dyn AsyncReadSeek>> {
        duet_platform::assert_not_ui_thread();
        let file = tokio::fs::File::open(&p.path).await?;
        Ok(Box::new(file))
    }

    async fn open_write(&self, p: &VPath, _o: WriteOpts) -> Result<Box<dyn AsyncWriteCommit>> {
        duet_platform::assert_not_ui_thread();
        // Simple stub write commit wrapping tokio file
        struct LocalWriteCommit {
            file: tokio::fs::File,
        }

        impl tokio::io::AsyncWrite for LocalWriteCommit {
            fn poll_write(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
                buf: &[u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                std::pin::Pin::new(&mut self.file).poll_write(cx, buf)
            }

            fn poll_flush(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::pin::Pin::new(&mut self.file).poll_flush(cx)
            }

            fn poll_shutdown(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::pin::Pin::new(&mut self.file).poll_shutdown(cx)
            }
        }

        #[async_trait]
        impl AsyncWriteCommit for LocalWriteCommit {
            async fn commit(self: Box<Self>) -> Result<()> {
                Ok(())
            }
        }

        let file = tokio::fs::File::create(&p.path).await?;
        Ok(Box::new(LocalWriteCommit { file }))
    }

    async fn create_dir(&self, p: &VPath, _mode: Option<u32>) -> Result<()> {
        duet_platform::assert_not_ui_thread();
        tokio::fs::create_dir(&p.path).await?;
        Ok(())
    }

    async fn remove(&self, p: &VPath, kind: RemoveKind) -> Result<()> {
        duet_platform::assert_not_ui_thread();
        match kind {
            RemoveKind::File => tokio::fs::remove_file(&p.path).await?,
            RemoveKind::Directory => tokio::fs::remove_dir(&p.path).await?,
        }
        Ok(())
    }

    async fn rename(&self, from: &VPath, to: &VPath, _flags: RenameFlags) -> Result<()> {
        duet_platform::assert_not_ui_thread();
        tokio::fs::rename(&from.path, &to.path).await?;
        Ok(())
    }

    async fn set_meta(&self, _p: &VPath, _m: &MetaPatch) -> Result<()> {
        duet_platform::assert_not_ui_thread();
        Ok(())
    }

    fn watch(&self, _p: &VPath) -> Result<BoxStream<'_, ChangeEvent>> {
        duet_platform::assert_not_ui_thread();
        Ok(Box::pin(stream::empty()))
    }

    async fn server_side_copy(&self, _from: &VPath, _to: &VPath) -> Result<CopyOutcome> {
        duet_platform::assert_not_ui_thread();
        Ok(CopyOutcome::Unsupported)
    }
}
