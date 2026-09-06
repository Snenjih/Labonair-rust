//! Local filesystem services shared by feature modules.
//!
//! This crate intentionally has no GPUI or application-shell dependency. It
//! owns path resolution, file access, directory traversal, mutation, and
//! search primitives. UI modules consume these services through this crate;
//! filesystem watching remains an application-event integration until its
//! callback contract is extracted.

pub mod file;
pub mod grep;
pub mod mutate;
pub mod paths;
pub mod search;
pub mod tree;
