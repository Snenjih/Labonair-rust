//! Theme registry + JSON theme families (T20-005).
//!
//! Where [`crate::import`] models the *single imported custom theme* of the
//! T02-003 era, this module turns that into a **registry**: multiple theme
//! *families*, each with one or more named light/dark variants, loaded from
//! embedded JSON (the built-in "Labonair" family) plus the user's
//! `<config_dir>/labonair/themes/*.json`, listed as metadata and resolved to a
//! renderable [`Theme`] by id at runtime.
//!
//! # JSON format ([`ThemeFamilyContent`])
//!
//! ```json
//! {
//!   "name": "Labonair",
//!   "author": "Labonair",
//!   "themes": [
//!     { "name": "Labonair Light", "appearance": "light", "colors": { "background": "#f7f7f7", … } },
//!     { "name": "Labonair Dark",  "appearance": "dark",  "colors": { "background": "#181818", … } }
//!   ]
//! }
//! ```
//!
//! `colors` is a flat token-name → color-string map (same token names and color
//! grammar as [`crate::import`], see [`crate::COLOR_TOKENS`]). Any token a
//! variant omits inherits the built-in default of the **same appearance**
//! ([`Theme::light`] / [`Theme::dark`]). Unknown token names and unparseable
//! color values are skipped with a warning — a malformed entry never aborts the
//! rest of the registry.
//!
//! Legacy [`crate::ThemeFile`] documents (the `variants: { … }` map format) are
//! still accepted by [`ThemeRegistry::load_user_themes`] via
//! [`ThemeFamilyContent::from`], so existing user theme files keep working.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::color::parse_color;
use crate::import::{get_token, set_token, ThemeFile};
use crate::tokens::Theme;
use crate::COLOR_TOKENS;

/// Which system appearance a theme variant targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    /// `true` for [`Appearance::Dark`].
    pub fn is_dark(self) -> bool {
        matches!(self, Appearance::Dark)
    }

    fn base_theme(self) -> Theme {
        match self {
            Appearance::Light => Theme::light(),
            Appearance::Dark => Theme::dark(),
        }
    }
}

/// One named color scheme inside a [`ThemeFamilyContent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeVariantContent {
    /// Display name, unique within the family (e.g. `"Labonair Dark"`).
    pub name: String,
    /// `"light"` or `"dark"`.
    pub appearance: Appearance,
    /// Token name → color string. Omitted tokens inherit the built-in default.
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

/// A theme family document: a name plus one or more light/dark variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeFamilyContent {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub themes: Vec<ThemeVariantContent>,
}

impl ThemeFamilyContent {
    /// Parse from JSON. Tries the [`ThemeFamilyContent`] shape first, then falls
    /// back to the legacy [`ThemeFile`] (`variants` map) format.
    pub fn from_json(json: &str) -> Result<Self, String> {
        match serde_json::from_str::<ThemeFamilyContent>(json) {
            Ok(f) if !f.themes.is_empty() => Ok(f),
            _ => {
                let legacy = ThemeFile::from_json(json)?;
                Ok(ThemeFamilyContent::from(&legacy))
            }
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("failed to serialize theme: {e}"))
    }

    /// Build a full-color family document from a pair of renderable themes —
    /// the inverse of [`Self::resolve_variant`]. Every [`COLOR_TOKENS`] entry is
    /// written for both variants, so the result re-parses without inheritance.
    pub fn from_themes(name: impl Into<String>, light: &Theme, dark: &Theme) -> Self {
        let colors_of = |t: &Theme| -> BTreeMap<String, String> {
            COLOR_TOKENS
                .iter()
                .map(|&k| (k.to_string(), crate::to_hex(get_token(t, k))))
                .collect()
        };
        let name = name.into();
        Self {
            name: name.clone(),
            author: None,
            themes: vec![
                ThemeVariantContent {
                    name: format!("{name} Light"),
                    appearance: Appearance::Light,
                    colors: colors_of(light),
                },
                ThemeVariantContent {
                    name: format!("{name} Dark"),
                    appearance: Appearance::Dark,
                    colors: colors_of(dark),
                },
            ],
        }
    }

