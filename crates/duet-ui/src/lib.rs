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
pub use workspace::{ActivePanel, WorkspaceView};

#[cfg(test)]
mod tests {
    use super::*;
    use duet_index::EntryInput;
    use duet_types::{EntryId, FileType, VPath};

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
}
