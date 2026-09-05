//! `labonair-settings` — the layered [`SettingsStore`] (T19-002).
//!
//! Loads every settings layer (`assets/settings/default.json` < the user's
//! `config.json` < …), merges them in the fixed order normative in
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
pub mod keymap;
pub mod project;
mod registry;
pub mod schema;
mod settings_trait;
mod store;
mod watch;

pub use concrete::{
    AiSettings, EditorSettings, PersonalizationSettings, TerminalSettings, ThemeSettings,
    WorkspaceSettings,
};
pub use keymap::{ensure_user_keymap_file, user_keymap_path};
pub use project::{ensure_project_settings_file, PROJECT_SETTINGS_WHITELIST};
pub use registry::{register_all, RegisteredSetting};
pub use schema::{description_for_path, json_schema, SettingsValidationError};
pub use settings_trait::Settings;
pub use store::{
    ensure_user_settings_file, settings_schema_path, user_settings_path, SettingsLayer,
    SettingsStore, WorktreeId,
};
pub use watch::{watch_dir, watch_file};

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
/// `~/.config/labonair/config.json`, publish it as the
/// [`SettingsStore`] global, register every `#[derive(RegisterSetting)]`
/// type, and start the live fs-watch. Call once, before the first render
/// (`crates/app/src/main.rs`, right after `labonair_shell::init_fonts`).
pub fn init(cx: &mut App) {
    store::init(cx);
    register_all(cx);
    let user_path = cx.global::<SettingsStore>().user_path().to_path_buf();
    watch::spawn(cx, user_path);
    store::write_schema_file();
}

/// Set (or clear, with `None`) the active project root — the folder the
/// active pane/explorer currently has open (T19-003). Loads `<root>/
/// .labonair/settings.json` (if present) as the `SettingsLayer::Project`
/// layer, through the whitelist filter (`project::filter_and_parse`), and
/// (re)starts that file's live fs-watch; a no-op if `root` is already the
/// active (canonicalized) root. This crate has no notion of "explorer" or
/// "active pane" itself (leaf crate — `docs/architecture.md` §3) — call this
/// from `labonair-workspace` whenever the active pane's cwd changes.
pub fn set_active_project_root(cx: &mut App, root: Option<std::path::PathBuf>) {
    let changed = cx
        .global_mut::<SettingsStore>()
        .set_active_project_root(root);
    if !changed {
        return;
    }
    let store = cx.global::<SettingsStore>();
    let generation = store.project_watch_generation();
    if let Some(root) = store.project_root().map(std::path::Path::to_path_buf) {
        let dir = root.join(".labonair");
        // If `.labonair` doesn't exist yet, there's nothing to watch — the
        // create-scaffold command re-invokes this function afterward
        // (`Workspace`'s "open/create project settings" command sets the
        // root again once the directory exists), which then starts the
        // watch normally.
        if dir.is_dir() {
            watch::spawn_project(cx, dir, generation);
        }
    }
}

/// Force-reload the active project layer and (re)start its fs-watch, even
/// though the root itself hasn't changed. Call this right after
/// [`ensure_project_settings_file`] creates `<root>/.labonair/` for a root
/// that had no such directory yet — until now there was nothing to watch,
/// so `set_active_project_root` with the same root would otherwise stay a
/// no-op. No-op if no project root is currently active.
pub fn refresh_project_watch(cx: &mut App) {
    let Some(generation) = cx.global_mut::<SettingsStore>().rewatch_project() else {
        return;
    };
    let store = cx.global::<SettingsStore>();
    if let Some(root) = store.project_root().map(std::path::Path::to_path_buf) {
        let dir = root.join(".labonair");
        if dir.is_dir() {
            watch::spawn_project(cx, dir, generation);
        }
    }
}
