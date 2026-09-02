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
    anchored, deferred, div, point, px, size, App, AppContext, Bounds, ClickEvent, ClipboardItem,
    Context, Entity, FocusHandle, Focusable, Global, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, PathPromptOptions, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, TitlebarOptions, Window, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
};
use gpui_component::Root;
use serde_json::Value;
use tokio::runtime::Handle as TokioHandle;

use labonair_backend::modules::fs::paths::config_dir;
use labonair_theme::ThemeFile;

use crate::background::BackgroundStore;
use crate::command_palette::{
    effective_binding, resolve_conflict, shortcut, shortcut_slug, shortcuts, Conflict, KeybindMap,
    ShortcutId,
};

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

// ─────────────────────────── Settings OS window ──────────────────────────

/// The 10 top-level settings sections, in the reference sidebar order
/// (`reference-src/src/settings/SettingsApp.tsx`). Deep-link targets from the
/// menu / command palette / `labonair:settings-tab` equivalent map onto these.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Appearance,
    Themes,
    Terminal,
    Editor,
    FileManager,
    Connections,
    Workspace,
    Shortcuts,
    Ai,
}

impl SettingsTab {
    /// Index into [`CATEGORIES`].
    fn category_index(self) -> usize {
        let name = match self {
            SettingsTab::General => "General",
            SettingsTab::Appearance => CAT_APPEARANCE,
            SettingsTab::Themes => "Themes",
            SettingsTab::Terminal => "Terminal",
            SettingsTab::Editor => "Editor",
            SettingsTab::FileManager => "File Manager",
            SettingsTab::Connections => "Connections",
            SettingsTab::Workspace => "Workspace",
            SettingsTab::Shortcuts => KEYBOARD,
            SettingsTab::Ai => "AI",
        };
        CATEGORIES.iter().position(|c| *c == name).unwrap_or(0)
    }

    /// Port of the reference deep-link aliases (`SettingsApp.tsx`): `models |
    /// agents | connections | directives → ai`, `bookmarks | command-palette |
    /// source-control | layout → workspace/appearance`, …
    pub fn from_deep_link(slug: &str) -> Option<Self> {
        Some(match slug {
            "general" => SettingsTab::General,
            "appearance" | "layout" => SettingsTab::Appearance,
            "themes" => SettingsTab::Themes,
            "terminal" => SettingsTab::Terminal,
            "editor" => SettingsTab::Editor,
            "file-manager" => SettingsTab::FileManager,
            "remote-connections" | "connections" => SettingsTab::Connections,
            "workspace" | "bookmarks" | "command-palette" | "source-control" => {
                SettingsTab::Workspace
            }
            "shortcuts" => SettingsTab::Shortcuts,
            "models" | "agents" | "ai" | "directives" | "security" => SettingsTab::Ai,
            _ => return None,
        })
    }
}

/// Shared handles the settings window needs, published by `AppShell` once at
/// startup (the window is opened lazily, possibly long after `AppShell::new`).
#[derive(Clone)]
struct SettingsDeps {
    prefs: Entity<PreferencesStore>,
    backend: Backend,
    tokio: TokioHandle,
}

impl Global for SettingsDeps {}

/// The live settings window, if one is open. Checked on every open request so a
/// second invocation focuses the existing window instead of duplicating it.
#[derive(Default)]
struct SettingsWindowRef {
    handle: Option<WindowHandle<Root>>,
}

impl Global for SettingsWindowRef {}

/// The section a pending deep-link wants to show. `SettingsView` observes this
/// global so an already-open window jumps to the requested tab.
#[derive(Clone, Copy, Default)]
struct SettingsTarget(Option<SettingsTab>);

impl Global for SettingsTarget {}

/// Publish the shared handles the settings window builds from. Call once from
/// `AppShell::new` after the [`PreferencesStore`] exists.
pub fn set_settings_deps(
    prefs: Entity<PreferencesStore>,
    backend: Backend,
    tokio: TokioHandle,
    cx: &mut App,
) {
    cx.set_global(SettingsDeps {
        prefs,
        backend,
        tokio,
    });
}

/// Window bounds: 860 logical px wide, height = 80 % of the primary display
/// clamped to `[580, 900]` — a straight port of `settings_window_size()` in
/// `reference-src/src-tauri/src/lib.rs`.
fn settings_bounds(cx: &mut App) -> Bounds<gpui::Pixels> {
    let display_h = cx
        .primary_display()
        .map(|d| f32::from(d.bounds().size.height))
        .unwrap_or(1000.0);
    let h = (display_h * 0.8).clamp(580.0, 900.0);
    Bounds::centered(None, size(px(860.0), px(h)), cx)
}

/// Open the settings window, or focus it if it is already open, optionally
/// deep-linking to `tab`. Replaces the old in-`AppShell` modal overlay
/// (T16-009). The window destroys on close and is cheaply rebuilt on the next
/// open (GPUI 0.2.2 has no per-window hide); shared state lives in the
/// [`PreferencesStore`] / theme / background entities, so nothing is lost.
pub fn open_settings_window(tab: Option<SettingsTab>, cx: &mut App) {
    cx.set_global(SettingsTarget(tab));

    let existing = cx.try_global::<SettingsWindowRef>().and_then(|w| w.handle);
    if let Some(handle) = existing {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            cx.activate(true);
            return;
        }
        // Stale handle (window was closed) — fall through and open a fresh one.
        cx.set_global(SettingsWindowRef { handle: None });
    }

    let Some(deps) = cx.try_global::<SettingsDeps>().cloned() else {
        tracing::warn!("settings deps not published; cannot open settings window");
        return;
    };

    let bounds = settings_bounds(cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Settings".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(19.0), px((44.0 - 14.0) / 2.0))),
            }),
            window_min_size: Some(size(px(720.0), px(480.0))),
            kind: WindowKind::Normal,
            is_movable: true,
            ..Default::default()
        },
        move |window, cx| {
            let theme = crate::theme::theme_store(cx);
            let background = crate::background::background_store(cx);
            let view = cx.new(|cx| {
                let mut v = SettingsView::new(
                    deps.prefs.clone(),
                    theme,
                    background,
                    deps.backend.clone(),
                    deps.tokio.clone(),
                    cx,
                );
                v.windowed = true;
                v.open = true;
                if let Some(SettingsTarget(Some(tab))) = cx.try_global::<SettingsTarget>().copied()
                {
                    v.active_cat = tab.category_index();
                }
                v.refresh_themes();
                v.refresh_mcp_status(cx);
                window.focus(&v.focus);
                v
            });
            let view: gpui::AnyView = view.into();
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(handle) => {
            cx.set_global(SettingsWindowRef {
                handle: Some(handle),
            });
            cx.activate(true);
        }
        Err(e) => tracing::error!("failed to open settings window: {e}"),
    }
}

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

/// The "AI Agent Bridge" (MCP) sub-section — rendered inside the `Connections`
/// pane, matching `reference-src/src/settings/sections/ConnectionsSection.tsx`
/// (it used to be a wrong top-level entry).
pub const AGENT_BRIDGE: &str = "AI Agent Bridge";
pub const KEYBOARD: &str = "Shortcuts";
pub const CAT_APPEARANCE: &str = "Appearance & Layout";