    /// Resolve one variant to a renderable [`Theme`]: start from the built-in
    /// default of the variant's appearance, then overlay every token it sets.
    /// Returns non-fatal warnings for unknown tokens / bad color values.
    fn resolve_variant(&self, variant: &ThemeVariantContent) -> (Theme, Vec<String>) {
        let mut theme = variant.appearance.base_theme();
        let mut warnings = Vec::new();
        for (key, value) in &variant.colors {
            match parse_color(value) {
                Ok(color) => {
                    if !set_token(&mut theme, key, color) {
                        warnings.push(format!(
                            "{}: unknown color token {key:?} — ignored",
                            self.name
                        ));
                    }
                }
                Err(e) => warnings.push(format!(
                    "{}: bad color for {key:?} ({value:?}): {e} — kept default",
                    self.name
                )),
            }
        }
        (theme, warnings)
    }

    fn first_of(&self, appearance: Appearance) -> Option<&ThemeVariantContent> {
        self.themes
            .iter()
            .find(|v| v.appearance == appearance)
            .or_else(|| self.themes.first())
    }
}

impl From<&ThemeFile> for ThemeFamilyContent {
    fn from(file: &ThemeFile) -> Self {
        let themes = file
            .variants
            .iter()
            .map(|(key, v)| ThemeVariantContent {
                name: v.label.clone().unwrap_or_else(|| key.clone()),
                appearance: if v.mode == "dark" {
                    Appearance::Dark
                } else {
                    Appearance::Light
                },
                colors: v.colors.clone(),
            })
            .collect();
        Self {
            name: file.name.clone(),
            author: (!file.author.is_empty()).then(|| file.author.clone()),
            themes,
        }
    }
}

/// Metadata for one selectable theme variant — what a picker lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeMeta {
    /// The family's stable id — the source file stem for user themes,
    /// `"default"` for the built-in family. Persisted to `appearance.app_theme`
    /// (with the variant appended, see [`Self::id`]).
    pub family_id: String,
    /// The family display name (`"Labonair"`).
    pub family: String,
    /// The variant name (`"Labonair Dark"`).
    pub variant_name: String,
    pub appearance: Appearance,
    /// `true` for the embedded built-in family.
    pub builtin: bool,
}

impl ThemeMeta {
    /// The canonical id used by [`ThemeRegistry::get`] — `"family_id/Variant"`.
    pub fn id(&self) -> String {
        format!("{}/{}", self.family_id, self.variant_name)
    }
}

/// An unknown theme id was requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeNotFoundError(pub String);

impl std::fmt::Display for ThemeNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "theme not found: {}", self.0)
    }
}

impl std::error::Error for ThemeNotFoundError {}

/// The embedded built-in family — a different serialization form of the exact
/// `globals.css` values in [`crate::tokens`]. Regenerate with
/// `REGEN_BUILTIN_THEME=1 cargo test -p labonair-theme builtin_json`.
pub const BUILTIN_THEME_JSON: &str = include_str!("../assets/themes/labonair.json");

/// The built-in family name.
pub const BUILTIN_FAMILY: &str = "Labonair";

/// The stable id of the built-in family (kept as the historical `appTheme`
/// sentinel so pre-T20-005 settings keep resolving).
pub const BUILTIN_FAMILY_ID: &str = "default";

struct RegisteredFamily {
    /// Stable id: file stem for user themes, [`BUILTIN_FAMILY_ID`] for built-in.
    id: String,
    content: ThemeFamilyContent,
    builtin: bool,
}

