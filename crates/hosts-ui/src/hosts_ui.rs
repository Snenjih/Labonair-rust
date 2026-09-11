//! `labonair-hosts-ui` — the host connect list + host / credential editing UI.
//!
//! Extracted from `labonair-workspace` in T16-008. This is deliberately **not**
//! a dock panel and has **no** `impl Panel` (see `docs/architecture.md §2`):
//! the management surface is owned by the Hosts capability and the connect
//! surface is fed to the command palette as data. It never depends on
//! `labonair-workspace` or `labonair-panel`; `Workspace` renders `HostManagerView`
//! as its `Hosts` tab body (composition root wiring), and opening an SSH/SFTP
//! tab from within it still happens via [`HostManagerEvent`] emitted to the
//! caller, not a direct call — only tab presentation, not this crate's
//! dependency direction, changed with the window→tab migration.

pub mod command_provider;

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

mod hosts;

pub use hosts::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};
