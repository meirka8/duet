//! Connection Manager UI (Task T-7.1.8) profile management, SSH config import, and test-connection widget.

use duet_vfs::remote::ConnectionProfile;
use gpui::*;

use crate::input::{InputState, InputWidget};

/// State for the Remote Connection Manager dialog.
#[derive(Debug, Clone)]
pub struct ConnectionManagerDialogState {
    pub profiles: Vec<ConnectionProfile>,
    pub selected_profile_idx: Option<usize>,
    pub name_input: InputState,
    pub host_input: InputState,
    pub port_input: InputState,
    pub user_input: InputState,
    pub path_input: InputState,
    pub secret_input: InputState,
    pub test_status_msg: Option<String>,
}

impl Default for ConnectionManagerDialogState {
    fn default() -> Self {
        Self {
            profiles: vec![
                ConnectionProfile {
                    id: "sftp-demo".to_string(),
                    name: "Demo SFTP Server".to_string(),
                    scheme: "sftp".to_string(),
                    host: "sftp.example.org".to_string(),
                    port: 22,
                    user: "admin".to_string(),
                    remote_path: "/var/www/html".to_string(),
                },
                ConnectionProfile {
                    id: "s3-backup".to_string(),
                    name: "AWS S3 Backup Bucket".to_string(),
                    scheme: "s3".to_string(),
                    host: "s3.us-west-2.amazonaws.com".to_string(),
                    port: 443,
                    user: "AKIAIOSFODNN7EXAMPLE".to_string(),
                    remote_path: "/backups".to_string(),
                },
            ],
            selected_profile_idx: Some(0),
            name_input: InputState {
                value: "Demo SFTP Server".to_string(),
                placeholder: "Connection Profile Name".to_string(),
                is_focused: true,
                ..Default::default()
            },
            host_input: InputState {
                value: "sftp.example.org".to_string(),
                placeholder: "Hostname or IP address".to_string(),
                is_focused: false,
                ..Default::default()
            },
            port_input: InputState {
                value: "22".to_string(),
                placeholder: "Port (e.g. 22)".to_string(),
                is_focused: false,
                ..Default::default()
            },
            user_input: InputState {
                value: "admin".to_string(),
                placeholder: "Username".to_string(),
                is_focused: false,
                ..Default::default()
            },
            path_input: InputState {
                value: "/var/www/html".to_string(),
                placeholder: "Remote Directory Path".to_string(),
                is_focused: false,
                ..Default::default()
            },
            secret_input: InputState {
                value: String::new(),
                placeholder: "Password or Secret Key".to_string(),
                is_focused: false,
                ..Default::default()
            },
            test_status_msg: None,
        }
    }
}

pub struct ConnectionManagerDialog;

impl ConnectionManagerDialog {
    pub fn render(
        state: &ConnectionManagerDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "Remote Connection Manager & Profiles (Task T-7.1.8)";

        let profile_list_items = state.profiles.iter().enumerate().map(|(idx, prof)| {
            let is_sel = state.selected_profile_idx == Some(idx);
            let item_bg = if is_sel { active_border } else { rgba(0x222222ff) };
            let text_clr = if is_sel { rgb(0xffffff) } else { fg };

            div()
                .px_3()
                .py_1_5()
                .bg(item_bg)
                .text_color(text_clr)
                .rounded_sm()
                .text_xs()
                .child(format!("[{}] {} ({}:{})", prof.scheme.to_uppercase(), prof.name, prof.host, prof.port))
        });

        let mut list_col = div().flex().flex_col().gap_1().w(px(220.0));
        for item in profile_list_items {
            list_col = list_col.child(item);
        }

        let test_status_div = match &state.test_status_msg {
            Some(msg) => div().text_xs().text_color(rgba(0x22c55e00)).child(msg.clone()),
            None => div().text_xs().text_color(rgba(0xaaaaaaff)).child("Status: Idle"),
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
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_sm()
                            .border_b_1()
                            .border_color(border_color)
                            .pb_2()
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_4()
                            .child(list_col)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_xs().child("Profile Name:"))
                                    .child(InputWidget::render(
                                        &state.name_input,
                                        bg,
                                        fg,
                                        border_color,
                                        active_border,
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_row()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .child(div().text_xs().child("Host:"))
                                                    .child(InputWidget::render(
                                                        &state.host_input,
                                                        bg,
                                                        fg,
                                                        border_color,
                                                        active_border,
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .w(px(80.0))
                                                    .child(div().text_xs().child("Port:"))
                                                    .child(InputWidget::render(
                                                        &state.port_input,
                                                        bg,
                                                        fg,
                                                        border_color,
                                                        active_border,
                                                    )),
                                            ),
                                    )
                                    .child(div().text_xs().child("User:"))
                                    .child(InputWidget::render(
                                        &state.user_input,
                                        bg,
                                        fg,
                                        border_color,
                                        active_border,
                                    ))
                                    .child(div().text_xs().child("Remote Path:"))
                                    .child(InputWidget::render(
                                        &state.path_input,
                                        bg,
                                        fg,
                                        border_color,
                                        active_border,
                                    )),
                            ),
                    )
                    .child(test_status_div)
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
                                    .child("Import from ~/.ssh/config"),
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
                                            .bg(rgba(0x3b82f6ff))
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Test Connection"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(active_border)
                                            .text_color(rgb(0xffffff))
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Connect (Enter)"),
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
