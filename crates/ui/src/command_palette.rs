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
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use crate::components::IconName;
use crate::settings::PreferencesStore;
use crate::tabs::TabKind;
use crate::theme::{EditorThemeId, ThemePreference, ThemeStore};
use crate::workspace::Workspace;
use labonair_backend::modules::settings::preferences::PaletteSearchMode;

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
    Shortcut { id: ShortcutsOpen,   label: "Show keyboard shortcuts",   keys: &["\u{2318}", "?"],                 group: General,   binding: "cmd-shift-/" },
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
/// can block a user from rebinding onto one.
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
    OpenAiSettings,
    ToggleEditorWordWrap,
    ToggleLineNumbers,
    ToggleFormatOnSave,
    ToggleCursorBlink,
    TogglePaneHeader,
    TogglePaneFooter,
    ToggleVimMode,
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
    HostsSsh,
    HostsSftp,
    Snippets,
    AiSessions,
    Outline,
    GitBranches,
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
            Page::HostsSsh => "Search SSH hosts\u{2026}",
            Page::HostsSftp => "Search SFTP hosts\u{2026}",
            Page::Snippets => "Search snippets\u{2026}",
            Page::AiSessions => "Search sessions\u{2026}",
            Page::Outline => "Search symbols\u{2026}",
            Page::GitBranches => "Search branches\u{2026}",
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
            Page::HostsSsh => "SSH Hosts",
            Page::HostsSftp => "SFTP Hosts",
            Page::Snippets => "Snippets",
            Page::AiSessions => "AI Sessions",
            Page::Outline => "Symbols",
            Page::GitBranches => "Branches",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fuzzy / prefix / substring matcher (port of the reference `filter` in
// `CommandPalette.tsx`, which switches on `commandPaletteSearchMode`)
// ─────────────────────────────────────────────────────────────────────────────

/// The three search modes the reference footer cycles through.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SearchMode {
    Contains,
    StartsWith,
    Fuzzy,
}

impl SearchMode {
    pub fn from_pref(p: PaletteSearchMode) -> Self {
        match p {
            PaletteSearchMode::Contains => SearchMode::Contains,
            PaletteSearchMode::StartsWith => SearchMode::StartsWith,
            PaletteSearchMode::Fuzzy => SearchMode::Fuzzy,
        }
    }
    pub fn to_pref(self) -> PaletteSearchMode {
        match self {
            SearchMode::Contains => PaletteSearchMode::Contains,
            SearchMode::StartsWith => PaletteSearchMode::StartsWith,
            SearchMode::Fuzzy => PaletteSearchMode::Fuzzy,
        }
    }
    /// Next mode in the cycle (matches the reference `cycleSearchMode` order).
    pub fn next(self) -> Self {
        match self {
            SearchMode::Contains => SearchMode::StartsWith,
            SearchMode::StartsWith => SearchMode::Fuzzy,
            SearchMode::Fuzzy => SearchMode::Contains,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            SearchMode::Contains => "contains",
            SearchMode::StartsWith => "startsWith",
            SearchMode::Fuzzy => "fuzzy",
        }
    }
}

