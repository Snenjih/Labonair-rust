//! Recent Projects — a small persisted MRU list of project folders opened
//! through Spaces (`Workspace::create_space_from_project` / "Open Project…"),
//! so the Spaces menu can offer instant reopen without re-browsing the
//! filesystem.
//!
//! Deliberately independent of [`crate::spaces::SpaceStore`]: closing a Space
//! removes it from the live workspace, but the project it was rooted at
//! should still be one click away from "Recent Projects" — the two lists
//! answer different questions ("what's open right now" vs. "what did I open
//! recently").

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Oldest entries are dropped past this count.
pub const MAX_RECENT_PROJECTS: usize = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub root: PathBuf,
    pub name: String,
    /// Unix seconds this project was last opened (diagnostics / future
    /// sorting only — the list itself is already stored MRU-first).
    pub last_opened: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecentProjectsList(Vec<RecentProject>);

impl RecentProjectsList {
    pub fn entries(&self) -> &[RecentProject] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Record `root` as just-opened: moves it to the front if already
    /// present (refreshing its name/timestamp), otherwise inserts it at the
    /// front. Capped at [`MAX_RECENT_PROJECTS`].
    pub fn record(&mut self, root: PathBuf) {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let last_opened = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.0.retain(|e| e.root != root);
        self.0.insert(
            0,
            RecentProject {
                root,
                name,
                last_opened,
            },
        );
        self.0.truncate(MAX_RECENT_PROJECTS);
    }

    /// Drop an entry — offered from the UI for projects that no longer exist
    /// or the user just wants to forget.
    pub fn remove(&mut self, root: &Path) {
        self.0.retain(|e| e.root != root);
    }
}

// ─────────────────────────────── storage ─────────────────────────────────

fn store_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("labonair")
        .join("recent_projects.json")
}

/// The persisted list, or an empty one if there is none / it is unreadable.
pub fn load() -> RecentProjectsList {
    load_from(&store_path())
}

/// Persist `list` (best-effort; failures are logged, never propagated).
pub fn save(list: &RecentProjectsList) {
    save_to(&store_path(), list);
}

pub(crate) fn load_from(path: &Path) -> RecentProjectsList {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub(crate) fn save_to(path: &Path, list: &RecentProjectsList) {
    if let Some(dir) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(dir) {
            tracing::warn!(%err, "failed to create recent-projects state dir");
            return;
        }
    }
    match serde_json::to_string_pretty(list) {
        Ok(raw) => {
            if let Err(err) = std::fs::write(path, raw) {
                tracing::warn!(%err, "failed to write recent-projects list");
            }
        }
        Err(err) => tracing::warn!(%err, "failed to serialize recent-projects list"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_inserts_at_front() {
        let mut list = RecentProjectsList::default();
        list.record(PathBuf::from("/a"));
        list.record(PathBuf::from("/b"));
        assert_eq!(list.entries()[0].root, PathBuf::from("/b"));
        assert_eq!(list.entries()[1].root, PathBuf::from("/a"));
    }

    #[test]
    fn record_moves_existing_entry_to_front_without_duplicating() {
        let mut list = RecentProjectsList::default();
        list.record(PathBuf::from("/a"));
        list.record(PathBuf::from("/b"));
        list.record(PathBuf::from("/a"));
        assert_eq!(list.entries().len(), 2);
        assert_eq!(list.entries()[0].root, PathBuf::from("/a"));
    }

    #[test]
    fn record_caps_at_max_recent_projects() {
        let mut list = RecentProjectsList::default();
        for i in 0..(MAX_RECENT_PROJECTS + 5) {
            list.record(PathBuf::from(format!("/p{i}")));
        }
        assert_eq!(list.entries().len(), MAX_RECENT_PROJECTS);
        assert_eq!(
            list.entries()[0].root,
            PathBuf::from(format!("/p{}", MAX_RECENT_PROJECTS + 4))
        );
    }

    #[test]
    fn remove_drops_the_entry() {
        let mut list = RecentProjectsList::default();
        list.record(PathBuf::from("/a"));
        list.remove(Path::new("/a"));
        assert!(list.is_empty());
    }

    #[test]
    fn derives_name_from_folder() {
        let mut list = RecentProjectsList::default();
        list.record(PathBuf::from("/Users/x/Developer/Labonair-rust"));
        assert_eq!(list.entries()[0].name, "Labonair-rust");
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = std::env::temp_dir().join(format!(
            "labonair-recent-projects-test-{}",
            std::process::id()
        ));
        let path = dir.join("recent_projects.json");
        let mut list = RecentProjectsList::default();
        list.record(PathBuf::from("/a"));
        save_to(&path, &list);
        let loaded = load_from(&path);
        assert_eq!(loaded, list);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_missing_file_is_empty() {
        let loaded = load_from(Path::new("/definitely/does/not/exist.json"));
        assert!(loaded.is_empty());
    }
}
