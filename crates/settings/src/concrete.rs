//! Concrete `Settings` structs for the app's main feature areas (T19-002
//! Anweisung #5). Each wraps its `SettingsContent` area, resolved against
//! that area's own `defaults()` so every field is guaranteed `Some` (as long
//! as the area's `defaults()` populates every leaf, which
//! `labonair-settings-content`'s own tests enforce).
//!
//! Real consumers (this task, per the acceptance criteria): `ThemeSettings`
//! (`crates/workspace/src/workspace.rs`, `reduce_motion`) and
//! `TerminalSettings` (`crates/workspace/src/views/terminal.rs`, terminal
//! opacity / copy-on-select / right-click-pastes). The other four
//! (`EditorSettings`, `AiSettings`, `WorkspaceSettings`,
//! `PersonalizationSettings`) are registered and available via `XSettings::
//! get(cx)` now; wiring every remaining `GlobalPreferences` call site over to
//! them is deliberately out of scope here (see the task's `## Notizen`) and
//! follows incrementally (T20-007 and friends).

use labonair_settings_content::{
    ai::AiContent, appearance::AppearanceContent, editor::EditorContent,
    personalization::PersonalizationContent, terminal::TerminalContent,
    workspace::WorkspaceContent, MergeFrom, SettingsContent,
};

use crate::settings_trait::Settings;
use crate::RegisterSetting;

/// `appearance` area — active theme id, variant overrides, reduce-motion,
/// typography, background, tab-chrome layout. Named `ThemeSettings` (not
/// `AppearanceSettings`) because the `themes` custom top-level category
/// (`labonair_settings_content::areas::AREAS`) reads/writes this same
/// `target_module`.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct ThemeSettings(AppearanceContent);

impl Settings for ThemeSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = AppearanceContent::defaults();
        merged.merge_from(&content.appearance);
        // T20-007 read-time migration: a pre-T20-007 file that set the legacy
        // `appCornerRadius` (px) but never `cornerRadiusScale` keeps its
        // rounding — px ÷ 5 (the historical default base) becomes the scale.
        // The legacy key itself is left in place.
        if content.appearance.corner_radius_scale.is_none() {
            if let Some(px) = content.appearance.app_corner_radius {
                if px != 5 {
                    merged.corner_radius_scale = Some(px as f32 / 5.0);
                }
            }
        }
        Self(merged)
    }
}

impl ThemeSettings {
    pub fn content(&self) -> &AppearanceContent {
        &self.0
    }

    pub fn app_theme(&self) -> &str {
        self.0.app_theme.as_deref().unwrap_or("default")
    }

    pub fn icon_theme(&self) -> &str {
        self.0.icon_theme.as_deref().unwrap_or("default")
    }

    pub fn reduce_motion(&self) -> bool {
        self.0.reduce_motion.unwrap_or(false)
    }

    // ── T20-007 `theme_settings` layer ───────────────────────────────────

    /// UI-chrome font family (empty = the theme's own family).
    pub fn ui_font_family(&self) -> &str {
        self.0.app_font_family.as_deref().unwrap_or("")
    }

    /// UI-chrome font size, px.
    pub fn ui_font_size(&self) -> f32 {
        self.0.app_font_size.unwrap_or(13) as f32
    }

    /// UI-chrome line-height multiple.
    pub fn ui_line_height(&self) -> f32 {
        self.0.app_line_height.unwrap_or(1.5)
    }

    /// Editor/terminal text font family (empty = the theme's own mono family).
    pub fn buffer_font_family(&self) -> &str {
        self.0.buffer_font_family.as_deref().unwrap_or("")
    }

    /// Editor/terminal text font size, px.
    pub fn buffer_font_size(&self) -> f32 {
        self.0.buffer_font_size.unwrap_or(13) as f32
    }

    /// Editor/terminal text line-height multiple.
    pub fn buffer_line_height(&self) -> f32 {
        self.0.buffer_line_height.unwrap_or(1.5)
    }

    /// UI density token (`"compact"` | `"default"` | `"comfortable"`).
    pub fn ui_density(&self) -> &str {
        self.0.ui_density.as_deref().unwrap_or("default")
    }

