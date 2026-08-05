//! Button Bar Widget (Task T-9.1.10).

use gpui::*;

#[derive(Debug, Clone)]
pub struct ButtonItem {
    pub label: String,
    pub command_id: String,
    pub tooltip: String,
}

#[derive(Debug, Clone)]
pub struct ButtonBarData {
    pub buttons: Vec<ButtonItem>,
}

impl Default for ButtonBarData {
    fn default() -> Self {
        Self {
            buttons: vec![
                ButtonItem {
                    label: "Terminal".to_string(),
                    command_id: "tool.open_terminal".to_string(),
                    tooltip: "Open embedded terminal in active path".to_string(),
                },
                ButtonItem {
                    label: "Multi-Rename".to_string(),
                    command_id: "tool.multi_rename".to_string(),
                    tooltip: "Batch rename selected files with patterns".to_string(),
                },
                ButtonItem {
                    label: "Sync Dirs".to_string(),
                    command_id: "tool.sync_dirs".to_string(),
                    tooltip: "Synchronise left and right directories".to_string(),
                },
                ButtonItem {
                    label: "Connection Mgr".to_string(),
                    command_id: "tool.connection_manager".to_string(),
                    tooltip: "Manage SFTP/FTP/WebDAV/S3 profiles".to_string(),
                },
                ButtonItem {
                    label: "Plugins".to_string(),
                    command_id: "tool.plugin_manager".to_string(),
                    tooltip: "WASM Plugin manager and permissions".to_string(),
                },
            ],
        }
    }
}

pub struct ButtonBar;

impl ButtonBar {
    pub fn render(
        data: &ButtonBarData,
        bg: Rgba,
        fg: Rgba,
        _active_border: Rgba,
    ) -> Div {
        let mut row = div()
            .flex()
            .flex_row()
            .h(px(28.0))
            .bg(bg)
            .text_color(fg)
            .px_2()
            .gap_1()
            .items_center();

        for btn in &data.buttons {
            row = row.child(
                div()
                    .px_2()
                    .py_0p5()
                    .bg(rgba(0x222222ff))
                    .border_1()
                    .border_color(rgba(0x333333ff))
                    .rounded_sm()
                    .text_xs()
                    .child(btn.label.clone()),
            );
        }

        row
    }
}
