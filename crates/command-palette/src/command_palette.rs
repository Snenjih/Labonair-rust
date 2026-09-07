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
//! * `labonair-keymap` — the keymap runtime, user overrides and conflict
//!   detection.
//! * [`palette`] — the command-palette presentation adapter and view.

mod fuzzy;
mod palette;

pub mod command_provider;

pub use fuzzy::{match_score, SearchMode};
pub use labonair_command_palette_core::{
    canonical_action_name, compatibility_action_names, known_action_names, toggle_pref_key,
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandRegistry,
    CommandRegistryError, CommandSubmenu, SubmenuAction, SubmenuDescriptor, SubmenuItem,
    SubmenuProvider, SubmenuRegistry, SubmenuSecondary, SubmenuSnapshot,
};
pub use labonair_keymap::{
    effective_binding, effective_keys, find_conflict, keystroke_tokens, resolve_conflict, shortcut,
    shortcut_from_slug, shortcut_keys, shortcut_slug, shortcuts, Conflict, KeybindMap, Shortcut,
    ShortcutGroup, ShortcutId, RESERVED_ACCELERATORS,
};

/// GPUI-facing publication of the effective keymap. The keymap domain stays
/// UI-free; this wrapper is an adapter consumed by palette/statusbar views.
#[derive(Debug, Clone, Default)]
pub struct KeybindDisplay {
    /// Canonical display lookup keyed by the command identity.
    pub by_command: std::collections::HashMap<CommandId, Option<String>>,
}

impl KeybindDisplay {
    /// Return display tokens for a command. `None` in the map means explicitly
    /// unbound; an absent entry uses the supplied descriptor fallback.
    pub fn keys_for(&self, command: CommandId, fallback: Option<&str>) -> Vec<String> {
        let binding = match self.by_command.get(&command) {
            Some(binding) => binding.as_deref(),
            None => fallback,
        };
        binding
            .map(labonair_keymap::keystroke_tokens)
            .unwrap_or_default()
    }
}

impl gpui::Global for KeybindDisplay {}
pub use palette::{
    context_of, Command, CommandPalette, Page, PaletteData, PaletteEvent, PaletteTabKind,
    PaletteTabRow, PaletteWorkspace,
};
