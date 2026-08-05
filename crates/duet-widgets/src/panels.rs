//! `ResizableSplitter` dual-panel container widget façade.

use gpui::*;

/// Direction of the panel split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // Side-by-side (left/right)
    Vertical,   // Stacked (top/bottom)
}

/// State for the resizable splitter container.
#[derive(Debug, Clone)]
pub struct SplitterState {
    pub ratio: f32, // 0.0 to 1.0 (portion assigned to first panel)
    pub direction: SplitDirection,
    pub is_dragging: bool,
}

impl Default for SplitterState {
    fn default() -> Self {
        Self {
            ratio: 0.50,
            direction: SplitDirection::Horizontal,
            is_dragging: false,
        }
    }
}

impl SplitterState {
    pub fn new(ratio: f32, direction: SplitDirection) -> Self {
        Self {
            ratio: ratio.clamp(0.10, 0.90),
            direction,
            is_dragging: false,
        }
    }

    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(0.10, 0.90);
    }

    pub fn step_left(&mut self) {
        self.set_ratio(self.ratio - 0.05);
    }

    pub fn step_right(&mut self) {
        self.set_ratio(self.ratio + 0.05);
    }

    pub fn reset_equal(&mut self) {
        self.ratio = 0.50;
    }
}

/// Façade widget wrapper for rendering dual-pane layout with a splitter handle.
pub struct ResizableSplitter {
    state: SplitterState,
}

impl ResizableSplitter {
    pub fn new(state: SplitterState) -> Self {
        Self { state }
    }

    pub fn render<F1, F2>(
        &self,
        left_child: F1,
        right_child: F2,
        splitter_color: Rgba,
        active_border_color: Rgba,
    ) -> Div
    where
        F1: IntoElement,
        F2: IntoElement,
    {
        let ratio = self.state.ratio.clamp(0.10, 0.90);
        let rem_ratio = 1.0 - ratio;

        match self.state.direction {
            SplitDirection::Horizontal => div()
                .flex()
                .flex_row()
                .size_full()
                .child(
                    div()
                        .w(Length::Definite(DefiniteLength::Fraction(ratio)))
                        .h_full()
                        .child(left_child),
                )
                .child(
                    div()
                        .w(px(4.0))
                        .h_full()
                        .bg(if self.state.is_dragging { active_border_color } else { splitter_color })
                        .cursor_col_resize(),
                )
                .child(
                    div()
                        .w(Length::Definite(DefiniteLength::Fraction(rem_ratio)))
                        .h_full()
                        .child(right_child),
                ),
            SplitDirection::Vertical => div()
                .flex()
                .flex_col()
                .size_full()
                .child(
                    div()
                        .h(Length::Definite(DefiniteLength::Fraction(ratio)))
                        .w_full()
                        .child(left_child),
                )
                .child(
                    div()
                        .h(px(4.0))
                        .w_full()
                        .bg(if self.state.is_dragging { active_border_color } else { splitter_color })
                        .cursor_row_resize(),
                )
                .child(
                    div()
                        .h(Length::Definite(DefiniteLength::Fraction(rem_ratio)))
                        .w_full()
                        .child(right_child),
                ),
        }
    }
}
