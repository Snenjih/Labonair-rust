//! Theme-owned application policy (R08-005).
//!
//! `labonair-theme` owns the runtime `ThemeStore`, the deterministic built-in
//! catalogs, and the preview/activation *mechanics*. This sibling owns the
//! *policy* that connects those to the layered settings values: reading the
//! persisted preferences into the store, persisting a picker selection, and
//! the live hover-preview / cancel-revert used by the command palette.
//!
//! It was extracted from `labonair-settings-ui` so the Settings UI is a
//! values-only editor and does not own `ThemeStore` policy. `labonair-theme`
//! stays free of any `labonair-settings` dependency; the settings→theme
//! direction lives only here.

mod apply;
pub mod command_provider;
mod settings_views;

pub use apply::{
    activate_app_theme, apply_prefs_to_theme, apply_theme_metrics, apply_window_blur,
    preview_app_theme, theme_metrics_from_settings, window_background,
};
pub use settings_views::{ColorThemeSettings, IconThemeSettings};

use std::path::{Path, PathBuf};

use gpui::{App, AppContext as _, Entity};
use labonair_theme::registry::RejectedTheme;
use labonair_theme::ThemeStore;

/// Rescan the user themes folder and re-apply the persisted selection, so a
/// user family chosen in an earlier session becomes active once its file is
/// (re)loaded. Returns the files that were skipped (unreadable, malformed, or
/// missing a light or dark variant); the composition root reports each one
/// through the notification center.
pub fn reload_user_themes(
    theme: &Entity<ThemeStore>,
    dir: &Path,
    cx: &mut App,
) -> Vec<RejectedTheme> {
    let rejected = theme.update(cx, |store, cx| store.reload_user_themes(dir, cx));
    apply_prefs_to_theme(theme, cx);
    rejected
}

/// Contribute the Theme-owned Settings surfaces. Both are inline: the colour
/// theme grid and the icon-theme picker render inside the Themes page.
/// `open_themes` is the fallback entry point (open Settings on the Themes
/// page) and `themes_dir` the user themes folder the grid offers to open.
pub fn register_settings_surfaces(
    registry: &mut labonair_settings::SettingsSurfaceRegistry,
    open_themes: impl Fn(&mut App) + Clone + 'static,
    themes_dir: PathBuf,
) -> Result<(), String> {
    use labonair_settings::{
        SettingsSurface, SettingsSurfaceContribution, SettingsSurfaceId, SettingsSurfacePage,
    };

    registry.register(SettingsSurfaceContribution::inline(
        SettingsSurface::new(
            SettingsSurfaceId::new("app-themes"),
            "Color theme",
            "Choose a color theme. Each theme provides a light and a dark variant.",
            "Choose theme…",
            SettingsSurfacePage::section("themes", "Themes", "Color theme"),
        ),
        open_themes.clone(),
        move |_window, cx| {
            let theme = labonair_theme::theme_store(cx);
            let themes_dir = themes_dir.clone();
            cx.new(|cx| ColorThemeSettings::new(theme, themes_dir, cx))
                .into()
        },
    ))?;
    registry.register(SettingsSurfaceContribution::inline(
        SettingsSurface::new(
            SettingsSurfaceId::new("icon-themes"),
            "Icon theme",
            "Choose the icon set used throughout the app.",
            "Choose icons…",
            SettingsSurfacePage::section("themes", "Themes", "Icon theme"),
        ),
        open_themes,
        |_window, cx| {
            let theme = labonair_theme::theme_store(cx);
            cx.new(|cx| IconThemeSettings::new(theme, cx)).into()
        },
    ))
}

#[cfg(test)]
mod settings_surface_tests {
    use super::*;

    #[test]
    fn theme_settings_surfaces_are_registered_by_their_owner() {
        let mut registry = labonair_settings::SettingsSurfaceRegistry::default();
        register_settings_surfaces(&mut registry, |_| {}, PathBuf::new()).unwrap();
        let surfaces = registry.surfaces();
        assert_eq!(surfaces.len(), 2);
        assert_eq!(
            surfaces[0].id,
            labonair_settings::SettingsSurfaceId::new("app-themes")
        );
        assert_eq!(
            surfaces[1].id,
            labonair_settings::SettingsSurfaceId::new("icon-themes")
        );
        assert!(surfaces.iter().all(|surface| surface.page.slug == "themes"));
    }
}
