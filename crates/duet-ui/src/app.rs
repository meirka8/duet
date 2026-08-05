//! GPUI application bootstrap and Tokio runtime executor integration.

use crate::workspace::WorkspaceView;
use gpui::*;

pub fn run_app() {
    run_app_with_paths(None, None);
}

pub fn run_app_with_paths(left_path: Option<String>, right_path: Option<String>) {
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

        let lp = left_path.clone();
        let rp = right_path.clone();

        cx.open_window(options, move |window, cx| {
            let view = cx.new(|cx| WorkspaceView::with_paths(cx, lp, rp));
            let fh = view.read(cx).focus_handle.clone();
            if let Some(fh) = fh {
                fh.focus(window);
            }
            view
        })
        .expect("Failed to open Duet workspace window");
    });
}
