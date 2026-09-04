//! Command registry + the [`CommandPalette`] modal overlay view.
//!
//! * **Data** — a static [`Command`] registry (id / title / section /
//!   contexts / optional shortcut) plus pure filtering / search helpers. All
//!   unit-tested, no GPUI needed.
//! * **View** — [`CommandPalette`], a modal overlay opened with `Cmd+P`. Type
//!   to filter, arrow keys to move, `Enter` to run, `Esc` to close. Commands
//!   that need an argument ("Switch Tab\u{2026}") push a follow-up page.
//!
//! Execution: the palette does not own the app state — on `Enter` it emits
//! [`PaletteEvent`], which the host shell turns into either a GPUI action
//! dispatch or a direct workspace call. The host wires the palette to its
//! concrete stores through the [`PalettePrefs`] / [`PaletteWorkspace`] /
//! [`UiTheme`] contracts.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use labonair_theme::{EditorThemeId, ThemePreference};
use labonair_ui_kit::{IconName, UiTheme};

use crate::fuzzy::{match_score, SearchMode};
use crate::keybind::{effective_keys, KeybindMap, ShortcutId};

// ─────────────────────────────────────────────────────────────────────────────
// Host contracts (decoupling — the palette crate never names `crates/ui`)
// ─────────────────────────────────────────────────────────────────────────────

/// The tab-kind surface [`context_of`] maps to a [`CommandContext`]. The
/// palette crate owns this enum so it never has to name `crates/ui`'s
/// `TabKind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaletteTabKind {
    Workspace,
    Editor,
    Sftp,
    Home,
    /// Any other tab kind — maps to no context.
    Other,
}

/// One open tab, as the host workspace exposes it for the `Switch Tab\u{2026}`
/// sub-page.
#[derive(Clone, Debug)]
pub struct PaletteTabRow {
    pub id: u64,
    pub label: String,
    pub kind_title: String,
    pub is_ssh: bool,
}

/// The preference surface the palette view reads. Implemented for the host
/// app's preferences store in `crates/ui`.
pub trait PalettePrefs {
    fn command_palette_search_mode(&self) -> SearchMode;
    fn command_palette_history_size(&self) -> u32;
    fn command_palette_opacity(&self) -> u32;
    fn command_palette_position(&self) -> String;
    fn command_palette_show_recent(&self) -> bool;
    fn command_palette_close_on_overlay_click(&self) -> bool;
    /// Persist a new search mode (`set_value("commandPaletteSearchMode", …)`).
    fn set_command_palette_search_mode(&mut self, mode: SearchMode, cx: &mut Context<Self>)
    where
        Self: Sized;

    // ── Live state the palette used to receive through `PaletteData` ──────
    // Read straight off the preferences store now (T17-007) — no per-open
    // `build_palette_data` snapshot for these pref/theme-derived values.
    /// Current app color-mode preference (`Change Color Mode…` sub-page state).
    fn color_mode(&self) -> ThemePreference;
    /// Active editor theme (`Change Editor Theme…` sub-page state).
    fn editor_theme(&self) -> EditorThemeId;
    /// Terminal font size, shown as the `Adjust Font Size…` subtitle.
    fn terminal_font_size(&self) -> u32;
    /// Current value of the boolean preference `key` flips (`Toggle: …` rows).
    fn toggle_state(&self, key: &str) -> bool;
    /// User keybind overrides, for rendering `effective_binding` hints.
    /// Takes `cx` (T19-008): the host now derives this from the
    /// `keymap.json`-backed `KeybindDisplay` GPUI global rather than a
    /// `Preferences` field.
    fn keybind_overrides(&self, cx: &App) -> KeybindMap;
}

