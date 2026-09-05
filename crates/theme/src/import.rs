//! User theme import/export (T02-003).
//!
//! Labonair lets a user import a theme as a JSON file, export the active theme,
//! and keep a list of imported themes. This module owns the *conversion* between
//! that on-disk JSON format ([`ThemeFile`]) and the typed [`Theme`] the renderer
//! consumes. Persistence (writing the files, listing them, deleting user
//! themes) lives in the backend `themes` module — this crate only parses and
//! converts, it never executes anything from a theme file.
//!
//! # JSON schema (compatible with Labonair's existing theme files)
//!
//! ```json
//! {
//!   "name": "My Theme",           // required
//!   "author": "you",              // optional
//!   "author_url": "https://…",    // optional
//!   "version": "1.0.0",           // optional
//!   "description": "…",           // optional
//!   "variants": {                  // required, >=1 "light" and >=1 "dark"
//!     "dark":  { "mode": "dark",  "label": "Dark",  "colors": { "background": "#181818", … } },
//!     "light": { "mode": "light", "label": "Light", "colors": { … } }
//!   }
//! }
//! ```
//!
//! `colors` is a flat map of token name → color string. Recognised color
//! formats: `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`, `rgb(…)` / `rgba(…)`,
//! `oklch(…)`, and `transparent`. Unknown token names and unparseable color
//! values are skipped (the built-in default value for that token is kept) and
//! reported as warnings — a malformed file never aborts the import of the rest.
//! Any token the file omits keeps its built-in default. The full list of token
//! names is in [`COLOR_TOKENS`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::color::{parse_color, to_hex};
use crate::tokens::Theme;

/// One color scheme inside a [`ThemeFile`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeFileVariant {
    /// `"light"` or `"dark"` — which system appearance this variant is for.
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Token name → color string. See module docs for the format.
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

/// A parsed Labonair theme file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeFile {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub variants: BTreeMap<String, ThemeFileVariant>,
}

impl ThemeFile {
    /// Parse a theme file from JSON text.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid theme JSON: {e}"))
    }

    /// Serialize back to pretty JSON.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("failed to serialize theme: {e}"))
    }

    /// A theme file must carry a name and at least one `light` and one `dark`
    /// variant (matching Labonair's own validation).
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("theme 'name' must not be empty".to_string());
        }
        if self.variants.is_empty() {
            return Err("'variants' must contain at least one entry".to_string());
        }
        let has_light = self.variants.values().any(|v| v.mode == "light");
        let has_dark = self.variants.values().any(|v| v.mode == "dark");
        if !has_light || !has_dark {
            return Err(
                "'variants' must include at least one entry with mode \"light\" and one with mode \"dark\""
                    .to_string(),
            );
        }
        Ok(())
    }

    /// Resolve the variant to render. If `variant_key` names an existing
    /// variant whose `mode` matches the requested appearance, it wins;
    /// otherwise fall back to the first variant of the right mode, then any.
    fn resolve_variant(&self, dark: bool, variant_key: Option<&str>) -> Option<&ThemeFileVariant> {
        let want = if dark { "dark" } else { "light" };
        if let Some(key) = variant_key {
            if let Some(v) = self.variants.get(key) {
                if v.mode == want {
                    return Some(v);
                }
            }
        }
        self.variants
            .values()
            .find(|v| v.mode == want)
            .or_else(|| self.variants.values().next())
    }

    /// The `(key, label)` pairs of every variant matching the given appearance,
    /// in key order — for a variant picker.
    pub fn variant_choices(&self, dark: bool) -> Vec<(String, String)> {
        let want = if dark { "dark" } else { "light" };
        self.variants
            .iter()
            .filter(|(_, v)| v.mode == want)
            .map(|(k, v)| (k.clone(), v.label.clone().unwrap_or_else(|| k.clone())))
            .collect()
    }
}

