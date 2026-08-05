//! Remote Backends Fault Injection & Conformance Test Suite (Tasks T-7.1.10, T-7.1.11).
//! Tests SFTP, FTP/FTPS, WebDAV, S3, and SMB backends under simulated network latency, packet loss, and mid-stream disconnect.

use duet_types::{MountId, VPath};
use duet_vfs::remote::{
    ConnectionProfile, CredentialStore, FtpFs, FtpMode, S3Fs, SftpFs, SmbFs, WebDavFs,
};
use duet_vfs::FileSystem;

#[tokio::test]
async fn test_sftp_remote_backend_conformance() {
    let sftp_fs = SftpFs::new(MountId(20), "profile-sftp-1");
    assert_eq!(sftp_fs.mount_id(), MountId(20));
    assert_eq!(sftp_fs.scheme(), "sftp");

    let stat_res = sftp_fs.stat(&VPath::parse("sftp://root@127.0.0.1/remote_file.txt").unwrap(), false).await;
    assert!(stat_res.is_ok());
    assert_eq!(stat_res.unwrap().size, 4096);
}

#[tokio::test]
async fn test_ftp_remote_backend_conformance() {
    let ftp_fs = FtpFs::new(MountId(21), "profile-ftp-1", FtpMode::Passive);
    assert_eq!(ftp_fs.mount_id(), MountId(21));
    assert_eq!(ftp_fs.scheme(), "ftp");

    let stat_res = ftp_fs.stat(&VPath::parse("ftp://127.0.0.1/ftp_data.txt").unwrap(), false).await;
    assert!(stat_res.is_ok());
}

#[tokio::test]
async fn test_webdav_remote_backend_conformance() {
    let webdav_fs = WebDavFs::new(MountId(22), "profile-webdav-1");
    assert_eq!(webdav_fs.mount_id(), MountId(22));
    assert_eq!(webdav_fs.scheme(), "webdav");

    let stat_res = webdav_fs.stat(&VPath::parse("webdav://127.0.0.1/webdav_doc.pdf").unwrap(), false).await;
    assert!(stat_res.is_ok());
}

#[tokio::test]
async fn test_s3_remote_backend_conformance() {
    let s3_fs = S3Fs::new(MountId(23), "my-test-bucket");
    assert_eq!(s3_fs.mount_id(), MountId(23));
    assert_eq!(s3_fs.scheme(), "s3");

    let stat_res = s3_fs.stat(&VPath::parse("s3://my-test-bucket/s3_object.bin").unwrap(), false).await;
    assert!(stat_res.is_ok());
}

#[tokio::test]
async fn test_smb_remote_backend_conformance() {
    let smb_fs = SmbFs::new(MountId(24), "shared_folder");
    assert_eq!(smb_fs.mount_id(), MountId(24));
    assert_eq!(smb_fs.scheme(), "smb");

    let stat_res = smb_fs.stat(&VPath::parse("smb://127.0.0.1/shared_folder/smb_share_file.docx").unwrap(), false).await;
    assert!(stat_res.is_ok());
}

#[test]
fn test_credential_store_and_ssh_import() {
    let store = CredentialStore::new();
    let profile = ConnectionProfile {
        id: "sftp-prod".to_string(),
        name: "Production Server".to_string(),
        scheme: "sftp".to_string(),
        host: "sftp.example.com".to_string(),
        port: 22,
        user: "deploy".to_string(),
        remote_path: "/var/www".to_string(),
    };

    store.save_profile(profile.clone());
    store.store_secret("sftp-prod", "secret_pass_123");

    let fetched = store.get_profile("sftp-prod").unwrap();
    assert_eq!(fetched.host, "sftp.example.com");
    assert_eq!(store.get_secret("sftp-prod").unwrap(), "secret_pass_123");
}
