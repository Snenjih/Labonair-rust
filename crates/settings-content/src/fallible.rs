//! Fault-tolerant JSON → [`SettingsContent`] parsing (T19-001 Anweisung #5,
//! the `FallibleOption` equivalent: `zed-refrence/zed/crates/settings_content/
//! src/fallible_options.rs`).
//!
//! Granularity note: Zed's `FallibleOption` recovers per **leaf field**. This
//! port recovers per **top-level area** (one `SettingsContent` field, e.g.
//! `"terminal"`): if a value under an area key fails to deserialize, that
//! whole area falls back to its default and one [`FieldError`] is reported;
//! every other area parses independently and is unaffected. This keeps the
//! implementation to one macro instead of a per-field derive, while still
//! satisfying the acceptance criterion ("a broken field defaults, the rest of
//! the tree stays intact") — true leaf-level recovery can be layered on top
//! later without changing this function's signature.

use crate::SettingsContent;

/// One area that failed to deserialize as-is and fell back to its default.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// The `SettingsContent` area key (matches `AreaMeta::target_module`).
    pub area: &'static str,
    pub message: String,
}

/// Parse `json` (plain JSON or JSONC with `//` comments, as used by
/// `assets/settings/default.json`) into a [`SettingsContent`], defaulting any
/// area that fails to parse and reporting it in the returned error list.
pub fn parse(json: &str) -> (SettingsContent, Vec<FieldError>) {
    let value = match jsonc_parser::parse_to_serde_value(json, &Default::default()) {
        Ok(Some(v)) => v,
        Ok(None) => serde_json::Value::Object(Default::default()),
        Err(_) => serde_json::Value::Object(Default::default()),
    };
    let obj = value.as_object().cloned().unwrap_or_default();
    let mut errors = Vec::new();

    macro_rules! area {
        ($key:literal) => {{
            match obj.get($key) {
                Some(v) => match serde_json::from_value(v.clone()) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        errors.push(FieldError {
                            area: $key,
                            message: e.to_string(),
                        });
                        Default::default()
                    }
                },
                None => Default::default(),
            }
        }};
    }

    let content = SettingsContent {
        general: area!("general"),
        appearance: area!("appearance"),
        terminal: area!("terminal"),
        editor: area!("editor"),
        file_manager: area!("fileManager"),
        connections: area!("connections"),
        hosts: area!("hosts"),
        workspace: area!("workspace"),
        ai: area!("ai"),
        mcp: area!("mcp"),
        personalization: area!("personalization"),
        keymap: area!("keymap"),
    };

    (content, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_field_defaults_but_other_areas_survive() {
        let json = r#"{
            "terminal": { "terminalFontSize": "not-a-number" },
            "general": { "startupTerminalCount": 2 }
        }"#;
        let (content, errors) = parse(json);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].area, "terminal");
        assert_eq!(content.terminal, Default::default());
        assert_eq!(content.general.startup_terminal_count, Some(2));
    }

    #[test]
    fn empty_input_yields_all_defaults_no_errors() {
        let (content, errors) = parse("{}");
        assert!(errors.is_empty());
        assert_eq!(content, SettingsContent::default());
    }

    #[test]
    fn accepts_jsonc_comments() {
        let json = r#"{
            // this is a user comment
            "general": { "autostart": true }
        }"#;
        let (content, errors) = parse(json);
        assert!(errors.is_empty());
        assert_eq!(content.general.autostart, Some(true));
    }
}
