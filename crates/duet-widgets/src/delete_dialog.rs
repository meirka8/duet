//! Delete Confirmation (F8, T-5.2.6) widget implementation.

use gpui::*;

/// State for Delete Confirmation dialog.
#[derive(Debug, Clone, Default)]
pub struct DeleteDialogState {
    pub items: Vec<String>,
    pub use_trash: bool,
    pub shift_pressed: bool,
}

pub struct DeleteConfirmationDialog;

impl DeleteConfirmationDialog {
    pub fn render(
        state: &DeleteDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let is_permanent = !state.use_trash || state.shift_pressed;

        let title = if is_permanent {
            "Permanent Delete Confirmation (Shift+F8)"
        } else {
            "Move to Trash Confirmation (F8)"
        };

        let warning_color = if is_permanent {
            rgba(0xef4444ff) // Red
        } else {
            rgba(0xeab308ff) // Yellow
        };

        let action_label = if is_permanent {
            "PERMANENTLY DELETE"
        } else {
            "Move to Trash"
        };

        let item_summary = if state.items.is_empty() {
            "No files selected".to_string()
        } else if state.items.len() == 1 {
            format!("Target: {}", state.items[0])
        } else {
            format!("Target: {} items selected", state.items.len())
        };

        let mode_checkbox_str = if is_permanent {
            "[ ] Move to Trash (Shift+Del detected: Permanent Unlink)"
        } else {
            "[x] Move to Trash (Freedesktop Trash Specification)"
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
                    .w(px(480.0))
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
                            .text_color(warning_color)
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child(format!("Are you sure you want to {}?", action_label)),
                    )
                    .child(
                        div()
                            .p_2()
                            .bg(rgba(0x1e1e1eff))
                            .border_1()
                            .border_color(border_color)
                            .rounded_sm()
                            .text_xs()
                            .child(item_summary),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xaaaaaaff))
                            .child(mode_checkbox_str),
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
                                    .bg(warning_color)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child(if is_permanent { "Delete (Enter)" } else { "Trash (Enter)" }),
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
