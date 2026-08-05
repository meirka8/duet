//! `WorkspaceView` dual-panel workspace layout containing panels, splitter, command line, function key bar, and status bar.

use crate::panel::{DirectoryPanelState, DirectoryPanelWidget};
use crate::theme::ThemeTokens;
use duet_widgets::{
    FunctionBar, InputState, InputWidget, ResizableSplitter, SplitDirection, SplitterState,
    StatusBar, StatusBarData,
};
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    Left,
    Right,
}

pub struct WorkspaceView {
    pub left_panel: DirectoryPanelState,
    pub right_panel: DirectoryPanelState,
    pub active_panel: ActivePanel,
    pub splitter: SplitterState,
    pub cmdline_state: InputState,
    pub theme: ThemeTokens,
    pub focus_handle: FocusHandle,
}

impl WorkspaceView {
    pub fn new(cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();
        let cmdline_state = InputState {
            placeholder: "Enter shell command...".to_string(),
            ..Default::default()
        };

        Self {
            left_panel: DirectoryPanelState::default(),
            right_panel: DirectoryPanelState::default(),
            active_panel: ActivePanel::Left,
            splitter: SplitterState::new(0.50, SplitDirection::Horizontal),
            cmdline_state,
            theme: ThemeTokens::dark(),
            focus_handle,
        }
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

    pub fn toggle_active_panel(&mut self) {
        self.active_panel = match self.active_panel {
            ActivePanel::Left => ActivePanel::Right,
            ActivePanel::Right => ActivePanel::Left,
        };
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = &self.theme;

        // 1. Dual Panel Splitter
        let left_widget = DirectoryPanelWidget::render(
            &self.left_panel,
            self.active_panel == ActivePanel::Left,
            theme,
        );

        let right_widget = DirectoryPanelWidget::render(
            &self.right_panel,
            self.active_panel == ActivePanel::Right,
            theme,
        );

        let splitter_widget = ResizableSplitter::new(self.splitter.clone());
        let dual_pane = splitter_widget.render(
            left_widget,
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

        // 3. Status Bar
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

        let status_bar = StatusBar::render(
            &status_data,
            theme.status_bar_bg,
            theme.status_bar_fg,
            theme.status_bar_subtle_fg,
        );

        // 4. Function Key Bar (F1..F12)
        let function_bar = FunctionBar::render(
            theme.function_bar_bg,
            theme.function_bar_button_bg,
            theme.function_bar_key_fg,
            theme.function_bar_label_fg,
        );

        div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .child(div().flex_1().w_full().child(dual_pane))
            .child(cmdline_row)
            .child(status_bar)
            .child(function_bar)
    }
}
