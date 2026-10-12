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
    connections::ConnectionsContent,
    editor::{
        EditorContent, EditorCursorStyle, EditorDiagnosticMaxSeverity, EditorDocumentColorMode,
        EditorMinimapCurrentLineHighlight, EditorMinimapDisplayIn, EditorMinimapThumb,
        EditorMinimapThumbBorder, EditorMinimapVisibility, EditorModalMode,
        EditorScrollbarVisibility, SearchSeedBehavior,
    },
    file_manager::{default_sftp_columns, FileManagerContent, SftpColumn},
    general::{GeneralContent, StartupTab, ThemePref},
    terminal::{
        CursorStyle, FastScrollModifier, TerminalContent, TerminalFontWeight,
        DEFAULT_WORD_SEPARATOR,
    },
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
        self.0.app_font_size.unwrap_or(13)
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
        self.0.buffer_line_height.unwrap_or(1.55)
    }

    /// UI density token (`"compact"` | `"default"` | `"comfortable"`).
    pub fn ui_density(&self) -> &str {
        self.0.ui_density.as_deref().unwrap_or("default")
    }

    /// Corner-radius multiplier (`1.0` = unchanged). Legacy pixel values are
    /// converted during the versioned settings migration.
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
        self.0.terminal_font_size.unwrap_or(14)
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

    /// Cursor blink half-period, in milliseconds (clamped to a sane range).
    pub fn cursor_blink_interval_ms(&self) -> u64 {
        self.0
            .terminal_cursor_blink_interval
            .unwrap_or(1_000)
            .clamp(100, 5_000) as u64
    }

    /// Vertical line spacing as a multiple of the font size.
    pub fn line_height(&self) -> f32 {
        self.0.terminal_line_height.unwrap_or(1.05).clamp(0.8, 2.5)
    }

    /// Terminal text weight.
    pub fn font_weight(&self) -> TerminalFontWeight {
        self.0
            .terminal_font_weight
            .unwrap_or(TerminalFontWeight::Normal)
    }

    /// Characters that terminate a double-click word selection.
    pub fn word_separator(&self) -> &str {
        self.0
            .terminal_word_separator
            .as_deref()
            .unwrap_or(DEFAULT_WORD_SEPARATOR)
    }

    /// Multiplier applied to every mouse-wheel scroll step (>= 1).
    pub fn scroll_sensitivity(&self) -> u32 {
        self.0.terminal_scroll_sensitivity.unwrap_or(1).max(1)
    }

    /// Modifier that multiplies the scroll step while held.
    pub fn fast_scroll_modifier(&self) -> FastScrollModifier {
        self.0
            .terminal_fast_scroll_modifier
            .unwrap_or(FastScrollModifier::Alt)
    }

    /// Confirm before closing a terminal tab whose shell is still running.
    pub fn confirm_close_terminal_tab(&self) -> bool {
        self.0.confirm_close_terminal_tab.unwrap_or(false)
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

    /// Extra environment variables applied to every new shell.
    pub fn environment_variables(&self) -> Vec<(String, String)> {
        self.0
            .terminal_environment_variables
            .as_deref()
            .map(labonair_settings_content::terminal::parse_environment_variables)
            .unwrap_or_default()
    }

    /// Extra shell arguments appended after [`Self::shell`].
    pub fn shell_args(&self) -> Vec<String> {
        self.0
            .terminal_shell_args
            .as_deref()
            .map(labonair_settings_content::terminal::parse_shell_args)
            .unwrap_or_default()
    }
}

fn single_character_setting(value: Option<&str>, fallback: char) -> char {
    let Some(value) = value else {
        return fallback;
    };
    let mut characters = value.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) => character,
        _ => fallback,
    }
}

/// `editor` area.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct EditorSettings(EditorContent);

