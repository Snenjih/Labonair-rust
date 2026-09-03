//! Per-user AI model preferences — favourites + recently-used list for the
//! composer's ModelPicker (`Favorites` / `Recent` tabs).
//!
//! Persisted as `labonair-model-prefs.json` in the config dir.

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

const FILE: &str = "labonair-model-prefs.json";
const RECENT_CAP: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPrefs {
    /// Model ids the user starred (insertion order).
    #[serde(default)]
    pub favorites: Vec<String>,
    /// Recently-selected model ids, most-recent first, capped.
    #[serde(default)]
    pub recent: Vec<String>,
}

impl ModelPrefs {
    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|f| f == id)
    }

    /// Star / unstar `id`. Returns the new starred state.
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if let Some(pos) = self.favorites.iter().position(|f| f == id) {
            self.favorites.remove(pos);
            false
        } else {
            self.favorites.push(id.to_string());
            true
        }
    }

    /// Record `id` as just-used — moves it to the front, dedupes, caps the list.
    pub fn push_recent(&mut self, id: &str) {
        self.recent.retain(|r| r != id);
        self.recent.insert(0, id.to_string());
        self.recent.truncate(RECENT_CAP);
    }
}

fn path() -> std::path::PathBuf {
    config_dir().join(FILE)
}

fn load_from(p: &std::path::Path) -> ModelPrefs {
    std::fs::read_to_string(p)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_to(p: &std::path::Path, prefs: &ModelPrefs) -> Result<(), String> {
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(p, json).map_err(|e| e.to_string())
}

/// Load the model preferences from the config dir.
pub fn load() -> ModelPrefs {
    load_from(&path())
}

/// Persist the model preferences.
pub fn save(prefs: &ModelPrefs) -> Result<(), String> {
    save_to(&path(), prefs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_favorite_round_trips() {
        let mut p = ModelPrefs::default();
        assert!(p.toggle_favorite("gpt-5.5"));
        assert!(p.is_favorite("gpt-5.5"));
        assert!(!p.toggle_favorite("gpt-5.5"));
        assert!(!p.is_favorite("gpt-5.5"));
    }

    #[test]
    fn push_recent_dedupes_and_caps() {
        let mut p = ModelPrefs::default();
        for i in 0..12 {
            p.push_recent(&format!("m{i}"));
        }
        assert_eq!(p.recent.len(), RECENT_CAP);
        assert_eq!(p.recent[0], "m11");
        p.push_recent("m8");
        assert_eq!(p.recent[0], "m8");
        assert_eq!(p.recent.iter().filter(|r| *r == "m8").count(), 1);
    }

    #[test]
    fn persist_round_trip() {
        let dir = std::env::temp_dir().join(format!("mp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("mp.json");
        let mut prefs = ModelPrefs::default();
        prefs.toggle_favorite("a");
        prefs.push_recent("b");
        save_to(&p, &prefs).unwrap();
        assert_eq!(load_from(&p), prefs);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
