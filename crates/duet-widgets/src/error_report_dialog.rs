//! Error Report View (T-5.2.4) widget implementation.

use gpui::*;

/// Entry describing an operation error or skipped item.
#[derive(Debug, Clone)]
pub struct ErrorLogEntry {
    pub path: String,
    pub error_message: String,
    pub retryable: bool,
}

/// State for the Error Report View dialog.
#[derive(Debug, Clone, Default)]
pub struct ErrorReportState {
    pub job_id: u64,
    pub errors: Vec<ErrorLogEntry>,
}

pub struct ErrorReportDialog;

impl ErrorReportDialog {
    pub fn render(
        state: &ErrorReportState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let error_items: Vec<Div> = state
            .errors
            .iter()
            .map(|err| {
                let badge_bg = if err.retryable {
                    rgba(0xeab308ff) // Yellow
                } else {
                    rgba(0xef4444ff) // Red
                };

                let badge_label = if err.retryable {
                    "Retryable"
                } else {
                    "Fatal"
                };

                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .p_2()
                    .bg(rgba(0x1e1e1eff))
                    .border_1()
                    .border_color(border_color)
                    .rounded_sm()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(err.path.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0xef4444ff))
                                    .child(err.error_message.clone()),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(badge_bg)
                            .text_color(rgb(0x000000))
                            .rounded_sm()
                            .text_xs()
                            .child(badge_label),
                    )
            })
            .collect();

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
                    .w(px(580.0))
                    .max_h(px(460.0))
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
                            .child(format!(
                                "Operation Failure Log - Job #{} ({} error(s))",
                                state.job_id,
                                state.errors.len()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()

                            .children(error_items),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .pt_2()
                            .border_t_1()
                            .border_color(border_color)
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x3b82f6ff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Retry Failed Operations"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Dismiss Log (Esc)"),
                            ),
                    ),
            )
    }
}
