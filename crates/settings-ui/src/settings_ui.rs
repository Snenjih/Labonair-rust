//! `labonair-settings-ui` — the settings OS window and its preferences store.
//!
//! Extracted verbatim from `crates/ui/src/settings.rs` in T16-007, then
//! (T19-004) rebuilt to generate its UI from the typed `SettingsContent` tree
//! (`labonair-settings-content`) via the layered `SettingsStore`
//! (`labonair-settings`, T19-002/003) instead of the old hand-maintained
//! `FIELDS: &[FieldDef]` table — see `schema.rs` (the field registry) and
//! `pages.rs` (the declarative navigation model). `PreferencesStore` /
//! `GlobalPreferences` stay: they are the compatibility bridge every
//! not-yet-`Settings`-trait-migrated module (terminal, editor, workspace,
//! command-palette) still reads, kept in sync on every `SettingsStore` write
//! (`store.rs`'s `reload_from_disk`).

mod apply;
mod pages;
mod panes;
mod schema;
mod search;
mod store;
mod view;
mod window;

#[cfg(test)]
mod tests;

pub use apply::{activate_app_theme, apply_prefs_to_theme, preview_app_theme, theme_choices};
pub use store::{GlobalPreferences, PreferencesStore};
pub use view::SettingsView;
pub use window::{open_settings_window, set_keybind_apply_hook, set_settings_deps};
