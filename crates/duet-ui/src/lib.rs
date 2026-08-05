//! `duet-ui` presentation layer: shell scaffolding, dual-panel workspace layout, file table views, theme system, and navigation.

pub mod app;
pub mod icons;
pub mod panel;
pub mod table;
pub mod theme;
pub mod workspace;

pub use app::run_app;
pub use icons::{resolve_icon, IconCategory};
pub use panel::{DirectoryPanelState, DirectoryPanelWidget, PanelTab};
pub use table::{ColumnLayout, CursorState, FileTable, ViewMode};
pub use theme::{ThemeMode, ThemeTokens};
pub use workspace::{ActiveModal, ActivePanel, WorkspaceView};

#[cfg(test)]
mod tests {
    use super::*;
    use duet_index::EntryInput;
    use duet_ops::ConflictPolicy;
    use duet_types::{EntryId, FileType, VPath};
    use duet_widgets::{
        ErrorLogEntry, FileMetaSide, JournalRecoveryEntry, LinkKind,
    };

    #[test]
    fn test_directory_panel_navigation_and_parent_cursor_restoration() {
        let mut state = DirectoryPanelState::default();
        let path_a = VPath::parse("/home/user/docs").unwrap();
        state.navigate_to(path_a.clone());

        assert_eq!(state.active_tab().path.to_string(), "file:///home/user/docs");

        // Populate parent entries simulate model
        let active_tab = state.active_tab_mut();
        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "..".into(),
                file_type: FileType::Directory,
                size: 0,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                mtime: 0,
                atime: 0,
                ctime: 0,
                dev: 1,
                ino: 1,
                nlink: 2,
                flags: 0,
            },
            EntryInput {
                id: EntryId(2),
                name: "docs".into(),
                file_type: FileType::Directory,
                size: 0,
                mode: 0o755,
                uid: 1000,
                gid: 1000,
                mtime: 0,
                atime: 0,
                ctime: 0,
                dev: 1,
                ino: 2,
                nlink: 2,
                flags: 0,
            },
        ];
        active_tab.model.set_entries(entries);

        // Going parent from /home/user/docs to /home/user
        state.navigate_parent();
        assert_eq!(state.active_tab().path.to_string(), "file:///home/user");
    }

    #[test]
    fn test_tab_creation_and_locking() {
        let mut state = DirectoryPanelState::default();
        assert_eq!(state.tabs.len(), 1);

        state.create_tab(VPath::parse("/tmp").unwrap());
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.active_tab().path.to_string(), "file:///tmp");

        state.toggle_tab_lock();
        assert!(state.active_tab().is_locked);

        // When tab is locked, navigating to new path opens another tab
        state.navigate_to(VPath::parse("/var").unwrap());
        assert_eq!(state.tabs.len(), 3);
        assert_eq!(state.active_tab().path.to_string(), "file:///var");
    }

    #[test]
    fn test_selection_shortcuts() {
        let mut state = DirectoryPanelState::default();
        let active_tab = state.active_tab_mut();
        let entries = vec![
            EntryInput {
                id: EntryId(1),
                name: "file1.txt".into(),
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
                name: "file2.rs".into(),
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
        active_tab.model.set_entries(entries);

        // Select all
        state.select_all();
        assert_eq!(state.active_tab().model.selection().len(), 2);

        // Invert
        state.invert_selection();
        assert_eq!(state.active_tab().model.selection().len(), 0);

        // Match extension
        state.active_tab_mut().cursor.cursor_idx = 1; // on file2.rs
        state.match_extension_selection();
        assert!(state.active_tab().model.selection().contains(&EntryId(2)));
    }

    #[test]
    fn test_copy_and_move_dialog_triggers() {
        let mut ws = WorkspaceView::default();
        assert!(matches!(ws.active_modal, ActiveModal::None));

        ws.trigger_copy_dialog();
        if let ActiveModal::CopyMove(state) = &ws.active_modal {
            assert!(!state.is_move);
            assert_eq!(state.options.overwrite_policy, ConflictPolicy::AskUser);
        } else {
            panic!("Expected CopyMove modal for copy trigger");
        }

        ws.trigger_move_dialog();
        if let ActiveModal::CopyMove(state) = &ws.active_modal {
            assert!(state.is_move);
        } else {
            panic!("Expected CopyMove modal for move trigger");
        }

        ws.close_modal();
        assert!(matches!(ws.active_modal, ActiveModal::None));
    }

    #[test]
    fn test_operation_manager_and_conflict_and_error_report_triggers() {
        let mut ws = WorkspaceView::default();

        ws.trigger_progress_manager();
        assert!(matches!(ws.active_modal, ActiveModal::ProgressManager(_)));

        let src = FileMetaSide {
            path: "/src/file.txt".into(),
            size_bytes: 100,
            mtime_str: "2026-08-05".into(),
            hash: None,
        };
        let dst = FileMetaSide {
            path: "/dst/file.txt".into(),
            size_bytes: 200,
            mtime_str: "2026-08-04".into(),
            hash: None,
        };
        ws.trigger_conflict_dialog(src, dst);
        assert!(matches!(ws.active_modal, ActiveModal::ConflictResolution(_)));

        let errors = vec![ErrorLogEntry {
            path: "/file1.txt".into(),
            error_message: "EACCES Permission Denied".into(),
            retryable: true,
        }];
        ws.trigger_error_report(42, errors);
        assert!(matches!(ws.active_modal, ActiveModal::ErrorReport(_)));
    }

    #[test]
    fn test_recovery_delete_dir_rename_permissions_triggers() {
        let mut ws = WorkspaceView::default();

        ws.trigger_startup_recovery(vec![JournalRecoveryEntry {
            job_id: 1,
            summary: "Interrupted Copy".into(),
            step_progress: "Step 2/5".into(),
        }]);
        assert!(matches!(ws.active_modal, ActiveModal::StartupRecovery(_)));

        ws.trigger_delete_dialog(false);
        if let ActiveModal::DeleteConfirmation(state) = &ws.active_modal {
            assert!(state.use_trash);
        } else {
            panic!("Expected DeleteConfirmation modal");
        }

        ws.trigger_delete_dialog(true); // Shift+Del bypasses trash
        if let ActiveModal::DeleteConfirmation(state) = &ws.active_modal {
            assert!(!state.use_trash);
            assert!(state.shift_pressed);
        } else {
            panic!("Expected DeleteConfirmation modal");
        }

        ws.trigger_create_dir_dialog();
        assert!(matches!(ws.active_modal, ActiveModal::CreateDir(_)));

        ws.trigger_rename_dialog();
        assert!(matches!(ws.active_modal, ActiveModal::Rename(_)));

        ws.trigger_create_link_dialog();
        if let ActiveModal::CreateLink(state) = &ws.active_modal {
            assert_eq!(state.link_kind, LinkKind::Symbolic);
        } else {
            panic!("Expected CreateLink modal");
        }

        ws.trigger_permissions_dialog();
        assert!(matches!(ws.active_modal, ActiveModal::Permissions(_)));
    }

    #[test]
    fn test_viewer_search_feed_to_panel_quick_view_triggers() {
        let mut ws = WorkspaceView::default();

        ws.trigger_internal_viewer();
        assert!(matches!(ws.active_modal, ActiveModal::InternalViewer(_)));

        ws.trigger_search_view();
        assert!(matches!(ws.active_modal, ActiveModal::SearchView(_)));

        ws.trigger_feed_to_panel();
        assert_eq!(
            ws.active_panel_state().active_tab().path.to_string(),
            "search://results/"
        );

        assert!(!ws.is_quick_view);
        ws.trigger_quick_view();
        assert!(ws.is_quick_view);
        assert!(ws.quick_view_state.is_some());
    }

    #[test]
    fn test_pack_unpack_branch_view_triggers() {
        let mut ws = WorkspaceView::default();

        ws.trigger_pack();
        assert!(matches!(ws.active_modal, ActiveModal::Pack(_)));

        ws.trigger_unpack();
        assert!(matches!(ws.active_modal, ActiveModal::Unpack(_)));

        ws.trigger_branch_view();
        assert_eq!(
            ws.active_panel_state().active_tab().path.to_string(),
            "branch://flat_tree/"
        );

        ws.trigger_connection_manager();
        assert!(matches!(
            ws.active_modal,
            ActiveModal::ConnectionManager(_)
        ));

        ws.trigger_plugin_manager();
        assert!(matches!(
            ws.active_modal,
            ActiveModal::PluginManager(_)
        ));
    }
}
