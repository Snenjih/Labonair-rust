//! `personalization` area — status-bar item placement + panel-toggle
//! visibility (Phase 17/18: `statusBarItemPlacements`, `panelToggleVisibility`)
//! plus the legacy per-button status-bar toggles.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct PersonalizationContent {
    /// `{ itemId: { side, hidden } }` — see
    /// `labonair-workspace::status_placements::StatusPlacement`.
    pub status_bar_item_placements: Option<BTreeMap<String, Value>>,
    /// `{ panelName: bool }` — a panel absent from the map is visible by
    /// default.
    pub panel_toggle_visibility: Option<BTreeMap<String, bool>>,

    pub status_bar_show_explorer_button: Option<bool>,
    pub status_bar_show_snippets_button: Option<bool>,
    pub status_bar_show_source_control_button: Option<bool>,
    pub status_bar_show_tabs_button: Option<bool>,
    pub status_bar_show_cwd_breadcrumb: Option<bool>,
    pub status_bar_show_preview_url: Option<bool>,
    pub status_bar_show_ai_controls: Option<bool>,
}

impl PersonalizationContent {
    pub fn defaults() -> Self {
        Self {
            status_bar_item_placements: Some(BTreeMap::new()),
            panel_toggle_visibility: Some(BTreeMap::new()),
            status_bar_show_explorer_button: Some(true),
            status_bar_show_snippets_button: Some(true),
            status_bar_show_source_control_button: Some(true),
            status_bar_show_tabs_button: Some(true),
            status_bar_show_cwd_breadcrumb: Some(true),
            status_bar_show_preview_url: Some(true),
            status_bar_show_ai_controls: Some(true),
        }
    }
}
