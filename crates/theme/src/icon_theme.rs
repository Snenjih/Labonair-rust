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

use serde::Deserialize;

pub use labonair_theme_tokens::{
    ChevronIcons, DirectoryIcons, IconDefinition, IconThemeContent, BUILTIN_ICON_THEME_ID,
    BUILTIN_ICON_THEME_NAME, DEFAULT_FILE_ICONS, DEFAULT_FILE_STEMS, DEFAULT_FILE_SUFFIXES,
};

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

/// The embedded built-in icon theme. Optional file loading remains available
/// as a future extension adapter, but the current product catalog is static.
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
    ///
    /// This is an extension adapter, not part of the current static catalog.
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
