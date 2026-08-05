pub mod diff;
pub mod entry_store;
pub mod model;

pub use diff::{DiffBatch, EntryDiffData};
pub use entry_store::{EntryRecord, EntryStore, PER_ENTRY_BYTE_BUDGET};
pub use model::{
    DirectoryModel, EntryInput, FilterSpec, SortColumn, SortDirection, WatchEvent,
};

#[cfg(test)]
mod tests {
    use super::*;
    use duet_types::{EntryId, FileType};
    use std::time::{Duration, Instant};

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

        let total_capacity_mem = store.memory_usage_bytes();
        let bytes_per_entry_allocated = total_capacity_mem as f64 / 1000.0;
        println!("Allocated bytes per entry (including arena capacity): {bytes_per_entry_allocated:.2} B");
    }

    #[test]
    fn test_directory_model_sorting_filtering_selection() {
        let mut model = DirectoryModel::new();

        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "beta.txt".into(),
                file_type: FileType::File,
                size: 200,
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
                name: "alpha_dir".into(),
                file_type: FileType::Directory,
                size: 4096,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                mtime: 500,
                atime: 500,
                ctime: 500,
                dev: 1,
                ino: 2,
                nlink: 2,
                flags: 0,
            },
            EntryInput {
                id: EntryId(3),
                name: ".hidden_file".into(),
                file_type: FileType::File,
                size: 50,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 300,
                atime: 300,
                ctime: 300,
                dev: 1,
                ino: 3,
                nlink: 1,
                flags: 0,
            },
        ];

        let reset_diff = model.set_entries(entries);
        assert_eq!(reset_diff, DiffBatch::Reset);

        // Default: directories first, hidden files hidden, sorted by name ascending
        // Should have alpha_dir first, then beta.txt (.hidden_file is hidden)
        assert_eq!(model.len(), 2);
        assert_eq!(model.store().get_name(model.view_indices()[0]), "alpha_dir");
        assert_eq!(model.store().get_name(model.view_indices()[1]), "beta.txt");

        // Enable show_hidden
        model.filter(FilterSpec {
            show_hidden: true,
            quick_filter: None,
            mask: None,
        });
        assert_eq!(model.len(), 3);
        assert_eq!(model.store().get_name(model.view_indices()[0]), "alpha_dir"); // Directory first!

        // Selection test
        model.toggle_selection(EntryId(2));
        assert!(model.selection().contains(&EntryId(2)));
        assert_eq!(model.selection().len(), 1);
        model.clear_selection();
        assert!(model.selection().is_empty());
    }

    #[test]
    fn test_directory_model_debounced_watch_notifications() {
        let mut model = DirectoryModel::with_debounce(Duration::from_millis(50));
        let start_time = Instant::now();

        // Push a watch event for a new file creation
        model.push_watch_event(WatchEvent::Created {
            id: EntryId(10),
            name: "new_file.txt".into(),
            file_type: FileType::File,
            size: 100,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            mtime: 1000,
            atime: 1000,
            ctime: 1000,
            dev: 1,
            ino: 10,
            nlink: 1,
            flags: 0,
        });

        // Immediately checking should return empty diffs because 50ms hasn't elapsed
        let diffs_immediate = model.process_watch_events(start_time);
        assert!(diffs_immediate.is_empty());
        assert_eq!(model.len(), 0);

        // Simulating 100ms later
        let later_time = start_time + Duration::from_millis(100);
        let diffs_later = model.process_watch_events(later_time);

        assert_eq!(diffs_later.len(), 1);
        assert!(matches!(diffs_later[0], DiffBatch::Insert { .. }));
        assert_eq!(model.len(), 1);
        assert_eq!(model.store().get_name(model.view_indices()[0]), "new_file.txt");
    }
}
