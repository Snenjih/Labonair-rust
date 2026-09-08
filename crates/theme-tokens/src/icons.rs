//! Icon-theme data types + the built-in file/folder glyph tables.
//!
//! Moved out of `labonair-theme::icon_theme` in R08-008 so `labonair-ui-kit`
//! can resolve file/folder icons without depending on the Themes feature
//! crate. The runtime registry (`IconThemeRegistry`, user-theme loading)
//! stays in `labonair-theme`.

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

// ─────────────────────────── canonical built-in tables ──────────────────────
//
// A 1:1 transcription of Zed's `"Zed (Default)"` icon theme. These three tables
// are the single source of truth for the embedded
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
