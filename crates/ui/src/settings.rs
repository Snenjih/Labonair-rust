//! Settings window & preferences store (T13-001).
//!
//! Port of `reference-src/src/settings/*` + `reference-src/src/modules/settings/
//! preferences.ts`. The web app rendered a separate OS window; GPUI has no
//! child-window story wired here yet, so the settings surface is a modal
//! overlay over [`crate::app_shell::AppShell`] (same pattern the command
//! palette uses). It is opened by the `Settings…` menu item / `cmd-,` and the
//! `OpenSettings` command-palette entry.
//!
//! Two pieces live here:
//! * [`PreferencesStore`] — a GPUI entity holding the typed
//!   [`Preferences`] model, with generic key-addressed read/write that
//!   persists (`preferences_save`) and notifies on every change. Modules
//!   observe it and re-read their slice.
//! * [`SettingsView`] — the modal UI. A category sidebar + a field list
//!   rendered from the static [`FIELDS`] definitions (Switch / Int / Select /
//!   Text), plus a hand-built **AI Agent Bridge** pane (MCP) that the
//!   T11-006 work deferred here because no settings window existed yet.

use std::fs;
use std::path::{Path, PathBuf};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, Focusable, Global,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PathPromptOptions, Render,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use serde_json::Value;
use tokio::runtime::Handle as TokioHandle;

use labonair_backend::modules::fs::paths::config_dir;
use labonair_theme::ThemeFile;

use crate::background::BackgroundStore;

use labonair_backend::modules::mcp::{
    mcp_get_status, mcp_regenerate_token, mcp_set_auto_revoke_minutes, mcp_set_enabled,
    mcp_set_max_command_timeout_secs, mcp_set_port,
};
use labonair_backend::modules::settings::mcp::{mcp_prefs_load, mcp_prefs_save, McpPrefs};
use labonair_backend::modules::settings::preferences::{
    preferences_load, preferences_load_from, preferences_save, preferences_save_to, Preferences,
    ThemePref,
};
use labonair_backend::App as Backend;

use crate::notifications::{notification_center, Notification};
use crate::theme::{ThemePreference, ThemeStore};

// ─────────────────────────── Global snapshot ─────────────────────────────

/// App-wide read-only snapshot of [`Preferences`], republished by
/// [`PreferencesStore`] on every change. Modules that can't hold an
/// `Entity<PreferencesStore>` (the terminal engine spawn path, editor views)
/// read it via `cx.global::<GlobalPreferences>()` / `cx.observe_global`.
#[derive(Clone, Default)]
pub struct GlobalPreferences(pub Preferences);

impl Global for GlobalPreferences {}

// ─────────────────────────── Preferences store ───────────────────────────

/// GPUI entity wrapping the persisted [`Preferences`]. Generic key access
/// keeps the settings UI table-driven without a giant match: a mutation
/// serializes the model to a JSON object, swaps the one key, and validates by
/// deserializing back — a wrong-typed value is rejected, not stored.
pub struct PreferencesStore {
    prefs: Preferences,
    /// `None` = the shared per-user config file; `Some` = an explicit config
    /// directory (used by tests so they never touch the real settings file).
    dir: Option<std::path::PathBuf>,
}

impl Default for PreferencesStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesStore {
    pub fn new() -> Self {
        Self {
            prefs: preferences_load(),
            dir: None,
        }
    }

    /// Construct against an explicit config directory (testing).
    pub fn with_dir(dir: std::path::PathBuf) -> Self {
        Self {
            prefs: preferences_load_from(&dir),
            dir: Some(dir),
        }
    }

    fn persist(&self) -> Result<(), String> {
        match &self.dir {
            Some(dir) => preferences_save_to(dir, &self.prefs),
            None => preferences_save(&self.prefs),
        }
    }

    /// The full typed model — modules read their slice from here.
    pub fn get(&self) -> &Preferences {
        &self.prefs
    }

    /// (Re)publish the current model into the [`GlobalPreferences`] global.
    /// Call once at startup after construction; `set_value` keeps it current.
    pub fn publish_global(&self, cx: &mut App) {
        cx.set_global(GlobalPreferences(self.prefs.clone()));
    }

    /// The current JSON value for one camelCase key.
    pub fn value(&self, key: &str) -> Option<Value> {
        serde_json::to_value(&self.prefs).ok()?.get(key).cloned()
    }

    /// Set one key. Persists + notifies if the value parsed and changed.
    pub fn set_value(&mut self, key: &str, value: Value, cx: &mut Context<Self>) {
        let Ok(Value::Object(mut map)) = serde_json::to_value(&self.prefs) else {
            return;
        };
        map.insert(key.to_string(), value);
        match serde_json::from_value::<Preferences>(Value::Object(map)) {
            Ok(next) if next != self.prefs => {
                self.prefs = next;
                if let Err(e) = self.persist() {
                    tracing::warn!("failed to persist preferences: {e}");
                }
                cx.set_global(GlobalPreferences(self.prefs.clone()));
                cx.notify();
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("rejected preference `{key}`: {e}"),
        }
    }
}

// ─────────────────────────── Field definitions ───────────────────────────

/// UI control type for a preference field.
#[derive(Clone, Copy)]
pub enum FieldKind {
    Switch,
    Int {
        min: i64,
        max: i64,
        step: i64,
    },
    /// The options are the exact serialized token strings.
    Select(&'static [&'static str]),
    Text,
}

/// One rendered settings row.
pub struct FieldDef {
    pub key: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub category: &'static str,
    pub kind: FieldKind,
}

pub const AGENT_BRIDGE: &str = "AI Agent Bridge";

pub const CATEGORIES: &[&str] = &[
    "General",
    "Appearance",
    "Terminal",
    "Editor",
    "File Manager",
    "Command Palette",
    "Source Control",
    "AI",
    AGENT_BRIDGE,
];

use FieldKind::{Int, Select, Switch, Text};

