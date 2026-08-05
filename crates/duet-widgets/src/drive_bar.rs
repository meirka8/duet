//! Mount / Drive Bar Widget (T-6.1.14) showing mounted devices, gauges, and unmount/eject controls.

use gpui::*;

/// Device / filesystem mount types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    Nvme,
    Usb,
    Gvfs,
    Local,
    Optical,
}

impl DriveKind {
    pub fn label(&self) -> &'static str {
        match self {
            DriveKind::Nvme => "NVMe",
            DriveKind::Usb => "USB",
            DriveKind::Gvfs => "GVFS",
            DriveKind::Local => "Local",
            DriveKind::Optical => "Optical",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            DriveKind::Nvme => "⚡",
            DriveKind::Usb => "🔌",
            DriveKind::Gvfs => "🌐",
            DriveKind::Local => "🖴",
            DriveKind::Optical => "💿",
        }
    }
}

/// Mounted drive volume representation.
#[derive(Debug, Clone)]
pub struct DriveEntry {
    pub id: String,
    pub label: String,
    pub mount_point: String,
    pub kind: DriveKind,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub is_removable: bool,
    pub is_mounted: bool,
}

impl DriveEntry {
    pub fn used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            let used = self.total_bytes.saturating_sub(self.free_bytes);
            ((used as f64 / self.total_bytes as f64) * 100.0) as f32
        }
    }

    pub fn formatted_free_space(&self) -> String {
        let free_gb = self.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let total_gb = self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        format!("{:.1}/{:.1} GB", free_gb, total_gb)
    }
}

/// State data for the Drive Bar.
#[derive(Debug, Clone)]
pub struct DriveBarData {
    pub drives: Vec<DriveEntry>,
    pub active_mount_point: Option<String>,
}

impl Default for DriveBarData {
    fn default() -> Self {
        Self {
            drives: vec![
                DriveEntry {
                    id: "nvme0n1p2".to_string(),
                    label: "Root SSD".to_string(),
                    mount_point: "/".to_string(),
                    kind: DriveKind::Nvme,
                    total_bytes: 500_000_000_000,
                    free_bytes: 120_000_000_000,
                    is_removable: false,
                    is_mounted: true,
                },
                DriveEntry {
                    id: "sdb1".to_string(),
                    label: "USB Flash".to_string(),
                    mount_point: "/media/usb_flash".to_string(),
                    kind: DriveKind::Usb,
                    total_bytes: 64_000_000_000,
                    free_bytes: 32_000_000_000,
                    is_removable: true,
                    is_mounted: true,
                },
                DriveEntry {
                    id: "gvfs-smb".to_string(),
                    label: "NAS Share".to_string(),
                    mount_point: "/run/user/1000/gvfs/smb-share".to_string(),
                    kind: DriveKind::Gvfs,
                    total_bytes: 2_000_000_000_000,
                    free_bytes: 850_000_000_000,
                    is_removable: true,
                    is_mounted: true,
                },
            ],
            active_mount_point: Some("/".to_string()),
        }
    }
}

pub struct DriveBar;

impl DriveBar {
    pub fn render(
        data: &DriveBarData,
        bg: Rgba,
        fg: Rgba,
        subtle_fg: Rgba,
        active_border: Rgba,
    ) -> Div {
        let mut bar = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(32.0))
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .items_center()
            .px_2()
            .gap_2()
            .border_b_1()
            .border_color(rgba(0x333333ff));

        for drive in &data.drives {
            let is_active = data
                .active_mount_point
                .as_ref()
                .map(|p| p == &drive.mount_point)
                .unwrap_or(false);

            let card_border = if is_active {
                active_border
            } else {
                rgba(0x444444ff)
            };

            let used_pct = drive.used_percent();
            let gauge_color = if used_pct > 90.0 {
                rgba(0xef4444ff)
            } else if is_active {
                active_border
            } else {
                rgba(0x3b82f6ff)
            };

            let mut card = div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .bg(rgba(0x222222ff))
                .border_1()
                .border_color(card_border)
                .rounded_sm()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(format!("{} {}", drive.kind.symbol(), drive.label)),
                )
                .child(
                    div()
                        .text_color(subtle_fg)
                        .child(format!("({})", drive.formatted_free_space())),
                )
                .child(
                    div()
                        .w(px(40.0))
                        .h(px(6.0))
                        .bg(rgba(0x444444ff))
                        .rounded_sm()
                        .child(
                            div()
                                .h_full()
                                .w(px(40.0 * (used_pct / 100.0)))
                                .bg(gauge_color)
                                .rounded_sm(),
                        ),
                );

            if drive.is_removable || drive.kind == DriveKind::Usb || drive.kind == DriveKind::Gvfs {
                card = card.child(
                    div()
                        .px_1()
                        .bg(rgba(0x444444ff))
                        .hover(|style| style.bg(rgba(0xef4444ff)))
                        .rounded_sm()
                        .child("⏏"),
                );
            }

            bar = bar.child(card);
        }

        bar
    }
}
