extern crate libc;

use duet_types::{VPath, VfsError, VfsResult};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;

/// Copy strategy outcome enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyStrategyUsed {
    Reflink,
    CopyFileRange,
    SparseBuffered,
    FallbackBuffered,
}

/// Execute copy strategy ladder for local files:
/// 1. FICLONE reflink
/// 2. copy_file_range
/// 3. Sparse-aware fadvise(DONTNEED) buffered copy
pub fn execute_copy_strategy_ladder(
    src_vpath: &VPath,
    dst_vpath: &VPath,
    expected_size: u64,
) -> VfsResult<(CopyStrategyUsed, u64)> {
    if src_vpath.scheme != "file" || dst_vpath.scheme != "file" {
        return Err(VfsError::Unsupported(
            "Strategy ladder only applies to local file paths".into(),
        ));
    }

    let src_file = File::open(&src_vpath.path)?;

    if let Some(parent) = Path::new(&dst_vpath.path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&dst_vpath.path)?;

    let src_fd = src_file.as_raw_fd();
    let dst_fd = dst_file.as_raw_fd();

    // 1. Try FICLONE reflink
    #[cfg(target_os = "linux")]
    {
        // FICLONE ioctl code: _IOW('9', 9, int) = 0x40049409
        const FICLONE: libc::c_ulong = 0x40049409;
        let res = unsafe { libc::ioctl(dst_fd, FICLONE, src_fd) };
        if res == 0 {
            return Ok((CopyStrategyUsed::Reflink, expected_size));
        }
    }

    // 2. Try copy_file_range
    #[cfg(target_os = "linux")]
    {
        let mut total_copied = 0u64;
        let mut error = false;
        let chunk_size = 1024 * 1024 * 8; // 8 MiB chunks

        while total_copied < expected_size {
            let to_copy = (expected_size - total_copied).min(chunk_size);
            let ret = unsafe {
                libc::copy_file_range(
                    src_fd,
                    std::ptr::null_mut(),
                    dst_fd,
                    std::ptr::null_mut(),
                    to_copy as usize,
                    0,
                )
            };

            if ret > 0 {
                total_copied += ret as u64;
            } else if ret == 0 {
                break;
            } else {
                error = true;
                break;
            }
        }

        if !error && total_copied == expected_size {
            return Ok((CopyStrategyUsed::CopyFileRange, total_copied));
        }
    }

    // 3. Sparse-aware buffered copy with posix_fadvise(POSIX_FADV_DONTNEED)
    let copied = copy_sparse_buffered(src_fd, dst_fd, expected_size)?;
    Ok((CopyStrategyUsed::SparseBuffered, copied))
}

fn copy_sparse_buffered(src_fd: i32, dst_fd: i32, expected_size: u64) -> VfsResult<u64> {
    const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB buffer
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut offset: u64 = 0;

    #[cfg(target_os = "linux")]
    const SEEK_DATA: libc::c_int = 3;
    #[cfg(target_os = "linux")]
    const SEEK_HOLE: libc::c_int = 4;

    while offset < expected_size {
        #[cfg(target_os = "linux")]
        let data_start = unsafe { libc::lseek(src_fd, offset as libc::off_t, SEEK_DATA) };
        #[cfg(not(target_os = "linux"))]
        let data_start = -1;

        if data_start >= 0 && (data_start as u64) > offset {
            let hole_end = (data_start as u64).min(expected_size);
            // Seek dst to hole_end / truncate dst to extend hole
            unsafe {
                libc::lseek(dst_fd, hole_end as libc::off_t, libc::SEEK_SET);
            }
            offset = hole_end;
            if offset >= expected_size {
                break;
            }
        }

        #[cfg(target_os = "linux")]
        let hole_start = unsafe { libc::lseek(src_fd, offset as libc::off_t, SEEK_HOLE) };
        #[cfg(not(target_os = "linux"))]
        let hole_start = -1;

        let seg_end = if hole_start > 0 {
            (hole_start as u64).min(expected_size)
        } else {
            expected_size
        };

        // Copy segment from offset to seg_end
        let mut seg_pos = offset;
        // Seek src_fd and dst_fd to seg_pos
        unsafe {
            libc::lseek(src_fd, seg_pos as libc::off_t, libc::SEEK_SET);
            libc::lseek(dst_fd, seg_pos as libc::off_t, libc::SEEK_SET);
        }

        while seg_pos < seg_end {
            let to_read = ((seg_end - seg_pos) as usize).min(BUFFER_SIZE);
            let nread = unsafe { libc::read(src_fd, buffer.as_mut_ptr() as *mut libc::c_void, to_read) };
            if nread <= 0 {
                break;
            }

            let nwritten = unsafe { libc::write(dst_fd, buffer.as_ptr() as *const libc::c_void, nread as usize) };
            if nwritten <= 0 {
                return Err(VfsError::Io(std::io::Error::last_os_error()));
            }

            // Issue posix_fadvise DONTNEED on read portion of src file to avoid bloating page cache
            #[cfg(target_os = "linux")]
            unsafe {
                libc::posix_fadvise(
                    src_fd,
                    seg_pos as libc::off_t,
                    nwritten as libc::off_t,
                    libc::POSIX_FADV_DONTNEED,
                );
            }

            seg_pos += nwritten as u64;
        }

        offset = seg_end;
    }

    // Ensure final size matches expected size
    unsafe {
        libc::ftruncate(dst_fd, expected_size as libc::off_t);
    }

    Ok(offset)
}
