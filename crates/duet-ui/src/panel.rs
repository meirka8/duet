//! `DirectoryPanel` state and view representation.

use crate::table::{ColumnLayout, CursorState, FileTable, ViewMode};
use crate::theme::ThemeTokens;
use duet_index::{DirectoryModel, EntryInput, FilterSpec, SortColumn, SortDirection};
use duet_types::{EntryId, FileType, VPath};
use duet_widgets::{TabBar, TabBarColors, TabItem};
use gpui::*;

#[derive(Debug, Clone)]
pub struct PanelTab {
    pub id: usize,
    pub path: VPath,
    pub model: DirectoryModel,
    pub cursor: CursorState,
    pub view_mode: ViewMode,
    pub history_back: Vec<VPath>,
    pub history_forward: Vec<VPath>,
    pub is_locked: bool,
    pub lock_dir_change: bool,
}

impl PanelTab {
    pub fn new(id: usize, path: VPath) -> Self {
        let mut model = DirectoryModel::new();
        // Load initial dummy / parent entry if applicable
        let entries = vec![EntryInput {
            id: EntryId(0),
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
            ino: 0,
            nlink: 2,
            flags: 0,
        }];
        model.set_entries(entries);

        Self {
            id,
            path,
            model,
            cursor: CursorState::default(),
            view_mode: ViewMode::Full,
            history_back: Vec::new(),
            history_forward: Vec::new(),
            is_locked: false,
            lock_dir_change: false,
        }
    }

    pub fn title(&self) -> String {
        self.path.file_name().unwrap_or("/").to_string()
    }
}

/// Panel state containing tabs, column settings, filter overlays, and active panel focus.
#[derive(Debug, Clone)]
pub struct DirectoryPanelState {
    pub tabs: Vec<PanelTab>,
    pub active_tab_idx: usize,
    pub column_layout: ColumnLayout,
    pub sort_col: SortColumn,
    pub sort_dir: SortDirection,
    pub free_bytes: u64,
    pub total_bytes: u64,
    pub quick_search_query: Option<String>,
    pub quick_filter_query: Option<String>,
    pub next_tab_id: usize,
}

impl Default for DirectoryPanelState {
    fn default() -> Self {
        let root_tab = PanelTab::new(1, VPath::parse("/").unwrap_or_default());
        Self {
            tabs: vec![root_tab],
            active_tab_idx: 0,
            column_layout: ColumnLayout::default(),
            sort_col: SortColumn::Name,
            sort_dir: SortDirection::Ascending,
            free_bytes: 120_000_000_000, // 120 GB
            total_bytes: 500_000_000_000, // 500 GB
            quick_search_query: None,
            quick_filter_query: None,
            next_tab_id: 2,
        }
    }
}

impl DirectoryPanelState {
    pub fn active_tab(&self) -> &PanelTab {
        &self.tabs[self.active_tab_idx]
    }

    pub fn active_tab_mut(&mut self) -> &mut PanelTab {
        &mut self.tabs[self.active_tab_idx]
    }

    // --- Tab Operations (T-4.3.2) ---
    pub fn create_tab(&mut self, path: VPath) {
        let new_id = self.next_tab_id;
        self.next_tab_id += 1;
        let tab = PanelTab::new(new_id, path);
        self.tabs.push(tab);
        self.active_tab_idx = self.tabs.len() - 1;
    }

