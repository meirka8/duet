use serde::{Deserialize, Serialize};

/// Conflict resolution policy for operation handling.
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
