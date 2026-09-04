//! Static settings field definitions: `SettingsTab`, `FieldKind`, `FieldDef`,
//! the top-level `CATEGORIES`, and the table-driven `FIELDS` list. Split out of
//! the old `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move —
//! no logic change).

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
    pub(crate) fn category_index(self) -> usize {
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
    /// Float stepper (line-height, letter-spacing, temperature). `step` is
    /// expressed in hundredths to keep the type `Copy`/`i64`.
    Float {
        min_centi: i64,
        max_centi: i64,
        step_centi: i64,
    },
    /// The options are the exact serialized token strings.
    Select(&'static [&'static str]),
    /// A font-family picker — a dropdown populated from the scanned system
    /// fonts plus a `(default)` entry.
    FontFamily,
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

#[allow(clippy::enum_glob_use)]
use FieldKind::Float;
use FieldKind::{FontFamily, Int, Select, Switch, Text};

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
        "What opens on launch when there is no session to restore.",
        "General",
        Select(&["terminal", "empty"]),
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
        FontFamily,
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
        FontFamily,
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
        FontFamily,
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
    d(
        "commandPalettePosition",
        "Position",
        "Where the palette opens vertically.",
        "Workspace",
        Select(&["top", "high", "center"]),
    ),
    d(
        "commandPaletteOpacity",
        "Card opacity",
        "Palette card opacity (%).",
        "Workspace",
        Int {
            min: 35,
            max: 100,
            step: 5,
        },
    ),
    d(
        "commandPaletteHistorySize",
        "Recent history size",
        "How many recently-run commands to remember.",
        "Workspace",
        Int {
            min: 0,
            max: 20,
            step: 1,
        },
    ),
    d(
        "commandPaletteCloseOnOverlayClick",
        "Close on click-away",
        "Dismiss the palette when clicking outside the card.",
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
    // ── Float rows (added T16-010) ──────────────────────────────────────
    d(
        "appLineHeight",
        "UI line height",
        "Line height multiplier for application text.",
        CAT_APPEARANCE,
        Float { min_centi: 100, max_centi: 200, step_centi: 5 },
    ),
    d(
        "editorLineHeight",
        "Line height",
        "Editor line height multiplier.",
        "Editor",
        Float { min_centi: 100, max_centi: 300, step_centi: 5 },
    ),
    d(
        "terminalLineHeight",
        "Line height",
        "Terminal line height multiplier.",
        "Terminal",
        Float { min_centi: 80, max_centi: 200, step_centi: 5 },
    ),
    d(
        "terminalLetterSpacing",
        "Letter spacing",
        "Extra horizontal spacing between glyphs, in pixels.",
        "Terminal",
        Float { min_centi: -200, max_centi: 1000, step_centi: 50 },
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
