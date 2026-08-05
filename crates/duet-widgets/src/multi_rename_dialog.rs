//! Multi-Rename Dialog Widget (Tasks T-9.1.1, T-9.1.2).

use duet_ops::{MultiRenameEngine, MultiRenameOptions, RenamePair};
use gpui::*;

use crate::input::{InputState, InputWidget};

#[derive(Debug, Clone)]
pub struct MultiRenameDialogState {
    pub options: MultiRenameOptions,
    pub previews: Vec<RenamePair>,
    pub pattern_input: InputState,
    pub find_input: InputState,
    pub replace_input: InputState,
}

impl Default for MultiRenameDialogState {
    fn default() -> Self {
        let opts = MultiRenameOptions {
            pattern: "photo_[C:3]".to_string(),
            ..Default::default()
        };
        let items = vec![
            "/tmp/DSC_0001.JPG".to_string(),
            "/tmp/DSC_0002.JPG".to_string(),
            "/tmp/DSC_0003.JPG".to_string(),
        ];
        let previews = MultiRenameEngine::compute_previews(&items, &opts);

        Self {
            options: opts,
            previews,
            pattern_input: InputState {
                value: "photo_[C:3]".to_string(),
                placeholder: "Rename Pattern (e.g. [N]_[C:3])".to_string(),
                is_focused: true,
                ..Default::default()
            },
            find_input: InputState {
                value: String::new(),
                placeholder: "Find Regex (optional)".to_string(),
                is_focused: false,
                ..Default::default()
            },
            replace_input: InputState {
                value: String::new(),
                placeholder: "Replace With (optional)".to_string(),
                is_focused: false,
                ..Default::default()
            },
        }
    }
}

pub struct MultiRenameDialog;

impl MultiRenameDialog {
    pub fn render(
        state: &MultiRenameDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Multi-Rename Tool & Live Preview (Task T-9.1.1, T-9.1.2)";

        let preview_rows = state.previews.iter().map(|p| {
            let warn_color = if p.collision_warning {
                rgba(0xef4444ff)
            } else {
                rgba(0x22c55eff)
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
                .child(div().text_color(rgba(0xaaaaaaff)).child(p.original_path.clone()))
                .child(div().text_color(warn_color).child(format!("→ {}", p.new_name)))
        });

        let mut preview_box = div()
            .flex()
            .flex_col()
            .h(px(200.0))
            .bg(rgba(0x151515ff))
            .border_1()
            .border_color(border_color)
            .rounded_sm()
            .p_2();

        for row in preview_rows {
            preview_box = preview_box.child(row);
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
                    .w(px(640.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .p_4()
                    .gap_3()
                    .child(div().font_weight(gpui::FontWeight::BOLD).text_sm().child(title))
                    .child(div().text_xs().child("Rename Pattern:"))
                    .child(InputWidget::render(
                        &state.pattern_input,
                        bg,
                        fg,
                        border_color,
                        active_border,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .child(div().text_xs().child("Find Regex:"))
                                    .child(InputWidget::render(
                                        &state.find_input,
                                        bg,
                                        fg,
                                        border_color,
                                        active_border,
                                    )),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(div().text_xs().child("Replace With:"))
                                    .child(InputWidget::render(
                                        &state.replace_input,
                                        bg,
                                        fg,
                                        border_color,
                                        active_border,
                                    )),
                            ),
                    )
                    .child(div().text_xs().child("Live Preview & Collision Audit:"))
                    .child(preview_box)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .pt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Undo Last Batch"),
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
                                            .child("Start Batch Rename (Enter)"),
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
                    ),
            )
    }
}
