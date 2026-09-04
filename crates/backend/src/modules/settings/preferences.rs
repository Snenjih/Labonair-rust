//! Central, typed application preferences (T13-001).
//!
//! Port of the relevant subset of `reference-src/src/modules/settings/store.ts`
//! (`Preferences` type + `DEFAULT_PREFERENCES` + `loadPreferences`). The web
//! app kept ~130 loosely-typed keys in a `LazyStore`; here the fields the Rust
//! app actually consumes are modelled as one concretely-typed struct, grouped
//! into the same categories, persisted as a `preferences` object inside the
//! shared `labonair-settings.json` (the same file `settings::editor` /
//! `settings::mcp` / the bar-item registry use).
//!
//! Persistence rules:
//! * load at startup, write on every change;
//! * missing / unknown fields fall back to their `Default` value
//!   (`#[serde(default)]` per field) — a preferences file written by an older
//!   build never fails to load;
//! * a corrupt settings file (not valid JSON, or not a JSON object) is moved
//!   aside to `labonair-settings.json.bak` and defaults are used, so a bad
//!   write can never brick the app.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::modules::fs::paths::config_dir;

const SETTINGS_FILE: &str = "labonair-settings.json";
const KEY: &str = "preferences";

/// The app theme the user picked. `System` follows the OS appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub const ALL: [ThemePref; 3] = [ThemePref::System, ThemePref::Light, ThemePref::Dark];

    pub fn as_str(self) -> &'static str {
        match self {
            ThemePref::System => "system",
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
        }
    }
}

/// Which tab the app opens on launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StartupTab {
    Terminal,
    #[default]
    HostManager,
}

/// Terminal cursor shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

/// Command-palette match strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaletteSearchMode {
    #[default]
    Contains,
    StartsWith,
    Fuzzy,
}

