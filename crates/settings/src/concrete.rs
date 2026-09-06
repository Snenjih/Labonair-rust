//! Concrete `Settings` structs for the app's main feature areas (T19-002
//! Anweisung #5). Each wraps its `SettingsContent` area, resolved against
//! that area's own `defaults()` so every field is guaranteed `Some` (as long
//! as the area's `defaults()` populates every leaf, which
//! `labonair-settings-content`'s own tests enforce).
//!
//! These slices are the *single* settings surface every feature reads from —
//! the legacy `Preferences` / `PreferencesStore` / `GlobalPreferences` bridge
//! was retired once every call site moved onto `XSettings::get(cx)`.

use labonair_settings_content::{
    appearance::AppearanceContent,
    editor::EditorContent,
    file_manager::FileManagerContent,
    general::{GeneralContent, StartupTab, ThemePref},
    personalization::PersonalizationContent,
    terminal::{CursorStyle, TerminalContent},
    workspace::{PaletteSearchMode, WorkspaceContent},
    MergeFrom, SettingsContent,
};

use crate::settings_trait::Settings;
use crate::RegisterSetting;

/// `general` area — startup / lifecycle behaviour (app color-mode preference,
/// update checks, session restore, the startup tab).
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct GeneralSettings(GeneralContent);

impl Settings for GeneralSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = GeneralContent::defaults();
        merged.merge_from(&content.general);
        Self(merged)
    }
}

impl GeneralSettings {
    pub fn content(&self) -> &GeneralContent {
        &self.0
    }

    /// App color-mode preference (`System` follows the OS appearance).
    pub fn theme_pref(&self) -> ThemePref {
        self.0.theme.unwrap_or(ThemePref::System)
    }

    /// Check for new versions automatically on launch.
    pub fn check_for_updates(&self) -> bool {
        self.0.check_for_updates.unwrap_or(true)
    }

    /// Reopen the previous tabs / split layout on the next launch.
    pub fn session_restore(&self) -> bool {
        self.0.session_restore.unwrap_or(false)
    }

