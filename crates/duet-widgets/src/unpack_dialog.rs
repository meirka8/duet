//! Unpack Dialog (Alt+F9, T-6.1.9) widget implementation.

use duet_ops::ConflictPolicy;
use gpui::*;

use crate::input::{InputState, InputWidget};

/// State for the Unpack Archive dialog.
#[derive(Debug, Clone)]
pub struct UnpackDialogState {
    pub dest_input: InputState,
    pub create_subfolder: bool,
    pub overwrite_policy: ConflictPolicy,
    pub source_archives: Vec<String>,
}

impl Default for UnpackDialogState {
    fn default() -> Self {
        Self {
            dest_input: InputState {
                value: String::new(),
                placeholder: "Unpack destination directory path".to_string(),
                is_focused: true,
                ..Default::default()
            },
            create_subfolder: true,
            overwrite_policy: ConflictPolicy::AskUser,
            source_archives: Vec::new(),
        }
    }
}

pub struct UnpackDialog;

impl UnpackDialog {
    pub fn render(
        state: &UnpackDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Unpack Archive Files (Alt+F9)";

        let src_summary = if state.source_archives.is_empty() {
            "No archive selected".to_string()
        } else if state.source_archives.len() == 1 {
            format!("Archive: {}", state.source_archives[0])
        } else {
            format!("Archives selected: {} items", state.source_archives.len())
        };

        let subfolder_toggle_str = if state.create_subfolder {
            "[x] Extract into subfolder named after archive"
        } else {
            "[ ] Unpack directly into target destination directory"
        };

        let policy_str = match state.overwrite_policy {
            ConflictPolicy::AskUser => "Ask user on conflict",
            ConflictPolicy::OverwriteAll => "Overwrite existing files",
            ConflictPolicy::SkipAll => "Skip existing files",
            ConflictPolicy::AutoRenameAll => "Auto-rename extracted files",
            _ => "Ask user on conflict",
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
                            .child(div().text_xs().child("Unpack destination path:"))
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
                            .text_xs()
                            .text_color(rgba(0xaaaaaaff))
                            .child(subfolder_toggle_str),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .text_xs()
                            .child(div().child(format!("Overwrite policy: {policy_str}"))),
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
                                    .child("Unpack (Enter)"),
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
