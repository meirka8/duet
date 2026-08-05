//! `Menu` context & dropdown menu widget façade.

use gpui::*;

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub is_separator: bool,
}

pub struct MenuWidget;

impl MenuWidget {
    pub fn render(
        items: &[MenuItem],
        bg: Rgba,
        fg: Rgba,
        hover_bg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let mut menu = div()
            .flex()
            .flex_col()
            .min_w(px(160.0))
            .bg(bg)
            .text_color(fg)
            .border_1()
            .border_color(border_color)
            .rounded_sm()
            .shadow_lg()
            .py_1();

        for item in items {
            if item.is_separator {
                menu = menu.child(
                    div()
                        .h(px(1.0))
                        .w_full()
                        .my_1()
                        .bg(border_color),
                );
            } else {
                let mut row = div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .cursor_pointer();

                if item.disabled {
                    row = row.opacity(0.5);
                } else {
                    row = row.hover(|s| s.bg(hover_bg));
                }

                row = row.child(item.label.clone());
                if let Some(sc) = &item.shortcut {
                    row = row.child(
                        div()
                            .ml_4()
                            .text_color(rgba(0xaaaaaab3))
                            .child(sc.clone()),
                    );
                }

                menu = menu.child(row);
            }
        }

        menu
    }
}
