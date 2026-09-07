//! Persisted preferences for the local MCP bridge.
//!
//! The MCP capability owns this wire shape. Filesystem location is supplied by
//! the composition root so this UI-free crate stays independent of platform
//! path discovery.

use std::path::Path;

use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.json";
const KEY: &str = "mcp";
const DEFAULT_PORT: u16 = 47823;
const DEFAULT_MAX_COMMAND_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct McpPreferences {
    pub bridge_enabled: bool,
    pub bridge_port: u16,
    pub max_command_timeout_secs: u64,
    pub auto_revoke_minutes: u32,
    pub notify_on_activity: bool,
}

impl Default for McpPreferences {
    fn default() -> Self {
        Self {
            bridge_enabled: false,
            bridge_port: DEFAULT_PORT,
            max_command_timeout_secs: DEFAULT_MAX_COMMAND_TIMEOUT_SECS,
            auto_revoke_minutes: 0,
            notify_on_activity: false,
        }
    }
}

impl McpPreferences {
    /// Load preferences from the shared config file, falling back per field to
    /// the capability defaults when the file or object is incomplete.
    pub fn load_from(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(CONFIG_FILE))
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| value.get(KEY).cloned())
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default()
    }

    /// Merge preferences into the shared config file without discarding other
    /// capability-owned top-level keys.
    pub fn save_to(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join(CONFIG_FILE);
        let mut map = std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        map.insert(
            KEY.to_string(),
            serde_json::to_value(self).map_err(|error| error.to_string())?,
        );
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        let temp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&map).map_err(|error| error.to_string())?;
        std::fs::write(&temp, json).map_err(|error| error.to_string())?;
        std::fs::rename(&temp, &path).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("labonair-mcp-core-{label}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trip_merges_without_dropping_other_config() {
        let dir = temp_dir("round-trip");
        std::fs::write(dir.join(CONFIG_FILE), r#"{"editor":{"number":true}}"#).unwrap();
        let prefs = McpPreferences {
            bridge_enabled: true,
            bridge_port: 49152,
            max_command_timeout_secs: 120,
            auto_revoke_minutes: 15,
            notify_on_activity: true,
        };

        prefs.save_to(&dir).unwrap();
        assert_eq!(McpPreferences::load_from(&dir), prefs);
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(CONFIG_FILE)).unwrap()).unwrap();
        assert_eq!(config["editor"]["number"], true);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_or_partial_config_uses_defaults() {
        let dir = temp_dir("defaults");
        assert_eq!(McpPreferences::load_from(&dir), McpPreferences::default());
        std::fs::write(dir.join(CONFIG_FILE), r#"{"mcp":{"bridgeEnabled":true}}"#).unwrap();
        let prefs = McpPreferences::load_from(&dir);
        assert!(prefs.bridge_enabled);
        assert_eq!(prefs.bridge_port, DEFAULT_PORT);
        let _ = std::fs::remove_dir_all(dir);
    }
}
