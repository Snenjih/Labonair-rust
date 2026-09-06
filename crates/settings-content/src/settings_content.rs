//! `labonair-settings-content` — the fully typed settings tree (T19-001).
//!
//! Blueprint: `zed-refrence/zed/crates/settings_content/src/settings_content.rs`
//! (+ `merge_from.rs`, `fallible_options.rs`). This crate has **no** GPUI, UI,
//! or `labonair-backend` dependency (`docs/architecture.md` §2 settings
//! track) — it is a pure data model that `T19-002`'s `SettingsStore` layers
//! (default < user < project) and that `labonair-backend` bridges back onto
//! the legacy flat `Preferences` struct (`impl From<&SettingsContent> for
//! Preferences`, in `labonair-backend::modules::settings::content_bridge`) so
//! existing call sites keep working unchanged until `T19-002` lands.

mod merge_from;

pub mod areas;
pub mod fallible;

pub mod appearance;
pub mod connections;
pub mod editor;
pub mod file_manager;
pub mod general;
// Migration-only wire types for reading legacy host settings. Hosts are not
// part of `SettingsContent` and are never serialized by this crate's tree.
pub mod hosts;
// Migration-only wire types; MCP runtime configuration belongs to the
// AI/MCP capability and is not part of SettingsContent.
pub mod mcp;
// Migration-only wire types; statusbar/panel layout belongs to Workspace.
pub mod personalization;
pub mod terminal;
pub mod workspace;

pub use areas::{AreaKind, AreaMeta, AREAS};
pub use fallible::{parse, FieldError};
pub use merge_from::MergeFrom;
// `#[derive(MergeFrom)]` — see `labonair-settings-macros`. Lives in a
// different namespace than the `MergeFrom` trait above, so both can share the
// name (mirrors `serde`'s `Serialize` trait + derive macro).
pub use labonair_settings_macros::MergeFrom;

use serde::{Deserialize, Serialize};

use appearance::AppearanceContent;
use connections::ConnectionsContent;
use editor::EditorContent;
use file_manager::FileManagerContent;
use general::GeneralContent;
use terminal::TerminalContent;
use workspace::WorkspaceContent;

/// The fully typed settings tree. Every leaf field across the area structs is
/// `Option<T>` (`None` = "not set at this layer", distinct from "set to the
/// zero value") so that [`MergeFrom`] can tell an unset field from an
/// explicit override when layering default < user < project settings files.
#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsContent {
    pub general: GeneralContent,
    pub appearance: AppearanceContent,
    pub terminal: TerminalContent,
    pub editor: EditorContent,
    #[serde(rename = "fileManager")]
    pub file_manager: FileManagerContent,
    pub connections: ConnectionsContent,
    pub workspace: WorkspaceContent,
}

impl SettingsContent {
    /// The fully populated default tree — every leaf is `Some(..)`. Must
    /// stay in sync with `assets/settings/default.json`
    /// (`tests::defaults_matches_shipped_default_json` enforces this).
    pub fn defaults() -> Self {
        Self {
            general: GeneralContent::defaults(),
            appearance: AppearanceContent::defaults(),
            terminal: TerminalContent::defaults(),
            editor: EditorContent::defaults(),
            file_manager: FileManagerContent::defaults(),
            connections: ConnectionsContent::defaults(),
            workspace: WorkspaceContent::defaults(),
        }
    }
}

/// The shipped, documented default tree (`assets/settings/default.json`,
/// JSONC — comments per key, Zed's `assets/settings/default.json` pattern).
pub const DEFAULT_JSON: &str = include_str!("../assets/settings/default.json");

#[cfg(test)]
mod tests {
    use super::*;

    /// `SettingsContent`'s actual top-level field names — the only valid
    /// `AreaMeta::target_module` values (`every_area_hits_a_real_module`).
    const FIELD_NAMES: &[&str] = &[
        "general",
        "appearance",
        "terminal",
        "editor",
        "file_manager",
        "connections",
        "workspace",
    ];

    #[test]
    fn defaults_matches_shipped_default_json() {
        let (from_json, errors) = fallible::parse(DEFAULT_JSON);
        assert!(
            errors.is_empty(),
            "default.json failed to parse cleanly: {errors:?}"
        );
        assert_eq!(
            SettingsContent::defaults(),
            from_json,
            "SettingsContent::defaults() and assets/settings/default.json drifted"
        );
    }

    #[test]
    fn every_area_hits_a_real_module() {
        for area in AREAS {
            assert!(
                FIELD_NAMES.contains(&area.target_module),
                "AREAS entry {:?} points at unknown target_module {:?}",
                area.key,
                area.target_module
            );
        }
    }

    #[test]
    fn custom_areas_match_settings_guidelines_rule_4() {
        let custom: Vec<&str> = AREAS
            .iter()
            .filter(|a| a.kind == AreaKind::Custom)
            .map(|a| a.key)
            .collect();
        assert!(
            custom.is_empty(),
            "Settings has no integration-owned custom areas"
        );
    }

    #[test]
    fn capability_management_areas_are_not_settings_categories() {
        for removed in ["themes", "hosts", "shortcuts", "keymap"] {
            assert!(
                !AREAS.iter().any(|area| area.key == removed),
                "{removed} must be owned by its capability, not Settings"
            );
        }
    }

    #[test]
    fn merge_from_layers_user_over_default_over_project() {
        let mut layered = SettingsContent::defaults();
        assert_eq!(layered.terminal.terminal_font_size, Some(15));

        let mut user = SettingsContent::default();
        user.terminal.terminal_font_size = Some(18);
        layered.merge_from(&user);
        assert_eq!(layered.terminal.terminal_font_size, Some(18));
        // Unrelated fields survive the merge untouched.
        assert_eq!(layered.terminal.terminal_scrollback, Some(5_000));

        let mut project = SettingsContent::default();
        project.terminal.terminal_font_size = Some(22);
        layered.merge_from(&project);
        assert_eq!(layered.terminal.terminal_font_size, Some(22));
    }

    #[test]
    fn merge_from_none_never_overwrites() {
        let mut layered = SettingsContent::defaults();
        let empty_layer = SettingsContent::default();
        layered.merge_from(&empty_layer);
        assert_eq!(layered, SettingsContent::defaults());
    }

    #[test]
    fn legacy_capability_sections_are_not_parsed_or_serialized() {
        let (content, errors) = fallible::parse(
            r#"{
                "hosts": { "entries": [{ "name": "legacy" }] },
                "keymap": { "baseKeymap": "vscode" }
            }"#,
        );
        assert!(errors.is_empty());

        let json = serde_json::to_value(content).unwrap();
        let json_str = json.to_string();
        for forbidden in ["hosts", "keymap", "legacy", "baseKeymap"] {
            assert!(
                !json_str.to_lowercase().contains(&forbidden.to_lowercase()),
                "settings JSON must never contain removed capability field {forbidden}"
            );
        }
    }
}
