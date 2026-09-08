//! Theme-owned palette action handling.
//!
//! `labonair-theme`'s `ThemeCommandProvider` contributes the command
//! descriptors and submenu snapshots; this handler applies the user's typed
//! selection (preview / activate / persist) through the [`crate::apply`]
//! policy. Moved out of `labonair-settings-ui` in R08-005.

use gpui::Entity;
use labonair_command_palette_core::{PaletteAction, SubmenuAction};
use labonair_command_palette_runtime::PaletteActionHandlerRegistry;
use labonair_theme::{EditorThemeId, ThemePreference, ThemeStore};

/// Register the runtime bridge for theme palette actions. `labonair-theme`
/// owns the static catalogs and `ThemeStore`; this owner applies and persists
/// the selection.
pub fn register_palette_action_handlers(
    registry: &mut PaletteActionHandlerRegistry,
    theme: &Entity<ThemeStore>,
) {
    let theme = theme.clone();
    registry
        .register("themes", move |action, _window, cx| match action {
            PaletteAction::PreviewAppTheme(id) => {
                crate::preview_app_theme(id.as_deref(), &theme, cx);
                true
            }
            PaletteAction::PreviewIconTheme(id) => {
                let _ = theme.update(cx, |theme, cx| theme.preview_icon_theme(id.as_deref(), cx));
                true
            }
            PaletteAction::Submenu(SubmenuAction::SetAppTheme(id)) => {
                crate::activate_app_theme(id, &theme, cx);
                true
            }
            PaletteAction::Submenu(SubmenuAction::SetIconTheme(id)) => {
                if cx.has_global::<labonair_settings::SettingsStore>() {
                    let persisted_id = id.clone();
                    let _ = cx
                        .global_mut::<labonair_settings::SettingsStore>()
                        .update_user_settings(move |c| {
                            c.appearance.icon_theme = Some(persisted_id);
                        });
                }
                let _ = theme.update(cx, |theme, cx| theme.set_active_icon_theme(id.clone(), cx));
                true
            }
            PaletteAction::Submenu(SubmenuAction::SetColorMode(mode)) => {
                let preference = match mode.as_str() {
                    "system" => ThemePreference::System,
                    "light" => ThemePreference::Light,
                    "dark" => ThemePreference::Dark,
                    _ => return false,
                };
                let value = match preference {
                    ThemePreference::System => {
                        labonair_settings::content::general::ThemePref::System
                    }
                    ThemePreference::Light => labonair_settings::content::general::ThemePref::Light,
                    ThemePreference::Dark => labonair_settings::content::general::ThemePref::Dark,
                };
                if cx.has_global::<labonair_settings::SettingsStore>() {
                    let _ = cx
                        .global_mut::<labonair_settings::SettingsStore>()
                        .update_user_settings(|c| c.general.theme = Some(value));
                }
                crate::apply_prefs_to_theme(&theme, cx);
                true
            }
            PaletteAction::Submenu(SubmenuAction::SetEditorTheme(id)) => {
                let Some(editor_theme) = EditorThemeId::from_slug(id) else {
                    return false;
                };
                if cx.has_global::<labonair_settings::SettingsStore>() {
                    let editor_slug = editor_theme.slug().to_string();
                    let _ = cx
                        .global_mut::<labonair_settings::SettingsStore>()
                        .update_user_settings(move |c| {
                            c.editor.editor_theme = Some(editor_slug);
                        });
                }
                crate::apply_prefs_to_theme(&theme, cx);
                true
            }
            _ => false,
        })
        .expect("theme palette action handler must have a unique owner");
}
