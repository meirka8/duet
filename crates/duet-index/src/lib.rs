pub mod diff;
pub mod entry_store;
pub mod model;
pub mod size_service;
pub mod watcher;
pub mod branch_view;

pub use branch_view::BranchView;
pub use diff::{DiffBatch, EntryDiffData};
pub use entry_store::{EntryRecord, EntryStore, PER_ENTRY_BYTE_BUDGET};
pub use model::{
    glob_match, natural_cmp, DirectoryModel, EntryInput, FilterSpec, SortColumn, SortDirection,
    WatchEvent,
};
pub use size_service::{CacheKey, DirSizeResult, DirSizeService};
pub use watcher::{CoalescedWatchEvent, DirectoryWatcher};

#[cfg(test)]
mod tests {
    use super::*;
    use duet_types::{EntryId, FileType};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_entry_store_soa_memory_budget() {
        let per_entry_soa_bytes = EntryStore::per_entry_bytes_soa();
        println!("EntryStore per-entry SoA array size: {per_entry_soa_bytes} bytes");
        assert!(
            per_entry_soa_bytes <= PER_ENTRY_BYTE_BUDGET,
            "Per-entry SoA array size ({per_entry_soa_bytes} B) exceeds budget of {PER_ENTRY_BYTE_BUDGET} B"
        );

        let mut store = EntryStore::with_capacity(1000);
        for i in 0..1000 {
            store.push(
                EntryId(i),
                &format!("file_{i:04}.txt"),
                FileType::File,
                1024 * i,
                0o644,
                1000,
                1000,
                1700000000 + i as i64,
                1700000000 + i as i64,
                1700000000 + i as i64,
                1,
                i,
                1,
                0,
            );
        }

        assert_eq!(store.len(), 1000);
        assert_eq!(store.get_name(0), "file_0000.txt");
        assert_eq!(store.get_name(999), "file_0999.txt");
        assert_eq!(store.size(500), 1024 * 500);
    }

    #[test]
    fn test_natural_sorting_and_multi_column() {
        assert_eq!(
            natural_cmp("file2.txt", "file10.txt"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            natural_cmp("item100", "item20"),
            std::cmp::Ordering::Greater
        );

        let mut model = DirectoryModel::new();
        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "file10.txt".into(),
                file_type: FileType::File,
                size: 100,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 100,
                atime: 100,
                ctime: 100,
                dev: 1,
                ino: 1,
                nlink: 1,
                flags: 0,
            },
            EntryInput {
                id: EntryId(2),
                name: "file2.txt".into(),
                file_type: FileType::File,
                size: 200,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 100,
                atime: 100,
                ctime: 100,
                dev: 1,
                ino: 2,
                nlink: 1,
                flags: 0,
            },
        ];
        model.set_entries(entries);

        // With natural sorting: file2.txt should come before file10.txt
        assert_eq!(model.store().get_name(model.view_indices()[0]), "file2.txt");
        assert_eq!(model.store().get_name(model.view_indices()[1]), "file10.txt");
    }

    #[test]
    fn test_glob_mask_filtering() {
        assert!(glob_match("*.txt", "readme.txt"));
        assert!(glob_match("file?.rs", "file1.rs"));
        assert!(!glob_match("*.rs", "readme.txt"));

        let mut model = DirectoryModel::new();
        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "doc.txt".into(),
                file_type: FileType::File,
                size: 10,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 1,
                nlink: 1,
                flags: 0,
            },
            EntryInput {
                id: EntryId(2),
                name: "code.rs".into(),
                file_type: FileType::File,
                size: 20,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 2,
                nlink: 1,
                flags: 0,
            },
        ];
        model.set_entries(entries);

        model.filter(FilterSpec {
            show_hidden: true,
            quick_filter: None,
            mask: Some("*.rs".into()),
        });

        assert_eq!(model.len(), 1);
        assert_eq!(model.store().get_name(model.view_indices()[0]), "code.rs");
    }

    #[test]
    fn test_selection_tracking_survives_rebuild() {
        let mut model = DirectoryModel::new();
        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "a.txt".into(),
                file_type: FileType::File,
                size: 100,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 1,
                nlink: 1,
                flags: 0,
            },
            EntryInput {
                id: EntryId(2),
                name: "b.txt".into(),
                file_type: FileType::File,
                size: 200,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 2,
                nlink: 1,
                flags: 0,
            },
        ];
        model.set_entries(entries);

        model.toggle_selection(EntryId(2));
        assert!(model.selection().contains(&EntryId(2)));

        // Filter out b.txt, then restore filter -> selection should still contain EntryId(2)
        model.filter(FilterSpec {
            show_hidden: true,
            quick_filter: None,
            mask: Some("a*".into()),
        });
        assert_eq!(model.len(), 1);
        assert!(model.selection().contains(&EntryId(2)));

        model.filter(FilterSpec::default());
        assert_eq!(model.len(), 2);
        assert!(model.selection().contains(&EntryId(2)));

        let (count, total_bytes) = model.selection_stats();
        assert_eq!(count, 1);
        assert_eq!(total_bytes, 200);
    }

    #[tokio::test]
    async fn test_dir_size_service_calculation_and_caching() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("file1.bin"), b"12345").unwrap();
        std::fs::write(dir.path().join("file2.bin"), b"123").unwrap();

        let service = DirSizeService::new();
        let cancel = Arc::new(AtomicBool::new(false));

        let res = service
            .compute_dir_size(dir.path().to_path_buf(), 1, 1, 100, cancel.clone())
            .await
            .unwrap();

        assert_eq!(res.total_bytes, 8);
        assert_eq!(res.total_files, 2);
        assert_eq!(res.total_dirs, 1);

        // Cache hit
        let cached = service
            .compute_dir_size(dir.path().to_path_buf(), 1, 1, 100, cancel)
            .await
            .unwrap();
        assert_eq!(cached, res);
    }
}
