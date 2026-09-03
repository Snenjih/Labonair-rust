//! Bundled static assets (icon SVGs) and the [`gpui::AssetSource`] that serves
//! them.
//!
//! The reference app renders every UI/file/folder icon as an SVG (Hugeicons +
//! Catppuccin sets). The pure-Rust port ships the equivalent [Lucide] glyphs
//! under `crates/ui/assets/icons/` and resolves them through this source, which
//! is registered on the `gpui::Application` in `crates/app`.
//!
//! [Lucide]: https://lucide.dev (ISC licensed)

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Every embedded icon file, as `("icons/<name>.svg", bytes)`.
///
/// Keep this list in sync with [`labonair_ui_kit::IconName`].
macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((
            concat!("icons/", $name, ".svg"),
            include_bytes!(concat!("../assets/icons/", $name, ".svg")).as_slice(),
        )),*]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "archive",
    "arrow-down-up",
    "bell",
    "binary",
    "book",
    "bookmark",
    "braces",
    "brackets",
    "circle-check",
    "circle-x",
    "chevron-down",
    "chevron-right",
    "command",
    "copy",
    "corner-down-right",
    "database",
    "download",
    "ellipsis",
    "eye",
    "eye-off",
    "file",
    "file-code",
    "file-json",
    "file-terminal",
    "file-text",
    "film",
    "folder",
    "folder-open",
    "folder-tree",
    "git-branch",
    "git-compare",
    "globe",
    "hash",
    "house",
    "image",
    "info",
    "key-round",
    "link",
    "lock",
    "menu",
    "music",
    "package",
    "palette",
    "message-square",
    "minus",
    "panel-bottom",
    "panel-left",
    "panel-top",
    "paperclip",
    "pencil",
    "plus",
    "refresh-cw",
    "scissors",
    "search",
    "server",
    "shield",
    "sparkles",
    "square",
    "square-check-big",
    "square-pen",
    "table",
    "terminal",
    "trash-2",
    "triangle-alert",
    "type",
    "x",
    "zap",
];

/// Serves the bundled icon SVGs to GPUI's `svg()` element and to
/// `gpui-component`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(ICONS
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, bytes)| Cow::Borrowed(*bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_loads() {
        for (name, _) in ICONS {
            assert!(
                Assets.load(name).unwrap().is_some(),
                "asset source failed to load {name}"
            );
        }
    }

    /// Cross-crate invariant: every `labonair_ui_kit::IconName` variant resolves
    /// to a bundled SVG served by this source. (The enum lives in
    /// `labonair-ui-kit`; the SVG bundle lives here.)
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
}
