//! Command palette + keyboard-shortcut system (T12-002).
//!
//! Port of `reference-src/src/modules/command-palette/*` and
//! `reference-src/src/modules/shortcuts/*`. Two layers:
//!
//! * **Data** — a static [`Command`] registry (id / title / section /
//!   contexts / optional shortcut) and a static [`Shortcut`] table (the
//!   single source of truth the reference kept in `shortcuts.ts`), plus
//!   pure filtering / search / conflict-detection helpers. All unit-tested,
//!   no GPUI needed.
//! * **View** — [`CommandPalette`], a modal overlay opened with `Cmd+P`
//!   (bound in [`crate::menu`]). Type to filter, arrow keys to move, `Enter`
//!   to run, `Esc` to close. Commands that need an argument ("Switch
//!   Tab\u{2026}") push a follow-up page listing the choices.
//!
//! Execution: the palette does not own the app state — on `Enter` it emits
//! [`PaletteEvent`], which [`crate::app_shell::AppShell`] turns into either a
//! GPUI action dispatch (same code path as the native menu, so later phases
//! that wire a handler light the command up automatically) or a direct
//! workspace call. This mirrors the reference `RegistryCallbacks` indirection.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::tabs::TabKind;
use crate::theme::ThemeStore;
use crate::workspace::Workspace;

// ─────────────────────────────────────────────────────────────────────────────
// Shortcut table (port of shortcuts.ts)
// ─────────────────────────────────────────────────────────────────────────────

/// Every rebindable keyboard shortcut. IDs match the reference `ShortcutId`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShortcutId {
    CommandPalette,
    ShortcutsOpen,
    TabNew,
    TabNewPreview,
    TabNewEditor,
    TabClose,
    TabNext,
    TabPrev,
    TabSelect1,
    TabSelect2,
    TabSelect3,
    TabSelect4,
    TabSelect5,
    TabSelect6,
    TabSelect7,
    TabSelect8,
    TabSelect9,
    PaneSplitRight,
    PaneSplitDown,
    PaneClose,
    PaneFocusNext,
    SearchFocus,
    AiToggle,
    AiAskSelection,
    SidebarToggle,
    ViewZenMode,
    ViewZoomIn,
    ViewZoomOut,
    ViewZoomReset,
    BookmarksOpen,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShortcutGroup {
    General,
    Tabs,
    Search,
    Ai,
    View,
    Bookmarks,
}

/// One shortcut entry: display tokens for the cheat sheet + the GPUI
/// keystroke string the runtime actually binds.
pub struct Shortcut {
    pub id: ShortcutId,
    pub label: &'static str,
    pub keys: &'static [&'static str],
    pub group: ShortcutGroup,
    /// GPUI keystroke (e.g. `"cmd-shift-p"`) — parseable by
    /// [`gpui::Keystroke::parse`].
    pub binding: &'static str,
}

use ShortcutGroup::*;
use ShortcutId::*;

