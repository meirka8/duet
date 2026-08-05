use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Strongly typed 64-bit mount ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MountId(pub u64);

impl fmt::Display for MountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mount-{}", self.0)
    }
}

/// Strongly typed 64-bit entry ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntryId(pub u64);

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "entry-{}", self.0)
    }
}

/// File classification enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    File,
    Directory,
    Symlink,
    BlockDevice,
    CharDevice,
    Fifo,
    Socket,
    Unknown,
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        matches!(self, FileType::Directory)
    }

    pub fn is_symlink(&self) -> bool {
        matches!(self, FileType::Symlink)
    }

    pub fn is_file(&self) -> bool {
        matches!(self, FileType::File)
    }
}

/// Virtual path capable of representing local paths, remote paths, and nested archives.
/// Examples:
/// - `file:///tmp/foo.txt`
/// - `sftp://user@host:22/srv/logs`
/// - `zip:file:///tmp/foo.zip!/bar/baz.txt`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VPath {
    pub scheme: String,
    pub authority: Option<String>,
    pub path: String,
    pub nested: Option<Box<VPath>>,
}

impl VPath {
    /// Construct a standard local file path VPath.
    pub fn new_local(path: impl Into<String>) -> Self {
        let p = path.into();
        Self {
            scheme: "file".to_string(),
            authority: None,
            path: if p.starts_with('/') { p } else { format!("/{p}") },
            nested: None,
        }
    }

    /// Parse a VPath string. Alias for `FromStr::from_str`.
    pub fn parse(s: &str) -> std::result::Result<Self, VfsError> {
        s.parse()
    }

    /// Return true if path represents a nested location inside an archive or secondary filesystem.
    pub fn is_nested(&self) -> bool {
        self.nested.is_some()
    }

    /// Extract the final file name or directory name component of the path.
    pub fn file_name(&self) -> Option<&str> {
        let p = self.path.trim_end_matches('/');
        if p.is_empty() {
            Some("/")
        } else {
            p.rsplit('/').next()
        }
    }

    /// Return the parent directory VPath, or None if already at root.
    pub fn parent(&self) -> Option<VPath> {
        let p = self.path.trim_end_matches('/');
        if p.is_empty() || p == "/" {
            return None;
        }

        if let Some(idx) = p.rfind('/') {
            let parent_path = if idx == 0 { "/" } else { &p[..idx] };
            let mut parent_vpath = self.clone();
            parent_vpath.path = parent_path.to_string();
            Some(parent_vpath)
        } else {
            None
        }
    }
}

impl Default for VPath {
    fn default() -> Self {
        Self::new_local("/")
    }
}

impl fmt::Display for VPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref nested) = self.nested {
            write!(f, "{}:{}!{}", self.scheme, nested, self.path)
        } else {
            write!(
                f,
                "{}://{}{}",
                self.scheme,
                self.authority.as_deref().unwrap_or(""),
                self.path
            )
        }
    }
}

impl FromStr for VPath {
    type Err = VfsError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(VfsError::InvalidPath("Empty path string".to_string()));
        }

        // Handle nested archive syntax: "scheme:nested_path!/path/in/archive"
        if let Some(excl_idx) = s.rfind("!/") {
            let before_excl = &s[..excl_idx];
            let after_excl = &s[excl_idx + 1..];

            let colon_idx = before_excl
                .find(':')
                .ok_or_else(|| VfsError::InvalidPath("Nested path missing scheme separator".to_string()))?;

            let scheme = before_excl[..colon_idx].to_string();
            let nested_str = &before_excl[colon_idx + 1..];
            let nested_vpath = VPath::from_str(nested_str)?;

            return Ok(VPath {
                scheme,
                authority: None,
                path: after_excl.to_string(),
                nested: Some(Box::new(nested_vpath)),
            });
        }

        // Plain Unix local path starting with '/'
        if s.starts_with('/') {
            return Ok(VPath::new_local(s));
        }

        // URI format: "scheme://[authority]/path"
        let colon_idx = s
            .find("://")
            .ok_or_else(|| VfsError::InvalidPath(format!("Invalid path format (missing ://): {s}")))?;

        let scheme = s[..colon_idx].to_string();
        let rest = &s[colon_idx + 3..];

        let (authority, path) = if let Some(slash_idx) = rest.find('/') {
            let auth = &rest[..slash_idx];
            let p = &rest[slash_idx..];
            (
                if auth.is_empty() { None } else { Some(auth.to_string()) },
                p.to_string(),
            )
        } else {
            (
                if rest.is_empty() { None } else { Some(rest.to_string()) },
                "/".to_string(),
            )
        };

        Ok(VPath {
            scheme,
            authority,
            path,
            nested: None,
        })
    }
}