pub const FIELDS: &[FieldDef] = &[
    // General
    d(
        "theme",
        "Theme",
        "System, light, or dark appearance.",
        "General",
        Select(&["system", "light", "dark"]),
    ),
    d(
        "restoreWindowState",
        "Restore window",
        "Restore window size & position on launch.",
        "General",
        Switch,
    ),
    d(
        "defaultStartupTab",
        "Startup tab",
        "Which tab opens on launch.",
        "General",
        Select(&["terminal", "host-manager"]),
    ),
    d(
        "notifyOnErrors",
        "Notify on errors",
        "Show a toast when a background task fails.",
        "General",
        Switch,
    ),
    d(
        "confirmQuitWithSsh",
        "Confirm quit with SSH",
        "Ask before quitting with active SSH sessions.",
        "General",
        Switch,
    ),
    d(
        "checkForUpdates",
        "Check for updates",
        "Check for new versions automatically.",
        "General",
        Switch,
    ),
    // Appearance
    d(
        "appFontFamily",
        "UI font family",
        "Font used for all application UI text (empty = system default).",
        "Appearance",
        Text,
    ),
    d(
        "appFontSize",
        "App font size",
        "Base UI font size in points.",
        "Appearance",
        Int {
            min: 9,
            max: 24,
            step: 1,
        },
    ),
    d(
        "reduceMotion",
        "Reduce motion",
        "Minimise animations and transitions.",
        "Appearance",
        Switch,
    ),
    // Terminal
    d(
        "terminalShell",
        "Shell",
        "Program to launch (empty = system default).",
        "Terminal",
        Text,
    ),
    d(
        "terminalFontFamily",
        "Font family",
        "Terminal typeface.",
        "Terminal",
        Text,
    ),
    d(
        "terminalFontSize",
        "Font size",
        "Terminal font size in points.",
        "Terminal",
        Int {
            min: 8,
            max: 32,
            step: 1,
        },
    ),
    d(
        "terminalScrollback",
        "Scrollback lines",
        "Lines of history kept per terminal.",
        "Terminal",
        Int {
            min: 1000,
            max: 200_000,
            step: 1000,
        },
    ),
    d(
        "terminalCursorStyle",
        "Cursor style",
        "Shape of the terminal cursor.",
        "Terminal",
        Select(&["block", "underline", "bar"]),
    ),
    d(
        "terminalCursorBlink",
        "Cursor blink",
        "Blink the terminal cursor.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalCopyOnSelect",
        "Copy on select",
        "Copy selected text to the clipboard automatically.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalBell",
        "Audible bell",
        "Play a sound on the terminal bell.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalOpacity",
        "Background opacity",
        "Terminal background opacity in percent (100 = opaque).",
        "Terminal",
        Int {
            min: 20,
            max: 100,
            step: 5,
        },
    ),
    // Editor
    d(
        "editorFontFamily",
        "Font family",
        "Editor typeface.",
        "Editor",
        Text,
    ),
    d(
        "editorFontSize",
        "Font size",
        "Editor font size in points.",
        "Editor",
        Int {
            min: 8,
            max: 32,
            step: 1,
        },
    ),
    d(
        "editorTabSize",
        "Tab size",
        "Spaces per indentation level.",
        "Editor",
        Int {
            min: 2,
            max: 8,
            step: 2,
        },
    ),
    d(
        "editorWordWrap",
        "Word wrap",
        "Wrap long lines to the viewport width.",
        "Editor",
        Switch,
    ),
    d(
        "editorLineNumbers",
        "Line numbers",
        "Show the line-number gutter.",
        "Editor",
        Switch,
    ),
    d(
        "editorRelativeLineNumbers",
        "Relative line numbers",
        "Number lines relative to the cursor.",
        "Editor",
        Switch,
    ),
    d(
        "editorVimMode",
        "Vim mode",
        "Enable the modal Vim keybinding layer.",
        "Editor",
        Switch,
    ),
    d(
        "editorTheme",
        "Syntax theme",
        "Editor colour scheme (auto follows the app theme).",
        "Editor",
        Select(&[
            "auto",
            "atomone",
            "aura",
            "copilot",
            "github-dark",
            "github-light",
            "nord",
            "tokyo-night",
            "xcode-dark",
            "xcode-light",
        ]),
    ),
    d(
        "editorIndentWithTabs",
        "Indent with tabs",
        "Use tab characters instead of spaces.",
        "Editor",
        Switch,
    ),
    d(
        "editorFormatOnSave",
        "Format on save",
        "Run the formatter when saving.",
        "Editor",
        Switch,
    ),
    // File Manager
    d(
        "sftpShowHiddenFiles",
        "Show hidden files",
        "Show dotfiles in the file browser by default.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpFontSize",
        "Font size",
        "File-browser font size in points.",
        "File Manager",
        Int {
            min: 8,
            max: 32,
            step: 1,
        },
    ),
    d(
        "sftpMaxConcurrentTransfers",
        "Max concurrent transfers",
        "Parallel SFTP transfers.",
        "File Manager",
        Int {
            min: 1,
            max: 16,
            step: 1,
        },
    ),
    // Command Palette
    d(
        "commandPaletteSearchMode",
        "Search mode",
        "How the palette matches your query.",
        "Command Palette",
        Select(&["contains", "startsWith", "fuzzy"]),
    ),
    d(
        "commandPaletteShowRecent",
        "Show recent",
        "Surface recently-run commands first.",
        "Command Palette",
        Switch,
    ),
    // Source Control
    d(
        "gitStatusPollIntervalMs",
        "Status poll interval",
        "How often to refresh git status (ms).",
        "Source Control",
        Int {
            min: 500,
            max: 30_000,
            step: 500,
        },
    ),
    // AI
    d(
        "aiEnabled",
        "Enable AI features",
        "Master switch for the AI chat and agent tools.",
        "AI",
        Switch,
    ),
    d(
        "aiMaxAgentSteps",
        "Max agent steps",
        "Upper bound on tool-use iterations per run.",
        "AI",
        Int {
            min: 1,
            max: 50,
            step: 1,
        },
    ),
    d(
        "aiTerminalContextLines",
        "Terminal context lines",
        "Lines of terminal scrollback fed to the model.",
        "AI",
        Int {
            min: 0,
            max: 2000,
            step: 50,
        },
    ),
    d(
        "aiWarnDestructiveCommands",
        "Warn on destructive commands",
        "Flag risky shell commands before running.",
        "AI",
        Switch,
    ),
];

const fn d(
    key: &'static str,
    title: &'static str,
    desc: &'static str,
    category: &'static str,
    kind: FieldKind,
) -> FieldDef {
    FieldDef {
        key,
        title,
        desc,
        category,
        kind,
    }
}

