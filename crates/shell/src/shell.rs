//! `labonair-shell` — the thin composition layer.
//!
//! `AppShell` composes the titlebar, docks, workspace, statusbar and the
//! modal / overlay layer. It is the **only** crate that knows the concrete
//! panel types (it performs registration). No feature logic lives here — the
//! `app_shell.rs` body is still large and will be slimmed to pure composition
//! in T17-006; T16-009 only isolated it out of the former `crates/ui` monolith.
//!
//! Modules kept here are shell-near: the native macOS menu bar (`menu`), the
//! window-geometry persistence (`window_state`), the bundled icon assets
//! (`assets`), plus the shell-near overlays / statusbar helpers
//! (`updater`). Some of these move again in Phase 17 (into dedicated statusbar
//! items), but they belong to the shell composition until then. The former
//! `sidebar_slot` module was removed in T17-002 — the dock
//! model now lives in `labonair-workspace` (`dock.rs`).

pub mod actions;
pub mod app_shell;
pub mod assets;
pub mod bootstrap;
pub mod commands;
pub mod composition;
pub mod keymap_loader;
mod local_terminal_access;
pub mod menu;
pub mod modals;
mod settings_services;
pub mod status_items;
pub mod titlebar;
pub mod window_state;

pub use app_shell::AppShell;
pub use assets::Assets;
pub use composition::AppComposition;
pub use labonair_updater_ui::{UpdaterStatus, UpdaterView};
pub use menu::{apply_keymap, init as init_menus};
pub use titlebar::Titlebar;

// --- Convenience re-exports for the `crates/app` bootstrap -----------------
// `main.rs` stays a straight-line bootstrap: it reaches every init hook
// through the single `labonair_shell::` import root. These simply forward to
// the crate that actually owns each hook (theme store, background store,
// notification center), so no bootstrap logic changed with the crate split.
pub use labonair_background::init as init_background;
pub use labonair_notifications::init as init_notifications;
pub use labonair_settings::init as init_settings;
pub use labonair_theme::{init_fonts, init_theme};

/// Run the one-time import of legacy Settings layout values into the
/// workspace-owned layout file. The app bootstrap exposes this through the
/// shell so the entrypoint does not need a direct dependency on Workspace.
pub fn migrate_legacy_workspace_layout(config_dir: &std::path::Path) -> Result<bool, String> {
    labonair_workspace::layout::migrate_legacy_settings_file(config_dir)
}

/// Runs the one-time settings migrations before the first native Settings
/// entity is created. Migration ordering is a composition concern; value
/// ownership remains in the Settings and Workspace owners.
pub fn migrate_legacy_settings() {
    use labonair_settings::legacy_migrations::{
        migrate_config_file_name, migrate_settings_v1_to_v2, sparsify_v2_settings,
    };

    let config_dir = labonair_filesystem::paths::config_dir();
    if let Err(error) = migrate_config_file_name(&config_dir) {
        tracing::warn!("config filename migration failed: {error}");
    }
    match migrate_legacy_workspace_layout(&config_dir) {
        Ok(true) => tracing::info!("migrated legacy workspace layout before Settings"),
        Ok(false) => {}
        Err(error) => tracing::warn!("workspace layout migration failed: {error}"),
    }
    match migrate_settings_v1_to_v2(&config_dir) {
        Ok(outcome) => tracing::info!("settings v1->v2 migration: {outcome:?}"),
        Err(error) => tracing::warn!("settings v1->v2 migration failed: {error}"),
    }
    match sparsify_v2_settings(&config_dir) {
        Ok(outcome) => tracing::info!("settings v2 sparsify: {outcome:?}"),
        Err(error) => tracing::warn!("settings v2 sparsify failed: {error}"),
    }
}

pub use labonair_updater::{UpdateManifest, CURRENT_VERSION};

// --- Internal re-export shims --------------------------------------------------
// `app_shell.rs` / `updater.rs` were moved verbatim from `crates/ui` (their
// diet is T17-006). These `crate::…` paths kept resolving there through
// `crates/ui`'s own shims; the same shims live here now so the moved files
// stay byte-for-byte identical.
pub(crate) mod background {
    pub use labonair_background::*;
}
pub(crate) mod session {
    pub use labonair_workspace::session::*;
}
pub(crate) mod updater {
    pub use labonair_updater_ui::*;
}
pub(crate) mod theme {
    pub use labonair_theme::store::*;
}
pub(crate) mod workspace {
    pub use labonair_workspace::{Workspace, WorkspaceEvent};
}
