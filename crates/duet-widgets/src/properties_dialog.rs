//! File Properties, Permissions & Checksums Dialog (Tasks T-9.1.6, T-9.1.8).

use gpui::*;

#[derive(Debug, Clone)]
pub struct PropertiesDialogState {
    pub file_path: String,
    pub size_bytes: u64,
    pub mode_octal: String,
    pub owner_uid_gid: String,
    pub mtime_str: String,
    pub blake3_hash: Option<String>,
    pub sha256_hash: Option<String>,
    pub is_calculating_hash: bool,
}

impl Default for PropertiesDialogState {
    fn default() -> Self {
        Self {
            file_path: "/home/user/document.pdf".to_string(),
            size_bytes: 1048576,
            mode_octal: "0644".to_string(),
            owner_uid_gid: "1000:1000 (user:user)".to_string(),
            mtime_str: "2026-08-05 14:32:00".to_string(),
            blake3_hash: Some("af1349b9f5f9a1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6".to_string()),
            sha256_hash: Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()),
            is_calculating_hash: false,
        }
    }
}

pub struct PropertiesDialog;

impl PropertiesDialog {
    pub fn render(
        state: &PropertiesDialogState,
        bg: Rgba,
        fg: Rgba,
        border_color: Rgba,
        active_border: Rgba,
    ) -> Div {
        let title = "File Properties & Checksums (Task T-9.1.6, T-9.1.8)";

        let blake3_str = state.blake3_hash.as_deref().unwrap_or("Calculating...");
        let sha256_str = state.sha256_hash.as_deref().unwrap_or("Calculating...");

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
                    .w(px(600.0))
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
                            .flex_col()
                            .gap_1_5()
                            .text_xs()
                            .child(div().child(format!("Path: {}", state.file_path)))
                            .child(div().child(format!("Size: {} bytes (1 MB)", state.size_bytes)))
                            .child(div().child(format!("Permissions: {}", state.mode_octal)))
                            .child(div().child(format!("Ownership: {}", state.owner_uid_gid)))
                            .child(div().child(format!("Modified: {}", state.mtime_str))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .bg(rgba(0x151515ff))
                            .p_2()
                            .rounded_sm()
                            .text_xs()
                            .child(div().font_weight(gpui::FontWeight::BOLD).child("On-Demand Cryptographic Hashes:"))
                            .child(div().text_color(rgba(0x22c55eff)).child(format!("BLAKE3: {}", blake3_str)))
                            .child(div().text_color(rgba(0x3b82f6ff)).child(format!("SHA-256: {}", sha256_str))),
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
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .text_color(fg)
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Split File"),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .bg(rgba(0x333333ff))
                                            .text_color(fg)
                                            .rounded_sm()
                                            .text_xs()
                                            .child("Merge Parts"),
                                    ),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .bg(active_border)
                                    .text_color(rgb(0xffffff))
                                    .rounded_sm()
                                    .text_xs()
                                    .child("OK (Esc)"),
                            ),
                    ),
            )
    }
}
