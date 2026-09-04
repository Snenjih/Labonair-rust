//! Loads, merges, validates and live-reloads `keymap.json` (T19-008), and
//! feeds the result into [`crate::menu::apply_keymap`] plus the
//! `KeybindDisplay` GPUI global the Shortcuts pane / command palette /
//! panel-toggle tooltips read to show effective bindings.
//!
//! This is the one place in the app that bridges `labonair-settings::keymap`
//! (pure, decoupled from `CommandId`) with `labonair-command-palette`
//! (`CommandId`, `ShortcutId`) and `labonair-shell::menu` (concrete `Action`
//! types) — the shell is the only crate allowed to see all three.

use std::cell::RefCell;

use gpui::App;
use labonair_command_palette::{
    command_for_shortcut, known_action_names, shortcut_slug, shortcuts, KeybindDisplay, KeybindMap,
};
use labonair_settings::keymap::{
    self, merge_keymaps, parse_keymap_jsonc, validate_keymap, EffectiveBinding, KeybindSource,
    KeymapFile, Severity, ValidationIssue,
};

thread_local! {
    /// The last successfully-parsed user `keymap.json`. Kept across a reload
    /// that fails to parse, so a syntax error in the file never regresses
    /// live bindings back to "all defaults" (T19-008 Anweisung #6 — "kaputte
    /// Datei -> Banner + letzte gute Keymap"). `RefCell` because this is
    /// mutated from plain `&App` call sites (no `&mut` available at every
    /// read/reload boundary) and the app is single-threaded on the main GPUI
    /// thread where this is ever touched.
    static LAST_GOOD_USER: RefCell<KeymapFile> = RefCell::new(KeymapFile::default());
    static LAST_ISSUES: RefCell<Vec<ValidationIssue>> = const { RefCell::new(Vec::new()) };
}

/// The current validation issues from the last `keymap.json` (re)load — for a
/// settings-window banner or the Shortcuts pane to surface. Empty when the
/// file is missing, empty, or fully valid.
pub fn last_issues() -> Vec<ValidationIssue> {
    LAST_ISSUES.with(|c| c.borrow().clone())
}

fn known_actions() -> std::collections::BTreeSet<&'static str> {
    known_action_names()
}

/// Parse + validate the user `keymap.json`. On success, caches it as the new
/// "last known good" file. On a parse/structural failure, keeps the previous
/// last-good file and records the issue. Missing file = an empty (valid)
/// keymap, not an error.
fn load_user_keymap() -> KeymapFile {
    let path = keymap::user_keymap_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        LAST_ISSUES.with(|c| c.borrow_mut().clear());
        LAST_GOOD_USER.with(|c| *c.borrow_mut() = KeymapFile::default());
        return KeymapFile::default();
    };

    match parse_keymap_jsonc(&text) {
        Ok(file) => {
            let issues = validate_keymap(&file, &known_actions(), &text);
            let has_errors = issues.iter().any(|i| i.severity == Severity::Error);
            if has_errors {
                tracing::warn!(
                    path = %path.display(),
                    "keymap.json has validation errors — keeping the last good keymap"
                );
                LAST_ISSUES.with(|c| *c.borrow_mut() = issues);
                LAST_GOOD_USER.with(|c| c.borrow().clone())
            } else {
                LAST_ISSUES.with(|c| *c.borrow_mut() = issues);
                LAST_GOOD_USER.with(|c| *c.borrow_mut() = file.clone());
                file
            }
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "keymap.json is not valid JSON/JSONC — keeping the last good keymap"
            );
            LAST_ISSUES.with(|c| {
                *c.borrow_mut() = vec![ValidationIssue {
                    message: e.message,
                    line: e.line,
                    severity: Severity::Error,
                }]
            });
            LAST_GOOD_USER.with(|c| c.borrow().clone())
        }
    }
}

/// Parse the shipped default keymap for this platform. Not user data — a
/// parse failure here is a build-time bug, not a runtime condition, so it
/// only warns and falls back to an empty keymap rather than panicking the
/// running app.
fn load_default_keymap() -> KeymapFile {
    match parse_keymap_jsonc(keymap::default_asset()) {
        Ok(file) => file,
        Err(e) => {
            tracing::error!(error = %e, "shipped default keymap failed to parse");
            KeymapFile::default()
        }
    }
}

/// The merged effective keymap: shipped defaults, then the user's
/// `keymap.json` (or the last known-good snapshot of it) on top.
pub fn effective_bindings() -> Vec<EffectiveBinding> {
    let default = load_default_keymap();
    let user = load_user_keymap();
    merge_keymaps(&[
        (KeybindSource::Default, &default),
        (KeybindSource::User, &user),
    ])
}

/// Derive the `ShortcutId`-keyed display map ([`KeybindDisplay`]) the
/// Shortcuts pane / command palette / panel-toggle tooltips render: absent =
/// "runs on the `SHORTCUTS` table default", `Some(keystrokes)` = overridden,
/// `Some("")` = explicitly unbound. Context-agnostic by design (T19-008's
/// documented scope reduction — the display picks the first effective
/// binding for the command regardless of context).
fn display_map(effective: &[EffectiveBinding]) -> KeybindMap {
    let mut map = KeybindMap::new();
    for s in shortcuts() {
        let Some(cmd_id) = command_for_shortcut(s.id) else {
            continue;
        };
        let action_name = cmd_id.action_name();
        match effective.iter().find(|b| b.action == action_name) {
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
/// called at startup, on live-reload, and after the Shortcuts pane writes a
/// surgical `keymap.json` edit.
pub fn reload_and_apply(cx: &mut App) {
    let effective = effective_bindings();
    crate::menu::apply_keymap(cx, &effective);
    cx.set_global(KeybindDisplay(display_map(&effective)));
}

/// Start the live fs-watch on `keymap.json` (T19-008 Anweisung #6). Call once
/// at startup, after the first [`reload_and_apply`].
pub fn watch(cx: &App) {
    labonair_settings::watch_file(cx, keymap::user_keymap_path(), |cx| {
        reload_and_apply(cx);
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
        let default = load_default_keymap();
        let effective = effective_bindings();
        let default_actions: std::collections::BTreeSet<&str> = default
            .0
            .iter()
            .flat_map(|b| b.bindings.iter())
            .filter_map(|(_, a)| a.as_deref())
            .collect();
        let effective_actions: std::collections::BTreeSet<&str> =
            effective.iter().map(|b| b.action.as_str()).collect();
        for action in default_actions {
            assert!(
                effective_actions.contains(action),
                "default action {action} missing from effective bindings"
            );
        }
    }

    #[test]
    fn display_map_omits_unshifted_defaults() {
        let effective = effective_bindings();
        let map = display_map(&effective);
        // `TabNew`'s default (`cmd-t`) is unchanged in a clean environment,
        // so it must not appear as an "override" in the display map.
        assert!(!map.contains_key(shortcut_slug(labonair_command_palette::ShortcutId::TabNew)));
    }
}
