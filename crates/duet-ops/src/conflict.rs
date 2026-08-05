use duet_types::{Metadata, VPath};
use serde::{Deserialize, Serialize};

/// Conflict resolution policy for file operation engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ConflictPolicy {
    #[default]
    AskUser,
    OverwriteAll,
    OverwriteOlder,
    OverwriteDifferentSize,
    SkipAll,
    AutoRenameAll,
    Cancel,
}

/// Action decision resulting from conflict evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictDecision {
    Overwrite,
    Skip,
    AutoRename(VPath),
    Cancel,
}

/// Evaluates a conflict policy against source and destination metadata.
pub fn resolve_conflict(
    policy: ConflictPolicy,
    _src_vpath: &VPath,
    dst_vpath: &VPath,
    src_meta: Option<&Metadata>,
    dst_meta: Option<&Metadata>,
) -> ConflictDecision {
    match policy {
        ConflictPolicy::OverwriteAll => ConflictDecision::Overwrite,
        ConflictPolicy::SkipAll => ConflictDecision::Skip,
        ConflictPolicy::Cancel => ConflictDecision::Cancel,

        ConflictPolicy::OverwriteOlder => {
            if let (Some(sm), Some(dm)) = (src_meta, dst_meta) {
                let sm_time = sm.modified.unwrap_or(0);
                let dm_time = dm.modified.unwrap_or(0);
                if sm_time > dm_time {
                    ConflictDecision::Overwrite
                } else {
                    ConflictDecision::Skip
                }
            } else {
                ConflictDecision::Overwrite
            }
        }

        ConflictPolicy::OverwriteDifferentSize => {
            if let (Some(sm), Some(dm)) = (src_meta, dst_meta) {
                if sm.size != dm.size {
                    ConflictDecision::Overwrite
                } else {
                    ConflictDecision::Skip
                }
            } else {
                ConflictDecision::Overwrite
            }
        }

        ConflictPolicy::AutoRenameAll => {
            let auto_path = generate_auto_rename_path(dst_vpath);
            ConflictDecision::AutoRename(auto_path)
        }

        ConflictPolicy::AskUser => {
            // Default to skip if unhandled interactively
            ConflictDecision::Skip
        }
    }
}

/// Generates an alternative non-conflicting path, e.g. "file (1).txt".
pub fn generate_auto_rename_path(vpath: &VPath) -> VPath {
    let mut new_vpath = vpath.clone();
    let path_str = &vpath.path;

    let (stem, ext) = if let Some(dot_idx) = path_str.rfind('.') {
        if let Some(slash_idx) = path_str.rfind('/') {
            if dot_idx > slash_idx {
                (&path_str[..dot_idx], &path_str[dot_idx..])
            } else {
                (path_str.as_str(), "")
            }
        } else {
            (&path_str[..dot_idx], &path_str[dot_idx..])
        }
    } else {
        (path_str.as_str(), "")
    };

    let mut count = 1;
    loop {
        let candidate = format!("{stem} ({count}){ext}");
        if !std::path::Path::new(&candidate).exists() {
            new_vpath.path = candidate;
            break;
        }
        count += 1;
    }

    new_vpath
}