impl Settings for EditorSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let legacy_vim_mode = content.editor.editor_modal_mode.is_none()
            && content.editor.editor_vim_mode.unwrap_or(false);
        let mut merged = EditorContent::defaults();
        merged.merge_from(&content.editor);
        if legacy_vim_mode {
            merged.editor_modal_mode = Some(EditorModalMode::Vim);
        }
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
        self.0.editor_modal_mode == Some(EditorModalMode::Vim)
    }

    pub fn helix_mode(&self) -> bool {
        self.0.editor_modal_mode == Some(EditorModalMode::Helix)
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

    pub fn search_case_sensitive(&self) -> bool {
        self.0.search_case_sensitive.unwrap_or(false)
    }

    pub fn search_smart_case(&self) -> bool {
        self.0.search_smart_case.unwrap_or(false)
    }

    pub fn search_whole_word(&self) -> bool {
        self.0.search_whole_word.unwrap_or(false)
    }

    pub fn search_regex(&self) -> bool {
        self.0.search_regex.unwrap_or(false)
    }

    pub fn search_wrap(&self) -> bool {
        self.0.search_wrap.unwrap_or(true)
    }

    pub fn search_include_ignored(&self) -> bool {
        self.0.search_include_ignored.unwrap_or(false)
    }

    pub fn search_on_type(&self) -> bool {
        self.0.search_on_type.unwrap_or(true)
    }

    pub fn search_seed_from_cursor(&self) -> SearchSeedBehavior {
        self.0.search_seed_from_cursor.unwrap_or_default()
    }

    /// Syntax colour-scheme slug.
    pub fn editor_theme(&self) -> &str {
        self.0.editor_theme.as_deref().unwrap_or("atomone")
    }

    pub fn font_family(&self) -> &str {
        self.0
            .editor_font_family
            .as_deref()
            .unwrap_or("\"JetBrains Mono\", SFMono-Regular, Menlo, monospace")
    }

    pub fn font_size(&self) -> u32 {
        self.0.editor_font_size.unwrap_or(13)
    }

    /// Tint the line the caret is on.
    pub fn highlight_current_line(&self) -> bool {
        self.0.editor_highlight_current_line.unwrap_or(true)
    }

    pub fn line_height(&self) -> f32 {
        self.0.editor_line_height.unwrap_or(1.55).clamp(1.0, 2.5)
    }

    pub fn indentation_guides(&self) -> bool {
        self.0.editor_indentation_guides.unwrap_or(true)
    }

    pub fn whitespace(&self) -> &str {
        self.0.editor_whitespace.as_deref().unwrap_or("none")
    }

    /// Marker used when rendering a space in the editor whitespace map.
    pub fn whitespace_space_marker(&self) -> char {
        single_character_setting(self.0.editor_whitespace_space.as_deref(), '•')
    }

    /// Marker used when rendering a tab in the editor whitespace map.
    pub fn whitespace_tab_marker(&self) -> char {
        single_character_setting(self.0.editor_whitespace_tab.as_deref(), '→')
    }

    pub fn minimap(&self) -> bool {
        self.minimap_visibility() != EditorMinimapVisibility::Never
    }

    pub fn minimap_visibility(&self) -> EditorMinimapVisibility {
        self.0.editor_minimap.unwrap_or_default().visibility()
    }

    pub fn minimap_display_in(&self) -> EditorMinimapDisplayIn {
        self.0
            .editor_minimap_display_in
            .unwrap_or(EditorMinimapDisplayIn::ActiveEditor)
    }

    pub fn minimap_thumb(&self) -> EditorMinimapThumb {
        self.0
            .editor_minimap_thumb
            .unwrap_or(EditorMinimapThumb::Always)
    }

    pub fn minimap_thumb_border(&self) -> EditorMinimapThumbBorder {
        self.0
            .editor_minimap_thumb_border
            .unwrap_or(EditorMinimapThumbBorder::LeftOpen)
    }

    pub fn minimap_current_line_highlight(&self) -> EditorMinimapCurrentLineHighlight {
        self.0
            .editor_minimap_current_line_highlight
            .unwrap_or(EditorMinimapCurrentLineHighlight::Inherit)
    }

    pub fn minimap_max_width_columns(&self) -> u32 {
        self.0
            .editor_minimap_max_width_columns
            .unwrap_or(80)
            .clamp(20, 160)
    }

    pub fn scrollbar_visibility(&self) -> EditorScrollbarVisibility {
        self.0
            .editor_scrollbar_show
            .unwrap_or(EditorScrollbarVisibility::Auto)
    }

    pub fn scrollbar_cursors(&self) -> bool {
        self.0.editor_scrollbar_cursors.unwrap_or(true)
    }

    pub fn scrollbar_git_diff(&self) -> bool {
        self.0.editor_scrollbar_git_diff.unwrap_or(true)
    }

    pub fn scrollbar_search_results(&self) -> bool {
        self.0.editor_scrollbar_search_results.unwrap_or(true)
    }

    pub fn scrollbar_selected_text(&self) -> bool {
        self.0.editor_scrollbar_selected_text.unwrap_or(true)
    }

    pub fn scrollbar_selected_symbol(&self) -> bool {
        self.0.editor_scrollbar_selected_symbol.unwrap_or(true)
    }

    pub fn scrollbar_diagnostics(&self) -> EditorDiagnosticMaxSeverity {
        self.0
            .editor_scrollbar_diagnostics
            .unwrap_or(EditorDiagnosticMaxSeverity::All)
    }

    pub fn horizontal_scrollbar(&self) -> bool {
        self.0.editor_scrollbar_horizontal.unwrap_or(true)
    }

    pub fn vertical_scrollbar(&self) -> bool {
        self.0.editor_scrollbar_vertical.unwrap_or(true)
    }

    pub fn bracket_matching(&self) -> bool {
        self.0.editor_bracket_matching.unwrap_or(true)
    }

    pub fn cursor_blink(&self) -> bool {
        self.0.editor_cursor_blink.unwrap_or(true)
    }

    /// Caret blink half-period, in milliseconds (clamped to a sane range).
    pub fn cursor_blink_interval_ms(&self) -> u64 {
        self.0
            .editor_cursor_blink_interval_ms
            .unwrap_or(530)
            .clamp(100, 5_000) as u64
    }

    /// Caret shape.
    pub fn cursor_style(&self) -> EditorCursorStyle {
        self.0.editor_cursor_style.unwrap_or(EditorCursorStyle::Bar)
    }

    pub fn rulers(&self) -> &str {
        self.0.editor_rulers.as_deref().unwrap_or("")
    }

    pub fn scroll_beyond_last_line(&self) -> bool {
        self.0.editor_scroll_beyond_last_line.unwrap_or(true)
    }

    pub fn sticky_context(&self) -> bool {
        self.0.editor_sticky_context.unwrap_or(false)
    }

    pub fn diagnostics(&self) -> bool {
        self.0.editor_diagnostics.unwrap_or(true)
    }

    pub fn diagnostics_max_severity(&self) -> EditorDiagnosticMaxSeverity {
        self.0
            .editor_diagnostics_max_severity
            .unwrap_or(EditorDiagnosticMaxSeverity::All)
    }

    pub fn lsp_document_colors(&self) -> EditorDocumentColorMode {
        self.0
            .editor_lsp_document_colors
            .unwrap_or(EditorDocumentColorMode::Inlay)
    }

    pub fn error_lens(&self) -> bool {
        self.0.editor_error_lens.unwrap_or(false)
    }

    pub fn error_lens_delay_ms(&self) -> u64 {
        u64::from(self.0.editor_error_lens_delay_ms.unwrap_or(150).min(5_000))
    }

    pub fn error_lens_spacing(&self) -> u32 {
        self.0.editor_error_lens_spacing.unwrap_or(4).min(32)
    }

    pub fn error_lens_minimum_column(&self) -> u32 {
        self.0
            .editor_error_lens_minimum_column
            .unwrap_or(0)
            .min(2_000)
    }

    pub fn semantic_tokens(&self) -> bool {
        self.0.editor_semantic_tokens.unwrap_or(true)
    }

    pub fn inlay_hints(&self) -> bool {
        self.0.editor_inlay_hints.unwrap_or(false)
    }

    pub fn inlay_type_hints(&self) -> bool {
        self.0.editor_inlay_type_hints.unwrap_or(true)
    }

    pub fn inlay_parameter_hints(&self) -> bool {
        self.0.editor_inlay_parameter_hints.unwrap_or(true)
    }

    pub fn inlay_other_hints(&self) -> bool {
        self.0.editor_inlay_other_hints.unwrap_or(true)
    }

    pub fn inlay_show_background(&self) -> bool {
        self.0.editor_inlay_show_background.unwrap_or(false)
    }

    pub fn inlay_edit_debounce_ms(&self) -> u64 {
        u64::from(
            self.0
                .editor_inlay_edit_debounce_ms
                .unwrap_or(700)
                .min(10_000),
        )
    }

    pub fn inlay_scroll_debounce_ms(&self) -> u64 {
        u64::from(
            self.0
                .editor_inlay_scroll_debounce_ms
                .unwrap_or(50)
                .min(2_000),
        )
    }

    pub fn git_gutter(&self) -> bool {
        self.0.editor_git_gutter.unwrap_or(true)
    }

    /// Show conservative intra-line decorations for Git changes. Line-level
    /// gutter markers and hunk actions are controlled independently by
    /// [`Self::git_gutter`].
    pub fn git_word_diff(&self) -> bool {
        self.0.editor_git_word_diff.unwrap_or(true)
    }

    pub fn completion(&self) -> bool {
        self.0.editor_completion.unwrap_or(true)
    }

    pub fn hover(&self) -> bool {
        self.0.editor_hover.unwrap_or(true)
    }

    pub fn format_on_save(&self) -> bool {
        self.0.editor_format_on_save.unwrap_or(false)
    }

    pub fn auto_save(&self) -> bool {
        self.0.editor_auto_save.unwrap_or(false)
    }

    pub fn auto_save_delay_ms(&self) -> u64 {
        u64::from(
            self.0
                .editor_auto_save_delay
                .unwrap_or(1_000)
                .clamp(250, 60_000),
        )
    }

    pub fn trim_trailing_whitespace(&self) -> bool {
        self.0.editor_trim_trailing_whitespace.unwrap_or(false)
    }

    pub fn insert_final_newline(&self) -> bool {
        self.0.editor_insert_final_newline.unwrap_or(false)
    }

    pub fn show_cursor_position(&self) -> bool {
        self.0.editor_show_cursor_position.unwrap_or(true)
    }

    pub fn show_selection_stats(&self) -> bool {
        self.0.editor_show_selection_stats.unwrap_or(false)
    }

    pub fn show_outline(&self) -> bool {
        self.0.editor_show_outline.unwrap_or(false)
    }

    pub fn selections_menu(&self) -> bool {
        self.0.editor_selections_menu.unwrap_or(true)
    }

    pub fn toolbar_breadcrumbs(&self) -> bool {
        self.0.editor_toolbar_breadcrumbs.unwrap_or(true)
    }

    pub fn toolbar_quick_actions(&self) -> bool {
        self.0.editor_toolbar_quick_actions.unwrap_or(true)
    }

    pub fn toolbar_code_actions(&self) -> bool {
        self.0.editor_toolbar_code_actions.unwrap_or(false)
    }

    pub fn autocomplete_debounce_ms(&self) -> u32 {
        self.0
            .editor_autocomplete_debounce_ms
            .unwrap_or(350)
            .clamp(50, 2_000)
    }

    pub fn max_file_size_mb(&self) -> u32 {
        self.0.editor_max_file_size_mb.unwrap_or(10).max(1)
    }
}

