//! `Dialog` modal overlay widget façade.

use gpui::*;

pub struct DialogWidget;

impl DialogWidget {
    pub fn render(
        title: &str,
        content: impl IntoElement,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
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
                    .min_w(px(320.0))
                    .max_w(px(600.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .border_b_1()
                            .border_color(border_color)
                            .child(title.to_string()),
                    )
                    .child(div().p_4().child(content)),
            )
    }
}
