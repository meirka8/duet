//! `Select` dropdown widget façade.

use gpui::*;

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

pub struct SelectWidget;

impl SelectWidget {
    pub fn render(
        selected_label: &str,
        is_open: bool,
        options: &[SelectOption],
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let mut select_box = div()
            .relative()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(26.0))
            .px_2()
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .rounded_sm()
            .border_1()
            .border_color(border_color)
            .cursor_pointer()
            .child(selected_label.to_string())
            .child(if is_open { " ▲" } else { " ▼" });

        if is_open {
            let mut dropdown = div()
                .absolute()
                .top_full()
                .left_0()
                .w_full()
                .bg(bg)
                .border_1()
                .border_color(border_color)
                .rounded_b_sm()
                .shadow_md();

            for opt in options {
                dropdown = dropdown.child(
                    div()
                        .px_2()
                        .py_1()
                        .hover(|s| s.bg(rgba(0xffffff1a)))
                        .child(opt.label.clone()),
                );
            }

            select_box = select_box.child(dropdown);
        }

        select_box
    }
}
