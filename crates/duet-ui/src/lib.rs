use gpui::*;

pub struct MainView {
    pub text: SharedString,
}

impl Render for MainView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<'_, Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .bg(rgb(0x1e1e24))
            .size_full()
            .justify_center()
            .items_center()
            .text_color(rgb(0xe0e0e0))
            .text_xl()
            .child(self.text.clone())
    }
}

pub fn run_app() {
    // Set the UI thread flag so that any blocking VFS calls from this thread panic
    duet_platform::set_ui_thread(true);

    gpui::Application::new().run(|cx: &mut gpui::App| {
        cx.open_window(gpui::WindowOptions::default(), |_, cx| {
            cx.new(|_cx| MainView {
                text: "Duet File Manager - Phase 2 Bootstrap".into(),
            })
        })
        .unwrap();
    });
}