/// The workspace surface the palette view reads.
pub trait PaletteWorkspace {
    /// The active tab's [`CommandContext`], if it maps to one.
    fn palette_active_context(&self, cx: &App) -> Option<CommandContext>;
    /// Every open tab, for the `Switch Tab\u{2026}` sub-page.
    fn palette_tab_rows(&self, cx: &App) -> Vec<PaletteTabRow>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Command registry (port of command-palette/*)
// ─────────────────────────────────────────────────────────────────────────────

/// The surface the active tab exposes — drives which context-scoped
/// commands the palette offers. Port of the reference `CommandContext`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandContext {
    Terminal,
    Editor,
    Sftp,
    Home,
    SshTerminal,
}

/// Every palette command. New domains/phases add a variant + a [`COMMANDS`]
/// row + an arm in the host's `run_palette_command`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum CommandId {
    NewTerminalTab,
    NewEditorTab,
    DuplicateTab,
    CloseOtherTabs,
    SplitRight,
    SplitDown,
    ClosePane,
    CloseTab,
    NextTab,
    PrevTab,
    /// Opens the tab-switcher follow-up page.
    SwitchTab,
    Find,
    ToggleSidebar,
    ToggleFullScreen,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleAiPanel,
    AskSelection,
    NewAiSession,
    OpenSnippetsPanel,
    OpenGitGraph,
    FocusSourceControl,
    OpenHostManager,
    ClearTerminal,
    OpenShortcuts,
    OpenSettings,
    /// Create (if missing) and open `<active pane's cwd>/.labonair/
    /// settings.json` — the per-project settings layer (T19-003).
    OpenProjectSettings,
    /// Create (if missing) and open `~/.config/labonair/labonair-settings.json`
    /// as an editor tab — the raw JSON path, alongside the Settings UI
    /// (T19-005).
    OpenSettingsJson,
    CheckForUpdates,
    FormatDocument,
    /// Opens the path-bookmarks popover (T12-003).
    OpenPathBookmarks,
    /// Zen-mode toggles (T13-005) — mirror `useSettingsCommands.ts`.
    ToggleZenModeHeader,
    ToggleZenModeStatusbar,
    ToggleZenMode,
    // Block D — sub-page navigators + extra settings toggles.
    AdjustFontSize,
    ConnectSsh,
    OpenSftp,
    ChangeAppTheme,
    ChangeColorMode,
    ChangeEditorTheme,
    SwitchAiSession,
    RunSnippet,
    GitSwitchBranch,
    GoToSymbol,
    /// Opens the "show hidden status-bar item" follow-up page (T18-005) —
    /// the palette-side escape hatch for items the user hid via the
    /// statusbar's right-click menu.
    ShowStatusBarItem,
    OpenAiSettings,
    ToggleEditorWordWrap,
    ToggleLineNumbers,
    ToggleFormatOnSave,
    ToggleCursorBlink,
    TogglePaneHeader,
    TogglePaneFooter,
    ToggleVimMode,
    // ── Menu / keyboard-only ids (no `COMMANDS` row) ──────────────────────
    // Dispatched through the shell's `CommandRegistry` (T17-007) from the
    // native menu bar + key bindings; they never appear as palette rows.
    OpenCommandPalette,
    NewPreviewTab,
    Save,
    NewSshTab,
    NewSftpTab,
    NewSshConnection,
    NewQuickSsh,
    ClearChat,
    FocusNextPane,
    SelectTab1,
    SelectTab2,
    SelectTab3,
    SelectTab4,
    SelectTab5,
    SelectTab6,
    SelectTab7,
    SelectTab8,
    SelectTab9,
    DebugCyclePanelDock,
    DebugToggleDockZoom,
    /// Open `keymap.json` as an editor tab (T19-008) — mirrors
    /// `OpenSettingsJson`/`OpenProjectSettings`.
    OpenKeymapJson,
}

/// `(CommandId, "<namespace>::<Name>")` — the action-name vocabulary
/// `keymap.json` bindings reference (T19-008). One entry per [`CommandId`]
/// variant; [`CommandId::action_name`] / [`CommandId::from_action_name`] are
/// built from this single table so the two directions can't drift.
#[rustfmt::skip]
const ACTION_NAMES: &[(CommandId, &str)] = &[
    (CommandId::NewTerminalTab, "tab::NewTerminal"),
    (CommandId::NewEditorTab, "tab::NewEditor"),
    (CommandId::NewPreviewTab, "tab::NewPreview"),
    (CommandId::NewSshTab, "tab::NewSsh"),
    (CommandId::NewSftpTab, "tab::NewSftp"),
    (CommandId::DuplicateTab, "tab::Duplicate"),
    (CommandId::CloseOtherTabs, "tab::CloseOthers"),
    (CommandId::CloseTab, "tab::Close"),
    (CommandId::NextTab, "tab::Next"),
    (CommandId::PrevTab, "tab::Prev"),
    (CommandId::SwitchTab, "tab::Switch"),
    (CommandId::SelectTab1, "tab::Select1"),
    (CommandId::SelectTab2, "tab::Select2"),
    (CommandId::SelectTab3, "tab::Select3"),
    (CommandId::SelectTab4, "tab::Select4"),
    (CommandId::SelectTab5, "tab::Select5"),
    (CommandId::SelectTab6, "tab::Select6"),
    (CommandId::SelectTab7, "tab::Select7"),
    (CommandId::SelectTab8, "tab::Select8"),
    (CommandId::SelectTab9, "tab::Select9"),
    (CommandId::Save, "tab::Save"),
    (CommandId::SplitRight, "pane::SplitRight"),
    (CommandId::SplitDown, "pane::SplitDown"),
    (CommandId::ClosePane, "pane::Close"),
    (CommandId::FocusNextPane, "pane::FocusNext"),
    (CommandId::ClearTerminal, "terminal::Clear"),
    (CommandId::Find, "search::Toggle"),
    (CommandId::ToggleSidebar, "sidebar::Toggle"),
    (CommandId::ToggleFullScreen, "view::ToggleFullScreen"),
    (CommandId::ZoomIn, "view::ZoomIn"),
    (CommandId::ZoomOut, "view::ZoomOut"),
    (CommandId::ZoomReset, "view::ZoomReset"),
    (CommandId::AdjustFontSize, "view::AdjustFontSize"),
    (CommandId::ChangeAppTheme, "view::ChangeAppTheme"),
    (CommandId::ChangeColorMode, "view::ChangeColorMode"),
    (CommandId::ChangeEditorTheme, "view::ChangeEditorTheme"),
    (CommandId::ToggleZenMode, "view::ToggleZenMode"),
    (CommandId::ToggleZenModeHeader, "view::ToggleZenModeHeader"),
    (CommandId::ToggleZenModeStatusbar, "view::ToggleZenModeStatusbar"),
    (CommandId::ShowStatusBarItem, "view::ShowStatusBarItem"),
    (CommandId::ToggleEditorWordWrap, "editor::ToggleWordWrap"),
    (CommandId::ToggleLineNumbers, "editor::ToggleLineNumbers"),
    (CommandId::ToggleFormatOnSave, "editor::ToggleFormatOnSave"),
    (CommandId::FormatDocument, "editor::FormatDocument"),
    (CommandId::GoToSymbol, "editor::GoToSymbol"),
    (CommandId::ToggleVimMode, "editor::ToggleVimMode"),
    (CommandId::ToggleCursorBlink, "terminal::ToggleCursorBlink"),
    (CommandId::TogglePaneHeader, "terminal::TogglePaneHeader"),
    (CommandId::TogglePaneFooter, "terminal::TogglePaneFooter"),
    (CommandId::ToggleAiPanel, "ai::TogglePanel"),
    (CommandId::AskSelection, "ai::AskSelection"),
    (CommandId::NewAiSession, "ai::NewSession"),
    (CommandId::SwitchAiSession, "ai::SwitchSession"),
    (CommandId::ClearChat, "ai::ClearChat"),
    (CommandId::RunSnippet, "snippets::Run"),
    (CommandId::OpenSnippetsPanel, "snippets::OpenPanel"),
    (CommandId::OpenGitGraph, "git::OpenGraph"),
    (CommandId::FocusSourceControl, "git::FocusSourceControl"),
    (CommandId::GitSwitchBranch, "git::SwitchBranch"),
    (CommandId::OpenHostManager, "connections::OpenHostManager"),
    (CommandId::NewSshConnection, "connections::NewSshConnection"),
    (CommandId::NewQuickSsh, "connections::NewQuickSsh"),
    (CommandId::ConnectSsh, "connections::Connect"),
    (CommandId::OpenSftp, "connections::OpenSftp"),
    (CommandId::OpenPathBookmarks, "bookmarks::Open"),
    (CommandId::OpenCommandPalette, "command_palette::Toggle"),
    (CommandId::OpenShortcuts, "settings::OpenShortcuts"),
    (CommandId::OpenSettings, "settings::Open"),
    (CommandId::OpenAiSettings, "settings::OpenAi"),
    (CommandId::OpenProjectSettings, "settings::OpenProjectJson"),
    (CommandId::OpenSettingsJson, "settings::OpenUserJson"),
    (CommandId::OpenKeymapJson, "zed::OpenKeymap"),
    (CommandId::CheckForUpdates, "app::CheckForUpdates"),
    (CommandId::DebugCyclePanelDock, "debug::CyclePanelDock"),
    (CommandId::DebugToggleDockZoom, "debug::ToggleDockZoom"),
];

impl CommandId {
    /// The `<namespace>::<Name>` string a `keymap.json` binding names this
    /// command by. Every variant has exactly one entry in [`ACTION_NAMES`]
    /// (enforced by `tests::every_command_id_has_a_unique_action_name`).
    pub fn action_name(self) -> &'static str {
        ACTION_NAMES
            .iter()
            .find(|(id, _)| *id == self)
            .map(|(_, name)| *name)
            .unwrap_or_else(|| panic!("CommandId::{self:?} has no ACTION_NAMES entry"))
    }

    /// Reverse of [`Self::action_name`] — resolves a `keymap.json` action
    /// string back to the [`CommandId`] it dispatches, or `None` if the
    /// action name is unknown.
    pub fn from_action_name(name: &str) -> Option<Self> {
        ACTION_NAMES
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(id, _)| *id)
    }
}

/// Every valid `keymap.json` action name — the "known actions" set
/// `labonair_settings::keymap::validate_keymap` needs, without that pure
/// crate having to depend on this one (T19-008).
pub fn known_action_names() -> std::collections::BTreeSet<&'static str> {
    ACTION_NAMES.iter().map(|(_, name)| *name).collect()
}

/// The camelCase preference key a `Toggle: …` command flips, if any.
pub fn toggle_pref_key(id: CommandId) -> Option<&'static str> {
    Some(match id {
        CommandId::ToggleZenModeHeader => "zenModeShowHeader",
        CommandId::ToggleZenModeStatusbar => "zenModeShowStatusbar",
        CommandId::ToggleEditorWordWrap => "editorWordWrap",
        CommandId::ToggleLineNumbers => "editorLineNumbers",
        CommandId::ToggleFormatOnSave => "editorFormatOnSave",
        CommandId::ToggleCursorBlink => "terminalCursorBlink",
        CommandId::TogglePaneHeader => "terminalShowPaneHeader",
        CommandId::TogglePaneFooter => "terminalShowPaneFooter",
        CommandId::ToggleVimMode => "vimMode",
        _ => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Sub-pages (port of the reference `CommandPage` registry — 11 named pages)
// ─────────────────────────────────────────────────────────────────────────────

/// Every palette page. `Root` is the command list; the rest are the
/// context-drill sub-pages the reference registers in `useCommandRegistry`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Page {
    Root,
    Zoom,
    Tabs,
    ColorMode,
    EditorTheme,
    Themes,
    Hosts,
    Snippets,
    AiSessions,
    Outline,
    GitBranches,
    StatusBarHidden,
}

impl Page {
    /// The search-input placeholder for this page.
    pub fn placeholder(self) -> &'static str {
        match self {
            Page::Root => "Search commands\u{2026}",
            Page::Zoom => "Adjust font size\u{2026}",
            Page::Tabs => "Search open tabs\u{2026}",
            Page::ColorMode => "Search color modes\u{2026}",
            Page::EditorTheme => "Search editor themes\u{2026}",
            Page::Themes => "Search themes\u{2026}",
            Page::Hosts => "Search hosts\u{2026}",
            Page::Snippets => "Search snippets\u{2026}",
            Page::AiSessions => "Search sessions\u{2026}",
            Page::Outline => "Search symbols\u{2026}",
            Page::GitBranches => "Search branches\u{2026}",
            Page::StatusBarHidden => "Search hidden items\u{2026}",
        }
    }

    /// The breadcrumb label for this page.
    pub fn label(self) -> &'static str {
        match self {
            Page::Root => "Commands",
            Page::Zoom => "Font Size",
            Page::Tabs => "Open Tabs",
            Page::ColorMode => "Color Mode",
            Page::EditorTheme => "Editor Theme",
            Page::Themes => "App Theme",
            Page::Hosts => "Hosts",
            Page::Snippets => "Snippets",
            Page::AiSessions => "AI Sessions",
            Page::Outline => "Symbols",
            Page::GitBranches => "Branches",
            Page::StatusBarHidden => "Hidden Status Bar Items",
        }
    }
}

pub struct Command {
    pub id: CommandId,
    pub title: &'static str,
    pub section: &'static str,
    /// Empty = always available; otherwise only when the active context is
    /// listed (reference `filterByContext`).
    pub contexts: &'static [CommandContext],
    /// Right-aligned shortcut hint, if the command has a bound shortcut.
    pub shortcut: Option<ShortcutId>,
    /// Leading icon (reference renders a Hugeicons glyph on every row).
    pub icon: IconName,
    /// If set, picking this row navigates to a sub-page instead of running.
    pub sub_page: Option<Page>,
}

use CommandContext::{Editor as CtxEditor, SshTerminal, Terminal as CtxTerminal};
use IconName as I;
use ShortcutId::*;

#[rustfmt::skip]
static COMMANDS: &[Command] = &[
    Command { id: CommandId::NewTerminalTab,     title: "New Terminal Tab",        section: "Layout",         contexts: &[],                            shortcut: Some(TabNew),        icon: I::Terminal,   sub_page: None },
    Command { id: CommandId::NewEditorTab,       title: "New Editor Tab",          section: "Layout",         contexts: &[],                            shortcut: Some(TabNewEditor),  icon: I::File,       sub_page: None },
    Command { id: CommandId::DuplicateTab,       title: "Duplicate Tab",           section: "Layout",         contexts: &[],                            shortcut: None,                icon: I::Copy,       sub_page: None },
    Command { id: CommandId::CloseOtherTabs,     title: "Close Other Tabs",        section: "Layout",         contexts: &[],                            shortcut: None,                icon: I::X,          sub_page: None },
    Command { id: CommandId::SwitchTab,          title: "Switch Tab\u{2026}",      section: "Layout",         contexts: &[],                            shortcut: None,                icon: I::Terminal,   sub_page: Some(Page::Tabs) },
    Command { id: CommandId::AdjustFontSize,     title: "Adjust Font Size\u{2026}",section: "Layout",         contexts: &[CtxTerminal, CtxEditor],      shortcut: None,                icon: I::ArrowDownUp,sub_page: Some(Page::Zoom) },
    Command { id: CommandId::SplitRight,         title: "Split Pane Right",        section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneSplitRight),icon: I::ChevronRight,sub_page: None },
    Command { id: CommandId::SplitDown,          title: "Split Pane Down",         section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneSplitDown), icon: I::ChevronDown,sub_page: None },
    Command { id: CommandId::ClosePane,          title: "Close Active Pane",       section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneClose),     icon: I::X,          sub_page: None },
    Command { id: CommandId::CloseTab,           title: "Close Current Tab",       section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabClose),      icon: I::X,          sub_page: None },
    Command { id: CommandId::NextTab,            title: "Next Tab",                section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabNext),       icon: I::ChevronRight,sub_page: None },
    Command { id: CommandId::PrevTab,            title: "Previous Tab",            section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabPrev),       icon: I::ChevronRight,sub_page: None },
    Command { id: CommandId::ClearTerminal,      title: "Clear Terminal",          section: "Terminal",       contexts: &[CtxTerminal, SshTerminal],    shortcut: None,                icon: I::Trash,      sub_page: None },
    Command { id: CommandId::OpenHostManager,    title: "Open Host Manager",       section: "Connections",    contexts: &[],                            shortcut: None,                icon: I::Server,     sub_page: None },
    Command { id: CommandId::ConnectSsh,         title: "Connect SSH\u{2026}",     section: "Connections",    contexts: &[],                            shortcut: None,                icon: I::Terminal,   sub_page: Some(Page::Hosts) },
    Command { id: CommandId::OpenSftp,           title: "Open SFTP\u{2026}",       section: "Connections",    contexts: &[],                            shortcut: None,                icon: I::Folder,     sub_page: Some(Page::Hosts) },
    Command { id: CommandId::Find,               title: "Find in Current Pane",    section: "Search",         contexts: &[],                            shortcut: Some(SearchFocus),   icon: I::Search,     sub_page: None },
    Command { id: CommandId::ToggleSidebar,      title: "Toggle File Explorer",    section: "View",           contexts: &[],                            shortcut: Some(SidebarToggle), icon: I::PanelLeft,  sub_page: None },
    Command { id: CommandId::ToggleFullScreen,   title: "Toggle Full Screen",      section: "View",           contexts: &[],                            shortcut: None,                icon: I::Square,     sub_page: None },
    Command { id: CommandId::ChangeAppTheme,     title: "Change App Theme\u{2026}",section: "View",           contexts: &[],                            shortcut: None,                icon: I::Sparkles,   sub_page: Some(Page::Themes) },
    Command { id: CommandId::ChangeColorMode,    title: "Change Color Mode\u{2026}",section: "View",          contexts: &[],                            shortcut: None,                icon: I::ArrowDownUp,sub_page: Some(Page::ColorMode) },
    Command { id: CommandId::ChangeEditorTheme,  title: "Change Editor Theme\u{2026}",section: "View",        contexts: &[CtxEditor],                   shortcut: None,                icon: I::Sparkles,   sub_page: Some(Page::EditorTheme) },
    Command { id: CommandId::ZoomIn,             title: "Zoom In",                 section: "View",           contexts: &[],                            shortcut: Some(ViewZoomIn),    icon: I::Plus,       sub_page: None },
    Command { id: CommandId::ZoomOut,            title: "Zoom Out",                section: "View",           contexts: &[],                            shortcut: Some(ViewZoomOut),   icon: I::Minus,      sub_page: None },
    Command { id: CommandId::ZoomReset,          title: "Reset Zoom",              section: "View",           contexts: &[],                            shortcut: Some(ViewZoomReset), icon: I::Refresh,    sub_page: None },
    Command { id: CommandId::ToggleAiPanel,      title: "Toggle AI Panel",         section: "AI",             contexts: &[],                            shortcut: Some(AiToggle),      icon: I::Sparkles,   sub_page: None },
    Command { id: CommandId::AskSelection,       title: "Ask AI About Selection",  section: "AI",             contexts: &[],                            shortcut: Some(AiAskSelection),icon: I::Sparkles,   sub_page: None },
    Command { id: CommandId::NewAiSession,       title: "New AI Session",          section: "AI",             contexts: &[],                            shortcut: None,                icon: I::Refresh,    sub_page: None },
    Command { id: CommandId::SwitchAiSession,    title: "Switch AI Session\u{2026}",section: "AI",            contexts: &[],                            shortcut: None,                icon: I::Sparkles,   sub_page: Some(Page::AiSessions) },
    Command { id: CommandId::OpenSnippetsPanel,  title: "Open Snippets Panel",     section: "Snippets",       contexts: &[],                            shortcut: None,                icon: I::Command,    sub_page: None },
    Command { id: CommandId::RunSnippet,         title: "Run Snippet\u{2026}",     section: "Snippets",       contexts: &[],                            shortcut: None,                icon: I::Command,    sub_page: Some(Page::Snippets) },
    Command { id: CommandId::OpenPathBookmarks,  title: "Open Path Bookmarks",     section: "Bookmarks",      contexts: &[],                            shortcut: Some(BookmarksOpen), icon: I::Bookmark,   sub_page: None },
    Command { id: CommandId::OpenGitGraph,       title: "Open Git Graph",          section: "Source Control", contexts: &[],                            shortcut: None,                icon: I::GitBranch,  sub_page: None },
    Command { id: CommandId::FocusSourceControl, title: "Focus Source Control",    section: "Source Control", contexts: &[],                            shortcut: None,                icon: I::GitBranch,  sub_page: None },
    Command { id: CommandId::GitSwitchBranch,    title: "Git: Switch Branch\u{2026}",section: "Source Control",contexts: &[],                          shortcut: None,                icon: I::GitBranch,  sub_page: Some(Page::GitBranches) },
    Command { id: CommandId::FormatDocument,     title: "Format Document",         section: "Editor",         contexts: &[CtxEditor],                   shortcut: None,                icon: I::SquarePen,  sub_page: None },
    Command { id: CommandId::GoToSymbol,         title: "Go to Symbol\u{2026}",    section: "Editor",         contexts: &[CtxEditor],                   shortcut: None,                icon: I::FileCode,   sub_page: Some(Page::Outline) },
    Command { id: CommandId::ShowStatusBarItem,  title: "Statusbar: Show Hidden Item\u{2026}", section: "View", contexts: &[],                           shortcut: None,                icon: I::Eye,        sub_page: Some(Page::StatusBarHidden) },
    Command { id: CommandId::ToggleZenModeHeader,    title: "Toggle: Show Header Bar",   section: "Settings",  contexts: &[],                           shortcut: None,                icon: I::Eye,        sub_page: None },
    Command { id: CommandId::ToggleZenModeStatusbar, title: "Toggle: Show Status Bar",   section: "Settings",  contexts: &[],                           shortcut: None,                icon: I::Eye,        sub_page: None },
    Command { id: CommandId::ToggleZenMode,          title: "Toggle: Zen Mode",         section: "Settings",  contexts: &[],                           shortcut: Some(ViewZenMode),   icon: I::Eye,        sub_page: None },
    Command { id: CommandId::ToggleEditorWordWrap,   title: "Toggle: Editor Word Wrap",  section: "Settings",  contexts: &[CtxEditor],                   shortcut: None,               icon: I::ArrowDownUp,sub_page: None },
    Command { id: CommandId::ToggleLineNumbers,      title: "Toggle: Line Numbers",     section: "Settings",  contexts: &[CtxEditor],                   shortcut: None,               icon: I::SquareCheck,sub_page: None },
    Command { id: CommandId::ToggleFormatOnSave,     title: "Toggle: Format on Save",   section: "Settings",  contexts: &[CtxEditor],                   shortcut: None,               icon: I::SquareCheck,sub_page: None },
    Command { id: CommandId::ToggleCursorBlink,      title: "Toggle: Terminal Cursor Blink", section: "Settings", contexts: &[CtxTerminal],             shortcut: None,               icon: I::Eye,        sub_page: None },
    Command { id: CommandId::TogglePaneHeader,       title: "Toggle: Terminal Pane Header",   section: "Settings", contexts: &[CtxTerminal],             shortcut: None,               icon: I::PanelTop,   sub_page: None },
    Command { id: CommandId::TogglePaneFooter,       title: "Toggle: Terminal Pane Footer",   section: "Settings", contexts: &[CtxTerminal],             shortcut: None,               icon: I::PanelBottom,sub_page: None },
    Command { id: CommandId::ToggleVimMode,          title: "Toggle: Vim Mode",         section: "Settings",  contexts: &[],                            shortcut: None,               icon: I::SquareCheck,sub_page: None },
    Command { id: CommandId::OpenShortcuts,      title: "Keyboard Shortcuts",      section: "Application",    contexts: &[],                            shortcut: Some(ShortcutsOpen), icon: I::SquareCheck,sub_page: None },
    Command { id: CommandId::OpenSettings,       title: "Open Settings",           section: "Application",    contexts: &[],                            shortcut: None,                icon: I::SquarePen,  sub_page: None },
    Command { id: CommandId::OpenProjectSettings,title: "Open Project Settings (.labonair/settings.json)", section: "Application", contexts: &[],       shortcut: None,                icon: I::SquarePen,  sub_page: None },
    Command { id: CommandId::OpenSettingsJson,title: "Open Settings (JSON)", section: "Application", contexts: &[],       shortcut: None,                icon: I::SquarePen,  sub_page: None },
    Command { id: CommandId::OpenAiSettings,     title: "Manage AI Keys & Models", section: "Application",    contexts: &[],                            shortcut: None,                icon: I::Sparkles,   sub_page: None },
    Command { id: CommandId::CheckForUpdates,    title: "Check for Updates\u{2026}", section: "Application",   contexts: &[],                            shortcut: None,                icon: I::Download,   sub_page: None },
];

/// The whole registry, unfiltered.
pub fn commands() -> &'static [Command] {
    COMMANDS
}

/// Look up a command by id.
pub fn command(id: CommandId) -> &'static Command {
    COMMANDS
        .iter()
        .find(|c| c.id == id)
        .expect("every CommandId has a COMMANDS entry")
}

/// Commands available in `ctx`. Port of `useCommandRegistry`'s
/// `filterByContext`: no-context commands always show; context-scoped ones
/// only when their context is active.
pub fn available(ctx: Option<CommandContext>) -> Vec<&'static Command> {
    COMMANDS
        .iter()
        .filter(|c| match ctx {
            None => c.contexts.is_empty(),
            Some(active) => c.contexts.is_empty() || c.contexts.contains(&active),
        })
        .collect()
}

/// Search over title + section, restricted to what's available in `ctx`,
/// using `mode` (substring / prefix / fuzzy). Results are ranked by score
/// (best first); ties keep registry order.
pub fn search_mode(
    query: &str,
    ctx: Option<CommandContext>,
    mode: SearchMode,
) -> Vec<&'static Command> {
    let mut scored: Vec<(i64, usize, &'static Command)> = available(ctx)
        .into_iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let hay = format!("{} {}", c.title, c.section);
            match_score(mode, &hay, query).map(|s| (s, i, c))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, _, c)| c).collect()
}

/// Back-compat substring search (used by tests and callers that don't care
/// about the configured mode).
pub fn search(query: &str, ctx: Option<CommandContext>) -> Vec<&'static Command> {
    search_mode(query, ctx, SearchMode::Contains)
}

/// Which command a keyboard shortcut triggers, if any — the palette and the
/// global shortcut handler run the *same* command for a given binding.
pub fn command_for_shortcut(id: ShortcutId) -> Option<CommandId> {
    COMMANDS
        .iter()
        .find(|c| c.shortcut == Some(id))
        .map(|c| c.id)
}

/// The active tab's [`CommandContext`], if it maps to one.
pub fn context_of(kind: PaletteTabKind, is_ssh: bool) -> Option<CommandContext> {
    Some(match kind {
        PaletteTabKind::Workspace if is_ssh => CommandContext::SshTerminal,
        PaletteTabKind::Workspace => CommandContext::Terminal,
        PaletteTabKind::Editor => CommandContext::Editor,
        PaletteTabKind::Sftp => CommandContext::Sftp,
        PaletteTabKind::Home => CommandContext::Home,
        PaletteTabKind::Other => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// View
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the user picks something in the palette. The host shell
/// handles it.
#[derive(Clone, Debug)]
pub enum PaletteEvent {
    Run(CommandId),
    SwitchToTab(u64),
    SetColorMode(ThemePreference),
    SetEditorTheme(EditorThemeId),
    /// Open an SSH terminal (`sftp = false`) or SFTP browser (`sftp = true`)
    /// tab for the given host id — picked on the `Connect SSH` / `Open SFTP`
    /// sub-pages.
    ConnectHost {
        host_id: String,
        sftp: bool,
    },
    /// Activate a JSON app theme by id (`"default"` = built-in light/dark).
    SetAppTheme(String),
    /// Live hover-preview a theme by id (`Some`) or revert (`None`) — fired as
    /// the highlight moves across the `Themes` sub-page.
    PreviewAppTheme(Option<String>),
    /// Run a saved snippet by id with its default execution mode.
    RunSnippet(String),
    /// Switch the AI panel to a chat session by id.
    SwitchAiSession(String),
    /// Check out a git branch by name.
    SwitchBranch(String),
    /// Jump the active editor's caret to a 0-based line (Go to Symbol).
    GoToLine(usize),
    /// Un-hide a status-bar item by id (T18-005).
    ShowStatusBarItem(String),
}

/// A dynamic choice rendered on a sub-page (tab, host, session, branch…).
#[derive(Clone, Debug, Default)]
pub struct PaletteChoice {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub active: bool,
}

/// Live application state the palette needs for its dynamic sub-pages and
/// `rightLabel` states. The host rebuilds this each render and hands it over
/// via [`CommandPalette::set_data`]. Domains not wired yet (hosts, snippets,
/// AI sessions, git branches, editor outline) stay empty until their block
/// lands — the pages exist and render a clean empty state meanwhile.
///
/// Slimmed in T17-007: the pref/theme-derived scalars (`color_mode`,
/// `editor_theme`, `font_size`, the toggle bools) moved to [`PalettePrefs`]
/// reads. What remains is the genuinely panel-/workspace-/settings-sourced
/// choice lists that the palette crate cannot pull itself without a crate
/// cycle (`labonair-panel-* → labonair-command-palette` back-edge,
/// `labonair-settings-ui` dependency).
#[derive(Clone, Debug, Default)]
pub struct PaletteData {
    pub hosts: Vec<PaletteChoice>,
    /// Most-recently-connected hosts (host pre-sorts by `last_connected_at`,
    /// caps at 5) — shown as quick-connect rows at the palette root.
    pub recent_hosts: Vec<PaletteChoice>,
    pub ai_sessions: Vec<PaletteChoice>,
    pub snippets: Vec<PaletteChoice>,
    pub git_branches: Vec<PaletteChoice>,
    pub symbols: Vec<PaletteChoice>,
    pub app_themes: Vec<PaletteChoice>,
    /// Status-bar items the user has hidden via the right-click menu
    /// (T18-005) — the `StatusBarHidden` page's "click to show again" list.
    pub status_bar_hidden: Vec<PaletteChoice>,
}

/// Persisted "recently used" command ids (mirrors the reference
/// `labonair-palette-recent` localStorage list). Stored as debug-formatted
/// [`CommandId`] strings in `command-palette-recent.json` in the config dir.
mod recent {
    use super::CommandId;

    fn path() -> std::path::PathBuf {
        labonair_backend::modules::fs::paths::config_dir().join("command-palette-recent.json")
    }

    pub fn load() -> Vec<CommandId> {
        let Ok(raw) = std::fs::read_to_string(path()) else {
            return Vec::new();
        };
        let ids: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
        ids.iter().filter_map(|s| from_slug(s)).collect()
    }

    pub fn save(ids: &[CommandId]) {
        let slugs: Vec<String> = ids.iter().map(|id| format!("{id:?}")).collect();
        if let Ok(json) = serde_json::to_string(&slugs) {
            let _ = std::fs::write(path(), json);
        }
    }

    fn from_slug(s: &str) -> Option<CommandId> {
        super::COMMANDS
            .iter()
            .map(|c| c.id)
            .find(|id| format!("{id:?}") == s)
    }
}

#[derive(Clone)]
enum RowKey {
    Command(CommandId),
    Navigate(Page),
    Tab(u64),
    SetColorMode(ThemePreference),
    SetEditorTheme(EditorThemeId),
    ConnectHost {
        host_id: String,
        sftp: bool,
    },
    /// Activate a JSON app theme by id (`"default"` = built-in).
    SetAppTheme(String),
    /// Run a saved snippet by id with its default execution mode.
    RunSnippet(String),
    /// Switch the AI panel to a chat session by id.
    SwitchAiSession(String),
    /// Check out a git branch by name.
    SwitchBranch(String),
    /// Jump the active editor's caret to a 0-based line (Go to Symbol).
    GoToLine(usize),
    /// Un-hide a status-bar item by id.
    ShowStatusBarItem(String),
    /// Non-actionable (empty-state placeholder line).
    Noop,
}

/// A `Shift+Enter` alternate action for a row (e.g. "Open SFTP" on a host
/// row whose primary action is "Open SSH").
#[derive(Clone)]
struct SecondaryAction {
    label: &'static str,
    key: RowKey,
}

struct PaletteRow {
    key: RowKey,
    /// Optional `Shift+Enter` action; a footer hint is shown when present.
    secondary: Option<SecondaryAction>,
    icon: Option<IconName>,
    title: String,
    subtitle: Option<String>,
    section: String,
    keys: Vec<String>,
    right_label: Option<String>,
    has_sub: bool,
}

/// The backdrop fill for the palette overlay. The reference paints every
/// `DialogOverlay` with `bg-black/30`, theme-independent.
fn modal_scrim() -> Hsla {
    gpui::black().opacity(0.30)
}

/// The Cmd+P command palette overlay.
pub struct CommandPalette<P, W, Th> {
    theme: Entity<Th>,
    workspace: Entity<W>,
    prefs: Entity<P>,
    open: bool,
    /// Navigation stack — `[Root]` at rest, pushed on drill-in.
    pages: Vec<Page>,
    query: String,
    selected: usize,
    recent: Vec<CommandId>,
    data: PaletteData,
    focus: FocusHandle,
}

impl<P, W, Th> EventEmitter<PaletteEvent> for CommandPalette<P, W, Th>
where
    P: 'static,
    W: 'static,
    Th: 'static,
{
}

/// Emitted so a hosting [`ModalLayer`](labonair_workspace::modal_layer::ModalLayer)
/// can drop the palette when it closes itself (Esc / overlay click / a pick).
impl<P, W, Th> EventEmitter<DismissEvent> for CommandPalette<P, W, Th>
where
    P: 'static,
    W: 'static,
    Th: 'static,
{
}

impl<P, W, Th> CommandPalette<P, W, Th>
where
    P: PalettePrefs + 'static,
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    pub fn new(
        theme: Entity<Th>,
        workspace: Entity<W>,
        prefs: Entity<P>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            theme,
            workspace,
            prefs,
            open: false,
            pages: vec![Page::Root],
            query: String::new(),
            selected: 0,
            recent: recent::load(),
            data: PaletteData::default(),
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Refresh the live-state snapshot (called from the host's `render`).
    pub fn set_data(&mut self, data: PaletteData) {
        self.data = data;
    }

    fn page(&self) -> Page {
        *self.pages.last().unwrap_or(&Page::Root)
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.open(window, cx);
        }
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.pages = vec![Page::Root];
        self.query.clear();
        self.selected = 0;
        window.focus(&self.focus);
        cx.notify();
    }

    /// Open the palette navigated straight to `page` (used by `Cmd+Shift+N` →
    /// the Hosts page).
    pub fn open_to_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.open(window, cx);
        if page != Page::Root {
            self.pages.push(page);
        }
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        let was_open = self.open;
        self.open = false;
        self.query.clear();
        self.pages = vec![Page::Root];
        self.selected = 0;
        cx.emit(PaletteEvent::PreviewAppTheme(None));
        if was_open {
            cx.emit(DismissEvent);
        }
        cx.notify();
    }

    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        self.pages.push(page);
        self.query.clear();
        self.selected = 0;
        self.sync_theme_preview(cx);
        cx.notify();
    }

    fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.pages.len() > 1 {
            self.pages.pop();
            self.query.clear();
            self.selected = 0;
            self.sync_theme_preview(cx);
            cx.notify();
        }
    }

    fn go_back_to(&mut self, index: usize, cx: &mut Context<Self>) {
        if index + 1 < self.pages.len() {
            self.pages.truncate(index + 1);
            self.query.clear();
            self.selected = 0;
            self.sync_theme_preview(cx);
            cx.notify();
        }
    }

    /// Emit a live theme-preview for the highlighted row on the `Themes`
    /// sub-page, or a revert (`None`) on any other page. Called on every
    /// selection / navigation change while the palette is open.
    fn sync_theme_preview(&mut self, cx: &mut Context<Self>) {
        if matches!(self.page(), Page::Themes) {
            let rows = self.rows(cx);
            if let Some(RowKey::SetAppTheme(id)) = rows.get(self.selected).map(|r| r.key.clone()) {
                cx.emit(PaletteEvent::PreviewAppTheme(Some(id)));
                return;
            }
        }
        cx.emit(PaletteEvent::PreviewAppTheme(None));
    }

    fn active_context(&self, cx: &App) -> Option<CommandContext> {
        self.workspace.read(cx).palette_active_context(cx)
    }

    fn search_mode(&self, cx: &App) -> SearchMode {
        self.prefs.read(cx).command_palette_search_mode()
    }

    fn push_recent(&mut self, id: CommandId, cx: &App) {
        let max = self.prefs.read(cx).command_palette_history_size().max(1) as usize;
        self.recent.retain(|&r| r != id);
        self.recent.insert(0, id);
        self.recent.truncate(max);
        recent::save(&self.recent);
    }

    /// Rows for a dynamic list of choices, filtered by the current query.
    fn choice_rows(
        &self,
        choices: &[PaletteChoice],
        section: &str,
        icon: IconName,
        mode: SearchMode,
        empty_hint: &str,
        key_for: fn(&PaletteChoice) -> RowKey,
    ) -> Vec<PaletteRow> {
        if choices.is_empty() {
            return vec![PaletteRow {
                key: RowKey::Noop,
                secondary: None,
                icon: None,
                title: empty_hint.to_string(),
                subtitle: None,
                section: section.to_string(),
                keys: vec![],
                right_label: None,
                has_sub: false,
            }];
        }
        let mut scored: Vec<(i64, usize, &PaletteChoice)> = choices
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let hay = format!("{} {}", c.title, c.subtitle.as_deref().unwrap_or(""));
                match_score(mode, &hay, &self.query).map(|s| (s, i, c))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        scored
            .into_iter()
            .map(|(_, _, c)| PaletteRow {
                key: key_for(c),
                secondary: None,
                icon: Some(icon),
                title: c.title.clone(),
                subtitle: c.subtitle.clone(),
                section: section.to_string(),
                keys: vec![],
                right_label: c.active.then(|| "active".to_string()),
                has_sub: false,
            })
            .collect()
    }

    /// Host rows for the `Hosts` page and the root quick-connect section:
    /// primary action opens an SSH terminal, `Shift+Enter` opens SFTP. Port of
    /// `reference-src/src/modules/command-palette/hooks/useHostCommands.ts`.
    fn host_rows(
        &self,
        hosts: &[PaletteChoice],
        section: &str,
        mode: SearchMode,
    ) -> Vec<PaletteRow> {
        if hosts.is_empty() {
            return vec![PaletteRow {
                key: RowKey::Noop,
                secondary: None,
                icon: None,
                title: "No hosts configured yet".to_string(),
                subtitle: None,
                section: section.to_string(),
                keys: vec![],
                right_label: None,
                has_sub: false,
            }];
        }
        let mut scored: Vec<(i64, usize, &PaletteChoice)> = hosts
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let hay = format!("{} {}", c.title, c.subtitle.as_deref().unwrap_or(""));
                match_score(mode, &hay, &self.query).map(|s| (s, i, c))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        scored
            .into_iter()
            .map(|(_, _, c)| PaletteRow {
                key: RowKey::ConnectHost {
                    host_id: c.id.clone(),
                    sftp: false,
                },
                secondary: Some(SecondaryAction {
                    label: "Open SFTP",
                    key: RowKey::ConnectHost {
                        host_id: c.id.clone(),
                        sftp: true,
                    },
                }),
                icon: Some(IconName::Server),
                title: c.title.clone(),
                subtitle: c.subtitle.clone(),
                section: section.to_string(),
                keys: vec![],
                right_label: c.active.then(|| "active".to_string()),
                has_sub: false,
            })
            .collect()
    }

    fn rows(&self, cx: &App) -> Vec<PaletteRow> {
        let mode = self.search_mode(cx);
        let overrides = self.prefs.read(cx).keybind_overrides(cx);
        match self.page() {
            Page::Root => {
                let ctx = self.active_context(cx);
                let font_size = self.prefs.read(cx).terminal_font_size();
                let tab_count = self.workspace.read(cx).palette_tab_rows(cx).len();
                let mut root: Vec<PaletteRow> = search_mode(&self.query, ctx, mode)
                    .into_iter()
                    .map(|c| {
                        let right_label = toggle_pref_key(c.id).map(|k| {
                            if self.prefs.read(cx).toggle_state(k) {
                                "ON".to_string()
                            } else {
                                "OFF".to_string()
                            }
                        });
                        let subtitle = match c.id {
                            CommandId::AdjustFontSize => Some(format!("{font_size}px")),
                            CommandId::SwitchTab => Some(format!("{tab_count} open")),
                            _ => None,
                        };
                        PaletteRow {
                            key: c
                                .sub_page
                                .map(RowKey::Navigate)
                                .unwrap_or(RowKey::Command(c.id)),
                            secondary: None,
                            icon: Some(c.icon),
                            title: c.title.to_string(),
                            subtitle,
                            section: c.section.to_string(),
                            keys: c
                                .shortcut
                                .map(|s| effective_keys(s, &overrides))
                                .unwrap_or_default(),
                            right_label,
                            has_sub: c.sub_page.is_some(),
                        }
                    })
                    .collect();
                if !self.data.recent_hosts.is_empty() {
                    root.extend(self.host_rows(&self.data.recent_hosts, "Hosts", mode));
                }
                root
            }
            Page::Tabs => {
                let q = self.query.trim().to_lowercase();
                self.workspace
                    .read(cx)
                    .palette_tab_rows(cx)
                    .into_iter()
                    .filter(|t| q.is_empty() || t.label.to_lowercase().contains(&q))
                    .map(|t| PaletteRow {
                        key: RowKey::Tab(t.id),
                        secondary: None,
                        icon: Some(IconName::Terminal),
                        title: t.label,
                        subtitle: Some(t.kind_title),
                        section: "Open Tabs".to_string(),
                        keys: vec![],
                        right_label: None,
                        has_sub: false,
                    })
                    .collect()
            }
            Page::Zoom => [
                (CommandId::ZoomIn, "Increase Font Size", ViewZoomIn),
                (CommandId::ZoomOut, "Decrease Font Size", ViewZoomOut),
                (CommandId::ZoomReset, "Reset Font Size", ViewZoomReset),
            ]
            .into_iter()
            .filter(|(_, title, _)| match_score(mode, title, &self.query).is_some())
            .map(|(id, title, sc)| PaletteRow {
                key: RowKey::Command(id),
                secondary: None,
                icon: Some(IconName::ArrowDownUp),
                title: title.to_string(),
                subtitle: Some(format!("{}px", self.prefs.read(cx).terminal_font_size())),
                section: "Font Size".to_string(),
                keys: effective_keys(sc, &overrides),
                right_label: None,
                has_sub: false,
            })
            .collect(),
            Page::ColorMode => [
                (ThemePreference::Dark, "Dark Mode"),
                (ThemePreference::Light, "Light Mode"),
                (ThemePreference::System, "System (Auto)"),
            ]
            .into_iter()
            .filter(|(_, title)| match_score(mode, title, &self.query).is_some())
            .map(|(pref, title)| PaletteRow {
                key: RowKey::SetColorMode(pref),
                secondary: None,
                icon: Some(IconName::Refresh),
                title: title.to_string(),
                subtitle: None,
                section: "Color Mode".to_string(),
                keys: vec![],
                right_label: (self.prefs.read(cx).color_mode() == pref)
                    .then(|| "active".to_string()),
                has_sub: false,
            })
            .collect(),
            Page::EditorTheme => EditorThemeId::ALL
                .into_iter()
                .map(|id| (id, editor_theme_label(id)))
                .filter(|(_, label)| match_score(mode, label, &self.query).is_some())
                .map(|(id, label)| PaletteRow {
                    key: RowKey::SetEditorTheme(id),
                    secondary: None,
                    icon: Some(IconName::Sparkles),
                    title: label,
                    subtitle: None,
                    section: "Editor Themes".to_string(),
                    keys: vec![],
                    right_label: (self.prefs.read(cx).editor_theme() == id)
                        .then(|| "active".to_string()),
                    has_sub: false,
                })
                .collect(),
            Page::Themes => self.choice_rows(
                &self.data.app_themes,
                "App Themes",
                IconName::Sparkles,
                mode,
                "No themes installed yet",
                |c| RowKey::SetAppTheme(c.id.clone()),
            ),
            Page::Hosts => self.host_rows(&self.data.hosts, "Hosts", mode),
            Page::Snippets => self.choice_rows(
                &self.data.snippets,
                "Snippets",
                IconName::Command,
                mode,
                "No snippets saved yet",
                |c| RowKey::RunSnippet(c.id.clone()),
            ),
            Page::AiSessions => self.choice_rows(
                &self.data.ai_sessions,
                "AI Sessions",
                IconName::Sparkles,
                mode,
                "No AI sessions yet",
                |c| RowKey::SwitchAiSession(c.id.clone()),
            ),
            Page::Outline => self.choice_rows(
                &self.data.symbols,
                "Symbols",
                IconName::FileCode,
                mode,
                "No symbols found",
                |c| {
                    c.id.parse::<usize>()
                        .map(RowKey::GoToLine)
                        .unwrap_or(RowKey::Noop)
                },
            ),
            Page::GitBranches => self.choice_rows(
                &self.data.git_branches,
                "Branches",
                IconName::GitBranch,
                mode,
                "No repository detected",
                |c| RowKey::SwitchBranch(c.id.clone()),
            ),
            Page::StatusBarHidden => self.choice_rows(
                &self.data.status_bar_hidden,
                "Hidden Status Bar Items",
                IconName::Eye,
                mode,
                "No hidden status-bar items",
                |c| RowKey::ShowStatusBarItem(c.id.clone()),
            ),
        }
    }

    fn run_selected(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(key) = rows.get(self.selected).map(|r| r.key.clone()) else {
            return;
        };
        self.dispatch(key, cx);
    }

    /// `Shift+Enter` — run the selected row's secondary action, if it has one.
    fn run_secondary(&mut self, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(sec) = rows.get(self.selected).and_then(|r| r.secondary.clone()) else {
            return;
        };
        self.dispatch(sec.key, cx);
    }

    fn dispatch(&mut self, key: RowKey, cx: &mut Context<Self>) {
        match key {
            RowKey::Noop => {}
            RowKey::Navigate(page) => self.navigate(page, cx),
            RowKey::Command(id) => {
                self.push_recent(id, cx);
                self.close(cx);
                cx.emit(PaletteEvent::Run(id));
            }
            RowKey::Tab(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::SwitchToTab(id));
            }
            RowKey::SetColorMode(p) => {
                self.close(cx);
                cx.emit(PaletteEvent::SetColorMode(p));
            }
            RowKey::SetEditorTheme(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::SetEditorTheme(id));
            }
            RowKey::ConnectHost { host_id, sftp } => {
                self.close(cx);
                cx.emit(PaletteEvent::ConnectHost { host_id, sftp });
            }
            RowKey::SetAppTheme(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::SetAppTheme(id));
            }
            RowKey::RunSnippet(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::RunSnippet(id));
            }
            RowKey::SwitchAiSession(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::SwitchAiSession(id));
            }
            RowKey::SwitchBranch(name) => {
                self.close(cx);
                cx.emit(PaletteEvent::SwitchBranch(name));
            }
            RowKey::GoToLine(line) => {
                self.close(cx);
                cx.emit(PaletteEvent::GoToLine(line));
            }
            RowKey::ShowStatusBarItem(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::ShowStatusBarItem(id));
            }
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        let ks = &ev.keystroke;
        let len = self.rows(cx).len();
        match ks.key.as_str() {
            "escape" => {
                if !self.query.is_empty() {
                    self.query.clear();
                    self.selected = 0;
                    cx.notify();
                } else if self.pages.len() > 1 {
                    self.go_back(cx);
                } else {
                    self.close(cx);
                }
            }
            "enter" => {
                if ks.modifiers.shift {
                    self.run_secondary(cx);
                } else {
                    self.run_selected(window, cx);
                }
            }
            "down" => {
                if len > 0 {
                    self.selected = (self.selected + 1) % len;
                    cx.notify();
                }
            }
            "up" => {
                if len > 0 {
                    self.selected = (self.selected + len - 1) % len;
                    cx.notify();
                }
            }
            "backspace" => {
                if self.query.is_empty() && self.pages.len() > 1 {
                    self.go_back(cx);
                } else {
                    self.query.pop();
                    self.selected = 0;
                    cx.notify();
                }
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(ch) = ch {
                    self.query.push_str(&ch);
                    self.selected = 0;
                    cx.notify();
                }
            }
        }
        if self.open {
            self.sync_theme_preview(cx);
        }
        cx.stop_propagation();
    }

    fn cycle_search_mode(&mut self, cx: &mut Context<Self>) {
        let next = self.search_mode(cx).next();
        self.prefs
            .update(cx, |s, cx| s.set_command_palette_search_mode(next, cx));
        cx.notify();
    }
}

/// Human label for an editor-theme slug (e.g. `github-dark` → "Github Dark").
fn editor_theme_label(id: EditorThemeId) -> String {
    id.slug()
        .split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

impl<P, W, Th> Focusable for CommandPalette<P, W, Th>
where
    P: PalettePrefs + 'static,
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// A single `<Kbd>` chip.
fn kbd(label: impl Into<SharedString>, fg: Hsla, border: Hsla) -> impl IntoElement {
    div()
        .px(px(4.0))
        .py(px(1.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(border)
        .text_size(px(10.0))
        .text_color(fg)
        .child(label.into())
}

impl<P, W, Th> Render for CommandPalette<P, W, Th>
where
    P: PalettePrefs + 'static,
    W: PaletteWorkspace + 'static,
    Th: UiTheme + 'static,
{
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        let p = self.prefs.read(cx);
        let opacity = (p.command_palette_opacity() as f32 / 100.0).clamp(0.35, 1.0);
        let position = p.command_palette_position();
        let show_recent = p.command_palette_show_recent();
        let close_on_overlay = p.command_palette_close_on_overlay_click();
        let mode = self.search_mode(cx);

        let t = self.theme.read(cx);
        let (fg, muted, border, success, primary) = (
            t.foreground(),
            t.muted_foreground(),
            t.border(),
            t.status_success(),
            t.primary(),
        );
        let mut card = t.card();
        card.a *= opacity;
        let sel_fill = t.selected_fill();
        let chip_bg = t.muted();

        let page = self.page();
        let (input_text, input_color) = if self.query.is_empty() {
            (page.placeholder().to_string(), muted)
        } else {
            (self.query.clone(), fg)
        };

        // Rows, with an optional "Recently Used" group prepended on Root.
        let mut rows = Vec::new();
        if page == Page::Root && self.query.is_empty() && show_recent && !self.recent.is_empty() {
            let ctx = self.active_context(cx);
            let overrides = self.prefs.read(cx).keybind_overrides(cx);
            let avail: std::collections::HashSet<CommandId> =
                available(ctx).into_iter().map(|c| c.id).collect();
            for id in self.recent.iter().copied().filter(|id| avail.contains(id)) {
                let c = command(id);
                rows.push(PaletteRow {
                    key: c
                        .sub_page
                        .map(RowKey::Navigate)
                        .unwrap_or(RowKey::Command(id)),
                    secondary: None,
                    icon: Some(c.icon),
                    title: c.title.to_string(),
                    subtitle: None,
                    section: "Recently Used".to_string(),
                    keys: c
                        .shortcut
                        .map(|s| effective_keys(s, &overrides))
                        .unwrap_or_default(),
                    right_label: None,
                    has_sub: c.sub_page.is_some(),
                });
            }
        }
        rows.extend(self.rows(cx));
        let result_count = rows
            .iter()
            .filter(|r| !matches!(r.key, RowKey::Noop))
            .count();
        let selected = self.selected.min(rows.len().saturating_sub(1));

        // ── list ─────────────────────────────────────────────────────────────
        let mut list = div()
            .id("palette-list")
            .flex()
            .flex_col()
            .p(px(8.0))
            .max_h(px(384.0))
            .overflow_y_scroll();
        if result_count == 0 {
            list = list.child(
                div()
                    .py(px(40.0))
                    .flex()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(muted)
                    .child("No results found."),
            );
        }
        let mut last_section: Option<String> = None;
        for (i, row) in rows.iter().enumerate() {
            if last_section.as_deref() != Some(row.section.as_str()) {
                list = list.child(
                    div()
                        .px(px(12.0))
                        .py(px(8.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(muted.opacity(0.7))
                        .child(SharedString::from(row.section.to_uppercase())),
                );
                last_section = Some(row.section.clone());
            }
            let is_sel = i == selected;
            let actionable = !matches!(row.key, RowKey::Noop);
            let mut r = div()
                .id(("palette-row", i))
                .flex()
                .items_center()
                .gap(px(12.0))
                .mx(px(2.0))
                .my(px(2.0))
                .px(px(if is_sel { 10.0 } else { 12.0 }))
                .py(px(8.0))
                .min_h(px(40.0))
                .rounded(px(8.0))
                .border_l_2()
                .border_color(if is_sel {
                    primary
                } else {
                    gpui::transparent_black()
                })
                .text_color(fg)
                .when(is_sel, |d| d.bg(sel_fill))
                .when(actionable && !is_sel, |d| d.hover(|s| s.bg(sel_fill)));

            if let Some(icon) = row.icon {
                r = r.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(28.0))
                        .rounded(px(6.0))
                        .bg(chip_bg)
                        .flex_none()
                        .child(icon.svg(muted)),
                );
            }

            let mut text_col = div().flex().flex_col().flex_1().min_w_0().child(
                div()
                    .text_size(px(13.0))
                    .truncate()
                    .child(SharedString::from(row.title.clone())),
            );
            if let Some(sub) = &row.subtitle {
                text_col = text_col.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(muted)
                        .truncate()
                        .child(SharedString::from(sub.clone())),
                );
            }
            r = r.child(text_col);

            let mut right = div().flex().items_center().gap(px(6.0)).flex_none();
            if let Some(label) = &row.right_label {
                let on = label == "ON" || label == "active";
                right = right.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(if on { success } else { muted })
                        .child(SharedString::from(label.to_uppercase())),
                );
            }
            for k in &row.keys {
                right = right.child(kbd(k.clone(), muted, border));
            }
            if row.has_sub {
                right = right.child(IconName::ChevronRight.svg(muted).size(px(12.0)));
            }
            r = r.child(right);

            if actionable {
                r = r.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.selected = i;
                    this.run_selected(window, cx);
                }));
            }
            list = list.child(r);
        }

        // ── header (breadcrumb or search icon + input) ───────────────────────
        let mut header = div()
            .h(px(56.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .border_b_1()
            .border_color(border);
        if self.pages.len() > 1 {
            let mut crumbs = div().flex().items_center().gap(px(4.0)).flex_none();
            let last = self.pages.len() - 1;
            for (idx, pg) in self.pages.clone().into_iter().enumerate() {
                if idx > 0 {
                    crumbs = crumbs.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(muted)
                            .child("\u{203a}"),
                    );
                }
                let is_current = idx == last;
                crumbs = crumbs.child(
                    div()
                        .id(("crumb", idx))
                        .px(px(8.0))
                        .py(px(2.0))
                        .rounded(px(6.0))
                        .text_size(px(11.0))
                        .text_color(if is_current { fg } else { muted })
                        .when(is_current, |d| d.bg(sel_fill))
                        .when(!is_current, |d| {
                            d.hover(|s| s.bg(sel_fill)).on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| this.go_back_to(idx, cx),
                            ))
                        })
                        .child(SharedString::from(pg.label())),
                );
            }
            header = header.child(crumbs);
        } else {
            header = header.child(IconName::Search.svg(muted));
        }
        header = header.child(
            div()
                .flex_1()
                .text_size(px(15.0))
                .text_color(input_color)
                .child(SharedString::from(input_text)),
        );

        // ── footer ───────────────────────────────────────────────────────────
        let mut hints = div().flex().items_center().gap(px(12.0)).ml_auto();
        hints = hints
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(kbd("\u{2191}\u{2193}", muted, border))
                    .child("navigate"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(kbd("\u{21b5}", muted, border))
                    .child("select"),
            );
        if let Some(sec) = rows.get(selected).and_then(|r| r.secondary.as_ref()) {
            hints = hints.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(kbd("\u{21e7}\u{21b5}", muted, border))
                    .child(SharedString::from(sec.label)),
            );
        }
        if self.pages.len() > 1 {
            hints = hints.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(kbd("\u{232b}", muted, border))
                    .child("back"),
            );
        }
        hints = hints.child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(kbd("Esc", muted, border))
                .child("close"),
        );
        let footer = div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(16.0))
            .py(px(8.0))
            .border_t_1()
            .border_color(border)
            .child(
                div()
                    .id("palette-search-mode")
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(border)
                    .text_size(px(10.0))
                    .text_color(muted)
                    .hover(|s| s.text_color(fg))
                    .child(SharedString::from(mode.label()))
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.cycle_search_mode(cx)),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child(SharedString::from(format!(
                        "{result_count} {}",
                        if result_count == 1 {
                            "result"
                        } else {
                            "results"
                        }
                    ))),
            )
            .child(hints);

        let top = match position.as_str() {
            "high" => px(48.0),
            "center" => px(160.0),
            _ => px(96.0),
        };

        div()
            .id("palette-overlay")
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(top)
            .bg(modal_scrim())
            .track_focus(&self.focus)
            .key_context("CommandPalette")
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if close_on_overlay {
                    this.close(cx);
                }
            }))
            .child(
                div()
                    .id("palette-card")
                    .occlude()
                    .w(px(640.0))
                    .max_h(px(560.0))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(16.0))
                    .bg(card)
                    .border_1()
                    .border_color(border.opacity(0.6))
                    .shadow_lg()
                    .child(header)
                    .child(div().flex().flex_col().min_h_0().child(list))
                    .child(footer),
            )
            .into_any_element()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `CommandId` variant, hand-listed (no `strum` dep in this crate,
    /// per T19-008's scope) so `action_name`/`from_action_name` round-trip
    /// coverage doesn't silently skip a variant added later without also
    /// updating this list.
    #[rustfmt::skip]
    const ALL_COMMAND_IDS: &[CommandId] = &[
        CommandId::NewTerminalTab, CommandId::NewEditorTab, CommandId::DuplicateTab,
        CommandId::CloseOtherTabs, CommandId::SplitRight, CommandId::SplitDown,
        CommandId::ClosePane, CommandId::CloseTab, CommandId::NextTab, CommandId::PrevTab,
        CommandId::SwitchTab, CommandId::Find, CommandId::ToggleSidebar,
        CommandId::ToggleFullScreen, CommandId::ZoomIn, CommandId::ZoomOut,
        CommandId::ZoomReset, CommandId::ToggleAiPanel, CommandId::AskSelection,
        CommandId::NewAiSession, CommandId::OpenSnippetsPanel, CommandId::OpenGitGraph,
        CommandId::FocusSourceControl, CommandId::OpenHostManager, CommandId::ClearTerminal,
        CommandId::OpenShortcuts, CommandId::OpenSettings, CommandId::OpenProjectSettings,
        CommandId::OpenSettingsJson, CommandId::CheckForUpdates, CommandId::FormatDocument,
        CommandId::OpenPathBookmarks, CommandId::ToggleZenModeHeader,
        CommandId::ToggleZenModeStatusbar, CommandId::ToggleZenMode, CommandId::AdjustFontSize,
        CommandId::ConnectSsh, CommandId::OpenSftp, CommandId::ChangeAppTheme,
        CommandId::ChangeColorMode, CommandId::ChangeEditorTheme, CommandId::SwitchAiSession,
        CommandId::RunSnippet, CommandId::GitSwitchBranch, CommandId::GoToSymbol,
        CommandId::ShowStatusBarItem, CommandId::OpenAiSettings, CommandId::ToggleEditorWordWrap,
        CommandId::ToggleLineNumbers, CommandId::ToggleFormatOnSave, CommandId::ToggleCursorBlink,
        CommandId::TogglePaneHeader, CommandId::TogglePaneFooter, CommandId::ToggleVimMode,
        CommandId::OpenCommandPalette, CommandId::NewPreviewTab, CommandId::Save,
        CommandId::NewSshTab, CommandId::NewSftpTab, CommandId::NewSshConnection,
        CommandId::NewQuickSsh, CommandId::ClearChat, CommandId::FocusNextPane,
        CommandId::SelectTab1, CommandId::SelectTab2, CommandId::SelectTab3,
        CommandId::SelectTab4, CommandId::SelectTab5, CommandId::SelectTab6,
        CommandId::SelectTab7, CommandId::SelectTab8, CommandId::SelectTab9,
        CommandId::DebugCyclePanelDock, CommandId::DebugToggleDockZoom,
        CommandId::OpenKeymapJson,
    ];

    #[test]
    fn every_command_id_has_a_unique_action_name() {
        assert_eq!(
            ALL_COMMAND_IDS.len(),
            ACTION_NAMES.len(),
            "ALL_COMMAND_IDS and ACTION_NAMES have drifted apart"
        );
        for id in ALL_COMMAND_IDS {
            assert_eq!(
                CommandId::from_action_name(id.action_name()),
                Some(*id),
                "{id:?} action_name round-trip failed"
            );
        }
        let mut names: Vec<&str> = ACTION_NAMES.iter().map(|(_, n)| *n).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "duplicate action name");
    }

    #[test]
    fn registry_lists_all_domains() {
        let sections: std::collections::HashSet<_> = commands().iter().map(|c| c.section).collect();
        for expected in [
            "Layout",
            "Tab Actions",
            "Terminal",
            "Connections",
            "Search",
            "View",
            "AI",
            "Snippets",
            "Source Control",
            "Editor",
            "Settings",
            "Application",
        ] {
            assert!(sections.contains(expected), "missing domain {expected}");
        }
    }

    #[test]
    fn command_lookup_by_id() {
        assert_eq!(command(CommandId::ZoomIn).title, "Zoom In");
        assert_eq!(
            command(CommandId::SplitRight).contexts,
            &[CommandContext::Terminal]
        );
    }

    #[test]
    fn context_filtering() {
        // No context: only always-available commands.
        let home = available(None);
        assert!(home.iter().all(|c| c.contexts.is_empty()));
        assert!(home.iter().any(|c| c.id == CommandId::NewTerminalTab));
        assert!(!home.iter().any(|c| c.id == CommandId::SplitRight));

        // Terminal context: unlocks split/clear, still no editor commands.
        let term = available(Some(CommandContext::Terminal));
        assert!(term.iter().any(|c| c.id == CommandId::SplitRight));
        assert!(term.iter().any(|c| c.id == CommandId::ClearTerminal));
        assert!(!term.iter().any(|c| c.id == CommandId::FormatDocument));

        // Editor context: format shows, split does not.
        let editor = available(Some(CommandContext::Editor));
        assert!(editor.iter().any(|c| c.id == CommandId::FormatDocument));
        assert!(!editor.iter().any(|c| c.id == CommandId::SplitRight));

        // SSH terminal shares the "clear terminal" command.
        assert!(available(Some(CommandContext::SshTerminal))
            .iter()
            .any(|c| c.id == CommandId::ClearTerminal));
    }

    #[test]
    fn search_matches_title_and_section() {
        let by_title = search("split pane", Some(CommandContext::Terminal));
        assert!(by_title.iter().any(|c| c.id == CommandId::SplitRight));

        let by_section = search("source control", None);
        assert!(by_section.iter().any(|c| c.id == CommandId::OpenGitGraph));

        // Empty query returns everything available.
        assert_eq!(search("", None).len(), available(None).len());
        // Nonsense filters to nothing.
        assert!(search("zzzznope", None).is_empty());
    }

    #[test]
    fn shortcut_triggers_its_command() {
        assert_eq!(
            command_for_shortcut(ShortcutId::TabNew),
            Some(CommandId::NewTerminalTab),
        );
        assert_eq!(
            command_for_shortcut(ShortcutId::PaneSplitRight),
            Some(CommandId::SplitRight),
        );
        assert_eq!(
            command_for_shortcut(ShortcutId::BookmarksOpen),
            Some(CommandId::OpenPathBookmarks),
        );
        assert_eq!(
            command_for_shortcut(ShortcutId::ViewZenMode),
            Some(CommandId::ToggleZenMode),
        );
        // Shortcuts with no palette command (tab-number jumps, pane focus).
        assert_eq!(command_for_shortcut(ShortcutId::TabSelect5), None);
        assert_eq!(command_for_shortcut(ShortcutId::PaneFocusNext), None);
    }

    #[test]
    fn context_of_maps_tab_kinds() {
        assert_eq!(
            context_of(PaletteTabKind::Workspace, false),
            Some(CommandContext::Terminal)
        );
        assert_eq!(
            context_of(PaletteTabKind::Workspace, true),
            Some(CommandContext::SshTerminal)
        );
        assert_eq!(
            context_of(PaletteTabKind::Editor, false),
            Some(CommandContext::Editor)
        );
        assert_eq!(context_of(PaletteTabKind::Other, false), None);
    }

    #[test]
    fn search_mode_ranks_results() {
        let hits = search_mode("split", Some(CommandContext::Terminal), SearchMode::Fuzzy);
        assert!(hits.iter().any(|c| c.id == CommandId::SplitRight));
        // Fuzzy still filters nonsense out.
        assert!(search_mode("zzzznope", None, SearchMode::Fuzzy).is_empty());
    }

    #[test]
    fn every_command_has_an_icon_and_nav_targets_resolve() {
        for c in commands() {
            // `icon` is non-optional by type; assert `sub_page` rows are
            // navigators, not runnable no-ops.
            let _ = c.icon;
            if let Some(pg) = c.sub_page {
                assert_ne!(pg, Page::Root, "{:?} navigates to Root", c.id);
                assert!(!pg.placeholder().is_empty());
                assert!(!pg.label().is_empty());
            }
        }
    }

    #[test]
    fn toggle_commands_map_to_pref_keys() {
        assert_eq!(toggle_pref_key(CommandId::ToggleVimMode), Some("vimMode"));
        assert_eq!(
            toggle_pref_key(CommandId::ToggleEditorWordWrap),
            Some("editorWordWrap")
        );
        assert_eq!(toggle_pref_key(CommandId::NewTerminalTab), None);
    }

    #[test]
    fn editor_theme_label_is_titlecased() {
        assert_eq!(editor_theme_label(EditorThemeId::GithubDark), "Github Dark");
        assert_eq!(editor_theme_label(EditorThemeId::Nord), "Nord");
    }

    #[test]
    fn all_pages_have_distinct_labels() {
        let pages = [
            Page::Root,
            Page::Zoom,
            Page::Tabs,
            Page::ColorMode,
            Page::EditorTheme,
            Page::Themes,
            Page::Hosts,
            Page::Snippets,
            Page::AiSessions,
            Page::Outline,
            Page::GitBranches,
            Page::StatusBarHidden,
        ];
        let labels: std::collections::HashSet<_> = pages.iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), pages.len());
    }
}
