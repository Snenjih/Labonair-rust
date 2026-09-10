//! `keymap.json` — parsing, layered merging and validation (T19-008).
//!
//! Pure data/logic, decoupled from the palette view's `CommandId`
//! on purpose (see `docs/architecture.md`'s crate-graph rule: this crate must
//! not depend on the palette UI crate). The module-owned [`crate::adapter`]
//! supplies the command vocabulary and owner defaults; platform adapters turn
//! the resulting [`EffectiveBinding`]s into real `gpui::KeyBinding`s.
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
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

thread_local! {
    /// Last valid user document, retained when a later edit is malformed or
    /// structurally invalid. The keymap module owns this recovery state so
    /// adapters cannot accidentally restore platform defaults on a bad edit.
    static LAST_GOOD_USER: RefCell<KeymapFile> = RefCell::new(KeymapFile::default());
    static LAST_ISSUES: RefCell<Vec<ValidationIssue>> = const { RefCell::new(Vec::new()) };
}

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
/// in the keymap management surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeybindSource {
    Default,
    /// Reserved for a future VS Code / JetBrains preset layer
    /// (`BaseKeymap`, not implemented by this task — see the task's
    /// `## Notizen`).
    BaseKeymap,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapParseError {
    pub message: String,
    /// 1-based line number, best-effort.
    pub line: usize,
}

/// Lossless user document used by the keymap management/editor surface.
///
/// `source` is authoritative for editing and persistence. Parsing and
/// validation are derived views; neither may replace the source text, so
/// comments, unknown actions, and malformed content remain recoverable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapDocument {
    pub source: String,
    pub parsed: Result<KeymapFile, KeymapParseError>,
    pub issues: Vec<ValidationIssue>,
}

impl KeymapDocument {
    /// Build a document from source text without discarding it on failure.
    pub fn from_source(source: impl Into<String>, known_actions: &BTreeSet<&str>) -> Self {
        let source = source.into();
        let parsed = parse_keymap_jsonc(&source);
        let issues = match &parsed {
            Ok(file) => validate_keymap(file, known_actions, &source),
            Err(error) => vec![ValidationIssue {
                message: error.message.clone(),
                line: error.line,
                severity: Severity::Error,
            }],
        };
        Self {
            source,
            parsed,
            issues,
        }
    }

    /// Whether this document can safely become the active user layer.
    pub fn is_valid(&self) -> bool {
        self.parsed.is_ok()
            && self
                .issues
                .iter()
                .all(|issue| issue.severity != Severity::Error)
    }
}

/// Persist the user's raw keymap source without formatting or normalizing it.
/// The caller may intentionally save an invalid document while editing; the
/// runtime will keep using its last valid layer until the next valid reload.
pub fn save_user_keymap_document(source: &str) -> Result<PathBuf, String> {
    let path = ensure_user_keymap_file()?;
    std::fs::write(&path, source).map_err(|error| error.to_string())?;
    Ok(path)
}