    /// What opens on launch when there is no session snapshot to restore.
    pub fn default_startup_tab(&self) -> StartupTab {
        self.0.default_startup_tab.unwrap_or_default()
    }
}

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

    /// Per-theme light/dark variant selection (`themeVariantOverrides`).
    pub fn theme_variant_overrides(&self) -> std::collections::BTreeMap<String, serde_json::Value> {
        self.0.theme_variant_overrides.clone().unwrap_or_default()
    }

    /// Where the tab strip is drawn (`"titlebar"` | `"sidebar"` | …).
    pub fn tabs_location(&self) -> &str {
        self.0.tabs_location.as_deref().unwrap_or("titlebar")
    }

    /// Zen mode keeps the app header visible.
    pub fn zen_mode_show_header(&self) -> bool {
        self.0.zen_mode_show_header.unwrap_or(true)
    }

    /// Zen mode keeps the status bar visible.
    pub fn zen_mode_show_statusbar(&self) -> bool {
        self.0.zen_mode_show_statusbar.unwrap_or(true)
    }

    /// UI-chrome font size, px (raw `u32` — [`Self::ui_font_size`] gives the `f32`).
    pub fn app_font_size(&self) -> u32 {
        self.0.app_font_size.unwrap_or(16)
    }

    // ── T20-007 `theme_settings` layer ───────────────────────────────────

    /// UI-chrome font family (empty = the theme's own family).
    pub fn ui_font_family(&self) -> &str {
        self.0.app_font_family.as_deref().unwrap_or("")
    }

    /// UI-chrome font size, px.
    pub fn ui_font_size(&self) -> f32 {
        self.0.app_font_size.unwrap_or(16) as f32
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
        self.0.buffer_font_size.unwrap_or(15) as f32
    }

    /// Editor/terminal text line-height multiple.
    pub fn buffer_line_height(&self) -> f32 {
        self.0.buffer_line_height.unwrap_or(1.618)
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

    /// Terminal text size, px.
    pub fn font_size(&self) -> u32 {
        self.0.terminal_font_size.unwrap_or(15)
    }

    /// Total scrollback the emulator keeps in memory, in rows.
    pub fn scrollback(&self) -> u32 {
        self.0.terminal_scrollback.unwrap_or(5_000)
    }

    /// Login shell override (empty = the OS default shell).
    pub fn shell(&self) -> &str {
        self.0.terminal_shell.as_deref().unwrap_or("")
    }

    /// Terminal text font family (empty = the theme's own mono family).
    pub fn font_family(&self) -> &str {
        self.0.terminal_font_family.as_deref().unwrap_or("")
    }

    /// Cursor shape.
    pub fn cursor_style(&self) -> CursorStyle {
        self.0.terminal_cursor_style.unwrap_or(CursorStyle::Bar)
    }

    pub fn cursor_blink(&self) -> bool {
        self.0.terminal_cursor_blink.unwrap_or(true)
    }

    pub fn show_pane_header(&self) -> bool {
        self.0.terminal_show_pane_header.unwrap_or(false)
    }

    pub fn show_pane_footer(&self) -> bool {
        self.0.terminal_show_pane_footer.unwrap_or(false)
    }

    /// Ring the terminal bell on the BEL control character.
    pub fn bell(&self) -> bool {
        self.0.terminal_bell.unwrap_or(false)
    }

    /// Rows of scrollback persisted per pane on quit (`None` = persist everything).
    pub fn session_scrollback_lines(&self) -> Option<usize> {
        let n = self.0.session_scrollback_lines.unwrap_or(1_000);
        (n > 0).then_some(n as usize)
    }

    /// Per-file ceiling for a persisted scrollback, in bytes.
    pub fn scrollback_max_bytes(&self) -> usize {
        (self.0.scrollback_max_size_mb.unwrap_or(10).max(1) as usize) * 1024 * 1024
    }

    /// Seconds a persisted scrollback file is kept before cleanup (`None` = forever).
    pub fn scrollback_retention_secs(&self) -> Option<u64> {
        let d = self.0.scrollback_retention_days.unwrap_or(0);
        (d > 0).then(|| d as u64 * 86_400)
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

    pub fn word_wrap(&self) -> bool {
        self.0.editor_word_wrap.unwrap_or(false)
    }

    pub fn line_numbers(&self) -> bool {
        self.0.editor_line_numbers.unwrap_or(true)
    }

    pub fn relative_line_numbers(&self) -> bool {
        self.0.editor_relative_line_numbers.unwrap_or(false)
    }

    pub fn indent_with_tabs(&self) -> bool {
        self.0.editor_indent_with_tabs.unwrap_or(false)
    }

    pub fn tab_size(&self) -> u32 {
        self.0.editor_tab_size.unwrap_or(2)
    }

    pub fn vim_mode(&self) -> bool {
        self.0.editor_vim_mode.unwrap_or(false)
    }

    pub fn format_on_save(&self) -> bool {
        self.0.editor_format_on_save.unwrap_or(false)
    }

    pub fn vim_hlsearch(&self) -> bool {
        self.0.vim_hlsearch.unwrap_or(true)
    }

    pub fn vim_incsearch(&self) -> bool {
        self.0.vim_incsearch.unwrap_or(true)
    }

    pub fn vim_smartcase(&self) -> bool {
        self.0.vim_smartcase.unwrap_or(true)
    }

    /// Syntax colour-scheme slug.
    pub fn editor_theme(&self) -> &str {
        self.0.editor_theme.as_deref().unwrap_or("atomone")
    }

    pub fn font_family(&self) -> &str {
        self.0
            .editor_font_family
            .as_deref()
            .unwrap_or("\"Lilex\", SFMono-Regular, Menlo, monospace")
    }

    pub fn font_size(&self) -> u32 {
        self.0.editor_font_size.unwrap_or(15)
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

    // ── Dock / sidebar ──────────────────────────────────────────────────
    /// `"left"` | `"right"` — which edge hosts the primary sidebar.
    pub fn sidebar_position(&self) -> &str {
        self.0.sidebar_position.as_deref().unwrap_or("left")
    }

    /// Persisted dock layout JSON (empty = not yet persisted).
    pub fn dock_layout(&self) -> &str {
        self.0.dock_layout.as_deref().unwrap_or("")
    }

    /// Legacy pre-dock-persistence fallback (`bootstrap::migrate_dock_layout`)
    /// reads these four once, only when `dock_layout` is still empty.
    pub fn sidebar_open(&self) -> bool {
        self.0.sidebar_open.unwrap_or(true)
    }

    pub fn sidebar_active_panel(&self) -> &str {
        self.0.sidebar_active_panel.as_deref().unwrap_or("explorer")
    }

    pub fn sidebar_width(&self) -> u32 {
        self.0.sidebar_width.unwrap_or(225)
    }

    pub fn sidebar_right_width(&self) -> u32 {
        self.0.sidebar_right_width.unwrap_or(225)
    }

    // ── Command palette ─────────────────────────────────────────────────
    pub fn command_palette_search_mode(&self) -> PaletteSearchMode {
        self.0
            .command_palette_search_mode
            .unwrap_or(PaletteSearchMode::Contains)
    }

    pub fn command_palette_history_size(&self) -> u32 {
        self.0.command_palette_history_size.unwrap_or(5)
    }

    pub fn command_palette_opacity(&self) -> u32 {
        self.0.command_palette_opacity.unwrap_or(95)
    }

    pub fn command_palette_position(&self) -> &str {
        self.0.command_palette_position.as_deref().unwrap_or("top")
    }

    pub fn command_palette_show_recent(&self) -> bool {
        self.0.command_palette_show_recent.unwrap_or(true)
    }

    pub fn command_palette_close_on_overlay_click(&self) -> bool {
        self.0
            .command_palette_close_on_overlay_click
            .unwrap_or(true)
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

/// `file_manager` area, read by the sidebar Explorer (Zed-parity Phase 3):
/// indent guides, sticky ancestors, active-file reveal, single-child folding,
/// Git decorations, and the historical hidden-files default.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct ExplorerSettings(FileManagerContent);

impl Settings for ExplorerSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = FileManagerContent::defaults();
        merged.merge_from(&content.file_manager);
        Self(merged)
    }
}

/// `file_manager` area as read by the Source-Control panel (Zed-parity
/// Phase 4): tree vs flat change-list presentation.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct ScmSettings(FileManagerContent);

impl Settings for ScmSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = FileManagerContent::defaults();
        merged.merge_from(&content.file_manager);
        Self(merged)
    }
}

