//! `labonair-hosts-ui` — host and credential management UI.
//!
//! This is deliberately **not** a dock panel and has **no** `impl Panel` (see
//! `docs/architecture.md §2`): the management surface is owned by the Hosts
//! capability and the connect surface is fed to the command palette as data.
//! The Settings window embeds [`HostManagerView`] as its full-page Hosts
//! surface (ADR 0007, registered by [`register_settings_surfaces`]); opening
//! an SSH/SFTP tab from within it still happens via [`HostManagerEvent`]
//! emitted to the composition root, never by a direct call.

pub mod command_provider;

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

mod hosts;

pub use hosts::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};

use gpui::Entity;
use labonair_settings::{
    SettingsSurface, SettingsSurfaceContribution, SettingsSurfaceId, SettingsSurfacePage,
    SettingsSurfaceRegistry, FIRST_OWNER_PAGE_INDEX,
};

/// Contribute the Hosts page to the Settings navigation, directly under Keymap.
/// The surface is inline and full-page: Settings embeds `view` and gives it the
/// whole content area. `open_hosts` is the fallback entry point (open Settings
/// on the Hosts page).
pub fn register_settings_surfaces(
    registry: &mut SettingsSurfaceRegistry,
    view: Entity<HostManagerView>,
    open_hosts: impl Fn(&mut gpui::App) + 'static,
) -> Result<(), String> {
    registry.register(SettingsSurfaceContribution::inline(
        SettingsSurface::new(
            SettingsSurfaceId::new("hosts"),
            "Hosts",
            "Manage saved SSH hosts, groups, tunnels and credentials.",
            "Open hosts…",
            SettingsSurfacePage::new_page(
                "hosts",
                "Hosts",
                "Hosts & credentials",
                // Keymap takes `FIRST_OWNER_PAGE_INDEX`; Hosts follows it.
                FIRST_OWNER_PAGE_INDEX + 1,
            ),
        )
        .full_page(),
        open_hosts,
        move |_window, _cx| view.clone().into(),
    ))
}
