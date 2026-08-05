use duet_types::{MetaPatch, VPath};
use serde::{Deserialize, Serialize};

/// Granular operation step executed by the job engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    CopyFile { src: VPath, dst: VPath, size: u64 },
    CreateDir { path: VPath, mode: Option<u32> },
    MoveFile { src: VPath, dst: VPath },
    RemoveFile { path: VPath },
    SetMetadata { path: VPath, patch: MetaPatch },
    ApplyPatch { path: VPath, patch: MetaPatch },
    Truncate { path: VPath, size: u64 },
    AtomicRename { src: VPath, dst: VPath },
    Reflink { src: VPath, dst: VPath },
    CreateSymlink { target: String, link_path: VPath },
    CreateHardlink { src: VPath, dst: VPath },
    VerifyChecksum { path: VPath, expected_hash: String },
}
