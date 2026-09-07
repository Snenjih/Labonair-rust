//! One-time compatibility migrations for pre-v2 Labonair settings files.
//!
//! These readers intentionally remain separate from the live SettingsStore:
//! they deserialize historical wire shapes, convert them into current owner
//! formats, and are not a second runtime settings model.

use serde_json::{Map, Value};
use std::path::Path;

mod migrate_v2;
mod preferences;

pub use migrate_v2::{migrate_settings_v1_to_v2, sparsify_v2_settings};

pub const CONFIG_FILE: &str = "config.json";
const LEGACY_CONFIG_FILE: &str = "labonair-settings.json";

fn read_settings_from(dir: &Path) -> Map<String, Value> {
    std::fs::read_to_string(dir.join(CONFIG_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn write_settings_to(dir: &Path, settings: &Map<String, Value>) -> Result<(), String> {
    let path = dir.join(CONFIG_FILE);
    let temp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(&temp, json).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, &path).map_err(|error| error.to_string())
}

/// Adopt the former user-settings filename without overwriting an existing
/// current config.
pub fn migrate_config_file_name(dir: &Path) -> Result<(), String> {
    let legacy = dir.join(LEGACY_CONFIG_FILE);
    let current = dir.join(CONFIG_FILE);
    if current.exists() || !legacy.exists() {
        return Ok(());
    }
    std::fs::rename(&legacy, &current).map_err(|error| {
        format!(
            "failed to rename {} to {}: {error}",
            legacy.display(),
            current.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adopts_legacy_filename_only_when_config_is_missing() {
        let dir = std::env::temp_dir().join(format!(
            "labonair-settings-config-filename-{}",
            std::process::id()
        ));
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

        let _ = std::fs::remove_dir_all(&dir);
    }
}
