//! `labonair-command-palette` — the `Cmd+P` command palette plus the
//! command / keyboard-shortcut model shared with the (future) `keymap.json`.
//!
//! Extracted from `crates/ui/src/command_palette.rs` in T16-004. Originally
//! generic over three host contracts; pref/theme-derived reads have since
//! moved onto the layered `labonair-settings` slices directly, so the view is
//! now generic over [`PaletteWorkspace`] / [`labonair_ui_kit::UiTheme`] only.
//! The concrete `Workspace` / `ThemeStore` impls live in `crates/shell`.
//!
//! Layout:
//! * [`fuzzy`] — the `SearchMode` matcher (`match_score`), also used by the AI
//!   composer's `@`-file picker and the settings search.
//! * [`keybind`] — the rebindable [`ShortcutId`] table, [`KeybindMap`] user
//!   overrides and conflict detection.
//! * [`palette`] — the [`Command`] registry plus the [`CommandPalette`] view.

mod fuzzy;
mod keybind;
mod palette;

pub use fuzzy::{match_score, SearchMode};
pub use keybind::{
    effective_binding, effective_keys, find_conflict, keystroke_tokens, resolve_conflict, shortcut,
    shortcut_from_slug, shortcut_keys, shortcut_slug, shortcuts, Conflict, KeybindDisplay,
    KeybindMap, Shortcut, ShortcutGroup, ShortcutId, RESERVED_ACCELERATORS,
};
pub use palette::{
    available, command, command_for_shortcut, commands, context_of, known_action_names, search,
    search_mode, toggle_pref_key, Command, CommandContext, CommandId, CommandPalette, Page,
    PaletteChoice, PaletteData, PaletteEvent, PaletteTabKind, PaletteTabRow, PaletteWorkspace,
};
