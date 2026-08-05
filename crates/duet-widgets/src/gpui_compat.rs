//! `gpui-compat` shim module for GPUI context, view, and element isolation.
//! Complies with ADR-0003: Strict GPUI Version Pinning and `gpui-compat` Shim Strategy.

use gpui::*;

/// Shim for view and window contexts to shield duet code from upstream GPUI context API churn.
pub struct ContextShim<'a, 'b> {
    pub window: &'a mut Window,
    pub app: &'b mut App,
}

impl<'a, 'b> ContextShim<'a, 'b> {
    pub fn new(window: &'a mut Window, app: &'b mut App) -> Self {
        Self { window, app }
    }

    /// Schedule a refresh / request frame.
    pub fn request_refresh(&mut self) {
        self.window.refresh();
    }
}

/// Helper function to build a styled container div with uniform defaults.
pub fn container() -> Div {
    div().flex().relative()
}

/// Helper function to create a text element.
pub fn label(text: impl Into<SharedString>) -> Div {
    div().child(text.into())
}

/// Compatibility wrapper for FocusHandle creation and management.
pub fn create_focus_handle(cx: &mut App) -> FocusHandle {
    cx.focus_handle()
}

/// Color representation shim converting RGB hex or RGBA values to GPUI Hsla/Rgba.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorToken {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorToken {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_gpui(&self) -> Rgba {
        rgb((self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32))
    }
}
