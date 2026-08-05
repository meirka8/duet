use crate::conflict::ConflictPolicy;
use crate::journal::Journal;
use crate::plan::Plan;
use duet_types::{VPath, VfsError};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;

/// Strongly typed 64-bit identifier for an operation job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "job-{}", self.0)
    }
}

/// Lifecycle status of an operation job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Paused,
    Cancelling,
    Completed,
    Failed { error: String },
}

/// Live progress statistics for a running job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub files_processed: u64,
    pub total_files: u64,
    pub eta_seconds: Option<f64>,
    pub current_file: Option<VPath>,
}

impl JobProgress {
    pub fn new(total_files: u64, total_bytes: u64) -> Self {
        Self {
            bytes_transferred: 0,
            total_bytes,
            files_processed: 0,
            total_files,
            eta_seconds: None,
            current_file: None,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            if self.total_files == 0 {
                100.0
            } else {
                (self.files_processed as f64 / self.total_files as f64) * 100.0
            }
        } else {
            (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
        }
    }
}

/// Managed operation job entity.
#[derive(Debug)]
pub struct Job {
    pub id: JobId,
    pub plan: Plan,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub conflict_policy: ConflictPolicy,
    pub journal: Option<Journal>,
}

impl Job {
    pub fn new(id: JobId, plan: Plan, conflict_policy: ConflictPolicy) -> Self {
        let progress = JobProgress::new(plan.file_count(), plan.total_bytes());
        Self {
            id,
            plan,
            status: JobStatus::Pending,
            progress,
            conflict_policy,
            journal: None,
        }
    }

    pub fn with_journal(mut self, journal_path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let journal = Journal::open(journal_path)?;
        self.journal = Some(journal);
        Ok(self)
    }

    pub fn start(&mut self) {
        if self.status == JobStatus::Pending || self.status == JobStatus::Paused {
            self.status = JobStatus::Running;
        }
    }

    pub fn pause(&mut self) {
        if self.status == JobStatus::Running {
            self.status = JobStatus::Paused;
        }
    }

    pub fn cancel(&mut self) {
        if self.status == JobStatus::Running || self.status == JobStatus::Paused {
            self.status = JobStatus::Cancelling;
        }
    }

    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed {
            error: error.into(),
        };
    }
}
