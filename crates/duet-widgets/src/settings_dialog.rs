//! Settings UI & wincmd.ini Import Tool (Tasks T-9.1.16, T-9.1.17).

use gpui::*;

#[derive(Debug, Clone)]
pub struct SettingsDialogState {
    pub active_tab: String, // "General", "Appearance", "Keymap", "Import TC"
    pub settings_toml_preview: String,
    pub tc_ini_path: String,
    pub tc_import_status: Option<String>,
}

impl Default for SettingsDialogState {
    fn default() -> Self {
        Self {
            active_tab: "General".to_string(),
            settings_toml_preview: r#"[general]
show_hidden_files = true
confirm_delete = true

[ui]
theme = "dark"
font_size = 13.0

[keymap]
preset = "total_commander"
"#
            .to_string(),
            tc_ini_path: "~/.totalcmd/wincmd.ini".to_string(),
            tc_import_status: None,
        }
    }
}

pub struct SettingsDialog;

impl SettingsDialog {
    pub fn render(
        state: &SettingsDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Settings UI & wincmd.ini Importer (Task T-9.1.16, T-9.1.17)";

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
                    .w(px(640.0))
                    .bg(bg)
                    .text_color(fg)
                    .border_1()
                    .border_color(border_color)
                    .rounded_md()
                    .shadow_xl()
                    .p_4()
                    .gap_3()
                    .child(div().font_weight(gpui::FontWeight::BOLD).text_sm().child(title))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(active_border)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("General Settings"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Import wincmd.ini (TC)"),
                            ),
                    )
                    .child(div().text_xs().child("settings.toml Live Preview:"))
                    .child(
                        div()
                            .h(px(180.0))
                            .bg(rgba(0x121212ff))
                            .border_1()
                            .border_color(border_color)
                            .rounded_sm()
                            .p_2()
                            .text_xs()
                            .font_family("monospace")
                            .child(state.settings_toml_preview.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .pt_2()
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(rgba(0x333333ff))
                                    .text_color(fg)
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Import from TC wincmd.ini"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(active_border)
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Save Settings (Enter)"),
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
                    ),
            )
    }
}
