//! Startup Recovery Overlay (T-5.2.5) widget implementation.

use gpui::*;

/// Information about a journaled uncommitted operation.
#[derive(Debug, Clone)]
pub struct JournalRecoveryEntry {
    pub job_id: u64,
    pub summary: String,
    pub step_progress: String,
}

/// State for the Startup Recovery Overlay dialog.
#[derive(Debug, Clone, Default)]
pub struct StartupRecoveryState {
    pub journal_entries: Vec<JournalRecoveryEntry>,
}

pub struct StartupRecoveryOverlay;

impl StartupRecoveryOverlay {
    pub fn render(
        state: &StartupRecoveryState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let entries: Vec<Div> = state
            .journal_entries
            .iter()
            .map(|entry| {
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
                                    .child(format!("Job #{}: {}", entry.job_id, entry.summary)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgba(0xaaaaaaff))
                                    .child(format!("Last Recorded State: {}", entry.step_progress)),
                            ),
                    )
            })
            .collect();

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000a0))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(560.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_2()
                    .border_color(rgba(0xeab308ff)) // Alert yellow border
                    .rounded_md()
                    .shadow_2xl()
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_sm()
                                    .text_color(rgba(0xeab308ff))
                                    .child("Startup Recovery - Interrupted Operations Journal Found"),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child("Duet detected uncommitted operation journal records from a previous crash or SIGKILL process termination."),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(entries),
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
                                    .bg(rgba(0x22c55eff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Resume Journaled Jobs"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0xef4444ff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Discard Journals"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Inspect Details"),
                            ),
                    ),
            )
    }
}
