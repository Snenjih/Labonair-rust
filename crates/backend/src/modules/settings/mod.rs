use serde_json::{Map, Value};
use std::path::Path;
use tokio::sync::Mutex;

pub mod editor;
pub mod mcp;
pub mod preferences;

use crate::modules::fs::paths::config_dir;

const SETTINGS_FILE: &str = "labonair-settings.json";
const KEY_BAR_ITEM_PLACEMENTS: &str = "barItemPlacements";

/// Serializes every `settings_set_bar_item_placement` call across all
/// windows (they share one Rust process) so the read-merge-write of the
/// `barItemPlacements` blob can never interleave.
#[derive(Default)]
pub struct BarItemPlacementLock(pub Mutex<()>);

fn read_settings_from(dir: &Path) -> Map<String, Value> {
    std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_settings_to(dir: &Path, map: &Map<String, Value>) -> Result<(), String> {
    let path = dir.join(SETTINGS_FILE);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        set_bar_item_placement_in(&dir, "bookmarks".into(), json!({ "side": "left" })).unwrap();
        let loaded = bar_item_placements_load_from(&dir);
        assert!(loaded.contains_key("updater"));
        assert_eq!(
            loaded
                .get("bookmarks")
                .unwrap()
                .as_object()
                .unwrap()
                .get("side")
                .unwrap(),
            "left"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
