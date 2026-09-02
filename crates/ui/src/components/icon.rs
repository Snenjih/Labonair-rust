//! Icon system.
//!
//! The reference renders every glyph as an SVG (Hugeicons for UI, Catppuccin
//! for files). The port ships the equivalent [Lucide] icons under
//! `crates/ui/assets/icons/`, served by [`crate::assets::Assets`], and this
//! enum is the single lookup from a semantic name to an asset path.
//!
//! [Lucide]: https://lucide.dev

use gpui::{px, svg, Hsla, Styled, Svg};

macro_rules! icon_enum {
    ($($variant:ident => $file:literal),* $(,)?) => {
        /// A bundled icon. Every variant maps to `icons/<file>.svg` in the
        /// asset bundle (see `crate::assets`).
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum IconName {
            $($variant),*
        }

        impl IconName {
            /// The asset path passed to `gpui::svg().path(..)`.
            pub fn path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file, ".svg")),*
                }
            }

            /// Every variant — used by the asset round-trip test.
            #[cfg(test)]
            const ALL: &'static [IconName] = &[$(IconName::$variant),*];
        }
    };
}

icon_enum! {
    ArrowDownUp => "arrow-down-up",
    Braces => "braces",
    CircleCheck => "circle-check",
    CircleX => "circle-x",
    ChevronDown => "chevron-down",
    ChevronRight => "chevron-right",
    Command => "command",
    Copy => "copy",
    CornerDownRight => "corner-down-right",
    Eye => "eye",
    EyeOff => "eye-off",
    File => "file",
    FileCode => "file-code",
    FileText => "file-text",
    Folder => "folder",
    FolderOpen => "folder-open",
    GitBranch => "git-branch",
    GitCompare => "git-compare",
    Globe => "globe",
    Home => "house",
    Image => "image",
    Info => "info",
    Link => "link",
    Menu => "menu",
    Minus => "minus",
    PanelLeft => "panel-left",
    Paperclip => "paperclip",
    Pencil => "pencil",
    Plus => "plus",
    Refresh => "refresh-cw",
    Scissors => "scissors",
    Search => "search",
    Shield => "shield",
    Sparkles => "sparkles",
    Square => "square",
    SquareCheck => "square-check-big",
    SquarePen => "square-pen",
    Terminal => "terminal",
    Trash => "trash-2",
    Warning => "triangle-alert",
    X => "x",
}

impl IconName {
    /// A `size-4` (16px) `svg()` element tinted `color`. Callers override
    /// `.size(..)` for other scales.
    pub fn svg(self, color: Hsla) -> Svg {
        svg()
            .path(self.path())
            .size(px(16.0))
            .flex_none()
            .text_color(color)
    }
}

/// Resolves a file name to its icon (mirrors the reference `iconResolver.ts`
/// extension table, reduced to the port's Lucide set).
pub fn file_icon(name: &str) -> IconName {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "rs" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp" | "go"
        | "py" | "rb" | "java" | "sh" | "bash" | "zsh" => IconName::FileCode,
        "json" | "toml" | "yaml" | "yml" | "lock" | "ini" | "conf" => IconName::Braces,
        "md" | "markdown" | "txt" | "text" | "rst" | "adoc" => IconName::FileText,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "avif" => IconName::Image,
        _ => IconName::File,
    }
}

/// Folder icon by open/closed state.
pub fn folder_icon(expanded: bool) -> IconName {
    if expanded {
        IconName::FolderOpen
    } else {
        IconName::Folder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use gpui::AssetSource;

    #[test]
    fn every_icon_variant_has_an_asset() {
        for icon in IconName::ALL {
            assert!(
                Assets.load(icon.path()).unwrap().is_some(),
                "missing asset for {icon:?} ({})",
                icon.path()
            );
        }
    }

    #[test]
    fn file_icon_maps_known_extensions() {
        assert_eq!(file_icon("main.rs"), IconName::FileCode);
        assert_eq!(file_icon("Cargo.toml"), IconName::Braces);
        assert_eq!(file_icon("weird.unknownext"), IconName::File);
    }
}
