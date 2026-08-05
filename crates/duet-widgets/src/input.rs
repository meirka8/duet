//! `Input` text field widget façade.

use gpui::*;

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub value: String,
    pub placeholder: String,
    pub is_focused: bool,
    pub cursor_pos: usize,
}

pub struct InputWidget;

impl InputWidget {
    pub fn render(
        state: &InputState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        focus_border_color: Rgba,
    ) -> Div {
        let current_border = if state.is_focused {
            focus_border_color
        } else {
            border_color
        };

        let content = if state.value.is_empty() {
            state.placeholder.clone()
        } else {
            state.value.clone()
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(26.0))
            .w_full()
            .bg(bg)
            .text_color(if state.value.is_empty() { rgba(0xaaaaaab3) } else { fg })
            .text_xs()
            .px_2()
            .rounded_sm()
            .border_1()
            .border_color(current_border)
            .child(content)
    }
}
