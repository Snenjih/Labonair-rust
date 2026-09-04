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
//! (`updater`, `cwd_breadcrumb`). Some of these move again in Phase 17 (into
//! dedicated statusbar items), but they belong to the shell composition until
//! then. The former `sidebar_slot` module was removed in T17-002 — the dock
//! model now lives in `labonair-workspace` (`dock.rs`).

pub mod actions;
pub mod app_shell;
pub mod assets;
pub mod bootstrap;
pub mod commands;
pub mod cwd_breadcrumb;
pub mod menu;
pub mod modals;
pub mod status_items;
pub mod titlebar;
pub mod updater;
pub mod window_state;

pub use app_shell::AppShell;
pub use assets::Assets;
pub use menu::{apply_keybinds, init as init_menus};
pub use titlebar::Titlebar;
pub use updater::{UpdaterStatus, UpdaterView};

// --- Convenience re-exports for the `crates/app` bootstrap -----------------
// `main.rs` stays a straight-line bootstrap: it reaches every init hook
// through the single `labonair_shell::` import root. These simply forward to
// the crate that actually owns each hook (theme store, background store,
// notification center), so no bootstrap logic changed with the crate split.
pub use labonair_notifications::init as init_notifications;
pub use labonair_theme::{init_fonts, init_theme};
pub use labonair_workspace::background::init as init_background;

// --- Internal re-export shims --------------------------------------------------
// `app_shell.rs` / `updater.rs` were moved verbatim from `crates/ui` (their
// diet is T17-006). These `crate::…` paths kept resolving there through
// `crates/ui`'s own shims; the same shims live here now so the moved files
// stay byte-for-byte identical.
pub(crate) mod background {
    pub use labonair_workspace::background::*;
}
pub(crate) mod bar_items {
    pub use labonair_workspace::bar_items::*;
}
pub(crate) mod pane {
    pub use labonair_workspace::pane::*;
}
pub(crate) mod session {
    pub use labonair_workspace::session::*;
}
pub(crate) mod theme {
    pub use labonair_theme::store::*;
}
pub(crate) mod workspace {
    pub use labonair_workspace::Workspace;
}
