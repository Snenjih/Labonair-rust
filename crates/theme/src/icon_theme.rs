//! Icon themes (T20-006) — swappable file/folder glyph mapping.
//!
//! Where the port used to hard-code the file → icon table inside
//! `labonair-ui-kit::icon::file_icon`, this module turns that table into a
//! **JSON icon theme**: a name plus three maps
//! (`file_stems`, `file_suffixes`, `directory`/`chevron`) from a file name or
//! extension to a *glyph id*. Glyph ids are the stable kebab-case names of the
//! embedded SVG set (`labonair-ui-kit::IconName::from_glyph_id`); an unknown id
//! degrades to the default-file glyph so a hand-written user theme can never
//! crash the explorer.
//!
//! The built-in "Labonair" icon theme is generated 1:1 from the historical
//! `file_icon` mapping (see [`DEFAULT_FILE_STEMS`] / [`DEFAULT_FILE_SUFFIXES`]),
//! embedded as `assets/icon_themes/labonair.json`, and joined at runtime by
//! whatever valid `*.json` files live in `<config_dir>/labonair/icon_themes/`.
//!
//! # JSON format ([`IconThemeContent`])
//!
//! ```json
//! {
//!   "name": "Labonair",
//!   "author": "Labonair",
//!   "file_stems":    { "Dockerfile": "package", "Makefile": "terminal" },
//!   "file_suffixes": { "rs": "file-code", "ts": "braces", "tar.gz": "archive" },
//!   "directory": { "collapsed": "folder", "expanded": "folder-open" },
//!   "chevron":   { "collapsed": "chevron-right", "expanded": "chevron-down" },
//!   "default_file": "file"
//! }
//! ```
//!
//! Lookup order for a file name (see [`IconThemeContent::file_glyph`]):
//! 1. `file_stems[name]` (the whole file name, case-insensitive);
//! 2. `file_suffixes` — the **longest** dot-delimited suffix that matches
//!    (`archive.tar.gz` tries `tar.gz` before `gz`);
//! 3. `default_file`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Collapsed / expanded folder glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryIcons {
    pub collapsed: String,
    pub expanded: String,
}

/// Collapsed / expanded disclosure-chevron glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChevronIcons {
    pub collapsed: String,
    pub expanded: String,
}

/// A parsed icon-theme document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconThemeContent {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Whole-file-name → glyph id (checked first, case-insensitive).
    #[serde(default)]
    pub file_stems: BTreeMap<String, String>,
    /// Extension (possibly multi-segment, e.g. `tar.gz`) → glyph id.
    #[serde(default)]
    pub file_suffixes: BTreeMap<String, String>,
    pub directory: DirectoryIcons,
    pub chevron: ChevronIcons,
    /// Fallback glyph id for a file that matches nothing.
    pub default_file: String,
}

impl IconThemeContent {
    /// Parse from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid icon theme: {e}"))
    }

    /// Serialize (pretty) — used to regenerate the embedded built-in asset.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("failed to serialize: {e}"))
    }

    /// The glyph id for a file name, applying the stem → longest-suffix →
    /// `default_file` order documented on the module.
    pub fn file_glyph(&self, name: &str) -> &str {
        let lower = name.to_ascii_lowercase();
        if let Some(g) = self.file_stems.get(&lower) {
            return g;
        }
        // Every dot-delimited suffix, longest first: for "a.b.c" that is
        // "b.c" then "c". A leading-dot dotfile ("​.gitignore") has no
        // meaningful suffix here and falls through to `default_file` unless it
        // matched a stem above.
        let bytes = lower.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'.' && i + 1 < bytes.len() && i > 0 {
                if let Some(g) = self.file_suffixes.get(&lower[i + 1..]) {
                    return g;
                }
            }
        }
        &self.default_file
    }

    /// The folder glyph id for the given open/closed state.
    pub fn directory_glyph(&self, expanded: bool) -> &str {
        if expanded {
            &self.directory.expanded
        } else {
            &self.directory.collapsed
        }
    }

    /// The disclosure-chevron glyph id for the given open/closed state.
    pub fn chevron_glyph(&self, expanded: bool) -> &str {
        if expanded {
            &self.chevron.expanded
        } else {
            &self.chevron.collapsed
        }
    }
}

