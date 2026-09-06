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
    /// Legacy corner-radius base, in px (T20-007: superseded by
    /// `corner_radius_scale`; kept so old settings files keep parsing and a
    /// non-default value still migrates to a scale).
    pub app_corner_radius: Option<u32>,
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
    /// Background image filename (empty = none).
    pub background_image: Option<String>,
    pub background_opacity: Option<u32>,
    pub background_blur: Option<u32>,
    pub background_tint_color: Option<String>,
    pub background_tint_opacity: Option<u32>,
    /// `"titlebar"` | `"sidebar"`.
    pub tabs_location: Option<String>,
    /// Up to two of `path`/`connection`/`host`/`uptime`/`transfer`/`busy`.
    pub sidebar_tab_info_line: Option<Vec<String>>,
    pub sidebar_group_by_folder: Option<bool>,
    pub sidebar_group_single_tabs: Option<bool>,
    pub badges_always_visible: Option<bool>,
    /// Legacy titlebar icon side (`"auto"` | `"left"` | `"right"`).
    pub titlebars_icons_position: Option<String>,
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
            app_corner_radius: Some(5),
            buffer_font_family: Some(String::new()),
            buffer_font_size: Some(15),
            buffer_line_height: Some(1.618),
            ui_density: Some("default".to_string()),
            corner_radius_scale: Some(1.0),
            background_image: Some(String::new()),
            background_opacity: Some(30),
            background_blur: Some(0),
            background_tint_color: Some("#000000".to_string()),
            background_tint_opacity: Some(0),
            tabs_location: Some("titlebar".to_string()),
            sidebar_tab_info_line: Some(Vec::new()),
            sidebar_group_by_folder: Some(false),
            sidebar_group_single_tabs: Some(false),
            badges_always_visible: Some(true),
            titlebars_icons_position: Some("auto".to_string()),
            zen_mode_show_header: Some(true),
            zen_mode_show_statusbar: Some(true),
        }
    }
}
