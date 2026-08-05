//! Internal Viewer (F3, T-5.3.6) widget implementation.

use gpui::*;
use crate::input::{InputState, InputWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewerMode {
    #[default]
    Text,
    Hex,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewerEncoding {
    #[default]
    Utf8,
    Ascii,
    Iso8859_1,
    Auto,
}

#[derive(Debug, Clone, Default)]
pub struct ViewerSearchState {
    pub input: InputState,
    pub match_count: usize,
    pub current_match_idx: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ViewerState {
    pub file_path: String,
    pub mode: ViewerMode,
    pub encoding: ViewerEncoding,
    pub content_lines: Vec<String>,
    pub raw_bytes: Vec<u8>,
    pub search: Option<ViewerSearchState>,
    pub scroll_line: usize,
}

impl ViewerState {
    pub fn new_text(path: &str, text: &str) -> Self {
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        Self {
            file_path: path.to_string(),
            mode: ViewerMode::Text,
            encoding: ViewerEncoding::Utf8,
            content_lines: lines,
            raw_bytes: text.as_bytes().to_vec(),
            search: None,
            scroll_line: 0,
        }
    }
}

pub struct InternalViewerWidget;

impl InternalViewerWidget {
    pub fn render(
        state: &ViewerState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        // 1. Header Toolbar
        let mode_str = match state.mode {
            ViewerMode::Text => "Text View (F3)",
            ViewerMode::Hex => "Hex View (F4)",
            ViewerMode::Image => "Image Preview",
        };

        let enc_str = match state.encoding {
            ViewerEncoding::Utf8 => "UTF-8",
            ViewerEncoding::Ascii => "ASCII",
            ViewerEncoding::Iso8859_1 => "ISO-8859-1",
            ViewerEncoding::Auto => "Auto-Detect",
        };

        let toolbar = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(28.0))
            .bg(rgba(0x1a1a1aff))
            .text_color(fg)
            .text_xs()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(format!("Viewer - {}", state.file_path)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(rgba(0x333333ff))
                            .rounded_sm()
                            .child(mode_str),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .bg(rgba(0x333333ff))
                            .rounded_sm()
                            .child(format!("Encoding: {enc_str}")),
                    ),
            );

        // 2. Content Body Rendering
        let content_view = match state.mode {
            ViewerMode::Text => render_text_view(state, fg, border_color),
            ViewerMode::Hex => render_hex_view(state, fg),
            ViewerMode::Image => render_image_view(state, fg),
        };

        // 3. Search Bar Overlay (if active)
        let search_bar = if let Some(search) = &state.search {
            div()
                .flex()
                .flex_row()
                .w_full()
                .h(px(30.0))
                .bg(rgba(0x222222ff))
                .border_t_1()
                .border_color(border_color)
                .px_2()
                .gap_2()
                .items_center()
                .child(div().text_xs().child("Find:"))
                .child(
                    div()
                        .w(px(200.0))
                        .child(InputWidget::render(
                            &search.input,
                            bg,
                            fg,
                            border_color,
                            active_border,
                        )),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgba(0xaaaaaaff))
                        .child(format!("Match {} of {}", search.current_match_idx, search.match_count)),
                )
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .bg(rgba(0x333333ff))
                        .text_color(fg)
                        .rounded_sm()
                        .text_xs()
                        .child("Prev (N)"),
                )
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .bg(rgba(0x333333ff))
                        .text_color(fg)
                        .rounded_sm()
                        .text_xs()
                        .child("Next (n)"),
                )
        } else {
            div()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .child(toolbar)
            .child(div().flex_1().w_full().child(content_view))
            .child(search_bar)
    }
}

fn render_text_view(state: &ViewerState, fg: Rgba, border_color: Rgba) -> Div {
    let visible_lines = state.content_lines.iter().take(200).enumerate();

    let rows: Vec<Div> = visible_lines
        .map(|(idx, line)| {
            let line_num = idx + 1;
            div()
                .flex()
                .flex_row()
                .w_full()
                .text_xs()
                .font_weight(gpui::FontWeight::NORMAL)
                .child(
                    div()
                        .w(px(50.0))
                        .px_2()
                        .text_color(rgba(0x666666ff))
                        .border_r_1()
                        .border_color(border_color)
                        .child(format!("{line_num:>5}")),
                )
                .child(
                    div()
                        .flex_1()
                        .px_3()
                        .text_color(fg)
                        .child(line.clone()),
                )
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(rgba(0x121212ff))
        .overflow_hidden()
        .children(rows)
}

fn render_hex_view(state: &ViewerState, fg: Rgba) -> Div {
    let chunks = state.raw_bytes.chunks(16).take(100).enumerate();

    let hex_rows: Vec<Div> = chunks
        .map(|(row_idx, chunk)| {
            let offset = row_idx * 16;

            let mut hex_str = String::new();
            let mut ascii_str = String::new();

            for &b in chunk {
                hex_str.push_str(&format!("{:02x} ", b));
                if (32..=126).contains(&b) {
                    ascii_str.push(b as char);
                } else {
                    ascii_str.push('.');
                }
            }

            div()
                .flex()
                .flex_row()
                .w_full()
                .text_xs()
                .font_weight(gpui::FontWeight::NORMAL)
                .gap_3()
                .px_2()
                .child(
                    div()
                        .w(px(70.0))
                        .text_color(rgba(0x3b82f6ff))
                        .child(format!("{:08x}", offset)),
                )
                .child(
                    div()
                        .w(px(320.0))
                        .text_color(fg)
                        .child(format!("{:<48}", hex_str)),
                )
                .child(
                    div()
                        .flex_1()
                        .text_color(rgba(0x22c55eff))
                        .child(ascii_str),
                )
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(rgba(0x0a0a0aff))
        .overflow_hidden()
        .children(hex_rows)
}

fn render_image_view(state: &ViewerState, fg: Rgba) -> Div {
    div()
        .flex()
        .flex_col()
        .size_full()
        .items_center()
        .justify_center()
        .bg(rgba(0x181818ff))
        .child(
            div()
                .p_4()
                .border_1()
                .border_color(rgba(0x444444ff))
                .rounded_md()
                .bg(rgba(0x222222ff))
                .text_xs()
                .text_color(fg)
                .child(format!("Image Preview Placeholder for: {}", state.file_path)),
        )
}