/// The concretely-typed preferences model. Field order mirrors the category
/// grouping the settings UI renders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    // ── General ──────────────────────────────────────────────────────────
    pub theme: ThemePref,
    pub restore_window_state: bool,
    pub default_startup_tab: StartupTab,
    /// Number of terminals opened on launch (1..=3).
    pub startup_terminal_count: u32,
    /// Launch the app at login (mirrors the OS autostart entry).
    pub autostart: bool,
    /// Encrypt stored credentials at rest (drives `secrets_set_encryption_enabled`).
    pub credential_encryption: bool,
    pub notify_on_errors: bool,
    pub confirm_quit_with_ssh: bool,
    pub check_for_updates: bool,
    /// Reopen the previous tabs / split layout on the next launch (T14-001).
    pub session_restore: bool,

    // ── Appearance & Layout ──────────────────────────────────────────────
    /// Active JSON theme id (`"default"` = built-in light/dark).
    pub app_theme: String,
    /// Per-theme light/dark variant overrides (`{ id: { light?, dark? } }`).
    pub theme_variant_overrides: BTreeMap<String, Value>,
    pub app_font_size: u32,
    pub app_line_height: f32,
    /// UI font family (full CSS stack; empty = system default).
    pub app_font_family: String,
    pub reduce_motion: bool,
    pub app_corner_radius: u32,
    /// Background image filename (empty = none).
    pub background_image: String,
    pub background_opacity: u32,
    pub background_blur: u32,
    pub background_tint_color: String,
    pub background_tint_opacity: u32,
    /// `"titlebar"` | `"sidebar"`.
    pub tabs_location: String,
    /// Up to two of `path`/`connection`/`host`/`uptime`/`transfer`/`busy`.
    pub sidebar_tab_info_line: Vec<String>,
    pub sidebar_group_by_folder: bool,
    pub sidebar_group_single_tabs: bool,
    /// Per-item titlebar/statusbar/sidebar placement map (typed access lives in
    /// `settings::bar_items`; stored here verbatim for lossless roundtrip).
    pub bar_item_placements: BTreeMap<String, Value>,
    pub bar_layout_migrated: bool,
    pub badges_always_visible: bool,
    /// Legacy titlebar icon side (`"auto"` | `"left"` | `"right"`).
    pub titlebars_icons_position: String,
    /// Zen mode (T13-005): show the window header bar. Both zen flags visible =
    /// zen mode off; `view.zenMode` toggles both together.
    pub zen_mode_show_header: bool,
    /// Zen mode (T13-005): show the bottom status bar.
    pub zen_mode_show_statusbar: bool,

    // ── Status Bar toggles ───────────────────────────────────────────────
    pub status_bar_show_explorer_button: bool,
    pub status_bar_show_snippets_button: bool,
    pub status_bar_show_source_control_button: bool,
    pub status_bar_show_tabs_button: bool,
    pub status_bar_show_cwd_breadcrumb: bool,
    pub status_bar_show_preview_url: bool,
    pub status_bar_show_ai_controls: bool,

    // ── Sidebar / Host-Manager state (persisted, no settings row) ─────────
    pub sidebar_position: String,
    pub sidebar_open: bool,
    pub sidebar_active_panel: String,
    pub sidebar_right_open: bool,
    pub sidebar_right_active_panel: String,
    pub sidebar_width: u32,
    pub sidebar_right_width: u32,
    /// T17-002 dock layout: JSON array of `DockData` (open / size / zoom /
    /// active / panel order per edge dock). Empty string = not yet persisted;
    /// the shell then migrates the legacy `sidebar_*` fields above on first run.
    pub dock_layout: String,
    pub hm_layout: String,
    pub hm_sort: String,
    pub hm_card_scale: u32,

    // ── Terminal ─────────────────────────────────────────────────────────
    pub terminal_shell: String,
    pub terminal_default_path: String,
    pub new_tab_inherits_cwd: bool,
    pub confirm_close_terminal_tab: bool,
    pub terminal_font_family: String,
    pub terminal_font_size: u32,
    /// `"normal"` | `"medium"` | `"bold"`.
    pub terminal_font_weight: String,
    pub terminal_letter_spacing: f32,
    pub terminal_line_height: f32,
    pub terminal_scrollback: u32,
    /// Rows of scrollback persisted per pane on quit and replayed on the next
    /// launch (T14-002). `0` = persist everything the buffer holds.
    pub session_scrollback_lines: u32,
    /// Per-file ceiling for a persisted scrollback, in MB.
    pub scrollback_max_size_mb: u32,
    /// Days a persisted scrollback file is kept before cleanup deletes it.
    /// `0` = keep until the pane/session goes away.
    pub scrollback_retention_days: u32,
    pub terminal_cursor_style: CursorStyle,
    pub terminal_cursor_blink: bool,
    pub terminal_cursor_blink_interval: u32,
    pub terminal_copy_on_select: bool,
    pub terminal_right_click_pastes: bool,
    pub terminal_word_separator: String,
    pub terminal_scroll_sensitivity: u32,
    /// `"none"` | `"alt"` | `"ctrl"` | `"shift"`.
    pub terminal_fast_scroll_modifier: String,
    pub terminal_show_pane_header: bool,
    pub terminal_show_pane_footer: bool,
    pub terminal_use_webgl: bool,
    pub terminal_composer_enabled: bool,
    pub terminal_composer_history_popup: bool,
    pub terminal_composer_argument_completion: bool,
    pub terminal_blocks_enabled: bool,
    pub terminal_blocks_auto_collapse_on_alt_screen: bool,
    pub terminal_bell: bool,
    /// Terminal background opacity in percent (100 = fully opaque).
    /// PORT-ONLY: no reference row — a deliberate GPUI addition so a translucent
    /// terminal pane is possible independently of the whole-window
    /// `background_opacity`. Documented in `## Notizen` of T16-011.
    pub terminal_opacity: u32,

    // ── Editor ───────────────────────────────────────────────────────────
    pub editor_font_family: String,
    pub editor_font_size: u32,
    pub editor_line_height: f32,
    pub editor_tab_size: u32,
    pub editor_word_wrap: bool,
    pub editor_line_numbers: bool,
    /// PORT-ONLY: Vim `relativenumber` surfaced as a setting row (the reference
    /// keeps it only in the internal `EditorPrefs`). Kept because
    /// `Preferences::editor_prefs()` feeds it to the editor view.
    pub editor_relative_line_numbers: bool,
    pub editor_indent_with_tabs: bool,
    pub editor_format_on_save: bool,
    pub editor_trim_trailing_whitespace: bool,
    pub editor_insert_final_newline: bool,
    pub editor_bracket_matching: bool,
    pub editor_show_cursor_position: bool,
    pub editor_show_selection_stats: bool,
    pub editor_show_outline: bool,
    pub editor_indentation_guides: bool,
    /// `"off"` | `"afterDelay"` | `"onFocusChange"`.
    pub editor_auto_save: String,
    pub editor_auto_save_delay: u32,
    pub editor_autocomplete_debounce_ms: u32,
    pub editor_max_file_size_mb: u32,
    #[serde(rename = "vimMode")]
    pub editor_vim_mode: bool,
    /// Syntax colour scheme slug. Reference options: atomone/aura/copilot/
    /// github-dark/github-light/nord/tokyo-night/xcode-dark/xcode-light.
    /// PORT-ONLY extra value `"auto"` follows the app theme (documented).
    pub editor_theme: String,

    // ── File Manager ─────────────────────────────────────────────────────
    pub sftp_show_hidden_files: bool,
    pub sftp_show_up_folder: bool,
    pub explorer_show_hidden_by_default: bool,
    pub sftp_column_size: bool,
    pub sftp_column_modified: bool,
    pub sftp_column_permissions: bool,
    pub sftp_column_type: bool,
    pub sftp_remote_edit_show_transfers: bool,
    pub sftp_max_remote_file_size_mb: u32,
    pub sftp_font_size: u32,
    pub sftp_max_concurrent_transfers: u32,
    /// `"ask"` | `"overwrite"` | `"skip"`.
    pub sftp_default_conflict_resolution: String,
    pub sftp_chunk_size_kb: u32,
    /// `"ask"` | `"skip"` | `"abort"`.
    pub sftp_on_folder_file_error: String,

    // ── Connections (SSH / Explorer / Host availability) ─────────────────
    pub host_ping_interval: u32,
    pub ssh_connect_timeout_secs: u32,
    pub ssh_auto_reconnect: bool,
    pub ssh_auto_reconnect_delay: u32,
    pub ssh_auto_reconnect_max_attempts: u32,
    pub explorer_remote_poll_interval: u32,
    pub explorer_auto_reconnect: bool,
    pub explorer_idle_session_timeout_min: u32,
    pub explorer_max_idle_sessions: u32,
    pub explorer_max_cached_remote_scopes: u32,

    // ── Command Palette ──────────────────────────────────────────────────
    pub command_palette_search_mode: PaletteSearchMode,
    pub command_palette_show_recent: bool,
    pub command_palette_blur: u32,
    pub command_palette_opacity: u32,
    /// `"top"` | `"high"` | `"center"`.
    pub command_palette_position: String,
    /// `"fast"` | `"normal"` | `"slow"` | `"none"`.
    pub command_palette_animation: String,
    pub command_palette_history_size: u32,
    pub command_palette_close_on_overlay_click: bool,

    // ── Source Control ───────────────────────────────────────────────────
    pub git_status_poll_interval_ms: u32,

    // ── Bookmarks (T12-003) ──────────────────────────────────────────────
    pub bookmarks_enabled: bool,
    pub bookmarks_action_new_terminal: bool,
    pub bookmarks_action_current_terminal: bool,
    pub bookmarks_action_current_sftp: bool,
    pub bookmarks_action_new_sftp: bool,
    /// `"current"` | `"new"`.
    pub bookmarks_primary_click_behavior: String,
    pub bookmarks_show_badge: bool,

    // ── AI ───────────────────────────────────────────────────────────────
    pub ai_enabled: bool,
    pub ai_max_agent_steps: u32,
    pub ai_terminal_context_lines: u32,
    pub ai_temperature: f32,
    pub ai_warn_destructive_commands: bool,
    pub ai_auto_open_mini_on_send: bool,
    pub ai_notify_on_headless_command: bool,
    pub ai_shell_max_timeout_secs: u32,
    pub ai_shell_max_output_kb: u32,
    pub default_model_id: String,
    pub custom_instructions: String,
    pub autocomplete_enabled: bool,
    pub autocomplete_provider: String,
    pub autocomplete_model_id: String,
    #[serde(rename = "lmstudioBaseURL")]
    pub lmstudio_base_url: String,
    pub lmstudio_chat_model_id: String,
    #[serde(rename = "openaiCompatibleBaseURL")]
    pub openai_compatible_base_url: String,
    pub openai_compatible_model_id: String,
    #[serde(rename = "mlxBaseURL")]
    pub mlx_base_url: String,
    pub mlx_chat_model_id: String,
    #[serde(rename = "ollamaBaseURL")]
    pub ollama_base_url: String,
    pub ollama_chat_model_id: String,

    // ── AI Agent Bridge (MCP) mirror ─────────────────────────────────────
    /// Mirrors of `settings::mcp::McpPrefs`, kept here so the value roundtrips
    /// and the global search can address it. `settings::mcp` remains the
    /// authoritative store the bridge reads.
    pub mcp_bridge_enabled: bool,
    pub mcp_bridge_port: u32,
    pub mcp_max_command_timeout_secs: u32,
    pub mcp_auto_revoke_minutes: u32,
    pub mcp_notify_on_activity: bool,

    // ── Keyboard Shortcuts ───────────────────────────────────────────────
    /// User keybind overrides: shortcut slug (`"tab.new"`, …) → keystroke
    /// string. An empty string disables the shortcut; an absent key means
    /// "use the built-in default". An empty map = all defaults (first run).
    pub keybinds: std::collections::BTreeMap<String, String>,
}

