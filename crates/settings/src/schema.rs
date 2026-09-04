//! JSON-Schema generation + `settings.json` validation (T19-006).
//!
//! [`json_schema`] derives a `serde_json::Value` JSON Schema straight from
//! [`SettingsContent`]'s `#[derive(schemars::JsonSchema)]` (already present
//! since T19-001 — every area struct, enum, and doc-comment description is
//! covered because schemars walks the real Rust types, so this can never
//! drift the way a hand-maintained schema file would). [`validate`] then
//! checks a parsed `settings.json` value against that schema with the
//! `jsonschema` crate, producing one [`SettingsValidationError`] per
//! type/enum mismatch (dotted `json_path`, matching
//! `labonair_settings_ui::schema::AnyField::json_path`'s convention, plus a
//! best-effort line/column found via `labonair_settings_json::
//! find_value_range` when the raw text is available) and a separate,
//! non-fatal list of unknown-key warnings (Anweisung #2's "unbekannte Keys
//! sind Warnungen, nicht fatal" — `jsonschema`'s own `additionalProperties`
//! keyword isn't used for this since schemars doesn't emit `false` for it on
//! plain structs, so unknown keys are walked separately in
//! [`unknown_key_warnings`]).

use serde_json::{Map, Value};

use labonair_settings_content::SettingsContent;

/// One validation finding: a dotted path into the settings tree (e.g.
/// `"terminal.terminalFontSize"`) plus a human-readable message and, when a
/// raw settings.json text was supplied, a best-effort 1-based line/column.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsValidationError {
    pub json_path: String,
    pub message: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

impl std::fmt::Display for SettingsValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.json_path, self.message)?;
        if let Some(line) = self.line {
            write!(f, " (line {line})")?;
        }
        Ok(())
    }
}

/// The generated JSON Schema for [`SettingsContent`], as a plain
/// `serde_json::Value` (schemars' own `RootSchema` serializes to exactly
/// this shape — draft-07, `definitions` for every nested struct/enum,
/// `$ref`/`allOf` for cross-references). Regenerated on every call; schema
/// construction is cheap (no I/O), and this task's write-once-at-startup
/// schema file (`Store::write_schema_file`) and the on-load validation below
/// are the only two callers.
pub fn json_schema() -> Value {
    let root = schemars::schema_for!(SettingsContent);
    serde_json::to_value(root).expect("SettingsContent's derived JsonSchema always serializes")
}

/// Validate `instance` (a parsed `settings.json`-shaped value, e.g. from
/// `jsonc_parser::parse_to_serde_value`) against [`json_schema`]. `raw_text`,
/// if given, is used to look up a best-effort line/column for each error
/// (Warnung: "ohne Position trotzdem den `json_path` liefern" — a lookup
/// miss just leaves `line`/`col` as `None`, it never drops the error).
///
/// Returns `(errors, warnings)`: `errors` are type/enum mismatches (the
/// field this points at should fall back to its default, mirroring
/// `labonair_settings_content::fallible::parse`'s per-area behaviour but at
/// leaf granularity); `warnings` are unknown object keys, which are always
/// non-fatal (forward compatibility — an older binary opening a newer
/// settings file must not lose unrelated settings).
pub fn validate(
    instance: &Value,
    raw_text: Option<&str>,
) -> (Vec<SettingsValidationError>, Vec<SettingsValidationError>) {
    let schema = json_schema();

    let mut errors = Vec::new();
    match jsonschema::validator_for(&schema) {
        Ok(validator) => {
            for err in validator.iter_errors(instance) {
                let json_path = pointer_to_json_path(err.instance_path().as_str());
                let message = err.to_string();
                let (line, col) = raw_text
                    .and_then(|text| position_for_path(text, &json_path))
                    .map_or((None, None), |(l, c)| (Some(l), Some(c)));
                errors.push(SettingsValidationError {
                    json_path,
                    message,
                    line,
                    col,
                });
            }
        }
        Err(e) => {
            // The schema itself failed to compile — this is a bug in this
            // crate (a schemars output `jsonschema` can't parse), not a user
            // settings-file problem. Surface it as a single error rather
            // than silently skipping validation.
            errors.push(SettingsValidationError {
                json_path: String::new(),
                message: format!("internal: settings schema failed to compile: {e}"),
                line: None,
                col: None,
            });
        }
    }

    let warnings = unknown_key_warnings(instance, &schema, raw_text);

    (errors, warnings)
}

