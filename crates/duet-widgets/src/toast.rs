//! `Toast` notification overlay widget façade.

use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct ToastMessage {
    pub id: u64,
    pub kind: ToastKind,
    pub title: String,
    pub message: String,
}

pub struct ToastWidget;

impl ToastWidget {
    pub fn render(toasts: &[ToastMessage], bg: Rgba, fg: Rgba, border_color: Rgba) -> Div {
        let mut stack = div()
            .absolute()
            .bottom_4()
            .right_4()
            .flex()
            .flex_col()
            .gap_2();

        for toast in toasts {
            let accent_color = match toast.kind {
                ToastKind::Info => rgb(0x3b82f6),
                ToastKind::Success => rgb(0x22c55e),
                ToastKind::Warning => rgb(0xeab308),
                ToastKind::Error => rgb(0xef4444),
            };

            stack = stack.child(
                div()
                    .flex()
                    .flex_row()
                    .min_w(px(240.0))
                    .max_w(px(360.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .border_l_4()
                    .border_color(accent_color)
                    .rounded_sm()
                    .shadow_lg()
                    .p_3()
                    .text_xs()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child(toast.title.clone()),
                            )
                            .child(div().mt_1().child(toast.message.clone())),
                    ),
            );
        }

        stack
    }
}
