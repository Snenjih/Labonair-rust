use serde_json::{Map, Value};
use std::path::Path;
use tokio::sync::Mutex;

pub mod content_bridge;
pub mod editor;
pub mod mcp;
pub mod migrate_v2;
pub mod migrations;
pub mod preferences;

use crate::modules::fs::paths::config_dir;

/// The shared user configuration file used by every native settings writer.
pub const CONFIG_FILE: &str = "config.json";
const LEGACY_CONFIG_FILE: &str = "labonair-settings.json";
const KEY_BAR_ITEM_PLACEMENTS: &str = "barItemPlacements";
const KEY_STATUS_BAR_ITEM_PLACEMENTS: &str = "statusBarItemPlacements";
const KEY_PANEL_TOGGLE_VISIBILITY: &str = "panelToggleVisibility";

/// Serializes every `settings_set_bar_item_placement` call across all
/// windows (they share one Rust process) so the read-merge-write of the
/// `barItemPlacements` blob can never interleave.
#[derive(Default)]
pub struct BarItemPlacementLock(pub Mutex<()>);

/// Serializes every `settings_set_status_bar_placement` call across all
/// windows, analogous to [`BarItemPlacementLock`] (T18-005).
#[derive(Default)]
pub struct StatusBarPlacementLock(pub Mutex<()>);

/// Serializes every `settings_set_panel_toggle_visibility` call across all
/// windows, analogous to [`StatusBarPlacementLock`] (T18-007).
#[derive(Default)]
pub struct PanelToggleVisibilityLock(pub Mutex<()>);

