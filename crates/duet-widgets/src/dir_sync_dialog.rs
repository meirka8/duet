//! Directory Comparison & Synchronisation Dialog (Tasks T-9.1.3, T-9.1.4).

use gpui::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDirection {
    LeftToRight,
    RightToLeft,
    Bidirectional,
}

#[derive(Debug, Clone)]
pub struct SyncItemDiff {
    pub relative_path: String,
    pub left_size: u64,
    pub right_size: u64,
    pub status: String, // "Equal", "Left Newer", "Right Newer", "Left Only", "Right Only"
    pub selected_action: String, // "-->", "<--", "=", "Delete"
}

#[derive(Debug, Clone)]
pub struct DirSyncDialogState {
    pub left_dir: String,
    pub right_dir: String,
    pub direction: SyncDirection,
    pub diffs: Vec<SyncItemDiff>,
    pub compare_by_content: bool,
    pub is_dry_run_plan: bool,
}

impl Default for DirSyncDialogState {
    fn default() -> Self {
        Self {
            left_dir: "/home/user/projects/src".to_string(),
            right_dir: "/backup/projects/src".to_string(),
            direction: SyncDirection::LeftToRight,
            diffs: vec![
                SyncItemDiff {
                    relative_path: "main.rs".to_string(),
                    left_size: 4096,
                    right_size: 2048,
                    status: "Left Newer".to_string(),
                    selected_action: "-->".to_string(),
                },
                SyncItemDiff {
                    relative_path: "utils.rs".to_string(),
                    left_size: 1024,
                    right_size: 0,
                    status: "Left Only".to_string(),
                    selected_action: "-->".to_string(),
                },
            ],
            compare_by_content: true,
            is_dry_run_plan: true,
        }
    }
}

pub struct DirSyncDialog;

impl DirSyncDialog {
    pub fn render(
        state: &DirSyncDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Synchronise & Compare Directories (Task T-9.1.3, T-9.1.4)";

        let diff_rows = state.diffs.iter().map(|d| {
            let action_color = match d.selected_action.as_str() {
                "-->" => rgba(0x3b82f6ff),
                "<--" => rgba(0xeab308ff),
                "=" => rgba(0x888888ff),
                _ => rgba(0xef4444ff),
            };

            div()
                .flex()
                .flex_row()
                .justify_between()
                .px_2()
                .py_1()
                .border_b_1()
                .border_color(rgba(0x222222ff))
                .text_xs()
                .child(div().w(px(200.0)).child(d.relative_path.clone()))
                .child(div().w(px(100.0)).text_color(rgba(0x888888ff)).child(format!("{} B", d.left_size)))
                .child(div().w(px(60.0)).text_color(action_color).font_weight(gpui::FontWeight::BOLD).child(d.selected_action.clone()))
                .child(div().w(px(100.0)).text_color(rgba(0x888888ff)).child(format!("{} B", d.right_size)))
                .child(div().w(px(100.0)).text_color(rgba(0x22c55eff)).child(d.status.clone()))
        });

        let mut diff_box = div()
            .flex()
            .flex_col()
            .h(px(240.0))
            .bg(rgba(0x151515ff))
            .border_1()
            .border_color(border_color)
            .rounded_sm()
            .p_2();

        for row in diff_rows {
            diff_box = diff_box.child(row);
        }

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
                    .w(px(720.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .p_4()
                    .gap_3()
                    .child(div().font_weight(gpui::FontWeight::BOLD).text_sm().child(title))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .text_xs()
                            .child(div().child(format!("Left Directory: {}", state.left_dir)))
                            .child(div().child(format!("Right Directory: {}", state.right_dir))),
                    )
                    .child(diff_box)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .pt_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .text_color(fg)
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Compare (BLAKE3)"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .text_color(fg)
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Dry-Run Plan"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(active_border)
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Synchronise (Enter)"),
                                    )
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
                    ),
            )
    }
}
