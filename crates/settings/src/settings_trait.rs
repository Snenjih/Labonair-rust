//! `trait Settings` — a feature-local, typed slice of the merged
//! [`SettingsContent`] tree (T19-002 Anweisung #4).
//!
//! Port of `zed-refrence/zed/crates/settings/src/settings_store.rs`'s
//! `Settings` trait, trimmed to what this app needs: no `SettingsLocation`
//! parameter (project-scoped reads land with T19-003; until then every
//! `Settings` type resolves against the single global `merged` tree).

use gpui::App;

use labonair_settings_content::SettingsContent;

use crate::store::SettingsStore;

/// Implemented by every feature-local settings slice (`ThemeSettings`,
/// `TerminalSettings`, …). `#[derive(RegisterSetting)]` (this crate's
/// re-export of `labonair_settings_macros::RegisterSetting`) generates the
/// `inventory::submit!` entry that calls [`Settings::register`] for you —
/// see `crates/settings/src/concrete.rs`.
pub trait Settings: 'static + Sized {
    /// Build `Self` from the current effective tree. Should read through
    /// `SettingsContent::defaults()` merged with `content` (i.e. never
    /// `.unwrap_or(Rust-default)` on a leaf that has a documented default —
    /// use the area's own `defaults()` merge) so a `None` anywhere in
    /// `content` still resolves to the historically-correct default value.
    fn from_settings(content: &SettingsContent) -> Self;

    /// Register this type with the [`SettingsStore`] global. Called once at
    /// startup for every `#[derive(RegisterSetting)]` type via
    /// `labonair_settings::register_all`; safe to call more than once
    /// (`SettingsStore::register_setting` dedups by `TypeId`).
    #[track_caller]
    fn register(cx: &mut App)
    where
        Self: Sized,
    {
        cx.global_mut::<SettingsStore>().register_setting::<Self>();
    }

    /// The current computed value. Panics if [`Settings::register`] never
    /// ran for `Self` (see `SettingsStore::get`'s panic message).
    #[track_caller]
    fn get(cx: &App) -> &Self
    where
        Self: Sized,
    {
        cx.global::<SettingsStore>().get::<Self>()
    }

    /// [`Settings::get`], but `None` instead of a panic if the
    /// [`SettingsStore`] global doesn't exist yet or `Self` was never
    /// registered — for render paths that may run before
    /// `labonair_settings::init` (a headless test harness that never called
    /// it, for instance).
    fn try_get(cx: &App) -> Option<&Self>
    where
        Self: Sized,
    {
        cx.try_global::<SettingsStore>()
            .filter(|store| store.is_registered::<Self>())
            .map(|store| store.get::<Self>())
    }
}
