pub mod conflict;
pub mod event;
pub mod job;
pub mod journal;
pub mod plan;
pub mod step;

pub use conflict::ConflictPolicy;
pub use event::JobEvent;
pub use job::{Job, JobId, JobProgress, JobStatus};
pub use journal::{Journal, JournalRecord};
pub use plan::{CopyPlan, DeletePlan, MovePlan, Plan, SyncPlan};
pub use step::Step;

#[cfg(test)]
mod tests {
    use super::*;
    use duet_types::{MetaPatch, VPath};

    #[test]
    fn test_copy_3_file_directory_plan_serialization_roundtrip() {
        let _src_dir = VPath::new_local("/tmp/source_dir");
        let dst_dir = VPath::new_local("/tmp/dest_dir");

        let step1 = Step::CreateDir {
            path: dst_dir.clone(),
            mode: Some(0o755),
        };
        let step2 = Step::CopyFile {
            src: VPath::new_local("/tmp/source_dir/file1.txt"),
            dst: VPath::new_local("/tmp/dest_dir/file1.txt"),
            size: 1024,
        };
        let step3 = Step::CopyFile {
            src: VPath::new_local("/tmp/source_dir/file2.txt"),
            dst: VPath::new_local("/tmp/dest_dir/file2.txt"),
            size: 2048,
        };
        let step4 = Step::CopyFile {
            src: VPath::new_local("/tmp/source_dir/file3.txt"),
            dst: VPath::new_local("/tmp/dest_dir/file3.txt"),
            size: 4096,
        };
        let step5 = Step::SetMetadata {
            path: dst_dir,
            patch: MetaPatch {
                mode: Some(0o755),
                ..Default::default()
            },
        };

        let copy_plan = CopyPlan {
            file_count: 3,
            total_bytes: 7168,
            steps: vec![step1, step2, step3, step4, step5],
        };

        let plan = Plan::Copy(copy_plan);

        // Serialize to JSON
        let json_str = serde_json::to_string_pretty(&plan)
            .expect("Failed to serialize Plan to JSON");
        assert!(!json_str.is_empty());

        // Deserialize from JSON
        let deserialized_plan: Plan = serde_json::from_str(&json_str)
            .expect("Failed to deserialize Plan from JSON");

        // Verify equality
        assert_eq!(plan, deserialized_plan);
        assert_eq!(deserialized_plan.file_count(), 3);
        assert_eq!(deserialized_plan.total_bytes(), 7168);
        assert_eq!(deserialized_plan.steps().len(), 5);
    }

    #[test]
    fn test_journal_append_and_recovery() {
        let temp_dir = tempfile::tempdir().expect("tempdir failed");
        let journal_path = temp_dir.path().join("test_job.journal");

        let mut journal = Journal::open(&journal_path).expect("open journal failed");

        let job_id = JobId(42);
        let plan = Plan::Copy(CopyPlan {
            file_count: 1,
            total_bytes: 100,
            steps: vec![Step::CopyFile {
                src: VPath::new_local("/a"),
                dst: VPath::new_local("/b"),
                size: 100,
            }],
        });

        journal
            .append(&JournalRecord::JobInit {
                job_id,
                plan: plan.clone(),
                conflict_policy: ConflictPolicy::OverwriteAll,
            })
            .expect("append failed");

        journal
            .append(&JournalRecord::StepBegin {
                step_index: 0,
                step: plan.steps()[0].clone(),
            })
            .expect("append step begin failed");

        journal
            .append(&JournalRecord::StepCommit {
                step_index: 0,
                bytes_processed: 100,
            })
            .expect("append step commit failed");

        drop(journal);

        let records = Journal::read_all(&journal_path).expect("read_all failed");
        assert_eq!(records.len(), 3);

        let recovery = Journal::recover(&journal_path).expect("recover failed");
        assert!(recovery.is_some());
        let (recovered_id, recovered_plan, step_idx) = recovery.unwrap();
        assert_eq!(recovered_id, job_id);
        assert_eq!(recovered_plan, plan);
        assert_eq!(step_idx, 1);
    }
}
