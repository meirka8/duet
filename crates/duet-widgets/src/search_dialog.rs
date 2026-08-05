//! Search View (Alt+F7, T-5.3.7) widget implementation.

use gpui::*;
use crate::input::{InputState, InputWidget};
use crate::status_bar::format_bytes;

/// Data entry for a single search result.
#[derive(Debug, Clone)]
pub struct SearchResultEntry {
    pub path: String,
    pub size_bytes: u64,
    pub mtime_str: String,
}

/// State for the Search View (Alt+F7) dialog.
#[derive(Debug, Clone, Default)]
pub struct SearchDialogState {
    pub mask_input: InputState,
    pub use_regex: bool,
    pub min_size_kb: Option<u64>,
    pub max_size_kb: Option<u64>,
    pub content_input: InputState,
    pub results: Vec<SearchResultEntry>,
    pub is_searching: bool,
}

pub struct SearchViewWidget;

impl SearchViewWidget {
    pub fn render(
        state: &SearchDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let regex_btn_str = if state.use_regex {
            "[x] Regex mode"
        } else {
            "[ ] Regex mode"
        };

        let status_label = if state.is_searching {
            format!("Searching... ({} results found)", state.results.len())
        } else {
            format!("Search complete ({} results found)", state.results.len())
        };

        let result_rows: Vec<Div> = state
            .results
            .iter()
            .map(|res| {
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .items_center()
                    .px_2()
                    .py_1()
                    .bg(rgba(0x1a1a1aff))
                    .border_b_1()
                    .border_color(border_color)
                    .text_xs()
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_color(fg)
                            .child(res.path.clone()),
                    )
                    .child(
                        div()
                            .w(px(80.0))
                            .text_color(rgba(0xaaaaaaff))
                            .child(format_bytes(res.size_bytes)),
                    )
                    .child(
                        div()
                            .w(px(130.0))
                            .text_color(rgba(0xaaaaaaff))
                            .child(res.mtime_str.clone()),
                    )
            })
            .collect();

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
                    .w(px(680.0))
                    .max_h(px(560.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_2xl()
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child("Find Files / Content Search (Alt+F7)"),
                    )
                    // Input filters grid
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .child(div().w(px(90.0)).text_xs().child("File mask:"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(InputWidget::render(
                                                &state.mask_input,
                                                bg,
                                                fg,
                                                border_color,
                                                active_border,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child(regex_btn_str),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
                                    .child(div().w(px(90.0)).text_xs().child("Find text:"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(InputWidget::render(
                                                &state.content_input,
                                                bg,
                                                fg,
                                                border_color,
                                                active_border,
                                            )),
                                    ),
                            ),
                    )
                    // Status bar row
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .text_xs()
                            .text_color(rgba(0x3b82f6ff))
                            .child(status_label)
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(active_border)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .child(if state.is_searching { "Stop Search" } else { "Start Search (Enter)" }),
                            ),
                    )
                    // Results list view
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h(px(180.0))
                            .bg(rgba(0x111111ff))
                            .border_1()
                            .border_color(border_color)
                            .overflow_hidden()
                            .children(result_rows),
                    )
                    // Footer Action bar ("Feed to Panel")
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .pt_2()
                            .border_t_1()
                            .border_color(border_color)
                            .child(
                                div()
                                    .px_4()
                                    .py_1()
                                    .bg(rgba(0x22c55eff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .child("Feed to Panel (Alt+L)"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Close (Esc)"),
                            ),
                    ),
            )
    }
}
