//! Preferences store (`PreferencesStore`) + the app-wide `GlobalPreferences`
//! snapshot re-export. Split out of the old `crates/ui/src/settings.rs` monolith
//! in T16-007 (mechanical move — no logic change).

use serde_json::Value;

use gpui::{App, Context};

use labonair_backend::modules::settings::preferences::{
    preferences_load, preferences_load_from, preferences_save, preferences_save_to,
    PaletteSearchMode, Preferences, ThemePref,
};
use labonair_command_palette::{KeybindMap, PalettePrefs, SearchMode};
use labonair_theme::{EditorThemeId, ThemePreference};

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

    /// Refresh [`Self::prefs`] (and republish [`GlobalPreferences`] if it
    /// changed) after a write made through the new, T19-004 generated field
    /// grid — which persists into `labonair_settings::SettingsStore`
    /// (`SettingsContent`), not this store's own `Preferences` model. Every
    /// not-yet-`Settings`-trait-migrated consumer (terminal, editor,
    /// workspace) still reads `GlobalPreferences`, so this is the bridge the
    /// task's warning requires: "the `GlobalPreferences` bridge must be kept
    /// current after every write."
    ///
    /// In production (`self.dir.is_none()`) this derives `Preferences` from
    /// the live `SettingsStore` global via `labonair_backend`'s
    /// `impl From<&SettingsContent> for Preferences` bridge — the same
    /// conversion `labonair-settings`'s own doc comment describes. Three
    /// fields have no `SettingsContent` counterpart at all (`keybinds`,
    /// `bar_item_placements`, `bar_layout_migrated` — see that `impl`'s doc
    /// comment) and are preserved from the current in-memory value instead of
    /// being clobbered back to their historical defaults: they are kept live
    /// by `set_pref`'s own synchronous writes (shortcuts capture, the legacy
    /// bar-item editor), which this method must not undo.
    ///
    /// In test mode (`with_dir`) there is no shared `SettingsStore` global to
    /// read, so this simply re-reads the isolated test directory's file —
    /// same behaviour as before T19-004.
    pub fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let fresh = match &self.dir {
            Some(dir) => preferences_load_from(dir),
            None => match cx.try_global::<labonair_settings::SettingsStore>() {
                Some(store) => {
                    let mut bridged = Preferences::from(store.merged());
                    bridged.keybinds = self.prefs.keybinds.clone();
                    bridged.bar_item_placements = self.prefs.bar_item_placements.clone();
                    bridged.bar_layout_migrated = self.prefs.bar_layout_migrated;
                    bridged
                }
                None => preferences_load(),
            },
        };
        if fresh != self.prefs {
            self.prefs = fresh;
            cx.set_global(GlobalPreferences(self.prefs.clone()));
            cx.notify();
        }
    }

    /// Mirror a `set_value` write into the new `SettingsStore`
    /// (`SettingsContent`) too, if `key` has a matching generated field
    /// (`crate::schema::AnyField`) — keeps both trees consistent regardless
    /// of which one a particular pane happens to write through (T19-004's
    /// bidirectional half of the bridge; `reload_from_disk` is the other
    /// half). A key with no matching field (e.g. `"keybinds"`, `"provkey:…"`)
    /// is a no-op here — those have no `SettingsContent` counterpart.
    pub(crate) fn mirror_into_settings_store(&self, key: &str, value: &Value, cx: &mut App) {
        if self.dir.is_some() || !cx.has_global::<labonair_settings::SettingsStore>() {
            return; // test isolation, or the global settings track isn't up yet.
        }
        let Some(field) = crate::schema::all_fields()
            .into_iter()
            .find(|f| f.local_key() == key)
        else {
            return; // no SettingsContent counterpart (e.g. "keybinds", "provkey:…").
        };
        let value = value.clone();
        let _ = cx
            .global_mut::<labonair_settings::SettingsStore>()
            .update_user(move |c| {
                (field.set)(c, value.clone());
            });
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
        map.insert(key.to_string(), value.clone());
        match serde_json::from_value::<Preferences>(Value::Object(map)) {
            Ok(next) if next != self.prefs => {
                self.prefs = next;
                if let Err(e) = self.persist() {
                    tracing::warn!("failed to persist preferences: {e}");
                }
                self.mirror_into_settings_store(key, &value, cx);
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

    fn color_mode(&self) -> ThemePreference {
        match self.prefs.theme {
            ThemePref::System => ThemePreference::System,
            ThemePref::Light => ThemePreference::Light,
            ThemePref::Dark => ThemePreference::Dark,
        }
    }

    fn editor_theme(&self) -> EditorThemeId {
        EditorThemeId::from_slug(&self.prefs.editor_theme).unwrap_or_default()
    }

    fn terminal_font_size(&self) -> u32 {
        self.prefs.terminal_font_size
    }

    fn toggle_state(&self, key: &str) -> bool {
        match key {
            "zenModeShowHeader" => self.prefs.zen_mode_show_header,
            "zenModeShowStatusbar" => self.prefs.zen_mode_show_statusbar,
            "editorWordWrap" => self.prefs.editor_word_wrap,
            "editorLineNumbers" => self.prefs.editor_line_numbers,
            "editorFormatOnSave" => self.prefs.editor_format_on_save,
            "terminalCursorBlink" => self.prefs.terminal_cursor_blink,
            "terminalShowPaneHeader" => self.prefs.terminal_show_pane_header,
            "terminalShowPaneFooter" => self.prefs.terminal_show_pane_footer,
            "vimMode" => self.prefs.editor_vim_mode,
            _ => false,
        }
    }

    fn keybind_overrides(&self) -> KeybindMap {
        self.prefs.keybinds.clone()
    }
}
