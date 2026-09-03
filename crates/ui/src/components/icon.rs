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
    Archive => "archive",
    ArrowDownUp => "arrow-down-up",
    Bell => "bell",
    Binary => "binary",
    Book => "book",
    Bookmark => "bookmark",
    Braces => "braces",
    Brackets => "brackets",
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
    FileTerminal => "file-terminal",
    FileText => "file-text",
    Film => "film",
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
    KeyRound => "key-round",
    Link => "link",
    Lock => "lock",
    Menu => "menu",
    Music => "music",
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
    Table => "table",
    Terminal => "terminal",
    Trash => "trash-2",
    Type => "type",
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
        return IconName::KeyRound;
    }

    let ext = lower
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .unwrap_or_default();
    match ext.as_str() {
        // Systems / compiled languages.
        "rs" | "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "go" | "zig" | "swift" | "kt"
        | "kts" | "java" | "scala" | "clj" | "cljs" | "ex" | "exs" | "erl" | "hs" | "ml" | "fs"
        | "dart" | "nim" | "d" | "cs" | "vb" | "m" | "mm" | "ipynb" => IconName::FileCode,
        // JS / TS / component languages — curly-brace family.
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" | "coffee" | "vue"
        | "svelte" | "astro" | "elm" => IconName::Braces,
        // Other scripting languages.
        "py" | "pyi" | "pyw" | "rb" | "php" | "phtml" | "lua" | "pl" | "pm" | "r" | "jl"
        | "groovy" | "gd" | "tcl" | "rkt" | "raku" => IconName::Brackets,
        // Shell scripts.
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd" | "nu" | "awk" | "sed" => {
            IconName::FileTerminal
        }
        // Data / config.
        "json" | "jsonc" | "json5" | "geojson" | "ndjson" | "jsonl" => IconName::FileJson,
        "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "properties" | "editorconfig"
        | "env" | "dotenv" | "tf" | "tfvars" | "hcl" | "nix" => IconName::Hash,
        "xml" | "plist" | "xsd" | "xsl" | "xslt" | "rss" | "atom" => IconName::FileCode,
        // Tabular data / spreadsheets.
        "csv" | "tsv" | "xlsx" | "xls" | "ods" | "parquet" | "arrow" => IconName::Table,
        "lock" => IconName::Package,
        // Databases.
        "sql" | "db" | "sqlite" | "sqlite3" | "duckdb" | "prisma" | "graphql" | "gql" => {
            IconName::Database
        }
        // Docs / prose.
        "md" | "markdown" | "mdx" | "rst" | "adoc" | "asciidoc" | "org" | "tex" | "typ" => {
            IconName::Book
        }
        "pdf" | "epub" | "mobi" | "azw3" => IconName::Book,
        "doc" | "docx" | "odt" | "pages" | "rtf" | "ppt" | "pptx" | "odp" | "keynote" => {
            IconName::FileText
        }
        "txt" | "text" | "log" | "nfo" | "me" => IconName::FileText,
        // Styles / markup.
        "css" | "scss" | "sass" | "less" | "styl" | "pcss" => IconName::Palette,
        "html" | "htm" | "xhtml" | "ejs" | "hbs" | "handlebars" | "njk" | "pug" | "haml"
        | "liquid" | "mustache" | "erb" | "twig" => IconName::Globe,
        // Fonts.
        "ttf" | "otf" | "woff" | "woff2" | "eot" | "pfb" => IconName::Type,
        // Images.
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "icns" | "avif" | "tiff"
        | "tif" | "psd" | "ai" | "sketch" | "fig" | "xcf" | "heic" | "svg" => IconName::Image,
        // Video.
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv" | "m4v" | "mpg" | "mpeg" | "wmv" | "3gp" => {
            IconName::Film
        }
        // Audio.
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "mid" | "midi" | "aiff"
        | "wma" => IconName::Music,
        // Archives.
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "lz4" | "cab" => {
            IconName::Archive
        }
        // OS packages / disk images.
        "deb" | "rpm" | "dmg" | "pkg" | "msi" | "apk" | "appimage" | "iso" | "img" | "snap"
        | "flatpak" => IconName::Package,
        // Binaries / objects.
        "wasm" | "bin" | "exe" | "dll" | "so" | "dylib" | "a" | "o" | "obj" | "class" | "jar"
        | "pyc" | "pyd" | "node" => IconName::Binary,
        // Keys / certificates / secrets.
        "pem" | "key" | "crt" | "cer" | "p12" | "pfx" | "gpg" | "asc" | "keychain" | "pub"
        | "ppk" | "kdbx" | "csr" => IconName::KeyRound,
        // Diffs / patches.
        "diff" | "patch" => IconName::GitCompare,
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
        assert_eq!(file_icon("deploy.sh"), IconName::FileTerminal);
        assert_eq!(file_icon("app.tsx"), IconName::Braces);
        assert_eq!(file_icon("script.py"), IconName::Brackets);
        assert_eq!(file_icon("README.md"), IconName::Book);
        assert_eq!(file_icon(".env.local"), IconName::KeyRound);
        assert_eq!(file_icon("id_rsa.pem"), IconName::KeyRound);
        assert_eq!(file_icon("data.csv"), IconName::Table);
        assert_eq!(file_icon("clip.mp4"), IconName::Film);
        assert_eq!(file_icon("song.flac"), IconName::Music);
        assert_eq!(file_icon("bundle.tar.gz"), IconName::Archive);
        assert_eq!(file_icon("Inter.woff2"), IconName::Type);
        assert_eq!(file_icon("logo.png"), IconName::Image);
        assert_eq!(file_icon("Dockerfile"), IconName::Package);
        assert_eq!(file_icon("weird.unknownext"), IconName::File);
    }
}
