//! `GlobalPreferences` — app-wide read-only [`Preferences`] snapshot.
//!
//! Moved here from `crates/ui/src/settings.rs` in T16-006 so the workspace and
//! its tab-content views (`views::terminal`, `views::editor`) — plus the future
//! `panel-ai` — can read it without depending on `labonair-ui`. `SettingsView`
//! and the rest of `settings.rs` stay in `crates/ui`, which re-exports this
//! type via `pub use labonair_workspace::prefs::GlobalPreferences`.

use gpui::Global;
use labonair_backend::modules::settings::preferences::Preferences;

/// App-wide read-only snapshot of [`Preferences`], republished by
/// `PreferencesStore` (in `crates/ui`) on every change. Modules that can't hold
/// an `Entity<PreferencesStore>` (the terminal engine spawn path, editor views)
/// read it via `cx.global::<GlobalPreferences>()` / `cx.observe_global`.
#[derive(Clone, Default)]
pub struct GlobalPreferences(pub Preferences);

impl Global for GlobalPreferences {}
