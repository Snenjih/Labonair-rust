//! Loads, merges, validates and live-reloads `keymap.json` (T19-008), and
//! feeds the result into [`crate::menu::apply_keymap`] plus the
//! `KeybindDisplay` GPUI global the command palette and panel-toggle
//! tooltips read to show effective bindings.
//!
//! The keymap module owns loading, merging, validation and recovery. This file
//! only adapts its immutable snapshot to GPUI bindings, display hints, and the
//! existing filesystem watcher.

use gpui::App;
use labonair_command_palette::{
    canonical_action_name, shortcut_slug, shortcuts, KeybindDisplay, KeybindMap,
};
use labonair_keymap::file::{self, EffectiveBinding, ValidationIssue};

use crate::commands::CommandDispatcher;

/// The current validation issues from the last `keymap.json` (re)load — for a
/// settings-window banner to surface. Empty when the file is missing, empty,
/// or fully valid.
pub fn last_issues() -> Vec<ValidationIssue> {
    file::last_issues()
}

/// The merged effective keymap: shipped defaults, then the user's
/// `keymap.json` (or the last known-good snapshot of it) on top.
#[cfg(test)]
pub(crate) fn effective_bindings(registry: &CommandDispatcher) -> Vec<EffectiveBinding> {
    labonair_keymap::adapter::load_descriptors(registry.iter()).effective_bindings
}

/// Derive the command-keyed display snapshot used by the palette, plus the
/// temporary `ShortcutId` compatibility map still consumed by panel-toggle
/// tooltips. The command map is context-agnostic by design: it picks the first
/// effective binding for each command.
fn display_map(effective: &[EffectiveBinding], registry: &CommandDispatcher) -> KeybindDisplay {
    let mut legacy = KeybindMap::new();
    let mut by_command = registry
        .iter()
        .map(|descriptor| (descriptor.id, None))
        .collect::<std::collections::HashMap<_, _>>();
    for s in shortcuts() {
        let Some(cmd_id) = registry.command_for_shortcut(s.id) else {
            continue;
        };
        let action_name = cmd_id.action_name();
        match effective
            .iter()
            .find(|b| canonical_action_name(&b.action) == Some(action_name))
        {
            Some(b) if b.keystrokes != s.binding => {
                legacy.insert(shortcut_slug(s.id).to_string(), b.keystrokes.clone());
            }
            Some(_) => {}
            None => {
                legacy.insert(shortcut_slug(s.id).to_string(), String::new());
            }
        }
    }
    for binding in effective {
        if let Some(command) = labonair_keymap::runtime::command_for_action(&binding.action) {
            by_command.entry(command).and_modify(|current| {
                if current.is_none() {
                    *current = Some(binding.keystrokes.clone());
                }
            });
        }
    }
    KeybindDisplay { by_command, legacy }
}

/// Load, merge, bind and publish the display global — the single entry point
/// called at startup and on live-reload.
pub(crate) fn reload_and_apply(cx: &mut App, registry: &CommandDispatcher) {
    let snapshot = labonair_keymap::adapter::load_descriptors(registry.iter());
    let effective = snapshot.effective_bindings;
    crate::menu::apply_keymap(cx, &effective);
    cx.set_global(display_map(&effective, registry));
}

/// Start the live fs-watch on `keymap.json` (T19-008 Anweisung #6). Call once
/// at startup, after the first [`reload_and_apply`].
pub(crate) fn watch(cx: &App, registry: CommandDispatcher) {
    labonair_settings::watch_file(cx, file::user_keymap_path(), move |cx| {
        reload_and_apply(cx, &registry);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `effective_bindings()` always includes the shipped default asset's
    /// bindings when no user `keymap.json` exists in this process's config
    /// dir (CI/test environments don't have one).
    #[test]
    fn effective_bindings_include_defaults() {
        let default = file::parse_keymap_jsonc(file::default_asset()).unwrap();
        let registry = crate::commands::register_builtin_commands();
        let effective = effective_bindings(&registry);
        let default_actions: std::collections::BTreeSet<&str> = default
            .0
            .iter()
            .flat_map(|b| b.bindings.iter())
            .filter_map(|(_, a)| a.as_deref())
            .filter_map(canonical_action_name)
            .collect();
        let effective_actions: std::collections::BTreeSet<&str> = effective
            .iter()
            .filter_map(|binding| canonical_action_name(&binding.action))
            .collect();
        for action in default_actions {
            assert!(
                effective_actions.contains(action),
                "default action {action} missing from effective bindings"
            );
        }
    }

    #[test]
    fn display_map_omits_unshifted_defaults() {
        let registry = crate::commands::register_builtin_commands();
        let effective = effective_bindings(&registry);
        let map = display_map(&effective, &registry);
        // `TabNew`'s default (`cmd-t`) is unchanged in a clean environment,
        // so it must not appear as an "override" in the display map.
        assert!(!map
            .legacy
            .contains_key(shortcut_slug(labonair_command_palette::ShortcutId::TabNew)));
    }
}