impl Default for Preferences {
    fn default() -> Self {
        let mono = "\"JetBrains Mono\", SFMono-Regular, Menlo, monospace".to_string();
        Self {
            theme: ThemePref::System,
            restore_window_state: true,
            default_startup_tab: StartupTab::HostManager,
            startup_terminal_count: 1,
            autostart: false,
            credential_encryption: false,
            notify_on_errors: false,
            confirm_quit_with_ssh: true,
            check_for_updates: true,
            session_restore: false,

            app_theme: "default".to_string(),
            theme_variant_overrides: BTreeMap::new(),
            app_font_size: 13,
            app_line_height: 1.5,
            app_font_family: "\"Inter Variable\", sans-serif".to_string(),
            reduce_motion: false,
            app_corner_radius: 5,
            background_image: String::new(),
            background_opacity: 30,
            background_blur: 0,
            background_tint_color: "#000000".to_string(),
            background_tint_opacity: 0,
            tabs_location: "titlebar".to_string(),
            sidebar_tab_info_line: Vec::new(),
            sidebar_group_by_folder: false,
            sidebar_group_single_tabs: false,
            bar_item_placements: BTreeMap::new(),
            bar_layout_migrated: false,
            badges_always_visible: true,
            titlebars_icons_position: "auto".to_string(),
            zen_mode_show_header: true,
            zen_mode_show_statusbar: true,

            status_bar_show_explorer_button: true,
            status_bar_show_snippets_button: true,
            status_bar_show_source_control_button: true,
            status_bar_show_tabs_button: true,
            status_bar_show_cwd_breadcrumb: true,
            status_bar_show_preview_url: true,
            status_bar_show_ai_controls: true,

            sidebar_position: "left".to_string(),
            sidebar_open: true,
            sidebar_active_panel: "explorer".to_string(),
            sidebar_right_open: false,
            sidebar_right_active_panel: "explorer".to_string(),
            sidebar_width: 225,
            sidebar_right_width: 225,
            dock_layout: String::new(),
            hm_layout: "grid".to_string(),
            hm_sort: "last_connected".to_string(),
            hm_card_scale: 100,

            terminal_shell: String::new(),
            terminal_default_path: String::new(),
            new_tab_inherits_cwd: true,
            confirm_close_terminal_tab: false,
            terminal_font_family: mono.clone(),
            terminal_font_size: 14,
            terminal_font_weight: "normal".to_string(),
            terminal_letter_spacing: 0.0,
            terminal_line_height: 1.05,
            terminal_scrollback: 5_000,
            session_scrollback_lines: 1_000,
            scrollback_max_size_mb: 10,
            scrollback_retention_days: 0,
            terminal_cursor_style: CursorStyle::Bar,
            terminal_cursor_blink: true,
            terminal_cursor_blink_interval: 1000,
            terminal_copy_on_select: false,
            terminal_right_click_pastes: false,
            terminal_word_separator: " ()[]{}',\"`".to_string(),
            terminal_scroll_sensitivity: 1,
            terminal_fast_scroll_modifier: "alt".to_string(),
            terminal_show_pane_header: false,
            terminal_show_pane_footer: false,
            terminal_use_webgl: true,
            terminal_composer_enabled: false,
            terminal_composer_history_popup: false,
            terminal_composer_argument_completion: true,
            terminal_blocks_enabled: false,
            terminal_blocks_auto_collapse_on_alt_screen: true,
            terminal_bell: false,
            terminal_opacity: 100,

            editor_font_family: mono.clone(),
            editor_font_size: 13,
            editor_line_height: 1.55,
            editor_tab_size: 2,
            editor_word_wrap: false,
            editor_line_numbers: true,
            editor_relative_line_numbers: false,
            editor_indent_with_tabs: false,
            editor_format_on_save: false,
            editor_trim_trailing_whitespace: false,
            editor_insert_final_newline: false,
            editor_bracket_matching: true,
            editor_show_cursor_position: true,
            editor_show_selection_stats: true,
            editor_show_outline: false,
            editor_indentation_guides: true,
            editor_auto_save: "off".to_string(),
            editor_auto_save_delay: 1000,
            editor_autocomplete_debounce_ms: 350,
            editor_max_file_size_mb: 10,
            editor_vim_mode: false,
            editor_theme: "atomone".to_string(),

            sftp_show_hidden_files: false,
            sftp_show_up_folder: true,
            explorer_show_hidden_by_default: false,
            sftp_column_size: true,
            sftp_column_modified: true,
            sftp_column_permissions: true,
            sftp_column_type: false,
            sftp_remote_edit_show_transfers: true,
            sftp_max_remote_file_size_mb: 5,
            sftp_font_size: 13,
            sftp_max_concurrent_transfers: 2,
            sftp_default_conflict_resolution: "ask".to_string(),
            sftp_chunk_size_kb: 64,
            sftp_on_folder_file_error: "ask".to_string(),

            host_ping_interval: 60,
            ssh_connect_timeout_secs: 10,
            ssh_auto_reconnect: false,
            ssh_auto_reconnect_delay: 5,
            ssh_auto_reconnect_max_attempts: 3,
            explorer_remote_poll_interval: 20,
            explorer_auto_reconnect: false,
            explorer_idle_session_timeout_min: 5,
            explorer_max_idle_sessions: 3,
            explorer_max_cached_remote_scopes: 5,

            command_palette_search_mode: PaletteSearchMode::Contains,
            command_palette_show_recent: true,
            command_palette_blur: 4,
            command_palette_opacity: 95,
            command_palette_position: "top".to_string(),
            command_palette_animation: "normal".to_string(),
            command_palette_history_size: 5,
            command_palette_close_on_overlay_click: true,

            git_status_poll_interval_ms: 5000,

            bookmarks_enabled: true,
            bookmarks_action_new_terminal: true,
            bookmarks_action_current_terminal: true,
            bookmarks_action_current_sftp: true,
            bookmarks_action_new_sftp: true,
            bookmarks_primary_click_behavior: "current".to_string(),
            bookmarks_show_badge: true,

            ai_enabled: true,
            ai_max_agent_steps: 24,
            ai_terminal_context_lines: 300,
            ai_temperature: 0.7,
            ai_warn_destructive_commands: true,
            ai_auto_open_mini_on_send: true,
            ai_notify_on_headless_command: true,
            ai_shell_max_timeout_secs: 300,
            ai_shell_max_output_kb: 256,
            default_model_id: String::new(),
            custom_instructions: String::new(),
            autocomplete_enabled: false,
            autocomplete_provider: "cerebras".to_string(),
            autocomplete_model_id: String::new(),
            lmstudio_base_url: "http://localhost:1234/v1".to_string(),
            lmstudio_chat_model_id: String::new(),
            openai_compatible_base_url: String::new(),
            openai_compatible_model_id: String::new(),
            mlx_base_url: "http://localhost:8080".to_string(),
            mlx_chat_model_id: String::new(),
            ollama_base_url: "http://localhost:11434".to_string(),
            ollama_chat_model_id: String::new(),

            mcp_bridge_enabled: false,
            mcp_bridge_port: 47823,
            mcp_max_command_timeout_secs: 300,
            mcp_auto_revoke_minutes: 0,
            mcp_notify_on_activity: false,

            keybinds: std::collections::BTreeMap::new(),
        }
    }
}

