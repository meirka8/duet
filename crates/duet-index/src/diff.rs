use duet_types::{EntryId, FileType};
use serde::{Deserialize, Serialize};

/// Detailed payload for single entry UI diff mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryDiffData {
    pub id: EntryId,
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub mtime: i64,
}

/// Diff protocol batch enum for zero-redundancy UI panel update emission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffBatch {
    Insert {
        index: usize,
        entry: EntryDiffData,
    },
    Remove {
        index: usize,
        id: EntryId,
    },
    Update {
        index: usize,
        entry: EntryDiffData,
    },
    Reorder {
        mapping: Vec<(usize, usize)>, // (old_index, new_index)
    },
    Reset,
    Batch(Vec<DiffBatch>),
}

impl DiffBatch {
    pub fn is_empty(&self) -> bool {
        match self {
            DiffBatch::Batch(list) => list.is_empty(),
            _ => false,
        }
    }
}