#[rustfmt::skip]
static SHORTCUTS: &[Shortcut] = &[
    Shortcut { id: CommandPalette,  label: "Open command palette",      keys: &["\u{2318}", "P"],                 group: General,   binding: "cmd-p" },
    Shortcut { id: ShortcutsOpen,   label: "Show keyboard shortcuts",   keys: &["\u{2318}", "K"],                 group: General,   binding: "cmd-k" },
    Shortcut { id: TabNew,          label: "New tab",                   keys: &["\u{2318}", "T"],                 group: Tabs,      binding: "cmd-t" },
    Shortcut { id: TabNewPreview,   label: "New preview tab",           keys: &["\u{2318}", "\u{21e7}", "P"],     group: Tabs,      binding: "cmd-shift-p" },
    Shortcut { id: TabNewEditor,    label: "New editor tab",            keys: &["\u{2318}", "E"],                 group: Tabs,      binding: "cmd-e" },
    Shortcut { id: TabClose,        label: "Close tab",                 keys: &["\u{2318}", "W"],                 group: Tabs,      binding: "cmd-w" },
    Shortcut { id: TabNext,         label: "Next tab",                  keys: &["\u{2303}", "\u{21e5}"],          group: Tabs,      binding: "ctrl-tab" },
    Shortcut { id: TabPrev,         label: "Previous tab",              keys: &["\u{2303}", "\u{21e7}", "\u{21e5}"], group: Tabs,   binding: "ctrl-shift-tab" },
    Shortcut { id: TabSelect1,      label: "Jump to tab 1",             keys: &["\u{2318}", "1"],                 group: Tabs,      binding: "cmd-1" },
    Shortcut { id: TabSelect2,      label: "Jump to tab 2",             keys: &["\u{2318}", "2"],                 group: Tabs,      binding: "cmd-2" },
    Shortcut { id: TabSelect3,      label: "Jump to tab 3",             keys: &["\u{2318}", "3"],                 group: Tabs,      binding: "cmd-3" },
    Shortcut { id: TabSelect4,      label: "Jump to tab 4",             keys: &["\u{2318}", "4"],                 group: Tabs,      binding: "cmd-4" },
    Shortcut { id: TabSelect5,      label: "Jump to tab 5",             keys: &["\u{2318}", "5"],                 group: Tabs,      binding: "cmd-5" },
    Shortcut { id: TabSelect6,      label: "Jump to tab 6",             keys: &["\u{2318}", "6"],                 group: Tabs,      binding: "cmd-6" },
    Shortcut { id: TabSelect7,      label: "Jump to tab 7",             keys: &["\u{2318}", "7"],                 group: Tabs,      binding: "cmd-7" },
    Shortcut { id: TabSelect8,      label: "Jump to tab 8",             keys: &["\u{2318}", "8"],                 group: Tabs,      binding: "cmd-8" },
    Shortcut { id: TabSelect9,      label: "Jump to tab 9",             keys: &["\u{2318}", "9"],                 group: Tabs,      binding: "cmd-9" },
    Shortcut { id: PaneSplitRight,  label: "Split pane right",          keys: &["\u{2318}", "D"],                 group: Tabs,      binding: "cmd-d" },
    Shortcut { id: PaneSplitDown,   label: "Split pane down",           keys: &["\u{2318}", "\u{21e7}", "D"],     group: Tabs,      binding: "cmd-shift-d" },
    Shortcut { id: PaneClose,       label: "Close pane",               keys: &["\u{2318}", "\u{21e7}", "W"],     group: Tabs,      binding: "cmd-shift-w" },
    Shortcut { id: PaneFocusNext,   label: "Focus next pane",           keys: &["\u{2318}", "]"],                 group: Tabs,      binding: "cmd-]" },
    Shortcut { id: SearchFocus,     label: "Find in current pane",      keys: &["\u{2318}", "F"],                 group: Search,    binding: "cmd-f" },
    Shortcut { id: AiToggle,        label: "Toggle AI agent",           keys: &["\u{2318}", "I"],                 group: Ai,        binding: "cmd-i" },
    Shortcut { id: AiAskSelection,  label: "Ask AI about selection",    keys: &["\u{2318}", "L"],                 group: Ai,        binding: "cmd-l" },
    Shortcut { id: SidebarToggle,   label: "Toggle file explorer",      keys: &["\u{2318}", "B"],                 group: View,      binding: "cmd-b" },
    Shortcut { id: ViewZenMode,     label: "Toggle Zen mode",           keys: &["\u{2318}", "\u{21e7}", "Z"],     group: View,      binding: "cmd-shift-z" },
    Shortcut { id: ViewZoomIn,      label: "Zoom in",                   keys: &["\u{2318}", "+"],                 group: View,      binding: "cmd-=" },
    Shortcut { id: ViewZoomOut,     label: "Zoom out",                  keys: &["\u{2318}", "\u{2212}"],          group: View,      binding: "cmd--" },
    Shortcut { id: ViewZoomReset,   label: "Reset zoom",                keys: &["\u{2318}", "0"],                 group: View,      binding: "cmd-0" },
    Shortcut { id: BookmarksOpen,   label: "Open path bookmarks",       keys: &["\u{2318}", "\u{21e7}", "O"],     group: Bookmarks, binding: "cmd-shift-o" },
];

