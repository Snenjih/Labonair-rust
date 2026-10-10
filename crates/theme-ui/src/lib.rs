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

pub use apply::{
    activate_app_theme, apply_prefs_to_theme, apply_theme_metrics, preview_app_theme,
    theme_metrics_from_settings,
};

/// Contribute the Theme-owned pickers to Settings navigation.
pub fn register_settings_surfaces(
    registry: &mut labonair_settings::SettingsSurfaceRegistry,
    open_app_themes: impl Fn(&mut gpui::App) + 'static,
    open_icon_themes: impl Fn(&mut gpui::App) + 'static,
) -> Result<(), String> {
    use labonair_settings::{
        SettingsSurface, SettingsSurfaceContribution, SettingsSurfaceId, SettingsSurfacePage,
    };

    registry.register(SettingsSurfaceContribution::new(
        SettingsSurface::new(
            SettingsSurfaceId::new("app-themes"),
            "App theme",
            "Choose a color theme and preview its variants.",
            "Choose theme…",
            SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
        ),
        open_app_themes,
    ))?;
    registry.register(SettingsSurfaceContribution::new(
        SettingsSurface::new(
            SettingsSurfaceId::new("icon-themes"),
            "Icon theme",
            "Choose the icon set used throughout the app.",
            "Choose icons…",
            SettingsSurfacePage::section("appearance", "Appearance", "Theme"),
        ),
        open_icon_themes,
    ))
}

#[cfg(test)]
mod settings_surface_tests {
    use super::*;

    #[test]
    fn theme_settings_surfaces_are_registered_by_their_owner() {
        let mut registry = labonair_settings::SettingsSurfaceRegistry::default();
        register_settings_surfaces(&mut registry, |_| {}, |_| {}).unwrap();
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
        assert!(surfaces
            .iter()
            .all(|surface| surface.page.slug == "appearance"));
    }
}
