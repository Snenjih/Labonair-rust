//! Theme-owned application policy (R08-005).
//!
//! `labonair-theme` owns the runtime `ThemeStore`, the deterministic built-in
//! catalogs, and the preview/activation *mechanics*. This sibling owns the
//! *policy* that connects those to the layered settings values: reading the
//! persisted preferences into the store, persisting a picker selection, and
//! the live hover-preview / cancel-revert used by the command palette.
//!
//! It was extracted from `labonair-settings-ui` so the Settings UI is a
//! values-only editor and does not own `ThemeStore` policy. `labonair-theme`
//! stays free of any `labonair-settings` dependency; the settings→theme
//! direction lives only here.

mod apply;
pub mod command_provider;

pub use apply::{
    activate_app_theme, apply_prefs_to_theme, apply_theme_metrics, preview_app_theme,
    theme_metrics_from_settings,
};
