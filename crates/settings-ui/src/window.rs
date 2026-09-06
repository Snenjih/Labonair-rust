//! The settings OS window: `open_settings_window`, `settings_bounds`, and the
//! GPUI globals it is driven by (`SettingsDeps`, `SettingsWindowRef`,
//! `SettingsTarget`). Split out of the old `crates/ui/src/settings.rs` monolith
//! in T16-007 (mechanical move — no logic change).
//!
//! **GPUI 0.2.2 limitations vs. the reference Tauri window** (unportable):
//! `WindowOptions` has no always-on-top / window-level field, no max-size, and
//! no parent-window handle — so the reference `always_on_top(true)`,
//! `max_inner_size(1400, 900)` and `parent(main)` lifecycle tie have no
//! equivalent. There is also no per-window hide, so `request_close` destroys the
//! window; all persistent state lives in the shared `SettingsStore` / theme
//! / background entities, so the next open rebuilds it losslessly.

use gpui::{
    point, px, size, App, AppContext, Bounds, Global, TitlebarOptions, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
};
use gpui_component::Root;
use tokio::runtime::Handle as TokioHandle;

use crate::services::SettingsServices;
use crate::view::SettingsView;

/// Shared handles the settings window needs, published by `AppShell` once at
/// startup (the window is opened lazily, possibly long after `AppShell::new`).
#[derive(Clone)]
pub(crate) struct SettingsDeps {
    services: SettingsServices,
    tokio: TokioHandle,
}

impl Global for SettingsDeps {}

/// The live settings window, if one is open. Checked on every open request so a
/// second invocation focuses the existing window instead of duplicating it.
#[derive(Default)]
pub(crate) struct SettingsWindowRef {
    pub(crate) handle: Option<WindowHandle<Root>>,
}

impl Global for SettingsWindowRef {}

/// The section a pending deep-link wants to show — an
/// `labonair_settings_content::areas::AreaMeta::slug`, optionally followed by
/// `/<sub-page slug>` (rule 7). `SettingsView` observes this global so an
/// already-open window jumps to the requested slug
/// (`SettingsView::navigate_to_slug`).
#[derive(Clone, Copy, Default)]
pub(crate) struct SettingsTarget(pub(crate) Option<&'static str>);

impl Global for SettingsTarget {}

/// Publish the shared handles the settings window builds from. Call once from
/// `AppShell::new` after `labonair_settings::init` has run.
#[allow(clippy::too_many_arguments)]
pub fn set_settings_deps(services: SettingsServices, tokio: TokioHandle, cx: &mut App) {
    cx.set_global(SettingsDeps { services, tokio });
}

/// Window bounds: 860 logical px wide, height = 80 % of the primary display
/// clamped to `[580, 900]` — a straight port of `settings_window_size()` in
/// `reference-src/src-tauri/src/lib.rs`.
fn settings_bounds(cx: &mut App) -> Bounds<gpui::Pixels> {
    let display_h = cx
        .primary_display()
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1000.0);
    let h = (display_h * 0.8).clamp(580.0, 900.0);
    Bounds::centered(None, size(px(860.0), px(h)), cx)
}

/// Open the settings window, or focus it if it is already open, optionally
/// deep-linking to `tab`. Replaces the old in-`AppShell` modal overlay
/// (T16-009). The window destroys on close and is cheaply rebuilt on the next
/// open (GPUI 0.2.2 has no per-window hide); shared state lives in the
/// `SettingsStore` / theme / background entities, so nothing is lost.
pub fn open_settings_window(slug: Option<&'static str>, cx: &mut App) {
    cx.set_global(SettingsTarget(slug));

    let existing = cx.try_global::<SettingsWindowRef>().and_then(|w| w.handle);
    if let Some(handle) = existing {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            cx.activate(true);
            return;
        }
        // Stale handle (window was closed) — fall through and open a fresh one.
        cx.set_global(SettingsWindowRef { handle: None });
    }

    let Some(deps) = cx.try_global::<SettingsDeps>().cloned() else {
        tracing::warn!("settings deps not published; cannot open settings window");
        return;
    };

    let bounds = settings_bounds(cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Settings".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(19.0), px((44.0 - 14.0) / 2.0))),
            }),
            window_min_size: Some(size(px(720.0), px(480.0))),
            kind: WindowKind::Normal,
            is_movable: true,
            ..Default::default()
        },
        move |window, cx| {
            let theme = labonair_theme::theme_store(cx);
            let view = cx.new(|cx| {
                let mut v = SettingsView::new(theme, deps.services.clone(), deps.tokio.clone(), cx);
                v.windowed = true;
                v.open = true;
                if let Some(SettingsTarget(Some(slug))) = cx.try_global::<SettingsTarget>().copied()
                {
                    v.navigate_to_slug(slug);
                }
                v.publish_settings_diagnostics(cx);
                v.load_system_fonts(cx);
                window.focus(&v.focus);
                v
            });
            let view: gpui::AnyView = view.into();
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(handle) => {
            cx.set_global(SettingsWindowRef {
                handle: Some(handle),
            });
            cx.activate(true);
        }
        Err(e) => tracing::error!("failed to open settings window: {e}"),
    }
}