/// All shortcuts, in cheat-sheet order.
pub fn shortcuts() -> &'static [Shortcut] {
    SHORTCUTS
}

/// The shortcut for `id` (every [`ShortcutId`] has exactly one entry).
pub fn shortcut(id: ShortcutId) -> &'static Shortcut {
    SHORTCUTS
        .iter()
        .find(|s| s.id == id)
        .expect("every ShortcutId has a SHORTCUTS entry")
}

/// Display tokens (`["\u{2318}", "P"]`) for a shortcut — the palette's
/// right-aligned hint, mirroring the reference `useShortcutHint`.
pub fn shortcut_keys(id: ShortcutId) -> &'static [&'static str] {
    shortcut(id).keys
}

/// Native-menu-only accelerators with no [`ShortcutId`] — hardcoded in
/// [`crate::menu`] and never customizable. Kept here so conflict detection
/// can block a user from rebinding onto one. (`cmd-k` is *not* reserved: it
/// is the real, rebindable [`ShortcutId::ShortcutsOpen`].)
pub const RESERVED_ACCELERATORS: &[(&str, &str)] =
    &[("cmd-,", "Settings"), ("cmd-shift-n", "New SSH Connection")];

/// The thing a candidate binding collides with.
#[derive(Debug, PartialEq, Eq)]
pub enum Conflict {
    /// A native-menu accelerator (label).
    Reserved(&'static str),
    /// Another shortcut.
    Shortcut(ShortcutId),
}

/// Canonicalise a keystroke string so equivalent bindings compare equal
/// regardless of modifier order (`"shift-cmd-d"` == `"cmd-shift-d"`).
fn normalize(binding: &str) -> String {
    let mut parts: Vec<&str> = binding.split('-').filter(|s| !s.is_empty()).collect();
    // Trailing "" from a literal "cmd--" (minus key) — restore it.
    let key = if binding.ends_with("--") {
        parts.pop();
        "-"
    } else {
        parts.pop().unwrap_or("")
    };
    let rank = |m: &str| match m {
        "ctrl" | "control" => 0,
        "alt" | "option" => 1,
        "shift" => 2,
        "cmd" | "super" | "platform" | "win" => 3,
        _ => 4,
    };
    parts.sort_by_key(|m| rank(m));
    parts.dedup();
    let mods = parts.join("-");
    if mods.is_empty() {
        key.to_string()
    } else {
        format!("{mods}-{key}")
    }
}

/// Detect whether `binding` collides with a reserved accelerator or another
/// shortcut. `exclude` is the shortcut being rebound (skipped in the scan).
/// Port of `conflictDetector.ts::findConflict`.
pub fn find_conflict(binding: &str, exclude: Option<ShortcutId>) -> Option<Conflict> {
    let n = normalize(binding);
    for (b, label) in RESERVED_ACCELERATORS {
        if normalize(b) == n {
            return Some(Conflict::Reserved(label));
        }
    }
    for s in SHORTCUTS {
        if Some(s.id) == exclude {
            continue;
        }
        if normalize(s.binding) == n {
            return Some(Conflict::Shortcut(s.id));
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// User keybind overrides (port of shortcuts/keybinds-store.ts + useKeybindsStore)
// ─────────────────────────────────────────────────────────────────────────────

/// Stable string id for a shortcut, matching the reference `ShortcutId`
/// string literals in `shortcuts.ts`. This is the persisted key in the
/// user's keybind-override map.
pub fn shortcut_slug(id: ShortcutId) -> &'static str {
    match id {
        CommandPalette => "command.palette",
        ShortcutsOpen => "shortcuts.open",
        TabNew => "tab.new",
        TabNewPreview => "tab.newPreview",
        TabNewEditor => "tab.newEditor",
        TabClose => "tab.close",
        TabNext => "tab.next",
        TabPrev => "tab.prev",
        TabSelect1 => "tab.selectTab1",
        TabSelect2 => "tab.selectTab2",
        TabSelect3 => "tab.selectTab3",
        TabSelect4 => "tab.selectTab4",
        TabSelect5 => "tab.selectTab5",
        TabSelect6 => "tab.selectTab6",
        TabSelect7 => "tab.selectTab7",
        TabSelect8 => "tab.selectTab8",
        TabSelect9 => "tab.selectTab9",
        PaneSplitRight => "pane.splitRight",
        PaneSplitDown => "pane.splitDown",
        PaneClose => "pane.close",
        PaneFocusNext => "pane.focusNext",
        SearchFocus => "search.focus",
        AiToggle => "ai.toggle",
        AiAskSelection => "ai.askSelection",
        SidebarToggle => "sidebar.toggle",
        ViewZenMode => "view.zenMode",
        ViewZoomIn => "view.zoomIn",
        ViewZoomOut => "view.zoomOut",
        ViewZoomReset => "view.zoomReset",
        BookmarksOpen => "bookmarks.open",
    }
}

/// Reverse of [`shortcut_slug`].
pub fn shortcut_from_slug(slug: &str) -> Option<ShortcutId> {
    SHORTCUTS
        .iter()
        .map(|s| s.id)
        .find(|id| shortcut_slug(*id) == slug)
}

/// User keybind overrides: shortcut slug → keystroke string. An empty
/// string means the shortcut is explicitly disabled (unbound). An absent
/// key means "use the registry default" — so an empty map is a fresh
/// install running entirely on defaults.
pub type KeybindMap = std::collections::BTreeMap<String, String>;

/// The keystroke a shortcut currently resolves to, honouring user
/// overrides. `None` = the shortcut is disabled (overridden to empty).
pub fn effective_binding(id: ShortcutId, overrides: &KeybindMap) -> Option<String> {
    match overrides.get(shortcut_slug(id)) {
        Some(s) if s.is_empty() => None,
        Some(s) => Some(s.clone()),
        None => Some(shortcut(id).binding.to_string()),
    }
}

/// Like [`find_conflict`] but resolves every shortcut through `overrides`
/// first, so a rebound shortcut is compared at its *current* keystroke.
/// Port of the override-aware conflict check in `useKeybindsStore`.
pub fn resolve_conflict(
    binding: &str,
    exclude: Option<ShortcutId>,
    overrides: &KeybindMap,
) -> Option<Conflict> {
    let n = normalize(binding);
    for (b, label) in RESERVED_ACCELERATORS {
        if normalize(b) == n {
            return Some(Conflict::Reserved(label));
        }
    }
    for s in SHORTCUTS {
        if Some(s.id) == exclude {
            continue;
        }
        if let Some(eff) = effective_binding(s.id, overrides) {
            if normalize(&eff) == n {
                return Some(Conflict::Shortcut(s.id));
            }
        }
    }
    None
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
/// row + an arm in `AppShell::run_palette_command`.
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
    CheckForUpdates,
    FormatDocument,
    /// Opens the path-bookmarks popover (T12-003).
    OpenPathBookmarks,
    /// Zen-mode toggles (T13-005) — mirror `useSettingsCommands.ts`.
    ToggleZenModeHeader,
    ToggleZenModeStatusbar,
    ToggleZenMode,
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
}

use CommandContext::{Editor as CtxEditor, SshTerminal, Terminal as CtxTerminal};

#[rustfmt::skip]
static COMMANDS: &[Command] = &[
    Command { id: CommandId::NewTerminalTab,     title: "New Terminal Tab",        section: "Layout",         contexts: &[],                            shortcut: Some(TabNew) },
    Command { id: CommandId::NewEditorTab,       title: "New Editor Tab",          section: "Layout",         contexts: &[],                            shortcut: Some(TabNewEditor) },
    Command { id: CommandId::DuplicateTab,       title: "Duplicate Tab",           section: "Layout",         contexts: &[],                            shortcut: None },
    Command { id: CommandId::CloseOtherTabs,     title: "Close Other Tabs",        section: "Layout",         contexts: &[],                            shortcut: None },
    Command { id: CommandId::SwitchTab,          title: "Switch Tab\u{2026}",      section: "Layout",         contexts: &[],                            shortcut: None },
    Command { id: CommandId::SplitRight,         title: "Split Pane Right",        section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneSplitRight) },
    Command { id: CommandId::SplitDown,          title: "Split Pane Down",         section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneSplitDown) },
    Command { id: CommandId::ClosePane,          title: "Close Active Pane",       section: "Layout",         contexts: &[CtxTerminal],                 shortcut: Some(PaneClose) },
    Command { id: CommandId::CloseTab,           title: "Close Current Tab",       section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabClose) },
    Command { id: CommandId::NextTab,            title: "Next Tab",                section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabNext) },
    Command { id: CommandId::PrevTab,            title: "Previous Tab",            section: "Tab Actions",    contexts: &[],                            shortcut: Some(TabPrev) },
    Command { id: CommandId::ClearTerminal,      title: "Clear Terminal",          section: "Terminal",       contexts: &[CtxTerminal, SshTerminal],    shortcut: None },
    Command { id: CommandId::OpenHostManager,    title: "Open Host Manager",       section: "Connections",    contexts: &[],                            shortcut: None },
    Command { id: CommandId::Find,               title: "Find in Current Pane",    section: "Search",         contexts: &[],                            shortcut: Some(SearchFocus) },
    Command { id: CommandId::ToggleSidebar,      title: "Toggle File Explorer",    section: "View",           contexts: &[],                            shortcut: Some(SidebarToggle) },
    Command { id: CommandId::ToggleFullScreen,   title: "Toggle Full Screen",      section: "View",           contexts: &[],                            shortcut: None },
    Command { id: CommandId::ZoomIn,             title: "Zoom In",                 section: "View",           contexts: &[],                            shortcut: Some(ViewZoomIn) },
    Command { id: CommandId::ZoomOut,            title: "Zoom Out",                section: "View",           contexts: &[],                            shortcut: Some(ViewZoomOut) },
    Command { id: CommandId::ZoomReset,          title: "Reset Zoom",              section: "View",           contexts: &[],                            shortcut: Some(ViewZoomReset) },
    Command { id: CommandId::ToggleAiPanel,      title: "Toggle AI Panel",         section: "AI",             contexts: &[],                            shortcut: Some(AiToggle) },
    Command { id: CommandId::AskSelection,       title: "Ask AI About Selection",  section: "AI",             contexts: &[],                            shortcut: Some(AiAskSelection) },
    Command { id: CommandId::NewAiSession,       title: "New AI Session",          section: "AI",             contexts: &[],                            shortcut: None },
    Command { id: CommandId::OpenSnippetsPanel,  title: "Open Snippets Panel",     section: "Snippets",       contexts: &[],                            shortcut: None },
    Command { id: CommandId::OpenPathBookmarks,  title: "Open Path Bookmarks",     section: "Bookmarks",      contexts: &[],                            shortcut: Some(BookmarksOpen) },
    Command { id: CommandId::OpenGitGraph,       title: "Open Git Graph",          section: "Source Control", contexts: &[],                            shortcut: None },
    Command { id: CommandId::FocusSourceControl, title: "Focus Source Control",    section: "Source Control", contexts: &[],                            shortcut: None },
    Command { id: CommandId::FormatDocument,     title: "Format Document",         section: "Editor",         contexts: &[CtxEditor],                   shortcut: None },
    Command { id: CommandId::ToggleZenModeHeader,    title: "Toggle: Show Header Bar",   section: "Settings",       contexts: &[],                         shortcut: None },
    Command { id: CommandId::ToggleZenModeStatusbar, title: "Toggle: Show Status Bar",   section: "Settings",       contexts: &[],                         shortcut: None },
    Command { id: CommandId::ToggleZenMode,          title: "Toggle: Zen Mode",         section: "Settings",       contexts: &[],                         shortcut: Some(ViewZenMode) },
    Command { id: CommandId::OpenShortcuts,      title: "Keyboard Shortcuts",      section: "Application",    contexts: &[],                            shortcut: Some(ShortcutsOpen) },
    Command { id: CommandId::OpenSettings,       title: "Open Settings",           section: "Application",    contexts: &[],                            shortcut: None },
    Command { id: CommandId::CheckForUpdates,    title: "Check for Updates\u{2026}", section: "Application",   contexts: &[],                            shortcut: None },
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

