use gpui::{
    div, prelude::*, px, rgb, size, App, Application, Bounds, Window, WindowBounds, WindowOptions,
};

/// Root view of the Labonair window. Replaced by the real app shell in Phase 03 (T04-003).
struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().bg(rgb(0x1a1b26)).child(
            div()
                .p_4()
                .text_color(rgb(0xc0caf5))
                .child("Labonair-rust — ready for development"),
        )
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| Root),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
