//! `labonair-settings` — the layered [`SettingsStore`] (T19-002).
//!
//! Loads every settings layer (`assets/settings/default.json` < the user's
//! `labonair-settings.json` < …), merges them in the fixed order normative in
//! `docs/architecture.md`, watches the user file live, and hands out
//! feature-local typed slices (`ThemeSettings::get(cx)`,
//! `TerminalSettings::get(cx)`, …) through the [`Settings`] trait. The
//! `GlobalPreferences` bridge (`labonair-workspace::prefs`,
//! `labonair-settings-ui::PreferencesStore`) is untouched by this crate and
//! keeps every not-yet-migrated module working exactly as before — see the
//! task's `## Notizen` (full migration off `GlobalPreferences` is explicitly
//! out of scope here).
//!
//! `extern crate self as labonair_settings` lets `#[derive(RegisterSetting)]`
//! (used by this crate's own `concrete.rs`) address this crate's root via the
//! same `::labonair_settings::*` absolute path a downstream crate would use,
//! without special-casing "am I the defining crate" in the macro.
extern crate self as labonair_settings;

mod concrete;
mod registry;
mod settings_trait;
mod store;
mod watch;

pub use concrete::{
    AiSettings, EditorSettings, PersonalizationSettings, TerminalSettings, ThemeSettings,
    WorkspaceSettings,
};
pub use registry::{register_all, RegisteredSetting};
pub use settings_trait::Settings;
pub use store::{SettingsLayer, SettingsStore, WorktreeId};

// Re-exported so `#[derive(RegisterSetting)]`'s generated code can address
// `gpui`/`inventory` through this crate's own path, whatever the consuming
// crate's own dependency list looks like.
pub use gpui;
pub use inventory;
pub use labonair_settings_content as content;
pub use labonair_settings_content::SettingsContent;
pub use labonair_settings_macros::RegisterSetting;

use gpui::App;

/// Build the store from `assets/settings/default.json` + the user's
/// `~/.config/labonair/labonair-settings.json`, publish it as the
/// [`SettingsStore`] global, register every `#[derive(RegisterSetting)]`
/// type, and start the live fs-watch. Call once, before the first render
/// (`crates/app/src/main.rs`, right after `labonair_shell::init_fonts`).
pub fn init(cx: &mut App) {
    store::init(cx);
    register_all(cx);
    let user_path = cx.global::<SettingsStore>().user_path().to_path_buf();
    watch::spawn(cx, user_path);
}
