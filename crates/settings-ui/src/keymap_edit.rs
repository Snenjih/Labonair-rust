//! Surgical `keymap.json` writes for the Shortcuts pane (T19-008).
//!
//! The pane still records a single keystroke at a time (chord *recording* is
//! out of this task's shipped scope — the underlying model already supports
//! chords end-to-end, see `labonair_settings::keymap`); every write here
//! targets the top-level "global" block (`context: null`), matching the
//! pane's pre-T19-008 behaviour of rebinding without context-scoping.
//!
//! An edit always writes at most two things into that block: an explicit
//! `null` for the action's *previous* effective keystrokes (so the shipped
//! default binding doesn't keep firing alongside the new one — the merge key
//! is `(context, keystrokes)`, not `action`) and the new keystrokes → action
//! pair (omitted for a plain unbind).

use std::collections::BTreeSet;

use serde_json::Value;

use labonair_command_palette::{command_for_shortcut, ShortcutId};
use labonair_settings::keymap::{
    default_asset, ensure_user_keymap_file, merge_keymaps, parse_keymap_jsonc, KeybindSource,
    KeymapFile,
};
use labonair_settings_json::{
    append_top_level_array_value_in_json_text, infer_json_indent_size,
    replace_top_level_array_value_in_json_text,
};

fn action_name_for(id: ShortcutId) -> Result<&'static str, String> {
    command_for_shortcut(id)
        .map(|c| c.action_name())
        .ok_or_else(|| "this shortcut has no bindable command".to_string())
}

/// The current effective keystrokes for `action_name` (default keymap merged
/// with whatever the user's `keymap.json` currently contains), or `None` if
/// unbound.
fn effective_keystrokes_for(action_name: &str, user: &KeymapFile) -> Option<String> {
    let default = parse_keymap_jsonc(default_asset()).unwrap_or_default();
    let effective = merge_keymaps(&[
        (KeybindSource::Default, &default),
        (KeybindSource::User, user),
    ]);
    effective
        .iter()
        .find(|b| b.action == action_name)
        .map(|b| b.keystrokes.clone())
}

/// Rebind `id` to `keystrokes` (a single keystroke or, if the pane ever grows
/// chord recording, a space-joined chord).
pub(crate) fn rebind(id: ShortcutId, keystrokes: &str) -> Result<(), String> {
    apply_edits(&[(action_name_for(id)?, Some(keystrokes))])
}

/// Unbind `id` — a `"key": null` entry replacing its current effective
/// keystrokes.
pub(crate) fn unbind(id: ShortcutId) -> Result<(), String> {
    apply_edits(&[(action_name_for(id)?, None)])
}

/// Give `keystrokes` to `id`, taking it away from `other` first (no silent
/// double-binding) — the conflict-resolution "Overwrite" action.
pub(crate) fn rebind_with_overwrite(
    id: ShortcutId,
    other: ShortcutId,
    keystrokes: &str,
) -> Result<(), String> {
    apply_edits(&[
        (action_name_for(other)?, None),
        (action_name_for(id)?, Some(keystrokes)),
    ])
}

/// Reset every shortcut back to its shipped default: strips any user
/// `keymap.json` bindings for actions the [`labonair_command_palette`]
/// `ShortcutId` table knows about (bindings the pane never wrote — e.g. a
/// hand-added context-scoped override — are left untouched).
pub(crate) fn reset_all() -> Result<(), String> {
    let path = ensure_user_keymap_file()?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let user = parse_keymap_jsonc(&text).map_err(|e| e.to_string())?;
    let known_actions: BTreeSet<&'static str> = labonair_command_palette::shortcuts()
        .iter()
        .filter_map(|s| command_for_shortcut(s.id))
        .map(|c| c.action_name())
        .collect();

    let tab_size = infer_json_indent_size(&text);
    let mut next_text = text;
    // Walk blocks/bindings in reverse so earlier `array_index`/key removals
    // don't shift the indices of ones not yet processed.
    for (block_index, block) in user.0.iter().enumerate().rev() {
        for (key, action) in block.bindings.iter().rev() {
            if action.as_deref().is_some_and(|a| known_actions.contains(a)) {
                let (range, replacement) = replace_top_level_array_value_in_json_text(
                    &next_text,
                    &["bindings", key.as_str()],
                    None,
                    None,
                    block_index,
                    tab_size,
                );
                next_text.replace_range(range, &replacement);
            }
        }
    }
    std::fs::write(&path, next_text).map_err(|e| e.to_string())
}

