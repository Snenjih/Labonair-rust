//! `labonair-hosts-ui` — the host connect list + host / credential editing UI.
//!
//! Extracted from `labonair-workspace` in T16-008. This is deliberately **not**
//! a dock panel and has **no** `impl Panel` (see `docs/architecture.md §2`):
//! the management surface is embedded by `labonair-settings-ui` (Settings ›
//! Hosts, T19-010) and the connect surface is fed to the command palette as
//! data. Per dependency-rule 9 this crate depends only on `labonair-backend`,
//! `labonair-ui-kit`, `labonair-theme` and `labonair-notifications` — never on
//! `labonair-workspace` or `labonair-panel`. Opening an SSH/SFTP tab happens
//! via [`HostManagerEvent`] emitted to the caller, not a direct call.

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

mod hosts;
pub mod ssh_connection;

pub use hosts::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};
