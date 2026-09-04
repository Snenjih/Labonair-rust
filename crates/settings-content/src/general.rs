//! `general` area — startup / lifecycle behaviour. Field names carried over
//! 1:1 (camelCase serde) from `labonair-backend`'s `Preferences` "General"
//! group.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};

/// The app theme the user picked. `System` follows the OS appearance.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl MergeFrom for ThemePref {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

/// What the app opens on launch (when session restore has no snapshot).
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum StartupTab {
    /// Open one local terminal tab.
    Terminal,
    /// Open nothing — start on the empty-workspace surface (T17-009). The old
    /// `host-manager` value migrates here.
    #[default]
    #[serde(alias = "host-manager")]
    Empty,
}

impl MergeFrom for StartupTab {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct GeneralContent {
    pub theme: Option<ThemePref>,
    pub restore_window_state: Option<bool>,
    pub default_startup_tab: Option<StartupTab>,
    /// Number of terminals opened on launch (1..=3).
    pub startup_terminal_count: Option<u32>,
    /// Launch the app at login (mirrors the OS autostart entry).
    pub autostart: Option<bool>,
    /// Encrypt stored credentials at rest.
    pub credential_encryption: Option<bool>,
    pub notify_on_errors: Option<bool>,
    pub confirm_quit_with_ssh: Option<bool>,
    pub check_for_updates: Option<bool>,
    /// Reopen the previous tabs / split layout on the next launch.
    pub session_restore: Option<bool>,
}

impl GeneralContent {
    pub fn defaults() -> Self {
        Self {
            theme: Some(ThemePref::System),
            restore_window_state: Some(true),
            default_startup_tab: Some(StartupTab::Empty),
            startup_terminal_count: Some(1),
            autostart: Some(false),
            credential_encryption: Some(false),
            notify_on_errors: Some(false),
            confirm_quit_with_ssh: Some(true),
            check_for_updates: Some(true),
            session_restore: Some(false),
        }
    }
}
