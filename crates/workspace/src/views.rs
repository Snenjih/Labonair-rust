//! Tab-content views owned by [`Workspace`](crate::Workspace).
//!
//! Moved out of `crates/ui` in T16-006. `git_graph` moved to
//! `labonair-panel-git-graph`; `hosts` reaches the Hosts UI through the narrow
//! `labonair-hosts-host::HostView` contract (R08-012); the connection-status
//! store is `crate::ssh_connection`.

pub mod editor;
pub mod preview;
pub mod project_diff;
pub mod sftp;
pub mod terminal;
