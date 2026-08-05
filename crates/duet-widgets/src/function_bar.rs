//! `FunctionBar` bottom F1..F12 action key bar façade widget.

use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl FunctionKey {
    pub const ALL: [FunctionKey; 12] = [
        FunctionKey::F1,
        FunctionKey::F2,
        FunctionKey::F3,
        FunctionKey::F4,
        FunctionKey::F5,
        FunctionKey::F6,
        FunctionKey::F7,
        FunctionKey::F8,
        FunctionKey::F9,
        FunctionKey::F10,
        FunctionKey::F11,
        FunctionKey::F12,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            FunctionKey::F1 => "F1 Help",
            FunctionKey::F2 => "F2 Rename",
            FunctionKey::F3 => "F3 View",
            FunctionKey::F4 => "F4 Edit",
            FunctionKey::F5 => "F5 Copy",
            FunctionKey::F6 => "F6 Move",
            FunctionKey::F7 => "F7 MkDir",
            FunctionKey::F8 => "F8 Delete",
            FunctionKey::F9 => "F9 Menu",
            FunctionKey::F10 => "F10 Quit",
            FunctionKey::F11 => "F11 Plugin",
            FunctionKey::F12 => "F12 Config",
        }
    }

    pub fn key_number(&self) -> &'static str {
        match self {
            FunctionKey::F1 => "1",
            FunctionKey::F2 => "2",
            FunctionKey::F3 => "3",
            FunctionKey::F4 => "4",
            FunctionKey::F5 => "5",
            FunctionKey::F6 => "6",
            FunctionKey::F7 => "7",
            FunctionKey::F8 => "8",
            FunctionKey::F9 => "9",
            FunctionKey::F10 => "10",
            FunctionKey::F11 => "11",
            FunctionKey::F12 => "12",
        }
    }

    pub fn action_name(&self) -> &'static str {
        match self {
            FunctionKey::F1 => "Help",
            FunctionKey::F2 => "Rename",
            FunctionKey::F3 => "View",
            FunctionKey::F4 => "Edit",
            FunctionKey::F5 => "Copy",
            FunctionKey::F6 => "Move",
            FunctionKey::F7 => "MkDir",
            FunctionKey::F8 => "Delete",
            FunctionKey::F9 => "Menu",
            FunctionKey::F10 => "Quit",
            FunctionKey::F11 => "Plugin",
            FunctionKey::F12 => "Config",
        }
    }
}

/// Façade widget rendering the F1..F12 function key bar.
pub struct FunctionBar;

impl FunctionBar {
    pub fn render(
        bar_bg: Rgba,
        button_bg: Rgba,
        key_num_fg: Rgba,
        text_fg: Rgba,
    ) -> Div {
        let mut row = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(26.0))
            .bg(bar_bg)
            .items_center()
            .px_1()
            .gap_1();

        for key in FunctionKey::ALL {
            let key_item = div()
                .flex_1()
                .h(px(22.0))
                .bg(button_bg)
                .rounded_sm()
                .flex()
                .flex_row()
                .items_center()
                .justify_center()
                .px_1()
                .text_xs()
                .cursor_pointer()
                .child(
                    div()
                        .text_color(key_num_fg)
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(key.key_number()),
                )
                .child(
                    div()
                        .text_color(text_fg)
                        .ml_1()
                        .child(key.action_name()),
                );
            row = row.child(key_item);
        }

        row
    }
}
