//! `workspace` area — command palette, source control, dock / sidebar layout.

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
    pub command_palette_blur: Option<u32>,
    pub command_palette_opacity: Option<u32>,
    /// `"top"` | `"high"` | `"center"`.
    pub command_palette_position: Option<String>,
    /// `"fast"` | `"normal"` | `"slow"` | `"none"`.
    pub command_palette_animation: Option<String>,
    pub command_palette_history_size: Option<u32>,
    pub command_palette_close_on_overlay_click: Option<bool>,

    // ── Source control ───────────────────────────────────────────────────
    pub git_status_poll_interval_ms: Option<u32>,

    // ── Dock / sidebar layout reference ─────────────────────────────────
    /// T17-002 dock layout: JSON array of `DockData` (open / size / zoom /
    /// active / panel order per edge dock). Empty string = not yet
    /// persisted.
    pub dock_layout: Option<String>,
    pub sidebar_position: Option<String>,
    pub sidebar_open: Option<bool>,
    pub sidebar_active_panel: Option<String>,
    pub sidebar_right_open: Option<bool>,
    pub sidebar_right_active_panel: Option<String>,
    pub sidebar_width: Option<u32>,
    pub sidebar_right_width: Option<u32>,
}

impl WorkspaceContent {
    pub fn defaults() -> Self {
        Self {
            command_palette_search_mode: Some(PaletteSearchMode::Contains),
            command_palette_show_recent: Some(true),
            command_palette_blur: Some(4),
            command_palette_opacity: Some(95),
            command_palette_position: Some("top".to_string()),
            command_palette_animation: Some("normal".to_string()),
            command_palette_history_size: Some(5),
            command_palette_close_on_overlay_click: Some(true),

            git_status_poll_interval_ms: Some(5000),

            dock_layout: Some(String::new()),
            sidebar_position: Some("left".to_string()),
            sidebar_open: Some(true),
            sidebar_active_panel: Some("explorer".to_string()),
            sidebar_right_open: Some(false),
            sidebar_right_active_panel: Some("explorer".to_string()),
            sidebar_width: Some(225),
            sidebar_right_width: Some(225),
        }
    }
}
