//! Keyboard-shortcut model: the rebindable [`ShortcutId`] table (port of the
//! reference `shortcuts.ts`), user keybind overrides ([`KeybindMap`]) and
//! conflict detection.

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
/// the native menu and never customizable. Kept here so conflict detection
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

/// Split a GPUI keystroke string (`"cmd-shift-p"`, `"cmd--"`) into display
/// tokens (`["\u{2318}", "\u{21e7}", "P"]`) — the modifier glyphs the cheat
/// sheet and command palette render right-aligned.
pub fn keystroke_tokens(binding: &str) -> Vec<String> {
    let mut parts: Vec<&str> = binding.split('-').filter(|s| !s.is_empty()).collect();
    let key = if binding.ends_with("--") {
        parts.pop();
        "-".to_string()
    } else {
        parts.pop().unwrap_or("").to_string()
    };
    let mut out: Vec<String> = parts
        .iter()
        .map(|m| match *m {
            "ctrl" | "control" => "\u{2303}".to_string(),
            "alt" | "option" => "\u{2325}".to_string(),
            "shift" => "\u{21e7}".to_string(),
            "cmd" | "super" | "platform" | "win" => "\u{2318}".to_string(),
            other => other.to_string(),
        })
        .collect();
    if !key.is_empty() {
        out.push(if key.chars().count() == 1 {
            key.to_uppercase()
        } else {
            key
        });
    }
    out
}

/// Display tokens for a shortcut, honouring user overrides. Falls back to the
/// registry's cheat-sheet glyphs ([`shortcut_keys`]) when the shortcut runs on
/// its default binding; an explicitly-unbound shortcut yields no tokens.
pub fn effective_keys(id: ShortcutId, overrides: &KeybindMap) -> Vec<String> {
    match overrides.get(shortcut_slug(id)) {
        None => shortcut_keys(id).iter().map(|k| k.to_string()).collect(),
        Some(s) if s.is_empty() => Vec::new(),
        Some(s) => keystroke_tokens(s),
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
}