impl Default for IconThemeContent {
    /// The built-in "Labonair" icon theme, built directly from the canonical
    /// [`DEFAULT_FILE_STEMS`] / [`DEFAULT_FILE_SUFFIXES`] tables. Used as the
    /// fallback when the embedded JSON asset fails to parse.
    fn default() -> Self {
        Self {
            name: BUILTIN_ICON_THEME_NAME.to_string(),
            author: Some(BUILTIN_ICON_THEME_NAME.to_string()),
            file_stems: DEFAULT_FILE_STEMS
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            file_suffixes: DEFAULT_FILE_SUFFIXES
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            directory: DirectoryIcons {
                collapsed: "folder".to_string(),
                expanded: "folder-open".to_string(),
            },
            chevron: ChevronIcons {
                collapsed: "chevron-right".to_string(),
                expanded: "chevron-down".to_string(),
            },
            default_file: "file".to_string(),
        }
    }
}

/// The built-in icon-theme display name and stable registry id sentinel.
pub const BUILTIN_ICON_THEME_NAME: &str = "Labonair";
/// Stable id of the built-in icon theme (mirrors the theme registry's `"default"`).
pub const BUILTIN_ICON_THEME_ID: &str = "default";

/// The embedded built-in icon theme. Regenerate after editing the tables below
/// with `REGEN_BUILTIN_ICON_THEME=1 cargo test -p labonair-theme builtin_icon`.
pub const BUILTIN_ICON_THEME_JSON: &str = include_str!("../assets/icon_themes/labonair.json");

/// Metadata for one selectable icon theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconThemeMeta {
    /// File stem for user themes, [`BUILTIN_ICON_THEME_ID`] for the built-in.
    pub id: String,
    /// Display name.
    pub name: String,
    pub builtin: bool,
}

/// An unknown icon-theme id was requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconThemeNotFoundError(pub String);

impl std::fmt::Display for IconThemeNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "icon theme not found: {}", self.0)
    }
}

impl std::error::Error for IconThemeNotFoundError {}

struct RegisteredIconTheme {
    id: String,
    content: IconThemeContent,
    builtin: bool,
}

/// The embedded built-in icon theme plus whatever valid `*.json` files were
/// found in the user icon-themes directory. Never empty.
pub struct IconThemeRegistry {
    themes: Vec<RegisteredIconTheme>,
}

impl Default for IconThemeRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl IconThemeRegistry {
    /// A registry holding only the embedded built-in theme. A broken embedded
    /// asset falls back to [`IconThemeContent::default`].
    pub fn builtin() -> Self {
        let content = IconThemeContent::from_json(BUILTIN_ICON_THEME_JSON).unwrap_or_else(|e| {
            eprintln!("labonair-theme: embedded icon theme is invalid ({e}); using tables");
            IconThemeContent::default()
        });
        Self {
            themes: vec![RegisteredIconTheme {
                id: BUILTIN_ICON_THEME_ID.to_string(),
                content,
                builtin: true,
            }],
        }
    }

    /// Replace the non-built-in themes with everything valid in `dir`.
    /// Malformed / unreadable files are skipped and returned as warnings; the
    /// built-in theme always remains.
    pub fn load_user_icon_themes(&mut self, dir: &Path) -> Vec<String> {
        self.themes.retain(|t| t.builtin);
        let mut warnings = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else {
            return warnings;
        };
        let mut loaded: Vec<RegisteredIconTheme> = Vec::new();
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
            if id == BUILTIN_ICON_THEME_ID {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(e) => {
                    warnings.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            match IconThemeContent::from_json(&raw) {
                Ok(content) if !content.name.trim().is_empty() => {
                    loaded.push(RegisteredIconTheme {
                        id,
                        content,
                        builtin: false,
                    });
                }
                Ok(_) => warnings.push(format!("{}: missing name", path.display())),
                Err(e) => warnings.push(format!("{}: {e}", path.display())),
            }
        }
        loaded.sort_by(|a, b| {
            a.content
                .name
                .to_lowercase()
                .cmp(&b.content.name.to_lowercase())
        });
        self.themes.extend(loaded);
        warnings
    }

    /// Every selectable icon theme, built-in first.
    pub fn list(&self) -> Vec<IconThemeMeta> {
        self.themes
            .iter()
            .map(|t| IconThemeMeta {
                id: t.id.clone(),
                name: t.content.name.clone(),
                builtin: t.builtin,
            })
            .collect()
    }

    /// Whether `id` names a known icon theme.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_ok()
    }