/// The hover description for a dotted `json_path` (e.g.
/// `&["terminal", "terminalFontSize"]`) — the field's own doc-comment
/// description if it has one (schemars attaches it to the field's schema
/// node), else the nearest ancestor's, else `None`. Used by the settings
/// editor's schema-hover helper (T19-006 Anweisung #5) so hovering a key in
/// `labonair-settings.json` shows the same text a Settings-UI row for that
/// field would.
pub fn description_for_path(json_path: &[&str]) -> Option<String> {
    let schema = json_schema();
    let defs = schema
        .get("definitions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut node = schema.clone();
    let mut last_desc = node
        .get("description")
        .and_then(Value::as_str)
        .map(String::from);

    for key in json_path {
        let container = resolve(&node, &defs);
        let field_schema = container
            .get("properties")
            .and_then(Value::as_object)
            .and_then(|props| props.get(*key))?
            .clone();

        let mut desc = field_schema
            .get("description")
            .and_then(Value::as_str)
            .map(String::from);
        if desc.is_none() {
            let inner = resolve(&field_schema, &defs);
            desc = inner
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);
        }
        if desc.is_some() {
            last_desc = desc;
        }
        node = field_schema;
    }

    last_desc
}

/// `"/terminal/terminalFontSize"` (a JSON Pointer, `jsonschema`'s
/// `instance_path` format) -> `"terminal.terminalFontSize"` (this project's
/// dotted `json_path` convention, matching `AnyField::json_path`).
fn pointer_to_json_path(pointer: &str) -> String {
    pointer
        .split('/')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(".")
}

fn position_for_path(text: &str, json_path: &str) -> Option<(usize, usize)> {
    if json_path.is_empty() {
        return None;
    }
    let segments: Vec<&str> = json_path.split('.').collect();
    let range = labonair_settings_json::find_value_range(text, &segments)?;
    Some(labonair_settings_json::line_col_at(text, range.start))
}

/// Resolve a schema node that's a bare `$ref` or an `"allOf": [{ "$ref": .. }]`
/// wrapper (schemars 0.8's shape for every struct/enum field that carries a
/// `default`/`description` alongside its `$ref`) to the definition it points
/// at. Returns `node` itself unresolved if it isn't a ref-shaped node.
fn resolve<'a>(node: &'a Value, defs: &'a Map<String, Value>) -> &'a Value {
    if let Some(name) = node
        .get("$ref")
        .and_then(Value::as_str)
        .and_then(|r| r.strip_prefix("#/definitions/"))
    {
        if let Some(resolved) = defs.get(name) {
            return resolved;
        }
    }
    if let Some(first) = node
        .get("allOf")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
    {
        return resolve(first, defs);
    }
    node
}

