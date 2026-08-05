//! `TabBar` widget façade.

use gpui::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItem {
    pub id: usize,
    pub title: String,
    pub path: String,
    pub is_active: bool,
    pub is_locked: bool,
    pub lock_dir_change: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct TabBarColors {
    pub bg: Rgba,
    pub active_bg: Rgba,
    pub inactive_bg: Rgba,
    pub fg: Rgba,
    pub active_fg: Rgba,
    pub border_color: Rgba,
}

pub struct TabBar;

impl TabBar {
    pub fn render<FNew, FSelect, FClose>(
        tabs: &[TabItem],
        colors: TabBarColors,
        _on_new: FNew,
        _on_select: FSelect,
        _on_close: FClose,
    ) -> Div
    where
        FNew: Fn() + 'static,
        FSelect: Fn(usize) + 'static,
        FClose: Fn(usize) + 'static,
    {
        let mut bar = div()
            .flex()
            .flex_row()
            .w_full()
            .h(px(28.0))
            .bg(colors.bg)
            .items_center()
            .px_1()
            .gap_1()
            .border_b_1()
            .border_color(colors.border_color);

        for tab in tabs {
            let tab_bg = if tab.is_active { colors.active_bg } else { colors.inactive_bg };
            let tab_fg = if tab.is_active { colors.active_fg } else { colors.fg };

            let mut tab_el = div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(24.0))
                .px_2()
                .rounded_t_sm()
                .bg(tab_bg)
                .text_color(tab_fg)
                .text_xs()
                .cursor_pointer();

            if tab.is_locked {
                tab_el = tab_el.child(
                    div()
                        .mr_1()
                        .text_xs()
                        .child("🔒"),
                );
            }

            tab_el = tab_el.child(
                div()
                    .truncate()
                    .max_w(px(120.0))
                    .child(tab.title.clone()),
            );

            // Close button
            if tabs.len() > 1 {
                tab_el = tab_el.child(
                    div()
                        .ml_1()
                        .px_1()
                        .rounded_sm()
                        .hover(|s| s.bg(rgba(0xffffff33)))
                        .child("×"),
                );
            }

            bar = bar.child(tab_el);
        }

        // Add Tab Button "+"
        bar.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(24.0))
                .h(px(24.0))
                .rounded_sm()
                .text_xs()
                .text_color(colors.fg)
                .hover(|s| s.bg(colors.inactive_bg))
                .cursor_pointer()
                .child("+"),
        )
    }
}
