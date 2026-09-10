//! The `Settings` -> `ThemeStore` bridge: preference/font/metrics application,
//! registry-theme activation and persistence, and the palette hover-preview.
//! Moved out of `labonair-settings-ui` in R08-005 (mechanical move — no logic
//! change).

use gpui::{App, Entity};

use labonair_settings::content::general::ThemePref;
use labonair_settings::{
    EditorSettings, GeneralSettings, Settings as _, SettingsStore, TerminalSettings, ThemeSettings,
};
use labonair_theme::ThemePreference;
use labonair_theme::{ThemeMetrics, ThemeStore, UiDensity};

/// Build the [`labonair_theme::FontOverrides`] snapshot from the
/// typography-relevant settings slices. A blank family / zero size means "keep
/// the theme default".
pub(crate) fn font_overrides_from_settings(cx: &App) -> labonair_theme::FontOverrides {
    let appearance = ThemeSettings::try_get(cx);
    let editor = EditorSettings::try_get(cx);
    let terminal = TerminalSettings::try_get(cx);
    labonair_theme::FontOverrides {
        app_family: appearance
            .map(|s| s.ui_font_family().to_string())
            .unwrap_or_default(),
        app_size: appearance.map(|s| s.ui_font_size()).unwrap_or(16.0),
        editor_family: editor
            .map(|s| s.font_family().to_string())
            .unwrap_or_default(),
        editor_size: editor.map(|s| s.font_size() as f32).unwrap_or(15.0),
        terminal_family: terminal
            .map(|s| s.font_family().to_string())
            .unwrap_or_default(),
        terminal_size: terminal.map(|s| s.font_size() as f32).unwrap_or(15.0),
        terminal_line_height: terminal.map(|s| s.line_height()).unwrap_or(0.0),
        terminal_weight: terminal.map(|s| mono_weight(s.font_weight())),
    }
}

/// Map the `terminal` area's font-weight setting onto the theme typography enum.
fn mono_weight(
    w: labonair_settings::content::terminal::TerminalFontWeight,
) -> labonair_theme::MonoFontWeight {
    use labonair_settings::content::terminal::TerminalFontWeight as W;
    match w {
        W::Normal => labonair_theme::MonoFontWeight::Normal,
        W::Medium => labonair_theme::MonoFontWeight::Medium,
        W::Bold => labonair_theme::MonoFontWeight::Bold,
    }
}

/// Build the T20-007 [`ThemeMetrics`] from the layered `ThemeSettings`
/// (`appearance` area). Returns `None` if the `SettingsStore` global was never
/// installed (a headless harness) — callers then leave the store's default
/// metrics in place.
pub fn theme_metrics_from_settings(cx: &App) -> Option<ThemeMetrics> {
    let s = ThemeSettings::try_get(cx)?;
    Some(ThemeMetrics {
        ui_font_family: s.ui_font_family().to_string(),
        ui_font_size: s.ui_font_size(),
        ui_line_height: s.ui_line_height(),
        buffer_font_family: s.buffer_font_family().to_string(),
        buffer_font_size: s.buffer_font_size(),
        buffer_line_height: s.buffer_line_height(),
        density: UiDensity::from_str_or_default(s.ui_density()),
        corner_radius_scale: s.corner_radius_scale(),
        reduce_motion: s.reduce_motion(),
    })
}

/// Push the current [`ThemeMetrics`] (T20-007) into the [`ThemeStore`]. Called
/// on startup and from the `SettingsStore` observer in `labonair-shell`.
pub fn apply_theme_metrics(theme: &Entity<ThemeStore>, cx: &mut App) {
    if let Some(metrics) = theme_metrics_from_settings(cx) {
        theme.update(cx, |t, cx| t.set_metrics(metrics, cx));
    }
}

