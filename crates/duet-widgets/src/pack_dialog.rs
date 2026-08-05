//! Pack Dialog (Alt+F5, T-6.1.8) widget implementation.

use gpui::*;

use crate::input::{InputState, InputWidget};

/// Archive format options supported for packing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    Zip,
    TarGz,
    TarXz,
    TarZstd,
    SevenZip,
}

impl ArchiveFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => ".zip",
            ArchiveFormat::TarGz => ".tar.gz",
            ArchiveFormat::TarXz => ".tar.xz",
            ArchiveFormat::TarZstd => ".tar.zst",
            ArchiveFormat::SevenZip => ".7z",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "Zip (.zip)",
            ArchiveFormat::TarGz => "Tar.Gz (.tar.gz)",
            ArchiveFormat::TarXz => "Tar.Xz (.tar.xz)",
            ArchiveFormat::TarZstd => "Tar.Zstd (.tar.zst)",
            ArchiveFormat::SevenZip => "7-Zip (.7z)",
        }
    }

    pub fn all() -> &'static [ArchiveFormat] {
        &[
            ArchiveFormat::Zip,
            ArchiveFormat::TarGz,
            ArchiveFormat::TarXz,
            ArchiveFormat::TarZstd,
            ArchiveFormat::SevenZip,
        ]
    }
}

/// State for the Pack to Archive dialog.
#[derive(Debug, Clone)]
pub struct PackDialogState {
    pub dest_input: InputState,
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub password_input: InputState,
    pub move_to_archive: bool,
    pub source_files: Vec<String>,
}

impl Default for PackDialogState {
    fn default() -> Self {
        Self {
            dest_input: InputState {
                value: "archive.zip".to_string(),
                placeholder: "Archive name (e.g. archive.zip)".to_string(),
                is_focused: true,
                ..Default::default()
            },
            format: ArchiveFormat::Zip,
            compression_level: 6,
            password_input: InputState {
                value: String::new(),
                placeholder: "Password (optional)".to_string(),
                is_focused: false,
                ..Default::default()
            },
            move_to_archive: false,
            source_files: Vec::new(),
        }
    }
}

pub struct PackDialog;

impl PackDialog {
    pub fn render(
        state: &PackDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Pack Files to Archive (Alt+F5)";

        let src_summary = if state.source_files.is_empty() {
            "No files selected".to_string()
        } else if state.source_files.len() == 1 {
            format!("File: {}", state.source_files[0])
        } else {
            format!("Selected: {} items", state.source_files.len())
        };

        let move_toggle_str = if state.move_to_archive {
            "[x] Move files to archive after packing (delete source files)"
        } else {
            "[ ] Keep original source files after packing"
        };

        let level_label = match state.compression_level {
            0 => "0 (Store / Fast)",
            1..=3 => "Fast compression",
            4..=6 => "6 (Normal / Standard)",
            7..=8 => "High compression",
            9 => "9 (Ultra / Maximum)",
            _ => "Custom",
        };

        let format_buttons = ArchiveFormat::all().iter().map(|fmt| {
            let is_selected = *fmt == state.format;
            let button_bg = if is_selected {
                active_border
            } else {
                rgba(0x333333ff)
            };
            let text_clr = if is_selected { rgb(0xffffff) } else { fg };

            div()
                .px_2()
                .py_1()
                .bg(button_bg)
                .text_color(text_clr)
                .rounded_sm()
                .text_xs()
                .child(fmt.display_name())
        });

        let mut format_row = div().flex().flex_row().gap_2().flex_wrap();
        for btn in format_buttons {
            format_row = format_row.child(btn);
        }

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
                    .w(px(540.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgba(0xccccccff))
                            .child(src_summary),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().child("Destination archive path:"))
                            .child(InputWidget::render(
                                &state.dest_input,
                                bg,
                                fg,
                                border_color,
                                active_border,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().child("Archive format:"))
                            .child(format_row),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .text_xs()
                            .child(div().child(format!("Compression level: {level_label}")))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .rounded_sm()
                                            .child("-"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .rounded_sm()
                                            .child("+"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_xs().child("Password (optional):"))
                            .child(InputWidget::render(
                                &state.password_input,
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
                            .child(move_toggle_str),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .pt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(active_border)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Pack (Enter)"),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Cancel (Esc)"),
                            ),
                    ),
            )
    }
}
