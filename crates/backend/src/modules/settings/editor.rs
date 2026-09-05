//! Persisted editor / Vim preferences (T06-003).
//!
//! Stored as an `editor` object inside the shared `config.json`
//! (same file the rest of the app uses). Phase 12's settings UI will write
//! these; until then the editor view just reads them at construction and the
//! `:set` ex-command mutates the in-memory copy for the session.

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

use super::CONFIG_FILE;
const KEY: &str = "editor";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EditorPrefs {
    /// Master switch for the Vim keybinding layer.
    pub vim_mode: bool,
    pub number: bool,
    pub relative_number: bool,
    pub hlsearch: bool,
    pub incsearch: bool,
    pub smartcase: bool,
    pub expandtab: bool,
    pub tabstop: usize,
    pub shiftwidth: usize,
}

impl Default for EditorPrefs {
    fn default() -> Self {
        Self {
            vim_mode: false,
            number: true,
            relative_number: false,
            hlsearch: true,
            incsearch: true,
            smartcase: true,
            expandtab: true,
            tabstop: 4,
            shiftwidth: 4,
        }
    }
}

/// Load the persisted editor preferences (defaults if none saved yet).
pub fn editor_prefs_load() -> EditorPrefs {
    load_from(&config_dir())
}

/// Persist the editor preferences, merging into the shared settings file.
pub fn editor_prefs_save(prefs: &EditorPrefs) -> Result<(), String> {
    save_to(&config_dir(), prefs)
}

fn load_from(dir: &std::path::Path) -> EditorPrefs {
    std::fs::read_to_string(dir.join(CONFIG_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(KEY).cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn save_to(dir: &std::path::Path, prefs: &EditorPrefs) -> Result<(), String> {
    let path = dir.join(CONFIG_FILE);
    let mut map = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    map.insert(
        KEY.to_string(),
        serde_json::to_value(prefs).map_err(|e| e.to_string())?,
    );
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_merges_into_shared_file() {
        let dir = std::env::temp_dir().join(format!("labonair-ed-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(CONFIG_FILE), r#"{"other":1}"#).unwrap();

        let p = EditorPrefs {
            vim_mode: true,
            tabstop: 2,
            ..Default::default()
        };
        save_to(&dir, &p).unwrap();

        let back = load_from(&dir);
        assert!(back.vim_mode);
        assert_eq!(back.tabstop, 2);

        let raw = std::fs::read_to_string(dir.join(CONFIG_FILE)).unwrap();
        assert!(raw.contains("\"other\""));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("labonair-ed-{}", uuid::Uuid::new_v4()));
        assert_eq!(load_from(&dir), EditorPrefs::default());
    }
}