/// Fuzzy-ish search over title + section (case-insensitive substring),
/// restricted to what's available in `ctx`.
pub fn search(query: &str, ctx: Option<CommandContext>) -> Vec<&'static Command> {
    let q = query.trim().to_lowercase();
    available(ctx)
        .into_iter()
        .filter(|c| {
            q.is_empty()
                || c.title.to_lowercase().contains(&q)
                || c.section.to_lowercase().contains(&q)
        })
        .collect()
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
pub fn context_of(tab_kind: TabKind, is_ssh: bool) -> Option<CommandContext> {
    Some(match tab_kind {
        TabKind::Workspace if is_ssh => CommandContext::SshTerminal,
        TabKind::Workspace => CommandContext::Terminal,
        TabKind::Editor => CommandContext::Editor,
        TabKind::Sftp => CommandContext::Sftp,
        TabKind::Home => CommandContext::Home,
        _ => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// View
// ─────────────────────────────────────────────────────────────────────────────

/// Emitted when the user picks something in the palette. `AppShell` handles it.
#[derive(Clone, Debug)]
pub enum PaletteEvent {
    Run(CommandId),
    SwitchToTab(u64),
}

#[derive(PartialEq)]
enum Page {
    Root,
    SwitchTab,
}

/// The Cmd+P command palette overlay.
pub struct CommandPalette {
    theme: Entity<ThemeStore>,
    workspace: Entity<Workspace>,
    open: bool,
    page: Page,
    query: String,
    selected: usize,
    focus: FocusHandle,
}

impl EventEmitter<PaletteEvent> for CommandPalette {}

impl CommandPalette {
    pub fn new(
        theme: Entity<ThemeStore>,
        workspace: Entity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            theme,
            workspace,
            open: false,
            page: Page::Root,
            query: String::new(),
            selected: 0,
            focus: cx.focus_handle(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
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
        self.page = Page::Root;
        self.query.clear();
        self.selected = 0;
        window.focus(&self.focus);
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.query.clear();
        self.page = Page::Root;
        self.selected = 0;
        cx.notify();
    }

    fn active_context(&self, cx: &App) -> Option<CommandContext> {
        self.workspace.read(cx).active_context(cx)
    }

    /// Rows for the current page: `(id, title, section, shortcut hint)`.
    fn rows(&self, cx: &App) -> Vec<PaletteRow> {
        match self.page {
            Page::Root => search(&self.query, self.active_context(cx))
                .into_iter()
                .map(|c| PaletteRow {
                    key: RowKey::Command(c.id),
                    title: c.title.to_string(),
                    section: c.section.to_string(),
                    hint: c.shortcut.map(|s| shortcut_keys(s).join(" ")),
                })
                .collect(),
            Page::SwitchTab => {
                let q = self.query.trim().to_lowercase();
                self.workspace
                    .read(cx)
                    .tab_store()
                    .read(cx)
                    .tabs()
                    .iter()
                    .filter(|t| q.is_empty() || t.label().to_lowercase().contains(&q))
                    .map(|t| PaletteRow {
                        key: RowKey::Tab(t.id),
                        title: t.label(),
                        section: t.kind.default_title().to_string(),
                        hint: None,
                    })
                    .collect()
            }
        }
    }

    fn run_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(row) = rows.get(self.selected) else {
            return;
        };
        match row.key {
            RowKey::Command(CommandId::SwitchTab) => {
                self.page = Page::SwitchTab;
                self.query.clear();
                self.selected = 0;
                window.focus(&self.focus);
                cx.notify();
            }
            RowKey::Command(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::Run(id));
            }
            RowKey::Tab(id) => {
                self.close(cx);
                cx.emit(PaletteEvent::SwitchToTab(id));
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
                if self.page == Page::SwitchTab {
                    self.page = Page::Root;
                    self.query.clear();
                    self.selected = 0;
                    cx.notify();
                } else {
                    self.close(cx);
                }
            }
            "enter" => self.run_selected(window, cx),
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
                self.query.pop();
                self.selected = 0;
                cx.notify();
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
        cx.stop_propagation();
    }
}

struct PaletteRow {
    key: RowKey,
    title: String,
    section: String,
    hint: Option<String>,
}

#[derive(Clone, Copy)]
enum RowKey {
    Command(CommandId),
    Tab(u64),
}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div();
        }

        let t = self.theme.read(cx);
        let (bg, fg, muted, border, card) = (
            t.background(),
            t.foreground(),
            t.muted_foreground(),
            t.border(),
            t.card(),
        );
        // D1 — cmdk command items use `data-selected:bg-muted`; a pointer hover
        // gets the same fill (the reference has no separate hover state).
        let sel_fill = t.selected_fill();

        let placeholder = match self.page {
            Page::Root => "Search commands\u{2026}",
            Page::SwitchTab => "Search open tabs\u{2026}",
        };
        let (input_text, input_color) = if self.query.is_empty() {
            (placeholder.to_string(), muted)
        } else {
            (self.query.clone(), fg)
        };

        let rows = self.rows(cx);
        let selected = self.selected;

        let mut list = div().flex().flex_col().py(px(4.0)).max_h(px(360.0));
        if rows.is_empty() {
            list = list.child(
                div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .text_size(px(11.0))
                    .text_color(muted)
                    .child("No matching commands"),
            );
        }
        let mut last_section: Option<String> = None;
        for (i, row) in rows.iter().enumerate() {
            if last_section.as_deref() != Some(row.section.as_str()) {
                list = list.child(
                    div()
                        .px(px(12.0))
                        .pt(px(6.0))
                        .pb(px(2.0))
                        .text_size(px(9.0))
                        .text_color(muted)
                        .child(SharedString::from(row.section.to_uppercase())),
                );
                last_section = Some(row.section.clone());
            }
            let is_sel = i == selected;
            let hint = row.hint.clone();
            list = list.child(
                div()
                    .id(("palette-row", i))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .mx(px(4.0))
                    .px(px(crate::theme::menu_metrics::ITEM_PAD_X))
                    .h(px(26.0))
                    .rounded_sm()
                    .text_size(px(12.0))
                    .text_color(fg)
                    .when(is_sel, |d| d.bg(sel_fill))
                    .when(!is_sel, |d| d.hover(|s| s.bg(sel_fill)))
                    .child(SharedString::from(row.title.clone()))
                    .when_some(hint, |d, h| {
                        d.child(
                            div()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child(SharedString::from(h)),
                        )
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.selected = i;
                        this.run_selected(window, cx);
                    })),
            );
        }

        div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(px(80.0))
            .bg(crate::theme::modal_scrim())
            .track_focus(&self.focus)
            .key_context("CommandPalette")
            .on_key_down(cx.listener(Self::on_key))
            .child(
                div()
                    .w(px(520.0))
                    .max_h(px(440.0))
                    .flex()
                    .flex_col()
                    .rounded_md()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .child(
                        div()
                            .h(px(36.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .border_b_1()
                            .border_color(border)
                            .bg(bg)
                            .text_size(px(12.0))
                            .text_color(input_color)
                            .child(SharedString::from(input_text)),
                    )
                    .child(list),
            )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shortcut_id_has_one_entry_and_parses() {
        for s in shortcuts() {
            assert_eq!(shortcut(s.id).id, s.id);
            gpui::Keystroke::parse(s.binding)
                .unwrap_or_else(|e| panic!("bad binding {:?}: {e:?}", s.binding));
        }
    }

    #[test]
    fn default_bindings_have_no_conflicts() {
        for s in shortcuts() {
            assert_eq!(
                find_conflict(s.binding, Some(s.id)),
                None,
                "default binding {:?} conflicts",
                s.binding
            );
        }
    }

    #[test]
    fn conflict_detects_duplicate_shortcut() {
        // cmd-t is TabNew; trying to bind it elsewhere collides.
        assert_eq!(
            find_conflict("cmd-t", Some(ShortcutId::CommandPalette)),
            Some(Conflict::Shortcut(ShortcutId::TabNew)),
        );
        // Modifier order does not matter.
        assert_eq!(
            find_conflict("shift-cmd-d", Some(ShortcutId::CommandPalette)),
            Some(Conflict::Shortcut(ShortcutId::PaneSplitDown)),
        );
    }

    #[test]
    fn conflict_detects_reserved_accelerator() {
        assert_eq!(
            find_conflict("cmd-,", None),
            Some(Conflict::Reserved("Settings")),
        );
        assert_eq!(
            find_conflict("cmd-shift-n", None),
            Some(Conflict::Reserved("New SSH Connection")),
        );
    }

    #[test]
    fn conflict_none_for_free_binding() {
        assert_eq!(find_conflict("cmd-shift-y", None), None);
    }

    #[test]
    fn slug_roundtrips_for_every_shortcut() {
        for s in shortcuts() {
            assert_eq!(shortcut_from_slug(shortcut_slug(s.id)), Some(s.id));
        }
    }

    #[test]
    fn first_start_uses_registry_defaults() {
        let empty = KeybindMap::new();
        for s in shortcuts() {
            assert_eq!(
                effective_binding(s.id, &empty).as_deref(),
                Some(s.binding),
                "{:?} default binding",
                s.id
            );
        }
    }

    #[test]
    fn effective_binding_honours_overrides() {
        let mut m = KeybindMap::new();
        assert_eq!(
            effective_binding(ShortcutId::TabNew, &m).as_deref(),
            Some("cmd-t")
        );
        m.insert("tab.new".into(), "cmd-shift-t".into());
        assert_eq!(
            effective_binding(ShortcutId::TabNew, &m).as_deref(),
            Some("cmd-shift-t")
        );
        m.insert("tab.new".into(), String::new());
        assert_eq!(effective_binding(ShortcutId::TabNew, &m), None);
    }

    #[test]
    fn resolve_conflict_follows_rebinds() {
        let mut m = KeybindMap::new();
        // cmd-t is TabNew's default → collides for anyone else.
        assert_eq!(
            resolve_conflict("cmd-t", Some(ShortcutId::CommandPalette), &m),
            Some(Conflict::Shortcut(ShortcutId::TabNew))
        );
        // Disable TabNew → cmd-t is now free.
        m.insert("tab.new".into(), String::new());
        assert_eq!(
            resolve_conflict("cmd-t", Some(ShortcutId::CommandPalette), &m),
            None
        );
        // Rebind CommandPalette onto cmd-t → it becomes the new owner.
        m.insert("command.palette".into(), "cmd-t".into());
        assert_eq!(
            resolve_conflict("cmd-t", None, &m),
            Some(Conflict::Shortcut(ShortcutId::CommandPalette))
        );
        // Reserved accelerators stay blocked regardless of overrides.
        assert_eq!(
            resolve_conflict("cmd-,", None, &m),
            Some(Conflict::Reserved("Settings"))
        );
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
            context_of(TabKind::Workspace, false),
            Some(CommandContext::Terminal)
        );
        assert_eq!(
            context_of(TabKind::Workspace, true),
            Some(CommandContext::SshTerminal)
        );
        assert_eq!(
            context_of(TabKind::Editor, false),
            Some(CommandContext::Editor)
        );
        assert_eq!(context_of(TabKind::GitGraph, false), None);
    }
}