/// Walk `instance` alongside `schema`, collecting one warning per object key
/// that isn't in the matching schema node's `properties` set. Nodes shaped
/// as a map (`additionalProperties` is a schema, `properties` is absent —
/// schemars' output for `HashMap<String, T>`/`BTreeMap<String, T>` fields)
/// are recursed into by *value*, never flagged on their keys — those keys
/// are user data (host ids, panel names, …), not settings fields.
fn unknown_key_warnings(
    instance: &Value,
    schema: &Value,
    raw_text: Option<&str>,
) -> Vec<SettingsValidationError> {
    let defs = schema
        .get("definitions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut path = Vec::new();
    walk_unknown_keys(instance, schema, &defs, &mut path, raw_text, &mut out);
    out
}

fn walk_unknown_keys(
    instance: &Value,
    schema_node: &Value,
    defs: &Map<String, Value>,
    path: &mut Vec<String>,
    raw_text: Option<&str>,
    out: &mut Vec<SettingsValidationError>,
) {
    let Value::Object(obj) = instance else {
        return;
    };
    let node = resolve(schema_node, defs);

    if let Some(props) = node.get("properties").and_then(Value::as_object) {
        for (key, value) in obj {
            path.push(key.clone());
            match props.get(key) {
                Some(sub_schema) => {
                    walk_unknown_keys(value, sub_schema, defs, path, raw_text, out);
                }
                None => {
                    let json_path = path.join(".");
                    let (line, col) = raw_text
                        .and_then(|text| position_for_path(text, &json_path))
                        .map_or((None, None), |(l, c)| (Some(l), Some(c)));
                    out.push(SettingsValidationError {
                        json_path,
                        message: format!("unknown setting key {key:?} (ignored)"),
                        line,
                        col,
                    });
                }
            }
            path.pop();
        }
    } else if let Some(value_schema @ Value::Object(_)) = node.get("additionalProperties") {
        for (key, value) in obj {
            path.push(key.clone());
            walk_unknown_keys(value, value_schema, defs, path, raw_text, out);
            path.pop();
        }
    }
    // Neither `properties` nor an object-schema `additionalProperties`:
    // a leaf type (or a map whose values aren't themselves object-shaped) —
    // nothing further to check.
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schema_covers_every_area_and_an_enum() {
        let schema = json_schema();
        let props = schema["properties"].as_object().unwrap();
        // Every `SettingsContent` field's serde (camelCase, or explicit
        // `#[serde(rename)]` for `file_manager`) key must have a schema
        // entry — checked against a real serialized instance rather than
        // `AreaMeta::target_module` (Rust field names), which uses
        // `file_manager` while the JSON/schema key is `fileManager`.
        let sample =
            serde_json::to_value(SettingsContent::default()).expect("SettingsContent serializes");
        for key in sample.as_object().unwrap().keys() {
            assert!(
                props.contains_key(key),
                "schema missing top-level area {key}"
            );
        }
        let defs = schema["definitions"].as_object().unwrap();
        let cursor_style = &defs["CursorStyle"];
        let variants: Vec<&str> = cursor_style["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(variants, vec!["block", "underline", "bar"]);
    }

    #[test]
    fn valid_default_json_has_no_errors_or_warnings() {
        let value: Value =
            serde_json::from_str(&serde_json::to_string(&SettingsContent::defaults()).unwrap())
                .unwrap();
        let (errors, warnings) = validate(&value, None);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn wrong_type_is_reported_with_json_path() {
        let text = "{\n  \"terminal\": {\n    \"terminalFontSize\": \"gross\"\n  }\n}";
        let instance: Value = serde_json::from_str(text).unwrap();
        let (errors, _warnings) = validate(&instance, Some(text));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].json_path, "terminal.terminalFontSize");
        assert_eq!(errors[0].line, Some(3));
    }

    #[test]
    fn invalid_enum_value_is_reported() {
        let text = r#"{"terminal": {"terminalCursorStyle": "blinky"}}"#;
        let instance: Value = serde_json::from_str(text).unwrap();
        let (errors, _warnings) = validate(&instance, Some(text));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].json_path, "terminal.terminalCursorStyle");
    }

    #[test]
    fn unknown_key_is_a_warning_not_an_error() {
        let instance = json!({"terminal": {"notARealField": 1}});
        let (errors, warnings) = validate(&instance, None);
        assert!(errors.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].json_path, "terminal.notARealField");
    }

    #[test]
    fn unknown_top_level_legacy_key_is_a_warning() {
        let instance = json!({"preferences": {}, "general": {}});
        let (errors, warnings) = validate(&instance, None);
        assert!(errors.is_empty());
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].json_path, "preferences");
    }

    #[test]
    fn description_for_path_finds_the_field_doc_comment() {
        let desc = description_for_path(&["terminal", "scrollbackMaxSizeMb"]).unwrap();
        assert!(desc.contains("scrollback"), "{desc:?}");
    }

    #[test]
    fn description_for_path_unknown_field_is_none() {
        assert!(description_for_path(&["terminal", "notAField"]).is_none());
    }

    #[test]
    fn map_shaped_field_does_not_warn_on_its_keys() {
        let instance = json!({
            "personalization": {
                "panelToggleVisibility": { "explorer": true, "sftp": false }
            }
        });
        let (errors, warnings) = validate(&instance, None);
        assert!(errors.is_empty());
        assert!(warnings.is_empty(), "{warnings:?}");
    }
}
