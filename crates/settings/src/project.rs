//! Project-layer settings (`.labonair/settings.json`, T19-003).
//!
//! `.labonair/settings.json` under the active project root
//! (`SettingsStore::set_active_project_root` / the crate-root
//! `set_active_project_root` wrapper) is merged as `SettingsLayer::Project`,
//! **above** `User`. Unlike the user file, a project file travels with a
//! cloned repo and is not trusted (Warnung in the task file) — every key
//! read from it goes through [`PROJECT_SETTINGS_WHITELIST`] first via
//! [`filter_and_parse`], which this module owns end to end so
//! `crate::store::SettingsStore` only ever has to call one function.

use std::path::{Path, PathBuf};

use labonair_settings_content::SettingsContent;

/// Relative path (from a project root) to its optional settings file.
pub const PROJECT_SETTINGS_RELATIVE_PATH: &str = ".labonair/settings.json";

/// The commented scaffold [`ensure_project_settings_file`] writes for a
/// project that doesn't have one yet. Every key it mentions is on
/// [`PROJECT_SETTINGS_WHITELIST`].
const INITIAL_PROJECT_SETTINGS: &str =
    include_str!("../assets/settings/initial_project_settings.json");

/// Key whitelist for the `SettingsLayer::Project` layer (Anweisung #4 /
/// Warnung: project settings files are **not trusted** — never let one
/// reach anything security-, network-, or credential-relevant).
///
/// `(area JSON key, allowed leaf JSON keys within that area)`. An area not
/// listed here is dropped **entirely** from a project file. Notably absent:
/// `hosts` and `ai` (no "reference an already-saved host" / "reference an
/// AI-directives file" *scalar* field exists yet in `SettingsContent` — the
/// Kontext's "Default-SSH-Host" / "AI-Directives-Datei-Verweis" use cases
/// need such a field added first, which is a future, narrower whitelist
/// addition, not a reason to open either area's existing fields, which are
/// all either credentials-adjacent or network endpoints); `mcp`
/// (bridge port/enable — explicitly forbidden by the task); `connections`
/// (SSH/explorer network timing knobs); `keymap` (explicitly forbidden);
/// `appearance`, `file_manager`, `personalization` (cosmetic, but not asked
/// for — narrower is cheaper to widen later than a leak is to undo, per the
/// task's own `## Notizen`).
pub const PROJECT_SETTINGS_WHITELIST: &[(&str, &[&str])] = &[
    (
        "general",
        &[
            "defaultStartupTab",
            "startupTerminalCount",
            "sessionRestore",
            "restoreWindowState",
        ],
    ),
    (
        "workspace",
        &[
            "dockLayout",
            "sidebarPosition",
            "sidebarOpen",
            "sidebarActivePanel",
            "sidebarRightOpen",
            "sidebarRightActivePanel",
            "sidebarWidth",
            "sidebarRightWidth",
            "commandPaletteSearchMode",
            "commandPaletteShowRecent",
            "commandPalettePosition",
            "commandPaletteAnimation",
            "commandPaletteHistorySize",
            "commandPaletteCloseOnOverlayClick",
        ],
    ),
    (
        "editor",
        &[
            "editorTabSize",
            "editorWordWrap",
            "editorIndentWithTabs",
            "editorFormatOnSave",
            "editorTrimTrailingWhitespace",
            "editorInsertFinalNewline",
            "editorIndentationGuides",
            "editorLineNumbers",
            "editorRelativeLineNumbers",
            "editorAutoSave",
            "editorAutoSaveDelay",
            "editorMaxFileSizeMb",
        ],
    ),
];

fn allowed_leaves(area: &str) -> Option<&'static [&'static str]> {
    PROJECT_SETTINGS_WHITELIST
        .iter()
        .find(|(key, _)| *key == area)
        .map(|(_, leaves)| *leaves)
}

/// Drop every top-level area not on [`PROJECT_SETTINGS_WHITELIST`], and
/// every leaf key within a whitelisted area that isn't itself whitelisted,
/// from `value` (expected to be a JSON object — anything else yields an
/// empty object). Returns the filtered value plus the dotted `"area"` /
/// `"area.leaf"` paths that got dropped, in encounter order.
fn filter_json(value: serde_json::Value) -> (serde_json::Value, Vec<String>) {
    use serde_json::{Map, Value};

    let mut rejected = Vec::new();
    let Value::Object(obj) = value else {
        return (Value::Object(Map::new()), rejected);
    };

    let mut out = Map::new();
    for (area, area_value) in obj {
        let Some(leaves) = allowed_leaves(&area) else {
            rejected.push(area);
            continue;
        };
        let Value::Object(area_obj) = area_value else {
            // Not an object at all — nothing under it can be whitelisted.
            rejected.push(area);
            continue;
        };
        let mut kept = Map::new();
        for (leaf, leaf_value) in area_obj {
            if leaves.contains(&leaf.as_str()) {
                kept.insert(leaf, leaf_value);
            } else {
                rejected.push(format!("{area}.{leaf}"));
            }
        }
        out.insert(area, Value::Object(kept));
    }
    (Value::Object(out), rejected)
}

