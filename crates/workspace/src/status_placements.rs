//! The `statusBarItemPlacements` blob — T18-005.
//!
//! Replaces the old, titlebar-and-statusbar-spanning `barItemPlacements` /
//! `BarItemId` / `BarLoc` mechanism (a fixed 15-variant enum from before the
//! T17 layout rework). That system had no live consumer: the titlebar
//! (T18-001) and the statusbar (T17-003+) both render from their own
//! registries now, keyed by an arbitrary `&'static str` id rather than a
//! closed enum, and the titlebar has no "moveable item" concept at all. This
//! module is its statusbar-only replacement: it converts between the
//! persisted JSON blob (`{ itemId: { side, hidden } }`) and
//! [`labonair_panel::StatusPlacement`], for whatever ids the running
//! [`labonair_panel::StatusItemRegistry`] happens to have registered.
//!
//! This module owns both the JSON conversion and the atomic persistence of the
//! live status-bar layout. It is deliberately separate from value Settings:
//! placement and panel-toggle state is workspace chrome state.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use gpui::Global;
use serde_json::{json, Map, Value};

use labonair_panel::{StatusPlacement, StatusSide};

/// Bumped whenever any window persists a status-bar placement change, so
/// every other window's [`crate::status_bar::StatusBar`] re-reads the blob
/// from disk (`cx.observe_global`) and stays in sync (T18-005 point 8, "two
/// windows").
#[derive(Default)]
pub struct StatusBarLayoutTick(pub u64);

impl Global for StatusBarLayoutTick {}

fn side_as_str(side: StatusSide) -> &'static str {
    match side {
        StatusSide::Left => "left",
        StatusSide::Right => "right",
    }
}

fn side_from_str(s: &str) -> Option<StatusSide> {
    match s {
        "left" => Some(StatusSide::Left),
        "right" => Some(StatusSide::Right),
        _ => None,
    }
}

/// Parse the persisted `statusBarItemPlacements` blob into an override table
/// keyed by item id. Entries for ids nobody registered are kept (harmless —
/// [`labonair_panel::StatusItemRegistry::resolve_side`] simply never looks
/// them up) so a placement set while a plugin/panel was temporarily absent
/// isn't lost.
pub fn overrides_from_blob(blob: &Map<String, Value>) -> HashMap<String, StatusPlacement> {
    let mut out = HashMap::new();
    for (id, raw) in blob {
        let side = raw
            .get("side")
            .and_then(Value::as_str)
            .and_then(side_from_str)
            .unwrap_or(StatusSide::Right);
        let hidden = raw.get("hidden").and_then(Value::as_bool).unwrap_or(false);
        out.insert(id.clone(), StatusPlacement { side, hidden });
    }
    out
}

/// JSON patch for one status-bar placement update — only the keys that
/// changed, so an unrelated concurrent write to the same item's other field
/// is never clobbered.
pub fn placement_patch(side: Option<StatusSide>, hidden: Option<bool>) -> Value {
    let mut patch = json!({});
    let obj = patch.as_object_mut().unwrap();
    if let Some(s) = side {
        obj.insert(
            "side".to_string(),
            Value::String(side_as_str(s).to_string()),
        );
    }
    if let Some(h) = hidden {
        obj.insert("hidden".to_string(), Value::Bool(h));
    }
    patch
}

const CONFIG_FILE: &str = "config.json";
const STATUS_BAR_PLACEMENTS_KEY: &str = "statusBarItemPlacements";
const PANEL_TOGGLE_VISIBILITY_KEY: &str = "panelToggleVisibility";

static WRITE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn write_lock() -> &'static tokio::sync::Mutex<()> {
    WRITE_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

