//! Per-pane `impl SettingsView` render blocks, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move — no logic
//! change). Each sibling module is one additional `impl SettingsView` block; a
//! future custom pane (e.g. "Hosts", T19-010) drops in here without a rebuild.

mod ai;
mod generic;
mod personalization;
mod shortcuts;
mod themes;