    pub fn close_active_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab_idx);
            if self.active_tab_idx >= self.tabs.len() {
                self.active_tab_idx = self.tabs.len() - 1;
            }
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab_idx = (self.active_tab_idx + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab_idx == 0 {
                self.active_tab_idx = self.tabs.len() - 1;
            } else {
                self.active_tab_idx -= 1;
            }
        }
    }

    pub fn toggle_tab_lock(&mut self) {
        let tab = self.active_tab_mut();
        tab.is_locked = !tab.is_locked;
    }

    // --- Navigation (T-4.3.1) ---
    pub fn navigate_to(&mut self, new_path: VPath) {
        let tab = self.active_tab_mut();
        if tab.is_locked && !tab.lock_dir_change {
            // Lock mode opens new tab instead
            let new_id = self.next_tab_id;
            self.next_tab_id += 1;
            let new_tab = PanelTab::new(new_id, new_path);
            self.tabs.push(new_tab);
            self.active_tab_idx = self.tabs.len() - 1;
            return;
        }

        let old_path = tab.path.clone();
        tab.history_back.push(old_path);
        tab.history_forward.clear();
        tab.path = new_path;
        tab.cursor.move_home();
    }

    pub fn navigate_parent(&mut self) {
        let active = self.active_tab();
        let current_path = active.path.clone();
        if let Some(parent) = current_path.parent() {
            let child_name = current_path.file_name().map(|s| s.to_string());
            self.navigate_to(parent);

            // Restore cursor position to matching child directory (T-4.3.1 AC)
            if let Some(child_name) = child_name {
                let tab = self.active_tab_mut();
                let indices = tab.model.view_indices();
                let store = tab.model.store();
                for (idx, &store_idx) in indices.iter().enumerate() {
                    if store.get_name(store_idx) == child_name {
                        tab.cursor.cursor_idx = idx;
                        tab.cursor.adjust_scroll();
                        break;
                    }
                }
            }
        }
    }

    pub fn history_back(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(prev) = tab.history_back.pop() {
            let curr = tab.path.clone();
            tab.history_forward.push(curr);
            tab.path = prev;
            tab.cursor.move_home();
        }
    }

    pub fn history_forward(&mut self) {
        let tab = self.active_tab_mut();
        if let Some(next) = tab.history_forward.pop() {
            let curr = tab.path.clone();
            tab.history_back.push(curr);
            tab.path = next;
            tab.cursor.move_home();
        }
    }

    // --- Selection Shortcuts (T-4.2.3) ---
    pub fn selection_insert_step(&mut self) {
        let tab = self.active_tab_mut();
        let total = tab.model.len();
        if total == 0 {
            return;
        }
        let store_idx = tab.model.view_indices()[tab.cursor.cursor_idx];
        let id = tab.model.store().id(store_idx);
        tab.model.toggle_selection(id);
        tab.cursor.move_down(total);
    }

    pub fn selection_toggle_space(&mut self) {
        let tab = self.active_tab_mut();
        let total = tab.model.len();
        if total == 0 {
            return;
        }
        let store_idx = tab.model.view_indices()[tab.cursor.cursor_idx];
        let id = tab.model.store().id(store_idx);
        tab.model.toggle_selection(id);
    }

    pub fn select_all(&mut self) {
        let tab = self.active_tab_mut();
        tab.model.select_all();
    }

    pub fn invert_selection(&mut self) {
        let tab = self.active_tab_mut();
        tab.model.invert_selection();
    }

    pub fn match_extension_selection(&mut self) {
        let tab = self.active_tab_mut();
        let total = tab.model.len();
        if total == 0 {
            return;
        }
        let store_idx = tab.model.view_indices()[tab.cursor.cursor_idx];
        let name = tab.model.store().get_name(store_idx);
        if let Some(ext) = name.rsplit('.').next() {
            let mask = format!("*.{ext}");
            tab.model.select_by_pattern(&mask);
        }
    }

    // --- Quick Search & Filter (T-4.3.3) ---
    pub fn update_quick_search(&mut self, query: Option<String>) {
        self.quick_search_query = query.clone();
        if let Some(q) = query {
            if q.is_empty() {
                return;
            }
            let tab = self.active_tab_mut();
            let indices = tab.model.view_indices();
            let store = tab.model.store();
            let q_lower = q.to_lowercase();
            for (idx, &store_idx) in indices.iter().enumerate() {
                let name = store.get_name(store_idx).to_lowercase();
                if name.starts_with(&q_lower) {
                    tab.cursor.cursor_idx = idx;
                    tab.cursor.adjust_scroll();
                    break;
                }
            }
        }
    }

    pub fn update_quick_filter(&mut self, query: Option<String>) {
        self.quick_filter_query = query.clone();
        let tab = self.active_tab_mut();
        tab.model.filter(FilterSpec {
            show_hidden: true,
            quick_filter: query.clone(),
            mask: None,
        });
        tab.cursor.move_home();
    }

    // --- Column Header Click Sort (T-4.2.4) ---
    pub fn toggle_column_sort(&mut self, col: SortColumn) {
        if self.sort_col == col {
            self.sort_dir = match self.sort_dir {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_col = col;
            self.sort_dir = SortDirection::Ascending;
        }
        let sort_col = self.sort_col;
        let sort_dir = self.sort_dir;
        let tab = self.active_tab_mut();
        tab.model.sort(sort_col, sort_dir);
    }
}

