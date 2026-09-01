use gpui::{
    div, prelude::*, px, size, App, Application, Bounds, Entity, Window, WindowBounds,
    WindowOptions,
};
use labonair_backend::{App as Backend, AppEvent};
use labonair_ui::{BackgroundStore, LayerScope, TerminalView, ThemeStore};
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

/// Root view of the Labonair window. Currently hosts a single interactive
/// terminal; the real tabbed app shell arrives in Phase 03 (T04-003).
struct Root {
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    terminal: Entity<TerminalView>,
}

impl Root {
    fn new(
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();
        let terminal =
            cx.new(|cx| TerminalView::new(theme.clone(), background.clone(), window, cx));
        Self {
            theme,
            background,
            terminal,
        }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = self.theme.read(cx).background();
        let background_layer = self.background.read(cx).layer(LayerScope::App);
        div()
            .relative()
            .size_full()
            .bg(bg)
            .child(self.terminal.clone())
            .children(background_layer)
    }
}

/// `tracing` logging: default-off for noisy deps, `debug` for our crates, all
/// overridable via `RUST_LOG`.
fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,labonair=debug,labonair_backend=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .with_target(true)
        .init();
}

/// Subscribes to the backend event bus and logs every event, decoding the
/// typed [`AppEvent`] form where possible. This is where the GPUI layer will
/// later route events into views/entities (T04-003+).
fn spawn_event_logger(backend: &Backend) {
    let mut rx = backend.events.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(raw) => match AppEvent::from_raw(&raw) {
                    Some(event) => tracing::debug!(?event, "backend event"),
                    None => tracing::trace!(name = %raw.name, "backend event (untyped)"),
                },
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "event bus subscriber lagged");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn main() {
    init_logging();

    let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
    let guard = runtime.enter();

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("labonair");
    let backend = Backend::new(&data_dir).expect("failed to initialize backend state");
    backend.spawn_workers();
    spawn_event_logger(&backend);

    drop(guard);
    // Keep the runtime (and its background workers) alive for the process.
    std::mem::forget(runtime);

    tracing::info!("Labonair-rust starting");

    Application::new().run(|cx: &mut App| {
        labonair_ui::init_fonts(cx);
        let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let theme = labonair_ui::init_theme(window.appearance(), cx);
                window
                    .observe_window_appearance({
                        let theme = theme.clone();
                        move |window, cx| {
                            let appearance = window.appearance();
                            theme.update(cx, |store, cx| {
                                store.set_system_appearance(appearance, cx)
                            });
                        }
                    })
                    .detach();
                let background = labonair_ui::init_background(cx);
                cx.new(|cx| Root::new(theme, background, window, cx))
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