/// Append one user override block while preserving the existing JSONC source.
/// Existing bindings for the same command/context are explicitly unbound
/// before the replacement is added, so a default or earlier user binding does
/// not remain active. The operation refuses malformed top-level documents;
/// those must be repaired in the raw editor first.
pub fn append_user_binding_override(
    source: &str,
    context: Option<&str>,
    old_bindings: &[String],
    action: &str,
    replacement: Option<&str>,
) -> Result<String, String> {
    let parsed = parse_keymap_jsonc(source)
        .map_err(|error| format!("cannot add a binding while keymap.json is invalid: {error}"))?;
    let Some(close) = source.rfind(']') else {
        return Err("keymap.json must be a top-level array".to_string());
    };
    if !source[close + 1..].trim().is_empty() {
        return Err("keymap.json has content after its top-level array".to_string());
    }

    let mut bindings = Vec::<(String, Option<String>)>::new();
    for old in old_bindings {
        if replacement.is_some_and(|new| normalize_chord(old) == normalize_chord(new)) {
            continue;
        }
        bindings.push((old.clone(), None));
    }
    if let Some(new) = replacement {
        bindings.push((new.to_string(), Some(action.to_string())));
    }
    if bindings.is_empty() {
        return Ok(source.to_string());
    }

    let context_json = context.map_or_else(
        || "".to_string(),
        |value| {
            format!(
                "\"context\": {}, ",
                serde_json::to_string(value).unwrap_or_default()
            )
        },
    );
    let binding_json = bindings
        .iter()
        .map(|(keystrokes, value)| {
            let key = serde_json::to_string(keystrokes).unwrap_or_default();
            let value = value
                .as_deref()
                .map(|value| serde_json::to_string(value).unwrap_or_default())
                .unwrap_or_else(|| "null".to_string());
            format!("{key}: {value}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let block = format!("{{ {context_json}\"bindings\": {{ {binding_json} }} }}");
    let separator = if parsed.0.is_empty() { "" } else { "," };
    let mut updated = String::with_capacity(source.len() + block.len() + 8);
    updated.push_str(&source[..close]);
    updated.push_str(separator);
    updated.push('\n');
    updated.push_str("  ");
    updated.push_str(&block);
    updated.push('\n');
    updated.push_str(&source[close..]);
    Ok(updated)
}

/// Remove every user override that binds — or explicitly unbinds — `action`
/// in `context`, leaving all other source text (comments, unrelated bindings)
/// untouched. This is the keymap surface's "Reset to default".
///
/// Matching is by resolved command identity, so a user file that used a
/// migration alias is still reset by its canonical command. A `"<chord>": null`
/// entry is removed when its chord matches one of `default_keystrokes`, which
/// is how an explicit user unbind of a shipped default is undone. A block that
/// would be left with no bindings is removed whole.
///
/// Returns the source unchanged when there is nothing to remove. Returns `Err`
/// — without proposing any write — when the document is malformed or the edit
/// would not round-trip; the caller then routes the user to the raw editor.
pub fn remove_user_binding_override(
    source: &str,
    context: Option<&str>,
    action: &str,
    default_keystrokes: &[String],
) -> Result<String, String> {
    parse_keymap_jsonc(source)
        .map_err(|error| format!("cannot reset a binding while keymap.json is invalid: {error}"))?;

    let ast = jsonc_parser::parse_to_ast(
        source,
        &jsonc_parser::CollectOptions::default(),
        &jsonc_parser::ParseOptions::default(),
    )
    .map_err(|error| error.to_string())?;
    let Some(jsonc_parser::ast::Value::Array(array)) = ast.value else {
        return Ok(source.to_string());
    };

    let target = crate::runtime::command_for_action(action);
    let wanted_context = context.map(str::to_string);
    let wanted_context = normalize_context(&wanted_context);
    let default_norm: Vec<String> = default_keystrokes
        .iter()
        .map(|chord| normalize_chord(chord))
        .collect();

    let mut removals: Vec<(usize, usize)> = Vec::new();
    for element in &array.elements {
        let jsonc_parser::ast::Value::Object(object) = element else {
            continue;
        };
        let element_context = object
            .properties
            .iter()
            .find(|prop| prop.name.as_str() == "context")
            .and_then(|prop| match &prop.value {
                jsonc_parser::ast::Value::StringLit(lit) => Some(lit.value.to_string()),
                _ => None,
            });
        if normalize_context(&element_context) != wanted_context {
            continue;
        }
        let Some(bindings_prop) = object
            .properties
            .iter()
            .find(|prop| prop.name.as_str() == "bindings")
        else {
            continue;
        };
        let jsonc_parser::ast::Value::Object(bindings) = &bindings_prop.value else {
            continue;
        };

        let matched: Vec<&jsonc_parser::ast::ObjectProp> = bindings
            .properties
            .iter()
            .filter(|prop| {
                let chord = prop.name.as_str();
                match &prop.value {
                    jsonc_parser::ast::Value::StringLit(lit) => match target {
                        Some(id) => {
                            crate::runtime::command_for_action(lit.value.as_ref()) == Some(id)
                        }
                        None => lit.value.as_ref() == action,
                    },
                    jsonc_parser::ast::Value::NullKeyword(_) => {
                        default_norm.contains(&normalize_chord(chord))
                    }
                    _ => false,
                }
            })
            .collect();

        if matched.is_empty() {
            continue;
        }
        if matched.len() == bindings.properties.len() {
            removals.push((object.range.start, object.range.end));
        } else {
            for prop in matched {
                removals.push((prop.range.start, prop.range.end));
            }
        }
    }

    if removals.is_empty() {
        return Ok(source.to_string());
    }

    let updated = collapse_blank_lines(&splice_out(source, &mut removals));
    parse_keymap_jsonc(&updated).map_err(|error| {
        format!("automatic reset produced invalid keymap.json ({error}); edit the file directly")
    })?;
    Ok(updated)
}

/// Remove `ranges` (byte offsets into `source`), each expanded to also swallow
/// one adjacent list separator so the surrounding array/object stays valid.
/// `ranges` is sorted and overlap-merged in place.
fn splice_out(source: &str, ranges: &mut [(usize, usize)]) -> String {
    ranges.sort_by_key(|(start, _)| *start);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges.iter().copied() {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut pos = 0usize;
    for (start, end) in merged {
        let mut lo = start;
        let mut hi = end;
        let mut scan = hi;
        while scan < bytes.len() && bytes[scan].is_ascii_whitespace() {
            scan += 1;
        }
        if scan < bytes.len() && bytes[scan] == b',' {
            hi = scan + 1;
        } else {
            let mut back = lo;
            while back > 0 && bytes[back - 1].is_ascii_whitespace() {
                back -= 1;
            }
            if back > 0 && bytes[back - 1] == b',' {
                lo = back - 1;
            }
        }
        if lo > pos {
            out.push_str(&source[pos..lo]);
        }
        pos = hi.max(pos);
    }
    out.push_str(&source[pos..]);
    out
}

/// Collapse runs of blank lines left behind by [`splice_out`] to a single one;
/// every non-blank line is preserved byte-for-byte.
fn collapse_blank_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut prev_blank = false;
    for line in source.split_inclusive('\n') {
        let blank = line.strip_suffix('\n').unwrap_or(line).trim().is_empty();
        if blank && prev_blank {
            continue;
        }
        prev_blank = blank;
        out.push_str(line);
    }
    out
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Where the user's `keymap.json` lives (`~/.config/labonair/keymap.json`).
pub fn user_keymap_path() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    let base = dirs::home_dir()
        .expect("cannot resolve home dir")
        .join(".config");
    #[cfg(target_os = "windows")]
    let base = dirs::config_dir().expect("cannot resolve config dir");
    base.join("labonair").join("keymap.json")
}

/// Create the user `keymap.json` with an empty-array scaffold if it doesn't
/// exist yet (never overwrites an existing file), and return its path. Used
/// by the "Open Keymap (JSON)" command (T19-008), mirroring
/// [`crate::ensure_user_settings_file`].
pub fn ensure_user_keymap_file() -> Result<PathBuf, String> {
    let path = user_keymap_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
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

/// Read the lossless user document used by management surfaces. A missing
/// file is materialized through [`ensure_user_keymap_file`], while an existing
/// file is returned byte-for-byte so comments and invalid edits remain
/// recoverable by the editor.
pub fn read_user_keymap_document(known_actions: &BTreeSet<&str>) -> Result<KeymapDocument, String> {
    let path = ensure_user_keymap_file()?;
    let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(KeymapDocument::from_source(source, known_actions))
}

/// Validation issues from the most recent user-file load.
pub fn last_issues() -> Vec<ValidationIssue> {
    LAST_ISSUES.with(|issues| issues.borrow().clone())
}

/// Load, validate, and merge the built-in and user keymap layers. A malformed
/// user file keeps the previous valid user document while exposing its
/// diagnostics to the presentation adapter. The caller supplies the current
/// command vocabulary; the keymap module does not depend on feature registries.
pub fn effective_bindings(known_actions: &BTreeSet<&'static str>) -> Vec<EffectiveBinding> {
    effective_bindings_with_defaults(known_actions, &[])
}

/// Load the effective file bindings while incorporating defaults contributed
/// by owner command providers. The built-in JSON remains a migration-stable
/// compatibility layer; provider defaults are applied after it and before the
/// user layer.
pub fn effective_bindings_with_defaults(
    known_actions: &BTreeSet<&'static str>,
    owner_defaults: &[crate::runtime::KeymapBinding],
) -> Vec<EffectiveBinding> {
    let default = match parse_keymap_jsonc(default_asset()) {
        Ok(file) => file,
        Err(error) => {
            tracing::error!(error = %error, "shipped default keymap failed to parse");
            KeymapFile::default()
        }
    };
    let owner_default_file = owner_defaults_file(owner_defaults);
    let user = load_user_keymap(known_actions);
    merge_keymaps(&[
        (KeybindSource::Default, &default),
        (KeybindSource::Default, &owner_default_file),
        (KeybindSource::User, &user),
    ])
}

fn owner_defaults_file(bindings: &[crate::runtime::KeymapBinding]) -> KeymapFile {
    KeymapFile(
        bindings
            .iter()
            .map(|binding| KeymapBlock {
                context: binding
                    .context
                    .map(crate::runtime::context_name)
                    .map(str::to_string),
                bindings: vec![(
                    binding.keystrokes.clone(),
                    Some(binding.command.action_name().to_string()),
                )],
            })
            .collect(),
    )
}

fn load_user_keymap(known_actions: &BTreeSet<&'static str>) -> KeymapFile {
    let path = user_keymap_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        LAST_ISSUES.with(|issues| issues.borrow_mut().clear());
        LAST_GOOD_USER.with(|file| *file.borrow_mut() = KeymapFile::default());
        return KeymapFile::default();
    };

    let document = KeymapDocument::from_source(text, known_actions);
    match document.parsed {
        Ok(file) => {
            let has_errors = document
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Error);
            if has_errors {
                tracing::warn!(
                    path = %path.display(),
                    "keymap.json has validation errors — keeping the last good keymap"
                );
                LAST_ISSUES.with(|current| *current.borrow_mut() = document.issues);
                LAST_GOOD_USER.with(|previous| previous.borrow().clone())
            } else {
                LAST_ISSUES.with(|current| *current.borrow_mut() = document.issues);
                LAST_GOOD_USER.with(|previous| *previous.borrow_mut() = file.clone());
                file
            }
        }
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                error = %error,
                "keymap.json is not valid JSON/JSONC — keeping the last good keymap"
            );
            LAST_ISSUES.with(|current| *current.borrow_mut() = document.issues);
            LAST_GOOD_USER.with(|previous| previous.borrow().clone())
        }
    }
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
    fn document_preserves_comments_and_unknown_actions() {
        let source = "// Keep this comment.\n[{\"bindings\": {\"cmd-x\": \"future::Action\"}}]\n";
        let known: BTreeSet<&str> = BTreeSet::new();
        let document = KeymapDocument::from_source(source, &known);

        assert_eq!(document.source, source);
        assert!(document.parsed.is_ok());
        assert!(!document.is_valid());
        assert_eq!(document.issues[0].severity, Severity::Error);
        assert_eq!(document.issues[0].line, 2);
    }

    #[test]
    fn document_preserves_malformed_source_for_editor_recovery() {
        let source = "[{\"bindings\": {\"cmd-x\": }}]";
        let known: BTreeSet<&str> = BTreeSet::new();
        let document = KeymapDocument::from_source(source, &known);

        assert_eq!(document.source, source);
        assert!(document.parsed.is_err());
        assert!(!document.is_valid());
        assert_eq!(document.issues[0].severity, Severity::Error);
    }

    #[test]
    fn owner_defaults_materialize_as_a_default_file_layer() {
        let file = owner_defaults_file(&[crate::runtime::KeymapBinding::in_context(
            "cmd-f",
            labonair_command_palette_core::CommandId::Find,
            labonair_command_palette_core::CommandContext::Editor,
        )]);

        assert_eq!(file.0[0].context.as_deref(), Some("Editor"));
        assert_eq!(file.0[0].bindings[0].1.as_deref(), Some("search::Toggle"));
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

    #[test]
    fn append_user_binding_override_preserves_source_and_writes_unbind_then_replacement() {
        let source = "// Keep this comment.\n[{\"bindings\": {\"cmd-f\": \"search::Toggle\"}}]\n";
        let updated = append_user_binding_override(
            source,
            Some("Editor"),
            &["cmd-f".to_string()],
            "search::ToggleGlobal",
            Some("cmd-g"),
        )
        .expect("valid source should accept a binding override");

        assert!(updated.starts_with("// Keep this comment."));
        assert!(updated.contains("\"cmd-f\": null"));
        assert!(updated.contains("\"cmd-g\": \"search::ToggleGlobal\""));
        assert!(parse_keymap_jsonc(&updated).is_ok());
    }

    #[test]
    fn append_user_binding_override_rejects_malformed_source_and_supports_unbind() {
        let error = append_user_binding_override(
            "[{\"bindings\": {\"cmd-f\": }}]",
            None,
            &["cmd-f".to_string()],
            "search::Toggle",
            None,
        )
        .expect_err("malformed source must be repaired in the raw editor first");
        assert!(error.contains("invalid"));

        let updated = append_user_binding_override(
            "[]",
            None,
            &["cmd-f".to_string()],
            "search::Toggle",
            None,
        )
        .expect("an empty valid document can receive an unbind override");
        assert!(updated.contains("\"cmd-f\": null"));
        assert!(parse_keymap_jsonc(&updated).is_ok());
    }

    #[test]
    fn remove_user_binding_override_deletes_a_whole_generated_block() {
        let source = "// header\n[\n  { \"context\": \"Editor\", \"bindings\": { \"cmd-f\": \"search::Toggle\" } }\n]\n";
        let updated =
            remove_user_binding_override(source, Some("Editor"), "search::Toggle", &[]).unwrap();
        assert!(updated.starts_with("// header"));
        assert!(!updated.contains("search::Toggle"));
        assert!(parse_keymap_jsonc(&updated).unwrap().0.is_empty());
    }

    #[test]
    fn remove_user_binding_override_keeps_sibling_bindings_and_comments() {
        let source = "[\n  { \"bindings\": {\n    \"cmd-f\": \"search::Toggle\", // rebound\n    \"cmd-g\": \"editor::Foo\"\n  } }\n]\n";
        let updated = remove_user_binding_override(source, None, "search::Toggle", &[]).unwrap();
        assert!(!updated.contains("cmd-f"));
        assert!(updated.contains("\"cmd-g\": \"editor::Foo\""));
        assert!(parse_keymap_jsonc(&updated).is_ok());
    }

    #[test]
    fn remove_user_binding_override_undoes_an_explicit_default_unbind() {
        let source = "[{ \"bindings\": { \"cmd-f\": null } }]";
        let updated =
            remove_user_binding_override(source, None, "search::Toggle", &["cmd-f".to_string()])
                .unwrap();
        assert!(!updated.contains("cmd-f"));
        assert!(parse_keymap_jsonc(&updated).is_ok());
    }

    #[test]
    fn remove_user_binding_override_matches_migration_aliases() {
        let canonical = labonair_command_palette_core::CommandId::OpenKeymapJson.action_name();
        let source = "[{ \"bindings\": { \"cmd-k cmd-s\": \"zed::OpenKeymap\" } }]";
        let updated = remove_user_binding_override(source, None, canonical, &[]).unwrap();
        assert!(!updated.contains("zed::OpenKeymap"));
    }

    #[test]
    fn remove_user_binding_override_is_noop_and_rejects_malformed() {
        let source = "[{ \"bindings\": { \"cmd-t\": \"tab::NewTerminal\" } }]";
        assert_eq!(
            remove_user_binding_override(source, None, "search::Toggle", &[]).unwrap(),
            source
        );
        assert!(
            remove_user_binding_override("[{\"bindings\": }]", None, "search::Toggle", &[])
                .unwrap_err()
                .contains("invalid")
        );
    }
}