/// Push the font + editor-syntax-theme preferences into the [`ThemeStore`], and
/// (re)load + activate the persisted registry theme (T20-005). Used at startup
/// (`AppShell`) and on every settings change.
pub fn apply_prefs_to_theme(theme: &Entity<ThemeStore>, cx: &mut App) {
    // App color-mode preference (`general.theme`) — system / light / dark.
    if let Some(g) = GeneralSettings::try_get(cx) {
        let pref = match g.theme_pref() {
            ThemePref::Light => ThemePreference::Light,
            ThemePref::Dark => ThemePreference::Dark,
            ThemePref::System => ThemePreference::System,
        };
        theme.update(cx, |t, cx| t.set_preference(pref, cx));
    }

    let overrides = font_overrides_from_settings(cx);
    theme.update(cx, |t, cx| t.set_font_overrides(overrides, cx));
    apply_theme_metrics(theme, cx);

    let appearance = ThemeSettings::try_get(cx).cloned();
    let editor_theme_slug = EditorSettings::try_get(cx)
        .map(|s| s.editor_theme().to_string())
        .unwrap_or_else(|| "atomone".to_string());
    if let Some(id) = labonair_theme::EditorThemeId::from_slug(&editor_theme_slug) {
        theme.update(cx, |t, cx| t.set_editor_theme(id, cx));
    }

    // Activate the persisted built-in theme id (`""` / `"default"` follows
    // the light/dark preference). Theme catalogs are intentionally static for
    // the current product surface; extension/download support is deferred.
    let app_theme = appearance
        .as_ref()
        .map(|s| s.app_theme().to_string())
        .unwrap_or_else(|| "default".to_string());
    let id = if app_theme.is_empty() {
        "default"
    } else {
        app_theme.as_str()
    };
    if theme
        .update(cx, |t, cx| t.set_active_theme(id, cx))
        .is_err()
    {
        // Stale id (e.g. a deleted theme) — fall back to the built-in.
        theme.update(cx, |t, cx| {
            let _ = t.set_active_theme("default", cx);
        });
    }
    let variant_overrides = appearance
        .as_ref()
        .map(|s| s.theme_variant_overrides())
        .unwrap_or_default();
    apply_stored_theme_variant(&variant_overrides, theme, cx);

    // Icon theme (`""` / `"default"` → built-in glyph set). The initial
    // catalog is embedded and deterministic.
    let icon_theme = appearance
        .as_ref()
        .map(|s| s.icon_theme().to_string())
        .unwrap_or_else(|| "default".to_string());
    let icon_id = if icon_theme.is_empty() {
        "default"
    } else {
        icon_theme.as_str()
    };
    if theme
        .update(cx, |t, cx| t.set_active_icon_theme(icon_id, cx))
        .is_err()
    {
        theme.update(cx, |t, cx| {
            let _ = t.set_active_icon_theme("default", cx);
        });
    }
}

/// Re-apply the persisted `themeVariantOverrides[family][mode]` selection to the
/// active registry family for the currently-resolved appearance.
pub(crate) fn apply_stored_theme_variant(
    overrides: &std::collections::BTreeMap<String, serde_json::Value>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) {
    let (family, appearance) = {
        let t = theme.read(cx);
        let Some(family) = t.registry().family_of(&t.active_theme_id()) else {
            return;
        };
        let appearance = match t.mode() {
            labonair_theme::ThemeMode::Dark => "dark",
            labonair_theme::ThemeMode::Light => "light",
        };
        (family, appearance)
    };
    let key = overrides
        .get(&family)
        .and_then(|v| v.get(appearance))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    theme.update(cx, |t, cx| t.set_registry_variant(key, cx));
}

/// Live hover-preview of a theme by id (`Some`) or revert (`None`) — no
/// persistence. Used by the command palette's Themes sub-page.
pub fn preview_app_theme(id: Option<&str>, theme: &Entity<ThemeStore>, cx: &mut App) {
    match id {
        None => theme.update(cx, |t, cx| t.cancel_preview(cx)),
        Some(id) => theme.update(cx, |t, cx| t.preview_registry_theme(id, cx)),
    }
}

/// Activate a built-in app theme by id (`"default"` = follows system), persist
/// the selection, and re-apply its stored variant. Used by the palette.
pub fn activate_app_theme(id: &str, theme: &Entity<ThemeStore>, cx: &mut App) {
    let id_owned = id.to_string();
    if cx.has_global::<SettingsStore>() {
        let _ = cx
            .global_mut::<SettingsStore>()
            .update_user_settings(move |c| c.appearance.app_theme = Some(id_owned));
    }
    apply_prefs_to_theme(theme, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use labonair_settings::{SettingsContent, SettingsLayer, SettingsStore};

    fn install_store(cx: &mut App, content: SettingsContent) {
        let path = std::env::temp_dir().join(format!(
            "labonair-theme-ui-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        let mut store = SettingsStore::new(path);
        store.register_setting::<labonair_settings::TerminalSettings>();
        store.register_setting::<labonair_settings::EditorSettings>();
        store.register_setting::<ThemeSettings>();
        store.set_layer(SettingsLayer::User, content);
        cx.set_global(store);
    }

    #[gpui::test]
    fn font_overrides_snapshot_reads_settings(cx: &mut TestAppContext) {
        let mut content = SettingsContent::default();
        content.terminal.terminal_font_size = Some(18);
        content.editor.editor_font_family = Some("Iosevka".to_string());
        cx.update(|cx| {
            install_store(cx, content);
            let o = font_overrides_from_settings(cx);
            assert_eq!(o.terminal_size, 18.0);
            assert_eq!(o.editor_family, "Iosevka");
        });
    }
}
