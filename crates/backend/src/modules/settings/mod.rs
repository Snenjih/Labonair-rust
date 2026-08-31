use serde_json::{Map, Value};
use tokio::sync::Mutex;

use crate::modules::fs::paths::config_dir;

const SETTINGS_FILE: &str = "labonair-settings.json";
const KEY_BAR_ITEM_PLACEMENTS: &str = "barItemPlacements";

/// Serializes every `settings_set_bar_item_placement` call across all
/// windows (they share one Rust process) so the read-merge-write of the
/// `barItemPlacements` blob can never interleave.
#[derive(Default)]
pub struct BarItemPlacementLock(pub Mutex<()>);

fn read_settings() -> Map<String, Value> {
    std::fs::read_to_string(config_dir().join(SETTINGS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_settings(map: &Map<String, Value>) -> Result<(), String> {
    let path = config_dir().join(SETTINGS_FILE);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// Atomically merges `patch` into `barItemPlacements[item_id]` and persists
/// it to the same `config_dir()` settings file the rest of the app reads.
pub async fn settings_set_bar_item_placement(
    lock: &BarItemPlacementLock,
    item_id: String,
    patch: Value,
) -> Result<(), String> {
    let _guard = lock.0.lock().await;

    let mut settings = read_settings();

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

    write_settings(&settings)
}
