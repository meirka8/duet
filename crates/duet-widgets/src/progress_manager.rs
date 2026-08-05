//! Progress Tray & Operation Manager (T-5.2.2) widget implementation.

use gpui::*;
use crate::status_bar::format_bytes;

/// Data for rendering the status bar progress tray widget.
#[derive(Debug, Clone, Default)]
pub struct StatusProgressTrayData {
    pub active_jobs_count: usize,
    pub queued_jobs_count: usize,
    pub overall_percentage: u8,
    pub current_speed_bytes_per_sec: u64,
    pub eta_seconds: Option<u64>,
}

pub struct StatusProgressTray;

impl StatusProgressTray {
    pub fn render(
        data: &StatusProgressTrayData,
        bg: Rgba,
        fg: Rgba,
        subtle_fg: Rgba,
    ) -> Div {
        if data.active_jobs_count == 0 && data.queued_jobs_count == 0 {
            return div();
        }

        let eta_str = match data.eta_seconds {
            Some(secs) => format!("{:02}:{:02}", secs / 60, secs % 60),
            None => "--:--".to_string(),
        };

        let summary = format!(
            "Ops: {} active, {} queued | {}% | {}/s | ETA: {}",
            data.active_jobs_count,
            data.queued_jobs_count,
            data.overall_percentage,
            format_bytes(data.current_speed_bytes_per_sec),
            eta_str
        );

        div()
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .py_1()
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .rounded_sm()
            .gap_2()
            .child(
                div()
                    .w(px(60.0))
                    .h(px(6.0))
                    .bg(rgba(0x444444ff))
                    .rounded_sm()
                    .child(
                        div()
                            .h_full()
                            .w(px(60.0 * (data.overall_percentage as f32 / 100.0)))
                            .bg(rgba(0x3b82f6ff)),
                    ),
            )
            .child(div().text_color(subtle_fg).child(summary))
    }
}

/// Display details for a single job item in the Operation Manager modal.
#[derive(Debug, Clone)]
pub struct JobItemDisplay {
    pub job_id: u64,
    pub title: String,
    pub op_type: String,
    pub src: String,
    pub dst: String,
    pub progress_percent: u8,
    pub copied_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: Option<u64>,
    pub status: String,
}

/// State for the expandable Operation Manager modal.
#[derive(Debug, Clone, Default)]
pub struct JobManagerModalState {
    pub jobs: Vec<JobItemDisplay>,
    pub is_expanded: bool,
}

pub struct OperationManagerModal;

impl OperationManagerModal {
    pub fn render(
        state: &JobManagerModalState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let job_items: Vec<Div> = state
            .jobs
            .iter()
            .map(|job| {
                let eta_str = match job.eta_seconds {
                    Some(s) => format!("{:02}:{:02}", s / 60, s % 60),
                    None => "ETA --:--".to_string(),
                };

                let progress_text = format!(
                    "{} / {} ({}%) - {}/s - {}",
                    format_bytes(job.copied_bytes),
                    format_bytes(job.total_bytes),
                    job.progress_percent,
                    format_bytes(job.speed_bytes_per_sec),
                    eta_str
                );

                div()
                    .flex()
                    .flex_col()
                    .p_2()
                    .bg(rgba(0x1e1e1eff))
                    .border_1()
                    .border_color(border_color)
                    .rounded_sm()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .text_xs()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Job #{}: {} [{}]", job.job_id, job.title, job.status))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_2()
                                            .py_0p5()
                                            .bg(rgba(0x3b82f6ff))
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .child("Pause/Resume"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py_0p5()
                                            .bg(rgba(0xef4444ff))
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .child("Cancel"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xaaaaaaff))
                            .child(format!("From: {}  ->  To: {}", job.src, job.dst)),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(8.0))
                            .bg(rgba(0x333333ff))
                            .rounded_sm()
                            .child(
                                div()
                                    .h_full()
                                    .w(px(560.0 * (job.progress_percent as f32 / 100.0)))
                                    .bg(rgba(0x22c55e99))
                                    .rounded_sm(),
                            ),
                    )
                    .child(div().text_xs().text_color(rgba(0xccccccff)).child(progress_text))
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
                    .w(px(600.0))
                    .max_h(px(500.0))
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
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_sm()
                                    .child(format!("Operation Manager ({} jobs)", state.jobs.len())),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()

                            .children(job_items),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .pt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Close (Esc)"),
                            ),
                    ),
            )
    }
}