impl Preferences {
    /// Project the editor-relevant preferences onto the [`EditorPrefs`] the
    /// editor view consumes, keeping the search-related Vim options
    /// (`hlsearch` / `incsearch` / `smartcase`) at their persisted values.
    pub fn editor_prefs(&self) -> super::editor::EditorPrefs {
        let base = super::editor::editor_prefs_load();
        super::editor::EditorPrefs {
            vim_mode: self.editor_vim_mode,
            number: self.editor_line_numbers,
            relative_number: self.editor_relative_line_numbers,
            expandtab: !self.editor_indent_with_tabs,
            tabstop: self.editor_tab_size as usize,
            shiftwidth: self.editor_tab_size as usize,
            ..base
        }
    }
}

/// Load the persisted preferences (defaults if none saved / on corruption).
pub fn preferences_load() -> Preferences {
    load_from(&config_dir())
}

/// Persist the preferences, merging into the shared settings file.
pub fn preferences_save(prefs: &Preferences) -> Result<(), String> {
    save_to(&config_dir(), prefs)
}

/// Load preferences from an explicit config directory (testing / non-default
/// profiles). `dir` is the directory that contains `labonair-settings.json`.
pub fn preferences_load_from(dir: &std::path::Path) -> Preferences {
    load_from(dir)
}

