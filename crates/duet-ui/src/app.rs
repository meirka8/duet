//! GPUI application bootstrap and Tokio runtime executor integration.

use crate::workspace::WorkspaceView;
use gpui::*;

pub fn run_app() {
    // Assert UI thread blocking guard (ADR-0002 / T-3.1.6)
    duet_platform::set_ui_thread(true);

    let app = Application::new();

    app.run(move |cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::new(px(100.0), px(100.0)),
                size: Size {
                    width: px(1280.0),
                    height: px(800.0),
                },
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("Duet File Manager".into()),
                ..Default::default()
            }),
            ..Default::default()
        };

        cx.open_window(options, |_window, cx| {
            cx.new(|cx| WorkspaceView::new(cx))
        })
        .expect("Failed to open Duet workspace window");
    });
}
