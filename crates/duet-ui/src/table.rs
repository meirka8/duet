//! `FileTable` virtualized table view over `EntryStore` & `DirectoryModel`.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

use crate::icons::resolve_icon;
use crate::theme::ThemeTokens;
use duet_index::{DirectoryModel, SortColumn, SortDirection};
use duet_types::{EntryId, FileType};
use duet_widgets::{TableColumnConfig, TableWidget, TextAlignment};
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Full,
    Brief,
    Thumbnails,
    Tree,
}

/// Active column configuration state.
#[derive(Debug, Clone)]
pub struct ColumnLayout {
    pub show_size: bool,
    pub show_mtime: bool,
    pub show_mode: bool,
    pub name_width: f32,
    pub size_width: f32,
    pub mtime_width: f32,
    pub mode_width: f32,
}

impl Default for ColumnLayout {
    fn default() -> Self {
        Self {
            show_size: true,
            show_mtime: true,
            show_mode: true,
            name_width: 240.0,
            size_width: 100.0,
            mtime_width: 150.0,
            mode_width: 80.0,
        }
    }
}

/// Navigation and cursor state inside the file table.
#[derive(Debug, Clone, Default)]
pub struct CursorState {
    pub cursor_idx: usize,
    pub scroll_top: usize,
    pub page_size: usize,
}

impl CursorState {
    pub fn move_up(&mut self) {
        if self.cursor_idx > 0 {
            self.cursor_idx -= 1;
            self.adjust_scroll();
        }
    }

    pub fn move_down(&mut self, total: usize) {
        if total > 0 && self.cursor_idx + 1 < total {
            self.cursor_idx += 1;
            self.adjust_scroll();
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_idx = 0;
        self.adjust_scroll();
    }

    pub fn move_end(&mut self, total: usize) {
        if total > 0 {
            self.cursor_idx = total - 1;
            self.adjust_scroll();
        }
    }

    pub fn move_page_up(&mut self) {
        let step = if self.page_size > 0 { self.page_size } else { 10 };
        if self.cursor_idx >= step {
            self.cursor_idx -= step;
        } else {
            self.cursor_idx = 0;
        }
        self.adjust_scroll();
    }

    pub fn move_page_down(&mut self, total: usize) {
        let step = if self.page_size > 0 { self.page_size } else { 10 };
        if total > 0 {
            if self.cursor_idx + step < total {
                self.cursor_idx += step;
            } else {
                self.cursor_idx = total - 1;
            }
            self.adjust_scroll();
        }
    }

    pub fn adjust_scroll(&mut self) {
        if self.page_size == 0 {
            self.page_size = 20;
        }
        if self.cursor_idx < self.scroll_top {
            self.scroll_top = self.cursor_idx;
        } else if self.cursor_idx >= self.scroll_top + self.page_size {
            self.scroll_top = self.cursor_idx - self.page_size + 1;
        }
    }
}

pub struct FileTable;

impl FileTable {
    pub fn render(
        model: &DirectoryModel,
        cursor: &CursorState,
        view_mode: ViewMode,
        layout: &ColumnLayout,
        sort_col: SortColumn,
        sort_dir: SortDirection,
        theme: &ThemeTokens,
    ) -> Div {
        let view_indices = model.view_indices();
        let store = model.store();
        let selection = model.selection();

        let _total_count = view_indices.len();

        match view_mode {
            ViewMode::Full => Self::render_full_mode(
                store,
                view_indices,
                selection,
                cursor,
                layout,
                sort_col,
                sort_dir,
                theme,
            ),
            ViewMode::Brief => Self::render_brief_mode(
                store,
                view_indices,
                selection,
                cursor,
                theme,
            ),
            ViewMode::Thumbnails => Self::render_thumbnails_mode(
                store,
                view_indices,
                selection,
                cursor,
                theme,
            ),
            ViewMode::Tree => Self::render_tree_mode(
                store,
                view_indices,
                selection,
                cursor,
                theme,
            ),
        }
    }