impl Theme {
    /// Build a renderable [`Theme`] from a [`ThemeFile`] for the given
    /// appearance. Starts from the built-in default for that mode and overlays
    /// every token the file provides. Unknown tokens / unparseable colors are
    /// returned as warnings rather than failing the whole import.
    pub fn from_theme_file(file: &ThemeFile, dark: bool) -> Result<(Theme, Vec<String>), String> {
        Self::from_theme_file_variant(file, dark, None)
    }

    /// As [`Self::from_theme_file`], but selects a named variant when
    /// `variant_key` matches an existing variant of the requested appearance.
    pub fn from_theme_file_variant(
        file: &ThemeFile,
        dark: bool,
        variant_key: Option<&str>,
    ) -> Result<(Theme, Vec<String>), String> {
        let variant = file
            .resolve_variant(dark, variant_key)
            .ok_or_else(|| "theme has no variants".to_string())?;

        let mut theme = if dark { Theme::dark() } else { Theme::light() };
        let mut warnings = Vec::new();

        for (key, value) in &variant.colors {
            match parse_color(value) {
                Ok(color) => {
                    if !set_token(&mut theme, key, color) {
                        warnings.push(format!("unknown color token {key:?} — ignored"));
                    }
                }
                Err(e) => warnings.push(format!(
                    "bad color for {key:?} ({value:?}): {e} — kept default"
                )),
            }
        }

        Ok((theme, warnings))
    }

    /// Serialize this theme's colors into a reusable [`ThemeFile`]. The same
    /// color set is written to both a `light` and a `dark` variant so the
    /// result always re-imports cleanly; edit the JSON afterwards to make the
    /// two modes differ.
    pub fn to_theme_file(&self, name: impl Into<String>, author: impl Into<String>) -> ThemeFile {
        let colors: BTreeMap<String, String> = COLOR_TOKENS
            .iter()
            .map(|&token| (token.to_string(), to_hex(get_token(self, token))))
            .collect();

        let variant = |mode: &str, label: &str| ThemeFileVariant {
            mode: mode.to_string(),
            label: Some(label.to_string()),
            colors: colors.clone(),
        };

        let mut variants = BTreeMap::new();
        variants.insert("light".to_string(), variant("light", "Light"));
        variants.insert("dark".to_string(), variant("dark", "Dark"));

        ThemeFile {
            name: name.into(),
            author: author.into(),
            author_url: String::new(),
            version: String::new(),
            description: String::new(),
            variants,
        }
    }
}

/// Every color token name understood by [`Theme::from_theme_file`] /
/// written by [`Theme::to_theme_file`]. Matches Labonair's dot-notation keys.
pub const COLOR_TOKENS: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card_foreground",
    "popover",
    "popover_foreground",
    "primary",
    "primary_foreground",
    "secondary",
    "secondary_foreground",
    "muted",
    "muted_foreground",
    "accent",
    "accent_foreground",
    "destructive",
    "destructive_foreground",
    "border",
    "input",
    "ring",
    "sidebar.background",
    "sidebar.foreground",
    "sidebar.primary",
    "sidebar.primary_foreground",
    "sidebar.accent",
    "sidebar.accent_foreground",
    "sidebar.border",
    "sidebar.ring",
    "toolbar.background",
    "title_bar.background",
    "status_bar.background",
    "border.variant",
    "border.focused",
    "border.selected",
    "border.transparent",
    "border.disabled",
    "modified",
    "error",
    "warning",
    "info",
    "hint",
    "success",
    "cursor",
    "selection",
    "terminal.background",
    "terminal.foreground",
    "terminal.bright_foreground",
    "terminal.dim_foreground",
    "terminal.ansi.black",
    "terminal.ansi.red",
    "terminal.ansi.green",
    "terminal.ansi.yellow",
    "terminal.ansi.blue",
    "terminal.ansi.magenta",
    "terminal.ansi.cyan",
    "terminal.ansi.white",
    "terminal.ansi.bright_black",
    "terminal.ansi.bright_red",
    "terminal.ansi.bright_green",
    "terminal.ansi.bright_yellow",
    "terminal.ansi.bright_blue",
    "terminal.ansi.bright_magenta",
    "terminal.ansi.bright_cyan",
    "terminal.ansi.bright_white",
    "terminal.ansi.dim_black",
    "terminal.ansi.dim_red",
    "terminal.ansi.dim_green",
    "terminal.ansi.dim_yellow",
    "terminal.ansi.dim_blue",
    "terminal.ansi.dim_magenta",
    "terminal.ansi.dim_cyan",
    "terminal.ansi.dim_white",
];

