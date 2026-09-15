//! `connections` area — global SSH connection defaults. Per-host values
//! (`hosts.keep_alive_interval`, `hosts.keep_alive_tries`) still win when a
//! host record sets them explicitly; these are only the fallback applied to
//! hosts that leave them unset, plus the one value (connect timeout) that has
//! no host-level override today.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct ConnectionsContent {
    /// Seconds to wait for the initial TCP+SSH handshake before giving up.
    pub ssh_connect_timeout_secs: Option<u64>,
    /// Fallback keep-alive ping interval, in seconds, for hosts that don't
    /// set their own.
    pub ssh_keepalive_interval_secs: Option<u64>,
    /// Fallback number of missed keep-alive pings tolerated before the
    /// connection is considered dead, for hosts that don't set their own.
    pub ssh_keepalive_max_failures: Option<u32>,
}

impl ConnectionsContent {
    pub fn defaults() -> Self {
        Self {
            ssh_connect_timeout_secs: Some(20),
            ssh_keepalive_interval_secs: Some(25),
            ssh_keepalive_max_failures: Some(3),
        }
    }
}