// ─────────────────────────────── Settings view ───────────────────────────

struct EditState {
    key: String,
    buffer: String,
    numeric: bool,
}

/// One row in the Appearance theme list (built-in default + user themes).
struct ThemeEntry {
    /// Filename stem — `"default"` for the built-in.
    id: String,
    /// Display name from the theme file.
    name: String,
    /// Built-in themes can be activated/exported but never deleted.
    builtin: bool,
}

pub struct SettingsView {
    prefs: Entity<PreferencesStore>,
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    backend: Backend,
    tokio: TokioHandle,
    open: bool,
    active_cat: usize,
    search: String,
    editing: Option<EditState>,
    mcp: McpPrefs,
    mcp_token: Option<String>,
    /// Available themes for the Appearance pane, refreshed when the modal opens.
    theme_files: Vec<ThemeEntry>,
    /// Which listed theme is active (`None` = built-in light/dark, no override).
    active_theme_id: Option<String>,
    focus: FocusHandle,
}

impl SettingsView {
    pub fn new(
        prefs: Entity<PreferencesStore>,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        backend: Backend,
        tokio: TokioHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&prefs, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();
        Self {
            prefs,
            theme,
            background,
            backend,
            tokio,
            open: false,
            active_cat: 0,
            search: String::new(),
            editing: None,
            mcp: mcp_prefs_load(),
            mcp_token: None,
            theme_files: Vec::new(),
            active_theme_id: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.editing = None;
        self.search.clear();
        window.focus(&self.focus);
        self.refresh_mcp_status(cx);
        self.refresh_themes();
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.editing = None;
        cx.notify();
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.open(window, cx);
        }
    }

    fn refresh_mcp_status(&self, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let task = self
            .tokio
            .spawn(async move { mcp_get_status(app.clone(), &app.mcp, &app.secrets).await });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(status)) = task.await {
                let _ = this.update(cx, |this, cx| {
                    this.mcp_token = status.token;
                    this.mcp.bridge_port = status.port;
                    this.mcp.max_command_timeout_secs = status.max_command_timeout_secs;
                    this.mcp.auto_revoke_minutes = status.auto_revoke_minutes;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    // ── appearance: themes ────────────────────────────────────────────────

    /// Rescans the user themes directory (`config_dir()/themes/*.json`) and
    /// rebuilds [`Self::theme_files`]. The built-in "Labonair" default is
    /// always first.
    fn refresh_themes(&mut self) {
        self.theme_files = scan_themes(&themes_dir());
    }

    /// Activates a listed theme. `"default"` clears any custom override and
    /// reverts to the built-in light/dark themes.
    fn activate_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        if id == "default" {
            self.theme.update(cx, |t, cx| t.clear_custom_theme(cx));
            self.active_theme_id = None;
            cx.notify();
            return;
        }
        let file = match read_theme_file_in(&themes_dir(), id) {
            Ok(f) => f,
            Err(e) => {
                self.notify_error(cx, "Failed to load theme", e);
                return;
            }
        };
        let result = self.theme.update(cx, |t, cx| t.import_theme_file(file, cx));
        match result {
            Ok(warnings) => {
                self.active_theme_id = Some(id.to_string());
                if !warnings.is_empty() {
                    self.notify(
                        cx,
                        Notification::warning("Theme applied with warnings", warnings.join("; ")),
                    );
                }
            }
            Err(e) => self.notify_error(cx, "Invalid theme", e),
        }
        cx.notify();
    }

    /// Opens the file picker, copies the chosen `.json` into the themes dir and
    /// activates it (T02-003 wiring).
    fn import_theme(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import theme".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            let Some(src) = paths.into_iter().next() else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_theme_from(src, cx));
        })
        .detach();
    }

    fn import_theme_from(&mut self, src: PathBuf, cx: &mut Context<Self>) {
        let raw = match fs::read_to_string(&src) {
            Ok(r) => r,
            Err(e) => return self.notify_error(cx, "Failed to read theme", e.to_string()),
        };
        let file = match ThemeFile::from_json(&raw).and_then(|f| f.validate().map(|_| f)) {
            Ok(f) => f,
            Err(e) => return self.notify_error(cx, "Invalid theme file", e),
        };
        let id = src
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("imported-theme")
            .to_string();
        if let Err(e) = save_theme_file_in(&themes_dir(), &id, &raw) {
            return self.notify_error(cx, "Failed to save theme", e);
        }
        match self.theme.update(cx, |t, cx| t.import_theme_file(file, cx)) {
            Ok(_) => {
                self.active_theme_id = Some(id);
                self.notify(
                    cx,
                    Notification::success("Theme imported", "The theme is now active."),
                );
            }
            Err(e) => self.notify_error(cx, "Invalid theme", e),
        }
        self.refresh_themes();
        cx.notify();
    }

