//! `appearance` area — theme / typography / background / tab-chrome layout.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, crate::MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct AppearanceContent {
    /// Active JSON theme id (`"default"` = built-in light/dark).
    pub app_theme: Option<String>,
    /// Active icon theme id (`"default"` = built-in "Labonair" glyph set).
    pub icon_theme: Option<String>,
    /// Per-theme light/dark variant overrides (`{ id: { light?, dark? } }`).
    pub theme_variant_overrides: Option<BTreeMap<String, Value>>,
    pub app_font_size: Option<u32>,
    pub app_line_height: Option<f32>,
    /// UI font family (full CSS stack; empty = system default).
    pub app_font_family: Option<String>,
    pub reduce_motion: Option<bool>,
    /// Editor/terminal text font family (empty = the theme's own mono family).
    pub buffer_font_family: Option<String>,
    /// Editor/terminal text font size, px (T20-007 `theme_settings` layer).
    pub buffer_font_size: Option<u32>,
    /// Editor/terminal text line-height multiple.
    pub buffer_line_height: Option<f32>,
    /// UI density (`"compact"` | `"default"` | `"comfortable"`) — spacing/size
    /// multiplier around the layout-contract base metrics (T20-007).
    pub ui_density: Option<String>,
    /// Corner-radius multiplier applied to the active theme's radius scale
    /// (`1.0` = unchanged) (T20-007).
    pub corner_radius_scale: Option<f32>,
    /// `"titlebar"` | `"sidebar"`.
    pub tabs_location: Option<String>,
    /// Zen mode: show the window header bar.
    pub zen_mode_show_header: Option<bool>,
    /// Zen mode: show the bottom status bar.
    pub zen_mode_show_statusbar: Option<bool>,
}

impl AppearanceContent {
    pub fn defaults() -> Self {
        Self {
            app_theme: Some("default".to_string()),
            icon_theme: Some("default".to_string()),
            theme_variant_overrides: Some(BTreeMap::new()),
            app_font_size: Some(16),
            app_line_height: Some(1.5),
            app_font_family: Some("\"IBM Plex Sans\", sans-serif".to_string()),
            reduce_motion: Some(false),
            buffer_font_family: Some(String::new()),
            buffer_font_size: Some(15),
            buffer_line_height: Some(1.618),
            ui_density: Some("default".to_string()),
            corner_radius_scale: Some(1.0),
            tabs_location: Some("titlebar".to_string()),
            zen_mode_show_header: Some(true),
            zen_mode_show_statusbar: Some(true),
        }
    }
}
