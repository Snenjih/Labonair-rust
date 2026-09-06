use gpui::{
    point, prelude::*, px, size, App, Application, Bounds, TitlebarOptions, WindowBounds,
    WindowOptions,
};

use gpui_component::Root;

mod dock_icon;
use labonair_backend::App as Backend;
#[cfg(debug_assertions)]
use labonair_backend::AppEvent;
use labonair_shell::{window_state, AppShell};
#[cfg(debug_assertions)]
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

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

/// Debug-only: subscribes to the backend event bus and logs every event,
/// decoding the typed [`AppEvent`] form where possible. The real UI routing
/// lives in `labonair_workspace::backend_event_bridge::BackendEventBridge`
/// (T17-008); this is purely a developer trace.
#[cfg(debug_assertions)]
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
    let tokio_handle = runtime.handle().clone();
    let guard = runtime.enter();

    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("labonair");
    let backend = Backend::new(&data_dir).expect("failed to initialize backend state");
    backend.spawn_workers();
    #[cfg(debug_assertions)]
    spawn_event_logger(&backend);

    // T19-009: one-time migration of the legacy `preferences`/`editor`/`mcp`
    // split into the flat `SettingsContent` area layout (+ `keymap.json` for
    // keybind overrides, + SQLite hosts into `hosts.entries`). Must run
    // before `labonair_settings::init(cx)` below, which reads the very same
    // `config.json` file and would otherwise silently see an
    // all-defaults tree for an old-format file.
    {
        use labonair_backend::modules::fs::paths::config_dir;
        use labonair_backend::modules::settings::{
            migrate_config_file_name,
            migrate_v2::{
                migrate_hosts_to_settings, migrate_settings_v1_to_v2, sparsify_v2_settings,
            },
        };

        let settings_dir = config_dir();
        if let Err(err) = migrate_config_file_name(&settings_dir) {
            tracing::warn!("config filename migration failed: {err}");
        }
        match migrate_settings_v1_to_v2(&settings_dir) {
            Ok(outcome) => tracing::info!("settings v1->v2 migration: {outcome:?}"),
            Err(err) => tracing::warn!("settings v1->v2 migration failed: {err}"),
        }
        // One-time cleanup of config.json files migrated before the migrator
        // learned to emit only overrides (every area spelled out at its
        // default). No-ops once `sparsified: true` is stamped.
        match sparsify_v2_settings(&settings_dir) {
            Ok(outcome) => tracing::info!("settings v2 sparsify: {outcome:?}"),
            Err(err) => tracing::warn!("settings v2 sparsify failed: {err}"),
        }
        runtime.block_on(async {
            match labonair_backend::modules::hosts::db::hosts_get_all(&backend.db).await {
                Ok(hosts) => match migrate_hosts_to_settings(&settings_dir, &hosts, &backend) {
                    Ok(outcome) => tracing::info!("hosts v1->v2 migration: {outcome:?}"),
                    Err(err) => tracing::warn!("hosts v1->v2 migration failed: {err}"),
                },
                Err(err) => tracing::warn!("failed to load hosts for v1->v2 migration: {err}"),
            }
        });
    }

    drop(guard);
    // Keep the runtime (and its background workers) alive for the process.
    std::mem::forget(runtime);

    tracing::info!("Labonair-rust starting");

    Application::new()
        .with_assets(labonair_shell::Assets)
        .run(move |cx: &mut App| {
            // Dock / cmd-tab icon for the un-bundled `cargo run` binary
            // (a packaged `.app` uses the embedded `icon.icns` instead).
            dock_icon::set_dock_icon();
            labonair_shell::init_fonts(cx);
            // T19-002: layered SettingsStore (default < user < …) — before
            // the first render, before gpui-component so nothing built below
            // can race a `XSettings::get(cx)` call against an unpopulated
            // store.
            labonair_settings::init(cx);
            gpui_component::init(cx);
            let bounds = window_state::load()
                .unwrap_or_else(|| Bounds::centered(None, size(px(1200.0), px(800.0)), cx));
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        // Reference uses macOS `titleBarStyle: "Overlay"` +
                        // `hiddenTitle: true`: one transparent overlay titlebar
                        // with the traffic lights floating over the app's own
                        // header. No OS-drawn title text.
                        title: None,
                        appears_transparent: true,
                        // Vertically centre the 14px-tall traffic lights inside
                        // the 40px custom header.
                        traffic_light_position: Some(point(px(19.0), px((40.0 - 14.0) / 2.0))),
                    }),
                    window_min_size: Some(size(px(720.0), px(480.0))),
                    ..Default::default()
                },
                move |window, cx| {
                    let theme = labonair_shell::init_theme(window.appearance(), cx);
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
                    let background = labonair_shell::init_background(cx);
                    let notifications = labonair_shell::init_notifications(cx);
                    let backend = backend.clone();
                    let tokio_handle = tokio_handle.clone();
                    // The window's first layer must be a `gpui_component::Root`
                    // so gpui-component primitives (Input, popovers, dialogs,
                    // notifications) can reach their deferred render layers.
                    let shell = cx.new(|cx| {
                        AppShell::new(
                            theme,
                            background,
                            notifications,
                            backend,
                            tokio_handle,
                            window,
                            cx,
                        )
                    });
                    let shell_view: gpui::AnyView = shell.into();
                    cx.new(|cx| Root::new(shell_view, window, cx))
                },
            )
            .expect("failed to open window");
            labonair_shell::init_menus(cx);
            cx.activate(true);
        });
}
