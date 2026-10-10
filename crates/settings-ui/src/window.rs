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
    point, px, size, App, AppContext, Bounds, Global, SharedString, TitlebarOptions, WindowBounds,
    WindowHandle, WindowKind, WindowOptions,
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

/// The target a pending Settings deep-link wants to show.
#[derive(Clone, Default)]
pub(crate) enum SettingsTarget {
    #[default]
    Home,
    Page(&'static str),
    Field(SharedString),
}

impl Global for SettingsTarget {}

/// Publish the shared handles the settings window builds from. Call once from
/// `AppShell::new` after `labonair_settings::init` has run.
#[allow(clippy::too_many_arguments)]
pub fn set_settings_deps(services: SettingsServices, tokio: TokioHandle, cx: &mut App) {
    cx.set_global(SettingsDeps { services, tokio });
}

/// Scale the native Settings window from the same 900×750 logical baseline as
/// the pinned macOS reference, using the active UI font size as its rem scale.
fn settings_bounds(cx: &mut App) -> Bounds<gpui::Pixels> {
    let ui_scale = labonair_theme::theme_store(cx).read(cx).ui_font_size() / 16.0;
    Bounds::centered(None, size(px(900.0 * ui_scale), px(750.0 * ui_scale)), cx)
}

/// Open the settings window, or focus it if it is already open, optionally
/// deep-linking to `tab`. Replaces the old in-`AppShell` modal overlay
/// (T16-009). The window destroys on close and is cheaply rebuilt on the next
/// open (GPUI 0.2.2 has no per-window hide); shared state lives in the
/// `SettingsStore` / theme / background entities, so nothing is lost.
pub fn open_settings_window(slug: Option<&'static str>, cx: &mut App) {
    open_settings_target(slug.map(SettingsTarget::Page).unwrap_or_default(), cx);
}

/// Open a Labonair Settings deep-link. Returns `false` for unrelated or
/// unknown URLs so the application composition root can leave them to their
/// owning handlers.
pub fn open_settings_deep_link(url: &str, cx: &mut App) -> bool {
    if matches!(url, "labonair://settings" | "labonair://settings/") {
        open_settings_target(SettingsTarget::Home, cx);
        return true;
    }

    let Some(path) = setting_path_from_url(url) else {
        return false;
    };
    if !is_known_setting_path(path) {
        return false;
    }

    open_settings_target(
        SettingsTarget::Field(SharedString::from(path.to_owned())),
        cx,
    );
    true
}

fn setting_path_from_url(url: &str) -> Option<&str> {
    let path = url.strip_prefix("labonair://settings/")?;
    if path.is_empty() || path.contains('/') || path.contains('?') || path.contains('#') {
        return None;
    }
    Some(path)
}

fn is_known_setting_path(path: &str) -> bool {
    crate::schema::all_fields()
        .iter()
        .any(|field| field.json_path == path)
}

fn open_settings_target(target: SettingsTarget, cx: &mut App) {
    cx.set_global(target.clone());

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
                title: Some("Labonair — Settings".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(12.0), px(12.0))),
            }),
            window_min_size: Some(size(px(626.0), px(240.0))),
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
                match cx
                    .try_global::<SettingsTarget>()
                    .cloned()
                    .unwrap_or_default()
                {
                    SettingsTarget::Home => {}
                    SettingsTarget::Page(slug) => v.navigate_to_slug(slug),
                    SettingsTarget::Field(path) => v.navigate_to_json_path(&path, cx),
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

#[cfg(test)]
mod tests {
    use super::{is_known_setting_path, setting_path_from_url};

    #[test]
    fn settings_deep_links_accept_one_json_path() {
        assert_eq!(
            setting_path_from_url("labonair://settings/editor.tab_size"),
            Some("editor.tab_size")
        );
        assert_eq!(setting_path_from_url("labonair://settings"), None);
        assert_eq!(setting_path_from_url("labonair://settings/"), None);
        assert_eq!(setting_path_from_url("labonair://settings/a/b"), None);
        assert_eq!(
            setting_path_from_url("https://example.com/editor.tab_size"),
            None
        );
    }

    #[test]
    fn settings_deep_links_must_target_a_registered_field() {
        assert!(is_known_setting_path("general.theme"));
        assert!(!is_known_setting_path("general.unknown"));
    }
}
