//! `labonair-settings-ui` — the settings OS window and its preferences store.
//!
//! Extracted verbatim from `crates/ui/src/settings.rs` in T16-007. This is a
//! pure crate split: `FIELDS`, `SECTION_GROUPS`, `CATEGORIES` and the bespoke
//! panes are unchanged; only `mod` / `use` / `pub use` lines differ. The
//! model-side of `PreferencesStore` may move to `labonair-settings` in Phase 18
//! (T19-*); it stays here for now because it is UI-near (holds
//! `GlobalPreferences`, notifies views).

mod apply;
mod fields;
mod panes;
mod sections;
mod store;
mod view;
mod window;

#[cfg(test)]
mod tests;

pub use apply::{activate_app_theme, apply_prefs_to_theme, preview_app_theme, theme_choices};
pub use fields::{
    FieldDef, FieldKind, SettingsTab, AGENT_BRIDGE, CATEGORIES, CAT_APPEARANCE, FIELDS, KEYBOARD,
};
pub use sections::SECTION_GROUPS;
pub use store::{GlobalPreferences, PreferencesStore};
pub use view::SettingsView;
pub use window::{open_settings_window, set_keybind_apply_hook, set_settings_deps};