pub struct DirectoryPanelWidget;

impl DirectoryPanelWidget {
    pub fn render(
        state: &DirectoryPanelState,
        is_active: bool,
        theme: &ThemeTokens,
    ) -> Div {
        let border_color = if is_active {
            theme.active_border
        } else {
            theme.inactive_border
        };

        let active_tab = state.active_tab();

        // 1. Tab Bar
        let tab_items: Vec<TabItem> = state
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, t)| TabItem {
                id: t.id,
                title: t.title(),
                path: t.path.to_string(),
                is_active: idx == state.active_tab_idx,
                is_locked: t.is_locked,
                lock_dir_change: t.lock_dir_change,
            })
            .collect();

        let tab_bar = TabBar::render(
            &tab_items,
            TabBarColors {
                bg: theme.table_header_bg,
                active_bg: theme.panel_bg,
                inactive_bg: theme.table_row_alt_bg,
                fg: theme.fg,
                active_fg: theme.dir_fg,
                border_color: theme.inactive_border,
            },
            || {},
            |_| {},
            |_| {},
        );

        // 2. Header (Path bar + Free space)
        let free_gb = state.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let header = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(24.0))
            .bg(theme.table_header_bg)
            .text_color(theme.fg)
            .text_xs()
            .items_center()
            .justify_between()
            .px_2()
            .border_b_1()
            .border_color(theme.inactive_border)
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .truncate()
                    .child(active_tab.path.to_string()),
            )
            .child(
                div()
                    .text_color(theme.status_bar_subtle_fg)
                    .child(format!("{:.1} GB free", free_gb)),
            );

        // 3. File Table
        let table = FileTable::render(
            &active_tab.model,
            &active_tab.cursor,
            active_tab.view_mode,
            &state.column_layout,
            state.sort_col,
            state.sort_dir,
            theme,
        );

        // 4. Footer (Selection Stats T-4.2.7)
        let (sel_count, sel_bytes) = active_tab.model.selection_stats();
        let total_count = active_tab.model.len();
        let footer_str = if sel_count > 0 {
            format!(
                "Selected: {} / {} entries ({})",
                sel_count,
                total_count,
                duet_widgets::status_bar::format_bytes(sel_bytes)
            )
        } else {
            format!("{} entries", total_count)
        };

        let mut footer = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(20.0))
            .bg(theme.table_header_bg)
            .text_color(theme.fg)
            .text_xs()
            .items_center()
            .justify_between()
            .px_2()
            .border_t_1()
            .border_color(theme.inactive_border)
            .child(footer_str);

        // Quick Search / Filter Overlays
        if let Some(qs) = &state.quick_search_query {
            footer = footer.child(
                div()
                    .px_2()
                    .rounded_sm()
                    .bg(theme.quick_search_bg)
                    .text_color(theme.quick_search_fg)
                    .child(format!("Jump: {}", qs)),
            );
        } else if let Some(qf) = &state.quick_filter_query {
            footer = footer.child(
                div()
                    .px_2()
                    .rounded_sm()
                    .bg(theme.quick_search_bg)
                    .text_color(theme.quick_search_fg)
                    .child(format!("Filter: {} ({} matches)", qf, active_tab.model.len())),
            );
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.panel_bg)
            .border_2()
            .border_color(border_color)
            .child(tab_bar)
            .child(header)
            .child(table)
            .child(footer)
    }
}