/// Score `needle` against `haystack` under `mode`. `None` = no match.
/// Higher score = better; an empty needle matches everything with score `0`.
/// The port adds ranking (the reference `filter` is boolean) so results order
/// by relevance, mirroring `cmdk`'s built-in scoring.
pub fn match_score(mode: SearchMode, haystack: &str, needle: &str) -> Option<i64> {
    let h = haystack.to_lowercase();
    let n = needle.trim().to_lowercase();
    if n.is_empty() {
        return Some(0);
    }
    match mode {
        SearchMode::StartsWith => h.starts_with(&n).then_some(1_000),
        SearchMode::Contains => h.find(&n).map(|idx| 1_000 - idx as i64),
        SearchMode::Fuzzy => {
            let hb: Vec<char> = h.chars().collect();
            let nb: Vec<char> = n.chars().collect();
            let mut hi = 0;
            let mut score = 0i64;
            let mut last_hit: Option<usize> = None;
            for &nc in &nb {
                let mut found = false;
                while hi < hb.len() {
                    if hb[hi] == nc {
                        if last_hit == Some(hi.wrapping_sub(1)) {
                            score += 8; // consecutive-match bonus
                        }
                        if hi == 0 {
                            score += 6; // start-of-string bonus
                        }
                        last_hit = Some(hi);
                        hi += 1;
                        found = true;
                        break;
                    }
                    score -= 1; // gap penalty
                    hi += 1;
                }
                if !found {
                    return None;
                }
            }
            Some(score)
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
    Command { id: CommandId::ConnectSsh,         title: "Connect SSH\u{2026}",     section: "Connections",    contexts: &[],                            shortcut: None,                icon: I::Terminal,   sub_page: Some(Page::HostsSsh) },
    Command { id: CommandId::OpenSftp,           title: "Open SFTP\u{2026}",       section: "Connections",    contexts: &[],                            shortcut: None,                icon: I::Folder,     sub_page: Some(Page::HostsSftp) },
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
/// `rightLabel` states. `AppShell` rebuilds this each render and hands it over
/// via [`CommandPalette::set_data`]. Domains not wired yet (hosts, snippets,
/// AI sessions, git branches, editor outline) stay empty until their block
/// lands — the pages exist and render a clean empty state meanwhile.
#[derive(Clone, Debug, Default)]
pub struct PaletteData {
    pub tabs: Vec<PaletteChoice>,
    pub hosts: Vec<PaletteChoice>,
    pub ai_sessions: Vec<PaletteChoice>,
    pub snippets: Vec<PaletteChoice>,
    pub git_branches: Vec<PaletteChoice>,
    pub symbols: Vec<PaletteChoice>,
    pub app_themes: Vec<PaletteChoice>,
    pub color_mode: ThemePreference,
    pub editor_theme: EditorThemeId,
    pub font_size: Option<u32>,
    /// camelCase pref key → current bool, for `Toggle: …` `rightLabel`s.
    pub toggles: std::collections::HashMap<&'static str, bool>,
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
    /// Non-actionable (empty-state placeholder line).
    Noop,
}

struct PaletteRow {
    key: RowKey,
    icon: Option<IconName>,
    title: String,
    subtitle: Option<String>,
    section: String,
    keys: Vec<String>,
    right_label: Option<String>,
    has_sub: bool,
}

/// The Cmd+P command palette overlay.
pub struct CommandPalette {
    theme: Entity<ThemeStore>,
    workspace: Entity<Workspace>,
    prefs: Entity<PreferencesStore>,
    open: bool,
    /// Navigation stack — `[Root]` at rest, pushed on drill-in.
    pages: Vec<Page>,
    query: String,
    selected: usize,
    recent: Vec<CommandId>,
    data: PaletteData,
    focus: FocusHandle,
}

impl EventEmitter<PaletteEvent> for CommandPalette {}

impl CommandPalette {
    pub fn new(
        theme: Entity<ThemeStore>,
        workspace: Entity<Workspace>,
        prefs: Entity<PreferencesStore>,
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

    /// Refresh the live-state snapshot (called from `AppShell::render`).
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

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.query.clear();
        self.pages = vec![Page::Root];
        self.selected = 0;
        cx.emit(PaletteEvent::PreviewAppTheme(None));
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
        self.workspace.read(cx).active_context(cx)
    }

    fn search_mode(&self, cx: &App) -> SearchMode {
        SearchMode::from_pref(self.prefs.read(cx).get().command_palette_search_mode)
    }

    fn push_recent(&mut self, id: CommandId, cx: &App) {
        let max = self
            .prefs
            .read(cx)
            .get()
            .command_palette_history_size
            .max(1) as usize;
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

    fn rows(&self, cx: &App) -> Vec<PaletteRow> {
        let mode = self.search_mode(cx);
        match self.page() {
            Page::Root => {
                let ctx = self.active_context(cx);
                search_mode(&self.query, ctx, mode)
                    .into_iter()
                    .map(|c| {
                        let right_label = toggle_pref_key(c.id).map(|k| {
                            if *self.data.toggles.get(k).unwrap_or(&false) {
                                "ON".to_string()
                            } else {
                                "OFF".to_string()
                            }
                        });
                        let subtitle = match c.id {
                            CommandId::AdjustFontSize => {
                                self.data.font_size.map(|s| format!("{s}px"))
                            }
                            CommandId::SwitchTab => Some(format!("{} open", self.data.tabs.len())),
                            _ => None,
                        };
                        PaletteRow {
                            key: c
                                .sub_page
                                .map(RowKey::Navigate)
                                .unwrap_or(RowKey::Command(c.id)),
                            icon: Some(c.icon),
                            title: c.title.to_string(),
                            subtitle,
                            section: c.section.to_string(),
                            keys: c
                                .shortcut
                                .map(|s| shortcut_keys(s).iter().map(|k| k.to_string()).collect())
                                .unwrap_or_default(),
                            right_label,
                            has_sub: c.sub_page.is_some(),
                        }
                    })
                    .collect()
            }
            Page::Tabs => {
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
                        icon: Some(IconName::Terminal),
                        title: t.label(),
                        subtitle: Some(t.kind.default_title().to_string()),
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
                icon: Some(IconName::ArrowDownUp),
                title: title.to_string(),
                subtitle: self.data.font_size.map(|s| format!("{s}px")),
                section: "Font Size".to_string(),
                keys: shortcut_keys(sc).iter().map(|k| k.to_string()).collect(),
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
                icon: Some(IconName::Refresh),
                title: title.to_string(),
                subtitle: None,
                section: "Color Mode".to_string(),
                keys: vec![],
                right_label: (self.data.color_mode == pref).then(|| "active".to_string()),
                has_sub: false,
            })
            .collect(),
            Page::EditorTheme => EditorThemeId::ALL
                .into_iter()
                .map(|id| (id, editor_theme_label(id)))
                .filter(|(_, label)| match_score(mode, label, &self.query).is_some())
                .map(|(id, label)| PaletteRow {
                    key: RowKey::SetEditorTheme(id),
                    icon: Some(IconName::Sparkles),
                    title: label,
                    subtitle: None,
                    section: "Editor Themes".to_string(),
                    keys: vec![],
                    right_label: (self.data.editor_theme == id).then(|| "active".to_string()),
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
            Page::HostsSsh => self.choice_rows(
                &self.data.hosts,
                "SSH Hosts",
                IconName::Terminal,
                mode,
                "No hosts — add one in the Host Manager",
                |c| RowKey::ConnectHost {
                    host_id: c.id.clone(),
                    sftp: false,
                },
            ),
            Page::HostsSftp => self.choice_rows(
                &self.data.hosts,
                "SFTP Hosts",
                IconName::Folder,
                mode,
                "No hosts — add one in the Host Manager",
                |c| RowKey::ConnectHost {
                    host_id: c.id.clone(),
                    sftp: true,
                },
            ),
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
                |_| RowKey::Noop,
            ),
            Page::GitBranches => self.choice_rows(
                &self.data.git_branches,
                "Branches",
                IconName::GitBranch,
                mode,
                "No repository detected",
                |c| RowKey::SwitchBranch(c.id.clone()),
            ),
        }
    }

    fn run_selected(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let Some(row) = rows.get(self.selected) else {
            return;
        };
        match row.key.clone() {
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
        self.prefs.update(cx, |s, cx| {
            s.set_value(
                "commandPaletteSearchMode",
                serde_json::Value::String(next.label().to_string()),
                cx,
            )
        });
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

impl Focusable for CommandPalette {
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

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        let p = self.prefs.read(cx).get();
        let opacity = (p.command_palette_opacity as f32 / 100.0).clamp(0.35, 1.0);
        let position = p.command_palette_position.clone();
        let show_recent = p.command_palette_show_recent;
        let close_on_overlay = p.command_palette_close_on_overlay_click;
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
            let avail: std::collections::HashSet<CommandId> =
                available(ctx).into_iter().map(|c| c.id).collect();
            for id in self.recent.iter().copied().filter(|id| avail.contains(id)) {
                let c = command(id);
                rows.push(PaletteRow {
                    key: c
                        .sub_page
                        .map(RowKey::Navigate)
                        .unwrap_or(RowKey::Command(id)),
                    icon: Some(c.icon),
                    title: c.title.to_string(),
                    subtitle: None,
                    section: "Recently Used".to_string(),
                    keys: c
                        .shortcut
                        .map(|s| shortcut_keys(s).iter().map(|k| k.to_string()).collect())
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
            .bg(crate::theme::modal_scrim())
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

    // ── Block D: fuzzy matcher / search modes / sub-pages ─────────────────

    #[test]
    fn match_score_modes() {
        // Empty needle always matches.
        assert!(match_score(SearchMode::Fuzzy, "anything", "").is_some());
        // StartsWith is anchored.
        assert!(match_score(SearchMode::StartsWith, "split pane right", "split").is_some());
        assert!(match_score(SearchMode::StartsWith, "split pane right", "pane").is_none());
        // Contains is a substring anywhere, earlier = higher score.
        let early = match_score(SearchMode::Contains, "split pane", "split").unwrap();
        let late = match_score(SearchMode::Contains, "split pane", "pane").unwrap();
        assert!(early > late);
        // Fuzzy matches a subsequence with gaps.
        assert!(match_score(SearchMode::Fuzzy, "split pane right", "spr").is_some());
        assert!(match_score(SearchMode::Fuzzy, "split pane right", "xyz").is_none());
        // Consecutive letters outrank scattered ones.
        let consec = match_score(SearchMode::Fuzzy, "format document", "format").unwrap();
        let scattered = match_score(SearchMode::Fuzzy, "focus source control mode", "format");
        assert!(scattered.is_none() || consec > scattered.unwrap());
    }

    #[test]
    fn search_mode_cycles_and_maps_prefs() {
        assert_eq!(SearchMode::Contains.next(), SearchMode::StartsWith);
        assert_eq!(SearchMode::StartsWith.next(), SearchMode::Fuzzy);
        assert_eq!(SearchMode::Fuzzy.next(), SearchMode::Contains);
        for m in [
            SearchMode::Contains,
            SearchMode::StartsWith,
            SearchMode::Fuzzy,
        ] {
            assert_eq!(SearchMode::from_pref(m.to_pref()), m);
        }
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
            Page::HostsSsh,
            Page::HostsSftp,
            Page::Snippets,
            Page::AiSessions,
            Page::Outline,
            Page::GitBranches,
        ];
        let labels: std::collections::HashSet<_> = pages.iter().map(|p| p.label()).collect();
        assert_eq!(labels.len(), pages.len());
    }
}
