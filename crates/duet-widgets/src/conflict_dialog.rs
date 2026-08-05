//! Conflict Resolution Dialog (T-5.2.3) widget implementation.

use duet_ops::ConflictPolicy;
use gpui::*;
use crate::status_bar::format_bytes;

/// File metadata side for comparison.
#[derive(Debug, Clone, Default)]
pub struct FileMetaSide {
    pub path: String,
    pub size_bytes: u64,
    pub mtime_str: String,
    pub hash: Option<String>,
}

/// State for Conflict Resolution Dialog.
#[derive(Debug, Clone, Default)]
pub struct ConflictDialogState {
    pub src: FileMetaSide,
    pub dst: FileMetaSide,
    pub apply_to_all: bool,
    pub selected_policy: Option<ConflictPolicy>,
}

pub struct ConflictResolutionDialog;

impl ConflictResolutionDialog {
    pub fn render(
        state: &ConflictDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let render_side = |label: &str, meta: &FileMetaSide| {
            let hash_display = meta
                .hash
                .as_deref()
                .unwrap_or("Click 'Calculate Hashes' to compute");

            div()
                .flex()
                .flex_col()
                .flex_1()
                .p_3()
                .bg(rgba(0x1e1e1eff))
                .border_1()
                .border_color(border_color)
                .rounded_sm()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_xs()
                        .text_color(rgba(0x3b82f6ff))
                        .child(label.to_string()),
                )
                .child(div().text_xs().truncate().child(format!("Path: {}", meta.path)))
                .child(
                    div()
                        .text_xs()
                        .child(format!("Size: {} ({} bytes)", format_bytes(meta.size_bytes), meta.size_bytes)),
                )
                .child(div().text_xs().child(format!("Modified: {}", meta.mtime_str)))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgba(0xaaaaaaff))
                        .child(format!("BLAKE3: {}", hash_display)),
                )
        };

        let apply_all_str = if state.apply_to_all {
            "[x] Apply action to all remaining conflicts"
        } else {
            "[ ] Apply action to all remaining conflicts"
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
                    .w(px(640.0))
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
                            .child("File Conflict Resolution - File Already Exists"),
                    )
                    // Side by side metadata
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .child(render_side("Source File (Newer/Incoming)", &state.src))
                            .child(render_side("Destination File (Existing Target)", &state.dst)),
                    )
                    // Hash trigger button row
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x3b82f6ff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Calculate Hashes (BLAKE3)"),
                            )
                            .child(div().text_xs().child(apply_all_str)),
                    )
                    // 7 Policy buttons row
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .pt_2()
                            .border_t_1()
                            .border_color(border_color)
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(policy_btn("Overwrite", true))
                                    .child(policy_btn("Overwrite Older", false))
                                    .child(policy_btn("Overwrite Diff Size", false))
                                    .child(policy_btn("Skip", false)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(policy_btn("Auto-Rename", false))
                                    .child(policy_btn("Overwrite All", false))
                                    .child(policy_btn("Cancel Job", false)),
                            ),
                    ),
            )
    }
}

fn policy_btn(label: &str, active: bool) -> Div {
    let bg_color = if active {
        rgba(0x3b82f6ff)
    } else {
        rgba(0x333333ff)
    };
    div()
        .px_3()
        .py_1()
        .bg(bg_color)
        .text_color(rgb(0xffffff))
        .rounded_sm()
        .text_xs()
        .child(label.to_string())
}