/// `workspace` area (command palette preferences).
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

    /// Block the first quit attempt while a terminal/SSH shell is running.
    pub fn confirm_quit_with_active_sessions(&self) -> bool {
        self.0.confirm_quit_with_active_sessions.unwrap_or(true)
    }
}

/// `connections` area — global SSH connection-behaviour defaults.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct ConnectionsSettings(ConnectionsContent);

impl Settings for ConnectionsSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = ConnectionsContent::defaults();
        merged.merge_from(&content.connections);
        Self(merged)
    }
}

impl ConnectionsSettings {
    pub fn content(&self) -> &ConnectionsContent {
        &self.0
    }

    /// Seconds to wait for the initial TCP+SSH handshake before giving up.
    pub fn connect_timeout_secs(&self) -> u64 {
        self.0.ssh_connect_timeout_secs.unwrap_or(20)
    }

    /// Fallback keep-alive ping interval, in seconds, for hosts that don't
    /// set their own.
    pub fn keepalive_interval_secs(&self) -> u64 {
        self.0.ssh_keepalive_interval_secs.unwrap_or(25)
    }

    /// Fallback missed-ping tolerance for hosts that don't set their own.
    pub fn keepalive_max_failures(&self) -> u32 {
        self.0.ssh_keepalive_max_failures.unwrap_or(3)
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

/// `file_manager` area as read by the dual-pane SFTP browser: hidden files,
/// the `..` up-folder row, zebra striping, relative timestamps, and the
/// ordered list of visible metadata columns.
#[derive(Clone, Debug, PartialEq, RegisterSetting)]
pub struct SftpBrowserSettings(FileManagerContent);

impl Settings for SftpBrowserSettings {
    fn from_settings(content: &SettingsContent) -> Self {
        let mut merged = FileManagerContent::defaults();
        merged.merge_from(&content.file_manager);
        Self(merged)
    }
}

impl SftpBrowserSettings {
    pub fn show_hidden_files(&self) -> bool {
        self.0.sftp_show_hidden_files.unwrap_or(false)
    }

    pub fn show_up_folder(&self) -> bool {
        self.0.sftp_show_up_folder.unwrap_or(true)
    }

    pub fn zebra_striping(&self) -> bool {
        self.0.sftp_zebra_striping.unwrap_or(true)
    }

    pub fn relative_times(&self) -> bool {
        self.0.sftp_relative_times.unwrap_or(true)
    }

    /// Visible metadata columns, in display order. De-duplicated; falls back
    /// to the canonical default when the setting is absent.
    pub fn columns(&self) -> Vec<SftpColumn> {
        let raw = self
            .0
            .sftp_columns
            .clone()
            .unwrap_or_else(default_sftp_columns);
        let mut seen = Vec::with_capacity(raw.len());
        for col in raw {
            if !seen.contains(&col) {
                seen.push(col);
            }
        }
        seen
    }

    /// Local-pane fraction of the draggable split between the local and
    /// remote SFTP panes. `0.5` (50/50) when never dragged.
    pub fn split_ratio(&self) -> f32 {
        self.0.sftp_split_ratio.unwrap_or(0.5).clamp(0.2, 0.8)
    }

    /// Persisted pixel width for `col`, if the user has resized it.
    pub fn column_width(&self, col: SftpColumn) -> Option<f32> {
        self.0
            .sftp_column_widths
            .as_ref()
            .and_then(|widths| widths.get(col.token()).copied())
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
    fn theme_settings_metric_accessors() {
        // Fresh defaults: unit scale, default density, reference-style 13px fonts.
        let base = ThemeSettings::from_settings(&SettingsContent::default());
        assert_eq!(base.corner_radius_scale(), 1.0);
        assert_eq!(base.ui_density(), "default");
        assert_eq!(base.ui_font_size(), 13.0);
        assert_eq!(base.buffer_font_size(), 13.0);
    }

    #[test]
    fn terminal_settings_reads_terminal_area_with_defaults_fallback() {
        let mut content = SettingsContent::default();
        content.terminal.terminal_opacity = Some(42);
        let settings = TerminalSettings::from_settings(&content);
        assert_eq!(settings.terminal_opacity(), 42);
        assert!(!settings.copy_on_select());
    }

    #[test]
    fn terminal_settings_new_field_accessors_and_clamping() {
        let base = TerminalSettings::from_settings(&SettingsContent::default());
        assert_eq!(base.line_height(), 1.05);
        assert_eq!(base.font_weight(), TerminalFontWeight::Normal);
        assert_eq!(base.cursor_blink_interval_ms(), 1_000);
        assert_eq!(base.word_separator(), DEFAULT_WORD_SEPARATOR);
        assert_eq!(base.scroll_sensitivity(), 1);
        assert_eq!(base.fast_scroll_modifier(), FastScrollModifier::Alt);
        assert!(!base.confirm_close_terminal_tab());

        let mut content = SettingsContent::default();
        content.terminal.terminal_line_height = Some(9.0);
        content.terminal.terminal_cursor_blink_interval = Some(1);
        content.terminal.terminal_scroll_sensitivity = Some(0);
        let s = TerminalSettings::from_settings(&content);
        assert_eq!(s.line_height(), 2.5, "line height clamps to the range max");
        assert_eq!(s.cursor_blink_interval_ms(), 100, "interval clamps up");
        assert_eq!(s.scroll_sensitivity(), 1, "sensitivity floors at 1");
    }

    #[test]
    fn editor_settings_reads_git_word_diff_with_default_fallback() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert!(defaults.git_word_diff());

        let mut content = SettingsContent::default();
        content.editor.editor_git_word_diff = Some(false);
        let settings = EditorSettings::from_settings(&content);
        assert!(!settings.git_word_diff());
    }

    #[test]
    fn editor_whitespace_map_accepts_one_character_and_falls_back_on_invalid_values() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert_eq!(defaults.whitespace_space_marker(), '•');
        assert_eq!(defaults.whitespace_tab_marker(), '→');

        let mut content = SettingsContent::default();
        content.editor.editor_whitespace_space = Some("∙".to_string());
        content.editor.editor_whitespace_tab = Some("tab".to_string());
        let settings = EditorSettings::from_settings(&content);
        assert_eq!(settings.whitespace_space_marker(), '∙');
        assert_eq!(settings.whitespace_tab_marker(), '→');
    }

    #[test]
    fn editor_settings_reads_search_defaults_and_overrides() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert!(!defaults.search_case_sensitive());
        assert!(!defaults.search_smart_case());
        assert!(!defaults.search_whole_word());
        assert!(!defaults.search_regex());
        assert!(defaults.search_wrap());
        assert!(!defaults.search_include_ignored());
        assert!(defaults.search_on_type());
        assert_eq!(
            defaults.search_seed_from_cursor(),
            SearchSeedBehavior::Always
        );

        let mut content = SettingsContent::default();
        content.editor.search_case_sensitive = Some(true);
        content.editor.search_smart_case = Some(true);
        content.editor.search_whole_word = Some(true);
        content.editor.search_regex = Some(true);
        content.editor.search_wrap = Some(false);
        content.editor.search_include_ignored = Some(true);
        content.editor.search_on_type = Some(false);
        content.editor.search_seed_from_cursor = Some(SearchSeedBehavior::Selection);
        let settings = EditorSettings::from_settings(&content);
        assert!(settings.search_case_sensitive());
        assert!(settings.search_smart_case());
        assert!(settings.search_whole_word());
        assert!(settings.search_regex());
        assert!(!settings.search_wrap());
        assert!(settings.search_include_ignored());
        assert!(!settings.search_on_type());
        assert_eq!(
            settings.search_seed_from_cursor(),
            SearchSeedBehavior::Selection
        );
    }

