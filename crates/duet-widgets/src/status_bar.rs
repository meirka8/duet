//! `StatusBar` widget façade.

use gpui::*;

/// Information displayed in the status bar.
#[derive(Debug, Clone, Default)]
pub struct StatusBarData {
    pub message: String,
    pub active_path: String,
    pub item_count: usize,
    pub selected_count: usize,
    pub total_selected_bytes: u64,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

/// Façade widget for rendering the status bar.
pub struct StatusBar;

impl StatusBar {
    pub fn render(
        data: &StatusBarData,
        bg: Rgba,
        fg: Rgba,
        subtle_fg: Rgba,
    ) -> Div {
        let size_str = if data.selected_count > 0 {
            format!(
                "Selected: {} / {} items ({})",
                data.selected_count,
                data.item_count,
                format_bytes(data.total_selected_bytes)
            )
        } else {
            format!("{} items", data.item_count)
        };

        let free_str = match (data.free_bytes, data.total_bytes) {
            (Some(free), Some(total)) => {
                format!("Free: {} / {}", format_bytes(free), format_bytes(total))
            }
            (Some(free), None) => format!("Free: {}", format_bytes(free)),
            _ => String::new(),
        };

        div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(22.0))
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .items_center()
            .justify_between()
            .px_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(
                        div()
                            .truncate()
                            .child(if data.message.is_empty() {
                                data.active_path.clone()
                            } else {
                                data.message.clone()
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .text_color(subtle_fg)
                    .child(div().child(size_str))
                    .child(div().child(free_str)),
            )
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
