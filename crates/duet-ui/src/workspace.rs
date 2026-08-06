//! `WorkspaceView` dual-panel workspace layout containing panels, splitter, command line, function key bar, status bar, and modal overlay stack.

use crate::panel::{DirectoryPanelState, DirectoryPanelWidget};
use crate::theme::ThemeTokens;
use duet_index::EntryInput;
use duet_types::{EntryId, FileType, VPath};
use duet_widgets::{
    ButtonBar, ButtonBarData, ConflictDialogState, ConflictResolutionDialog,
    ConnectionManagerDialog, ConnectionManagerDialogState, CopyMoveDialog, CopyMoveDialogState,
    CreateDirDialog, CreateDirDialogState, CreateLinkDialog, CreateLinkDialogState,
    DeleteConfirmationDialog, DeleteDialogState, DirSyncDialog, DirSyncDialogState, DriveBar,
    DriveBarData, ErrorLogEntry, ErrorReportDialog, ErrorReportState, FileMetaSide, FunctionBar,
    InputState, InputWidget, InternalViewerWidget, JobItemDisplay, JobManagerModalState,
    JournalRecoveryEntry, LinkKind, MultiRenameDialog, MultiRenameDialogState, OperationManagerModal,
    PackDialog, PackDialogState, PermissionsDialog, PermissionsDialogState, PluginManagerDialog,
    PluginManagerDialogState, PropertiesDialog, PropertiesDialogState, QuickViewWidget,
    RenameDialog, RenameDialogState, ResizableSplitter, SearchDialogState, SearchResultEntry,
    SearchViewWidget, SettingsDialog, SettingsDialogState, SplitDirection, SplitterState,
    StatusBar, StatusBarData, StatusProgressTray, StatusProgressTrayData, StartupRecoveryOverlay,
    StartupRecoveryState, TerminalPanelState, TerminalPanelWidget, UnpackDialog, UnpackDialogState,
    ViewerState,
};
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

#[derive(Debug, Clone, Default)]
pub enum ActiveModal {
    #[default]
    None,
    CopyMove(CopyMoveDialogState),
    ProgressManager(JobManagerModalState),
    ConflictResolution(ConflictDialogState),
    ErrorReport(ErrorReportState),
    StartupRecovery(StartupRecoveryState),
    DeleteConfirmation(DeleteDialogState),
    CreateDir(CreateDirDialogState),
    Rename(RenameDialogState),
    CreateLink(CreateLinkDialogState),
    Permissions(PermissionsDialogState),
    InternalViewer(ViewerState),
    SearchView(SearchDialogState),
    Pack(PackDialogState),
    Unpack(UnpackDialogState),
    ConnectionManager(Box<ConnectionManagerDialogState>),
    PluginManager(Box<PluginManagerDialogState>),
    MultiRename(Box<MultiRenameDialogState>),
    DirSync(Box<DirSyncDialogState>),
    Properties(Box<PropertiesDialogState>),
    Settings(Box<SettingsDialogState>),
}

pub struct WorkspaceView {
    pub left_panel: DirectoryPanelState,
    pub right_panel: DirectoryPanelState,
    pub active_panel: ActivePanel,
    pub splitter: SplitterState,
    pub cmdline_state: InputState,
    pub active_modal: ActiveModal,
    pub progress_tray: StatusProgressTrayData,
    pub drive_bar: DriveBarData,
    pub button_bar: ButtonBarData,
    pub terminal_panel: TerminalPanelState,
    pub is_quick_view: bool,
    pub quick_view_state: Option<ViewerState>,
    pub theme: ThemeTokens,
    pub focus_handle: Option<FocusHandle>,
}

impl Default for WorkspaceView {
    fn default() -> Self {
        let cmdline_state = InputState {
            placeholder: "Enter shell command...".to_string(),
            ..Default::default()
        };

        Self {
            left_panel: DirectoryPanelState::default(),
            right_panel: DirectoryPanelState::default(),
            active_panel: ActivePanel::Left,
            splitter: SplitterState::new(0.5, SplitDirection::Horizontal),
            cmdline_state,
            active_modal: ActiveModal::None,
            progress_tray: StatusProgressTrayData::default(),
            drive_bar: DriveBarData::default(),
            button_bar: ButtonBarData::default(),
            terminal_panel: TerminalPanelState::default(),
            is_quick_view: false,
            quick_view_state: None,
            theme: ThemeTokens::dark(),
            focus_handle: None,
        }
    }
}

