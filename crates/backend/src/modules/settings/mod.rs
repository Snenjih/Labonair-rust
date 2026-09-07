use serde_json::{Map, Value};
use std::path::Path;

pub mod migrate_v2;
pub mod preferences;

/// The shared user configuration file used by every native settings writer.
pub const CONFIG_FILE: &str = "config.json";
const LEGACY_CONFIG_FILE: &str = "labonair-settings.json";

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