/// Parse a project settings file's raw JSON/JSONC text into a
/// [`SettingsContent`], first dropping everything not on
/// [`PROJECT_SETTINGS_WHITELIST`]. Reuses
/// `labonair_settings_content::parse`'s own per-area fault tolerance for
/// whatever's left, so a broken *allowed* leaf still only defaults its own
/// area rather than the whole file. Never fails — invalid JSON/JSONC yields
/// an all-default `SettingsContent` with no rejections reported (there is
/// nothing to whitelist-reject if nothing parsed).
///
/// Returns the content plus every dropped key: whitelist rejections
/// (`"mcp"`, `"general.credentialEncryption"`, …) and per-area parse
/// failures on what *was* allowed through (`"editor (parse error)"`).
pub fn filter_and_parse(raw: &str) -> (SettingsContent, Vec<String>) {
    let value = match jsonc_parser::parse_to_serde_value(raw, &Default::default()) {
        Ok(Some(v)) => v,
        _ => serde_json::Value::Object(Default::default()),
    };
    let (filtered, mut rejected) = filter_json(value);
    let filtered_json = filtered.to_string();
    let (content, parse_errors) = labonair_settings_content::parse(&filtered_json);
    for e in parse_errors {
        rejected.push(format!("{} (parse error)", e.area));
    }
    (content, rejected)
}

/// Create `<root>/.labonair/settings.json` with a commented scaffold if it
/// doesn't already exist (never overwrites an existing file), and return its
/// path. Does not touch git — the Warnung is explicit that the command must
/// not `git add` anything.
pub fn ensure_project_settings_file(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join(".labonair");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("settings.json");
    if !path.exists() {
        std::fs::write(&path, INITIAL_PROJECT_SETTINGS).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_and_parse_drops_a_forbidden_area_entirely() {
        let (content, rejected) = filter_and_parse(r#"{"mcp":{"bridgePort":9999}}"#);
        assert_eq!(content.mcp, Default::default());
        assert_eq!(rejected, vec!["mcp".to_string()]);
    }

    #[test]
    fn filter_and_parse_drops_a_forbidden_leaf_but_keeps_allowed_siblings() {
        let (content, rejected) = filter_and_parse(
            r#"{"general":{"defaultStartupTab":"terminal","credentialEncryption":true}}"#,
        );
        assert_eq!(
            content.general.default_startup_tab,
            Some(labonair_settings_content::general::StartupTab::Terminal)
        );
        assert_eq!(content.general.credential_encryption, None);
        assert_eq!(rejected, vec!["general.credentialEncryption".to_string()]);
    }

    #[test]
    fn filter_and_parse_keeps_every_whitelisted_leaf() {
        let (content, rejected) =
            filter_and_parse(r#"{"editor":{"editorTabSize":4,"editorFormatOnSave":true}}"#);
        assert!(rejected.is_empty());
        assert_eq!(content.editor.editor_tab_size, Some(4));
        assert_eq!(content.editor.editor_format_on_save, Some(true));
    }

    #[test]
    fn filter_and_parse_never_fails_on_garbage() {
        let (content, rejected) = filter_and_parse("not json at all {{{");
        assert_eq!(content, SettingsContent::default());
        assert!(rejected.is_empty());
    }

    #[test]
    fn ensure_project_settings_file_creates_once_and_never_overwrites() {
        let root = std::env::temp_dir().join(format!("labonair-project-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();

        let path = ensure_project_settings_file(&root).unwrap();
        assert!(path.ends_with(".labonair/settings.json"));
        let scaffold = std::fs::read_to_string(&path).unwrap();
        assert!(scaffold.contains("Labonair project settings"));

        std::fs::write(&path, r#"{"general":{"defaultStartupTab":"terminal"}}"#).unwrap();
        let path2 = ensure_project_settings_file(&root).unwrap();
        assert_eq!(path, path2);
        let unchanged = std::fs::read_to_string(&path).unwrap();
        assert_eq!(unchanged, r#"{"general":{"defaultStartupTab":"terminal"}}"#);
    }
}
