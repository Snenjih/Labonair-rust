//! Detached session host (ADR 0009): wire [`protocol`] and the daemon
//! [`server`]. GPUI-free; depends only on `labonair-terminal-core`.

pub mod protocol;
#[cfg(unix)]
pub mod server;