/// Apply `edits` (`action_name -> new keystrokes, or None to unbind`) to the
/// user `keymap.json`'s global (`context: null`) block, creating the file /
/// block if needed. Each edit first nulls out the action's current effective
/// keystrokes (if any and if different from the new ones) so a rebind
/// doesn't leave the old default binding active alongside the new one.
fn apply_edits(edits: &[(&str, Option<&str>)]) -> Result<(), String> {
    let path = ensure_user_keymap_file()?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let user = parse_keymap_jsonc(&text).map_err(|e| e.to_string())?;
    let tab_size = infer_json_indent_size(&text);

    let ops = compute_ops(edits, &user);
    if ops.is_empty() {
        return Ok(());
    }

    let global_index = user.0.iter().position(|b| b.context.is_none());

    let mut next_text = text;
    if let Some(index) = global_index {
        for (key, value) in &ops {
            let (range, replacement) = replace_top_level_array_value_in_json_text(
                &next_text,
                &["bindings", key.as_str()],
                Some(value),
                None,
                index,
                tab_size,
            );
            next_text.replace_range(range, &replacement);
        }
    } else {
        let mut bindings = serde_json::Map::new();
        for (key, value) in &ops {
            bindings.insert(key.clone(), value.clone());
        }
        let block = serde_json::json!({ "context": Value::Null, "bindings": bindings });
        let (range, replacement) =
            append_top_level_array_value_in_json_text(&next_text, &block, tab_size);
        next_text.replace_range(range, &replacement);
    }

    std::fs::write(&path, next_text).map_err(|e| e.to_string())
}

/// Pure diff: what to write into the global block for `edits`, given the
/// user's currently-parsed `keymap.json`. Split out from [`apply_edits`] so
/// the surgical-write logic is unit-testable without ever touching the real
/// per-user config path (`user_keymap_path()` always points at
/// `~/.config/labonair/keymap.json` — there is no directory-override
/// mechanism here the way `PreferencesStore::with_dir` has for prefs, so
/// tests must not perform real I/O against it).
fn compute_ops(edits: &[(&str, Option<&str>)], user: &KeymapFile) -> Vec<(String, Value)> {
    let mut ops: Vec<(String, Value)> = Vec::new();
    for (action_name, new_keystrokes) in edits {
        let old = effective_keystrokes_for(action_name, user);
        if old.as_deref() == *new_keystrokes {
            continue; // already bound to exactly this — nothing to write.
        }
        if let Some(old) = old {
            ops.push((old, Value::Null));
        }
        if let Some(new_keystrokes) = new_keystrokes {
            ops.push((
                new_keystrokes.to_string(),
                Value::String(action_name.to_string()),
            ));
        }
    }
    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebind_nulls_the_old_default_keystrokes_and_binds_the_new_one() {
        let user = KeymapFile::default(); // no user overrides yet
        let ops = compute_ops(&[("tab::NewTerminal", Some("cmd-shift-t"))], &user);
        assert_eq!(
            ops,
            vec![
                ("cmd-t".to_string(), Value::Null),
                (
                    "cmd-shift-t".to_string(),
                    Value::String("tab::NewTerminal".to_string())
                ),
            ]
        );
    }

    #[test]
    fn unbind_only_nulls_the_current_keystrokes() {
        let user = KeymapFile::default();
        let ops = compute_ops(&[("tab::NewTerminal", None)], &user);
        assert_eq!(ops, vec![("cmd-t".to_string(), Value::Null)]);
    }

    #[test]
    fn rebind_to_the_same_keystrokes_is_a_no_op() {
        let user = KeymapFile::default();
        let ops = compute_ops(&[("tab::NewTerminal", Some("cmd-t"))], &user);
        assert!(ops.is_empty());
    }

    #[test]
    fn overwrite_unbinds_other_and_binds_self() {
        let user = KeymapFile::default();
        // `AiToggle`'s default is `cmd-i`; give it to `TabNew` instead. Both
        // the loser's old keystrokes (`cmd-i`) and the winner's own previous
        // default (`cmd-t`) get explicitly nulled.
        let ops = compute_ops(
            &[
                ("ai::TogglePanel", None),
                ("tab::NewTerminal", Some("cmd-i")),
            ],
            &user,
        );
        assert_eq!(
            ops,
            vec![
                ("cmd-i".to_string(), Value::Null),
                ("cmd-t".to_string(), Value::Null),
                (
                    "cmd-i".to_string(),
                    Value::String("tab::NewTerminal".to_string())
                ),
            ]
        );
    }
}