    /// Exports the currently active theme to a user-chosen `.json` file.
    fn export_theme(&mut self, cx: &mut Context<Self>) {
        let name = self
            .active_theme_id
            .as_deref()
            .and_then(|id| self.theme_files.iter().find(|t| t.id == id))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Labonair".to_string());
        let json = match self
            .theme
            .read(cx)
            .active_theme_file(name.clone())
            .to_json()
        {
            Ok(j) => j,
            Err(e) => return self.notify_error(cx, "Export failed", e),
        };
        let slug = slugify(&name);
        let receiver = cx.prompt_for_new_path(&config_dir(), Some(&format!("{slug}.json")));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |this, cx| match fs::write(&dest, &json) {
                Ok(()) => this.notify(
                    cx,
                    Notification::success("Theme exported", dest.to_string_lossy().to_string()),
                ),
                Err(e) => this.notify_error(cx, "Export failed", e.to_string()),
            });
        })
        .detach();
    }

    /// Deletes a user theme file. Built-in themes are protected.
    fn delete_theme(&mut self, id: &str, cx: &mut Context<Self>) {
        if id == "default" {
            return;
        }
        if let Err(e) = delete_theme_in(&themes_dir(), id) {
            self.notify_error(cx, "Failed to delete theme", e);
            return;
        }
        if self.active_theme_id.as_deref() == Some(id) {
            self.theme.update(cx, |t, cx| t.clear_custom_theme(cx));
            self.active_theme_id = None;
        }
        self.refresh_themes();
        cx.notify();
    }

    fn notify(&self, cx: &mut Context<Self>, n: Notification) {
        notification_center(cx).update(cx, |c, cx| {
            c.push(n, cx);
        });
    }

    fn notify_error(&self, cx: &mut Context<Self>, title: &'static str, body: String) {
        self.notify(cx, Notification::error(title, body));
    }

    // ── generic field mutation ────────────────────────────────────────────

    fn set_pref(&mut self, key: &str, value: Value, cx: &mut Context<Self>) {
        let key_owned = key.to_string();
        self.prefs
            .update(cx, |p, cx| p.set_value(&key_owned, value, cx));
        // Propagate the values modules can't observe generically.
        if key == "theme" {
            let pref = match self.prefs.read(cx).get().theme {
                ThemePref::System => ThemePreference::System,
                ThemePref::Light => ThemePreference::Light,
                ThemePref::Dark => ThemePreference::Dark,
            };
            self.theme.update(cx, |t, cx| t.set_preference(pref, cx));
        }
        // Typography + editor syntax scheme are pushed into the ThemeStore so
        // open terminals / editors pick them up live (T13-003).
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    /// Mirror the font / editor-theme preferences into the [`ThemeStore`].
    fn sync_theme_from_prefs(&mut self, cx: &mut Context<Self>) {
        let p = self.prefs.read(cx).get().clone();
        let theme = self.theme.clone();
        apply_prefs_to_theme(&p, &theme, cx);
    }

    fn toggle_bool(&mut self, key: &str, cx: &mut Context<Self>) {
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.set_pref(key, Value::Bool(!cur), cx);
    }

    fn bump_int(&mut self, key: &str, min: i64, max: i64, delta: i64, cx: &mut Context<Self>) {
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(min);
        let next = (cur + delta).clamp(min, max);
        self.set_pref(key, Value::from(next), cx);
    }

    fn cycle_select(&mut self, key: &str, options: &[&str], cx: &mut Context<Self>) {
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let idx = options.iter().position(|o| *o == cur).unwrap_or(0);
        let next = options[(idx + 1) % options.len()];
        self.set_pref(key, Value::String(next.to_string()), cx);
    }

    fn begin_edit(&mut self, key: &str, numeric: bool, cx: &mut Context<Self>) {
        let buffer = self
            .prefs
            .read(cx)
            .value(key)
            .map(|v| match v {
                Value::String(s) => s,
                other => other.to_string(),
            })
            .unwrap_or_default();
        self.editing = Some(EditState {
            key: key.to_string(),
            buffer,
            numeric,
        });
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.editing.take() else {
            return;
        };
        let value = if edit.numeric {
            match edit.buffer.trim().parse::<i64>() {
                Ok(n) => Value::from(n),
                Err(_) => {
                    cx.notify();
                    return;
                }
            }
        } else {
            Value::String(edit.buffer.trim().to_string())
        };
        self.set_pref(&edit.key, value, cx);
    }

    // ── key handling ──────────────────────────────────────────────────────

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let key = ks.key.as_str();
        if self.editing.is_some() {
            match key {
                "escape" => {
                    self.editing = None;
                    cx.notify();
                }
                "enter" => self.commit_edit(cx),
                "backspace" => {
                    if let Some(e) = self.editing.as_mut() {
                        e.buffer.pop();
                    }
                    cx.notify();
                }
                _ => {
                    if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                        return;
                    }
                    if let Some(ch) = char_of(ks) {
                        if let Some(e) = self.editing.as_mut() {
                            e.buffer.push_str(&ch);
                        }
                        cx.notify();
                    }
                }
            }
            cx.stop_propagation();
            return;
        }

        match key {
            "escape" => self.close(cx),
            "backspace" => {
                self.search.pop();
                cx.notify();
            }
            _ => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = char_of(ks) {
                    self.search.push_str(&ch);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    // ── rendering ─────────────────────────────────────────────────────────

    fn render_field(
        &self,
        def: &FieldDef,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let key = def.key;
        let control = match def.kind {
            FieldKind::Switch => {
                let on = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                div()
                    .id(SharedString::from(format!("sw-{key}")))
                    .w(px(38.0))
                    .h(px(20.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .px(px(2.0))
                    .bg(if on { c.accent } else { c.border })
                    .child(
                        div()
                            .size(px(16.0))
                            .rounded_full()
                            .bg(c.bg)
                            .when(on, |d| d.ml(px(16.0))),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.toggle_bool(key, cx);
                    }))
                    .into_any_element()
            }
            FieldKind::Int { min, max, step } => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_i64())
                    .unwrap_or(min);
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(step_btn("dec", key, "\u{2212}", c, cx, move |this, cx| {
                        this.bump_int(key, min, max, -step, cx)
                    }))
                    .child(
                        div()
                            .min_w(px(52.0))
                            .text_center()
                            .text_color(c.fg)
                            .child(SharedString::from(cur.to_string())),
                    )
                    .child(step_btn("inc", key, "+", c, cx, move |this, cx| {
                        this.bump_int(key, min, max, step, cx)
                    }))
                    .into_any_element()
            }
            FieldKind::Select(options) => {
                let cur = self
                    .prefs
                    .read(cx)
                    .value(key)
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_default();
                div()
                    .id(SharedString::from(format!("sel-{key}")))
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .text_color(c.fg)
                    .child(SharedString::from(format!("{cur}  \u{25BE}")))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.cycle_select(key, options, cx);
                    }))
                    .into_any_element()
            }
            FieldKind::Text => {
                let editing = self
                    .editing
                    .as_ref()
                    .filter(|e| e.key == key)
                    .map(|e| e.buffer.clone());
                let value = editing.clone().unwrap_or_else(|| {
                    self.prefs
                        .read(cx)
                        .value(key)
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default()
                });
                let active = editing.is_some();
                let empty = value.is_empty();
                div()
                    .id(SharedString::from(format!("txt-{key}")))
                    .w(px(200.0))
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(if active { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(if empty { c.muted } else { c.fg })
                    .child(SharedString::from(if empty {
                        "(default)".to_string()
                    } else {
                        value
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.begin_edit(key, false, cx);
                    }))
                    .into_any_element()
            }
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_2()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_color(c.fg).child(SharedString::from(def.title)))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child(SharedString::from(def.desc)),
                    ),
            )
            .child(control)
            .into_any_element()
    }

    fn render_agent_bridge(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let m = self.mcp;
        let setup = if m.bridge_enabled {
            self.mcp_token.as_ref().map(|tok| {
                format!(
                    "claude mcp add --transport http labonair http://127.0.0.1:{}/mcp --header \"Authorization: Bearer {}\" --scope user",
                    m.bridge_port, tok
                )
            })
        } else {
            None
        };

        let mut col = div().flex().flex_col();

        col = col.child(bridge_switch_row(
            "Enable AI Agent Bridge",
            "Let an external agent CLI drive granted SSH / local tabs over MCP.",
            m.bridge_enabled,
            c,
            cx,
            |this, cx| {
                let next = !this.mcp.bridge_enabled;
                this.mcp.bridge_enabled = next;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                this.tokio.spawn(async move {
                    let _ = mcp_set_enabled(next, app.clone(), &app.mcp, &app.secrets).await;
                });
                this.refresh_mcp_status(cx);
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Port",
            m.bridge_port as i64,
            1024,
            65535,
            1,
            c,
            cx,
            |this, v, cx| {
                this.mcp.bridge_port = v as u16;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let port = this.mcp.bridge_port;
                this.tokio.spawn(async move {
                    let _ = mcp_set_port(port, app.clone(), &app.mcp, &app.secrets).await;
                });
                this.refresh_mcp_status(cx);
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Max command timeout (s)",
            m.max_command_timeout_secs as i64,
            5,
            3600,
            5,
            c,
            cx,
            |this, v, cx| {
                this.mcp.max_command_timeout_secs = v as u64;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let secs = this.mcp.max_command_timeout_secs;
                this.tokio.spawn(async move {
                    let _ = mcp_set_max_command_timeout_secs(secs, &app.mcp).await;
                });
                cx.notify();
            },
        ));

        col = col.child(bridge_int_row(
            "Auto-revoke after (min, 0 = off)",
            m.auto_revoke_minutes as i64,
            0,
            1440,
            5,
            c,
            cx,
            |this, v, cx| {
                this.mcp.auto_revoke_minutes = v as u32;
                let _ = mcp_prefs_save(&this.mcp);
                let app = this.backend.clone();
                let mins = this.mcp.auto_revoke_minutes;
                this.tokio.spawn(async move {
                    let _ = mcp_set_auto_revoke_minutes(mins, &app.mcp).await;
                });
                cx.notify();
            },
        ));

        col = col.child(bridge_switch_row(
            "Notify on agent activity",
            "Show a toast for every command / keystroke an agent sends.",
            m.notify_on_activity,
            c,
            cx,
            |this, cx| {
                this.mcp.notify_on_activity = !this.mcp.notify_on_activity;
                let _ = mcp_prefs_save(&this.mcp);
                cx.notify();
            },
        ));

        col = col.child(
            div().flex().items_center().gap_2().py_2().child(
                div()
                    .id("mcp-regen")
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .text_color(c.fg)
                    .child("Regenerate token")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        let app = this.backend.clone();
                        let task = this.tokio.spawn(async move {
                            mcp_regenerate_token(app.clone(), &app.mcp, &app.secrets).await
                        });
                        cx.spawn(async move |this, cx| {
                            if let Ok(Ok(status)) = task.await {
                                let _ = this.update(cx, |this, cx| {
                                    this.mcp_token = status.token;
                                    cx.notify();
                                });
                            }
                        })
                        .detach();
                    })),
            ),
        );

        if let Some(cmd) = setup {
            let copy = cmd.clone();
            col = col.child(
                div()
                    .mt_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child("claude mcp add \u{2026}"),
                    )
                    .child(
                        div()
                            .p_2()
                            .rounded_sm()
                            .bg(c.bg)
                            .border_1()
                            .border_color(c.border)
                            .text_size(px(11.0))
                            .text_color(c.fg)
                            .child(SharedString::from(cmd)),
                    )
                    .child(
                        div()
                            .id("mcp-copy")
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.fg)
                            .child("Copy")
                            .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                                notification_center(cx).update(cx, |n, cx| {
                                    n.push(
                                        Notification::info("Copied", "Setup command copied."),
                                        cx,
                                    );
                                });
                            })),
                    ),
            );
        }

        col.into_any_element()
    }

    fn render_appearance(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let cur_pref = self.prefs.read(cx).get().theme;
        let bg = self.background.read(cx).settings().clone();
        let available = self.background.read(cx).available();
        let has_image = !bg.background_image.is_empty();

        // Color scheme selector.
        let scheme = div().flex().gap_2().py_2().children(
            [
                (ThemePref::System, "System", "system"),
                (ThemePref::Light, "Light", "light"),
                (ThemePref::Dark, "Dark", "dark"),
            ]
            .into_iter()
            .map(|(pref, label, token)| {
                let active = cur_pref == pref;
                div()
                    .id(SharedString::from(format!("scheme-{token}")))
                    .flex_1()
                    .h(px(56.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .border_1()
                    .border_color(if active { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(if active { c.fg } else { c.muted })
                    .child(SharedString::from(label))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.set_pref("theme", Value::String(token.to_string()), cx);
                    }))
            }),
        );

        // Theme list.
        let active_id = self.active_theme_id.clone();
        let theme_rows: Vec<_> = self
            .theme_files
            .iter()
            .map(|t| {
                let id = t.id.clone();
                let id_del = t.id.clone();
                let is_active = active_id.as_deref() == Some(t.id.as_str())
                    || (active_id.is_none() && t.id == "default");
                let builtin = t.builtin;
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .py_1p5()
                    .border_b_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().size(px(10.0)).rounded_full().bg(if is_active {
                                c.accent
                            } else {
                                c.border
                            }))
                            .child(
                                div()
                                    .text_color(c.fg)
                                    .child(SharedString::from(t.name.clone())),
                            )
                            .when(builtin, |d| {
                                d.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(c.muted)
                                        .child("built-in"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .id(SharedString::from(format!("theme-use-{}", t.id)))
                                    .px_2()
                                    .py(px(2.0))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(c.border)
                                    .text_color(c.fg)
                                    .hover(|s| s.bg(c.border))
                                    .child(if is_active { "Active" } else { "Activate" })
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        this.activate_theme(&id, cx);
                                    })),
                            )
                            .when(!builtin, |d| {
                                d.child(
                                    div()
                                        .id(SharedString::from(format!("theme-del-{}", id_del)))
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(c.border)
                                        .text_color(c.muted)
                                        .hover(|s| s.text_color(c.fg))
                                        .child("Delete")
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                this.delete_theme(&id_del, cx);
                                            },
                                        )),
                                )
                            }),
                    )
            })
            .collect();

        let theme_actions = div()
            .flex()
            .gap_2()
            .py_2()
            .child(
                div()
                    .id("theme-import")
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .text_color(c.fg)
                    .hover(|s| s.bg(c.border))
                    .child("Import theme\u{2026}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.import_theme(cx))),
            )
            .child(
                div()
                    .id("theme-export")
                    .px_2()
                    .py(px(3.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .text_color(c.fg)
                    .hover(|s| s.bg(c.border))
                    .child("Export active theme\u{2026}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.export_theme(cx))),
            );

        // Background image tiles.
        let mut tiles = div().flex().flex_wrap().gap_2().py_2();
        tiles = tiles.child(bg_tile("none", "None", !has_image, c, cx, |this, cx| {
            this.background.update(cx, |b, cx| b.set_image("", cx));
        }));
        for info in &available {
            let name = info.filename.clone();
            let sel = bg.background_image == info.filename;
            let name_del = info.filename.clone();
            tiles = tiles.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(bg_tile(
                        &format!("bg-{}", info.filename),
                        &trim_ext(&info.filename),
                        sel,
                        c,
                        cx,
                        move |this, cx| {
                            let n = name.clone();
                            this.background.update(cx, |b, cx| b.set_image(n, cx));
                        },
                    ))
                    .child(
                        div()
                            .id(SharedString::from(format!("bg-del-{}", info.filename)))
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child("\u{2715}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                let n = name_del.clone();
                                let _ = this.background.update(cx, |b, cx| b.delete(&n, cx));
                            })),
                    ),
            );
        }
        tiles = tiles.child(
            div()
                .id("bg-add")
                .w(px(96.0))
                .h(px(60.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .border_dashed()
                .border_color(c.border)
                .text_color(c.muted)
                .hover(|s| s.text_color(c.fg))
                .child("+ Add")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.background.update(cx, |b, cx| b.prompt_and_import(cx));
                })),
        );

        let mut root = div()
            .flex()
            .flex_col()
            .child(section_label("Color scheme", c))
            .child(scheme)
            .child(section_label("Themes", c))
            .child(div().flex().flex_col().children(theme_rows))
            .child(theme_actions)
            .child(section_label("Background image", c))
            .child(tiles);

        if has_image {
            root = root
                .child(bridge_int_row(
                    "Wallpaper opacity (%)",
                    bg.background_opacity as i64,
                    0,
                    100,
                    5,
                    c,
                    cx,
                    |this, v, cx| {
                        this.background
                            .update(cx, |b, cx| b.set_opacity(v as u8, cx));
                    },
                ))
                .child(bridge_int_row(
                    "Image blur (px)",
                    bg.background_blur as i64,
                    0,
                    20,
                    1,
                    c,
                    cx,
                    |this, v, cx| {
                        this.background.update(cx, |b, cx| b.set_blur(v as u8, cx));
                    },
                ))
                .child(bridge_int_row(
                    "Color tint (%)",
                    bg.background_tint_opacity as i64,
                    0,
                    100,
                    5,
                    c,
                    cx,
                    |this, v, cx| {
                        this.background
                            .update(cx, |b, cx| b.set_tint_opacity(v as u8, cx));
                    },
                ));
        }

        root = root.child(section_label("Typography", c));
        for key in ["appFontFamily", "appFontSize", "reduceMotion"] {
            if let Some(def) = FIELDS.iter().find(|f| f.key == key) {
                root = root.child(self.render_field(def, c, cx));
            }
        }

        root.into_any_element()
    }

    fn render_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.search.trim().to_lowercase();
        if !query.is_empty() {
            let matches: Vec<&FieldDef> = FIELDS
                .iter()
                .filter(|f| {
                    f.title.to_lowercase().contains(&query)
                        || f.category.to_lowercase().contains(&query)
                })
                .collect();
            if matches.is_empty() {
                return div()
                    .p_4()
                    .text_color(c.muted)
                    .child("No matching settings.")
                    .into_any_element();
            }
            return div()
                .flex()
                .flex_col()
                .children(matches.into_iter().map(|f| self.render_field(f, c, cx)))
                .into_any_element();
        }

        let cat = CATEGORIES[self.active_cat];
        if cat == AGENT_BRIDGE {
            return self.render_agent_bridge(c, cx);
        }
        if cat == "Appearance" {
            return self.render_appearance(c, cx);
        }
        div()
            .flex()
            .flex_col()
            .children(
                FIELDS
                    .iter()
                    .filter(|f| f.category == cat)
                    .map(|f| self.render_field(f, c, cx)),
            )
            .into_any_element()
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }
        let t = self.theme.read(cx);
        let c = Palette {
            bg: t.background(),
            fg: t.foreground(),
            muted: t.muted_foreground(),
            border: t.border(),
            card: t.card(),
            accent: t.accent(),
        };
        let active_cat = self.active_cat;
        let searching = !self.search.trim().is_empty();

        let sidebar = div()
            .w(px(180.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(c.border)
            .children(CATEGORIES.iter().enumerate().map(|(i, name)| {
                let is_active = i == active_cat && !searching;
                div()
                    .id(SharedString::from(*name))
                    .px_2()
                    .py(px(4.0))
                    .rounded_sm()
                    .text_color(if is_active { c.fg } else { c.muted })
                    .when(is_active, |d| d.bg(c.accent))
                    .when(!is_active, |d| d.hover(|s| s.bg(c.border)))
                    .child(SharedString::from(*name))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.active_cat = i;
                        this.search.clear();
                        cx.notify();
                    }))
            }));

        let search_box = div()
            .mb_2()
            .px_2()
            .py(px(4.0))
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .text_color(if self.search.is_empty() {
                c.muted
            } else {
                c.fg
            })
            .child(SharedString::from(if self.search.is_empty() {
                "Search settings\u{2026}".to_string()
            } else {
                self.search.clone()
            }));

        let body = self.render_body(&c, cx);

        div()
            .id("settings-overlay")
            .track_focus(&self.focus)
            .key_context("Settings")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::hsla(0.0, 0.0, 0.0, 0.5))
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.close(cx)))
            .child(
                div()
                    .id("settings-card")
                    .w(px(760.0))
                    .h(px(540.0))
                    .flex()
                    .flex_col()
                    .rounded_lg()
                    .bg(c.card)
                    .border_1()
                    .border_color(c.border)
                    .overflow_hidden()
                    .on_click(|_, _w, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .py_2()
                            .border_b_1()
                            .border_color(c.border)
                            .child(
                                div()
                                    .text_color(c.fg)
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Settings"),
                            )
                            .child(
                                div()
                                    .id("settings-close")
                                    .text_color(c.muted)
                                    .hover(|s| s.text_color(c.fg))
                                    .child("\u{2715}")
                                    .on_click(
                                        cx.listener(|this, _: &ClickEvent, _w, cx| this.close(cx)),
                                    ),
                            ),
                    )
                    .child(
                        div().flex_1().min_h_0().flex().child(sidebar).child(
                            div()
                                .id("settings-scroll")
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .p_4()
                                .overflow_y_scroll()
                                .child(search_box)
                                .child(body),
                        ),
                    ),
            )
            .into_any_element()
    }
}

