//! Permissions Dialog (T-5.2.8) widget implementation.

use gpui::*;
use crate::input::{InputState, InputWidget};

/// State for Permissions & Attributes Dialog.
#[derive(Debug, Clone)]
pub struct PermissionsDialogState {
    pub path: String,
    pub octal_mode: u32,
    pub user_r: bool,
    pub user_w: bool,
    pub user_x: bool,
    pub group_r: bool,
    pub group_w: bool,
    pub group_x: bool,
    pub other_r: bool,
    pub other_w: bool,
    pub other_x: bool,
    pub recursive: bool,
    pub atime_input: InputState,
    pub mtime_input: InputState,
}

impl Default for PermissionsDialogState {
    fn default() -> Self {
        let mut s = Self {
            path: String::new(),
            octal_mode: 0o755,
            user_r: true,
            user_w: true,
            user_x: true,
            group_r: true,
            group_w: false,
            group_x: true,
            other_r: true,
            other_w: false,
            other_x: true,
            recursive: false,
            atime_input: InputState {
                value: "2026-08-05 12:00:00".to_string(),
                placeholder: "YYYY-MM-DD HH:MM:SS".to_string(),
                ..Default::default()
            },
            mtime_input: InputState {
                value: "2026-08-05 12:00:00".to_string(),
                placeholder: "YYYY-MM-DD HH:MM:SS".to_string(),
                ..Default::default()
            },
        };
        s.sync_from_octal(0o755);
        s
    }
}

impl PermissionsDialogState {
    pub fn sync_from_octal(&mut self, mode: u32) {
        self.octal_mode = mode & 0o777;
        let u = (mode >> 6) & 7;
        let g = (mode >> 3) & 7;
        let o = mode & 7;

        self.user_r = (u & 4) != 0;
        self.user_w = (u & 2) != 0;
        self.user_x = (u & 1) != 0;

        self.group_r = (g & 4) != 0;
        self.group_w = (g & 2) != 0;
        self.group_x = (g & 1) != 0;

        self.other_r = (o & 4) != 0;
        self.other_w = (o & 2) != 0;
        self.other_x = (o & 1) != 0;
    }

    pub fn compute_octal(&self) -> u32 {
        let u = (if self.user_r { 4 } else { 0 })
            + (if self.user_w { 2 } else { 0 })
            + (if self.user_x { 1 } else { 0 });
        let g = (if self.group_r { 4 } else { 0 })
            + (if self.group_w { 2 } else { 0 })
            + (if self.group_x { 1 } else { 0 });
        let o = (if self.other_r { 4 } else { 0 })
            + (if self.other_w { 2 } else { 0 })
            + (if self.other_x { 1 } else { 0 });

        (u << 6) | (g << 3) | o
    }

    pub fn symbolic_string(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}{}{}",
            if self.user_r { 'r' } else { '-' },
            if self.user_w { 'w' } else { '-' },
            if self.user_x { 'x' } else { '-' },
            if self.group_r { 'r' } else { '-' },
            if self.group_w { 'w' } else { '-' },
            if self.group_x { 'x' } else { '-' },
            if self.other_r { 'r' } else { '-' },
            if self.other_w { 'w' } else { '-' },
            if self.other_x { 'x' } else { '-' },
        )
    }
}

pub struct PermissionsDialog;

impl PermissionsDialog {
    pub fn render(
        state: &PermissionsDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let octal_str = format!("{:04o}", state.compute_octal());
        let sym_str = state.symbolic_string();

        let render_perm_col = |label: &str, r: bool, w: bool, x: bool| {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .p_2()
                .bg(rgba(0x1e1e1eff))
                .border_1()
                .border_color(border_color)
                .rounded_sm()
                .gap_1()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_xs()
                        .child(label.to_string()),
                )
                .child(div().text_xs().child(format!("[{}] Read (r)", if r { "x" } else { " " })))
                .child(div().text_xs().child(format!("[{}] Write (w)", if w { "x" } else { " " })))
                .child(div().text_xs().child(format!("[{}] Execute (x)", if x { "x" } else { " " })))
        };

        let rec_str = if state.recursive {
            "[x] Apply recursively to subdirectories and contents"
        } else {
            "[ ] Apply recursively to subdirectories and contents"
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
                    .w(px(560.0))
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
                            .child("File Permissions & Attributes Editor"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .truncate()
                            .child(format!("Target: {}", state.path)),
                    )
                    // Octal & Symbolic Mode summary
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .p_2()
                            .bg(rgba(0x222222ff))
                            .rounded_sm()
                            .text_xs()
                            .child(div().child(format!("Octal mode: {}", octal_str)))
                            .child(div().child(format!("Symbolic mode: {}", sym_str))),
                    )
                    // Symbolic Grid (User, Group, Others)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(render_perm_col("User (Owner)", state.user_r, state.user_w, state.user_x))
                            .child(render_perm_col("Group", state.group_r, state.group_w, state.group_x))
                            .child(render_perm_col("Others", state.other_r, state.other_w, state.other_x)),
                    )
                    .child(div().text_xs().text_color(rgba(0x3b82f6ff)).child(rec_str))
                    // Timestamps section
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .border_t_1()
                            .border_color(border_color)
                            .pt_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .child(div().w(px(100.0)).text_xs().child("Access Time:"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(InputWidget::render(
                                                &state.atime_input,
                                                bg,
                                                fg,
                                                border_color,
                                                active_border,
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .child(div().w(px(100.0)).text_xs().child("Modify Time:"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(InputWidget::render(
                                                &state.mtime_input,
                                                bg,
                                                fg,
                                                border_color,
                                                active_border,
                                            )),
                                    ),
                            ),
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
                                    .child("Apply Changes"),
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