impl WorkspaceView {
    pub fn new(cx: &mut App) -> Self {
        Self::with_paths(cx, None, None)
    }

    pub fn with_paths(cx: &mut App, left_path: Option<String>, right_path: Option<String>) -> Self {
        let focus_handle = cx.focus_handle();
        let mut ws = Self {
            focus_handle: Some(focus_handle),
            ..Default::default()
        };

        if let Some(lp) = left_path {
            ws.left_panel.active_tab_mut().load_path(&VPath::new_local(lp));
        }
        if let Some(rp) = right_path {
            ws.right_panel.active_tab_mut().load_path(&VPath::new_local(rp));
        }

        ws
    }

    pub fn active_panel_state(&self) -> &DirectoryPanelState {
        match self.active_panel {
            ActivePanel::Left => &self.left_panel,
            ActivePanel::Right => &self.right_panel,
        }
    }

    pub fn active_panel_state_mut(&mut self) -> &mut DirectoryPanelState {
        match self.active_panel {
            ActivePanel::Left => &mut self.left_panel,
            ActivePanel::Right => &mut self.right_panel,
        }
    }

    pub fn target_panel_state(&self) -> &DirectoryPanelState {
        match self.active_panel {
            ActivePanel::Left => &self.right_panel,
            ActivePanel::Right => &self.left_panel,
        }
    }

    pub fn target_panel_state_mut(&mut self) -> &mut DirectoryPanelState {
        match self.active_panel {
            ActivePanel::Left => &mut self.right_panel,
            ActivePanel::Right => &mut self.left_panel,
        }
    }