// ─────────────────────────────── helpers ─────────────────────────────────

#[derive(Clone, Copy)]
struct Palette {
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    accent: gpui::Hsla,
}

/// Build the [`FontOverrides`] snapshot from the typography-relevant
/// preferences. A blank family / zero size means "keep the theme default".
pub(crate) fn font_overrides_from(p: &Preferences) -> crate::theme::FontOverrides {
    crate::theme::FontOverrides {
        app_family: p.app_font_family.clone(),
        app_size: p.app_font_size as f32,
        editor_family: p.editor_font_family.clone(),
        editor_size: p.editor_font_size as f32,
        terminal_family: p.terminal_font_family.clone(),
        terminal_size: p.terminal_font_size as f32,
        terminal_line_height: 0.0,
    }
}

/// Push the font + editor-syntax-theme preferences into the [`ThemeStore`].
/// Used at startup (`AppShell`) and on every settings change.
pub(crate) fn apply_prefs_to_theme(p: &Preferences, theme: &Entity<ThemeStore>, cx: &mut App) {
    let overrides = font_overrides_from(p);
    theme.update(cx, |t, cx| t.set_font_overrides(overrides, cx));
    if let Some(id) = crate::theme::EditorThemeId::from_slug(&p.editor_theme) {
        theme.update(cx, |t, cx| t.set_editor_theme(id, cx));
    }
}

fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

/// Scans `dir` for valid user theme files. The built-in "Labonair" default is
/// always the first entry; user themes follow, sorted by display name.
fn scan_themes(dir: &Path) -> Vec<ThemeEntry> {
    let mut entries = vec![ThemeEntry {
        id: "default".to_string(),
        name: "Labonair".to_string(),
        builtin: true,
    }];
    if let Ok(rd) = fs::read_dir(dir) {
        let mut users: Vec<ThemeEntry> = rd
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|e| {
                let path = e.path();
                let id = path.file_stem()?.to_str()?.to_string();
                if id == "default" {
                    return None;
                }
                let file = ThemeFile::from_json(&fs::read_to_string(&path).ok()?).ok()?;
                file.validate().ok()?;
                Some(ThemeEntry {
                    id,
                    name: file.name,
                    builtin: false,
                })
            })
            .collect();
        users.sort_by_key(|a| a.name.to_lowercase());
        entries.extend(users);
    }
    entries
}

fn read_theme_file_in(dir: &Path, id: &str) -> Result<ThemeFile, String> {
    let raw = fs::read_to_string(dir.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
    ThemeFile::from_json(&raw)
}

fn save_theme_file_in(dir: &Path, id: &str, raw: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{id}.json")), raw).map_err(|e| e.to_string())
}

fn delete_theme_in(dir: &Path, id: &str) -> Result<(), String> {
    if id == "default" {
        return Err("the built-in theme cannot be deleted".to_string());
    }
    fs::remove_file(dir.join(format!("{id}.json"))).map_err(|e| e.to_string())
}

fn slugify(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() {
        "theme".to_string()
    } else {
        s
    }
}

fn trim_ext(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(s, _)| s.to_string())
        .unwrap_or_else(|| name.to_string())
}

