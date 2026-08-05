//! Virtualized `List` widget façade.

use gpui::*;

pub struct ListWidget;

impl ListWidget {
    pub fn render_container(bg: Rgba, fg: Rgba) -> Div {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .text_color(fg)
    }
}
