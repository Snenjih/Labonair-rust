//! Icon themes (T20-006, Zed-parity model — `docs/architecture.md` §8.19).
//!
//! A **swappable file/folder glyph mapping**, transcribed 1:1 from Zed
//! (`zed-refrence/zed/crates/theme/src/icon_theme.rs` +
//! `zed-refrence/zed/crates/file_icons`). Two levels of indirection, exactly
//! like Zed:
//!
//! 1. `file_stems` / `file_suffixes` map a file name or extension to an
//!    **icon key** (`"rust"`, `"typescript"`, …);
//! 2. `file_icons` maps an icon key to an **asset path**
//!    (`"icons/file_icons/rust.svg"`) — a vendored copy of Zed's per-language
//!    SVG set under `crates/shell/assets/icons/file_icons/`.
//!
//! `directory` / `chevron` (+ optional `named_directory_icons`) are direct
//! asset paths. The built-in "Labonair" theme is generated from the
//! [`DEFAULT_FILE_STEMS`] / [`DEFAULT_FILE_SUFFIXES`] / [`DEFAULT_FILE_ICONS`]
//! tables (themselves a transcription of Zed's `"Zed (Default)"`), embedded as
//! `assets/icon_themes/labonair.json`, and joined at runtime by any valid
//! `*.json` in `<config_dir>/labonair/icon_themes/`.
//!
//! # JSON format ([`IconThemeContent`])
//!
//! ```json
//! {
//!   "name": "Labonair",
//!   "file_stems":    { "Dockerfile": "docker" },
//!   "file_suffixes": { "rs": "rust", "tsx": "react" },
//!   "file_icons":    { "rust": { "path": "icons/file_icons/rust.svg" } },
//!   "named_directory_icons": {
//!     ".github": { "collapsed": "icons/file_icons/folder.svg",
//!                  "expanded":  "icons/file_icons/folder_open.svg" }
//!   },
//!   "directory": { "collapsed": "icons/file_icons/folder.svg",
//!                  "expanded":  "icons/file_icons/folder_open.svg" },
//!   "chevron":   { "collapsed": "icons/file_icons/chevron_right.svg",
//!                  "expanded":  "icons/file_icons/chevron_down.svg" },
//!   "default_file": "default"
//! }
//! ```
//!
//! Lookup order for a file name (see [`IconThemeContent::file_icon_path`],
//! mirrors Zed's `FileIcons::get_icon`):
//! 1. the whole (lower-cased) file name in `file_stems` then `file_suffixes`;
//! 2. every dot-delimited trailing suffix, **longest first**
//!    (`archive.tar.gz` tries `tar.gz` before `gz`; `.gitignore` tries
//!    `gitignore`);
//! 3. the `default_file` key.
//!
//! The resolved key is looked up in `file_icons`; a missing key falls back to
//! the `default_file` key, then to the literal `icons/file_icons/file.svg` —
//! a user theme can never blank the tree or panic.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Collapsed / expanded folder glyph **asset paths**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryIcons {
    pub collapsed: String,
    pub expanded: String,
}

/// Collapsed / expanded disclosure-chevron glyph **asset paths**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChevronIcons {
    pub collapsed: String,
    pub expanded: String,
}

/// One icon-key → asset-path entry (Zed's `IconDefinition`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconDefinition {
    pub path: String,
}

/// A parsed icon-theme document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IconThemeContent {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Whole-file-name → icon key (checked first, case-insensitive).
    #[serde(default)]
    pub file_stems: BTreeMap<String, String>,
    /// Extension (possibly multi-segment, e.g. `tar.gz`) → icon key.
    #[serde(default)]
    pub file_suffixes: BTreeMap<String, String>,
    /// Icon key → asset path.
    #[serde(default)]
    pub file_icons: BTreeMap<String, IconDefinition>,
    /// Per-name directory glyph overrides (e.g. `.github`), keyed lower-case.
    #[serde(default)]
    pub named_directory_icons: BTreeMap<String, DirectoryIcons>,
    pub directory: DirectoryIcons,
    pub chevron: ChevronIcons,
    /// Fallback icon key for a file that matches nothing.
    pub default_file: String,
}

