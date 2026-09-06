//! `labonair-hosts-ui` — the host connect list + host / credential editing UI.
//!
//! Extracted from `labonair-workspace` in T16-008. This is deliberately **not**
//! a dock panel and has **no** `impl Panel` (see `docs/architecture.md §2`):
//! the management surface is owned by the Hosts capability and the connect
//! surface is fed to the command palette as data. Per dependency-rule 9 this
//! capability-owned host, credential, snippet, persistence, and secret
//! contracts, plus injected SSH services. It never depends on
//! `labonair-workspace` or `labonair-panel`; opening an SSH/SFTP tab happens
//! via [`HostManagerEvent`] emitted to the caller, not a direct call.

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

mod hosts;
pub mod ssh_connection;

pub use hosts::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};