    fn render_full_mode(
        store: &duet_index::EntryStore,
        indices: &[usize],
        selection: &std::collections::HashSet<EntryId>,
        cursor: &CursorState,
        layout: &ColumnLayout,
        sort_col: SortColumn,
        sort_dir: SortDirection,
        theme: &ThemeTokens,
    ) -> Div {
        let sorted_col_str = match sort_col {
            SortColumn::Name => "name",
            SortColumn::Size => "size",
            SortColumn::Mtime => "mtime",
            SortColumn::Mode => "mode",
            _ => "name",
        };

        let mut columns = vec![TableColumnConfig {
            id: "name".to_string(),
            title: "Name".to_string(),
            width_px: layout.name_width,
            alignment: TextAlignment::Left,
        }];

        if layout.show_size {
            columns.push(TableColumnConfig {
                id: "size".to_string(),
                title: "Size".to_string(),
                width_px: layout.size_width,
                alignment: TextAlignment::Right,
            });
        }
        if layout.show_mtime {
            columns.push(TableColumnConfig {
                id: "mtime".to_string(),
                title: "Date".to_string(),
                width_px: layout.mtime_width,
                alignment: TextAlignment::Left,
            });
        }
        if layout.show_mode {
            columns.push(TableColumnConfig {
                id: "mode".to_string(),
                title: "Mode".to_string(),
                width_px: layout.mode_width,
                alignment: TextAlignment::Left,
            });
        }

        let header = TableWidget::render_header(
            &columns,
            Some(sorted_col_str),
            sort_dir == SortDirection::Ascending,
            theme.table_header_bg,
            theme.table_header_fg,
            theme.inactive_border,
        );

        let mut body = div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .overflow_hidden();

        let visible_count = cursor.page_size.max(20);
        let start = cursor.scroll_top.min(indices.len());
        let end = (start + visible_count).min(indices.len());

        for i in start..end {
            let store_idx = indices[i];
            let id = store.id(store_idx);
            let name = store.get_name(store_idx);
            let file_type = store.file_type(store_idx);
            let size = store.size(store_idx);
            let mtime = store.mtime(store_idx);
            let mode = store.mode(store_idx);

            let is_cursor = i == cursor.cursor_idx;
            let is_selected = selection.contains(&id);

            let icon = resolve_icon(name, file_type);

            let row_bg = if is_cursor {
                theme.cursor_bg
            } else if is_selected {
                theme.selection_bg
            } else if i % 2 == 1 {
                theme.table_row_alt_bg
            } else {
                theme.panel_bg
            };

            let fg_color = if is_selected {
                theme.selection_fg
            } else if file_type == FileType::Directory {
                theme.dir_fg
            } else if mode & 0o111 != 0 {
                theme.executable_fg
            } else if file_type == FileType::Symlink {
                theme.symlink_fg
            } else {
                theme.fg
            };

            let mut row = div()
                .flex()
                .flex_row()
                .w_full()
                .h(px(20.0))
                .bg(row_bg)
                .text_color(fg_color)
                .text_xs()
                .items_center();

            if is_cursor {
                row = row.border_1().border_color(theme.cursor_border);
            }

            // Name Cell
            let name_cell = div()
                .flex_none()
                .w(px(layout.name_width))
                .px_2()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .truncate()
                .child(icon.glyph())
                .child(name.to_string());
            row = row.child(name_cell);

            // Size Cell
            if layout.show_size {
                let size_str = if file_type == FileType::Directory {
                    "<DIR>".to_string()
                } else {
                    format_size(size)
                };
                let size_cell = div()
                    .flex_none()
                    .w(px(layout.size_width))
                    .px_2()
                    .truncate()
                    .child(size_str);
                row = row.child(size_cell);
            }

            // Mtime Cell
            if layout.show_mtime {
                let mtime_str = format_mtime(mtime);
                let mtime_cell = div()
                    .flex_none()
                    .w(px(layout.mtime_width))
                    .px_2()
                    .truncate()
                    .child(mtime_str);
                row = row.child(mtime_cell);
            }

            // Mode Cell
            if layout.show_mode {
                let mode_str = format!("{:o}", mode & 0o777);
                let mode_cell = div()
                    .flex_none()
                    .w(px(layout.mode_width))
                    .px_2()
                    .truncate()
                    .child(mode_str);
                row = row.child(mode_cell);
            }

            body = body.child(row);
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(header)
            .child(body)
    }

    fn render_brief_mode(
        store: &duet_index::EntryStore,
        indices: &[usize],
        selection: &std::collections::HashSet<EntryId>,
        cursor: &CursorState,
        theme: &ThemeTokens,
    ) -> Div {
        let mut grid = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .w_full()
            .flex_1()
            .p_2()
            .gap_2()
            .overflow_hidden();

        let visible_count = cursor.page_size.max(40);
        let start = cursor.scroll_top.min(indices.len());
        let end = (start + visible_count).min(indices.len());

        for i in start..end {
            let store_idx = indices[i];
            let id = store.id(store_idx);
            let name = store.get_name(store_idx);
            let file_type = store.file_type(store_idx);

            let is_cursor = i == cursor.cursor_idx;
            let is_selected = selection.contains(&id);
            let icon = resolve_icon(name, file_type);

            let item_bg = if is_cursor {
                theme.cursor_bg
            } else if is_selected {
                theme.selection_bg
            } else {
                theme.panel_bg
            };

            let item = div()
                .w(px(160.0))
                .h(px(22.0))
                .bg(item_bg)
                .px_2()
                .rounded_sm()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .text_xs()
                .text_color(if is_selected { theme.selection_fg } else { theme.fg })
                .truncate()
                .child(icon.glyph())
                .child(name.to_string());

            grid = grid.child(item);
        }

        grid
    }

    fn render_thumbnails_mode(
        store: &duet_index::EntryStore,
        indices: &[usize],
        selection: &std::collections::HashSet<EntryId>,
        cursor: &CursorState,
        theme: &ThemeTokens,
    ) -> Div {
        let mut grid = div()
            .flex()
            .flex_row()
            .flex_wrap()
            .w_full()
            .flex_1()
            .p_2()
            .gap_3()
            .overflow_hidden();

        let visible_count = cursor.page_size.max(30);
        let start = cursor.scroll_top.min(indices.len());
        let end = (start + visible_count).min(indices.len());

        for i in start..end {
            let store_idx = indices[i];
            let id = store.id(store_idx);
            let name = store.get_name(store_idx);
            let file_type = store.file_type(store_idx);

            let is_cursor = i == cursor.cursor_idx;
            let is_selected = selection.contains(&id);
            let icon = resolve_icon(name, file_type);

            let card_bg = if is_cursor {
                theme.cursor_bg
            } else if is_selected {
                theme.selection_bg
            } else {
                theme.table_header_bg
            };

            let card = div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .w(px(90.0))
                .h(px(90.0))
                .bg(card_bg)
                .rounded_md()
                .p_2()
                .text_xs()
                .text_color(theme.fg)
                .child(div().text_2xl().child(icon.glyph()))
                .child(
                    div()
                        .mt_1()
                        .truncate()
                        .max_w(px(80.0))
                        .child(name.to_string()),
                );

            grid = grid.child(card);
        }

        grid
    }

    fn render_tree_mode(
        store: &duet_index::EntryStore,
        indices: &[usize],
        selection: &std::collections::HashSet<EntryId>,
        cursor: &CursorState,
        theme: &ThemeTokens,
    ) -> Div {
        let mut list = div()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .p_2()
            .overflow_hidden();

        let visible_count = cursor.page_size.max(20);
        let start = cursor.scroll_top.min(indices.len());
        let end = (start + visible_count).min(indices.len());

        for i in start..end {
            let store_idx = indices[i];
            let id = store.id(store_idx);
            let name = store.get_name(store_idx);
            let file_type = store.file_type(store_idx);

            let is_cursor = i == cursor.cursor_idx;
            let is_selected = selection.contains(&id);
            let icon = resolve_icon(name, file_type);

            let indent = if file_type == FileType::Directory { "├── " } else { "│   " };

            let row_bg = if is_cursor {
                theme.cursor_bg
            } else if is_selected {
                theme.selection_bg
            } else {
                theme.panel_bg
            };

            let row = div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(20.0))
                .px_2()
                .bg(row_bg)
                .text_xs()
                .text_color(theme.fg)
                .child(div().text_color(theme.status_bar_subtle_fg).child(indent))
                .child(icon.glyph())
                .child(div().ml_1().child(name.to_string()));

            list = list.child(row);
        }

        list
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} K", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn format_mtime(mtime: i64) -> String {
    if mtime <= 0 {
        return "-".to_string();
    }
    // Simple readable ISO timestamp helper without chrono dep
    let secs = mtime as u64;
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let month = ((days % 365) / 30) + 1;
    let day = (days % 30) + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}
