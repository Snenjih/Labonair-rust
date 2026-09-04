//! `keymap.json` — parsing, layered merging and validation (T19-008).
//!
//! Pure data/logic, decoupled from `labonair-command-palette`'s `CommandId`
//! on purpose (see `docs/architecture.md`'s crate-graph rule: this crate must
//! not depend on the palette crate). Callers that *do* know the action
//! vocabulary (`labonair-shell`, `labonair-settings-ui`) supply the set of
//! valid action names to [`validate_keymap`] and turn the resulting
//! [`EffectiveBinding`]s into real `gpui::KeyBinding`s themselves.
//!
//! File shape (a JSONC array of blocks):
//! ```jsonc
//! [
//!   { "context": "Workspace", "bindings": {
//!       "cmd-t": "tab::NewTerminal",
//!       "cmd-k cmd-s": "zed::OpenKeymap"   // chord — space-separated keystrokes
//!   }},
//!   { "context": "Editor", "bindings": { "cmd-f": "search::Toggle" } },
//!   { "bindings": { "cmd-w": null } }        // no context = always active;
//!                                             // `null` action = explicit unbind
//! ]
//! ```
//! Chords need no bespoke parser: a keystrokes string is just passed through
//! space-joined to `gpui::KeyBinding::new`/`load`, which already splits it.

use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

/// One `{ context, bindings }` block in a keymap.json array.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapBlock {
    pub context: Option<String>,
    /// `None` action = explicit unbind (`"key": null`). Keys are unique per
    /// JSON object, so insertion order across the vec doesn't matter for
    /// correctness (only cross-block order, i.e. array order, does).
    pub bindings: Vec<(String, Option<String>)>,
}

/// A parsed `keymap.json` document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapFile(pub Vec<KeymapBlock>);

/// Where an [`EffectiveBinding`] came from — shown as the binding's "source"
/// in the Shortcuts settings pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeybindSource {
    Default,
    /// Reserved for a future VS Code / JetBrains preset layer
    /// (`BaseKeymap`, not implemented by this task — see the task's
    /// `## Notizen`).
    BaseKeymap,
    User,
}

#[derive(Debug, Clone)]
pub struct KeymapParseError {
    pub message: String,
    /// 1-based line number, best-effort.
    pub line: usize,
}

impl std::fmt::Display for KeymapParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn line_at(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].matches('\n').count() + 1
}

/// Best-effort line number for a diagnostic about `needle` (an action name or
/// keystroke string) — a plain substring search, matching the precision the
/// rest of the settings track uses for JSONC diagnostics (see
/// `SettingsStore::reload_user_layer`).
fn line_for_needle(text: &str, needle: &str) -> usize {
    text.find(needle).map_or(1, |idx| line_at(text, idx))
}

/// Parse a JSONC `keymap.json` document. A missing/empty file is not this
/// function's concern — callers treat "file absent" as `KeymapFile::default()`
/// before ever calling this.
pub fn parse_keymap_jsonc(text: &str) -> Result<KeymapFile, KeymapParseError> {
    let value = match jsonc_parser::parse_to_serde_value(text, &Default::default()) {
        Ok(v) => v,
        Err(e) => {
            return Err(KeymapParseError {
                message: e.message,
                line: line_at(text, e.range.start),
            });
        }
    };
    let Some(value) = value else {
        return Ok(KeymapFile::default());
    };
    let Value::Array(items) = value else {
        return Err(KeymapParseError {
            message: "keymap.json must be a top-level array".to_string(),
            line: 1,
        });
    };

    let mut blocks = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        let Value::Object(obj) = item else {
            return Err(KeymapParseError {
                message: format!("keymap block {i} must be an object"),
                line: 1,
            });
        };
        let context = match obj.get("context") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(KeymapParseError {
                    message: format!("keymap block {i}: `context` must be a string or null"),
                    line: 1,
                });
            }
        };
        let bindings_value = obj
            .get("bindings")
            .cloned()
            .unwrap_or_else(|| Value::Object(Default::default()));
        let Value::Object(bindings_obj) = bindings_value else {
            return Err(KeymapParseError {
                message: format!("keymap block {i}: `bindings` must be an object"),
                line: 1,
            });
        };
        let mut bindings = Vec::with_capacity(bindings_obj.len());
        for (key, val) in bindings_obj.into_iter() {
            let action = match val {
                Value::Null => None,
                Value::String(s) => Some(s),
                _ => {
                    return Err(KeymapParseError {
                        message: format!(
                            "keymap block {i}: binding `{key}` must be a string action name or null"
                        ),
                        line: 1,
                    });
                }
            };
            bindings.push((key, action));
        }
        blocks.push(KeymapBlock { context, bindings });
    }
    Ok(KeymapFile(blocks))
}

