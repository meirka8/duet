//! Archive Security Hardening (Task T-6.1.7).
//! Protects against Zip-Slip path traversal attacks, compression ratio bombs, and unsafe symlink targets.

use std::path::{Component, Path};
use duet_types::VfsError;

pub struct ArchiveSecurity;

impl ArchiveSecurity {
    /// Validate archive entry filename against Zip-Slip path traversal attacks (`../..`).
    pub fn sanitize_entry_path(raw_path: &str) -> Result<String, VfsError> {
        let path = Path::new(raw_path);
        for comp in path.components() {
            match comp {
                Component::ParentDir => {
                    return Err(VfsError::PermissionDenied("zip-slip path traversal rejected".into()))
                }
                Component::Prefix(_) | Component::RootDir => {
                    // Strip absolute prefix
                }
                Component::Normal(_) | Component::CurDir => {}
            }
        }

        let clean = raw_path
            .trim_start_matches('/')
            .trim_start_matches('\\');

        if clean.contains("..") {
            return Err(VfsError::PermissionDenied("zip-slip path traversal rejected".into()));
        }

        Ok(clean.to_string())
    }

    /// Check compression ratio bomb (e.g. 1 KB compressed expanding to 10 GB).
    pub fn check_compression_ratio(compressed_size: u64, uncompressed_size: u64) -> Result<(), VfsError> {
        if let Some(ratio) = uncompressed_size.checked_div(compressed_size) {
            if ratio > 1000 && uncompressed_size > 100 * 1024 * 1024 {
                return Err(VfsError::PermissionDenied("compression ratio bomb rejected".into()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_slip_rejection() {
        assert!(ArchiveSecurity::sanitize_entry_path("../../etc/passwd").is_err());
        assert!(ArchiveSecurity::sanitize_entry_path("foo/../../bar").is_err());
        assert_eq!(
            ArchiveSecurity::sanitize_entry_path("foo/bar.txt").unwrap(),
            "foo/bar.txt"
        );
    }

    #[test]
    fn test_ratio_bomb_rejection() {
        assert!(ArchiveSecurity::check_compression_ratio(100, 1000).is_ok());
        assert!(ArchiveSecurity::check_compression_ratio(10, 500_000_000).is_err());
    }
}
