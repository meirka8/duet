use duet_types::{VfsError, VfsResult};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// Detect MIME type based on file extension and magic byte sniffing fallback.
pub fn detect_mime_type(path: &Path) -> String {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext_lower = ext.to_lowercase();
        match ext_lower.as_str() {
            "txt" | "md" | "markdown" | "log" => return "text/plain".to_string(),
            "html" | "htm" => return "text/html".to_string(),
            "css" => return "text/css".to_string(),
            "js" | "mjs" => return "application/javascript".to_string(),
            "json" => return "application/json".to_string(),
            "toml" => return "application/toml".to_string(),
            "xml" => return "application/xml".to_string(),
            "png" => return "image/png".to_string(),
            "jpg" | "jpeg" => return "image/jpeg".to_string(),
            "gif" => return "image/gif".to_string(),
            "svg" => return "image/svg+xml".to_string(),
            "webp" => return "image/webp".to_string(),
            "bmp" => return "image/bmp".to_string(),
            "pdf" => return "application/pdf".to_string(),
            "zip" => return "application/zip".to_string(),
            "gz" | "tgz" => return "application/gzip".to_string(),
            "bz2" => return "application/x-bzip2".to_string(),
            "xz" => return "application/x-xz".to_string(),
            "zst" => return "application/zstd".to_string(),
            "7z" => return "application/x-7z-compressed".to_string(),
            "tar" => return "application/x-tar".to_string(),
            "mp4" => return "video/mp4".to_string(),
            "webm" => return "video/webm".to_string(),
            "mkv" => return "video/x-matroska".to_string(),
            "mp3" => return "audio/mpeg".to_string(),
            "wav" => return "audio/wav".to_string(),
            "ogg" => return "audio/ogg".to_string(),
            "rs" | "c" | "cpp" | "h" | "py" | "sh" | "bash" => return "text/x-source-code".to_string(),
            "deb" => return "application/vnd.debian.binary-package".to_string(),
            "rpm" => return "application/x-rpm".to_string(),
            _ => {}
        }
    }

    // Sniff magic bytes if file exists
    if path.is_file() {
        if let Ok(mut file) = File::open(path) {
            let mut header = [0u8; 16];
            if let Ok(n) = file.read(&mut header) {
                if n >= 4 {
                    if &header[..4] == b"\x7fELF" {
                        return "application/x-executable".to_string();
                    }
                    if &header[..4] == b"%PDF" {
                        return "application/pdf".to_string();
                    }
                    if &header[..4] == b"\x89PNG" {
                        return "image/png".to_string();
                    }
                    if &header[..3] == b"\xff\xd8\xff" {
                        return "image/jpeg".to_string();
                    }
                    if &header[..4] == b"GIF8" {
                        return "image/gif".to_string();
                    }
                    if &header[..4] == b"PK\x03\x04" {
                        return "application/zip".to_string();
                    }
                    if &header[..2] == b"\x1f\x8b" {
                        return "application/gzip".to_string();
                    }
                    if &header[..3] == b"\x42\x5a\x68" {
                        return "application/x-bzip2".to_string();
                    }
                    if &header[..6] == b"\xfd7zXZ\x00" {
                        return "application/x-xz".to_string();
                    }
                    if &header[..6] == b"7z\xbc\xaf\x27\x1c" {
                        return "application/x-7z-compressed".to_string();
                    }
                    if &header[..2] == b"BM" {
                        return "image/bmp".to_string();
                    }
                }
            }
        }
    }

    "application/octet-stream".to_string()
}

/// Launch application with XDG field code replacement (`%f %F %u %U`).
pub fn launch_desktop_app(
    exec_cmd: &str,
    target_files: &[PathBuf],
    cwd: Option<&Path>,
) -> VfsResult<Child> {
    crate::assert_not_ui_thread();

    let first_file = target_files
        .first()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let all_files = target_files
        .iter()
        .map(|p| format!("\"{}\"", p.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");

    let first_uri = target_files
        .first()
        .map(|p| format!("file://{}", p.to_string_lossy()))
        .unwrap_or_default();
    let all_uris = target_files
        .iter()
        .map(|p| format!("file://{}", p.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");

    let mut expanded = exec_cmd.to_string();

    if expanded.contains("%f") {
        expanded = expanded.replace("%f", &first_file);
    }
    if expanded.contains("%F") {
        expanded = expanded.replace("%F", &all_files);
    }
    if expanded.contains("%u") {
        expanded = expanded.replace("%u", &first_uri);
    }
    if expanded.contains("%U") {
        expanded = expanded.replace("%U", &all_uris);
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&expanded);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    cmd.spawn()
        .map_err(|e| VfsError::Fatal(format!("Failed to launch application '{exec_cmd}': {e}")))
}