/// Ultimate fallback when even the `default_file` key is absent.
const HARD_FALLBACK_FILE_ICON: &str = "icons/file_icons/file.svg";

impl IconThemeContent {
    /// Parse from JSON.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("invalid icon theme: {e}"))
    }

    /// Serialize (pretty) — used to regenerate the embedded built-in asset.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("failed to serialize: {e}"))
    }

    /// The icon **key** for a file name, applying the whole-name → longest-suffix
    /// → `default_file` order documented on the module.
    pub fn file_glyph_key(&self, name: &str) -> &str {
        let lower = name.to_ascii_lowercase();
        if let Some(k) = self
            .file_stems
            .get(&lower)
            .or_else(|| self.file_suffixes.get(&lower))
        {
            return k;
        }
        // Progressively shorter suffixes: for "a.b.c" that is "b.c" then "c";
        // for ".gitignore" that is "gitignore".
        let mut rest = lower.as_str();
        while let Some(idx) = rest.find('.') {
            let suffix = &rest[idx + 1..];
            if suffix.is_empty() {
                break;
            }
            if let Some(k) = self
                .file_stems
                .get(suffix)
                .or_else(|| self.file_suffixes.get(suffix))
            {
                return k;
            }
            rest = suffix;
        }
        &self.default_file
    }

    /// The resolved file-icon **asset path** for `name`.
    pub fn file_icon_path(&self, name: &str) -> &str {
        let key = self.file_glyph_key(name);
        self.file_icons
            .get(key)
            .or_else(|| self.file_icons.get(&self.default_file))
            .map(|d| d.path.as_str())
            .unwrap_or(HARD_FALLBACK_FILE_ICON)
    }

    /// The folder-icon **asset path** for a directory named `name`.
    pub fn directory_icon_path(&self, name: &str, expanded: bool) -> &str {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            if let Some(icons) = self
                .named_directory_icons
                .get(&trimmed.to_ascii_lowercase())
            {
                return if expanded {
                    &icons.expanded
                } else {
                    &icons.collapsed
                };
            }
        }
        if expanded {
            &self.directory.expanded
        } else {
            &self.directory.collapsed
        }
    }

    /// The disclosure-chevron **asset path** for the given open/closed state.
    pub fn chevron_icon_path(&self, expanded: bool) -> &str {
        if expanded {
            &self.chevron.expanded
        } else {
            &self.chevron.collapsed
        }
    }
}

