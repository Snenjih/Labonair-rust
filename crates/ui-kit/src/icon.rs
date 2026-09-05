//! Icon system.
//!
//! The reference renders every glyph as an SVG (Hugeicons for UI, Catppuccin
//! for files). The port ships the equivalent [Lucide] icons under
//! `crates/ui/assets/icons/`, served by [`crate::assets::Assets`], and this
//! enum is the single lookup from a semantic name to an asset path.
//!
//! [Lucide]: https://lucide.dev

use gpui::{px, svg, Hsla, Styled, Svg};
use labonair_theme::{icon_theme::IconThemeContent, IconThemeRegistry};
use std::sync::LazyLock;

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

            /// Every variant, in declaration order. Consumed by the asset
            /// round-trip test in `crates/ui` (the icon SVG bundle lives
            /// there, not in this crate).
            pub const ALL: &'static [IconName] = &[$(IconName::$variant),*];

            /// Resolve an icon-theme *glyph id* (the kebab-case SVG stem, e.g.
            /// `"file-code"`) to its [`IconName`]. `None` for an id outside the
            /// bundled set — see [`glyph_icon`] for the fallback behavior.
            pub fn from_glyph_id(id: &str) -> Option<IconName> {
                match id {
                    $($file => Some(IconName::$variant),)*
                    _ => None,
                }
            }
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

/// The embedded built-in icon theme, parsed once — the fallback theme behind
/// the back-compat [`file_icon`] / [`folder_icon`] wrappers.
static BUILTIN_ICON_THEME: LazyLock<IconThemeRegistry> = LazyLock::new(IconThemeRegistry::builtin);

fn builtin_icon_theme() -> &'static IconThemeContent {
    BUILTIN_ICON_THEME.builtin_theme()
}

/// Resolve an icon-theme *glyph id* to a bundled [`IconName`]. An id outside the
/// bundled set (e.g. a typo in a hand-written user icon theme) degrades to
/// [`IconName::File`] with a one-time warning — a user theme can never crash a
/// view (T20-006 warning §2).
pub fn glyph_icon(id: &str) -> IconName {
    IconName::from_glyph_id(id).unwrap_or_else(|| {
        static WARNED: std::sync::Once = std::sync::Once::new();
        WARNED.call_once(|| {
            eprintln!(
                "labonair-ui-kit: icon theme references unknown glyph {id:?} — using \"file\""
            );
        });
        IconName::File
    })
}

/// The [`IconName`] for a filesystem path under a given [`IconThemeContent`].
///
/// For directories this returns the theme's `directory` glyph for the given
/// open/closed state; for files it applies the theme's stem → longest-suffix →
/// `default_file` order. This is the icon-theme-aware replacement for the
/// hard-coded [`file_icon`] / [`folder_icon`] wrappers below.
pub fn icon_for_path(
    theme: &IconThemeContent,
    name: &str,
    is_dir: bool,
    is_expanded: bool,
) -> IconName {
    let glyph = if is_dir {
        theme.directory_glyph(is_expanded)
    } else {
        theme.file_glyph(name)
    };
    glyph_icon(glyph)
}

/// The disclosure-chevron [`IconName`] for `theme` at the given open/closed state.
pub fn chevron_icon(theme: &IconThemeContent, is_expanded: bool) -> IconName {
    glyph_icon(theme.chevron_glyph(is_expanded))
}

/// Resolves a file name to its icon under the **built-in** icon theme. Thin
/// back-compat wrapper over [`icon_for_path`] — call sites that need the user's
/// active icon theme go through [`icon_for_path`] with `ThemeStore::icon_theme`.
pub fn file_icon(name: &str) -> IconName {
    icon_for_path(builtin_icon_theme(), name, false, false)
}

/// Folder icon by open/closed state, under the built-in icon theme.
pub fn folder_icon(expanded: bool) -> IconName {
    icon_for_path(builtin_icon_theme(), "", true, expanded)
}

#[cfg(test)]
fn legacy_file_icon(name: &str) -> IconName {
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

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_theme::icon_theme::{DEFAULT_FILE_STEMS, DEFAULT_FILE_SUFFIXES};
    use std::collections::HashSet;

    #[test]
    fn every_icon_path_is_well_formed_and_unique() {
        let mut seen = HashSet::new();
        for icon in IconName::ALL {
            let path = icon.path();
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "malformed icon path for {icon:?}: {path}"
            );
            assert!(seen.insert(path), "duplicate icon path: {path}");
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

    /// The built-in icon theme must reproduce the historical `file_icon`
    /// mapping for every extension / special filename it covered (T20-006
    /// acceptance §1). Multi-segment suffix keys (`tar.gz`, `d.ts`) are skipped
    /// here — they exercise the longest-suffix rule, which the single-`rsplit`
    /// legacy function had no equivalent for (see `longest_suffix_precedence`).
    #[test]
    fn builtin_icon_theme_matches_legacy_file_icon() {
        for (stem, _) in DEFAULT_FILE_STEMS {
            assert_eq!(
                file_icon(stem),
                legacy_file_icon(stem),
                "stem {stem:?} drifted from the legacy mapping"
            );
        }
        for (suffix, _) in DEFAULT_FILE_SUFFIXES {
            if suffix.contains('.') {
                continue;
            }
            let sample = format!("sample.{suffix}");
            assert_eq!(
                file_icon(&sample),
                legacy_file_icon(&sample),
                "suffix {suffix:?} drifted from the legacy mapping"
            );
        }
    }

    #[test]
    fn longest_suffix_precedence_and_folder_glyphs() {
        // `d.ts` (declaration file) beats the trailing `ts`.
        assert_eq!(file_icon("types.d.ts"), IconName::FileCode);
        assert_eq!(file_icon("app.ts"), IconName::Braces);
        // `archive.tar.gz` resolves via `tar.gz`, same glyph as `gz`.
        assert_eq!(file_icon("archive.tar.gz"), IconName::Archive);
        assert_eq!(folder_icon(false), IconName::Folder);
        assert_eq!(folder_icon(true), IconName::FolderOpen);
    }

    #[test]
    fn unknown_glyph_id_falls_back_to_file() {
        assert_eq!(glyph_icon("file-code"), IconName::FileCode);
        assert_eq!(glyph_icon("not-a-real-glyph"), IconName::File);
    }
}