/// Canonicalise a single keystroke (`"shift-cmd-d"` == `"cmd-shift-d"`) —
/// used only for the merge/conflict comparison key, never for the value GPUI
/// actually binds (that stays the original author-written string).
fn normalize_keystroke(ks: &str) -> String {
    let mut parts: Vec<String> = ks
        .split('-')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_lowercase())
        .collect();
    let key = if ks.ends_with("--") {
        parts.pop();
        "-".to_string()
    } else {
        parts.pop().unwrap_or_default()
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
    if parts.is_empty() {
        key
    } else {
        format!("{}-{}", parts.join("-"), key)
    }
}

/// Canonicalise a chord (space-separated keystrokes).
fn normalize_chord(chord: &str) -> String {
    chord
        .split_whitespace()
        .map(normalize_keystroke)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_context(context: &Option<String>) -> String {
    context.as_deref().unwrap_or("").trim().to_string()
}

/// One resolved binding after layering — what actually gets bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveBinding {
    /// The author-written keystrokes string (chord-joined with spaces),
    /// unnormalized — this is what gets handed to `gpui::KeyBinding`.
    pub keystrokes: String,
    pub action: String,
    pub context: Option<String>,
    pub source: KeybindSource,
}

/// Layer `layers` in the given order (later layers override earlier ones),
/// keyed by `(normalized context, normalized keystrokes)`. A `None` action in
/// a later layer removes a prior entry with the same key (`"key": null`
/// unbinds); it is a no-op if no prior entry existed. Order of the returned
/// vec follows first-appearance order across the layers.
pub fn merge_keymaps(layers: &[(KeybindSource, &KeymapFile)]) -> Vec<EffectiveBinding> {
    let mut order: Vec<(String, String)> = Vec::new();
    let mut map: HashMap<(String, String), Option<EffectiveBinding>> = HashMap::new();

    for (source, file) in layers {
        for block in &file.0 {
            let ctx_key = normalize_context(&block.context);
            for (keystrokes, action) in &block.bindings {
                let key = (ctx_key.clone(), normalize_chord(keystrokes));
                if !map.contains_key(&key) {
                    order.push(key.clone());
                }
                let resolved = action.as_ref().map(|a| EffectiveBinding {
                    keystrokes: keystrokes.clone(),
                    action: a.clone(),
                    context: block.context.clone(),
                    source: *source,
                });
                map.insert(key, resolved);
            }
        }
    }

    order
        .into_iter()
        .filter_map(|key| map.remove(&key).flatten())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub message: String,
    pub line: usize,
    pub severity: Severity,
}

