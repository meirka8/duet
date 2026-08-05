//! Copy/Move Dialog (F5/F6, T-5.2.1) widget implementation.

use duet_ops::ConflictPolicy;
use gpui::*;

use crate::input::{InputState, InputWidget};

/// Options for copy/move operation.
#[derive(Debug, Clone)]
pub struct CopyMoveOptions {
    pub overwrite_policy: ConflictPolicy,
    pub run_in_background: bool,
}

impl Default for CopyMoveOptions {
    fn default() -> Self {
        Self {
            overwrite_policy: ConflictPolicy::AskUser,
            run_in_background: false,
        }
    }
}

/// State for the Copy/Move dialog.
#[derive(Debug, Clone, Default)]
pub struct CopyMoveDialogState {
    pub is_move: bool,
    pub dest_input: InputState,
    pub options: CopyMoveOptions,
    pub source_files: Vec<String>,
}

pub struct CopyMoveDialog;

impl CopyMoveDialog {
    pub fn render(
        state: &CopyMoveDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = if state.is_move {
            "Move Files (F6)"
        } else {
            "Copy Files (F5)"
        };

        let src_summary = if state.source_files.is_empty() {
            "No files selected".to_string()
        } else if state.source_files.len() == 1 {
            format!("File: {}", state.source_files[0])
        } else {
            format!("Selected: {} items", state.source_files.len())
        };

        let policy_str = match state.options.overwrite_policy {
            ConflictPolicy::AskUser => "Ask user on conflict",
            ConflictPolicy::OverwriteAll => "Overwrite all",
            ConflictPolicy::OverwriteOlder => "Overwrite if older",
            ConflictPolicy::OverwriteDifferentSize => "Overwrite if different size",
            ConflictPolicy::SkipAll => "Skip existing",
            ConflictPolicy::AutoRenameAll => "Auto-rename target",
            ConflictPolicy::Cancel => "Cancel on conflict",
        };

        let bg_mode_str = if state.options.run_in_background {
            "[x] Queue in background"
        } else {
            "[ ] Immediate execution"
        };

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(520.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child(src_summary),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().child("Destination directory:"))
                            .child(InputWidget::render(
                                &state.dest_input,
                                bg,
                                fg,
                                border_color,
                                active_border,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .text_xs()
                            .child(div().child(format!("Overwrite policy: {policy_str}")))
                            .child(div().child(bg_mode_str)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .pt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(active_border)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("OK (Enter)"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x444444ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Queue (F2)"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Cancel (Esc)"),
                            ),
                    ),
            )
    }
}
