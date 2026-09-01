//! Persisted MCP-bridge preferences (T11-006).
//!
//! The Rust `McpState` (see `modules::mcp`) has no persistence of its own — it
//! always boots disabled with default port/timeout — so these preferences are
//! the load-bearing source of truth for the bridge's configuration, mirrored
//! into `McpState` at startup and on every settings change (matches the
//! reference `useMcpTabBridge.ts` re-sync effect).
//!
//! Stored as an `mcp` object inside the shared `labonair-settings.json`, the
//! same file `settings::editor` and the bar-item placements use.

use serde::{Deserialize, Serialize};

use crate::modules::fs::paths::config_dir;

const SETTINGS_FILE: &str = "labonair-settings.json";
const KEY: &str = "mcp";

const DEFAULT_PORT: u16 = 47823;
const DEFAULT_MAX_COMMAND_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct McpPrefs {
    /// Whether the bridge listener should be started at launch.
    pub bridge_enabled: bool,
    /// Local port the Streamable-HTTP listener binds to (1024–65535).
    pub bridge_port: u16,
    /// Upper bound on a single agent-run command before it returns
    /// `still_running`, regardless of what the agent requested.
    pub max_command_timeout_secs: u64,
    /// Revoke a granted tab after this many minutes without agent activity.
    /// `0` disables auto-revoke.
    pub auto_revoke_minutes: u32,
    /// Show a toast on every agent action (run command / send keys /
    /// open-close tab). Error toasts are always shown, independent of this.
    pub notify_on_activity: bool,
}

impl Default for McpPrefs {
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

/// Load the persisted MCP preferences (defaults if none saved yet).
pub fn mcp_prefs_load() -> McpPrefs {
    load_from(&config_dir())
}

/// Persist the MCP preferences, merging into the shared settings file.
pub fn mcp_prefs_save(prefs: &McpPrefs) -> Result<(), String> {
    save_to(&config_dir(), prefs)
}

fn load_from(dir: &std::path::Path) -> McpPrefs {
    std::fs::read_to_string(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(KEY).cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn save_to(dir: &std::path::Path, prefs: &McpPrefs) -> Result<(), String> {
    let path = dir.join(SETTINGS_FILE);
    let mut map = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    map.insert(
        KEY.to_string(),
        serde_json::to_value(prefs).map_err(|e| e.to_string())?,
    );
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&map).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_merges_into_shared_file() {
        let dir = std::env::temp_dir().join(format!("labonair-mcp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"editor":{"number":true}}"#).unwrap();

        let p = McpPrefs {
            bridge_enabled: true,
            bridge_port: 51000,
            auto_revoke_minutes: 15,
            notify_on_activity: true,
            ..Default::default()
        };
        save_to(&dir, &p).unwrap();

        let back = load_from(&dir);
        assert!(back.bridge_enabled);
        assert_eq!(back.bridge_port, 51000);
        assert_eq!(back.auto_revoke_minutes, 15);
        assert!(back.notify_on_activity);

        let raw = std::fs::read_to_string(dir.join(SETTINGS_FILE)).unwrap();
        assert!(raw.contains("\"editor\""), "unrelated keys preserved");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = std::env::temp_dir().join(format!("labonair-mcp-{}", uuid::Uuid::new_v4()));
        assert_eq!(load_from(&dir), McpPrefs::default());
    }

    #[test]
    fn partial_json_falls_back_field_by_field() {
        let dir = std::env::temp_dir().join(format!("labonair-mcp-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SETTINGS_FILE), r#"{"mcp":{"bridgeEnabled":true}}"#).unwrap();
        let back = load_from(&dir);
        assert!(back.bridge_enabled);
        assert_eq!(back.bridge_port, DEFAULT_PORT, "missing field uses default");
        std::fs::remove_dir_all(&dir).ok();
    }
}
