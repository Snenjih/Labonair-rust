//! Bundled static assets (icon SVGs) and the [`gpui::AssetSource`] that serves
//! them.
//!
//! The vendored icon set (`crates/shell/assets/icons/`) is a verbatim copy of
//! Zed's Lucide-derived UI glyphs (`assets/icons/*.svg`, ISC — see
//! `assets/icons/LICENSES`) plus Zed's per-language file/folder glyphs
//! (`assets/icons/file_icons/*.svg`) and a small `+ Labonair addition` set for
//! glyphs Zed has no equivalent for. `labonair_ui_kit::IconName` names the UI
//! set; `labonair_theme::icon_theme` maps file names to the `file_icons/`
//! paths. The whole tree is embedded with `rust-embed` so the list never has
//! to be maintained by hand.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

/// Every file under `crates/shell/assets/`, embedded at compile time. Keys are
/// relative to that directory (e.g. `icons/file_code.svg`,
/// `icons/file_icons/rust.svg`).
#[derive(RustEmbed)]
#[folder = "assets/"]
#[include = "icons/*"]
#[include = "icons/file_icons/*"]
pub struct EmbeddedAssets;

/// Serves the bundled assets to GPUI's `svg()`/`img()` elements and to
/// `gpui-component`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(EmbeddedAssets::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let prefix = path.trim_end_matches('/');
        Ok(EmbeddedAssets::iter()
            .filter(|p| {
                prefix.is_empty()
                    || p.as_ref() == prefix
                    || p.strip_prefix(prefix)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
            .map(|p| SharedString::from(p.into_owned()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_the_full_icon_set() {
        let n = EmbeddedAssets::iter()
            .filter(|p| p.starts_with("icons/"))
            .count();
        assert!(n > 350, "expected the full vendored icon set, got {n}");
        assert!(Assets.load("icons/file_code.svg").unwrap().is_some());
        assert!(Assets.load("icons/file_icons/rust.svg").unwrap().is_some());
    }

    /// Cross-crate invariant: every `labonair_ui_kit::IconName` variant resolves
    /// to a bundled SVG. (The enum lives in `labonair-ui-kit`; the SVG bundle
    /// lives here.)
    #[test]
    fn every_icon_variant_has_an_asset() {
        for icon in labonair_ui_kit::IconName::ALL {
            assert!(
                Assets.load(icon.path()).unwrap().is_some(),
                "missing asset for {icon:?} ({})",
                icon.path()
            );
        }
    }

    /// No dangling UI SVG: every `icons/*.svg` (excluding the `file_icons/`
    /// theme set and the `LICENSES` file) maps back to an `IconName` variant.
    #[test]
    fn no_dangling_ui_icon() {
        for p in EmbeddedAssets::iter() {
            let p = p.as_ref();
            let Some(name) = p.strip_prefix("icons/").filter(|n| n.ends_with(".svg")) else {
                continue;
            };
            if name.contains('/') {
                continue; // file_icons/*
            }
            let stem = name.trim_end_matches(".svg");
            assert!(
                labonair_ui_kit::IconName::from_glyph_id(stem).is_some(),
                "dangling icon SVG with no IconName variant: {p}"
            );
        }
    }

    /// Every file-icon path referenced by the built-in icon theme resolves to a
    /// bundled SVG.
    #[test]
    fn builtin_icon_theme_paths_all_resolve() {
        let t = labonair_theme::IconThemeContent::default();
        let mut paths: Vec<&str> = t.file_icons.values().map(|d| d.path.as_str()).collect();
        paths.push(t.directory.collapsed.as_str());
        paths.push(t.directory.expanded.as_str());
        paths.push(t.chevron.collapsed.as_str());
        paths.push(t.chevron.expanded.as_str());
        for p in paths {
            assert!(
                Assets.load(p).unwrap().is_some(),
                "built-in icon theme references a missing asset: {p}"
            );
        }
    }
}
