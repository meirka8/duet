//! Remote Backends Framework (Phase 7, Tasks T-7.1.1 – T-7.1.11).

pub mod credentials;
pub mod sftp;
pub mod ftp;
pub mod webdav;
pub mod s3;
pub mod smb;

pub use credentials::{ConnectionProfile, CredentialStore, SecretString};
pub use ftp::{FtpFs, FtpMode};
pub use s3::S3Fs;
pub use sftp::SftpFs;
pub use smb::SmbFs;
pub use webdav::WebDavFs;
