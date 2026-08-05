use crate::step::Step;
use serde::{Deserialize, Serialize};

/// High-level plan for copying files/directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyPlan {
    pub file_count: u64,
    pub total_bytes: u64,
    pub steps: Vec<Step>,
}

/// High-level plan for moving files/directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovePlan {
    pub file_count: u64,
    pub total_bytes: u64,
    pub steps: Vec<Step>,
}

/// High-level plan for deleting files/directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletePlan {
    pub file_count: u64,
    pub total_bytes: u64,
    pub steps: Vec<Step>,
}

/// High-level plan for synchronising directories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncPlan {
    pub file_count: u64,
    pub total_bytes: u64,
    pub steps: Vec<Step>,
}

/// Execution plan enum encompassing all operational strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Plan {
    Copy(CopyPlan),
    Move(MovePlan),
    Delete(DeletePlan),
    Sync(SyncPlan),
}

impl Plan {
    pub fn file_count(&self) -> u64 {
        match self {
            Plan::Copy(p) => p.file_count,
            Plan::Move(p) => p.file_count,
            Plan::Delete(p) => p.file_count,
            Plan::Sync(p) => p.file_count,
        }
    }

    pub fn total_bytes(&self) -> u64 {
        match self {
            Plan::Copy(p) => p.total_bytes,
            Plan::Move(p) => p.total_bytes,
            Plan::Delete(p) => p.total_bytes,
            Plan::Sync(p) => p.total_bytes,
        }
    }

    pub fn steps(&self) -> &[Step] {
        match self {
            Plan::Copy(p) => &p.steps,
            Plan::Move(p) => &p.steps,
            Plan::Delete(p) => &p.steps,
            Plan::Sync(p) => &p.steps,
        }
    }

    pub fn steps_mut(&mut self) -> &mut Vec<Step> {
        match self {
            Plan::Copy(p) => &mut p.steps,
            Plan::Move(p) => &mut p.steps,
            Plan::Delete(p) => &mut p.steps,
            Plan::Sync(p) => &mut p.steps,
        }
    }
}
