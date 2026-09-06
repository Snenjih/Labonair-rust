//! Per-pane `impl SettingsView` render blocks, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move — no logic
//! change). Each sibling module is one additional `impl SettingsView` block; a
//! Future custom value surfaces can be added here without changing the
//! navigation model.

mod generic;
mod personalization;
