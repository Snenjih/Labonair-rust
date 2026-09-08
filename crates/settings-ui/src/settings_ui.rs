//! `labonair-settings-ui` — the settings OS window and its preferences store.
//!
//! Extracted verbatim from `crates/ui/src/settings.rs` in T16-007, then
//! (T19-004) rebuilt to generate its UI from the typed `SettingsContent` tree
//! (`labonair-settings-content`) via the layered `SettingsStore`
//! (`labonair-settings`, T19-002/003) instead of the old hand-maintained
//! `FIELDS: &[FieldDef]` table — see `schema.rs` (the field registry) and
//! `pages.rs` (the declarative navigation model). Every feature now reads its
//! slice of the layered `SettingsStore` through the `Settings` trait — the old
//! `PreferencesStore` / `GlobalPreferences` bridge has been retired.

mod apply;
pub mod command_provider;
mod pages;
mod panes;
mod schema;
mod search;
mod services;
mod view;
mod window;

#[cfg(test)]
mod tests;

pub use services::{ServiceFuture, SettingsServices, SystemFontService};
pub use view::SettingsView;
pub use window::{open_settings_window, set_settings_deps};
