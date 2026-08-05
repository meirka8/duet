//! Create Dir / Rename / Links (F7, Shift+F6, T-5.2.7) widget implementations.

use gpui::*;
use crate::input::{InputState, InputWidget};

// --- 1. Create Directory Dialog (F7) ---
#[derive(Debug, Clone, Default)]
pub struct CreateDirDialogState {
    pub input: InputState,
}

pub struct CreateDirDialog;

impl CreateDirDialog {
    pub fn render(
        state: &CreateDirDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
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
                    .w(px(460.0))
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
                            .child("Create Directory (F7)"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child("Enter directory name or relative path (nested paths like 'a/b/c' supported):"),
                    )
                    .child(InputWidget::render(
                        &state.input,
                        bg,
                        fg,
                        border_color,
                        active_border,
                    ))
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
                                    .child("Create (Enter)"),
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

// --- 2. In-place Stem-only Rename Dialog (Shift+F6) ---
#[derive(Debug, Clone, Default)]
pub struct RenameDialogState {
    pub old_name: String,
    pub input: InputState,
    pub stem_end_idx: usize,
}

impl RenameDialogState {
    pub fn new(old_name: &str) -> Self {
        let stem_len = old_name.rsplit('.').next().map_or(old_name.len(), |ext| {
            if ext.len() < old_name.len() {
                old_name.len() - ext.len() - 1
            } else {
                old_name.len()
            }
        });

        let input = InputState {
            value: old_name.to_string(),
            placeholder: "New filename".to_string(),
            is_focused: true,
            cursor_pos: stem_len,
        };

        Self {
            old_name: old_name.to_string(),
            input,
            stem_end_idx: stem_len,
        }
    }
}

pub struct RenameDialog;

impl RenameDialog {
    pub fn render(
        state: &RenameDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
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
                    .w(px(460.0))
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
                            .child("Inline Rename (Shift+F6)"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child(format!("Original name: {}", state.old_name)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xaaaaaaff))
                            .child("(Stem is selected by default; extension preserved unless edited)"),
                    )
                    .child(InputWidget::render(
                        &state.input,
                        bg,
                        fg,
                        border_color,
                        active_border,
                    ))
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
                                    .child("Rename (Enter)"),
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

// --- 3. Create Symlink / Hardlink Dialog ---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkKind {
    #[default]
    Symbolic,
    Hard,
}

#[derive(Debug, Clone, Default)]
pub struct CreateLinkDialogState {
    pub target_path: String,
    pub link_name_input: InputState,
    pub link_kind: LinkKind,
}

pub struct CreateLinkDialog;

impl CreateLinkDialog {
    pub fn render(
        state: &CreateLinkDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = match state.link_kind {
            LinkKind::Symbolic => "Create Symbolic Link (Ctrl+Shift+F5)",
            LinkKind::Hard => "Create Hard Link",
        };

        let sym_btn_bg = if state.link_kind == LinkKind::Symbolic {
            active_border
        } else {
            rgba(0x333333ff)
        };

        let hard_btn_bg = if state.link_kind == LinkKind::Hard {
            active_border
        } else {
            rgba(0x333333ff)
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
                    .w(px(500.0))
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
                            .child(format!("Link target path: {}", state.target_path)),
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
                                    .bg(sym_btn_bg)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Symbolic Link (symlink)"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(hard_btn_bg)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Hard Link"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().child("Link name / path:"))
                            .child(InputWidget::render(
                                &state.link_name_input,
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
                                    .child("Create Link (Enter)"),
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
