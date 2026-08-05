//! Virtualized `Table` widget façade.

use gpui::*;

#[derive(Debug, Clone)]
pub struct TableColumnConfig {
    pub id: String,
    pub title: String,
    pub width_px: f32,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

pub struct TableWidget;

impl TableWidget {
    pub fn render_header(
        columns: &[TableColumnConfig],
        sorted_col: Option<&str>,
        sort_asc: bool,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
    ) -> Div {
        let mut header = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(24.0))
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .font_weight(gpui::FontWeight::BOLD)
            .items_center()
            .border_b_1()
            .border_color(border_color);

        for col in columns {
            let sort_indicator = if sorted_col == Some(col.id.as_str()) {
                if sort_asc { " ▲" } else { " ▼" }
            } else {
                ""
            };

            let title_str = format!("{}{}", col.title, sort_indicator);

            let cell = div()
                .flex_none()
                .w(px(col.width_px))
                .px_2()
                .truncate()
                .cursor_pointer()
                .child(title_str);

            header = header.child(cell);
        }

        header
    }
}