fn section_label(text: &'static str, c: &Palette) -> impl IntoElement {
    div()
        .pt_3()
        .pb_1()
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(c.muted)
        .child(text)
}

fn bg_tile(
    id: &str,
    label: &str,
    selected: bool,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.to_string()))
        .w(px(96.0))
        .h(px(60.0))
        .flex()
        .items_end()
        .p_1()
        .rounded_md()
        .border_1()
        .border_color(if selected { c.accent } else { c.border })
        .bg(c.bg)
        .text_color(if selected { c.fg } else { c.muted })
        .text_size(px(10.0))
        .overflow_hidden()
        .child(SharedString::from(label.to_string()))
        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx)))
}

fn char_of(ks: &gpui::Keystroke) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| {
            (ks.key.chars().count() == 1 && !ks.key.chars().any(|c| c.is_control()))
                .then(|| ks.key.clone())
        })
}

fn step_btn(
    tag: &'static str,
    key: &'static str,
    glyph: &'static str,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("{tag}-{key}")))
        .size(px(20.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(c.border)
        .text_color(c.fg)
        .hover(|s| s.bg(c.border))
        .child(glyph)
        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx)))
}

fn bridge_switch_row(
    title: &'static str,
    desc: &'static str,
    on: bool,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_2()
        .border_b_1()
        .border_color(c.border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .flex_1()
                .min_w_0()
                .child(div().text_color(c.fg).child(title))
                .child(div().text_size(px(11.0)).text_color(c.muted).child(desc)),
        )
        .child(
            div()
                .id(SharedString::from(format!("mcp-sw-{title}")))
                .w(px(38.0))
                .h(px(20.0))
                .rounded_full()
                .flex()
                .items_center()
                .px(px(2.0))
                .bg(if on { c.accent } else { c.border })
                .child(
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(c.bg)
                        .when(on, |d| d.ml(px(16.0))),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx))),
        )
}