/// Validate a parsed keymap file: an action name not in `known_actions` is an
/// [`Severity::Error`]; the same `(context, keystrokes)` bound twice *within
/// this file* is a [`Severity::Warning`] (the same chord in two *different*
/// contexts is not a conflict). `text` is the original source, used to
/// recover best-effort line numbers.
pub fn validate_keymap(
    file: &KeymapFile,
    known_actions: &BTreeSet<&str>,
    text: &str,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for block in &file.0 {
        let ctx_key = normalize_context(&block.context);
        for (keystrokes, action) in &block.bindings {
            if let Some(a) = action {
                if !known_actions.contains(a.as_str()) {
                    issues.push(ValidationIssue {
                        message: format!("unknown action `{a}`"),
                        line: line_for_needle(text, a),
                        severity: Severity::Error,
                    });
                }
            }
            let key = (ctx_key.clone(), normalize_chord(keystrokes));
            if !seen.insert(key) {
                issues.push(ValidationIssue {
                    message: format!("`{keystrokes}` is bound more than once in this context"),
                    line: line_for_needle(text, keystrokes),
                    severity: Severity::Warning,
                });
            }
        }
    }
    issues
}

/// The shipped default keymap for the current platform — macOS or Linux (no
/// Windows, per the repo's platform scope). `linux` currently mirrors
/// `macos` 1:1 with `cmd-` swapped for `ctrl-` (the app doesn't yet branch its
/// live key bindings by OS anywhere else either, see `crates/shell/src/menu.rs`).
pub fn default_asset() -> &'static str {
    if cfg!(target_os = "macos") {
        include_str!("../assets/keymaps/default-macos.json")
    } else {
        include_str!("../assets/keymaps/default-linux.json")
    }
}

/// Where the user's `keymap.json` lives — a sibling of
/// [`crate::user_settings_path`] (`~/.config/labonair/keymap.json`).
pub fn user_keymap_path() -> PathBuf {
    crate::user_settings_path().with_file_name("keymap.json")
}