impl Default for IconThemeContent {
    /// The built-in "Labonair" icon theme, built from the canonical
    /// [`DEFAULT_FILE_STEMS`] / [`DEFAULT_FILE_SUFFIXES`] / [`DEFAULT_FILE_ICONS`]
    /// tables. Used when the embedded JSON asset fails to parse.
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
            file_icons: DEFAULT_FILE_ICONS
                .iter()
                .map(|(k, p)| {
                    (
                        k.to_string(),
                        IconDefinition {
                            path: p.to_string(),
                        },
                    )
                })
                .collect(),
            named_directory_icons: BTreeMap::new(),
            directory: DirectoryIcons {
                collapsed: "icons/file_icons/folder.svg".to_string(),
                expanded: "icons/file_icons/folder_open.svg".to_string(),
            },
            chevron: ChevronIcons {
                collapsed: "icons/file_icons/chevron_right.svg".to_string(),
                expanded: "icons/file_icons/chevron_down.svg".to_string(),
            },
            default_file: "default".to_string(),
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
    /// Accepts both a bare [`IconThemeContent`] and a Zed-style *family*
    /// (`{ "name", "author", "themes": [ … ] }`). Malformed / unreadable files
    /// are skipped and returned as warnings; the built-in theme always remains.
    pub fn load_user_icon_themes(&mut self, dir: &std::path::Path) -> Vec<String> {
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
            let Some(stem) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if stem == BUILTIN_ICON_THEME_ID {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(r) => r,
                Err(e) => {
                    warnings.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            match parse_user_icon_theme_file(&raw) {
                Ok(themes) if !themes.is_empty() => {
                    let multi = themes.len() > 1;
                    for content in themes {
                        if content.name.trim().is_empty() {
                            warnings.push(format!("{}: missing name", path.display()));
                            continue;
                        }
                        let id = if multi {
                            format!("{stem}/{}", content.name)
                        } else {
                            stem.clone()
                        };
                        loaded.push(RegisteredIconTheme {
                            id,
                            content,
                            builtin: false,
                        });
                    }
                }
                Ok(_) => warnings.push(format!("{}: no themes", path.display())),
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

/// Parse a user icon-theme file as either a single theme or a Zed-style family.
fn parse_user_icon_theme_file(raw: &str) -> Result<Vec<IconThemeContent>, String> {
    #[derive(Deserialize)]
    struct Family {
        themes: Vec<IconThemeContent>,
    }
    if let Ok(fam) = serde_json::from_str::<Family>(raw) {
        if !fam.themes.is_empty() {
            return Ok(fam.themes);
        }
    }
    IconThemeContent::from_json(raw).map(|t| vec![t])
}

// ─────────────────────────── canonical built-in tables ──────────────────────
//
// A 1:1 transcription of Zed's `"Zed (Default)"` icon theme
// (`zed-refrence/zed/crates/theme/src/icon_theme.rs`): `FILE_STEMS_BY_ICON_KEY`
// and `FILE_SUFFIXES_BY_ICON_KEY` flattened to `(association, icon_key)` pairs
// (lower-cased), and `FILE_ICONS` verbatim as `(icon_key, asset_path)`. These
// three tables are the single source of truth for the embedded
// `assets/icon_themes/labonair.json` (regenerate via the
// `builtin_icon_theme_json_round_trips` test with `REGEN_BUILTIN_ICON_THEME=1`).

/// Whole-file-name → icon key (checked before any suffix).
pub const DEFAULT_FILE_STEMS: &[(&str, &str)] = &[
    ("containerfile", "docker"),
    ("dockerfile", "docker"),
    (".dockerignore", "docker"),
    ("podfile", "ruby"),
    ("procfile", "heroku"),
];

/// Extension (or whole name) → icon key. Multi-segment keys (`tar.gz`) are
/// matched before their trailing single segment (longest-suffix rule).
pub const DEFAULT_FILE_SUFFIXES: &[(&str, &str)] = &[
    ("astro", "astro"),
    ("aac", "audio"),
    ("flac", "audio"),
    ("m4a", "audio"),
    ("mka", "audio"),
    ("mp3", "audio"),
    ("ogg", "audio"),
    ("opus", "audio"),
    ("wav", "audio"),
    ("wma", "audio"),
    ("wv", "audio"),
    ("bak", "backup"),
    ("bal", "ballerina"),
    ("bicep", "bicep"),
    ("lockb", "bun"),
    ("c", "c"),
    ("h", "c"),
    ("cairo", "cairo"),
    ("handlebars", "code"),
    ("metadata", "code"),
    ("rkt", "code"),
    ("scm", "code"),
    ("coffee", "coffeescript"),
    ("c++", "cpp"),
    ("h++", "cpp"),
    ("cc", "cpp"),
    ("cpp", "cpp"),
    ("cppm", "cpp"),
    ("cxx", "cpp"),
    ("hh", "cpp"),
    ("hpp", "cpp"),
    ("hxx", "cpp"),
    ("inl", "cpp"),
    ("ixx", "cpp"),
    ("cr", "crystal"),
    ("ecr", "crystal"),
    ("cs", "csharp"),
    ("csproj", "csproj"),
    ("css", "css"),
    ("pcss", "css"),
    ("postcss", "css"),
    ("cue", "cue"),
    ("dart", "dart"),
    ("diff", "diff"),
    ("docker-compose.yml", "docker"),
    ("docker-compose.yaml", "docker"),
    ("compose.yml", "docker"),
    ("compose.yaml", "docker"),
    ("doc", "document"),
    ("docx", "document"),
    ("mdx", "document"),
    ("odp", "document"),
    ("ods", "document"),
    ("odt", "document"),
    ("pdf", "document"),
    ("ppt", "document"),
    ("pptx", "document"),
    ("rtf", "document"),
    ("txt", "document"),
    ("xls", "document"),
    ("xlsx", "document"),
    ("editorconfig", "editorconfig"),
    ("eex", "elixir"),
    ("ex", "elixir"),
    ("exs", "elixir"),
    ("heex", "elixir"),
    ("leex", "elixir"),
    ("neex", "elixir"),
    ("elm", "elm"),
    ("emakefile", "erlang"),
    ("app.src", "erlang"),
    ("erl", "erlang"),
    ("escript", "erlang"),
    ("hrl", "erlang"),
    ("rebar.config", "erlang"),
    ("xrl", "erlang"),
    ("yrl", "erlang"),
    ("eslint.config.cjs", "eslint"),
    ("eslint.config.cts", "eslint"),
    ("eslint.config.js", "eslint"),
    ("eslint.config.mjs", "eslint"),
    ("eslint.config.mts", "eslint"),
    ("eslint.config.ts", "eslint"),
    ("eslintrc", "eslint"),
    ("eslintrc.js", "eslint"),
    ("eslintrc.json", "eslint"),
    ("otf", "font"),
    ("ttf", "font"),
    ("woff", "font"),
    ("woff2", "font"),
    ("fs", "fsharp"),
    ("fsproj", "fsproj"),
    ("gitlab-ci.yml", "gitlab"),
    ("gitlab-ci.yaml", "gitlab"),
    ("gleam", "gleam"),
    ("go", "go"),
    ("mod", "go"),
    ("work", "go"),
    ("gql", "graphql"),
    ("graphql", "graphql"),
    ("graphqls", "graphql"),
    ("hs", "haskell"),
    ("hcl", "hcl"),
    ("helmfile.yaml", "helm"),
    ("helmfile.yml", "helm"),
    ("chart.yaml", "helm"),
    ("chart.yml", "helm"),
    ("chart.lock", "helm"),
    ("values.yaml", "helm"),
    ("values.yml", "helm"),
    ("requirements.yaml", "helm"),
    ("requirements.yml", "helm"),
    ("tpl", "helm"),
    ("htm", "html"),
    ("html", "html"),
    ("avif", "image"),
    ("bmp", "image"),
    ("gif", "image"),
    ("heic", "image"),
    ("heif", "image"),
    ("ico", "image"),
    ("j2k", "image"),
    ("jfif", "image"),
    ("jp2", "image"),
    ("jpeg", "image"),
    ("jpg", "image"),
    ("jxl", "image"),
    ("png", "image"),
    ("psd", "image"),
    ("qoi", "image"),
    ("svg", "image"),
    ("tiff", "image"),
    ("webp", "image"),
    ("ipynb", "ipynb"),
    ("java", "java"),
    ("cjs", "javascript"),
    ("js", "javascript"),
    ("mjs", "javascript"),
    ("json", "json"),
    ("jsonc", "json"),
    ("jl", "julia"),
    ("kdl", "kdl"),
    ("kt", "kotlin"),
    ("lock", "lock"),
    ("log", "log"),
    ("lua", "lua"),
    ("luau", "luau"),
    ("markdown", "markdown"),
    ("md", "markdown"),
    ("metal", "metal"),
    ("nim", "nim"),
    ("nims", "nim"),
    ("nimble", "nim"),
    ("nix", "nix"),
    ("ml", "ocaml"),
    ("mli", "ocaml"),
    ("mlx", "ocaml"),
    ("odin", "odin"),
    ("php", "php"),
    ("prettier.config.cjs", "prettier"),
    ("prettier.config.js", "prettier"),
    ("prettier.config.mjs", "prettier"),
    ("prettierignore", "prettier"),
    ("prettierrc", "prettier"),
    ("prettierrc.cjs", "prettier"),
    ("prettierrc.js", "prettier"),
    ("prettierrc.json", "prettier"),
    ("prettierrc.json5", "prettier"),
    ("prettierrc.mjs", "prettier"),
    ("prettierrc.toml", "prettier"),
    ("prettierrc.yaml", "prettier"),
    ("prettierrc.yml", "prettier"),
    ("prisma", "prisma"),
    ("pp", "puppet"),
    ("py", "python"),
    ("r", "r"),
    ("cjsx", "react"),
    ("ctsx", "react"),
    ("jsx", "react"),
    ("mjsx", "react"),
    ("mtsx", "react"),
    ("tsx", "react"),
    ("roc", "roc"),
    ("rb", "ruby"),
    ("rs", "rust"),
    ("sass", "sass"),
    ("scss", "sass"),
    ("scala", "scala"),
    ("sc", "scala"),
    ("conf", "settings"),
    ("ini", "settings"),
    ("sol", "solidity"),
    ("accdb", "storage"),
    ("csv", "storage"),
    ("dat", "storage"),
    ("db", "storage"),
    ("dbf", "storage"),
    ("dll", "storage"),
    ("fmp", "storage"),
    ("fp7", "storage"),
    ("frm", "storage"),
    ("gdb", "storage"),
    ("ib", "storage"),
    ("ldf", "storage"),
    ("mdb", "storage"),
    ("mdf", "storage"),
    ("myd", "storage"),
    ("myi", "storage"),
    ("pdb", "storage"),
    ("psv", "storage"),
    ("rdata", "storage"),
    ("sav", "storage"),
    ("sdf", "storage"),
    ("sql", "storage"),
    ("sqlite", "storage"),
    ("ssv", "storage"),
    ("tsv", "storage"),
    ("stylelint.config.cjs", "stylelint"),
    ("stylelint.config.js", "stylelint"),
    ("stylelint.config.mjs", "stylelint"),
    ("stylelintignore", "stylelint"),
    ("stylelintrc", "stylelint"),
    ("stylelintrc.cjs", "stylelint"),
    ("stylelintrc.js", "stylelint"),
    ("stylelintrc.json", "stylelint"),
    ("stylelintrc.mjs", "stylelint"),
    ("stylelintrc.yaml", "stylelint"),
    ("stylelintrc.yml", "stylelint"),
    ("surql", "surrealql"),
    ("svelte", "svelte"),
    ("swift", "swift"),
    ("tcl", "tcl"),
    ("hbs", "template"),
    ("plist", "template"),
    ("xml", "template"),
    ("bash", "terminal"),
    ("bash_aliases", "terminal"),
    ("bash_login", "terminal"),
    ("bash_logout", "terminal"),
    ("bash_profile", "terminal"),
    ("bashrc", "terminal"),
    ("brushrc", "terminal"),
    ("fish", "terminal"),
    ("nu", "terminal"),
    ("profile", "terminal"),
    ("ps1", "terminal"),
    ("sh", "terminal"),
    ("zlogin", "terminal"),
    ("zlogout", "terminal"),
    ("zprofile", "terminal"),
    ("zsh", "terminal"),
    ("zsh_aliases", "terminal"),
    ("zsh_histfile", "terminal"),
    ("zsh_history", "terminal"),
    ("zshenv", "terminal"),
    ("zshrc", "terminal"),
    ("tf", "terraform"),
    ("tfvars", "terraform"),
    ("toml", "toml"),
    ("cts", "typescript"),
    ("mts", "typescript"),
    ("ts", "typescript"),
    ("v", "v"),
    ("vsh", "v"),
    ("vv", "v"),
    ("commit_editmsg", "vcs"),
    ("edit_description", "vcs"),
    ("merge_msg", "vcs"),
    ("notes_editmsg", "vcs"),
    ("tag_editmsg", "vcs"),
    ("gitattributes", "vcs"),
    ("gitignore", "vcs"),
    ("gitkeep", "vcs"),
    ("gitmodules", "vcs"),
    ("vbproj", "vbproj"),
    ("avi", "video"),
    ("m4v", "video"),
    ("mkv", "video"),
    ("mov", "video"),
    ("mp4", "video"),
    ("webm", "video"),
    ("wmv", "video"),
    ("sln", "vs_sln"),
    ("suo", "vs_suo"),
    ("vue", "vue"),
    ("vy", "vyper"),
    ("vyi", "vyper"),
    ("wgsl", "wgsl"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("zig", "zig"),
];

/// Icon key → asset path (Zed's `FILE_ICONS`).
pub const DEFAULT_FILE_ICONS: &[(&str, &str)] = &[
    ("astro", "icons/file_icons/astro.svg"),
    ("audio", "icons/file_icons/audio.svg"),
    ("ballerina", "icons/file_icons/ballerina.svg"),
    ("bicep", "icons/file_icons/file.svg"),
    ("bun", "icons/file_icons/bun.svg"),
    ("c", "icons/file_icons/c.svg"),
    ("cairo", "icons/file_icons/cairo.svg"),
    ("code", "icons/file_icons/code.svg"),
    ("coffeescript", "icons/file_icons/coffeescript.svg"),
    ("cpp", "icons/file_icons/cpp.svg"),
    ("crystal", "icons/file_icons/file.svg"),
    ("csharp", "icons/file_icons/file.svg"),
    ("csproj", "icons/file_icons/file.svg"),
    ("css", "icons/file_icons/css.svg"),
    ("cue", "icons/file_icons/file.svg"),
    ("dart", "icons/file_icons/dart.svg"),
    ("default", "icons/file_icons/file.svg"),
    ("diff", "icons/file_icons/diff.svg"),
    ("docker", "icons/file_icons/docker.svg"),
    ("document", "icons/file_icons/book.svg"),
    ("editorconfig", "icons/file_icons/editorconfig.svg"),
    ("elixir", "icons/file_icons/elixir.svg"),
    ("elm", "icons/file_icons/elm.svg"),
    ("erlang", "icons/file_icons/erlang.svg"),
    ("eslint", "icons/file_icons/eslint.svg"),
    ("font", "icons/file_icons/font.svg"),
    ("fsharp", "icons/file_icons/fsharp.svg"),
    ("fsproj", "icons/file_icons/file.svg"),
    ("gitlab", "icons/file_icons/gitlab.svg"),
    ("gleam", "icons/file_icons/gleam.svg"),
    ("go", "icons/file_icons/go.svg"),
    ("graphql", "icons/file_icons/graphql.svg"),
    ("haskell", "icons/file_icons/haskell.svg"),
    ("hcl", "icons/file_icons/hcl.svg"),
    ("helm", "icons/file_icons/helm.svg"),
    ("heroku", "icons/file_icons/heroku.svg"),
    ("html", "icons/file_icons/html.svg"),
    ("image", "icons/file_icons/image.svg"),
    ("ipynb", "icons/file_icons/jupyter.svg"),
    ("java", "icons/file_icons/java.svg"),
    ("javascript", "icons/file_icons/javascript.svg"),
    ("json", "icons/file_icons/code.svg"),
    ("julia", "icons/file_icons/julia.svg"),
    ("kdl", "icons/file_icons/kdl.svg"),
    ("kotlin", "icons/file_icons/kotlin.svg"),
    ("lock", "icons/file_icons/lock.svg"),
    ("log", "icons/file_icons/info.svg"),
    ("lua", "icons/file_icons/lua.svg"),
    ("luau", "icons/file_icons/luau.svg"),
    ("markdown", "icons/file_icons/book.svg"),
    ("metal", "icons/file_icons/metal.svg"),
    ("nim", "icons/file_icons/nim.svg"),
    ("nix", "icons/file_icons/nix.svg"),
    ("ocaml", "icons/file_icons/ocaml.svg"),
    ("odin", "icons/file_icons/odin.svg"),
    ("phoenix", "icons/file_icons/phoenix.svg"),
    ("php", "icons/file_icons/php.svg"),
    ("prettier", "icons/file_icons/prettier.svg"),
    ("prisma", "icons/file_icons/prisma.svg"),
    ("puppet", "icons/file_icons/puppet.svg"),
    ("python", "icons/file_icons/python.svg"),
    ("r", "icons/file_icons/r.svg"),
    ("react", "icons/file_icons/react.svg"),
    ("roc", "icons/file_icons/roc.svg"),
    ("ruby", "icons/file_icons/ruby.svg"),
    ("rust", "icons/file_icons/rust.svg"),
    ("sass", "icons/file_icons/sass.svg"),
    ("scala", "icons/file_icons/scala.svg"),
    ("settings", "icons/file_icons/settings.svg"),
    ("solidity", "icons/file_icons/file.svg"),
    ("storage", "icons/file_icons/database.svg"),
    ("stylelint", "icons/file_icons/javascript.svg"),
    ("surrealql", "icons/file_icons/surrealql.svg"),
    ("svelte", "icons/file_icons/html.svg"),
    ("swift", "icons/file_icons/swift.svg"),
    ("tcl", "icons/file_icons/tcl.svg"),
    ("template", "icons/file_icons/html.svg"),
    ("terminal", "icons/file_icons/terminal.svg"),
    ("terraform", "icons/file_icons/terraform.svg"),
    ("toml", "icons/file_icons/toml.svg"),
    ("typescript", "icons/file_icons/typescript.svg"),
    ("v", "icons/file_icons/v.svg"),
    ("vbproj", "icons/file_icons/file.svg"),
    ("vcs", "icons/file_icons/git.svg"),
    ("video", "icons/file_icons/video.svg"),
    ("vs_sln", "icons/file_icons/file.svg"),
    ("vs_suo", "icons/file_icons/file.svg"),
    ("vue", "icons/file_icons/vue.svg"),
    ("vyper", "icons/file_icons/vyper.svg"),
    ("wgsl", "icons/file_icons/wgsl.svg"),
    ("yaml", "icons/file_icons/yaml.svg"),
    ("zig", "icons/file_icons/zig.svg"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_icon_theme_json_round_trips() {
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
    fn every_icon_key_referenced_is_defined_or_falls_back() {
        let t = IconThemeContent::default();
        // Every stem/suffix key that has a `file_icons` entry resolves to a
        // `file_icons/*.svg` path; the few Zed leaves undefined (`backup`)
        // fall through to `default`.
        for key in t.file_stems.values().chain(t.file_suffixes.values()) {
            let path = t
                .file_icons
                .get(key)
                .map(|d| d.path.as_str())
                .unwrap_or_else(|| t.file_icons[&t.default_file].path.as_str());
            assert!(
                path.starts_with("icons/file_icons/") && path.ends_with(".svg"),
                "{key}"
            );
        }
    }

    #[test]
    fn resolves_zed_paths_for_representative_files() {
        let t = IconThemeContent::default();
        assert_eq!(t.file_icon_path("main.rs"), "icons/file_icons/rust.svg");
        assert_eq!(t.file_icon_path("App.tsx"), "icons/file_icons/react.svg");
        assert_eq!(t.file_icon_path("server.go"), "icons/file_icons/go.svg");
        assert_eq!(t.file_icon_path("Cargo.toml"), "icons/file_icons/toml.svg");
        assert_eq!(t.file_icon_path("styles.css"), "icons/file_icons/css.svg");
        assert_eq!(
            t.file_icon_path("Dockerfile"),
            "icons/file_icons/docker.svg"
        );
    }

    #[test]
    fn whole_name_beats_suffix_and_case_folds() {
        let t = IconThemeContent::default();
        // `.dockerignore` is a stem → docker, not the `default` fallback.
        assert_eq!(
            t.file_icon_path(".dockerignore"),
            "icons/file_icons/docker.svg"
        );
        assert_eq!(
            t.file_icon_path(".DOCKERIGNORE"),
            "icons/file_icons/docker.svg"
        );
        // `.gitignore` → `gitignore` suffix → `vcs`.
        assert_eq!(t.file_icon_path(".gitignore"), "icons/file_icons/git.svg");
    }

    #[test]
    fn longest_suffix_wins() {
        let t = IconThemeContent::default();
        // `docker-compose.yml` is a whole-name suffix entry → docker, beating
        // the trailing `yml` → yaml.
        assert_eq!(
            t.file_icon_path("docker-compose.yml"),
            "icons/file_icons/docker.svg"
        );
        assert_eq!(t.file_icon_path("app.yml"), "icons/file_icons/yaml.svg");
    }

    #[test]
    fn unknown_falls_back_to_default_file_icon() {
        let t = IconThemeContent::default();
        assert_eq!(t.file_icon_path("mystery.qqq"), "icons/file_icons/file.svg");
        assert_eq!(t.file_icon_path("noextension"), "icons/file_icons/file.svg");
        assert_eq!(
            t.directory_icon_path("src", false),
            "icons/file_icons/folder.svg"
        );
        assert_eq!(
            t.directory_icon_path("src", true),
            "icons/file_icons/folder_open.svg"
        );
        assert_eq!(
            t.chevron_icon_path(false),
            "icons/file_icons/chevron_right.svg"
        );
        assert_eq!(
            t.chevron_icon_path(true),
            "icons/file_icons/chevron_down.svg"
        );
    }

    #[test]
    fn named_directory_icons_win_when_present() {
        let mut t = IconThemeContent::default();
        t.named_directory_icons.insert(
            ".github".to_string(),
            DirectoryIcons {
                collapsed: "icons/file_icons/github.svg".to_string(),
                expanded: "icons/file_icons/github.svg".to_string(),
            },
        );
        assert_eq!(
            t.directory_icon_path(".github", false),
            "icons/file_icons/github.svg"
        );
        assert_eq!(
            t.directory_icon_path("src", false),
            "icons/file_icons/folder.svg"
        );
    }

    #[test]
    fn registry_loads_user_themes_and_skips_broken() {
        let dir = std::env::temp_dir().join(format!("labonair-icon-themes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mono.json"),
            r#"{ "name": "Mono", "file_suffixes": { "rs": "binary" },
                "file_icons": { "binary": { "path": "icons/file_icons/file.svg" } },
                "directory": { "collapsed": "icons/file_icons/folder.svg", "expanded": "icons/file_icons/folder_open.svg" },
                "chevron": { "collapsed": "icons/file_icons/chevron_right.svg", "expanded": "icons/file_icons/chevron_down.svg" },
                "default_file": "default" }"#,
        )
        .unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();

        let mut reg = IconThemeRegistry::builtin();
        let warnings = reg.load_user_icon_themes(&dir);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("broken.json"));
        assert!(reg.contains("mono"));
        assert!(reg.contains("default"));
        assert_eq!(
            reg.get("mono").unwrap().file_icon_path("main.rs"),
            "icons/file_icons/file.svg"
        );

        std::fs::remove_file(dir.join("mono.json")).unwrap();
        reg.load_user_icon_themes(&dir);
        assert!(!reg.contains("mono"));
        assert!(reg.contains("default"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn registry_loads_a_family_file() {
        let dir = std::env::temp_dir().join(format!("labonair-icon-fam-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("pack.json"),
            r#"{ "name": "Pack", "author": "x", "themes": [
                { "name": "Pack Dark", "directory": { "collapsed": "icons/file_icons/folder.svg", "expanded": "icons/file_icons/folder_open.svg" },
                  "chevron": { "collapsed": "icons/file_icons/chevron_right.svg", "expanded": "icons/file_icons/chevron_down.svg" },
                  "default_file": "default" },
                { "name": "Pack Light", "directory": { "collapsed": "icons/file_icons/folder.svg", "expanded": "icons/file_icons/folder_open.svg" },
                  "chevron": { "collapsed": "icons/file_icons/chevron_right.svg", "expanded": "icons/file_icons/chevron_down.svg" },
                  "default_file": "default" }
            ] }"#,
        )
        .unwrap();
        let mut reg = IconThemeRegistry::builtin();
        reg.load_user_icon_themes(&dir);
        assert!(reg.contains("pack/Pack Dark"));
        assert!(reg.contains("pack/Pack Light"));
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