/// Persist preferences into an explicit config directory.
pub fn preferences_save_to(dir: &std::path::Path, prefs: &Preferences) -> Result<(), String> {
    save_to(dir, prefs)
}

fn load_from(dir: &std::path::Path) -> Preferences {
    let path = dir.join(SETTINGS_FILE);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Preferences::default();
    };
    match serde_json::from_str::<serde_json::Value>(&raw) {
        Ok(serde_json::Value::Object(map)) => map
            .get(KEY)
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        _ => {
            // File exists but is not a JSON object → corrupt. Preserve it for
            // forensics and fall back to defaults rather than crash / clobber.
            let _ = std::fs::rename(&path, path.with_extension("json.bak"));
            Preferences::default()
        }
    }
}

fn save_to(dir: &std::path::Path, prefs: &Preferences) -> Result<(), String> {
    let path = dir.join(SETTINGS_FILE);
    let mut map = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    map.insert(
        KEY.to_string(),
        serde_json::to_value(prefs).map_err(|e| e.to_string())?,
    );
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-prefs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn roundtrip_merges_into_shared_file() {
        let dir = tmp();
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"editor":{"number":true}}"#).unwrap();

        let p = Preferences {
            theme: ThemePref::Dark,
            terminal_font_size: 16,
            editor_word_wrap: true,
            ..Default::default()
        };
        save_to(&dir, &p).unwrap();

        let back = load_from(&dir);
        assert_eq!(back.theme, ThemePref::Dark);
        assert_eq!(back.terminal_font_size, 16);
        assert!(back.editor_word_wrap);

        let raw = std::fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap();
        assert!(raw.contains("\"editor\""), "unrelated keys preserved");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("labonair-prefs-{}", uuid::Uuid::new_v4()));
        assert_eq!(load_from(&dir), Preferences::default());
    }

    #[test]
    fn partial_json_falls_back_field_by_field() {
        let dir = tmp();
        std::fs::write(
            dir.join(SETTINGS_FILE),
            r#"{"preferences":{"terminalFontSize":20}}"#,
        )
        .unwrap();
        let back = load_from(&dir);
        assert_eq!(back.terminal_font_size, 20);
        assert_eq!(
            back.editor_font_size,
            Preferences::default().editor_font_size,
            "missing field uses default"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_file_is_backed_up_and_defaults_load() {
        let dir = tmp();
        std::fs::write(dir.join(SETTINGS_FILE), "not json at all {{{").unwrap();
        let back = load_from(&dir);
        assert_eq!(back, Preferences::default());
        assert!(
            dir.join("labonair-settings.json.bak").exists(),
            "corrupt file preserved as .bak"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn new_terminal_editor_fields_have_sensible_defaults() {
        let p = Preferences::default();
        assert_eq!(p.terminal_opacity, 100);
        assert_eq!(p.session_scrollback_lines, 1_000);
        assert_eq!(p.scrollback_max_size_mb, 10);
        assert_eq!(p.scrollback_retention_days, 0);
        assert_eq!(p.editor_theme, "atomone");
        assert!(!p.editor_vim_mode);
        assert!(!p.editor_relative_line_numbers);
        assert!(
            !p.session_restore,
            "session restore is off by default (ref parity)"
        );
        assert!(p.zen_mode_show_header, "header shown by default");
        assert!(p.zen_mode_show_statusbar, "status bar shown by default");
    }

    #[test]
    fn zen_mode_prefs_roundtrip() {
        let dir = tmp();
        let p = Preferences {
            zen_mode_show_header: false,
            zen_mode_show_statusbar: false,
            ..Default::default()
        };
        save_to(&dir, &p).unwrap();
        let back = load_from(&dir);
        assert!(!back.zen_mode_show_header);
        assert!(!back.zen_mode_show_statusbar);
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["zenModeShowHeader"], false);
        assert_eq!(json["zenModeShowStatusbar"], false);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn editor_prefs_projection_maps_fields() {
        let p = Preferences {
            editor_vim_mode: true,
            editor_line_numbers: false,
            editor_relative_line_numbers: true,
            editor_indent_with_tabs: true,
            editor_tab_size: 2,
            ..Default::default()
        };
        let e = p.editor_prefs();
        assert!(e.vim_mode);
        assert!(!e.number);
        assert!(e.relative_number);
        assert!(!e.expandtab);
        assert_eq!(e.tabstop, 2);
        assert_eq!(e.shiftwidth, 2);
    }

    #[test]
    fn keybinds_default_empty_and_roundtrip() {
        let dir = tmp();
        let mut p = Preferences::default();
        assert!(p.keybinds.is_empty(), "fresh install runs on defaults");
        p.keybinds.insert("tab.new".into(), "cmd-shift-t".into());
        p.keybinds.insert("pane.close".into(), String::new());
        save_to(&dir, &p).unwrap();
        let back = load_from(&dir);
        assert_eq!(
            back.keybinds.get("tab.new").map(String::as_str),
            Some("cmd-shift-t")
        );
        assert_eq!(
            back.keybinds.get("pane.close").map(String::as_str),
            Some("")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enums_serialize_to_reference_token_strings() {
        let json = serde_json::to_value(Preferences::default()).unwrap();
        assert_eq!(json["theme"], "system");
        assert_eq!(json["defaultStartupTab"], "host-manager");
        assert_eq!(json["terminalCursorStyle"], "bar");
        assert_eq!(json["commandPaletteSearchMode"], "contains");
        assert_eq!(json["vimMode"], false, "vim mode uses the reference key");
        assert!(json.get("editorVimMode").is_none(), "no legacy vim key");
    }

    #[test]
    fn reference_settings_blob_roundtrips() {
        let dir = tmp();
        // A representative slice of `reference-src` DEFAULT_PREFERENCES keys.
        std::fs::write(
            dir.join(SETTINGS_FILE),
            r#"{"preferences":{
                "vimMode":true,
                "defaultStartupTab":"terminal",
                "startupTerminalCount":3,
                "autostart":true,
                "sessionRestore":true,
                "sessionScrollbackLines":2000,
                "scrollbackMaxSizeMb":25,
                "appTheme":"nord",
                "appFontFamily":"\"Inter Variable\", sans-serif",
                "backgroundOpacity":40,
                "appCornerRadius":8,
                "tabsLocation":"sidebar",
                "sidebarTabInfoLine":["path","host"],
                "terminalFontSize":16,
                "terminalCursorStyle":"underline",
                "terminalWordSeparator":" ()",
                "terminalFastScrollModifier":"ctrl",
                "editorTabSize":4,
                "editorTheme":"nord",
                "editorAutoSave":"afterDelay",
                "editorLineHeight":1.8,
                "sftpColumnType":true,
                "sftpMaxConcurrentTransfers":6,
                "hostPingInterval":30,
                "sshAutoReconnect":true,
                "explorerRemotePollInterval":10,
                "commandPaletteBlur":12,
                "commandPalettePosition":"center",
                "bookmarksEnabled":false,
                "bookmarksPrimaryClickBehavior":"new",
                "aiMaxAgentSteps":40,
                "aiTemperature":0.3,
                "defaultModelId":"claude-x",
                "ollamaBaseURL":"http://host:1",
                "mcpBridgePort":50000,
                "statusBarShowAiControls":false
            }}"#,
        )
        .unwrap();
        let p = load_from(&dir);
        assert!(p.editor_vim_mode);
        assert_eq!(p.default_startup_tab, StartupTab::Terminal);
        assert_eq!(p.startup_terminal_count, 3);
        assert!(p.autostart);
        assert!(p.session_restore);
        assert_eq!(p.session_scrollback_lines, 2000);
        assert_eq!(p.scrollback_max_size_mb, 25);
        assert_eq!(p.app_theme, "nord");
        assert_eq!(p.background_opacity, 40);
        assert_eq!(p.app_corner_radius, 8);
        assert_eq!(p.tabs_location, "sidebar");
        assert_eq!(p.sidebar_tab_info_line, vec!["path", "host"]);
        assert_eq!(p.terminal_font_size, 16);
        assert_eq!(p.terminal_cursor_style, CursorStyle::Underline);
        assert_eq!(p.terminal_word_separator, " ()");
        assert_eq!(p.terminal_fast_scroll_modifier, "ctrl");
        assert_eq!(p.editor_tab_size, 4);
        assert_eq!(p.editor_theme, "nord");
        assert_eq!(p.editor_auto_save, "afterDelay");
        assert!((p.editor_line_height - 1.8).abs() < f32::EPSILON);
        assert!(p.sftp_column_type);
        assert_eq!(p.sftp_max_concurrent_transfers, 6);
        assert_eq!(p.host_ping_interval, 30);
        assert!(p.ssh_auto_reconnect);
        assert_eq!(p.explorer_remote_poll_interval, 10);
        assert_eq!(p.command_palette_blur, 12);
        assert_eq!(p.command_palette_position, "center");
        assert!(!p.bookmarks_enabled);
        assert_eq!(p.bookmarks_primary_click_behavior, "new");
        assert_eq!(p.ai_max_agent_steps, 40);
        assert!((p.ai_temperature - 0.3).abs() < f32::EPSILON);
        assert_eq!(p.default_model_id, "claude-x");
        assert_eq!(p.ollama_base_url, "http://host:1");
        assert_eq!(p.mcp_bridge_port, 50000);
        assert!(!p.status_bar_show_ai_controls);

        // Fields not present in the blob keep their (reference-correct) defaults.
        assert_eq!(p.editor_font_size, 13);
        assert_eq!(p.terminal_scrollback, 5_000);
        std::fs::remove_dir_all(&dir).ok();
    }
}