/// A set of theme families: the embedded built-in plus whatever valid `*.json`
/// files were found in the user themes directory.
pub struct ThemeRegistry {
    families: Vec<RegisteredFamily>,
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ThemeRegistry {
    /// A registry holding only the embedded built-in family. Never fails — a
    /// broken embedded asset falls back to the hardcoded [`Theme`] tokens.
    pub fn builtin() -> Self {
        let content = ThemeFamilyContent::from_json(BUILTIN_THEME_JSON).unwrap_or_else(|e| {
            eprintln!("labonair-theme: embedded built-in theme is invalid ({e}); using tokens");
            ThemeFamilyContent::from_themes(BUILTIN_FAMILY, &Theme::light(), &Theme::dark())
        });
        Self {
            families: vec![RegisteredFamily {
                id: BUILTIN_FAMILY_ID.to_string(),
                content,
                builtin: true,
            }],
        }
    }

    /// Replace the non-built-in families with everything valid in `dir`.
    /// Malformed / unreadable files are skipped and returned as warnings; the
    /// registry is never left empty (the built-in family always remains).
    pub fn load_user_themes(&mut self, dir: &Path) -> Vec<String> {
        self.families.retain(|f| f.builtin);
        let mut warnings = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else {
            return warnings;
        };
        let mut loaded: Vec<RegisteredFamily> = Vec::new();
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if id == BUILTIN_FAMILY_ID {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(e) => {
                    warnings.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            match ThemeFamilyContent::from_json(&raw) {
                Ok(content) if !content.themes.is_empty() && !content.name.trim().is_empty() => {
                    loaded.push(RegisteredFamily {
                        id,
                        content,
                        builtin: false,
                    });
                }
                Ok(_) => warnings.push(format!("{}: no variants", path.display())),
                Err(e) => warnings.push(format!("{}: {e}", path.display())),
            }
        }
        loaded.sort_by_key(|f| f.content.name.to_lowercase());
        self.families.extend(loaded);
        warnings
    }

    /// Every selectable variant, built-in family first, then user families by
    /// name; variants in document order within a family.
    pub fn list(&self) -> Vec<ThemeMeta> {
        self.families
            .iter()
            .flat_map(|f| {
                f.content.themes.iter().map(move |v| ThemeMeta {
                    family_id: f.id.clone(),
                    family: f.content.name.clone(),
                    variant_name: v.name.clone(),
                    appearance: v.appearance,
                    builtin: f.builtin,
                })
            })
            .collect()
    }

    /// Whether `id` names a known family or variant.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_ok()
    }

    /// Look a family up by its stable id first, then by display name.
    fn family(&self, key: &str) -> Option<&ThemeFamilyContent> {
        self.families
            .iter()
            .find(|f| f.id.eq_ignore_ascii_case(key) || f.content.name.eq_ignore_ascii_case(key))
            .map(|f| &f.content)
    }

    /// The stable family id for a display-name-or-id key.
    fn family_id(&self, key: &str) -> Option<&str> {
        self.families
            .iter()
            .find(|f| f.id.eq_ignore_ascii_case(key) || f.content.name.eq_ignore_ascii_case(key))
            .map(|f| f.id.as_str())
    }

    /// Resolve a theme id to a renderable [`Theme`].
    ///
    /// Accepted id forms:
    /// - `"Family/Variant"` — that exact variant;
    /// - `"Variant"` — the first variant with that name across all families;
    /// - `"Family"` — that family's first variant of the given `appearance`
    ///   (falls back to its first variant).
    ///
    /// `appearance` is the mode the caller wants when the id does not pin one.
    pub fn get(&self, id: &str) -> Result<Theme, ThemeNotFoundError> {
        self.resolve(id, Appearance::Dark).map(|(t, _)| t)
    }

    /// As [`Self::get`], honoring a preferred `appearance` for family-level ids
    /// and also returning any non-fatal token warnings.
    pub fn resolve(
        &self,
        id: &str,
        appearance: Appearance,
    ) -> Result<(Theme, Vec<String>), ThemeNotFoundError> {
        let not_found = || ThemeNotFoundError(id.to_string());

        if let Some((fam, var)) = id.split_once('/') {
            let family = self.family(fam).ok_or_else(not_found)?;
            let variant = family
                .themes
                .iter()
                .find(|v| v.name.eq_ignore_ascii_case(var))
                .ok_or_else(not_found)?;
            return Ok(family.resolve_variant(variant));
        }

        // Bare name: try a family first (pick its variant by appearance), then a
        // flat variant name across every family.
        if let Some(family) = self.family(id) {
            let variant = family.first_of(appearance).ok_or_else(not_found)?;
            return Ok(family.resolve_variant(variant));
        }
        for f in &self.families {
            if let Some(variant) = f
                .content
                .themes
                .iter()
                .find(|v| v.name.eq_ignore_ascii_case(id))
            {
                return Ok(f.content.resolve_variant(variant));
            }
        }
        Err(not_found())
    }

    /// The best variant of the family named by `family_id` for `appearance`,
    /// preferring `variant_name` when it names an existing variant of that
    /// appearance. Used to honor `theme_variant_overrides`.
    pub fn resolve_family_variant(
        &self,
        family_id: &str,
        appearance: Appearance,
        variant_name: Option<&str>,
    ) -> Result<(Theme, Vec<String>), ThemeNotFoundError> {
        let family = self
            .family(family_id)
            .or_else(|| {
                // `family_id` might be "Family/Variant" — take the family part.
                family_id.split_once('/').and_then(|(f, _)| self.family(f))
            })
            .ok_or_else(|| ThemeNotFoundError(family_id.to_string()))?;

        if let Some(name) = variant_name {
            if let Some(v) = family
                .themes
                .iter()
                .find(|v| v.name.eq_ignore_ascii_case(name) && v.appearance == appearance)
            {
                return Ok(family.resolve_variant(v));
            }
        }
        let variant = family
            .first_of(appearance)
            .ok_or_else(|| ThemeNotFoundError(family_id.to_string()))?;
        Ok(family.resolve_variant(variant))
    }

    /// The `(name, appearance)` variants of a family — for a variant picker.
    pub fn family_variants(&self, family_key: &str) -> Vec<(String, Appearance)> {
        let fam = family_key
            .split_once('/')
            .map(|(f, _)| f)
            .unwrap_or(family_key);
        self.family(fam)
            .map(|f| {
                f.themes
                    .iter()
                    .map(|v| (v.name.clone(), v.appearance))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The stable family id for an id (`"family/Variant"` → `"family"`; a bare
    /// family name/id → its id; a bare variant name → its owning family's id).
    pub fn family_of(&self, id: &str) -> Option<String> {
        if let Some((fam, _)) = id.split_once('/') {
            return self.family_id(fam).map(str::to_string);
        }
        if let Some(fid) = self.family_id(id) {
            return Some(fid.to_string());
        }
        self.families
            .iter()
            .find(|f| {
                f.content
                    .themes
                    .iter()
                    .any(|v| v.name.eq_ignore_ascii_case(id))
            })
            .map(|f| f.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::to_rgb8;

    fn close(a: [u8; 3], b: [u8; 3]) -> bool {
        a.iter()
            .zip(b.iter())
            .all(|(x, y)| (*x as i32 - *y as i32).abs() <= 1)
    }

    fn assert_theme_eq(got: &Theme, want: &Theme, ctx: &str) {
        assert_eq!(got.is_dark, want.is_dark, "{ctx}: is_dark");
        for &token in COLOR_TOKENS {
            assert!(
                close(
                    to_rgb8(get_token(got, token)),
                    to_rgb8(get_token(want, token))
                ),
                "{ctx}: token {token} drifted"
            );
        }
    }

    #[test]
    fn builtin_json_round_trips_to_the_hardcoded_theme() {
        // Optional regen: writes the asset from the current tokens.
        if std::env::var("REGEN_BUILTIN_THEME").is_ok() {
            let fam =
                ThemeFamilyContent::from_themes(BUILTIN_FAMILY, &Theme::light(), &Theme::dark());
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/themes/labonair.json");
            std::fs::write(path, fam.to_json().unwrap()).unwrap();
        }

        let reg = ThemeRegistry::builtin();
        let dark = reg.get("Labonair/Labonair Dark").unwrap();
        assert_theme_eq(&dark, &Theme::dark(), "builtin dark");
        let light = reg.get("Labonair/Labonair Light").unwrap();
        assert_theme_eq(&light, &Theme::light(), "builtin light");
    }

    #[test]
    fn json_theme_round_trips() {
        let fam = ThemeFamilyContent::from_themes("Trip", &Theme::light(), &Theme::dark());
        let json = fam.to_json().unwrap();
        let reparsed = ThemeFamilyContent::from_json(&json).unwrap();
        assert_eq!(fam, reparsed);
    }

    #[test]
    fn missing_tokens_inherit_the_same_appearance_default() {
        let json = r##"{
            "name": "Sparse",
            "themes": [
                { "name": "Sparse Dark",  "appearance": "dark",  "colors": { "primary": "#ff0000" } },
                { "name": "Sparse Light", "appearance": "light", "colors": {} }
            ]
        }"##;
        let mut reg = ThemeRegistry::builtin();
        // stash it as a user family via the legacy-tolerant parser
        let content = ThemeFamilyContent::from_json(json).unwrap();
        reg.families.push(RegisteredFamily {
            id: "user".to_string(),
            content,
            builtin: false,
        });

        let dark = reg.get("Sparse/Sparse Dark").unwrap();
        assert_eq!(to_rgb8(dark.core.primary), [0xff, 0x00, 0x00]);
        // Untouched token equals the dark default.
        assert_eq!(dark.core.background, Theme::dark().core.background);

        let light = reg.get("Sparse/Sparse Light").unwrap();
        assert_eq!(light.core.background, Theme::light().core.background);
    }

    #[test]
    fn unknown_id_is_a_typed_error() {
        let reg = ThemeRegistry::builtin();
        let err = reg.get("Nope/Nope").unwrap_err();
        assert_eq!(err, ThemeNotFoundError("Nope/Nope".to_string()));
        assert!(err.to_string().contains("theme not found"));
    }

    #[test]
    fn load_user_themes_skips_broken_files_with_warnings() {
        let dir = std::env::temp_dir().join(format!("labonair-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("good.json"),
            r##"{ "name": "Good", "themes": [
                { "name": "Good Dark", "appearance": "dark", "colors": {} },
                { "name": "Good Light", "appearance": "light", "colors": {} } ] }"##,
        )
        .unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();

        let mut reg = ThemeRegistry::builtin();
        let warnings = reg.load_user_themes(&dir);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("broken.json"));
        assert!(reg.contains("Good/Good Dark"));
        // Built-in still present.
        assert!(reg.contains("Labonair/Labonair Dark"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_theme_file_format_still_loads() {
        let legacy = r##"{
            "name": "Legacy",
            "variants": {
                "dark":  { "mode": "dark",  "label": "Legacy Dark",  "colors": { "primary": "#010203" } },
                "light": { "mode": "light", "label": "Legacy Light", "colors": {} }
            }
        }"##;
        let fam = ThemeFamilyContent::from_json(legacy).unwrap();
        assert_eq!(fam.name, "Legacy");
        assert_eq!(fam.themes.len(), 2);
        let mut reg = ThemeRegistry::builtin();
        reg.families.push(RegisteredFamily {
            id: "user".to_string(),
            content: fam,
            builtin: false,
        });
        let t = reg.get("Legacy/Legacy Dark").unwrap();
        assert_eq!(to_rgb8(t.core.primary), [0x01, 0x02, 0x03]);
    }

    #[test]
    fn resolve_family_variant_honors_override_then_appearance() {
        let json = r##"{
            "name": "Catppuccin",
            "themes": [
                { "name": "Latte",     "appearance": "light", "colors": { "primary": "#111111" } },
                { "name": "Frappe",    "appearance": "dark",  "colors": { "primary": "#222222" } },
                { "name": "Macchiato", "appearance": "dark",  "colors": { "primary": "#333333" } },
                { "name": "Mocha",     "appearance": "dark",  "colors": { "primary": "#444444" } }
            ]
        }"##;
        let mut reg = ThemeRegistry::builtin();
        reg.families.push(RegisteredFamily {
            id: "user".to_string(),
            content: ThemeFamilyContent::from_json(json).unwrap(),
            builtin: false,
        });

        // No override → first dark variant.
        let (def, _) = reg
            .resolve_family_variant("Catppuccin", Appearance::Dark, None)
            .unwrap();
        assert_eq!(to_rgb8(def.core.primary), [0x22, 0x22, 0x22]);
        // Override picks the named dark variant.
        let (mocha, _) = reg
            .resolve_family_variant("Catppuccin", Appearance::Dark, Some("Mocha"))
            .unwrap();
        assert_eq!(to_rgb8(mocha.core.primary), [0x44, 0x44, 0x44]);
        // Light appearance ignores a dark-only override name.
        let (latte, _) = reg
            .resolve_family_variant("Catppuccin", Appearance::Light, Some("Mocha"))
            .unwrap();
        assert_eq!(to_rgb8(latte.core.primary), [0x11, 0x11, 0x11]);
    }
}