    /// Look an icon theme up by its stable id (`""` / `"default"` → built-in).
    pub fn get(&self, id: &str) -> Result<&IconThemeContent, IconThemeNotFoundError> {
        let id = if id.is_empty() {
            BUILTIN_ICON_THEME_ID
        } else {
            id
        };
        self.themes
            .iter()
            .find(|t| t.id.eq_ignore_ascii_case(id) || t.content.name.eq_ignore_ascii_case(id))
            .map(|t| &t.content)
            .ok_or_else(|| IconThemeNotFoundError(id.to_string()))
    }

    /// The built-in icon theme — always present.
    pub fn builtin_theme(&self) -> &IconThemeContent {
        &self
            .themes
            .iter()
            .find(|t| t.builtin)
            .expect("registry always keeps the built-in icon theme")
            .content
    }
}

// ─────────────────────────── canonical built-in tables ──────────────────────
//
// A 1:1 transcription of the historical `labonair-ui-kit::icon::file_icon`
// match arms. Glyph ids are the `IconName` SVG stems. These two tables are the
// single source of truth for the embedded `assets/icon_themes/labonair.json`
// (regenerated by the `builtin_icon_theme_matches_legacy_file_icon` test).

/// Whole-file-name → glyph id (checked before any suffix).
pub const DEFAULT_FILE_STEMS: &[(&str, &str)] = &[
    // Containers.
    ("dockerfile", "package"),
    ("containerfile", "package"),
    (".dockerignore", "package"),
    // Build entry points.
    ("makefile", "terminal"),
    ("justfile", "terminal"),
    ("cmakelists.txt", "terminal"),
    // Package / lock manifests.
    ("cargo.toml", "package"),
    ("cargo.lock", "package"),
    ("package.json", "package"),
    ("package-lock.json", "package"),
    ("pnpm-lock.yaml", "package"),
    ("yarn.lock", "package"),
    ("go.mod", "package"),
    ("go.sum", "package"),
    ("gemfile", "package"),
    ("gemfile.lock", "package"),
    ("pyproject.toml", "package"),
    ("poetry.lock", "package"),
    ("composer.json", "package"),
    ("composer.lock", "package"),
    // Git metadata.
    (".gitignore", "git-branch"),
    (".gitattributes", "git-branch"),
    (".gitmodules", "git-branch"),
    (".git", "git-branch"),
    // Licenses / prose entry points.
    ("license", "book"),
    ("license.md", "book"),
    ("license.txt", "book"),
    ("copying", "book"),
    ("notice", "book"),
    ("readme", "book"),
    ("readme.md", "book"),
    ("readme.txt", "book"),
    ("changelog.md", "book"),
    ("contributing.md", "book"),
    // Dotenv family (the legacy `file_icon` matched every `.env*` by prefix;
    // the theme lists the common members explicitly).
    (".env", "key-round"),
    (".env.local", "key-round"),
    (".env.development", "key-round"),
    (".env.production", "key-round"),
    (".env.test", "key-round"),
    (".env.example", "key-round"),
    (".env.sample", "key-round"),
];

