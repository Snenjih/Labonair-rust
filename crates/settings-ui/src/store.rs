//! Preferences store (`PreferencesStore`) + the app-wide `GlobalPreferences`
//! snapshot re-export. Split out of the old `crates/ui/src/settings.rs` monolith
//! in T16-007 (mechanical move — no logic change).

use serde_json::Value;

use gpui::{App, Context};

use labonair_backend::modules::settings::preferences::{
    preferences_load, preferences_load_from, preferences_save, preferences_save_to,
    PaletteSearchMode, Preferences,
};
use labonair_command_palette::{PalettePrefs, SearchMode};

// ─────────────────────────── Global snapshot ─────────────────────────────

/// App-wide read-only snapshot of [`Preferences`], republished by
/// [`PreferencesStore`] on every change. Moved to `labonair-workspace` in
/// T16-006 (so the workspace + its tab views can read it without depending on
/// `labonair-ui`); re-exported here so every `labonair_settings_ui::GlobalPreferences`
/// path keeps resolving.
pub use labonair_workspace::prefs::GlobalPreferences;

// ─────────────────────────── Preferences store ───────────────────────────

/// GPUI entity wrapping the persisted [`Preferences`]. Generic key access
/// keeps the settings UI table-driven without a giant match: a mutation
/// serializes the model to a JSON object, swaps the one key, and validates by
/// deserializing back — a wrong-typed value is rejected, not stored.
pub struct PreferencesStore {
    prefs: Preferences,
    /// `None` = the shared per-user config file; `Some` = an explicit config
    /// directory (used by tests so they never touch the real settings file).
    dir: Option<std::path::PathBuf>,
}

impl Default for PreferencesStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesStore {
    pub fn new() -> Self {
        Self {
            prefs: preferences_load(),
            dir: None,
        }
    }

    /// Construct against an explicit config directory (testing).
    pub fn with_dir(dir: std::path::PathBuf) -> Self {
        Self {
            prefs: preferences_load_from(&dir),
            dir: Some(dir),
        }
    }

    fn persist(&self) -> Result<(), String> {
        match &self.dir {
            Some(dir) => preferences_save_to(dir, &self.prefs),
            None => preferences_save(&self.prefs),
        }
    }

    /// The full typed model — modules read their slice from here.
    pub fn get(&self) -> &Preferences {
        &self.prefs
    }

    /// (Re)publish the current model into the [`GlobalPreferences`] global.
    /// Call once at startup after construction; `set_value` keeps it current.
    pub fn publish_global(&self, cx: &mut App) {
        cx.set_global(GlobalPreferences(self.prefs.clone()));
    }

    /// The current JSON value for one camelCase key.
    pub fn value(&self, key: &str) -> Option<Value> {
        serde_json::to_value(&self.prefs).ok()?.get(key).cloned()
    }

    /// Set one key. Persists + notifies if the value parsed and changed.
    pub fn set_value(&mut self, key: &str, value: Value, cx: &mut Context<Self>) {
        let Ok(Value::Object(mut map)) = serde_json::to_value(&self.prefs) else {
            return;
        };
        map.insert(key.to_string(), value);
        match serde_json::from_value::<Preferences>(Value::Object(map)) {
            Ok(next) if next != self.prefs => {
                self.prefs = next;
                if let Err(e) = self.persist() {
                    tracing::warn!("failed to persist preferences: {e}");
                }
                cx.set_global(GlobalPreferences(self.prefs.clone()));
                cx.notify();
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("rejected preference `{key}`: {e}"),
        }
    }
}

/// Bridges the [`PreferencesStore`] to the `labonair-command-palette` view's
/// [`PalettePrefs`] contract (T16-004 decoupling). Every accessor is a verbatim
/// field read of the `command_palette_*` preferences; the setter is the same
/// `set_value("commandPaletteSearchMode", …)` call the palette footer used to
/// make directly.
impl PalettePrefs for PreferencesStore {
    fn command_palette_search_mode(&self) -> SearchMode {
        match self.prefs.command_palette_search_mode {
            PaletteSearchMode::Contains => SearchMode::Contains,
            PaletteSearchMode::StartsWith => SearchMode::StartsWith,
            PaletteSearchMode::Fuzzy => SearchMode::Fuzzy,
        }
    }

    fn command_palette_history_size(&self) -> u32 {
        self.prefs.command_palette_history_size
    }

    fn command_palette_opacity(&self) -> u32 {
        self.prefs.command_palette_opacity
    }

    fn command_palette_position(&self) -> String {
        self.prefs.command_palette_position.clone()
    }

    fn command_palette_show_recent(&self) -> bool {
        self.prefs.command_palette_show_recent
    }

    fn command_palette_close_on_overlay_click(&self) -> bool {
        self.prefs.command_palette_close_on_overlay_click
    }

    fn set_command_palette_search_mode(&mut self, mode: SearchMode, cx: &mut Context<Self>) {
        self.set_value(
            "commandPaletteSearchMode",
            Value::String(mode.label().to_string()),
            cx,
        );
    }
}
