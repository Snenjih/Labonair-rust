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

use serde::{Deserialize, Serialize};

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
    #[default]
    Terminal,
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
    pub notify_on_errors: bool,
    pub confirm_quit_with_ssh: bool,
    pub check_for_updates: bool,

    // ── Appearance ───────────────────────────────────────────────────────
    pub app_font_size: u32,
    pub app_line_height: f32,
    pub reduce_motion: bool,

    // ── Terminal ─────────────────────────────────────────────────────────
    pub terminal_shell: String,
    pub terminal_font_family: String,
    pub terminal_font_size: u32,
    pub terminal_scrollback: u32,
    pub terminal_cursor_style: CursorStyle,
    pub terminal_cursor_blink: bool,
    pub terminal_copy_on_select: bool,
    pub terminal_bell: bool,

    // ── Editor ───────────────────────────────────────────────────────────
    pub editor_font_family: String,
    pub editor_font_size: u32,
    pub editor_tab_size: u32,
    pub editor_word_wrap: bool,
    pub editor_line_numbers: bool,
    pub editor_indent_with_tabs: bool,
    pub editor_format_on_save: bool,

    // ── File Manager ─────────────────────────────────────────────────────
    pub sftp_show_hidden_files: bool,
    pub sftp_font_size: u32,
    pub sftp_max_concurrent_transfers: u32,

    // ── Command Palette ──────────────────────────────────────────────────
    pub command_palette_search_mode: PaletteSearchMode,
    pub command_palette_show_recent: bool,

    // ── Source Control ───────────────────────────────────────────────────
    pub git_status_poll_interval_ms: u32,

    // ── AI ───────────────────────────────────────────────────────────────
    pub ai_enabled: bool,
    pub ai_max_agent_steps: u32,
    pub ai_terminal_context_lines: u32,
    pub ai_warn_destructive_commands: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePref::System,
            restore_window_state: true,
            default_startup_tab: StartupTab::Terminal,
            notify_on_errors: true,
            confirm_quit_with_ssh: true,
            check_for_updates: true,

            app_font_size: 13,
            app_line_height: 1.5,
            reduce_motion: false,

            terminal_shell: String::new(),
            terminal_font_family: "JetBrains Mono".to_string(),
            terminal_font_size: 13,
            terminal_scrollback: 10_000,
            terminal_cursor_style: CursorStyle::Block,
            terminal_cursor_blink: true,
            terminal_copy_on_select: false,
            terminal_bell: false,

            editor_font_family: "JetBrains Mono".to_string(),
            editor_font_size: 13,
            editor_tab_size: 4,
            editor_word_wrap: false,
            editor_line_numbers: true,
            editor_indent_with_tabs: false,
            editor_format_on_save: false,

            sftp_show_hidden_files: false,
            sftp_font_size: 13,
            sftp_max_concurrent_transfers: 3,

            command_palette_search_mode: PaletteSearchMode::Contains,
            command_palette_show_recent: true,

            git_status_poll_interval_ms: 3000,

            ai_enabled: true,
            ai_max_agent_steps: 12,
            ai_terminal_context_lines: 200,
            ai_warn_destructive_commands: true,
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
    fn enums_serialize_to_reference_token_strings() {
        let json = serde_json::to_value(Preferences::default()).unwrap();
        assert_eq!(json["theme"], "system");
        assert_eq!(json["defaultStartupTab"], "terminal");
        assert_eq!(json["terminalCursorStyle"], "block");
        assert_eq!(json["commandPaletteSearchMode"], "contains");
    }
}