/// Normalize a token key so `-` and `_` are interchangeable in the flat part.
fn norm(key: &str) -> String {
    key.replace('-', "_")
}

/// Apply one color token to `theme`. Returns `false` for an unknown key.
pub(crate) fn set_token(theme: &mut Theme, key: &str, c: gpui::Hsla) -> bool {
    match norm(key).as_str() {
        "background" => theme.core.background = c,
        "foreground" => theme.core.foreground = c,
        "card" => theme.core.card = c,
        "card_foreground" => theme.core.card_foreground = c,
        "popover" => theme.core.popover = c,
        "popover_foreground" => theme.core.popover_foreground = c,
        "primary" => theme.core.primary = c,
        "primary_foreground" => theme.core.primary_foreground = c,
        "secondary" => theme.core.secondary = c,
        "secondary_foreground" => theme.core.secondary_foreground = c,
        "muted" => theme.core.muted = c,
        "muted_foreground" => theme.core.muted_foreground = c,
        "accent" => theme.core.accent = c,
        "accent_foreground" => theme.core.accent_foreground = c,
        "destructive" => theme.core.destructive = c,
        "destructive_foreground" => theme.core.destructive_foreground = c,
        "border" => theme.core.border = c,
        "input" => theme.core.input = c,
        "ring" => theme.core.ring = c,
        "chart_1" => theme.core.charts[0] = c,
        "chart_2" => theme.core.charts[1] = c,
        "chart_3" => theme.core.charts[2] = c,
        "chart_4" => theme.core.charts[3] = c,
        "chart_5" => theme.core.charts[4] = c,
        "sidebar" | "sidebar.background" => theme.sidebar.background = c,
        "sidebar.foreground" => theme.sidebar.foreground = c,
        "sidebar.primary" => theme.sidebar.primary = c,
        "sidebar.primary_foreground" => theme.sidebar.primary_foreground = c,
        "sidebar.accent" => theme.sidebar.accent = c,
        "sidebar.accent_foreground" => theme.sidebar.accent_foreground = c,
        "sidebar.border" => theme.sidebar.border = c,
        "sidebar.ring" => theme.sidebar.ring = c,
        "toolbar.background" => theme.surface.toolbar = c,
        "title_bar.background" => theme.surface.title_bar = c,
        "status_bar.background" => theme.surface.status_bar = c,
        "border.variant" => theme.border.variant = c,
        "border.focused" => theme.border.focused = c,
        "border.selected" => theme.border.selected = c,
        "border.transparent" => theme.border.transparent = c,
        "border.disabled" => theme.border.disabled = c,
        "modified" => theme.status.modified = c,
        "error" => theme.status.error = c,
        "warning" => theme.status.warning = c,
        "info" => theme.status.info = c,
        "hint" => theme.status.hint = c,
        "success" => theme.status.success = c,
        "cursor" => {
            theme.interaction.cursor = c;
            theme.terminal.cursor = c;
        }
        "selection" => {
            theme.interaction.selection = c;
            theme.terminal.selection = c;
        }
        "terminal.background" | "terminal.ansi.background" => theme.terminal.background = c,
        "terminal.foreground" => theme.terminal.foreground = c,
        "terminal.bright_foreground" => theme.terminal.bright_foreground = c,
        "terminal.dim_foreground" => theme.terminal.dim_foreground = c,
        "terminal.ansi.black" | "terminal_black" => theme.terminal.normal.black = c,
        "terminal.ansi.red" | "terminal_red" => theme.terminal.normal.red = c,
        "terminal.ansi.green" | "terminal_green" => theme.terminal.normal.green = c,
        "terminal.ansi.yellow" | "terminal_yellow" => theme.terminal.normal.yellow = c,
        "terminal.ansi.blue" | "terminal_blue" => theme.terminal.normal.blue = c,
        "terminal.ansi.magenta" | "terminal_magenta" => theme.terminal.normal.magenta = c,
        "terminal.ansi.cyan" | "terminal_cyan" => theme.terminal.normal.cyan = c,
        "terminal.ansi.white" | "terminal_white" => theme.terminal.normal.white = c,
        "terminal.ansi.bright_black" => theme.terminal.bright.black = c,
        "terminal.ansi.bright_red" => theme.terminal.bright.red = c,
        "terminal.ansi.bright_green" => theme.terminal.bright.green = c,
        "terminal.ansi.bright_yellow" => theme.terminal.bright.yellow = c,
        "terminal.ansi.bright_blue" => theme.terminal.bright.blue = c,
        "terminal.ansi.bright_magenta" => theme.terminal.bright.magenta = c,
        "terminal.ansi.bright_cyan" => theme.terminal.bright.cyan = c,
        "terminal.ansi.bright_white" => theme.terminal.bright.white = c,
        "terminal.ansi.dim_black" => theme.terminal.dim.black = c,
        "terminal.ansi.dim_red" => theme.terminal.dim.red = c,
        "terminal.ansi.dim_green" => theme.terminal.dim.green = c,
        "terminal.ansi.dim_yellow" => theme.terminal.dim.yellow = c,
        "terminal.ansi.dim_blue" => theme.terminal.dim.blue = c,
        "terminal.ansi.dim_magenta" => theme.terminal.dim.magenta = c,
        "terminal.ansi.dim_cyan" => theme.terminal.dim.cyan = c,
        "terminal.ansi.dim_white" => theme.terminal.dim.white = c,
        _ => return false,
    }
    true
}

