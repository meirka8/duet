//! Archive Backend Framework (Tasks T-6.1.2 – T-6.1.6).

pub mod security;
pub mod zip;
pub mod tar;
pub mod sevenz;
pub mod rar;
pub mod container;

pub use security::ArchiveSecurity;
pub use zip::ZipFs;
pub use tar::{TarCompression, TarFs};
pub use sevenz::SevenZipFs;
pub use rar::RarFs;
pub use container::{ContainerFs, ContainerKind};

/// In-memory archive member entry representation.
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub is_dir: bool,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub mode: u32,
    pub mtime: i64,
}
