//! Tab-content views owned by [`Workspace`](crate::Workspace).
//!
//! Moved out of `crates/ui` in T16-006. In T16-008 `git_graph` moved on to
//! `labonair-panel-git-graph` and `hosts` / `ssh_connection` to
//! `labonair-hosts-ui`; `Workspace` now imports those from their own crates.

pub mod diff;
pub mod editor;
pub mod preview;
pub mod sftp;
pub mod terminal;