    pub fn toggle_active_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
        self.update_quick_view_preview();
    }

    pub fn close_modal(&mut self) {
        self.active_modal = ActiveModal::None;
    }

    // --- Task 5.2.1: Copy/Move Dialog (F5 / F6) ---
    pub fn trigger_copy_dialog(&mut self) {
        let dest_path = self.target_panel_state().active_tab().path.to_string();
        let selected_files = self.get_active_selected_files();

        let state = CopyMoveDialogState {
            is_move: false,
            dest_input: InputState {
                value: dest_path,
                placeholder: "Destination directory".to_string(),
                is_focused: true,
                ..Default::default()
            },
            source_files: selected_files,
            ..Default::default()
        };
        self.active_modal = ActiveModal::CopyMove(state);
    }

    pub fn trigger_move_dialog(&mut self) {
        let dest_path = self.target_panel_state().active_tab().path.to_string();
        let selected_files = self.get_active_selected_files();

        let state = CopyMoveDialogState {
            is_move: true,
            dest_input: InputState {
                value: dest_path,
                placeholder: "Destination directory".to_string(),
                is_focused: true,
                ..Default::default()
            },
            source_files: selected_files,
            ..Default::default()
        };
        self.active_modal = ActiveModal::CopyMove(state);
    }

    // --- Task 5.2.2: Operation Manager Modal ---
    pub fn trigger_progress_manager(&mut self) {
        let state = JobManagerModalState {
            jobs: vec![
                JobItemDisplay {
                    job_id: 101,
                    title: "Copying 45 files".to_string(),
                    op_type: "Copy".to_string(),
                    src: "/home/user/documents".to_string(),
                    dst: "/mnt/backup/documents".to_string(),
                    progress_percent: 65,
                    copied_bytes: 650 * 1024 * 1024,
                    total_bytes: 1000 * 1024 * 1024,
                    speed_bytes_per_sec: 25 * 1024 * 1024,
                    eta_seconds: Some(14),
                    status: "Running".to_string(),
                },
            ],
            is_expanded: true,
        };
        self.active_modal = ActiveModal::ProgressManager(state);
    }

    // --- Task 5.2.3: Conflict Resolution Dialog ---
    pub fn trigger_conflict_dialog(&mut self, src: FileMetaSide, dst: FileMetaSide) {
        let state = ConflictDialogState {
            src,
            dst,
            apply_to_all: false,
            selected_policy: None,
        };
        self.active_modal = ActiveModal::ConflictResolution(state);
    }

    // --- Task 5.2.4: Error Report View ---
    pub fn trigger_error_report(&mut self, job_id: u64, errors: Vec<ErrorLogEntry>) {
        let state = ErrorReportState { job_id, errors };
        self.active_modal = ActiveModal::ErrorReport(state);
    }

    // --- Task 5.2.5: Startup Recovery Overlay ---
    pub fn trigger_startup_recovery(&mut self, entries: Vec<JournalRecoveryEntry>) {
        let state = StartupRecoveryState {
            journal_entries: entries,
        };
        self.active_modal = ActiveModal::StartupRecovery(state);
    }

    // --- Task 5.2.6: Delete Confirmation (F8) ---
    pub fn trigger_delete_dialog(&mut self, shift_pressed: bool) {
        let items = self.get_active_selected_files();
        let state = DeleteDialogState {
            items,
            use_trash: !shift_pressed,
            shift_pressed,
        };
        self.active_modal = ActiveModal::DeleteConfirmation(state);
    }

    // --- Task 5.2.7: Create Dir (F7), Rename (Shift+F6), Links ---
    pub fn trigger_create_dir_dialog(&mut self) {
        let state = CreateDirDialogState {
            input: InputState {
                placeholder: "New directory name (e.g. nested/path)".to_string(),
                is_focused: true,
                ..Default::default()
            },
        };
        self.active_modal = ActiveModal::CreateDir(state);
    }

    pub fn trigger_rename_dialog(&mut self) {
        let active_name = self.get_active_cursor_item_name().unwrap_or_default();
        let state = RenameDialogState::new(&active_name);
        self.active_modal = ActiveModal::Rename(state);
    }

    pub fn trigger_create_link_dialog(&mut self) {
        let target_path = self.get_active_cursor_item_path().unwrap_or_default();
        let state = CreateLinkDialogState {
            target_path,
            link_name_input: InputState {
                placeholder: "Link name or path".to_string(),
                is_focused: true,
                ..Default::default()
            },
            link_kind: LinkKind::Symbolic,
        };
        self.active_modal = ActiveModal::CreateLink(state);
    }

    // --- Task 5.2.8: Permissions Dialog ---
    pub fn trigger_permissions_dialog(&mut self) {
        let path = self.get_active_cursor_item_path().unwrap_or_default();
        let state = PermissionsDialogState {
            path,
            ..Default::default()
        };
        self.active_modal = ActiveModal::Permissions(state);
    }

    // --- Task 5.3.6: Internal Viewer (F3) ---
    pub fn trigger_internal_viewer(&mut self) {
        let path = self.get_active_cursor_item_path().unwrap_or_default();
        let sample_text = format!("// Duet File Viewer\n// Path: {}\n\nfn main() {{\n    println!(\"Hello Duet Viewer!\");\n}}", path);
        let state = ViewerState::new_text(&path, &sample_text);
        self.active_modal = ActiveModal::InternalViewer(state);
    }

    // --- Task 5.3.7: Search View (Alt+F7) & Feed to Panel ---
    pub fn trigger_search_view(&mut self) {
        let state = SearchDialogState {
            mask_input: InputState {
                value: "*".to_string(),
                placeholder: "File mask (e.g. *.rs)".to_string(),
                is_focused: true,
                ..Default::default()
            },
            ..Default::default()
        };
        self.active_modal = ActiveModal::SearchView(state);
    }

    pub fn trigger_feed_to_panel(&mut self) {
        let results = match &self.active_modal {
            ActiveModal::SearchView(state) => state.results.clone(),
            _ => vec![
                SearchResultEntry {
                    path: "/home/user/project/main.rs".into(),
                    size_bytes: 2048,
                    mtime_str: "2026-08-05 10:00".into(),
                },
                SearchResultEntry {
                    path: "/home/user/project/lib.rs".into(),
                    size_bytes: 4096,
                    mtime_str: "2026-08-05 11:30".into(),
                },
            ],
        };

        // Populate synthetic tab in active panel
        let active_tab = self.active_panel_state_mut().active_tab_mut();
        active_tab.path = VPath::parse("search://results").unwrap_or_default();

        let entries: Vec<EntryInput> = results
            .into_iter()
            .enumerate()
            .map(|(idx, res)| {
                let name = res.path.rsplit('/').next().unwrap_or(&res.path).to_string();
                EntryInput {
                    id: EntryId((idx + 1) as u64),
                    name,
                    file_type: FileType::File,
                    size: res.size_bytes,
                    mode: 0o644,
                    uid: 1000,
                    gid: 1000,
                    mtime: 1,
                    atime: 1,
                    ctime: 1,
                    dev: 1,
                    ino: (idx + 1) as u64,
                    nlink: 1,
                    flags: 0,
                }
            })
            .collect();

        active_tab.model.set_entries(entries);
        self.close_modal();
    }

    // --- Task 5.3.8: Quick View Panel (Ctrl+Q) ---
    pub fn trigger_quick_view(&mut self) {
        self.is_quick_view = !self.is_quick_view;
        self.update_quick_view_preview();
    }

    // --- Task 6.1.8: Pack Dialog (Alt+F5) ---
    pub fn trigger_pack(&mut self) {
        let source_files = self.get_active_selected_files();
        let state = PackDialogState {
            source_files,
            ..Default::default()
        };
        self.active_modal = ActiveModal::Pack(state);
    }

    // --- Task 6.1.9: Unpack Dialog (Alt+F9) ---
    pub fn trigger_unpack(&mut self) {
        let source_archives = self.get_active_selected_files();
        let target_dir = self.target_panel_state().active_tab().path.to_string();
        let state = UnpackDialogState {
            source_archives,
            dest_input: InputState {
                value: target_dir,
                placeholder: "Unpack destination directory path".to_string(),
                is_focused: true,
                ..Default::default()
            },
            ..Default::default()
        };
        self.active_modal = ActiveModal::Unpack(state);
    }

    // --- Task 6.1.13: Branch View (Ctrl+B) ---
    pub fn trigger_branch_view(&mut self) {
        let active_tab = self.active_panel_state_mut().active_tab_mut();
        active_tab.path = VPath::parse("branch://flat_tree").unwrap_or_default();

        let entries = vec![
            EntryInput {
                id: EntryId(1001),
                name: "src/main.rs".into(),
                file_type: FileType::File,
                size: 1024,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 1001,
                nlink: 1,
                flags: 0,
            },
            EntryInput {
                id: EntryId(1002),
                name: "src/lib.rs".into(),
                file_type: FileType::File,
                size: 2048,
                mode: 0o644,
                uid: 1000,
                gid: 1000,
                mtime: 1,
                atime: 1,
                ctime: 1,
                dev: 1,
                ino: 1002,
                nlink: 1,
                flags: 0,
            },
        ];

        active_tab.model.set_entries(entries);
    }

    // --- Task 7.1.8: Connection Manager UI ---
    pub fn trigger_connection_manager(&mut self) {
        let state = ConnectionManagerDialogState::default();
        self.active_modal = ActiveModal::ConnectionManager(Box::new(state));
    }

    // --- Task 8.1.11: Plugin Manager UI ---
    pub fn trigger_plugin_manager(&mut self) {
        let state = PluginManagerDialogState::default();
        self.active_modal = ActiveModal::PluginManager(Box::new(state));
    }

    // --- Task 9.1.1: Multi-Rename ---
    pub fn trigger_multi_rename(&mut self) {
        let state = MultiRenameDialogState::default();
        self.active_modal = ActiveModal::MultiRename(Box::new(state));
    }

    // --- Task 9.1.3: Dir Sync & Compare ---
    pub fn trigger_dir_sync(&mut self) {
        let state = DirSyncDialogState::default();
        self.active_modal = ActiveModal::DirSync(Box::new(state));
    }

    // --- Task 9.1.6: File Properties ---
    pub fn trigger_properties(&mut self) {
        let state = PropertiesDialogState::default();
        self.active_modal = ActiveModal::Properties(Box::new(state));
    }

    // --- Task 9.1.17: Settings UI ---
    pub fn trigger_settings(&mut self) {
        let state = SettingsDialogState::default();
        self.active_modal = ActiveModal::Settings(Box::new(state));
    }

    pub fn get_active_cursor_item_info(&self) -> Option<(bool, String)> {
        let tab = self.active_panel_state().active_tab();
        let len = tab.model.len();
        if len == 0 || tab.cursor.cursor_idx >= len {
            return None;
        }
        let store_idx = tab.model.view_indices()[tab.cursor.cursor_idx];
        let store = tab.model.store();
        let name = store.get_name(store_idx).to_string();
        let is_dir = store.file_type(store_idx).is_dir();
        Some((is_dir, name))
    }

    pub fn handle_left_row_click(&mut self, row_idx: usize, click_count: usize) {
        self.active_panel = ActivePanel::Left;
        let tab = self.left_panel.active_tab_mut();
        if row_idx < tab.model.len() {
            tab.cursor.cursor_idx = row_idx;
        }

        if click_count >= 2 {
            let cursor_item = self.get_active_cursor_item_info();
            if let Some((is_dir, name)) = cursor_item {
                if name == ".." {
                    self.active_panel_state_mut().navigate_parent();
                } else if is_dir {
                    let current_path = self.active_panel_state().active_tab().path.clone();
                    let clean = current_path.path.trim_end_matches('/');
                    let new_path = VPath::new_local(format!("{clean}/{name}"));
                    self.active_panel_state_mut().navigate_to(new_path);
                } else {
                    self.trigger_internal_viewer();
                }
            }
        }
    }

    pub fn handle_right_row_click(&mut self, row_idx: usize, click_count: usize) {
        self.active_panel = ActivePanel::Right;
        let tab = self.right_panel.active_tab_mut();
        if row_idx < tab.model.len() {
            tab.cursor.cursor_idx = row_idx;
        }

        if click_count >= 2 {
            let cursor_item = self.get_active_cursor_item_info();
            if let Some((is_dir, name)) = cursor_item {
                if name == ".." {
                    self.active_panel_state_mut().navigate_parent();
                } else if is_dir {
                    let current_path = self.active_panel_state().active_tab().path.clone();
                    let clean = current_path.path.trim_end_matches('/');
                    let new_path = VPath::new_local(format!("{clean}/{name}"));
                    self.active_panel_state_mut().navigate_to(new_path);
                } else {
                    self.trigger_internal_viewer();
                }
            }
        }
    }

    pub fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let alt = event.keystroke.modifiers.alt;

        if !matches!(self.active_modal, ActiveModal::None) {
            match key {
                "escape" => self.close_modal(),
                "enter" => self.close_modal(),
                _ => {}
            }
            cx.notify();
            return;
        }

        match (key, ctrl) {
            ("escape", _) => {
                if self.is_quick_view {
                    self.is_quick_view = false;
                    self.quick_view_state = None;
                }
            }
            ("tab", _) => {
                self.toggle_active_panel();
            }
            ("up", _) => {
                let state = self.active_panel_state_mut();
                let tab = state.active_tab_mut();
                tab.cursor.move_up();
            }
            ("down", _) => {
                let state = self.active_panel_state_mut();
                let total = state.active_tab().model.len();
                state.active_tab_mut().cursor.move_down(total);
            }
            ("pageup", _) => {
                let state = self.active_panel_state_mut();
                state.active_tab_mut().cursor.move_page_up();
            }
            ("pagedown", _) => {
                let state = self.active_panel_state_mut();
                let total = state.active_tab().model.len();
                state.active_tab_mut().cursor.move_page_down(total);
            }
            ("home", _) => {
                let state = self.active_panel_state_mut();
                state.active_tab_mut().cursor.move_home();
            }
            ("end", _) => {
                let state = self.active_panel_state_mut();
                let total = state.active_tab().model.len();
                state.active_tab_mut().cursor.move_end(total);
            }
            ("backspace", _) => {
                self.active_panel_state_mut().navigate_parent();
            }
            ("enter", _) => {
                let cursor_item = self.get_active_cursor_item_info();
                if let Some((is_dir, name)) = cursor_item {
                    if name == ".." {
                        self.active_panel_state_mut().navigate_parent();
                    } else if is_dir {
                        let current_path = self.active_panel_state().active_tab().path.clone();
                        let clean = current_path.path.trim_end_matches('/');
                        let new_path = VPath::new_local(format!("{clean}/{name}"));
                        self.active_panel_state_mut().navigate_to(new_path);
                    } else {
                        self.trigger_internal_viewer();
                    }
                }
            }
            ("insert", _) => {
                self.active_panel_state_mut().selection_insert_step();
            }
            ("space", _) => {
                self.active_panel_state_mut().selection_toggle_space();
            }
            ("f3", _) => self.trigger_internal_viewer(),
            ("f5", _) => {
                if alt {
                    self.trigger_pack();
                } else {
                    self.trigger_copy_dialog();
                }
            }
            ("f6", _) => self.trigger_move_dialog(),
            ("f7", _) => {
                if alt {
                    self.trigger_search_view();
                } else {
                    self.trigger_create_dir_dialog();
                }
            }
            ("f8", _) | ("delete", _) => self.trigger_delete_dialog(false),
            ("f9", _) => {
                if alt {
                    self.trigger_unpack();
                }
            }
            ("b", true) => self.trigger_branch_view(),
            ("q", true) => self.trigger_quick_view(),
            ("m", true) => self.trigger_multi_rename(),
            ("s", true) => self.trigger_dir_sync(),
            _ => {}
        }

        cx.notify();
    }

    fn update_quick_view_preview(&mut self) {
        if self.is_quick_view {
            let path = self.get_active_cursor_item_path().unwrap_or_default();
            let sample = format!("// Quick View Preview for {}\n\nContent preview placeholder...", path);
            self.quick_view_state = Some(ViewerState::new_text(&path, &sample));
        } else {
            self.quick_view_state = None;
        }
    }

    // Helper methods
    fn get_active_selected_files(&self) -> Vec<String> {
        let active_tab = self.active_panel_state().active_tab();
        let store = active_tab.model.store();
        let sel_ids = active_tab.model.selection();

        if sel_ids.is_empty() {
            if let Some(name) = self.get_active_cursor_item_name() {
                vec![name]
            } else {
                Vec::new()
            }
        } else {
            let mut list = Vec::new();
            for idx in 0..store.len() {
                if sel_ids.contains(&store.id(idx)) {
                    list.push(store.get_name(idx).to_string());
                }
            }
            list
        }
    }

    fn get_active_cursor_item_name(&self) -> Option<String> {
        let active_tab = self.active_panel_state().active_tab();
        let indices = active_tab.model.view_indices();
        if indices.is_empty() {
            return None;
        }
        let store_idx = indices[active_tab.cursor.cursor_idx.min(indices.len() - 1)];
        Some(active_tab.model.store().get_name(store_idx).to_string())
    }

    fn get_active_cursor_item_path(&self) -> Option<String> {
        let active_tab = self.active_panel_state().active_tab();
        let base_path = active_tab.path.to_string();
        let name = self.get_active_cursor_item_name()?;
        Some(format!("{}/{}", base_path.trim_end_matches('/'), name))
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = &self.theme;

        // 1. Dual Panel Splitter or Quick View Mode
        let left_widget = DirectoryPanelWidget::render(
            &self.left_panel,
            self.active_panel == ActivePanel::Left,
            theme,
            cx,
            true,
        )
        .into_any_element();

        let right_widget = if self.is_quick_view && self.active_panel == ActivePanel::Left {
            if let Some(qv_state) = &self.quick_view_state {
                QuickViewWidget::render(
                    qv_state,
                    theme.panel_bg,
                    theme.fg,
                    theme.inactive_border,
                    theme.active_border,
                )
                .into_any_element()
            } else {
                DirectoryPanelWidget::render(&self.right_panel, false, theme, cx, false)
                    .into_any_element()
            }
        } else if self.is_quick_view && self.active_panel == ActivePanel::Right {
            if let Some(qv_state) = &self.quick_view_state {
                QuickViewWidget::render(
                    qv_state,
                    theme.panel_bg,
                    theme.fg,
                    theme.inactive_border,
                    theme.active_border,
                )
                .into_any_element()
            } else {
                DirectoryPanelWidget::render(&self.left_panel, false, theme, cx, true)
                    .into_any_element()
            }
        } else {
            DirectoryPanelWidget::render(
                &self.right_panel,
                self.active_panel == ActivePanel::Right,
                theme,
                cx,
                false,
            )
            .into_any_element()
        };

        let left_pane = if self.is_quick_view && self.active_panel == ActivePanel::Right {
            if let Some(qv_state) = &self.quick_view_state {
                QuickViewWidget::render(
                    qv_state,
                    theme.panel_bg,
                    theme.fg,
                    theme.inactive_border,
                    theme.active_border,
                )
                .into_any_element()
            } else {
                DirectoryPanelWidget::render(&self.left_panel, false, theme, cx, true)
                    .into_any_element()
            }
        } else {
            left_widget
        };

        let splitter_widget = ResizableSplitter::new(self.splitter.clone());
        let dual_pane = splitter_widget.render(
            left_pane,
            right_widget,
            theme.inactive_border,
            theme.active_border,
        );

        // 2. Command Line Row
        let active_path = self.active_panel_state().active_tab().path.to_string();
        let prompt_str = format!("{}$ ", active_path);

        let cmdline_row = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(26.0))
            .bg(theme.cmdline_bg)
            .text_color(theme.cmdline_fg)
            .text_xs()
            .items_center()
            .px_2()
            .gap_2()
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(prompt_str),
            )
            .child(
                div()
                    .flex_1()
                    .child(InputWidget::render(
                        &self.cmdline_state,
                        theme.cmdline_bg,
                        theme.fg,
                        theme.inactive_border,
                        theme.active_border,
                    )),
            );

        // 3. Status Bar & Operation Tray
        let active_state = self.active_panel_state();
        let active_tab = active_state.active_tab();
        let (sel_count, sel_bytes) = active_tab.model.selection_stats();

        let status_data = StatusBarData {
            message: String::new(),
            active_path: active_tab.path.to_string(),
            item_count: active_tab.model.len(),
            selected_count: sel_count,
            total_selected_bytes: sel_bytes,
            free_bytes: Some(active_state.free_bytes),
            total_bytes: Some(active_state.total_bytes),
        };

        let main_status_bar = StatusBar::render(
            &status_data,
            theme.status_bar_bg,
            theme.status_bar_fg,
            theme.status_bar_subtle_fg,
        );

        let progress_tray = StatusProgressTray::render(
            &self.progress_tray,
            theme.status_bar_bg,
            theme.status_bar_fg,
            theme.status_bar_subtle_fg,
        );

        let status_row = div()
            .flex()
            .flex_row()
            .w_full()
            .items_center()
            .child(div().flex_1().child(main_status_bar))
            .child(progress_tray);

        // 4. Function Key Bar (F1..F12)
        let function_bar = FunctionBar::render(
            theme.function_bar_bg,
            theme.function_bar_button_bg,
            theme.function_bar_key_fg,
            theme.function_bar_label_fg,
        );

        // 5. Active Modal Overlay
        let modal_overlay = match &self.active_modal {
            ActiveModal::None => div(),
            ActiveModal::CopyMove(state) => CopyMoveDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::ProgressManager(state) => OperationManagerModal::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
            ),
            ActiveModal::ConflictResolution(state) => ConflictResolutionDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
            ),
            ActiveModal::ErrorReport(state) => ErrorReportDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
            ),
            ActiveModal::StartupRecovery(state) => StartupRecoveryOverlay::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
            ),
            ActiveModal::DeleteConfirmation(state) => DeleteConfirmationDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
            ),
            ActiveModal::CreateDir(state) => CreateDirDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Rename(state) => RenameDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::CreateLink(state) => CreateLinkDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Permissions(state) => PermissionsDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::InternalViewer(state) => InternalViewerWidget::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::SearchView(state) => SearchViewWidget::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Pack(state) => PackDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Unpack(state) => UnpackDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::ConnectionManager(state) => ConnectionManagerDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::PluginManager(state) => PluginManagerDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::MultiRename(state) => MultiRenameDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::DirSync(state) => DirSyncDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Properties(state) => PropertiesDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
            ActiveModal::Settings(state) => SettingsDialog::render(
                state,
                theme.panel_bg,
                theme.fg,
                theme.inactive_border,
                theme.active_border,
            ),
        };

        let drive_bar = DriveBar::render(
            &self.drive_bar,
            theme.table_header_bg,
            theme.fg,
            theme.status_bar_subtle_fg,
            theme.active_border,
        );

        let button_bar = ButtonBar::render(
            &self.button_bar,
            theme.table_header_bg,
            theme.fg,
            theme.active_border,
        );

        let terminal_panel = TerminalPanelWidget::render(
            &self.terminal_panel,
            theme.panel_bg,
            theme.fg,
            theme.active_border,
        );

        let mut root_div = div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .on_key_down(cx.listener(Self::handle_key_down));

        if let Some(fh) = &self.focus_handle {
            root_div = root_div.track_focus(fh);
        }

        root_div
            .child(drive_bar)
            .child(button_bar)
            .child(div().flex_1().w_full().child(dual_pane))
            .child(terminal_panel)
            .child(cmdline_row)
            .child(status_row)
            .child(function_bar)
            .child(modal_overlay)
    }
}
