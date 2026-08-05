use crate::job::{JobId, JobProgress};
use crate::step::Step;
use duet_types::VPath;
use serde::{Deserialize, Serialize};

/// Events emitted during job execution lifecycle for streaming to the UI shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobEvent {
    Created { job_id: JobId },
    Started { job_id: JobId },
    ProgressUpdated { job_id: JobId, progress: JobProgress },
    Paused { job_id: JobId },
    Resumed { job_id: JobId },
    ConflictEncountered { job_id: JobId, src: VPath, dst: VPath },
    Completed { job_id: JobId },
    Failed { job_id: JobId, error: String },
    Cancelled { job_id: JobId },
    StepStarted { job_id: JobId, step_index: usize, step: Step },
    StepCompleted { job_id: JobId, step_index: usize },
}