fn read_config_from(dir: &Path) -> Map<String, Value> {
    std::fs::read_to_string(dir.join(CONFIG_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn write_config_to(dir: &Path, settings: &Map<String, Value>) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let path = dir.join(CONFIG_FILE);
    let temp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(&temp, json).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, &path).map_err(|error| error.to_string())
}

/// Load current status-bar placement overrides from the workspace config.
pub fn status_bar_item_placements_load() -> Map<String, Value> {
    status_bar_item_placements_load_from(&labonair_filesystem::paths::config_dir())
}

fn status_bar_item_placements_load_from(dir: &Path) -> Map<String, Value> {
    read_config_from(dir)
        .get(STATUS_BAR_PLACEMENTS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Atomically merge one status-bar placement patch into the shared config.
pub async fn set_status_bar_item_placement(item_id: String, patch: Value) -> Result<(), String> {
    let _guard = write_lock().lock().await;
    set_status_bar_item_placement_in(&labonair_filesystem::paths::config_dir(), item_id, patch)
}

fn set_status_bar_item_placement_in(
    dir: &Path,
    item_id: String,
    patch: Value,
) -> Result<(), String> {
    let mut settings = read_config_from(dir);
    let mut placements = settings
        .get(STATUS_BAR_PLACEMENTS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut entry = placements
        .get(&item_id)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(patch) = patch.as_object() {
        for (key, value) in patch {
            entry.insert(key.clone(), value.clone());
        }
    }
    placements.insert(item_id, Value::Object(entry));
    settings.insert(
        STATUS_BAR_PLACEMENTS_KEY.to_string(),
        Value::Object(placements),
    );
    write_config_to(dir, &settings)
}

/// Remove all live status-bar placement overrides.
pub async fn clear_status_bar_item_placements() -> Result<(), String> {
    let _guard = write_lock().lock().await;
    let dir = labonair_filesystem::paths::config_dir();
    let mut settings = read_config_from(&dir);
    settings.remove(STATUS_BAR_PLACEMENTS_KEY);
    write_config_to(&dir, &settings)
}

/// Load panel-toggle visibility overrides. Missing panels are visible.
pub fn panel_toggle_visibility_load() -> Map<String, Value> {
    panel_toggle_visibility_load_from(&labonair_filesystem::paths::config_dir())
}

fn panel_toggle_visibility_load_from(dir: &Path) -> Map<String, Value> {
    read_config_from(dir)
        .get(PANEL_TOGGLE_VISIBILITY_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// Atomically persist one panel-toggle visibility value.
pub async fn set_panel_toggle_visibility(name: String, visible: bool) -> Result<(), String> {
    let _guard = write_lock().lock().await;
    set_panel_toggle_visibility_in(&labonair_filesystem::paths::config_dir(), name, visible)
}

fn set_panel_toggle_visibility_in(dir: &Path, name: String, visible: bool) -> Result<(), String> {
    let mut settings = read_config_from(dir);
    let mut visibility = settings
        .get(PANEL_TOGGLE_VISIBILITY_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    visibility.insert(name, Value::Bool(visible));
    settings.insert(
        PANEL_TOGGLE_VISIBILITY_KEY.to_string(),
        Value::Object(visibility),
    );
    write_config_to(dir, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_config_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "labonair-status-placements-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn overrides_from_blob_parses_known_keys() {
        let mut blob = Map::new();
        blob.insert("cwd".to_string(), json!({ "side": "left", "hidden": true }));
        blob.insert("jump-hosts".to_string(), json!({ "side": "right" }));
        let overrides = overrides_from_blob(&blob);

        assert_eq!(
            overrides.get("cwd"),
            Some(&StatusPlacement {
                side: StatusSide::Left,
                hidden: true,
            })
        );
        // Missing `hidden` defaults to visible.
        assert_eq!(
            overrides.get("jump-hosts"),
            Some(&StatusPlacement {
                side: StatusSide::Right,
                hidden: false,
            })
        );
        assert!(!overrides.contains_key("missing"));
    }

    #[test]
    fn overrides_from_blob_ignores_garbage_side() {
        let mut blob = Map::new();
        blob.insert("weird".to_string(), json!({ "side": "up" }));
        let overrides = overrides_from_blob(&blob);
        // Unparseable side falls back to the right cluster rather than
        // panicking or dropping the entry.
        assert_eq!(
            overrides.get("weird"),
            Some(&StatusPlacement {
                side: StatusSide::Right,
                hidden: false,
            })
        );
    }

    #[test]
    fn placement_patch_only_includes_set_fields() {
        let p = placement_patch(Some(StatusSide::Left), None);
        assert_eq!(p, json!({ "side": "left" }));
        let p = placement_patch(None, Some(true));
        assert_eq!(p, json!({ "hidden": true }));
        let p = placement_patch(Some(StatusSide::Right), Some(false));
        assert_eq!(p, json!({ "side": "right", "hidden": false }));
    }

    #[test]
    fn status_placement_persistence_merges_partial_updates() {
        let dir = temp_config_dir("status");

        set_status_bar_item_placement_in(
            &dir,
            "cwd".to_string(),
            json!({ "side": "left", "hidden": true }),
        )
        .unwrap();
        set_status_bar_item_placement_in(&dir, "cwd".to_string(), json!({ "hidden": false }))
            .unwrap();

        let loaded = status_bar_item_placements_load_from(&dir);
        assert_eq!(
            loaded.get("cwd"),
            Some(&json!({ "side": "left", "hidden": false }))
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn panel_visibility_persistence_preserves_other_panels() {
        let dir = temp_config_dir("panels");

        set_panel_toggle_visibility_in(&dir, "snippets".to_string(), false).unwrap();
        set_panel_toggle_visibility_in(&dir, "ai".to_string(), true).unwrap();

        let loaded = panel_toggle_visibility_load_from(&dir);
        assert_eq!(loaded.get("snippets"), Some(&json!(false)));
        assert_eq!(loaded.get("ai"), Some(&json!(true)));

        let _ = std::fs::remove_dir_all(dir);
    }
}
