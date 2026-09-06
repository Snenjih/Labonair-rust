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
pub mod hosts;
pub mod keymap;
pub mod mcp;
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
use hosts::HostsContent;
use keymap::KeymapContent;
use mcp::McpContent;
use personalization::PersonalizationContent;
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
    /// Top-level (peer of `themes`), not nested under `connections` — see
    /// `docs/architecture.md` §8.1.
    pub hosts: HostsContent,
    pub workspace: WorkspaceContent,
    pub mcp: McpContent,
    pub personalization: PersonalizationContent,
    pub keymap: KeymapContent,
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
            hosts: HostsContent::defaults(),
            workspace: WorkspaceContent::defaults(),
            mcp: McpContent::defaults(),
            personalization: PersonalizationContent::defaults(),
            keymap: KeymapContent::defaults(),
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
        "hosts",
        "workspace",
        "mcp",
        "personalization",
        "keymap",
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
        for expected in ["themes", "hosts", "shortcuts", "mcp", "personalization"] {
            assert!(
                custom.contains(&expected),
                "{expected} must be registered as a Custom top-level area"
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
    fn hosts_entries_serialize_without_secrets() {
        use hosts::{HostAuthMethod, HostEntry};

        let mut content = SettingsContent::defaults();
        content.hosts.entries = Some(vec![HostEntry {
            id: "h1".into(),
            name: "prod".into(),
            address: "prod.example.com".into(),
            port: 22,
            user: "deploy".into(),
            auth_method: HostAuthMethod::PublicKey,
            credential_ref: Some("keyring:prod-deploy".into()),
            ..Default::default()
        }]);

        let json = serde_json::to_value(&content.hosts).unwrap();
        let json_str = json.to_string();
        assert!(json_str.contains("keyring:prod-deploy"));
        // Nothing named "password"/"privateKey"/"secret" ever appears.
        for forbidden in ["password", "privateKey", "secret", "passphrase"] {
            assert!(
                !json_str.to_lowercase().contains(&forbidden.to_lowercase()),
                "hosts JSON must never contain {forbidden}"
            );
        }

        let back: hosts::HostsContent = serde_json::from_value(json).unwrap();
        assert_eq!(back, content.hosts);
    }
}