bitflags! {
    /// Capabilities supported by a FileSystem implementation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Capabilities: u32 {
        const READ             = 1 << 0;
        const WRITE            = 1 << 1;
        const SEEK             = 1 << 2;
        const ATOMIC_RENAME    = 1 << 3;
        const REFLINK          = 1 << 4;
        const STREAMING_LIST   = 1 << 5;
        const WATCH            = 1 << 6;
        const POSIX_PERMISSIONS = 1 << 7;
        const XATTRS           = 1 << 8;
        const SYMLINK          = 1 << 9;
        const HARDLINK         = 1 << 10;

        // Aliases for compatibility
        const RANDOM_READ      = 1 << 0;
        const RANDOM_WRITE     = 1 << 1;
        const RENAME           = 1 << 3;
        const ATOMIC_REPLACE   = 1 << 3;
        const PERMISSIONS      = 1 << 7;
        const XATTR            = 1 << 8;
        const TIMESTAMPS       = 1 << 11;
        const SPARSE           = 1 << 12;
        const CHEAP_STAT       = 1 << 13;
        const APPEND_RESUME    = 1 << 14;
    }
}

pub type Caps = Capabilities;

impl Serialize for Capabilities {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Capabilities::from_bits(bits)
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid Capabilities bits: {bits}")))
    }
}

/// Comprehensive filesystem metadata representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    pub size: u64,
    pub file_type: FileType,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub created: Option<i64>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
    pub dev: u64,
    pub ino: u64,
    pub nlink: u64,
    pub xattrs: BTreeMap<String, Vec<u8>>,
    pub acl: Option<String>,
    pub selinux: Option<String>,
    pub rotational: Option<bool>,
    pub reflink_supported: Option<bool>,
}

impl Metadata {
    pub fn is_dir(&self) -> bool {
        self.file_type.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.file_type.is_symlink()
    }

    pub fn is_file(&self) -> bool {
        self.file_type.is_file()
    }
}

/// Specification for mutating file metadata attributes.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MetaPatch {
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
    pub created: Option<i64>,
    pub xattrs: BTreeMap<String, Option<Vec<u8>>>,
}

/// Domain error taxonomy for VFS and core engine operations.
#[derive(Debug, thiserror::Error)]
pub enum VfsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Is a directory: {0}")]
    IsADirectory(String),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("Directory not empty: {0}")]
    DirectoryNotEmpty(String),

    #[error("Operation conflict: {0}")]
    Conflict(String),

    #[error("Out of space")]
    OutOfSpace,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Read-only filesystem")]
    ReadOnlyFilesystem,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Corrupt data: {0}")]
    CorruptData(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Fatal error: {0}")]
    Fatal(String),
}

pub type Error = VfsError;
pub type VfsResult<T> = std::result::Result<T, VfsError>;
pub type Result<T> = VfsResult<T>;

#[cfg(any(test, feature = "test-utils"))]
pub mod proptests {
    use super::VPath;
    use proptest::prelude::*;

    pub fn arb_scheme() -> impl Strategy<Value = String> {
        "[a-z0-9]+"
    }

    pub fn arb_path_segment() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_.-]+"
    }

    pub fn arb_vpath() -> impl Strategy<Value = VPath> {
        let leaf = (
            arb_scheme(),
            prop::option::of("[a-zA-Z0-9.-]+"),
            prop::collection::vec(arb_path_segment(), 1..5),
        )
            .prop_map(|(scheme, authority, segments)| {
                let path = format!("/{}", segments.join("/"));
                VPath {
                    scheme,
                    authority,
                    path,
                    nested: None,
                }
            });

        leaf.prop_recursive(
            3,  // 3 levels deep max
            16, // max size
            8,  // items per level
            |inner| {
                (
                    arb_scheme(),
                    inner,
                    prop::collection::vec(arb_path_segment(), 1..5),
                )
                    .prop_map(|(scheme, nested_v, segments)| {
                        let path = format!("/{}", segments.join("/"));
                        VPath {
                            scheme,
                            authority: None,
                            path,
                            nested: Some(Box::new(nested_v)),
                        }
                    })
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_vpath_parse_and_display() {
        let cases = vec![
            "file:///tmp/foo.txt",
            "sftp://user@host:22/srv/logs",
            "zip:file:///tmp/foo.zip!/bar/baz.txt",
        ];

        for case in cases {
            let parsed = VPath::parse(case).expect("failed to parse VPath");
            assert_eq!(parsed.to_string(), case);
        }
    }

    proptest! {
        #[test]
        fn test_vpath_roundtrip(v in proptests::arb_vpath()) {
            let s = v.to_string();
            let parsed = VPath::parse(&s).unwrap();
            prop_assert_eq!(v, parsed);
        }
    }
}
