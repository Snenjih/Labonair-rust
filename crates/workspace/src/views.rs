//! Tab-content views owned by [`Workspace`](crate::Workspace).
//!
//! Moved out of `crates/ui` in T16-006. `hosts` and `git_graph` keep their
//! current tab-view form here; their *panel* variants are T16-008 (`hosts` →
//! `labonair-panel-hosts`, `git_graph` panel → `labonair-panel-scm`).

pub mod diff;
pub mod editor;
pub mod git_graph;
pub mod hosts;
pub mod preview;
pub mod sftp;
pub mod ssh_connection;
pub mod terminal;
