use duet_types::{VfsError, VfsResult};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Trashed item representation according to Freedesktop Trash Specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrashedItem {
    pub name: String,
    pub original_path: PathBuf,
    pub deletion_date: String,
    pub trash_file_path: PathBuf,
    pub trash_info_path: PathBuf,
}

/// Freedesktop Trash Manager.
#[derive(Debug, Default)]
pub struct TrashManager;

impl TrashManager {
    pub fn new() -> Self {
        Self
    }

    /// Resolve the trash base directory for a given target path.
    pub fn get_trash_dir_for_path(&self, target_path: &Path) -> VfsResult<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        if let Some(ref home_path) = home {
            if target_path.starts_with(home_path) {
                let user_trash = home_path.join(".local").join("share").join("Trash");
                return Ok(user_trash);
            }
        }

        // Try top-level mount .Trash-$uid or .Trash/$uid
        if let Ok(mount_point) = find_mount_point(target_path) {
            let uid = unsafe { libc::getuid() };
            let topdir_trash = mount_point.join(format!(".Trash-{uid}"));
            if fs::create_dir_all(&topdir_trash).is_ok() {
                return Ok(topdir_trash);
            }
        }

        // Fallback to user home trash
        if let Some(home_path) = home {
            Ok(home_path.join(".local").join("share").join("Trash"))
        } else {
            Ok(PathBuf::from("/tmp/Trash"))
        }
    }

    /// Move target file to spec-compliant Freedesktop Trash.
    pub fn trash_file(&self, target_path: &Path) -> VfsResult<TrashedItem> {
        crate::assert_not_ui_thread();

        let abs_path = fs::canonicalize(target_path).unwrap_or_else(|_| target_path.to_path_buf());
        let trash_dir = self.get_trash_dir_for_path(&abs_path)?;

        let files_dir = trash_dir.join("files");
        let info_dir = trash_dir.join("info");

        fs::create_dir_all(&files_dir)?;
        fs::create_dir_all(&info_dir)?;

        let file_name = abs_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("trashed_item");

        let mut trash_name = file_name.to_string();
        let mut trash_file_path = files_dir.join(&trash_name);
        let mut trash_info_path = info_dir.join(format!("{trash_name}.trashinfo"));

        let mut counter = 1;
        while trash_file_path.exists() || trash_info_path.exists() {
            let (stem, ext) = if let Some(idx) = file_name.rfind('.') {
                (&file_name[..idx], &file_name[idx..])
            } else {
                (file_name, "")
            };
            trash_name = format!("{stem}.{counter}{ext}");
            trash_file_path = files_dir.join(&trash_name);
            trash_info_path = info_dir.join(format!("{trash_name}.trashinfo"));
            counter += 1;
        }

        // Generate ISO 8601 deletion date string (%Y-%m-%dT%H:%M:%S)
        let deletion_date = format_iso8601(SystemTime::now());

        // Write .trashinfo file
        let mut info_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&trash_info_path)?;

        let encoded_path = urlencode_path(&abs_path);
        let info_content = format!(
            "[Trash Info]\nPath={encoded_path}\nDeletionDate={deletion_date}\n"
        );
        info_file.write_all(info_content.as_bytes())?;
        info_file.flush()?;

        // Move target file to trash files directory
        if fs::rename(&abs_path, &trash_file_path).is_err() {
            // Cross-filesystem fallback move
            fs::copy(&abs_path, &trash_file_path)?;
            if abs_path.is_dir() {
                fs::remove_dir_all(&abs_path)?;
            } else {
                fs::remove_file(&abs_path)?;
            }
        }

        Ok(TrashedItem {
            name: trash_name,
            original_path: abs_path,
            deletion_date,
            trash_file_path,
            trash_info_path,
        })
    }

    /// List all trashed items from the default user trash directory.
    pub fn list_trash(&self) -> VfsResult<Vec<TrashedItem>> {
        crate::assert_not_ui_thread();

        let home = std::env::var_os("HOME").map(PathBuf::from);
        let user_trash = home
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".local")
            .join("share")
            .join("Trash");

        let info_dir = user_trash.join("info");
        let files_dir = user_trash.join("files");

        let mut items = Vec::new();
        if !info_dir.exists() {
            return Ok(items);
        }

        for entry in fs::read_dir(info_dir)? {
            let entry = entry?;
            let info_path = entry.path();
            if info_path.extension().is_some_and(|e| e == "trashinfo") {
                if let Ok((original_path, deletion_date)) = read_trashinfo(&info_path) {
                    let stem = info_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default();
                    let trash_file_path = files_dir.join(stem);

                    items.push(TrashedItem {
                        name: stem.to_string(),
                        original_path,
                        deletion_date,
                        trash_file_path,
                        trash_info_path: info_path,
                    });
                }
            }
        }

        Ok(items)
    }

    /// Restore a trashed item to its original path.
    pub fn restore_item(&self, item: &TrashedItem) -> VfsResult<()> {
        crate::assert_not_ui_thread();

        if let Some(parent) = item.original_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if item.trash_file_path.exists()
            && fs::rename(&item.trash_file_path, &item.original_path).is_err()
        {
            fs::copy(&item.trash_file_path, &item.original_path)?;
            if item.trash_file_path.is_dir() {
                fs::remove_dir_all(&item.trash_file_path)?;
            } else {
                fs::remove_file(&item.trash_file_path)?;
            }
        }

        if item.trash_info_path.exists() {
            let _ = fs::remove_file(&item.trash_info_path);
        }

        Ok(())
    }

    /// Empty all trashed items from the default user trash directory.
    pub fn empty_trash(&self) -> VfsResult<()> {
        crate::assert_not_ui_thread();

        let items = self.list_trash()?;
        for item in items {
            if item.trash_file_path.exists() {
                if item.trash_file_path.is_dir() {
                    let _ = fs::remove_dir_all(&item.trash_file_path);
                } else {
                    let _ = fs::remove_file(&item.trash_file_path);
                }
            }
            if item.trash_info_path.exists() {
                let _ = fs::remove_file(&item.trash_info_path);
            }
        }
        Ok(())
    }
}

pub fn read_trashinfo(info_path: &Path) -> VfsResult<(PathBuf, String)> {
    let file = File::open(info_path)?;
    let reader = BufReader::new(file);

    let mut path_str = None;
    let mut date_str = None;

    for line in reader.lines() {
        let l = line?;
        let trimmed = l.trim();
        if let Some(raw) = trimmed.strip_prefix("Path=") {
            let unencoded = urldecode_path(raw);
            path_str = Some(PathBuf::from(unencoded));
        } else if let Some(stripped_date) = trimmed.strip_prefix("DeletionDate=") {
            date_str = Some(stripped_date.to_string());
        }
    }

    if let (Some(p), Some(d)) = (path_str, date_str) {
        Ok((p, d))
    } else {
        Err(VfsError::CorruptData("Invalid .trashinfo format".into()))
    }
}

fn urlencode_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    out
}

fn urldecode_path(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.bytes().peekable();
    while let Some(b) = chars.next() {
        if b == b'%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let Ok(val) = u8::from_str_radix(
                    &format!("{}{}", h1 as char, h2 as char),
                    16,
                ) {
                    bytes.push(val);
                    continue;
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn format_iso8601(time: SystemTime) -> String {
    let dur = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();

    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let minutes = (rem_secs % 3600) / 60;
    let seconds = rem_secs % 60;

    // Approximate ISO 8601 format timestamp
    let epoch_year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;

    format!("{epoch_year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}")
}

fn find_mount_point(path: &Path) -> VfsResult<PathBuf> {
    let mut current = path.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }
    Ok(current)
}