impl ScmSettings {
    /// `true` → directory tree presentation; `false` → flat status buckets.
    pub fn file_tree(&self) -> bool {
        self.0.scm_file_tree.unwrap_or(false)
    }
}

impl ExplorerSettings {
    pub fn content(&self) -> &FileManagerContent {
        &self.0
    }

    pub fn show_hidden_by_default(&self) -> bool {
        self.0.explorer_show_hidden_by_default.unwrap_or(false)
    }

    pub fn indent_guides(&self) -> bool {
        self.0.explorer_indent_guides.unwrap_or(true)
    }

    pub fn sticky_ancestors(&self) -> bool {
        self.0.explorer_sticky_ancestors.unwrap_or(true)
    }

    pub fn auto_reveal_active_file(&self) -> bool {
        self.0.explorer_auto_reveal_active_file.unwrap_or(false)
    }

    pub fn fold_single_child_dirs(&self) -> bool {
        self.0.explorer_fold_single_child_dirs.unwrap_or(false)
    }

    pub fn git_decorations(&self) -> bool {
        self.0.explorer_git_decorations.unwrap_or(true)
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
        // Fresh defaults: unit scale, default density, Zed-style 16/15px fonts.
        let base = ThemeSettings::from_settings(&SettingsContent::default());
        assert_eq!(base.corner_radius_scale(), 1.0);
        assert_eq!(base.ui_density(), "default");
        assert_eq!(base.ui_font_size(), 16.0);
        assert_eq!(base.buffer_font_size(), 15.0);

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
