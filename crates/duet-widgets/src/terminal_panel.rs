//! Embedded Terminal Panel Widget (Task T-9.1.11).

use gpui::*;

#[derive(Debug, Clone)]
pub struct TerminalPanelState {
    pub is_visible: bool,
    pub cwd: String,
    pub output_lines: Vec<String>,
}

impl Default for TerminalPanelState {
    fn default() -> Self {
        Self {
            is_visible: false,
            cwd: "/home/user/projects/duet".to_string(),
            output_lines: vec![
                "bash-5.2$ echo 'Duet Embedded Terminal'".to_string(),
                "Duet Embedded Terminal".to_string(),
                "bash-5.2$ _".to_string(),
            ],
        }
    }
}

pub struct TerminalPanelWidget;

impl TerminalPanelWidget {
    pub fn render(
        state: &TerminalPanelState,
        _bg: Rgba,
        fg: Rgba,
        active_border: Rgba,
    ) -> Div {
        if !state.is_visible {
            return div();
        }

        let mut term_box = div()
            .flex()
            .flex_col()
            .h(px(160.0))
            .bg(rgba(0x0d0d0dff))
            .border_t_1()
            .border_color(active_border)
            .p_2()
            .text_xs()
            .font_family("monospace");

        for line in &state.output_lines {
            term_box = term_box.child(div().text_color(fg).child(line.clone()));
        }

        term_box
    }
}
