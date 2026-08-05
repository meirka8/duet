//! Quick View Panel (Ctrl+Q, T-5.3.8) widget implementation.

use gpui::*;
use crate::viewer_widget::{InternalViewerWidget, ViewerState};

pub struct QuickViewWidget;

impl QuickViewWidget {
    pub fn render(
        state: &ViewerState,
        theme_bg: Rgba,
        theme_fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme_bg)
            .border_2()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .h(px(24.0))
                    .bg(rgba(0x222222ff))
                    .text_color(theme_fg)
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .items_center()
                    .px_2()
                    .border_b_1()
                    .border_color(border_color)
                    .child(format!("Quick View Preview: {}", state.file_path)),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .child(InternalViewerWidget::render(
                        state,
                        theme_bg,
                        theme_fg,
                        border_color,
                        active_border,
                    )),
            )
    }
}
