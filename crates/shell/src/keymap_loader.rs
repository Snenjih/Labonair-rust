//! Loads, merges, validates and live-reloads `keymap.json` (T19-008), and
//! feeds the result into [`crate::menu::apply_keymap`] plus the
//! `KeybindDisplay` GPUI global the command palette and panel-toggle
//! tooltips read to show effective bindings.
//!
//! This is the one place in the app that bridges `labonair-settings::keymap`
//! (pure, decoupled from `CommandId`) with `labonair-command-palette`
//! (`CommandId`, `ShortcutId`) and `labonair-shell::menu` (concrete `Action`
//! types) — the shell is the only crate allowed to see all three.

use gpui::App;
use labonair_command_palette::{
    canonical_action_name, compatibility_action_names, shortcut_slug, shortcuts, KeybindDisplay,
    KeybindMap,
};
use labonair_keymap::file::{self, EffectiveBinding, ValidationIssue};

use crate::commands::CommandDispatcher;

/// The current validation issues from the last `keymap.json` (re)load — for a
/// settings-window banner to surface. Empty when the file is missing, empty,
/// or fully valid.
pub fn last_issues() -> Vec<ValidationIssue> {
    file::last_issues()
}

fn known_actions(registry: &CommandDispatcher) -> std::collections::BTreeSet<&'static str> {
    let mut actions = registry
        .iter()
        .map(|descriptor| descriptor.id.action_name())
        .collect::<std::collections::BTreeSet<_>>();
    actions.extend(compatibility_action_names());
    actions
}

/// The merged effective keymap: shipped defaults, then the user's
/// `keymap.json` (or the last known-good snapshot of it) on top.
pub(crate) fn effective_bindings(registry: &CommandDispatcher) -> Vec<EffectiveBinding> {
    let owner_defaults = labonair_keymap::runtime::defaults_from_descriptors(registry.iter());
    file::effective_bindings_with_defaults(&known_actions(registry), &owner_defaults)
}

/// Derive the `ShortcutId`-keyed display map ([`KeybindDisplay`]) the command
/// palette and panel-toggle tooltips render: absent =
/// "runs on the `SHORTCUTS` table default", `Some(keystrokes)` = overridden,
/// `Some("")` = explicitly unbound. Context-agnostic by design (T19-008's
/// documented scope reduction — the display picks the first effective
/// binding for the command regardless of context).
fn display_map(effective: &[EffectiveBinding], registry: &CommandDispatcher) -> KeybindMap {
    let mut map = KeybindMap::new();
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
                map.insert(shortcut_slug(s.id).to_string(), b.keystrokes.clone());
            }
            Some(_) => {}
            None => {
                map.insert(shortcut_slug(s.id).to_string(), String::new());
            }
        }
    }
    map
}

/// Load, merge, bind and publish the display global — the single entry point
/// called at startup and on live-reload.
pub(crate) fn reload_and_apply(cx: &mut App, registry: &CommandDispatcher) {
    let effective = effective_bindings(registry);
    crate::menu::apply_keymap(cx, &effective);
    cx.set_global(KeybindDisplay(display_map(&effective, registry)));
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
        assert!(!map.contains_key(shortcut_slug(labonair_command_palette::ShortcutId::TabNew)));
    }
}