/// Create the user `keymap.json` with an empty-array scaffold if it doesn't
/// exist yet (never overwrites an existing file), and return its path. Used
/// by the "Open Keymap (JSON)" command (T19-008), mirroring
/// [`crate::ensure_user_settings_file`].
pub fn ensure_user_keymap_file() -> Result<PathBuf, String> {
    let path = user_keymap_path();
    if !path.exists() {
        let scaffold = "// User keybindings — overrides the built-in default keymap.\n\
             // See docs/settings-guidelines.md. Example:\n\
             // [\n\
             //   { \"context\": \"Editor\", \"bindings\": { \"cmd-k cmd-s\": \"zed::OpenKeymap\" } }\n\
             // ]\n[]\n";
        std::fs::write(&path, scaffold).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{KeyBindingContextPredicate, KeyContext, Keystroke};

    #[test]
    fn chord_keystrokes_all_parse_individually() {
        for token in "cmd-k cmd-s".split_whitespace() {
            Keystroke::parse(token).unwrap_or_else(|e| panic!("bad keystroke {token:?}: {e:?}"));
        }
    }

    #[test]
    fn context_predicate_evaluates() {
        let pred = KeyBindingContextPredicate::parse("Editor && vim_mode == normal").unwrap();
        let mut matching = KeyContext::default();
        matching.add("Editor");
        matching.set("vim_mode", "normal");
        assert!(pred.eval_inner(&[matching.clone()], &[matching]));

        let mut non_matching = KeyContext::default();
        non_matching.add("Editor");
        non_matching.set("vim_mode", "insert");
        assert!(!pred.eval_inner(&[non_matching.clone()], &[non_matching]));
    }

    fn block(context: Option<&str>, bindings: &[(&str, Option<&str>)]) -> KeymapBlock {
        KeymapBlock {
            context: context.map(str::to_string),
            bindings: bindings
                .iter()
                .map(|(k, v)| (k.to_string(), v.map(str::to_string)))
                .collect(),
        }
    }

    #[test]
    fn merge_default_then_user_override_and_unbind() {
        let default = KeymapFile(vec![block(
            None,
            &[
                ("cmd-t", Some("tab::NewTerminal")),
                ("cmd-w", Some("tab::Close")),
            ],
        )]);
        let user = KeymapFile(vec![block(
            None,
            &[("cmd-t", Some("tab::NewEditor")), ("cmd-w", None)],
        )]);

        let effective = merge_keymaps(&[
            (KeybindSource::Default, &default),
            (KeybindSource::User, &user),
        ]);

        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].action, "tab::NewEditor");
        assert_eq!(effective[0].source, KeybindSource::User);
    }

    #[test]
    fn unknown_action_reports_correct_line() {
        let text = "[\n  { \"bindings\": { \"cmd-t\": \"bogus::Action\" } }\n]\n";
        let file = parse_keymap_jsonc(text).unwrap();
        let known: BTreeSet<&str> = BTreeSet::new();
        let issues = validate_keymap(&file, &known, text);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].line, 2);
    }

    #[test]
    fn same_chord_different_context_is_not_a_conflict() {
        let file = KeymapFile(vec![
            block(Some("Editor"), &[("cmd-f", Some("search::Toggle"))]),
            block(Some("Terminal"), &[("cmd-f", Some("terminal::Find"))]),
        ]);
        let known: BTreeSet<&str> = ["search::Toggle", "terminal::Find"].into_iter().collect();
        let issues = validate_keymap(&file, &known, "");
        assert!(issues.is_empty());
    }

    #[test]
    fn same_chord_same_context_is_a_conflict() {
        let file = KeymapFile(vec![block(
            Some("Editor"),
            &[
                ("cmd-f", Some("search::Toggle")),
                ("shift-cmd-f", Some("search::ToggleGlobal")),
            ],
        )]);
        // Force a literal duplicate by reusing the same context twice.
        let file = KeymapFile(vec![
            file.0[0].clone(),
            block(Some("Editor"), &[("cmd-f", Some("search::Other"))]),
        ]);
        let known: BTreeSet<&str> = ["search::Toggle", "search::ToggleGlobal", "search::Other"]
            .into_iter()
            .collect();
        let issues = validate_keymap(&file, &known, "");
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.severity == Severity::Warning)
                .count(),
            1
        );
    }

    #[test]
    fn live_reload_proxy_effective_bindings_change_across_snapshots() {
        let known: BTreeSet<&str> = ["tab::NewTerminal", "tab::NewEditor"].into_iter().collect();
        let text_v1 = r#"[{ "bindings": { "cmd-t": "tab::NewTerminal" } }]"#;
        let text_v2 = r#"[{ "bindings": { "cmd-t": "tab::NewEditor" } }]"#;

        let file_v1 = parse_keymap_jsonc(text_v1).unwrap();
        assert!(validate_keymap(&file_v1, &known, text_v1).is_empty());
        let eff_v1 = merge_keymaps(&[(KeybindSource::User, &file_v1)]);

        let file_v2 = parse_keymap_jsonc(text_v2).unwrap();
        assert!(validate_keymap(&file_v2, &known, text_v2).is_empty());
        let eff_v2 = merge_keymaps(&[(KeybindSource::User, &file_v2)]);

        assert_ne!(eff_v1, eff_v2);
        assert_eq!(eff_v1[0].action, "tab::NewTerminal");
        assert_eq!(eff_v2[0].action, "tab::NewEditor");
    }

    #[test]
    fn parse_error_reports_line() {
        let text = "[\n  { \"bindings\": { \"cmd-t\": } }\n]\n";
        let err = parse_keymap_jsonc(text).unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn missing_or_empty_document_is_an_empty_keymap() {
        assert_eq!(parse_keymap_jsonc("").unwrap(), KeymapFile::default());
        assert_eq!(parse_keymap_jsonc("[]").unwrap(), KeymapFile::default());
    }

    #[test]
    fn default_asset_parses_and_validates_against_itself() {
        let text = default_asset();
        let file = parse_keymap_jsonc(text).expect("shipped default keymap must parse");
        let known: BTreeSet<&str> = file
            .0
            .iter()
            .flat_map(|b| b.bindings.iter())
            .filter_map(|(_, a)| a.as_deref())
            .collect();
        let issues = validate_keymap(&file, &known, text);
        assert!(
            issues.iter().all(|i| i.severity != Severity::Error),
            "default keymap has validation errors: {issues:?}"
        );
    }
}
