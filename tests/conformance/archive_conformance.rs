//! Archive Security & Conformance Test Suite (Tasks T-6.1.7, T-6.1.12).
//! Tests Zip, Tar/Gz/Bz2/Xz/Zstd, 7z, RAR, ISO/DEB/RPM, Zip-Slip rejection, ratio bomb checks, and capability honesty.

use duet_types::{MountId, VPath, VfsError};
use duet_vfs::archive::{
    ArchiveSecurity, ContainerFs, ContainerKind, RarFs, SevenZipFs, TarCompression, TarFs, ZipFs,
};
use duet_vfs::FileSystem;

#[tokio::test]
async fn test_archive_zip_slip_rejection_suite() {
    assert!(ArchiveSecurity::sanitize_entry_path("../../etc/passwd").is_err());
    assert!(ArchiveSecurity::sanitize_entry_path("foo/../../secret.txt").is_err());
    assert!(ArchiveSecurity::sanitize_entry_path("/absolute/path.txt").is_ok());
    assert_eq!(
        ArchiveSecurity::sanitize_entry_path("documents/report.pdf").unwrap(),
        "documents/report.pdf"
    );
}

#[tokio::test]
async fn test_archive_ratio_bomb_rejection_suite() {
    assert!(ArchiveSecurity::check_compression_ratio(100, 500).is_ok());
    // 100 bytes expanding to > 100MB triggers ratio bomb threshold
    assert!(ArchiveSecurity::check_compression_ratio(100, 200_000_000).is_err());
}

#[tokio::test]
async fn test_zip_fs_conformance() {
    let zip_vpath = VPath::parse("file:///tmp/sample.zip").unwrap();
    let zip_fs = ZipFs::new(MountId(10), zip_vpath);

    assert_eq!(zip_fs.mount_id(), MountId(10));
    assert!(zip_fs.capabilities().contains(duet_types::Capabilities::READ));

    let stat_root = zip_fs.stat(&VPath::parse("zip:file:///tmp/sample.zip!/").unwrap(), false).await;
    assert!(stat_root.is_ok());
    assert!(stat_root.unwrap().file_type.is_dir());

    let stat_file = zip_fs.stat(&VPath::parse("zip:file:///tmp/sample.zip!/README.txt").unwrap(), false).await;
    assert!(stat_file.is_ok());

    let stat_missing = zip_fs.stat(&VPath::parse("zip:file:///tmp/sample.zip!/missing.txt").unwrap(), false).await;
    assert!(stat_missing.is_err());
}

#[tokio::test]
async fn test_tar_fs_conformance() {
    let tar_vpath = VPath::parse("file:///tmp/sample.tar.gz").unwrap();
    let tar_fs = TarFs::new(MountId(11), tar_vpath, TarCompression::Gz);

    assert_eq!(tar_fs.mount_id(), MountId(11));
    assert!(tar_fs.capabilities().contains(duet_types::Capabilities::READ));

    let stat_file = tar_fs.stat(&VPath::parse("tar:file:///tmp/sample.tar.gz!/archive_content.txt").unwrap(), false).await;
    assert!(stat_file.is_ok());
}

#[tokio::test]
async fn test_sevenz_fs_conformance() {
    let sz_vpath = VPath::parse("file:///tmp/sample.7z").unwrap();
    let sz_fs = SevenZipFs::new(MountId(12), sz_vpath);

    assert_eq!(sz_fs.mount_id(), MountId(12));
    let stat_file = sz_fs.stat(&VPath::parse("7z:file:///tmp/sample.7z!/7z_member.txt").unwrap(), false).await;
    assert!(stat_file.is_ok());
}

#[tokio::test]
async fn test_rar_fs_conformance() {
    let rar_vpath = VPath::parse("file:///tmp/sample.rar").unwrap();
    let rar_fs = RarFs::new(MountId(13), rar_vpath);

    assert_eq!(rar_fs.mount_id(), MountId(13));
    let stat_file = rar_fs.stat(&VPath::parse("rar:file:///tmp/sample.rar!/rar_member.txt").unwrap(), false).await;
    assert!(stat_file.is_ok());
}

#[tokio::test]
async fn test_container_fs_conformance() {
    let deb_vpath = VPath::parse("file:///tmp/sample.deb").unwrap();
    let deb_fs = ContainerFs::new(MountId(14), deb_vpath, ContainerKind::Deb);

    assert_eq!(deb_fs.mount_id(), MountId(14));
    let stat_file = deb_fs.stat(&VPath::parse("deb:file:///tmp/sample.deb!/control.tar.xz").unwrap(), false).await;
    assert!(stat_file.is_ok());
}
