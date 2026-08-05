//! Theme system with tokens, Light/Dark modes, follow-system detection, and TOML theme loader.

use duet_config::ThemeConfig;
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone)]
pub struct ThemeTokens {
    pub mode: ThemeMode,
    pub bg: Rgba,
    pub fg: Rgba,
    pub panel_bg: Rgba,
    pub active_border: Rgba,
    pub inactive_border: Rgba,
    pub selection_bg: Rgba,
    pub selection_fg: Rgba,
    pub cursor_bg: Rgba,
    pub cursor_border: Rgba,
    pub table_header_bg: Rgba,
    pub table_header_fg: Rgba,
    pub table_row_alt_bg: Rgba,
    pub status_bar_bg: Rgba,
    pub status_bar_fg: Rgba,
    pub status_bar_subtle_fg: Rgba,
    pub function_bar_bg: Rgba,
    pub function_bar_button_bg: Rgba,
    pub function_bar_key_fg: Rgba,
    pub function_bar_label_fg: Rgba,
    pub cmdline_bg: Rgba,
    pub cmdline_fg: Rgba,
    pub quick_search_bg: Rgba,
    pub quick_search_fg: Rgba,
    pub dir_fg: Rgba,
    pub executable_fg: Rgba,
    pub symlink_fg: Rgba,
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self::dark()
    }
}

impl ThemeTokens {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: rgb(0x18181b),                  // Zinc 900
            fg: rgb(0xf4f4f5),                  // Zinc 100
            panel_bg: rgb(0x1e1e24),            // Dark Panel
            active_border: rgb(0x3b82f6),       // Blue 500
            inactive_border: rgb(0x3f3f46),     // Zinc 700
            selection_bg: rgb(0x1e3a8a),        // Dark Blue Selection
            selection_fg: rgb(0x60a5fa),        // Light Blue Text
            cursor_bg: rgb(0x27272a),           // Zinc 800
            cursor_border: rgb(0x60a5fa),       // Light Blue Border
            table_header_bg: rgb(0x27272a),     // Zinc 800
            table_header_fg: rgb(0xd4d4d8),     // Zinc 300
            table_row_alt_bg: rgb(0x1a1a20),     // Slight Alt Row
            status_bar_bg: rgb(0x18181b),       // Zinc 900
            status_bar_fg: rgb(0xe4e4e7),       // Zinc 200
            status_bar_subtle_fg: rgb(0xa1a1aa),// Zinc 400
            function_bar_bg: rgb(0x18181b),     // Zinc 900
            function_bar_button_bg: rgb(0x27272a),// Zinc 800
            function_bar_key_fg: rgb(0xf59e0b), // Amber 500
            function_bar_label_fg: rgb(0xf4f4f5),// White Text
            cmdline_bg: rgb(0x09090b),          // Pitch Black
            cmdline_fg: rgb(0x10b981),          // Emerald 500
            quick_search_bg: rgb(0x7c3aed),     // Violet 600
            quick_search_fg: rgb(0xffffff),     // White Text
            dir_fg: rgb(0x60a5fa),              // Blue 400
            executable_fg: rgb(0x4ade80),       // Green 400
            symlink_fg: rgb(0x2dd4bf),          // Teal 400
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: rgb(0xf4f4f5),
            fg: rgb(0x18181b),
            panel_bg: rgb(0xffffff),
            active_border: rgb(0x2563eb),
            inactive_border: rgb(0xd4d4d8),
            selection_bg: rgb(0xdbeafe),
            selection_fg: rgb(0x1e40af),
            cursor_bg: rgb(0xe4e4e7),
            cursor_border: rgb(0x2563eb),
            table_header_bg: rgb(0xe4e4e7),
            table_header_fg: rgb(0x27272a),
            table_row_alt_bg: rgb(0xf8fafc),
            status_bar_bg: rgb(0xe4e4e7),
            status_bar_fg: rgb(0x18181b),
            status_bar_subtle_fg: rgb(0x71717a),
            function_bar_bg: rgb(0xe4e4e7),
            function_bar_button_bg: rgb(0xd4d4d8),
            function_bar_key_fg: rgb(0xd97706),
            function_bar_label_fg: rgb(0x18181b),
            cmdline_bg: rgb(0xffffff),
            cmdline_fg: rgb(0x059669),
            quick_search_bg: rgb(0x8b5cf6),
            quick_search_fg: rgb(0xffffff),
            dir_fg: rgb(0x2563eb),
            executable_fg: rgb(0x16a34a),
            symlink_fg: rgb(0x0d9488),
        }
    }

    pub fn from_config(config: &ThemeConfig) -> Self {
        let mut theme = if config.name.to_lowercase().contains("light") {
            Self::light()
        } else {
            Self::dark()
        };

        // Helper to parse hex colors from map
        let parse_color = |key: &str| -> Option<Rgba> {
            config.colors.get(key).and_then(|val| parse_hex(val))
        };

        if let Some(c) = parse_color("bg") { theme.bg = c; }
        if let Some(c) = parse_color("fg") { theme.fg = c; }
        if let Some(c) = parse_color("panel_bg") { theme.panel_bg = c; }
        if let Some(c) = parse_color("active_border") { theme.active_border = c; }
        if let Some(c) = parse_color("selection_bg") { theme.selection_bg = c; }
        if let Some(c) = parse_color("cursor_bg") { theme.cursor_bg = c; }

        theme
    }
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(rgb((r as u32) << 16 | (g as u32) << 8 | (b as u32)))
    } else {
        None
    }
}