fn read_settings_from(dir: &Path) -> Map<String, Value> {
    std::fs::read_to_string(dir.join(CONFIG_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_settings_to(dir: &Path, map: &Map<String, Value>) -> Result<(), String> {
    let path = dir.join(CONFIG_FILE);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Adopt the former user-settings filename once, without overwriting a
/// `config.json` that already exists. This runs before any settings reader at
/// startup, so users keep their existing configuration after the rename.
pub fn migrate_config_file_name(dir: &Path) -> Result<(), String> {
    let legacy = dir.join(LEGACY_CONFIG_FILE);
    let current = dir.join(CONFIG_FILE);
    if current.exists() || !legacy.exists() {
        return Ok(());
    }
    std::fs::rename(&legacy, &current).map_err(|e| {
        format!(
            "failed to rename {} to {}: {e}",
            legacy.display(),
            current.display()
        )
    })
}

/// The persisted `barItemPlacements` blob (`{ itemId: { bar, side, hidden } }`),
/// or an empty map if nothing has been customised yet. The UI merges this over
/// its compiled-in defaults.
pub fn bar_item_placements_load() -> Map<String, Value> {
    bar_item_placements_load_from(&config_dir())
}

/// [`bar_item_placements_load`] against an explicit config directory (tests /
/// alternate profiles).
pub fn bar_item_placements_load_from(dir: &Path) -> Map<String, Value> {
    read_settings_from(dir)
        .get(KEY_BAR_ITEM_PLACEMENTS)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Atomically merges `patch` into `barItemPlacements[item_id]` and persists
/// it to the same `config_dir()` settings file the rest of the app reads.
pub async fn settings_set_bar_item_placement(
    lock: &BarItemPlacementLock,
    item_id: String,
    patch: Value,
) -> Result<(), String> {
    let _guard = lock.0.lock().await;
    set_bar_item_placement_in(&config_dir(), item_id, patch)
}

/// Synchronous read-merge-write core of [`settings_set_bar_item_placement`],
/// parameterised on the config directory so it is unit-testable. Callers that
/// aren't already serialised by [`BarItemPlacementLock`] must not use this.
pub fn set_bar_item_placement_in(dir: &Path, item_id: String, patch: Value) -> Result<(), String> {
    let mut settings = read_settings_from(dir);

    let mut placements = settings
        .get(KEY_BAR_ITEM_PLACEMENTS)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let mut entry = placements
        .get(&item_id)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    if let Some(patch_obj) = patch.as_object() {
        for (k, v) in patch_obj {
            entry.insert(k.clone(), v.clone());
        }
    }
    entry.insert("itemId".to_string(), Value::String(item_id.clone()));

    placements.insert(item_id, Value::Object(entry));
    settings.insert(
        KEY_BAR_ITEM_PLACEMENTS.to_string(),
        Value::Object(placements),
    );

    write_settings_to(dir, &settings)
}

/// The persisted `statusBarItemPlacements` blob (`{ itemId: { side, hidden } }`),
/// or an empty map if nothing has been customised yet (T18-005).
pub fn status_bar_item_placements_load() -> Map<String, Value> {
    status_bar_item_placements_load_from(&config_dir())
}

/// [`status_bar_item_placements_load`] against an explicit config directory
/// (tests / alternate profiles).
pub fn status_bar_item_placements_load_from(dir: &Path) -> Map<String, Value> {
    read_settings_from(dir)
        .get(KEY_STATUS_BAR_ITEM_PLACEMENTS)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Atomically merges `patch` into `statusBarItemPlacements[item_id]` and
/// persists it to the same `config_dir()` settings file the rest of the app
/// reads.
pub async fn settings_set_status_bar_placement(
    lock: &StatusBarPlacementLock,
    item_id: String,
    patch: Value,
) -> Result<(), String> {
    let _guard = lock.0.lock().await;
    set_status_bar_placement_in(&config_dir(), item_id, patch)
}

/// Synchronous read-merge-write core of [`settings_set_status_bar_placement`],
/// parameterised on the config directory so it is unit-testable. Callers that
/// aren't already serialised by [`StatusBarPlacementLock`] must not use this.
pub fn set_status_bar_placement_in(
    dir: &Path,
    item_id: String,
    patch: Value,
) -> Result<(), String> {
    let mut settings = read_settings_from(dir);

    let mut placements = settings
        .get(KEY_STATUS_BAR_ITEM_PLACEMENTS)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    let mut entry = placements
        .get(&item_id)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();

    if let Some(patch_obj) = patch.as_object() {
        for (k, v) in patch_obj {
            entry.insert(k.clone(), v.clone());
        }
    }

    placements.insert(item_id, Value::Object(entry));
    settings.insert(
        KEY_STATUS_BAR_ITEM_PLACEMENTS.to_string(),
        Value::Object(placements),
    );

    write_settings_to(dir, &settings)
}

/// Deletes the whole `statusBarItemPlacements` blob (T18-007's Personalization
/// pane "Reset to default" action). Every item falls back to its compiled-in
/// `default_side`, unhidden.
pub async fn settings_clear_status_bar_placements(
    lock: &StatusBarPlacementLock,
) -> Result<(), String> {
    let _guard = lock.0.lock().await;
    clear_status_bar_placements_in(&config_dir())
}

/// Synchronous core of [`settings_clear_status_bar_placements`], parameterised
/// on the config directory so it is unit-testable.
pub fn clear_status_bar_placements_in(dir: &Path) -> Result<(), String> {
    let mut settings = read_settings_from(dir);
    settings.remove(KEY_STATUS_BAR_ITEM_PLACEMENTS);
    write_settings_to(dir, &settings)
}

/// The persisted `panelToggleVisibility` blob (`{ panelName: bool }`, T18-007),
/// or an empty map if nothing has been customised yet. A panel absent from the
/// map is visible in the status bar's panel-toggle cluster by default.
pub fn panel_toggle_visibility_load() -> Map<String, Value> {
    panel_toggle_visibility_load_from(&config_dir())
}

/// [`panel_toggle_visibility_load`] against an explicit config directory
/// (tests / alternate profiles).
pub fn panel_toggle_visibility_load_from(dir: &Path) -> Map<String, Value> {
    read_settings_from(dir)
        .get(KEY_PANEL_TOGGLE_VISIBILITY)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Sets `panelToggleVisibility[panel_name]` and persists it to the same
/// `config_dir()` settings file the rest of the app reads. This is the single
/// write path for panel-toggle visibility — both the status bar's own
/// "Hide from toggle bar" action and the Personalization settings pane call
/// it (`labonair_workspace::Workspace::set_panel_toggle_visible`).
pub async fn settings_set_panel_toggle_visibility(
    lock: &PanelToggleVisibilityLock,
    panel_name: String,
    visible: bool,
) -> Result<(), String> {
    let _guard = lock.0.lock().await;
    set_panel_toggle_visibility_in(&config_dir(), panel_name, visible)
}

/// Synchronous core of [`settings_set_panel_toggle_visibility`], parameterised
/// on the config directory so it is unit-testable. Callers that aren't already
/// serialised by [`PanelToggleVisibilityLock`] must not use this.
pub fn set_panel_toggle_visibility_in(
    dir: &Path,
    panel_name: String,
    visible: bool,
) -> Result<(), String> {
    let mut settings = read_settings_from(dir);

    let mut visibility = settings
        .get(KEY_PANEL_TOGGLE_VISIBILITY)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    visibility.insert(panel_name, Value::Bool(visible));

    settings.insert(
        KEY_PANEL_TOGGLE_VISIBILITY.to_string(),
        Value::Object(visibility),
    );
    write_settings_to(dir, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adopts_legacy_filename_only_when_config_is_missing() {
        let dir =
            std::env::temp_dir().join(format!("labonair-config-filename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let legacy = dir.join(LEGACY_CONFIG_FILE);
        let current = dir.join(CONFIG_FILE);
        std::fs::write(&legacy, r#"{"general":{"theme":"dark"}}"#).unwrap();

        migrate_config_file_name(&dir).unwrap();
        assert!(!legacy.exists());
        assert_eq!(
            std::fs::read_to_string(&current).unwrap(),
            r#"{"general":{"theme":"dark"}}"#
        );

        std::fs::write(&legacy, r#"{"general":{"theme":"light"}}"#).unwrap();
        migrate_config_file_name(&dir).unwrap();
        assert!(legacy.exists());
        assert_eq!(
            std::fs::read_to_string(&current).unwrap(),
            r#"{"general":{"theme":"dark"}}"#
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bar_item_placement_round_trips_and_merges() {
        let dir = std::env::temp_dir().join(format!("labonair-bar-items-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Nothing persisted yet.
        assert!(bar_item_placements_load_from(&dir).is_empty());

        // First patch: move to statusbar/left.
        set_bar_item_placement_in(
            &dir,
            "updater".into(),
            json!({ "bar": "statusbar", "side": "left", "hidden": false }),
        )
        .unwrap();

        let loaded = bar_item_placements_load_from(&dir);
        let updater = loaded.get("updater").unwrap().as_object().unwrap();
        assert_eq!(updater.get("bar").unwrap(), "statusbar");
        assert_eq!(updater.get("side").unwrap(), "left");
        assert_eq!(updater.get("itemId").unwrap(), "updater");

        // Partial patch keeps the untouched keys (merge, not replace).
        set_bar_item_placement_in(&dir, "updater".into(), json!({ "hidden": true })).unwrap();
        let loaded = bar_item_placements_load_from(&dir);
        let updater = loaded.get("updater").unwrap().as_object().unwrap();
        assert_eq!(updater.get("bar").unwrap(), "statusbar");
        assert_eq!(updater.get("hidden").unwrap(), &json!(true));

        // A second item does not disturb the first.
        set_bar_item_placement_in(&dir, "jump-hosts".into(), json!({ "side": "left" })).unwrap();
        let loaded = bar_item_placements_load_from(&dir);
        assert!(loaded.contains_key("updater"));
        assert_eq!(
            loaded
                .get("jump-hosts")
                .unwrap()
                .as_object()
                .unwrap()
                .get("side")
                .unwrap(),
            "left"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_bar_placement_round_trips_and_merges() {
        let dir =
            std::env::temp_dir().join(format!("labonair-status-bar-items-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Nothing persisted yet.
        assert!(status_bar_item_placements_load_from(&dir).is_empty());

        // First patch: move to left, hidden.
        set_status_bar_placement_in(
            &dir,
            "cwd".into(),
            json!({ "side": "left", "hidden": true }),
        )
        .unwrap();

        let loaded = status_bar_item_placements_load_from(&dir);
        let cwd = loaded.get("cwd").unwrap().as_object().unwrap();
        assert_eq!(cwd.get("side").unwrap(), "left");
        assert_eq!(cwd.get("hidden").unwrap(), &json!(true));

        // Partial patch keeps the untouched keys (merge, not replace).
        set_status_bar_placement_in(&dir, "cwd".into(), json!({ "hidden": false })).unwrap();
        let loaded = status_bar_item_placements_load_from(&dir);
        let cwd = loaded.get("cwd").unwrap().as_object().unwrap();
        assert_eq!(cwd.get("side").unwrap(), "left");
        assert_eq!(cwd.get("hidden").unwrap(), &json!(false));

        // A second item does not disturb the first.
        set_status_bar_placement_in(&dir, "jump-hosts".into(), json!({ "side": "right" })).unwrap();
        let loaded = status_bar_item_placements_load_from(&dir);
        assert!(loaded.contains_key("cwd"));
        assert_eq!(
            loaded
                .get("jump-hosts")
                .unwrap()
                .as_object()
                .unwrap()
                .get("side")
                .unwrap(),
            "right"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_status_bar_placements_removes_the_whole_blob() {
        let dir =
            std::env::temp_dir().join(format!("labonair-clear-status-bar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        set_status_bar_placement_in(&dir, "cwd".into(), json!({ "side": "left" })).unwrap();
        assert!(!status_bar_item_placements_load_from(&dir).is_empty());

        clear_status_bar_placements_in(&dir).unwrap();
        assert!(status_bar_item_placements_load_from(&dir).is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn panel_toggle_visibility_round_trips_and_merges() {
        let dir = std::env::temp_dir().join(format!(
            "labonair-panel-toggle-visibility-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Nothing persisted yet — every panel defaults to visible.
        assert!(panel_toggle_visibility_load_from(&dir).is_empty());

        set_panel_toggle_visibility_in(&dir, "snippets".into(), false).unwrap();
        let loaded = panel_toggle_visibility_load_from(&dir);
        assert_eq!(loaded.get("snippets").unwrap(), &json!(false));

        // A second panel does not disturb the first.
        set_panel_toggle_visibility_in(&dir, "ai".into(), true).unwrap();
        let loaded = panel_toggle_visibility_load_from(&dir);
        assert_eq!(loaded.get("snippets").unwrap(), &json!(false));
        assert_eq!(loaded.get("ai").unwrap(), &json!(true));

        // Re-showing a hidden panel overwrites its entry.
        set_panel_toggle_visibility_in(&dir, "snippets".into(), true).unwrap();
        let loaded = panel_toggle_visibility_load_from(&dir);
        assert_eq!(loaded.get("snippets").unwrap(), &json!(true));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