    #[test]
    fn inlay_hints_are_off_by_default_and_can_be_enabled() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert!(!defaults.inlay_hints());
        assert!(defaults.inlay_type_hints());
        assert!(defaults.inlay_parameter_hints());
        assert!(defaults.inlay_other_hints());
        assert!(!defaults.inlay_show_background());
        assert_eq!(defaults.inlay_edit_debounce_ms(), 700);
        assert_eq!(defaults.inlay_scroll_debounce_ms(), 50);

        let mut content = SettingsContent::default();
        content.editor.editor_inlay_hints = Some(true);
        content.editor.editor_inlay_type_hints = Some(false);
        content.editor.editor_inlay_show_background = Some(true);
        content.editor.editor_inlay_edit_debounce_ms = Some(20_000);
        content.editor.editor_inlay_scroll_debounce_ms = Some(25);
        let settings = EditorSettings::from_settings(&content);
        assert!(settings.inlay_hints());
        assert!(!settings.inlay_type_hints());
        assert!(settings.inlay_show_background());
        assert_eq!(settings.inlay_edit_debounce_ms(), 10_000);
        assert_eq!(settings.inlay_scroll_debounce_ms(), 25);
    }

    #[test]
    fn error_lens_settings_have_safe_defaults_and_clamped_values() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert_eq!(
            defaults.diagnostics_max_severity(),
            EditorDiagnosticMaxSeverity::All
        );
        assert!(!defaults.error_lens());
        assert_eq!(defaults.error_lens_delay_ms(), 150);
        assert_eq!(defaults.error_lens_spacing(), 4);
        assert_eq!(defaults.error_lens_minimum_column(), 0);

        let mut content = SettingsContent::default();
        content.editor.editor_diagnostics_max_severity = Some(EditorDiagnosticMaxSeverity::Warning);
        content.editor.editor_error_lens = Some(true);
        content.editor.editor_error_lens_delay_ms = Some(9_000);
        content.editor.editor_error_lens_spacing = Some(40);
        content.editor.editor_error_lens_minimum_column = Some(9_000);
        let settings = EditorSettings::from_settings(&content);
        assert_eq!(
            settings.diagnostics_max_severity(),
            EditorDiagnosticMaxSeverity::Warning
        );
        assert!(settings.error_lens());
        assert_eq!(settings.error_lens_delay_ms(), 5_000);
        assert_eq!(settings.error_lens_spacing(), 32);
        assert_eq!(settings.error_lens_minimum_column(), 2_000);
    }

    #[test]
    fn document_color_mode_defaults_to_inlay_and_accepts_each_mode() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert_eq!(
            defaults.lsp_document_colors(),
            EditorDocumentColorMode::Inlay
        );

        let mut content = SettingsContent::default();
        content.editor.editor_lsp_document_colors = Some(EditorDocumentColorMode::Inline);
        assert_eq!(
            EditorSettings::from_settings(&content).lsp_document_colors(),
            EditorDocumentColorMode::Inline
        );
    }

    #[test]
    fn editor_minimap_settings_default_and_legacy_boolean_mapping() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert_eq!(
            defaults.minimap_visibility(),
            EditorMinimapVisibility::Never
        );
        assert!(!defaults.minimap());
        assert_eq!(
            defaults.minimap_display_in(),
            EditorMinimapDisplayIn::ActiveEditor
        );
        assert_eq!(defaults.minimap_thumb(), EditorMinimapThumb::Always);
        assert_eq!(
            defaults.minimap_thumb_border(),
            EditorMinimapThumbBorder::LeftOpen
        );
        assert_eq!(
            defaults.minimap_current_line_highlight(),
            EditorMinimapCurrentLineHighlight::Inherit
        );
        assert_eq!(defaults.minimap_max_width_columns(), 80);

        let mut content = SettingsContent::default();
        content.editor.editor_minimap =
            Some(labonair_settings_content::editor::EditorMinimapSetting::Legacy(true));
        let legacy = EditorSettings::from_settings(&content);
        assert_eq!(legacy.minimap_visibility(), EditorMinimapVisibility::Always);
        assert!(legacy.minimap());
    }

    #[test]
    fn editor_scrollbar_settings_default_and_override() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert_eq!(
            defaults.scrollbar_visibility(),
            EditorScrollbarVisibility::Auto
        );
        assert!(defaults.scrollbar_cursors());
        assert!(defaults.scrollbar_git_diff());
        assert!(defaults.scrollbar_search_results());
        assert!(defaults.scrollbar_selected_text());
        assert!(defaults.scrollbar_selected_symbol());
        assert_eq!(
            defaults.scrollbar_diagnostics(),
            EditorDiagnosticMaxSeverity::All
        );
        assert!(defaults.horizontal_scrollbar());
        assert!(defaults.vertical_scrollbar());

        let mut content = SettingsContent::default();
        content.editor.editor_scrollbar_show = Some(EditorScrollbarVisibility::Never);
        content.editor.editor_scrollbar_selected_symbol = Some(false);
        content.editor.editor_scrollbar_diagnostics = Some(EditorDiagnosticMaxSeverity::Warning);
        content.editor.editor_scrollbar_horizontal = Some(false);
        let settings = EditorSettings::from_settings(&content);
        assert_eq!(
            settings.scrollbar_visibility(),
            EditorScrollbarVisibility::Never
        );
        assert!(!settings.scrollbar_selected_symbol());
        assert_eq!(
            settings.scrollbar_diagnostics(),
            EditorDiagnosticMaxSeverity::Warning
        );
        assert!(!settings.horizontal_scrollbar());
    }

    #[test]
    fn editor_selections_menu_defaults_on_and_can_be_hidden() {
        let defaults = EditorSettings::from_settings(&SettingsContent::default());
        assert!(defaults.selections_menu());
        assert!(defaults.toolbar_breadcrumbs());
        assert!(defaults.toolbar_quick_actions());
        assert!(!defaults.toolbar_code_actions());

        let mut content = SettingsContent::default();
        content.editor.editor_selections_menu = Some(false);
        content.editor.editor_toolbar_breadcrumbs = Some(false);
        content.editor.editor_toolbar_quick_actions = Some(false);
        content.editor.editor_toolbar_code_actions = Some(true);
        let settings = EditorSettings::from_settings(&content);
        assert!(!settings.selections_menu());
        assert!(!settings.toolbar_breadcrumbs());
        assert!(!settings.toolbar_quick_actions());
        assert!(settings.toolbar_code_actions());
    }

    #[test]
    fn sftp_split_ratio_defaults_and_clamps() {
        let base = SftpBrowserSettings::from_settings(&SettingsContent::default());
        assert_eq!(base.split_ratio(), 0.5, "50/50 when never dragged");

        let mut content = SettingsContent::default();
        content.file_manager.sftp_split_ratio = Some(0.05);
        let s = SftpBrowserSettings::from_settings(&content);
        assert_eq!(s.split_ratio(), 0.2, "clamps below the allowed range");

        content.file_manager.sftp_split_ratio = Some(0.95);
        let s = SftpBrowserSettings::from_settings(&content);
        assert_eq!(s.split_ratio(), 0.8, "clamps above the allowed range");
    }

    #[test]
    fn sftp_column_width_round_trips_by_token_and_falls_back_to_none() {
        let base = SftpBrowserSettings::from_settings(&SettingsContent::default());
        assert_eq!(
            base.column_width(SftpColumn::Size),
            None,
            "no persisted width yet"
        );

        let mut content = SettingsContent::default();
        content.file_manager.sftp_column_widths =
            Some([("size".to_string(), 120.0_f32)].into_iter().collect());
        let s = SftpBrowserSettings::from_settings(&content);
        assert_eq!(s.column_width(SftpColumn::Size), Some(120.0));
        assert_eq!(
            s.column_width(SftpColumn::Modified),
            None,
            "unset columns still fall back"
        );
    }
}
