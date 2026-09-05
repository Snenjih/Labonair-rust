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
    fn terminal_settings_reads_terminal_area_with_defaults_fallback() {
        let mut content = SettingsContent::default();
        content.terminal.terminal_opacity = Some(42);
        let settings = TerminalSettings::from_settings(&content);
        assert_eq!(settings.terminal_opacity(), 42);
        assert!(!settings.copy_on_select());
    }
}
