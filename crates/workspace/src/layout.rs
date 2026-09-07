//! Workspace-owned persistence for dock and sidebar layout.
//!
//! Layout is runtime/session state, not a user preference. It therefore lives
//! beside the workspace and is persisted independently from `SettingsContent`.
//! The legacy `workspace.sidebar*`/`dockLayout` values are imported once and
//! then removed from `config.json`.

use std::path::{Path, PathBuf};

use labonair_panel::DockPosition;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::dock::DockData;

const LAYOUT_FILE: &str = "workspace-layout.json";
const CONFIG_FILE: &str = "config.json";
const LEGACY_KEYS: &[&str] = &[
    "dockLayout",
    "sidebarPosition",
    "sidebarOpen",
    "sidebarActivePanel",
    "sidebarRightOpen",
    "sidebarRightActivePanel",
    "sidebarWidth",
    "sidebarRightWidth",
];

/// Persisted workspace chrome state. Tab/pane contents remain in
/// [`crate::session::SessionSnapshot`]; this file only owns the permanent dock
/// arrangement and the primary dock preference.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayoutSnapshot {
    pub version: u32,
    pub primary_position: String,
    pub docks: Vec<DockData>,
}

pub const CURRENT_VERSION: u32 = 1;

impl WorkspaceLayoutSnapshot {
    pub fn new(primary_position: DockPosition, docks: Vec<DockData>) -> Self {
        Self {
            version: CURRENT_VERSION,
            primary_position: crate::dock::position_slug(primary_position).to_string(),
            docks,
        }
    }

    pub fn primary_position(&self) -> DockPosition {
        crate::dock::position_from_slug(&self.primary_position).unwrap_or(DockPosition::Left)
    }

    pub fn docks_json(&self) -> String {
        serde_json::to_string(&self.docks).unwrap_or_default()
    }
}

fn layout_path() -> PathBuf {
    labonair_filesystem::paths::config_dir().join(LAYOUT_FILE)
}

/// Load the current workspace layout, falling back to an empty first-run
/// snapshot when the file is absent, invalid, or from an incompatible version.
pub fn load() -> WorkspaceLayoutSnapshot {
    load_from(&layout_path()).unwrap_or_default()
}

/// Persist the current layout. Layout persistence is best-effort because a
/// failed state write must not prevent the workspace from operating.
pub fn save(snapshot: &WorkspaceLayoutSnapshot) {
    if let Err(error) = save_to(&layout_path(), snapshot) {
        tracing::warn!(%error, "failed to persist workspace layout");
    }
}

fn load_from(path: &Path) -> Option<WorkspaceLayoutSnapshot> {
    let raw = std::fs::read_to_string(path).ok()?;
    let snapshot: WorkspaceLayoutSnapshot = serde_json::from_str(&raw).ok()?;
    (snapshot.version == CURRENT_VERSION).then_some(snapshot)
}

fn save_to(path: &Path, snapshot: &WorkspaceLayoutSnapshot) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let raw = serde_json::to_string_pretty(snapshot).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, raw).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}

/// Import legacy layout fields from `config.json` once. Existing top-level
/// layout state always wins; this function never overwrites a user's new file.
pub fn migrate_legacy_settings_file(config_dir: &Path) -> Result<bool, String> {
    let layout = config_dir.join(LAYOUT_FILE);
    if layout.exists() {
        return Ok(false);
    }

    let config = config_dir.join(CONFIG_FILE);
    let raw = match std::fs::read_to_string(&config) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    let mut document: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let Some(root) = document.as_object_mut() else {
        return Ok(false);
    };
    let Some(workspace) = root.get("workspace").and_then(Value::as_object) else {
        return Ok(false);
    };
    if !LEGACY_KEYS.iter().any(|key| workspace.contains_key(*key)) {
        return Ok(false);
    }

    let snapshot = snapshot_from_legacy(workspace);
    save_to(&layout, &snapshot)?;

    if let Some(workspace) = root.get_mut("workspace").and_then(Value::as_object_mut) {
        for key in LEGACY_KEYS {
            workspace.remove(*key);
        }
        if workspace.is_empty() {
            root.remove("workspace");
        }
    }
    let updated = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
    let temporary = config.with_extension("json.tmp");
    std::fs::write(&temporary, updated).map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, &config).map_err(|error| error.to_string())?;
    Ok(true)
}

fn snapshot_from_legacy(workspace: &Map<String, Value>) -> WorkspaceLayoutSnapshot {
    let primary = workspace
        .get("sidebarPosition")
        .and_then(Value::as_str)
        .filter(|position| *position == "right")
        .map_or(DockPosition::Left, |_| DockPosition::Right);

    if let Some(layout) = workspace.get("dockLayout").and_then(Value::as_str) {
        if let Ok(docks) = serde_json::from_str::<Vec<DockData>>(layout) {
            return WorkspaceLayoutSnapshot::new(primary, docks);
        }
    }

    WorkspaceLayoutSnapshot::new(
        primary,
        vec![
            DockData {
                position: "left".into(),
                open: bool_value(workspace, "sidebarOpen", true),
                size: number_value(workspace, "sidebarWidth", 260.0),
                zoomed: false,
                active_panel: string_value(workspace, "sidebarActivePanel")
                    .filter(|name| !matches!(name.as_str(), "hosts" | "tabs" | "ai")),
                panel_order: Vec::new(),
            },
            DockData {
                position: "right".into(),
                open: bool_value(workspace, "sidebarRightOpen", false),
                size: number_value(workspace, "sidebarRightWidth", 380.0),
                zoomed: false,
                active_panel: string_value(workspace, "sidebarRightActivePanel")
                    .filter(|name| !matches!(name.as_str(), "hosts" | "tabs" | "ai")),
                panel_order: Vec::new(),
            },
            DockData {
                position: "bottom".into(),
                open: false,
                size: 320.0,
                zoomed: false,
                active_panel: None,
                panel_order: Vec::new(),
            },
        ],
    )
}

fn bool_value(object: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    object.get(key).and_then(Value::as_bool).unwrap_or(fallback)
}

fn number_value(object: &Map<String, Value>, key: &str, fallback: f32) -> f32 {
    object
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .unwrap_or(fallback)
}

fn string_value(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("labonair-layout-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn layout_round_trips_with_primary_position() {
        let dir = temp_dir("roundtrip");
        let path = dir.join(LAYOUT_FILE);
        let snapshot = WorkspaceLayoutSnapshot::new(DockPosition::Right, vec![]);
        save_to(&path, &snapshot).unwrap();
        assert_eq!(load_from(&path), Some(snapshot));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_sidebar_values_migrate_and_are_removed() {
        let dir = temp_dir("legacy");
        let config = serde_json::json!({
            "general": {"theme": "dark"},
            "workspace": {
                "sidebarPosition": "right",
                "sidebarOpen": true,
                "sidebarWidth": 300,
                "sidebarActivePanel": "explorer"
            }
        });
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();

        assert!(migrate_legacy_settings_file(&dir).unwrap());
        let snapshot = load_from(&dir.join(LAYOUT_FILE)).unwrap();
        assert_eq!(snapshot.primary_position(), DockPosition::Right);
        assert_eq!(snapshot.docks[0].size, 300.0);
        let migrated: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(CONFIG_FILE)).unwrap()).unwrap();
        assert!(migrated.get("workspace").is_none());
        assert!(!migrate_legacy_settings_file(&dir).unwrap());
        let _ = std::fs::remove_dir_all(dir);
    }
}