/// Read one color token back out — inverse of [`set_token`], used by export.
/// Only the canonical keys in [`COLOR_TOKENS`] are passed here.
pub(crate) fn get_token(theme: &Theme, key: &str) -> gpui::Hsla {
    match key {
        "background" => theme.core.background,
        "foreground" => theme.core.foreground,
        "card" => theme.core.card,
        "card_foreground" => theme.core.card_foreground,
        "popover" => theme.core.popover,
        "popover_foreground" => theme.core.popover_foreground,
        "primary" => theme.core.primary,
        "primary_foreground" => theme.core.primary_foreground,
        "secondary" => theme.core.secondary,
        "secondary_foreground" => theme.core.secondary_foreground,
        "muted" => theme.core.muted,
        "muted_foreground" => theme.core.muted_foreground,
        "accent" => theme.core.accent,
        "accent_foreground" => theme.core.accent_foreground,
        "destructive" => theme.core.destructive,
        "destructive_foreground" => theme.core.destructive_foreground,
        "border" => theme.core.border,
        "input" => theme.core.input,
        "ring" => theme.core.ring,
        "sidebar.background" => theme.sidebar.background,
        "sidebar.foreground" => theme.sidebar.foreground,
        "sidebar.primary" => theme.sidebar.primary,
        "sidebar.primary_foreground" => theme.sidebar.primary_foreground,
        "sidebar.accent" => theme.sidebar.accent,
        "sidebar.accent_foreground" => theme.sidebar.accent_foreground,
        "sidebar.border" => theme.sidebar.border,
        "sidebar.ring" => theme.sidebar.ring,
        "toolbar.background" => theme.surface.toolbar,
        "title_bar.background" => theme.surface.title_bar,
        "status_bar.background" => theme.surface.status_bar,
        "border.variant" => theme.border.variant,
        "border.focused" => theme.border.focused,
        "border.selected" => theme.border.selected,
        "border.transparent" => theme.border.transparent,
        "border.disabled" => theme.border.disabled,
        "modified" => theme.status.modified,
        "error" => theme.status.error,
        "warning" => theme.status.warning,
        "info" => theme.status.info,
        "hint" => theme.status.hint,
        "success" => theme.status.success,
        "cursor" => theme.interaction.cursor,
        "selection" => theme.interaction.selection,
        "terminal.background" => theme.terminal.background,
        "terminal.foreground" => theme.terminal.foreground,
        "terminal.bright_foreground" => theme.terminal.bright_foreground,
        "terminal.dim_foreground" => theme.terminal.dim_foreground,
        "terminal.ansi.black" => theme.terminal.normal.black,
        "terminal.ansi.red" => theme.terminal.normal.red,
        "terminal.ansi.green" => theme.terminal.normal.green,
        "terminal.ansi.yellow" => theme.terminal.normal.yellow,
        "terminal.ansi.blue" => theme.terminal.normal.blue,
        "terminal.ansi.magenta" => theme.terminal.normal.magenta,
        "terminal.ansi.cyan" => theme.terminal.normal.cyan,
        "terminal.ansi.white" => theme.terminal.normal.white,
        "terminal.ansi.bright_black" => theme.terminal.bright.black,
        "terminal.ansi.bright_red" => theme.terminal.bright.red,
        "terminal.ansi.bright_green" => theme.terminal.bright.green,
        "terminal.ansi.bright_yellow" => theme.terminal.bright.yellow,
        "terminal.ansi.bright_blue" => theme.terminal.bright.blue,
        "terminal.ansi.bright_magenta" => theme.terminal.bright.magenta,
        "terminal.ansi.bright_cyan" => theme.terminal.bright.cyan,
        "terminal.ansi.bright_white" => theme.terminal.bright.white,
        "terminal.ansi.dim_black" => theme.terminal.dim.black,
        "terminal.ansi.dim_red" => theme.terminal.dim.red,
        "terminal.ansi.dim_green" => theme.terminal.dim.green,
        "terminal.ansi.dim_yellow" => theme.terminal.dim.yellow,
        "terminal.ansi.dim_blue" => theme.terminal.dim.blue,
        "terminal.ansi.dim_magenta" => theme.terminal.dim.magenta,
        "terminal.ansi.dim_cyan" => theme.terminal.dim.cyan,
        "terminal.ansi.dim_white" => theme.terminal.dim.white,
        other => unreachable!("get_token called with non-canonical key {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::to_rgb8;

    const SAMPLE: &str = r##"{
        "name": "Sample",
        "author": "tester",
        "variants": {
            "dark":  { "mode": "dark",  "colors": { "background": "#101010", "primary": "#ff0000", "terminal.ansi.green": "#00ff00" } },
            "light": { "mode": "light", "colors": { "background": "#fafafa", "primary": "#0000ff" } }
        }
    }"##;

    #[test]
    fn parses_and_converts_matching_variant() {
        let file = ThemeFile::from_json(SAMPLE).unwrap();
        file.validate().unwrap();

        let (dark, warnings) = Theme::from_theme_file(&file, true).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(dark.is_dark);
        assert_eq!(to_rgb8(dark.core.background), [0x10, 0x10, 0x10]);
        assert_eq!(to_rgb8(dark.core.primary), [0xff, 0x00, 0x00]);
        assert_eq!(to_rgb8(dark.terminal.normal.green), [0x00, 0xff, 0x00]);
        // A token the file omits keeps the built-in dark default.
        assert_eq!(dark.core.foreground, Theme::dark().core.foreground);

        let (light, _) = Theme::from_theme_file(&file, false).unwrap();
        assert_eq!(to_rgb8(light.core.primary), [0x00, 0x00, 0xff]);
    }

    #[test]
    fn named_variant_selection_picks_the_requested_scheme() {
        let json = r##"{
            "name": "Catppuccin",
            "variants": {
                "latte":     { "mode": "light", "colors": { "primary": "#111111" } },
                "frappe":    { "mode": "dark",  "label": "Frappé",    "colors": { "primary": "#222222" } },
                "macchiato": { "mode": "dark",  "label": "Macchiato", "colors": { "primary": "#333333" } },
                "mocha":     { "mode": "dark",  "label": "Mocha",     "colors": { "primary": "#444444" } }
            }
        }"##;
        let file = ThemeFile::from_json(json).unwrap();
        file.validate().unwrap();

        // No key → first dark variant in key order (`frappe`).
        let (def, _) = Theme::from_theme_file_variant(&file, true, None).unwrap();
        assert_eq!(to_rgb8(def.core.primary), [0x22, 0x22, 0x22]);

        // Explicit key applies that variant.
        let (mocha, _) = Theme::from_theme_file_variant(&file, true, Some("mocha")).unwrap();
        assert_eq!(to_rgb8(mocha.core.primary), [0x44, 0x44, 0x44]);

        // Key of the wrong appearance is ignored (falls back to first dark).
        let (fallback, _) = Theme::from_theme_file_variant(&file, true, Some("latte")).unwrap();
        assert_eq!(to_rgb8(fallback.core.primary), [0x22, 0x22, 0x22]);

        let choices = file.variant_choices(true);
        assert_eq!(
            choices,
            vec![
                ("frappe".to_string(), "Frappé".to_string()),
                ("macchiato".to_string(), "Macchiato".to_string()),
                ("mocha".to_string(), "Mocha".to_string()),
            ]
        );
    }

    #[test]
    fn bad_and_unknown_values_become_warnings_not_errors() {
        let json = r##"{
            "name": "Broken",
            "variants": {
                "dark":  { "mode": "dark",  "colors": { "background": "not-a-color", "totally_made_up": "#fff", "primary": "#123456" } },
                "light": { "mode": "light", "colors": {} }
            }
        }"##;
        let file = ThemeFile::from_json(json).unwrap();
        let (theme, warnings) = Theme::from_theme_file(&file, true).unwrap();
        assert_eq!(warnings.len(), 2);
        // valid token still applied
        assert_eq!(to_rgb8(theme.core.primary), [0x12, 0x34, 0x56]);
        // invalid one fell back to the default
        assert_eq!(theme.core.background, Theme::dark().core.background);
    }

    #[test]
    fn validate_rejects_missing_mode_or_name() {
        let only_dark = r#"{ "name": "X", "variants": { "d": { "mode": "dark", "colors": {} } } }"#;
        assert!(ThemeFile::from_json(only_dark).unwrap().validate().is_err());

        let no_name = r#"{ "name": "  ", "variants": {
            "d": { "mode": "dark", "colors": {} }, "l": { "mode": "light", "colors": {} } } }"#;
        assert!(ThemeFile::from_json(no_name).unwrap().validate().is_err());
    }

    #[test]
    fn export_round_trips() {
        let original = Theme::dark();
        let file = original.to_theme_file("Round Trip", "me");
        file.validate().expect("exported theme must be valid");

        let json = file.to_json().unwrap();
        let reparsed = ThemeFile::from_json(&json).unwrap();
        let (restored, warnings) = Theme::from_theme_file(&reparsed, true).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        for &token in COLOR_TOKENS {
            let a = to_rgb8(get_token(&original, token));
            let b = to_rgb8(get_token(&restored, token));
            assert!(
                a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| (*x as i32 - *y as i32).abs() <= 1),
                "token {token} drifted: {a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn every_canonical_token_is_settable() {
        // COLOR_TOKENS must round-trip through set_token too (export/import parity).
        let mut theme = Theme::light();
        for &token in COLOR_TOKENS {
            assert!(
                set_token(&mut theme, token, gpui::Hsla::default()),
                "{token}"
            );
        }
    }
}