/// The 10 top-level settings sections, matching the reference sidebar order.
pub const CATEGORIES: &[&str] = &[
    "General",
    CAT_APPEARANCE,
    "Themes",
    "Terminal",
    "Editor",
    "File Manager",
    "Connections",
    "Workspace",
    KEYBOARD,
    "AI",
];

// ─────────────────────────── keybind mutation (pure) ─────────────────────────

/// Result of capturing a keystroke for a shortcut.
pub(crate) enum KbCapture {
    /// The keystroke is free — here is the new override map to persist.
    Set(KeybindMap),
    /// The keystroke is already used by another shortcut — needs a decision.
    Conflict(ShortcutId),
    /// The keystroke is an OS/menu-reserved accelerator — refused.
    Reserved(&'static str),
}

/// Pure port of `useKeybindsStore.setKeybind` + conflict detection: decide
/// what capturing `binding` for `id` means, given the current `map`.
pub(crate) fn capture_keybind(map: &KeybindMap, id: ShortcutId, binding: &str) -> KbCapture {
    match resolve_conflict(binding, Some(id), map) {
        Some(Conflict::Reserved(label)) => KbCapture::Reserved(label),
        Some(Conflict::Shortcut(other)) => KbCapture::Conflict(other),
        None => {
            let mut m = map.clone();
            m.insert(shortcut_slug(id).to_string(), binding.to_string());
            KbCapture::Set(m)
        }
    }
}

/// Resolve a capture conflict by giving `binding` to `id` and unbinding the
/// previous owner — no silent double-binding.
pub(crate) fn overwrite_keybind(
    map: &KeybindMap,
    id: ShortcutId,
    other: ShortcutId,
    binding: &str,
) -> KeybindMap {
    let mut m = map.clone();
    m.insert(shortcut_slug(other).to_string(), String::new());
    m.insert(shortcut_slug(id).to_string(), binding.to_string());
    m
}

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
        "sessionRestore",
        "Session restore",
        "Reopen all tabs, SSH connections, SFTP paths and editor files on the next launch.",
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
        CAT_APPEARANCE,
        Text,
    ),
    d(
        "appFontSize",
        "App font size",
        "Base UI font size in points.",
        CAT_APPEARANCE,
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
        CAT_APPEARANCE,
        Switch,
    ),
    d(
        "appCornerRadius",
        "Corner radius",
        "Rounding of panels and cards (px).",
        CAT_APPEARANCE,
        Int { min: 0, max: 20, step: 1 },
    ),
    d(
        "tabsLocation",
        "Tab bar location",
        "Where the tab strip lives.",
        CAT_APPEARANCE,
        Select(&["titlebar", "sidebar"]),
    ),
    d(
        "sidebarGroupByFolder",
        "Group sidebar tabs by folder",
        "Group tabs that share a working directory.",
        CAT_APPEARANCE,
        Switch,
    ),
    d(
        "sidebarGroupSingleTabs",
        "Group single tabs too",
        "Also show a group header for a lone tab.",
        CAT_APPEARANCE,
        Switch,
    ),
    d(
        "badgesAlwaysVisible",
        "Always show badges",
        "Keep count badges visible even at zero.",
        CAT_APPEARANCE,
        Switch,
    ),
    d(
        "zenModeShowHeader",
        "Show header bar",
        "Show the window header bar (zen mode off).",
        CAT_APPEARANCE,
        Switch,
    ),
    d(
        "zenModeShowStatusbar",
        "Show status bar",
        "Show the bottom status bar (zen mode off).",
        CAT_APPEARANCE,
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
        "sessionScrollbackLines",
        "Persisted scrollback lines",
        "Rows of history saved per pane on quit and replayed on the next launch (0 = all).",
        "Terminal",
        Int {
            min: 0,
            max: 100_000,
            step: 500,
        },
    ),
    d(
        "scrollbackMaxSizeMb",
        "Persisted scrollback size cap",
        "Per-file ceiling for a saved scrollback, in MB.",
        "Terminal",
        Int {
            min: 1,
            max: 100,
            step: 1,
        },
    ),
    d(
        "scrollbackRetentionDays",
        "Persisted scrollback retention",
        "Days a saved scrollback file is kept before cleanup removes it (0 = keep with the session).",
        "Terminal",
        Int {
            min: 0,
            max: 365,
            step: 1,
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
        "vimMode",
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
        "Workspace",
        Select(&["contains", "startsWith", "fuzzy"]),
    ),
    d(
        "commandPaletteShowRecent",
        "Show recent",
        "Surface recently-run commands first.",
        "Workspace",
        Switch,
    ),
    // Source Control
    d(
        "gitStatusPollIntervalMs",
        "Status poll interval",
        "How often to refresh git status (ms).",
        "Workspace",
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
    // ── General (added T16-011) ──────────────────────────────────────────
    d(
        "autostart",
        "Launch at login",
        "Start Labonair automatically when you log in.",
        "General",
        Switch,
    ),
    d(
        "startupTerminalCount",
        "Startup terminal count",
        "How many terminals open on launch.",
        "General",
        Int { min: 1, max: 3, step: 1 },
    ),
    d(
        "credentialEncryption",
        "Encrypt stored credentials",
        "Encrypt saved credentials at rest with an OS-backed key.",
        "General",
        Switch,
    ),
    // ── Terminal (added T16-011) ─────────────────────────────────────────
    d(
        "terminalDefaultPath",
        "Default working directory",
        "Directory new terminals start in (empty = home).",
        "Terminal",
        Text,
    ),
    d(
        "newTabInheritsCwd",
        "New tab inherits directory",
        "Open new terminal tabs in the current tab's directory.",
        "Terminal",
        Switch,
    ),
    d(
        "confirmCloseTerminalTab",
        "Confirm before closing a terminal tab",
        "Ask for confirmation when closing a terminal tab.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalFontWeight",
        "Font weight",
        "Weight of the terminal typeface.",
        "Terminal",
        Select(&["normal", "medium", "bold"]),
    ),
    d(
        "terminalCursorBlinkInterval",
        "Cursor blink interval (ms)",
        "How fast the terminal cursor blinks.",
        "Terminal",
        Int { min: 200, max: 2000, step: 50 },
    ),
    d(
        "terminalRightClickPastes",
        "Right-click pastes",
        "Paste the clipboard on right-click instead of a context menu.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalWordSeparator",
        "Word separators",
        "Characters that break a word for double-click selection.",
        "Terminal",
        Text,
    ),
    d(
        "terminalScrollSensitivity",
        "Scroll sensitivity",
        "Lines scrolled per wheel notch.",
        "Terminal",
        Int { min: 1, max: 10, step: 1 },
    ),
    d(
        "terminalFastScrollModifier",
        "Fast-scroll modifier",
        "Hold this key to scroll faster.",
        "Terminal",
        Select(&["none", "alt", "ctrl", "shift"]),
    ),
    d(
        "terminalShowPaneHeader",
        "Show pane headers",
        "Show a header strip above each terminal pane.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalShowPaneFooter",
        "Show pane footer",
        "Show a footer strip below each terminal pane.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalComposerEnabled",
        "Command composer",
        "Show the composer input above the terminal.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalComposerHistoryPopup",
        "Composer history popup",
        "Show a history dropdown while composing.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalComposerArgumentCompletion",
        "Argument completion",
        "Suggest command arguments in the composer.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalBlocksEnabled",
        "Block terminal",
        "Group command output into collapsible blocks.",
        "Terminal",
        Switch,
    ),
    d(
        "terminalBlocksAutoCollapseOnAltScreen",
        "Auto-collapse blocks for full-screen apps",
        "Collapse blocks when an app takes the alternate screen.",
        "Terminal",
        Switch,
    ),
    // ── Editor (added T16-011) ───────────────────────────────────────────
    d(
        "editorAutoSave",
        "Auto save",
        "When to automatically save edited files.",
        "Editor",
        Select(&["off", "afterDelay", "onFocusChange"]),
    ),
    d(
        "editorAutoSaveDelay",
        "Auto save delay (ms)",
        "Idle time before an auto save when 'after delay' is selected.",
        "Editor",
        Int { min: 100, max: 60_000, step: 100 },
    ),
    d(
        "editorTrimTrailingWhitespace",
        "Trim trailing whitespace",
        "Remove trailing spaces on save.",
        "Editor",
        Switch,
    ),
    d(
        "editorInsertFinalNewline",
        "Insert final newline",
        "Ensure a trailing newline on save.",
        "Editor",
        Switch,
    ),
    d(
        "editorBracketMatching",
        "Bracket matching",
        "Highlight the matching bracket at the cursor.",
        "Editor",
        Switch,
    ),
    d(
        "editorShowCursorPosition",
        "Cursor position",
        "Show line/column in the status bar.",
        "Editor",
        Switch,
    ),
    d(
        "editorShowSelectionStats",
        "Selection stats",
        "Show selected character / line counts.",
        "Editor",
        Switch,
    ),
    d(
        "editorShowOutline",
        "Outline panel",
        "Show the document symbol outline.",
        "Editor",
        Switch,
    ),
    d(
        "editorIndentationGuides",
        "Indentation guides",
        "Draw vertical indentation guide lines.",
        "Editor",
        Switch,
    ),
    d(
        "editorAutocompleteDebounceMs",
        "Autocomplete debounce (ms)",
        "Idle time before requesting an AI completion.",
        "Editor",
        Int { min: 50, max: 2000, step: 50 },
    ),
    d(
        "editorMaxFileSizeMb",
        "Max file size (MB)",
        "Files larger than this open read-only / unhighlighted.",
        "Editor",
        Int { min: 1, max: 100, step: 1 },
    ),
    // ── File Manager (added T16-011) ─────────────────────────────────────
    d(
        "sftpShowUpFolder",
        "Show '..' up-folder entry",
        "Show an entry to go to the parent directory.",
        "File Manager",
        Switch,
    ),
    d(
        "explorerShowHiddenByDefault",
        "Explorer: show hidden files by default",
        "Show dotfiles in the sidebar explorer.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpColumnSize",
        "Show Size column",
        "Show the file size column.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpColumnModified",
        "Show Modified column",
        "Show the modification time column.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpColumnPermissions",
        "Show Permissions column",
        "Show the permissions column.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpColumnType",
        "Show Type column",
        "Show the file type column.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpRemoteEditShowTransfers",
        "Show remote edit transfers",
        "Show a transfer indicator when editing remote files.",
        "File Manager",
        Switch,
    ),
    d(
        "sftpMaxRemoteFileSizeMb",
        "Max remote file size (MB)",
        "Refuse to open remote files larger than this for editing.",
        "File Manager",
        Int { min: 1, max: 100, step: 1 },
    ),
    d(
        "sftpDefaultConflictResolution",
        "On name conflict",
        "Default action when a transfer target already exists.",
        "File Manager",
        Select(&["ask", "overwrite", "skip"]),
    ),
    d(
        "sftpChunkSizeKb",
        "Transfer chunk size (KB)",
        "Block size used for SFTP transfers.",
        "File Manager",
        Int { min: 16, max: 1024, step: 16 },
    ),
    d(
        "sftpOnFolderFileError",
        "On file error in folder transfers",
        "What to do when one file in a recursive transfer fails.",
        "File Manager",
        Select(&["ask", "skip", "abort"]),
    ),
    // ── Connections (added T16-011) ─────────────────────────────────────
    d(
        "hostPingInterval",
        "Host availability ping interval (s)",
        "How often to ping saved hosts (0 = never).",
        "Connections",
        Int { min: 0, max: 600, step: 10 },
    ),
    d(
        "sshConnectTimeoutSecs",
        "SSH connect timeout (s)",
        "Give up connecting after this long.",
        "Connections",
        Int { min: 3, max: 60, step: 1 },
    ),
    d(
        "sshAutoReconnect",
        "Auto-reconnect SSH sessions",
        "Reconnect dropped SSH terminal sessions automatically.",
        "Connections",
        Switch,
    ),
    d(
        "sshAutoReconnectDelay",
        "Reconnect delay (s)",
        "Wait this long before an SSH reconnect attempt.",
        "Connections",
        Int { min: 1, max: 30, step: 1 },
    ),
    d(
        "sshAutoReconnectMaxAttempts",
        "Max reconnect attempts",
        "Give up after this many SSH reconnect attempts.",
        "Connections",
        Int { min: 1, max: 10, step: 1 },
    ),
    d(
        "explorerRemotePollInterval",
        "Explorer: remote refresh interval (s)",
        "How often the remote explorer re-reads the directory (0 = never).",
        "Connections",
        Int { min: 0, max: 60, step: 10 },
    ),
    d(
        "explorerAutoReconnect",
        "Explorer: auto-reconnect remote sessions",
        "Reconnect dropped remote explorer sessions.",
        "Connections",
        Switch,
    ),
    d(
        "explorerIdleSessionTimeoutMin",
        "Explorer: idle session timeout (min)",
        "Close idle cached remote sessions after this long.",
        "Connections",
        Int { min: 1, max: 30, step: 1 },
    ),
    d(
        "explorerMaxIdleSessions",
        "Explorer: max cached remote sessions",
        "Upper bound on kept-alive idle remote sessions.",
        "Connections",
        Int { min: 1, max: 10, step: 1 },
    ),
    d(
        "explorerMaxCachedRemoteScopes",
        "Explorer: max cached remote folders",
        "Upper bound on cached remote directory listings.",
        "Connections",
        Int { min: 1, max: 20, step: 1 },
    ),
    // ── Command Palette (added T16-011) ─────────────────────────────────
    d(
        "commandPaletteBlur",
        "Background blur",
        "Backdrop blur behind the palette (px).",
        "Workspace",
        Int { min: 0, max: 20, step: 1 },
    ),
    d(
        "commandPaletteOpacity",
        "Palette opacity",
        "Opacity of the palette surface (%).",
        "Workspace",
        Int { min: 60, max: 100, step: 1 },
    ),
    d(
        "commandPalettePosition",
        "Open position",
        "Vertical position the palette opens at.",
        "Workspace",
        Select(&["top", "high", "center"]),
    ),
    d(
        "commandPaletteAnimation",
        "Animation speed",
        "Open/close animation speed.",
        "Workspace",
        Select(&["fast", "normal", "slow", "none"]),
    ),
    d(
        "commandPaletteHistorySize",
        "Recent history size",
        "How many recent commands to remember.",
        "Workspace",
        Int { min: 3, max: 20, step: 1 },
    ),
    d(
        "commandPaletteCloseOnOverlayClick",
        "Close on outside click",
        "Dismiss the palette when clicking outside it.",
        "Workspace",
        Switch,
    ),
    // ── Bookmarks (added T16-011) ──────────────────────────────────────
    d(
        "bookmarksEnabled",
        "Enable path bookmarks",
        "Show the bookmarks bar-item and jump targets.",
        "Workspace",
        Switch,
    ),
    d(
        "bookmarksActionNewTerminal",
        "Open in new terminal",
        "Offer 'open in a new terminal' for a bookmark.",
        "Workspace",
        Switch,
    ),
    d(
        "bookmarksActionCurrentTerminal",
        "Open in current terminal",
        "Offer 'cd in the current terminal' for a bookmark.",
        "Workspace",
        Switch,
    ),
    d(
        "bookmarksActionCurrentSftp",
        "Open in current SFTP manager",
        "Offer 'go to path in the current file manager'.",
        "Workspace",
        Switch,
    ),
    d(
        "bookmarksActionNewSftp",
        "Open in new SFTP tab",
        "Offer 'open the path in a new file-manager tab'.",
        "Workspace",
        Switch,
    ),
    d(
        "bookmarksPrimaryClickBehavior",
        "Primary click opens",
        "What a plain click on a bookmark does.",
        "Workspace",
        Select(&["current", "new"]),
    ),
    d(
        "bookmarksShowBadge",
        "Show bookmark count badge",
        "Show the number of bookmarks on the bar-item.",
        "Workspace",
        Switch,
    ),
    // ── AI (added T16-011) ────────────────────────────────────────────
    d(
        "aiTemperature",
        "Temperature (x100)",
        "Model sampling temperature, expressed as a percentage (70 = 0.70).",
        "AI",
        Int { min: 0, max: 100, step: 5 },
    ),
    d(
        "aiAutoOpenMiniOnSend",
        "Auto-open mini window on send",
        "Pop out the mini chat window when a message is sent.",
        "AI",
        Switch,
    ),
    d(
        "aiNotifyOnHeadlessCommand",
        "Notify on background commands",
        "Show a toast when the agent runs a headless command.",
        "AI",
        Switch,
    ),
    d(
        "aiShellMaxTimeoutSecs",
        "Max command timeout (s)",
        "Kill an agent shell command after this long.",
        "AI",
        Int { min: 30, max: 1800, step: 30 },
    ),
    d(
        "aiShellMaxOutputKb",
        "Max command output (KB)",
        "Truncate captured command output past this size.",
        "AI",
        Int { min: 64, max: 2048, step: 64 },
    ),
    d(
        "defaultModelId",
        "Chat model",
        "Model id used for new AI chats.",
        "AI",
        Text,
    ),
    d(
        "customInstructions",
        "Custom instructions",
        "Extra system-prompt guidance for the agent.",
        "AI",
        Text,
    ),
    d(
        "autocompleteEnabled",
        "Editor autocomplete",
        "Enable inline AI completions in the editor.",
        "AI",
        Switch,
    ),
    d(
        "autocompleteProvider",
        "Autocomplete provider",
        "Provider used for editor completions.",
        "AI",
        Text,
    ),
    d(
        "autocompleteModelId",
        "Autocomplete model id",
        "Model id used for editor completions.",
        "AI",
        Text,
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

/// Sub-section layout per top-level category, mirroring the group headers in
/// `reference-src/src/settings/sections/*`. `render_grouped` walks these in
/// order; any field not named here still renders under a trailing "Other".
type FieldGroup = (&'static str, &'static [&'static str]);
pub const SECTION_GROUPS: &[(&str, &[FieldGroup])] = &[
    (
        "General",
        &[
            (
                "Startup",
                &["defaultStartupTab", "startupTerminalCount", "autostart"],
            ),
            (
                "Session Restore",
                &[
                    "sessionRestore",
                    "sessionScrollbackLines",
                    "scrollbackMaxSizeMb",
                    "scrollbackRetentionDays",
                ],
            ),
            ("Window", &["restoreWindowState"]),
            ("Security", &["credentialEncryption"]),
            ("Quit", &["confirmQuitWithSsh"]),
            ("Updates", &["checkForUpdates"]),
            ("Notifications", &["notifyOnErrors"]),
        ],
    ),
    (
        "Terminal",
        &[
            (
                "Shell",
                &[
                    "terminalShell",
                    "terminalDefaultPath",
                    "newTabInheritsCwd",
                    "confirmCloseTerminalTab",
                ],
            ),
            (
                "Font",
                &[
                    "terminalFontFamily",
                    "terminalFontSize",
                    "terminalFontWeight",
                ],
            ),
            (
                "Cursor",
                &[
                    "terminalCursorStyle",
                    "terminalCursorBlink",
                    "terminalCursorBlinkInterval",
                ],
            ),
            (
                "Layout",
                &["terminalShowPaneHeader", "terminalShowPaneFooter"],
            ),
            (
                "Composer & Blocks",
                &[
                    "terminalComposerEnabled",
                    "terminalComposerHistoryPopup",
                    "terminalComposerArgumentCompletion",
                    "terminalBlocksEnabled",
                    "terminalBlocksAutoCollapseOnAltScreen",
                ],
            ),
            ("Bell", &["terminalBell"]),
            ("Buffer", &["terminalScrollback"]),
            (
                "Input",
                &[
                    "terminalCopyOnSelect",
                    "terminalRightClickPastes",
                    "terminalWordSeparator",
                ],
            ),
            (
                "Scrolling",
                &["terminalScrollSensitivity", "terminalFastScrollModifier"],
            ),
            ("Appearance", &["terminalOpacity"]),
        ],
    ),
    (
        "Editor",
        &[
            ("Keybindings", &["vimMode", "editorRelativeLineNumbers"]),
            ("Theme", &["editorTheme"]),
            ("Font", &["editorFontFamily", "editorFontSize"]),
            (
                "Behaviour",
                &[
                    "editorFormatOnSave",
                    "editorAutoSave",
                    "editorAutoSaveDelay",
                    "editorTabSize",
                ],
            ),
            ("Indentation", &["editorIndentWithTabs"]),
            ("Files", &["editorMaxFileSizeMb"]),
            (
                "Display",
                &[
                    "editorLineNumbers",
                    "editorWordWrap",
                    "editorBracketMatching",
                    "editorShowCursorPosition",
                    "editorShowSelectionStats",
                    "editorShowOutline",
                    "editorIndentationGuides",
                ],
            ),
            (
                "On Save",
                &["editorTrimTrailingWhitespace", "editorInsertFinalNewline"],
            ),
            ("AI Completion", &["editorAutocompleteDebounceMs"]),
        ],
    ),
    (
        "File Manager",
        &[
            (
                "Browsing",
                &[
                    "sftpShowHiddenFiles",
                    "sftpShowUpFolder",
                    "explorerShowHiddenByDefault",
                ],
            ),
            (
                "Columns",
                &[
                    "sftpColumnSize",
                    "sftpColumnModified",
                    "sftpColumnPermissions",
                    "sftpColumnType",
                ],
            ),
            (
                "Remote Editing",
                &[
                    "sftpRemoteEditShowTransfers",
                    "sftpMaxRemoteFileSizeMb",
                    "sftpFontSize",
                ],
            ),
            (
                "Transfers",
                &[
                    "sftpMaxConcurrentTransfers",
                    "sftpDefaultConflictResolution",
                    "sftpChunkSizeKb",
                    "sftpOnFolderFileError",
                ],
            ),
        ],
    ),
    (
        "Connections",
        &[
            ("Host Availability", &["hostPingInterval"]),
            (
                "SSH Terminal Sessions",
                &[
                    "sshConnectTimeoutSecs",
                    "sshAutoReconnect",
                    "sshAutoReconnectDelay",
                    "sshAutoReconnectMaxAttempts",
                ],
            ),
            (
                "Remote File Browsing",
                &[
                    "explorerRemotePollInterval",
                    "explorerAutoReconnect",
                    "explorerIdleSessionTimeoutMin",
                    "explorerMaxIdleSessions",
                    "explorerMaxCachedRemoteScopes",
                ],
            ),
        ],
    ),
    (
        "Workspace",
        &[
            (
                "Bookmarks",
                &[
                    "bookmarksEnabled",
                    "bookmarksActionNewTerminal",
                    "bookmarksActionCurrentTerminal",
                    "bookmarksActionCurrentSftp",
                    "bookmarksActionNewSftp",
                    "bookmarksPrimaryClickBehavior",
                    "bookmarksShowBadge",
                ],
            ),
            (
                "Command Palette",
                &[
                    "commandPaletteBlur",
                    "commandPaletteOpacity",
                    "commandPalettePosition",
                    "commandPaletteAnimation",
                    "commandPaletteShowRecent",
                    "commandPaletteHistorySize",
                    "commandPaletteSearchMode",
                    "commandPaletteCloseOnOverlayClick",
                ],
            ),
            ("Source Control", &["gitStatusPollIntervalMs"]),
        ],
    ),
    (
        "AI",
        &[
            (
                "Defaults",
                &[
                    "defaultModelId",
                    "autocompleteEnabled",
                    "autocompleteProvider",
                    "autocompleteModelId",
                ],
            ),
            ("General", &["aiEnabled", "aiWarnDestructiveCommands"]),
            (
                "Behaviour",
                &[
                    "aiAutoOpenMiniOnSend",
                    "aiNotifyOnHeadlessCommand",
                    "aiMaxAgentSteps",
                    "aiTemperature",
                    "aiTerminalContextLines",
                    "aiShellMaxTimeoutSecs",
                    "aiShellMaxOutputKb",
                ],
            ),
            ("Agents & Directives", &["customInstructions"]),
        ],
    ),
];

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
    /// Shortcut currently capturing a new key combination (`Keyboard` pane).
    recording: Option<ShortcutId>,
    /// A captured combination that collides with another shortcut, awaiting
    /// the user's overwrite / cancel decision.
    kb_conflict: Option<KbConflict>,
    /// `true` when this view is the root of its own OS window (T16-009); `false`
    /// for the legacy in-`AppShell` modal path (kept for tests only).
    windowed: bool,
    /// An open `Select` dropdown (key + anchor position + options), drawn as a
    /// deferred floating layer so it escapes the scroll clip.
    dropdown: Option<SelectMenu>,
    focus: FocusHandle,
}

struct SelectMenu {
    key: &'static str,
    options: &'static [&'static str],
    at: Point<Pixels>,
}

struct KbConflict {
    id: ShortcutId,
    binding: String,
    other: ShortcutId,
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
        // Deep-link: jump to the requested tab when another part of the app
        // asks for a specific settings section while this window is open.
        cx.observe_global::<SettingsTarget>(|this, cx| {
            if let Some(SettingsTarget(Some(tab))) = cx.try_global::<SettingsTarget>().copied() {
                this.active_cat = tab.category_index();
                this.search.clear();
                cx.notify();
            }
        })
        .detach();
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
            recording: None,
            kb_conflict: None,
            windowed: false,
            dropdown: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.editing = None;
        self.recording = None;
        self.kb_conflict = None;
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

    /// Close request from Esc / the header close button. In windowed mode this
    /// destroys the OS window (GPUI 0.2.2 has no per-window hide); the shared
    /// [`PreferencesStore`] keeps all persistent state so the next open is
    /// instant and lossless.
    fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.windowed {
            cx.set_global(SettingsWindowRef { handle: None });
            self.editing = None;
            window.remove_window();
        } else {
            self.close(cx);
        }
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
        // Rebind the keymap so a changed shortcut takes effect immediately
        // (and the native menu accelerators re-derive) (T13-004).
        if key == "keybinds" {
            let kb = self.prefs.read(cx).get().keybinds.clone();
            crate::menu::apply_keybinds(cx, &kb);
        }
        // Typography + editor syntax scheme are pushed into the ThemeStore so
        // open terminals / editors pick them up live (T13-003).
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    // ── keyboard shortcuts ────────────────────────────────────────────────

    fn keybinds(&self, cx: &App) -> KeybindMap {
        self.prefs.read(cx).get().keybinds.clone()
    }

    fn write_keybinds(&mut self, map: KeybindMap, cx: &mut Context<Self>) {
        let value = serde_json::to_value(map).unwrap_or(Value::Null);
        self.set_pref("keybinds", value, cx);
    }

    /// Translate a captured keystroke into a persisted override (or a
    /// conflict prompt / rejection).
    fn capture_shortcut(&mut self, id: ShortcutId, binding: String, cx: &mut Context<Self>) {
        let map = self.keybinds(cx);
        match capture_keybind(&map, id, &binding) {
            KbCapture::Set(next) => {
                self.kb_conflict = None;
                self.write_keybinds(next, cx);
            }
            KbCapture::Conflict(other) => {
                self.kb_conflict = Some(KbConflict { id, binding, other });
                cx.notify();
            }
            KbCapture::Reserved(label) => {
                self.notify_error(
                    cx,
                    "Reserved shortcut",
                    format!("{binding} is reserved for \u{201c}{label}\u{201d}."),
                );
                cx.notify();
            }
        }
    }

    fn resolve_kb_conflict(&mut self, cx: &mut Context<Self>) {
        let Some(kc) = self.kb_conflict.take() else {
            return;
        };
        let map = self.keybinds(cx);
        let next = overwrite_keybind(&map, kc.id, kc.other, &kc.binding);
        self.write_keybinds(next, cx);
    }

    fn reset_keybind(&mut self, id: ShortcutId, cx: &mut Context<Self>) {
        let mut map = self.keybinds(cx);
        if map.remove(shortcut_slug(id)).is_some() {
            self.write_keybinds(map, cx);
        }
    }

    fn reset_all_keybinds(&mut self, cx: &mut Context<Self>) {
        self.kb_conflict = None;
        self.recording = None;
        self.write_keybinds(KeybindMap::new(), cx);
    }

    /// Handle a key press while a shortcut row is recording.
    fn record_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.recording else { return };
        let ks = &ev.keystroke;
        let key = ks.key.as_str();
        window.focus(&self.focus);
        if key == "escape" {
            self.recording = None;
            self.kb_conflict = None;
            cx.notify();
            return;
        }
        // A bare modifier press just updates the live hint — keep waiting.
        if matches!(
            key,
            "cmd" | "ctrl" | "control" | "alt" | "option" | "shift" | "fn" | "function"
        ) {
            return;
        }
        // The reference `eventToBinding` requires a non-shift modifier.
        if !(ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt) {
            self.notify_error(
                cx,
                "Shortcut needs a modifier",
                "Combine the key with \u{2318}, \u{2303} or \u{2325}.".to_string(),
            );
            return;
        }
        let binding = ks.unparse();
        self.recording = None;
        self.capture_shortcut(id, binding, cx);
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

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.recording.is_some() {
            self.record_key(ev, window, cx);
            cx.stop_propagation();
            return;
        }
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
            "escape" => self.request_close(window, cx),
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

    /// The floating options list for an open `Select` (T16-010). Rendered as a
    /// `deferred` + `anchored` layer so it is not clipped by the scroll area,
    /// with a transparent full-window backdrop that dismisses it.
    fn render_dropdown(&self, c: &Palette, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let menu = self.dropdown.as_ref()?;
        let key = menu.key;
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        let list = anchored().position(menu.at).snap_to_window().child(
            div()
                .id("dropdown-list")
                .occlude()
                .min_w(px(180.0))
                .max_h(px(320.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .p_1()
                .rounded_md()
                .bg(c.card)
                .border_1()
                .border_color(c.border)
                .children(menu.options.iter().map(|opt| {
                    let opt = *opt;
                    let selected = opt == cur;
                    div()
                        .id(SharedString::from(format!("opt-{key}-{opt}")))
                        .px_2()
                        .py(px(4.0))
                        .rounded_sm()
                        .text_size(px(11.5))
                        .text_color(if selected { c.fg } else { c.muted })
                        .when(selected, |d| d.bg(c.accent))
                        .when(!selected, |d| d.hover(|s| s.bg(c.border)))
                        .child(SharedString::from(opt))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.dropdown = None;
                            this.set_pref(key, Value::String(opt.to_string()), cx);
                        }))
                })),
        );
        Some(
            deferred(
                div()
                    .absolute()
                    .inset_0()
                    .child(div().id("dropdown-backdrop").absolute().inset_0().on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.dropdown = None;
                            cx.notify();
                        }),
                    ))
                    .child(list),
            )
            .with_priority(200)
            .into_any_element(),
        )
    }

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
                let is_open = self.dropdown.as_ref().is_some_and(|d| d.key == key);
                div()
                    .id(SharedString::from(format!("sel-{key}")))
                    .min_w(px(160.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .py(px(4.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_open { c.accent } else { c.border })
                    .bg(c.bg)
                    .text_color(c.fg)
                    .text_size(px(11.5))
                    .child(SharedString::from(cur))
                    .child(div().text_color(c.muted).child("\u{25BE}"))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                        if this.dropdown.as_ref().is_some_and(|d| d.key == key) {
                            this.dropdown = None;
                        } else {
                            this.dropdown = Some(SelectMenu {
                                key,
                                options,
                                at: ev.position(),
                            });
                        }
                        cx.notify();
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

        // (Theme cards moved to the dedicated "Themes" pane — `render_themes`.)

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

        root = root.child(section_label("Layout", c));
        for key in [
            "tabsLocation",
            "appCornerRadius",
            "sidebarGroupByFolder",
            "sidebarGroupSingleTabs",
            "badgesAlwaysVisible",
            "zenModeShowHeader",
            "zenModeShowStatusbar",
        ] {
            if let Some(def) = FIELDS.iter().find(|f| f.key == key) {
                root = root.child(self.render_field(def, c, cx));
            }
        }

        root = root.child(section_label("Typography", c));
        for key in ["appFontFamily", "appFontSize", "reduceMotion"] {
            if let Some(def) = FIELDS.iter().find(|f| f.key == key) {
                root = root.child(self.render_field(def, c, cx));
            }
        }

        root.into_any_element()
    }

    fn render_shortcuts(
        &self,
        query: &str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let overrides = self.prefs.read(cx).get().keybinds.clone();
        let recording = self.recording;
        let conflict_id = self.kb_conflict.as_ref().map(|k| k.id);

        let mut root = div().flex().flex_col().child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .py_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.0))
                        .text_color(c.muted)
                        .child(
                            "Click a shortcut, then press the new key combination. Esc cancels.",
                        ),
                )
                .child(
                    div()
                        .id("kb-reset-all")
                        .px_2()
                        .py(px(3.0))
                        .rounded_sm()
                        .border_1()
                        .border_color(c.border)
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.border))
                        .child("Reset all")
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.reset_all_keybinds(cx)),
                        ),
                ),
        );

        for s in shortcuts() {
            if !query.is_empty()
                && !s.label.to_lowercase().contains(query)
                && !shortcut_slug(s.id).to_lowercase().contains(query)
            {
                continue;
            }
            let id = s.id;
            let slug = shortcut_slug(id);
            let overridden = overrides.contains_key(slug);
            let is_rec = recording == Some(id);
            let display = if is_rec {
                "Press keys\u{2026}".to_string()
            } else {
                effective_binding(id, &overrides).unwrap_or_else(|| "Disabled".to_string())
            };

            let row = div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .py_2()
                .border_b_1()
                .border_color(c.border)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(c.fg)
                        .child(SharedString::from(s.label)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .id(SharedString::from(format!("kb-rec-{slug}")))
                                .px_2()
                                .py(px(3.0))
                                .min_w(px(120.0))
                                .text_center()
                                .rounded_sm()
                                .border_1()
                                .border_color(if is_rec { c.accent } else { c.border })
                                .bg(c.bg)
                                .text_color(c.fg)
                                .hover(|st| st.bg(c.border))
                                .child(SharedString::from(display))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.recording = Some(id);
                                    this.kb_conflict = None;
                                    window.focus(&this.focus);
                                    cx.notify();
                                })),
                        )
                        .when(overridden, |d| {
                            d.child(
                                div()
                                    .id(SharedString::from(format!("kb-reset-{slug}")))
                                    .px_2()
                                    .py(px(3.0))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(c.border)
                                    .text_color(c.muted)
                                    .hover(|st| st.text_color(c.fg))
                                    .child("Reset")
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        this.reset_keybind(id, cx);
                                    })),
                            )
                        }),
                );
            root = root.child(row);

            if conflict_id == Some(id) {
                let kc = self.kb_conflict.as_ref().unwrap();
                let msg = format!(
                    "{} is already used by \u{201c}{}\u{201d}.",
                    kc.binding,
                    shortcut(kc.other).label
                );
                root = root.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .rounded_sm()
                        .bg(c.bg)
                        .border_1()
                        .border_color(c.accent)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(11.0))
                                .text_color(c.fg)
                                .child(SharedString::from(msg)),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .child(
                                    div()
                                        .id("kb-conflict-overwrite")
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(c.border)
                                        .text_color(c.fg)
                                        .hover(|st| st.bg(c.border))
                                        .child("Overwrite")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                            this.resolve_kb_conflict(cx)
                                        })),
                                )
                                .child(
                                    div()
                                        .id("kb-conflict-cancel")
                                        .px_2()
                                        .py(px(2.0))
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(c.border)
                                        .text_color(c.muted)
                                        .hover(|st| st.text_color(c.fg))
                                        .child("Cancel")
                                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                            this.kb_conflict = None;
                                            cx.notify();
                                        })),
                                ),
                        ),
                );
            }
        }

        root.into_any_element()
    }

    fn render_body(&mut self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.search.trim().to_lowercase();
        if CATEGORIES[self.active_cat] == KEYBOARD && query.is_empty() {
            return self.render_shortcuts(&query, c, cx);
        }
        if !query.is_empty() {
            // Global search: results grouped by their top-level section, a port
            // of the reference `SearchResults` layout.
            let mut root = div().flex().flex_col();
            let mut any = false;
            for &cat in CATEGORIES {
                let matches: Vec<&FieldDef> = FIELDS
                    .iter()
                    .filter(|f| {
                        f.category == cat
                            && (f.title.to_lowercase().contains(&query)
                                || f.desc.to_lowercase().contains(&query)
                                || f.key.to_lowercase().contains(&query))
                    })
                    .collect();
                if matches.is_empty() {
                    continue;
                }
                any = true;
                root = root.child(section_label(cat, c)).children(
                    matches
                        .into_iter()
                        .map(|f| self.render_field(f, c, cx))
                        .collect::<Vec<_>>(),
                );
            }
            if !any {
                return div()
                    .p_4()
                    .text_color(c.muted)
                    .child("No matching settings.")
                    .into_any_element();
            }
            return root.into_any_element();
        }

        let cat = CATEGORIES[self.active_cat];
        match cat {
            "General" => return self.render_general(c, cx),
            "Themes" => return self.render_themes(c, cx),
            _ if cat == CAT_APPEARANCE => return self.render_appearance(c, cx),
            "Connections" => {
                return div()
                    .flex()
                    .flex_col()
                    .child(self.render_grouped(cat, c, cx))
                    .child(section_label(AGENT_BRIDGE, c))
                    .child(self.render_agent_bridge(c, cx))
                    .into_any_element();
            }
            _ => {}
        }
        self.render_grouped(cat, c, cx)
    }

    /// Conditional-row visibility — a port of the reference's `hidden` /
    /// `show_if` predicates (e.g. `sessionScrollbackLines` only when
    /// `sessionRestore`). An unknown key is always visible.
    fn field_visible(&self, key: &str, cx: &App) -> bool {
        let p = self.prefs.read(cx).get();
        match key {
            "sessionScrollbackLines" | "scrollbackMaxSizeMb" | "scrollbackRetentionDays" => {
                p.session_restore
            }
            "terminalCursorBlinkInterval" => p.terminal_cursor_blink,
            "terminalComposerHistoryPopup" | "terminalComposerArgumentCompletion" => {
                p.terminal_composer_enabled
            }
            "terminalBlocksAutoCollapseOnAltScreen" => p.terminal_blocks_enabled,
            "editorAutoSaveDelay" => p.editor_auto_save != "off",
            "sshAutoReconnectDelay" | "sshAutoReconnectMaxAttempts" => p.ssh_auto_reconnect,
            "explorerIdleSessionTimeoutMin"
            | "explorerMaxIdleSessions"
            | "explorerMaxCachedRemoteScopes" => p.explorer_auto_reconnect,
            "autocompleteProvider" | "autocompleteModelId" => p.autocomplete_enabled,
            "bookmarksActionNewTerminal"
            | "bookmarksActionCurrentTerminal"
            | "bookmarksActionCurrentSftp"
            | "bookmarksActionNewSftp"
            | "bookmarksPrimaryClickBehavior"
            | "bookmarksShowBadge" => p.bookmarks_enabled,
            "commandPaletteHistorySize" => p.command_palette_show_recent,
            _ => true,
        }
    }

    /// Render a category's fields, split into the reference sub-sections
    /// (`SECTION_GROUPS`); any field not listed in a group falls through to a
    /// trailing "Other" block so nothing is ever silently dropped.
    fn render_grouped(&self, cat: &str, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let groups = SECTION_GROUPS
            .iter()
            .find(|(g, _)| *g == cat)
            .map(|(_, g)| *g)
            .unwrap_or(&[]);
        let mut placed: Vec<&str> = Vec::new();
        let mut root = div().flex().flex_col();
        for (label, keys) in groups {
            placed.extend(keys.iter().copied());
            let defs: Vec<&FieldDef> = keys
                .iter()
                .filter_map(|k| FIELDS.iter().find(|f| f.key == *k && f.category == cat))
                .filter(|f| self.field_visible(f.key, cx))
                .collect();
            if defs.is_empty() {
                continue;
            }
            root = root.child(section_label(label, c)).children(
                defs.into_iter()
                    .map(|f| self.render_field(f, c, cx))
                    .collect::<Vec<_>>(),
            );
        }
        let leftover: Vec<&FieldDef> = FIELDS
            .iter()
            .filter(|f| {
                f.category == cat && !placed.contains(&f.key) && self.field_visible(f.key, cx)
            })
            .collect();
        if !leftover.is_empty() {
            if !groups.is_empty() {
                root = root.child(section_label("Other", c));
            }
            root = root.children(
                leftover
                    .into_iter()
                    .map(|f| self.render_field(f, c, cx))
                    .collect::<Vec<_>>(),
            );
        }
        root.into_any_element()
    }

    /// The General pane: an About hero followed by the grouped rows.
    fn render_general(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .child(self.render_about_hero(c, cx))
            .child(self.render_grouped("General", c, cx))
            .into_any_element()
    }

    fn render_about_hero(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let link = |id: &'static str, label: &'static str, url: &'static str| {
            div()
                .id(id)
                .px_2()
                .py(px(3.0))
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .text_size(px(11.5))
                .text_color(c.fg)
                .hover(|s| s.bg(c.border))
                .child(label)
                .on_click(cx.listener(move |_, _: &ClickEvent, _w, cx| {
                    cx.open_url(url);
                }))
        };
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .py_4()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .size(px(56.0))
                    .rounded_lg()
                    .bg(c.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(c.bg)
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("L"),
            )
            .child(
                div()
                    .text_color(c.fg)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Labonair"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "Version {}  \u{2022}  {} {}",
                        env!("CARGO_PKG_VERSION"),
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                    ))),
            )
            .child(
                div()
                    .mt_1()
                    .flex()
                    .gap_2()
                    .child(link(
                        "about-report",
                        "Report a problem",
                        "https://github.com/Snenjih/Labonair-rust/issues/new",
                    ))
                    .child(link(
                        "about-github",
                        "GitHub",
                        "https://github.com/Snenjih/Labonair-rust",
                    ))
                    .child(link(
                        "about-website",
                        "Website",
                        "https://github.com/Snenjih/Labonair-rust",
                    )),
            )
            .into_any_element()
    }

    /// Themes pane — a card grid over the installed themes (built-in + user
    /// `~/.config/labonair/themes/*.json`), a port of `ThemeMarketplace` /
    /// `ThemeCard`. Community fetch is not wired (documented in T16-012).
    fn render_themes(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active_id = self.active_theme_id.clone();
        let cards: Vec<_> = self
            .theme_files
            .iter()
            .map(|t| {
                let id = t.id.clone();
                let id_del = t.id.clone();
                let is_active = active_id.as_deref() == Some(t.id.as_str())
                    || (active_id.is_none() && t.id == "default");
                let builtin = t.builtin;
                div()
                    .w(px(180.0))
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .overflow_hidden()
                    .border_1()
                    .border_color(if is_active { c.accent } else { c.border })
                    .child(
                        div()
                            .h(px(84.0))
                            .bg(c.bg)
                            .flex()
                            .items_end()
                            .p_2()
                            .gap_1()
                            .child(div().size(px(14.0)).rounded_sm().bg(c.accent))
                            .child(div().size(px(14.0)).rounded_sm().bg(c.muted))
                            .child(div().size(px(14.0)).rounded_sm().bg(c.border)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .child(
                                div()
                                    .text_color(c.fg)
                                    .text_size(px(11.5))
                                    .child(SharedString::from(t.name.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("theme-use-{}", t.id)))
                                            .px_2()
                                            .py(px(2.0))
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(c.border)
                                            .text_size(px(11.0))
                                            .text_color(c.fg)
                                            .hover(|s| s.bg(c.border))
                                            .child(if is_active { "Active" } else { "Activate" })
                                            .on_click(cx.listener(
                                                move |this, _: &ClickEvent, _w, cx| {
                                                    this.activate_theme(&id, cx);
                                                },
                                            )),
                                    )
                                    .when(!builtin, |d| {
                                        d.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "theme-del-{id_del}"
                                                )))
                                                .px_2()
                                                .py(px(2.0))
                                                .rounded_sm()
                                                .border_1()
                                                .border_color(c.border)
                                                .text_size(px(11.0))
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
                            ),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .child(
                div()
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
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.import_theme(cx)),
                            ),
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
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.export_theme(cx)),
                            ),
                    )
                    .child(
                        div()
                            .id("theme-folder")
                            .px_2()
                            .py(px(3.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(c.border)
                            .text_color(c.fg)
                            .hover(|s| s.bg(c.border))
                            .child("Open themes folder")
                            .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                                cx.reveal_path(&themes_dir());
                            })),
                    ),
            )
            .child(div().flex().flex_wrap().gap_3().py_2().children(cards))
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
        if !self.windowed && !self.open {
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

        let search_box = div()
            .mb_2()
            .px_2()
            .py(px(4.0))
            .rounded_sm()
            .border_1()
            .border_color(if searching { c.accent } else { c.border })
            .bg(c.bg)
            .text_size(px(11.5))
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

        let sidebar = div()
            .w(px(208.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(c.border)
            .child(search_box)
            .children(CATEGORIES.iter().enumerate().map(|(i, name)| {
                let is_active = i == active_cat && !searching;
                div()
                    .id(SharedString::from(*name))
                    .px_2()
                    .py(px(5.0))
                    .rounded_sm()
                    .text_size(px(12.0))
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

        let body = self.render_body(&c, cx);
        let windowed = self.windowed;

        let header = div()
            .h(px(44.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .id("settings-open-json")
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("Open settings.json")
                    .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                        cx.reveal_path(&config_dir().join("labonair-settings.json"));
                    })),
            )
            .child(
                div()
                    .text_color(c.fg)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_size(px(12.5))
                    .child("Settings"),
            )
            .child(
                div()
                    .id("settings-close")
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("\u{2715}")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.request_close(window, cx)
                    })),
            );

        let content = div().flex_1().min_h_0().flex().child(sidebar).child(
            div()
                .id("settings-scroll")
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .items_center()
                .p_4()
                .overflow_y_scroll()
                .child(
                    div()
                        .w_full()
                        .max_w(px(580.0))
                        .flex()
                        .flex_col()
                        .child(body),
                ),
        );

        let card = div()
            .id("settings-card")
            .track_focus(&self.focus)
            .key_context("Settings")
            .flex()
            .flex_col()
            .bg(c.card)
            .text_color(c.fg)
            .on_key_down(cx.listener(Self::on_key))
            .child(header)
            .child(content)
            .children(self.render_dropdown(&c, cx));

        if windowed {
            return card.size_full().into_any_element();
        }

        // Legacy in-`AppShell` modal path (kept for tests only).
        div()
            .id("settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.close(cx)))
            .child(
                card.w(px(820.0))
                    .h(px(560.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(c.border)
                    .overflow_hidden()
                    .on_click(|_, _w, cx| cx.stop_propagation()),
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

    #[test]
    fn capture_free_binding_sets_override() {
        match capture_keybind(&KeybindMap::new(), ShortcutId::TabNew, "cmd-shift-y") {
            KbCapture::Set(m) => {
                assert_eq!(m.get("tab.new").map(String::as_str), Some("cmd-shift-y"))
            }
            _ => panic!("expected a free binding"),
        }
    }

    #[test]
    fn capture_detects_conflict_then_overwrite_unbinds_loser() {
        let map = KeybindMap::new();
        match capture_keybind(&map, ShortcutId::CommandPalette, "cmd-t") {
            KbCapture::Conflict(other) => assert_eq!(other, ShortcutId::TabNew),
            _ => panic!("cmd-t should collide with TabNew"),
        }
        let next = overwrite_keybind(
            &map,
            ShortcutId::CommandPalette,
            ShortcutId::TabNew,
            "cmd-t",
        );
        assert_eq!(
            next.get("command.palette").map(String::as_str),
            Some("cmd-t")
        );
        assert_eq!(next.get("tab.new").map(String::as_str), Some(""));
        assert_eq!(effective_binding(ShortcutId::TabNew, &next), None);
        // No silent double-binding — cmd-t has exactly one owner now.
        assert_eq!(
            resolve_conflict("cmd-t", None, &next),
            Some(Conflict::Shortcut(ShortcutId::CommandPalette))
        );
    }

    #[test]
    fn capture_refuses_reserved_accelerator() {
        assert!(matches!(
            capture_keybind(&KeybindMap::new(), ShortcutId::TabNew, "cmd-,"),
            KbCapture::Reserved("Settings")
        ));
    }

    #[test]
    fn keyboard_category_is_registered() {
        assert!(CATEGORIES.contains(&KEYBOARD));
    }

    #[gpui::test]
    fn keybinds_persist_and_reset(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        let mut m = KeybindMap::new();
        m.insert("tab.new".into(), "cmd-shift-t".into());
        store.update(cx, |s, cx| {
            s.set_value("keybinds", serde_json::to_value(&m).unwrap(), cx);
        });
        cx.run_until_parked();
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .keybinds
                .get("tab.new")
                .map(String::as_str),
            Some("cmd-shift-t")
        );
        // Reset all → empty map persists across a reload.
        store.update(cx, |s, cx| {
            s.set_value("keybinds", serde_json::json!({}), cx);
        });
        cx.run_until_parked();
        assert!(PreferencesStore::with_dir(dir.clone())
            .get()
            .keybinds
            .is_empty());
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