#[allow(clippy::too_many_arguments)]
fn bridge_int_row(
    title: &'static str,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, i64, &mut Context<SettingsView>) + Clone + 'static,
) -> impl IntoElement {
    let f_dec = f.clone();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_2()
        .border_b_1()
        .border_color(c.border)
        .child(div().text_color(c.fg).flex_1().min_w_0().child(title))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("mcp-dec-{title}")))
                        .size(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(c.border)
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.border))
                        .child("\u{2212}")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            f_dec(this, (value - step).clamp(min, max), cx)
                        })),
                )
                .child(
                    div()
                        .min_w(px(52.0))
                        .text_center()
                        .text_color(c.fg)
                        .child(SharedString::from(value.to_string())),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("mcp-inc-{title}")))
                        .size(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(c.border)
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.border))
                        .child("+")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            f(this, (value + step).clamp(min, max), cx)
                        })),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn every_field_key_exists_on_the_model() {
        let json = serde_json::to_value(Preferences::default()).unwrap();
        let obj = json.as_object().unwrap();
        for f in FIELDS {
            assert!(
                obj.contains_key(f.key),
                "unknown preference key `{}`",
                f.key
            );
        }
    }

    #[test]
    fn every_field_category_is_registered() {
        for f in FIELDS {
            assert!(
                CATEGORIES.contains(&f.category),
                "field `{}` has unregistered category `{}`",
                f.key,
                f.category
            );
        }
    }

    #[test]
    fn select_options_are_valid_serialized_tokens() {
        for f in FIELDS {
            if let FieldKind::Select(opts) = f.kind {
                let mut base = serde_json::to_value(Preferences::default())
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .clone();
                for opt in opts {
                    base.insert(f.key.to_string(), Value::String((*opt).to_string()));
                    let parsed: Result<Preferences, _> =
                        serde_json::from_value(Value::Object(base.clone()));
                    assert!(parsed.is_ok(), "`{}` rejects option `{}`", f.key, opt);
                }
            }
        }
    }

    #[test]
    fn editor_theme_options_are_known_slugs() {
        let opts = FIELDS
            .iter()
            .find(|f| f.key == "editorTheme")
            .map(|f| match f.kind {
                FieldKind::Select(o) => o,
                _ => panic!("editorTheme should be a Select"),
            })
            .unwrap();
        for slug in opts {
            assert!(
                crate::theme::EditorThemeId::from_slug(slug).is_some(),
                "unknown editor theme slug `{slug}`"
            );
        }
    }

    #[test]
    fn font_overrides_snapshot_maps_prefs() {
        let p = Preferences {
            terminal_font_size: 18,
            editor_font_family: "Iosevka".to_string(),
            ..Default::default()
        };
        let o = font_overrides_from(&p);
        assert_eq!(o.terminal_size, 18.0);
        assert_eq!(o.editor_family, "Iosevka");
    }

    #[gpui::test]
    fn set_value_persists_and_notifies(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let d2 = dir.clone();
        let store = cx.new(|_| PreferencesStore::with_dir(d2));
        let count = std::rc::Rc::new(std::cell::RefCell::new(0));
        let c2 = count.clone();
        cx.update(|cx| {
            cx.observe(&store, move |_, _| *c2.borrow_mut() += 1)
                .detach();
        });
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::from(19), cx);
        });
        cx.run_until_parked();
        assert_eq!(store.read_with(cx, |s, _| s.get().terminal_font_size), 19);
        assert_eq!(*count.borrow(), 1);
        // Persisted to disk — a fresh store reads it back.
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .terminal_font_size,
            19
        );
        // Idempotent set does not notify again.
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::from(19), cx);
        });
        cx.run_until_parked();
        assert_eq!(*count.borrow(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    const SAMPLE_THEME: &str = r##"{
        "name": "Sample",
        "variants": {
            "dark":  { "mode": "dark",  "colors": { "primary": "#ff0000" } },
            "light": { "mode": "light", "colors": { "primary": "#0000ff" } }
        }
    }"##;

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-themes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn slugify_makes_filesystem_safe_names() {
        assert_eq!(slugify("Tokyo Night!!"), "tokyo-night");
        assert_eq!(slugify("  "), "theme");
        assert_eq!(slugify("Ayu_Mirage"), "ayu-mirage");
    }

    #[test]
    fn trim_ext_strips_extension() {
        assert_eq!(trim_ext("wall.png"), "wall");
        assert_eq!(trim_ext("no-ext"), "no-ext");
    }

    #[test]
    fn scan_themes_lists_valid_user_themes_and_skips_junk() {
        let dir = tmp();
        std::fs::write(dir.join("good.json"), SAMPLE_THEME).unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignore me").unwrap();
        // A file literally named default.json must never shadow the built-in.
        std::fs::write(dir.join("default.json"), SAMPLE_THEME).unwrap();

        let list = scan_themes(&dir);
        assert_eq!(list[0].id, "default");
        assert!(list[0].builtin);
        let ids: Vec<&str> = list.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["default", "good"]);
        assert_eq!(list[1].name, "Sample");
        assert!(!list[1].builtin);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_read_and_delete_theme_roundtrip() {
        let dir = tmp();
        save_theme_file_in(&dir, "mine", SAMPLE_THEME).unwrap();
        let file = read_theme_file_in(&dir, "mine").unwrap();
        assert_eq!(file.name, "Sample");

        assert!(delete_theme_in(&dir, "default").is_err());
        delete_theme_in(&dir, "mine").unwrap();
        assert!(read_theme_file_in(&dir, "mine").is_err());
        assert_eq!(scan_themes(&dir).len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[gpui::test]
    fn app_font_family_preference_persists(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        store.update(cx, |s, cx| {
            s.set_value("appFontFamily", Value::String("Inter".into()), cx);
        });
        cx.run_until_parked();
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .app_font_family,
            "Inter"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[gpui::test]
    fn bad_type_is_rejected(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::String("huge".into()), cx);
        });
        assert_eq!(
            store.read_with(cx, |s, _| s.get().terminal_font_size),
            Preferences::default().terminal_font_size
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
