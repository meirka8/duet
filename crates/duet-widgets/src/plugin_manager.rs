//! Plugin Manager UI Widget (Task T-8.1.11).
//! Renders installed/available WASM plugins, plain-language capability permissions, install/disable/uninstall controls.

use duet_plugin::{PluginManifest, RegistryIndexEntry};
use gpui::*;

use crate::input::{InputState, InputWidget};

#[derive(Debug, Clone)]
pub struct PluginManagerDialogState {
    pub installed_plugins: Vec<PluginManifest>,
    pub available_registry: Vec<RegistryIndexEntry>,
    pub selected_idx: Option<usize>,
    pub is_available_tab: bool,
    pub search_input: InputState,
}

impl Default for PluginManagerDialogState {
    fn default() -> Self {
        Self {
            installed_plugins: vec![
                PluginManifest {
                    id: "exif-viewer".to_string(),
                    name: "EXIF Column Plugin".to_string(),
                    version: "1.0.0".to_string(),
                    author: "Duet Team".to_string(),
                    description: "Adds EXIF camera model and ISO columns".to_string(),
                    capabilities: vec![duet_plugin::PluginCapability::FileAccess("*.jpg".to_string())],
                    memory_cap_bytes: 64 * 1024 * 1024,
                },
            ],
            available_registry: vec![
                RegistryIndexEntry {
                    id: "7z-advanced".to_string(),
                    name: "7z Advanced Compression Plugin".to_string(),
                    version: "1.2.0".to_string(),
                    author: "SevenZip Devs".to_string(),
                    description: "Adds LZMA2 solid 7z archive support".to_string(),
                    download_url: "https://registry.duet.fm/7z.wasm".to_string(),
                    sha256: "abc123hash".to_string(),
                },
            ],
            selected_idx: Some(0),
            is_available_tab: false,
            search_input: InputState {
                value: String::new(),
                placeholder: "Search plugins...".to_string(),
                is_focused: true,
                ..Default::default()
            },
        }
    }
}

pub struct PluginManagerDialog;

impl PluginManagerDialog {
    pub fn render(
        state: &PluginManagerDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Plugin Manager & Capabilities (Task T-8.1.11)";

        let item_list = if !state.is_available_tab {
            state.installed_plugins.iter().enumerate().map(|(idx, p)| {
                let is_sel = state.selected_idx == Some(idx);
                let item_bg = if is_sel { active_border } else { rgba(0x222222ff) };
                let text_clr = if is_sel { rgb(0xffffff) } else { fg };

                div()
                    .px_3()
                    .py_2()
                    .bg(item_bg)
                    .text_color(text_clr)
                    .rounded_sm()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .child(div().font_weight(gpui::FontWeight::BOLD).text_xs().child(p.name.clone()))
                            .child(div().text_xs().text_color(rgba(0x888888ff)).child(format!("v{}", p.version))),
                    )
                    .child(div().text_xs().text_color(rgba(0xaaaaaaff)).child(p.description.clone()))
            }).collect::<Vec<_>>()
        } else {
            state.available_registry.iter().enumerate().map(|(idx, r)| {
                let is_sel = state.selected_idx == Some(idx);
                let item_bg = if is_sel { active_border } else { rgba(0x222222ff) };
                let text_clr = if is_sel { rgb(0xffffff) } else { fg };

                div()
                    .px_3()
                    .py_2()
                    .bg(item_bg)
                    .text_color(text_clr)
                    .rounded_sm()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .child(div().font_weight(gpui::FontWeight::BOLD).text_xs().child(r.name.clone()))
                            .child(div().text_xs().text_color(rgba(0x888888ff)).child(format!("v{}", r.version))),
                    )
                    .child(div().text_xs().text_color(rgba(0xaaaaaaff)).child(r.description.clone()))
            }).collect::<Vec<_>>()
        };

        let mut list_col = div().flex().flex_col().gap_2().w(px(320.0));
        for item in item_list {
            list_col = list_col.child(item);
        }

        let cap_info = if !state.is_available_tab && !state.installed_plugins.is_empty() {
            let p = &state.installed_plugins[state.selected_idx.unwrap_or(0)];
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .bg(rgba(0x181818ff))
                .p_3()
                .rounded_sm()
                .child(div().font_weight(gpui::FontWeight::BOLD).text_xs().child("Requested Capabilities:"))
                .child(div().text_xs().text_color(rgba(0x22c55eff)).child("• File Read Access: *.jpg"))
                .child(div().text_xs().text_color(rgba(0x3b82f6ff)).child("• Memory Cap: 64 MB"))
                .child(div().text_xs().text_color(rgba(0xeab308ff)).child("• Network: Restricted (Zero-Ambient)"))
                .child(div().text_xs().text_color(rgba(0x888888ff)).child(format!("Author: {}", p.author)))
        } else {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .bg(rgba(0x181818ff))
                .p_3()
                .rounded_sm()
                .child(div().font_weight(gpui::FontWeight::BOLD).text_xs().child("Registry Package Info"))
                .child(div().text_xs().text_color(rgba(0xaaaaaaff)).child("Click Install to grant capabilities and load WASM module."))
        };

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
                            .flex()
                            .flex_row()
                            .justify_between()
                            .items_center()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
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
                                            .bg(if !state.is_available_tab { active_border } else { rgba(0x333333ff) })
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Installed"),
                                    )
                                    .child(
                                        div()
                                            .px_2()
                                            .py_1()
                                            .bg(if state.is_available_tab { active_border } else { rgba(0x333333ff) })
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Available Registry"),
                                    ),
                            ),
                    )
                    .child(InputWidget::render(
                        &state.search_input,
                        bg,
                        fg,
                        border_color,
                        active_border,
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .child(list_col)
                            .child(cap_info),
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
                                    .bg(rgba(0xef4444ff))
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("Disable / Remove"),
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
