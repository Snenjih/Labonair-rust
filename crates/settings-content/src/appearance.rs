//! `appearance` area — theme / typography / background / tab-chrome layout.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MergeFrom;

/// How panes are laid out inside a tab (`appearance.paneLayout`).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum PaneLayout {
    /// Floating, rounded panes separated by a gap.
    #[default]
    Comfortable,
    /// Edge-to-edge panes separated by a hairline.
    Compact,
}

impl MergeFrom for PaneLayout {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// UI motion level (`appearance.animations`).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Animations {
    /// Intended to follow the operating system's "Reduce motion" preference;
    /// currently resolves to `Full` (see `labonair-theme-ui` `resolve_animations`).
    #[default]
    System,
    /// All transitions.
    Full,
    /// Fades only; no movement or resizing transitions.
    Reduced,
    /// No transitions.
    Off,
}

impl MergeFrom for Animations {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

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
    /// Legacy boolean superseded by [`Self::animations`]. Still read: `true`
    /// without an explicit `animations` resolves to `Animations::Off`
    /// (`ThemeSettings`) and the legacy preferences migrator writes
    /// `animations` instead. Remove once no v2 `config.json` carries it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_motion: Option<bool>,
    /// UI motion level; `system` is meant to follow the OS "Reduce motion"
    /// preference but currently behaves like `full`.
    pub animations: Option<Animations>,
    /// Pane layout inside a tab (`comfortable` floating | `compact`).
    pub pane_layout: Option<PaneLayout>,
    /// Outline the active pane.
    pub pane_highlight: Option<bool>,
    /// Outline colour of the active pane as a hex string (`#rrggbb`).
    pub pane_highlight_color: Option<String>,
    /// Opacity of the active-pane outline, percent (0–100).
    pub pane_highlight_opacity: Option<u32>,
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
    /// Paint window chrome and surfaces translucent over the native window
    /// blur (`false` = opaque Ciel fills).
    pub translucent_surfaces: Option<bool>,
    /// How transparent Glassy surfaces are, 0–100 (only with Glassy).
    pub glass_transparency: Option<u32>,
    /// `"titlebar"` | `"sidebar"`.
    pub tabs_location: Option<String>,
    /// Tabs share the whole strip equally instead of sizing to their title.
    pub tab_fill_space: Option<bool>,
    /// Darken inactive panes so the active one stands out.
    pub dim_inactive_panes: Option<bool>,
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
            app_font_size: Some(13),
            app_line_height: Some(1.5),
            app_font_family: Some("\"Inter Variable\", sans-serif".to_string()),
            reduce_motion: None,
            animations: Some(Animations::System),
            pane_layout: Some(PaneLayout::Comfortable),
            pane_highlight: Some(false),
            pane_highlight_color: Some("#3b82f6".to_string()),
            pane_highlight_opacity: Some(100),
            buffer_font_family: Some(String::new()),
            buffer_font_size: Some(13),
            buffer_line_height: Some(1.55),
            ui_density: Some("default".to_string()),
            corner_radius_scale: Some(1.0),
            translucent_surfaces: Some(false),
            glass_transparency: Some(30),
            tabs_location: Some("titlebar".to_string()),
            tab_fill_space: Some(false),
            dim_inactive_panes: Some(false),
            zen_mode_show_header: Some(true),
            zen_mode_show_statusbar: Some(true),
        }
    }
}
