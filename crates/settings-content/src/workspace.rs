//! `workspace` area — command palette and source control.

use serde::{Deserialize, Serialize};

use crate::MergeFrom;

/// Command-palette match strategy.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum PaletteSearchMode {
    #[default]
    Contains,
    StartsWith,
    Fuzzy,
}

impl MergeFrom for PaletteSearchMode {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceContent {
    // ── Command palette ─────────────────────────────────────────────────
    pub command_palette_search_mode: Option<PaletteSearchMode>,
    pub command_palette_show_recent: Option<bool>,
    pub command_palette_opacity: Option<u32>,
    /// `"top"` | `"high"` | `"center"`.
    pub command_palette_position: Option<String>,
    pub command_palette_history_size: Option<u32>,
    pub command_palette_close_on_overlay_click: Option<bool>,

    // ── Source control ───────────────────────────────────────────────────
    pub git_status_poll_interval_ms: Option<u32>,
}

impl WorkspaceContent {
    pub fn defaults() -> Self {
        Self {
            command_palette_search_mode: Some(PaletteSearchMode::Contains),
            command_palette_show_recent: Some(true),
            command_palette_opacity: Some(95),
            command_palette_position: Some("top".to_string()),
            command_palette_history_size: Some(5),
            command_palette_close_on_overlay_click: Some(true),

            git_status_poll_interval_ms: Some(5000),
        }
    }
}
