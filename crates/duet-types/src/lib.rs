use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VPath {
    pub scheme: String,
    pub authority: Option<String>,
    pub path: String,
    pub nested: Option<Box<VPath>>,
}

impl VPath {
    pub fn new_local(path: &str) -> Self {
        Self {
            scheme: "file".to_string(),
            authority: None,
            path: path.to_string(),
            nested: None,
        }
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
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if let Some(excl_idx) = s.rfind("!/") {
            let before_excl = &s[..excl_idx];
            let after_excl = &s[excl_idx + 1..];

            let colon_idx = before_excl
                .find(':')
                .ok_or_else(|| "Nested path missing scheme separator".to_string())?;
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

        let colon_idx = s
            .find("://")
            .ok_or_else(|| format!("Invalid URI (missing ://): {}", s))?;
        let scheme = s[..colon_idx].to_string();
        let rest = &s[colon_idx + 3..];

        let (authority, path) = if let Some(slash_idx) = rest.find('/') {
            let auth = rest[..slash_idx].to_string();
            let p = rest[slash_idx..].to_string();
            (if auth.is_empty() { None } else { Some(auth) }, p)
        } else {
            (
                if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_string())
                },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MountId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub u64);

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Caps: u32 {
        const RANDOM_READ      = 1 << 0;
        const RANDOM_WRITE     = 1 << 1;
        const RENAME           = 1 << 2;
        const ATOMIC_REPLACE   = 1 << 3;
        const HARDLINK         = 1 << 4;
        const SYMLINK          = 1 << 5;
        const XATTR            = 1 << 6;
        const PERMISSIONS      = 1 << 7;
        const TIMESTAMPS       = 1 << 8;
        const SPARSE           = 1 << 9;
        const REFLINK          = 1 << 10;
        const WATCH            = 1 << 11;
        const CHEAP_STAT       = 1 << 12;
        const APPEND_RESUME    = 1 << 13;
    }
}

impl Serialize for Caps {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for Caps {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Caps::from_bits(bits)
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid Caps bits: {}", bits)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Metadata {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mtime: i64,
    pub atime: i64,
    pub mode: u32,
    pub nlink: u64,
    pub dev: u64,
    pub ino: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MetaPatch {
    pub mode: Option<u32>,
    pub mtime: Option<i64>,
    pub atime: Option<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Operation conflict: {0}")]
    Conflict(String),
    #[error("Out of space")]
    OutOfSpace,
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    #[error("Fatal error: {0}")]
    Fatal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

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

    proptest! {
        #[test]
        fn test_vpath_roundtrip(v in proptests::arb_vpath()) {
            let s = v.to_string();
            let parsed = VPath::from_str(&s).unwrap();
            prop_assert_eq!(v, parsed);
        }
    }
}