/// Extension → glyph id. Multi-segment keys (`tar.gz`, `d.ts`) are matched
/// before their trailing single segment (longest-suffix rule).
pub const DEFAULT_FILE_SUFFIXES: &[(&str, &str)] = &[
    // Systems / compiled languages.
    ("rs", "file-code"),
    ("c", "file-code"),
    ("h", "file-code"),
    ("cc", "file-code"),
    ("cpp", "file-code"),
    ("cxx", "file-code"),
    ("hpp", "file-code"),
    ("hxx", "file-code"),
    ("go", "file-code"),
    ("zig", "file-code"),
    ("swift", "file-code"),
    ("kt", "file-code"),
    ("kts", "file-code"),
    ("java", "file-code"),
    ("scala", "file-code"),
    ("clj", "file-code"),
    ("cljs", "file-code"),
    ("ex", "file-code"),
    ("exs", "file-code"),
    ("erl", "file-code"),
    ("hs", "file-code"),
    ("ml", "file-code"),
    ("fs", "file-code"),
    ("dart", "file-code"),
    ("nim", "file-code"),
    ("d", "file-code"),
    ("cs", "file-code"),
    ("vb", "file-code"),
    ("m", "file-code"),
    ("mm", "file-code"),
    ("ipynb", "file-code"),
    // TypeScript declaration files — longest-suffix wins over `ts`.
    ("d.ts", "file-code"),
    // JS / TS / component languages.
    ("js", "braces"),
    ("mjs", "braces"),
    ("cjs", "braces"),
    ("jsx", "braces"),
    ("ts", "braces"),
    ("mts", "braces"),
    ("cts", "braces"),
    ("tsx", "braces"),
    ("coffee", "braces"),
    ("vue", "braces"),
    ("svelte", "braces"),
    ("astro", "braces"),
    ("elm", "braces"),
    // Other scripting languages.
    ("py", "brackets"),
    ("pyi", "brackets"),
    ("pyw", "brackets"),
    ("rb", "brackets"),
    ("php", "brackets"),
    ("phtml", "brackets"),
    ("lua", "brackets"),
    ("pl", "brackets"),
    ("pm", "brackets"),
    ("r", "brackets"),
    ("jl", "brackets"),
    ("groovy", "brackets"),
    ("gd", "brackets"),
    ("tcl", "brackets"),
    ("rkt", "brackets"),
    ("raku", "brackets"),
    // Shell scripts.
    ("sh", "file-terminal"),
    ("bash", "file-terminal"),
    ("zsh", "file-terminal"),
    ("fish", "file-terminal"),
    ("ps1", "file-terminal"),
    ("psm1", "file-terminal"),
    ("bat", "file-terminal"),
    ("cmd", "file-terminal"),
    ("nu", "file-terminal"),
    ("awk", "file-terminal"),
    ("sed", "file-terminal"),
    // JSON.
    ("json", "file-json"),
    ("jsonc", "file-json"),
    ("json5", "file-json"),
    ("geojson", "file-json"),
    ("ndjson", "file-json"),
    ("jsonl", "file-json"),
    // Data / config.
    ("yaml", "hash"),
    ("yml", "hash"),
    ("toml", "hash"),
    ("ini", "hash"),
    ("cfg", "hash"),
    ("conf", "hash"),
    ("properties", "hash"),
    ("editorconfig", "hash"),
    ("env", "hash"),
    ("dotenv", "hash"),
    ("tf", "hash"),
    ("tfvars", "hash"),
    ("hcl", "hash"),
    ("nix", "hash"),
    // XML family.
    ("xml", "file-code"),
    ("plist", "file-code"),
    ("xsd", "file-code"),
    ("xsl", "file-code"),
    ("xslt", "file-code"),
    ("rss", "file-code"),
    ("atom", "file-code"),
    // Tabular data / spreadsheets.
    ("csv", "table"),
    ("tsv", "table"),
    ("xlsx", "table"),
    ("xls", "table"),
    ("ods", "table"),
    ("parquet", "table"),
    ("arrow", "table"),
    ("lock", "package"),
    // Databases.
    ("sql", "database"),
    ("db", "database"),
    ("sqlite", "database"),
    ("sqlite3", "database"),
    ("duckdb", "database"),
    ("prisma", "database"),
    ("graphql", "database"),
    ("gql", "database"),
    // Docs / prose.
    ("md", "book"),
    ("markdown", "book"),
    ("mdx", "book"),
    ("rst", "book"),
    ("adoc", "book"),
    ("asciidoc", "book"),
    ("org", "book"),
    ("tex", "book"),
    ("typ", "book"),
    ("pdf", "book"),
    ("epub", "book"),
    ("mobi", "book"),
    ("azw3", "book"),
    ("doc", "file-text"),
    ("docx", "file-text"),
    ("odt", "file-text"),
    ("pages", "file-text"),
    ("rtf", "file-text"),
    ("ppt", "file-text"),
    ("pptx", "file-text"),
    ("odp", "file-text"),
    ("keynote", "file-text"),
    ("txt", "file-text"),
    ("text", "file-text"),
    ("log", "file-text"),
    ("nfo", "file-text"),
    ("me", "file-text"),
    // Styles.
    ("css", "palette"),
    ("scss", "palette"),
    ("sass", "palette"),
    ("less", "palette"),
    ("styl", "palette"),
    ("pcss", "palette"),
    // Markup.
    ("html", "globe"),
    ("htm", "globe"),
    ("xhtml", "globe"),
    ("ejs", "globe"),
    ("hbs", "globe"),
    ("handlebars", "globe"),
    ("njk", "globe"),
    ("pug", "globe"),
    ("haml", "globe"),
    ("liquid", "globe"),
    ("mustache", "globe"),
    ("erb", "globe"),
    ("twig", "globe"),
    // Fonts.
    ("ttf", "type"),
    ("otf", "type"),
    ("woff", "type"),
    ("woff2", "type"),
    ("eot", "type"),
    ("pfb", "type"),
    // Images.
    ("png", "image"),
    ("jpg", "image"),
    ("jpeg", "image"),
    ("gif", "image"),
    ("webp", "image"),
    ("bmp", "image"),
    ("ico", "image"),
    ("icns", "image"),
    ("avif", "image"),
    ("tiff", "image"),
    ("tif", "image"),
    ("psd", "image"),
    ("ai", "image"),
    ("sketch", "image"),
    ("fig", "image"),
    ("xcf", "image"),
    ("heic", "image"),
    ("svg", "image"),
    // Video.
    ("mp4", "film"),
    ("mkv", "film"),
    ("mov", "film"),
    ("avi", "film"),
    ("webm", "film"),
    ("flv", "film"),
    ("m4v", "film"),
    ("mpg", "film"),
    ("mpeg", "film"),
    ("wmv", "film"),
    ("3gp", "film"),
    // Audio.
    ("mp3", "music"),
    ("wav", "music"),
    ("flac", "music"),
    ("ogg", "music"),
    ("m4a", "music"),
    ("aac", "music"),
    ("opus", "music"),
    ("mid", "music"),
    ("midi", "music"),
    ("aiff", "music"),
    ("wma", "music"),
    // Archives.
    ("zip", "archive"),
    ("tar", "archive"),
    ("gz", "archive"),
    ("tgz", "archive"),
    ("bz2", "archive"),
    ("xz", "archive"),
    ("7z", "archive"),
    ("rar", "archive"),
    ("zst", "archive"),
    ("lz4", "archive"),
    ("cab", "archive"),
    // Compound archive suffixes — longest-suffix rule (same glyph as the tail,
    // so no behavior change; they exist to exercise multi-segment matching).
    ("tar.gz", "archive"),
    ("tar.bz2", "archive"),
    ("tar.xz", "archive"),
    ("tar.zst", "archive"),
    // OS packages / disk images.
    ("deb", "package"),
    ("rpm", "package"),
    ("dmg", "package"),
    ("pkg", "package"),
    ("msi", "package"),
    ("apk", "package"),
    ("appimage", "package"),
    ("iso", "package"),
    ("img", "package"),
    ("snap", "package"),
    ("flatpak", "package"),
    // Binaries / objects.
    ("wasm", "binary"),
    ("bin", "binary"),
    ("exe", "binary"),
    ("dll", "binary"),
    ("so", "binary"),
    ("dylib", "binary"),
    ("a", "binary"),
    ("o", "binary"),
    ("obj", "binary"),
    ("class", "binary"),
    ("jar", "binary"),
    ("pyc", "binary"),
    ("pyd", "binary"),
    ("node", "binary"),
    // Keys / certificates / secrets.
    ("pem", "key-round"),
    ("key", "key-round"),
    ("crt", "key-round"),
    ("cer", "key-round"),
    ("p12", "key-round"),
    ("pfx", "key-round"),
    ("gpg", "key-round"),
    ("asc", "key-round"),
    ("keychain", "key-round"),
    ("pub", "key-round"),
    ("ppk", "key-round"),
    ("kdbx", "key-round"),
    ("csr", "key-round"),
    // Diffs / patches.
    ("diff", "git-compare"),
    ("patch", "git-compare"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_icon_theme_json_round_trips() {
        // Optional regen: rewrite the asset from the canonical tables.
        if std::env::var("REGEN_BUILTIN_ICON_THEME").is_ok() {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/icon_themes/labonair.json"
            );
            std::fs::write(path, IconThemeContent::default().to_json().unwrap()).unwrap();
        }
        let embedded = IconThemeContent::from_json(BUILTIN_ICON_THEME_JSON).unwrap();
        assert_eq!(embedded, IconThemeContent::default());
    }

    #[test]
    fn file_stem_beats_suffix_and_case_folds() {
        let t = IconThemeContent::default();
        // `cmakelists.txt` is a stem → "terminal", not the `.txt` suffix.
        assert_eq!(t.file_glyph("CMakeLists.txt"), "terminal");
        assert_eq!(t.file_glyph("cmakelists.txt"), "terminal");
        assert_eq!(t.file_glyph("plain.txt"), "file-text");
    }

    #[test]
    fn longest_suffix_wins() {
        let t = IconThemeContent::default();
        assert_eq!(t.file_glyph("archive.tar.gz"), "archive");
        assert_eq!(t.file_glyph("bundle.gz"), "archive");
        // `d.ts` (declaration file) is matched before the trailing `ts`.
        assert_eq!(t.file_glyph("types.d.ts"), "file-code");
        assert_eq!(t.file_glyph("app.ts"), "braces");
    }

    #[test]
    fn unknown_falls_back_to_default_file() {
        let t = IconThemeContent::default();
        assert_eq!(t.file_glyph("mystery.qqq"), "file");
        assert_eq!(t.file_glyph("noextension"), "file");
        assert_eq!(t.directory_glyph(false), "folder");
        assert_eq!(t.directory_glyph(true), "folder-open");
        assert_eq!(t.chevron_glyph(false), "chevron-right");
        assert_eq!(t.chevron_glyph(true), "chevron-down");
    }

    #[test]
    fn registry_loads_user_themes_and_skips_broken() {
        let dir = std::env::temp_dir().join(format!("labonair-icon-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mono.json"),
            r#"{ "name": "Mono", "file_suffixes": { "rs": "binary" },
                "directory": { "collapsed": "folder", "expanded": "folder-open" },
                "chevron": { "collapsed": "chevron-right", "expanded": "chevron-down" },
                "default_file": "file" }"#,
        )
        .unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();

        let mut reg = IconThemeRegistry::builtin();
        let warnings = reg.load_user_icon_themes(&dir);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("broken.json"));
        assert!(reg.contains("mono"));
        assert!(reg.contains("default"));
        assert_eq!(reg.get("mono").unwrap().file_glyph("main.rs"), "binary");

        // Live reload: drop the file → gone, built-in stays.
        std::fs::remove_file(dir.join("mono.json")).unwrap();
        reg.load_user_icon_themes(&dir);
        assert!(!reg.contains("mono"));
        assert!(reg.contains("default"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_id_is_typed_error() {
        let reg = IconThemeRegistry::builtin();
        let err = reg.get("nope").unwrap_err();
        assert_eq!(err, IconThemeNotFoundError("nope".to_string()));
        assert!(err.to_string().contains("icon theme not found"));
    }
}
