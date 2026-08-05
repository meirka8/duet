use crate::conflict::ConflictPolicy;
use crate::job::{JobId, JobStatus};
use crate::plan::Plan;
use crate::step::Step;
use duet_types::VfsError;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Individual entry in the append-only write-ahead operation journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JournalRecord {
    JobInit {
        job_id: JobId,
        plan: Plan,
        conflict_policy: ConflictPolicy,
    },
    StepBegin {
        step_index: usize,
        step: Step,
    },
    StepCommit {
        step_index: usize,
        bytes_processed: u64,
    },
    JobStatusChange {
        status: JobStatus,
    },
    Checkpoint {
        bytes_transferred: u64,
        files_processed: u64,
    },
}

/// Append-only write-ahead log handle for operation crash safety.
#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    file: Option<File>,
}

impl Journal {
    /// Open or create an append-only journal file at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path_buf)?;

        Ok(Self {
            path: path_buf,
            file: Some(file),
        })
    }

    /// Append a single record to the journal and flush/sync to disk.
    pub fn append(&mut self, record: &JournalRecord) -> Result<(), VfsError> {
        if let Some(ref mut file) = self.file {
            let mut line = serde_json::to_string(record)
                .map_err(|e| VfsError::Fatal(format!("Journal serialization error: {e}")))?;
            line.push('\n');
            file.write_all(line.as_bytes())?;
            file.flush()?;
            file.sync_data()?;
            Ok(())
        } else {
            Err(VfsError::Fatal("Journal file not open".to_string()))
        }
    }

    /// Read all records from a journal file.
    pub fn read_all(path: impl AsRef<Path>) -> Result<Vec<JournalRecord>, VfsError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();

        for line in reader.lines() {
            let l = line?;
            if l.trim().is_empty() {
                continue;
            }
            let rec: JournalRecord = serde_json::from_str(&l)
                .map_err(|e| VfsError::CorruptData(format!("Journal decode error: {e}")))?;
            records.push(rec);
        }

        Ok(records)
    }

    /// Attempt recovery from a journal file, returning the (JobId, Plan, last_committed_step_index).
    pub fn recover(path: impl AsRef<Path>) -> Result<Option<(JobId, Plan, usize)>, VfsError> {
        let records = Self::read_all(path)?;
        let mut job_info = None;
        let mut last_committed_step = 0;

        for rec in records {
            match rec {
                JournalRecord::JobInit { job_id, plan, .. } => {
                    job_info = Some((job_id, plan));
                }
                JournalRecord::StepCommit { step_index, .. } => {
                    last_committed_step = step_index + 1;
                }
                _ => {}
            }
        }

        if let Some((job_id, plan)) = job_info {
            Ok(Some((job_id, plan, last_committed_step)))
        } else {
            Ok(None)
        }
    }

    /// Return the path to the journal file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
