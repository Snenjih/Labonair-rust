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
    Bell => "bell",
    Binary => "binary",
    Book => "book",
    Bookmark => "bookmark",
    Braces => "braces",
    CircleCheck => "circle-check",
    CircleX => "circle-x",
    ChevronDown => "chevron-down",
    ChevronRight => "chevron-right",
    Command => "command",
    Copy => "copy",
    CornerDownRight => "corner-down-right",
    Database => "database",
    Download => "download",
    Ellipsis => "ellipsis",
    Eye => "eye",
    EyeOff => "eye-off",
    File => "file",
    FileCode => "file-code",
    FileJson => "file-json",
    FileText => "file-text",
    Folder => "folder",
    FolderOpen => "folder-open",
    FolderTree => "folder-tree",
    GitBranch => "git-branch",
    GitCompare => "git-compare",
    Globe => "globe",
    Hash => "hash",
    Home => "house",
    Image => "image",
    Info => "info",
    Link => "link",
    Lock => "lock",
    Menu => "menu",
    Package => "package",
    Palette => "palette",
    MessageSquare => "message-square",
    Minus => "minus",
    PanelBottom => "panel-bottom",
    PanelLeft => "panel-left",
    PanelTop => "panel-top",
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
    Server => "server",
    Zap => "zap",
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

/// Resolves a file name to its icon (a port of the reference `iconResolver.ts`
/// extension + special-filename tables, mapped onto the port's Lucide set —
/// distinct enough that the explorer/SFTP no longer show one glyph for
/// everything).
pub fn file_icon(name: &str) -> IconName {
    let lower = name.to_ascii_lowercase();

    // Special filenames (checked before extension).
    match lower.as_str() {
        "dockerfile" | "containerfile" | ".dockerignore" => return IconName::Package,
        "makefile" | "justfile" | "cmakelists.txt" => return IconName::Terminal,
        "cargo.toml" | "cargo.lock" | "package.json" | "package-lock.json" | "pnpm-lock.yaml"
        | "yarn.lock" | "go.mod" | "go.sum" | "gemfile" | "gemfile.lock" | "pyproject.toml"
        | "poetry.lock" | "composer.json" | "composer.lock" => return IconName::Package,
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".git" => return IconName::GitBranch,
        "license" | "license.md" | "license.txt" | "copying" | "notice" => return IconName::Book,
        "readme" | "readme.md" | "readme.txt" | "changelog.md" | "contributing.md" => {
            return IconName::Book
        }
        _ => {}
    }
    if lower.starts_with(".env") {
        return IconName::Lock;
    }

    let ext = lower
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .unwrap_or_default();
    match ext.as_str() {
        // Systems / compiled languages.
        "rs" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "go" | "zig" | "swift" | "kt"
        | "kts" | "java" | "scala" | "clj" | "cljs" | "ex" | "exs" | "erl" | "hs" | "ml" | "fs"
        | "dart" | "nim" | "d" | "cs" | "vb" | "m" | "mm" => IconName::FileCode,
        // Scripting / web languages.
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" | "py" | "pyi" | "rb"
        | "php" | "lua" | "pl" | "pm" | "r" | "jl" | "groovy" | "gd" | "vue" | "svelte"
        | "astro" | "elm" => IconName::FileCode,
        // Shell.
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd" | "nu" => {
            IconName::Terminal
        }
        // Data / config.
        "json" | "jsonc" | "json5" | "geojson" | "ndjson" => IconName::FileJson,
        "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "properties" | "editorconfig"
        | "env" => IconName::Hash,
        "xml" | "plist" | "xsd" | "xsl" => IconName::Braces,
        "lock" => IconName::Package,
        // Databases.
        "sql" | "db" | "sqlite" | "sqlite3" | "duckdb" | "prisma" => IconName::Database,
        // Docs / text.
        "md" | "markdown" | "mdx" | "rst" | "adoc" | "asciidoc" | "org" | "tex" | "rtf" => {
            IconName::Book
        }
        "txt" | "text" | "log" | "csv" | "tsv" => IconName::FileText,
        // Styles.
        "css" | "scss" | "sass" | "less" | "styl" | "pcss" => IconName::Palette,
        "html" | "htm" | "xhtml" | "ejs" | "hbs" | "njk" | "pug" | "haml" | "liquid" => {
            IconName::Globe
        }
        // Images / media.
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "avif" | "tiff"
        | "psd" | "ai" | "sketch" | "fig" => IconName::Image,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "mp4" | "mkv" | "mov" | "avi"
        | "webm" | "flv" => IconName::Image,
        // Archives / binaries.
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "deb" | "rpm" | "dmg"
        | "pkg" | "appimage" => IconName::Package,
        "wasm" | "bin" | "exe" | "dll" | "so" | "dylib" | "a" | "o" | "obj" | "class" => {
            IconName::Binary
        }
        // Keys / secrets.
        "pem" | "key" | "crt" | "cer" | "p12" | "pfx" | "gpg" | "asc" | "keychain" => {
            IconName::Lock
        }
        // Notebooks / misc code.
        "ipynb" => IconName::FileCode,
        "diff" | "patch" => IconName::GitCompare,
        "pdf" | "epub" | "mobi" => IconName::Book,
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
        assert_eq!(file_icon("Cargo.toml"), IconName::Package);
        assert_eq!(file_icon("config.toml"), IconName::Hash);
        assert_eq!(file_icon("data.json"), IconName::FileJson);
        assert_eq!(file_icon("styles.scss"), IconName::Palette);
        assert_eq!(file_icon("deploy.sh"), IconName::Terminal);
        assert_eq!(file_icon("README.md"), IconName::Book);
        assert_eq!(file_icon(".env.local"), IconName::Lock);
        assert_eq!(file_icon("Dockerfile"), IconName::Package);
        assert_eq!(file_icon("weird.unknownext"), IconName::File);
    }
}