    /// Corner-radius multiplier (`1.0` = unchanged). The legacy `appCornerRadius`
    /// fallback is resolved once in [`ThemeSettings::from_settings`].
    pub fn corner_radius_scale(&self) -> f32 {
        self.0.corner_radius_scale.unwrap_or(1.0)
    }
}

/// `terminal` area.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct TerminalSettings(TerminalContent);

impl Settings for TerminalSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = TerminalContent::defaults();
        merged.merge_from(&content.terminal);
        Self(merged)
    }
}

impl TerminalSettings {
    pub fn content(&self) -> &TerminalContent {
        &self.0
    }

    pub fn terminal_opacity(&self) -> u32 {
        self.0.terminal_opacity.unwrap_or(100)
    }

    pub fn copy_on_select(&self) -> bool {
        self.0.terminal_copy_on_select.unwrap_or(false)
    }

    pub fn right_click_pastes(&self) -> bool {
        self.0.terminal_right_click_pastes.unwrap_or(false)
    }
}

/// `editor` area.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct EditorSettings(EditorContent);

impl Settings for EditorSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = EditorContent::defaults();
        merged.merge_from(&content.editor);
        Self(merged)
    }
}

impl EditorSettings {
    pub fn content(&self) -> &EditorContent {
        &self.0
    }
}

/// `ai` area.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct AiSettings(AiContent);

impl Settings for AiSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = AiContent::defaults();
        merged.merge_from(&content.ai);
        Self(merged)
    }
}

impl AiSettings {
    pub fn content(&self) -> &AiContent {
        &self.0
    }
}

/// `workspace` area (startup tab, session restore, command palette, …).
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct WorkspaceSettings(WorkspaceContent);

impl Settings for WorkspaceSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = WorkspaceContent::defaults();
        merged.merge_from(&content.workspace);
        Self(merged)
    }
}

impl WorkspaceSettings {
    pub fn content(&self) -> &WorkspaceContent {
        &self.0
    }
}

/// `personalization` area (status-bar/panel placement, sidebar layout).
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct PersonalizationSettings(PersonalizationContent);

impl Settings for PersonalizationSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = PersonalizationContent::defaults();
        merged.merge_from(&content.personalization);
        Self(merged)
    }
}

impl PersonalizationSettings {
    pub fn content(&self) -> &PersonalizationContent {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_settings_reads_appearance_area_with_defaults_fallback() {
        let mut content = SettingsContent::default();
        content.appearance.reduce_motion = Some(true);
        let settings = ThemeSettings::from_settings(&content);
        assert!(settings.reduce_motion());
        // Untouched leaf still resolves to its documented default.
        assert_eq!(settings.app_theme(), "default");
    }

    #[test]
    fn theme_settings_metric_accessors_and_legacy_corner_radius_migration() {
        // Fresh defaults: unit scale, default density, 13px fonts.
        let base = ThemeSettings::from_settings(&SettingsContent::default());
        assert_eq!(base.corner_radius_scale(), 1.0);
        assert_eq!(base.ui_density(), "default");
        assert_eq!(base.ui_font_size(), 13.0);
        assert_eq!(base.buffer_font_size(), 13.0);

        // Pre-T20-007 file: only the legacy px key set → migrates to a scale.
        let mut legacy = SettingsContent::default();
        legacy.appearance.app_corner_radius = Some(10);
        let migrated = ThemeSettings::from_settings(&legacy);
        assert_eq!(migrated.corner_radius_scale(), 2.0);

        // An explicit new-style scale always wins over the legacy key.
        let mut both = SettingsContent::default();
        both.appearance.app_corner_radius = Some(10);
        both.appearance.corner_radius_scale = Some(0.5);
        assert_eq!(
            ThemeSettings::from_settings(&both).corner_radius_scale(),
            0.5
        );
    }

    #[test]
    fn terminal_settings_reads_terminal_area_with_defaults_fallback() {
        let mut content = SettingsContent::default();
        content.terminal.terminal_opacity = Some(42);
        let settings = TerminalSettings::from_settings(&content);
        assert_eq!(settings.terminal_opacity(), 42);
        assert!(!settings.copy_on_select());
    }
}
