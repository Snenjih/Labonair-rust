//! Host-view contract between `labonair-workspace` and the Hosts UI (R08-012).
//!
//! The workspace needs to read the host catalog (ids, names, jump-host labels,
//! picker rows) and push two live snapshots back (connection status, active
//! tunnels). Before this crate existed it held an `Entity<HostManagerView>`
//! and called that view's methods directly, coupling the workspace to the
//! Hosts *UI* crate. [`HostView`] replaces that entity with a narrow set of
//! injected callbacks, mirroring `labonair-explorer-host` /
//! `labonair-snippets-host`. The composition root builds the host from the
//! active `HostManagerView`; the workspace never sees a Hosts-UI type.

use std::rc::Rc;

use gpui::App;
use labonair_hosts::HostPickerRow;

/// Live connection status for a saved host, tracked off the SSH event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

impl HostStatus {
    /// Short label for the host-list item's status pill.
    pub fn label(self) -> &'static str {
        match self {
            HostStatus::Disconnected => "offline",
            HostStatus::Connecting => "connecting\u{2026}",
            HostStatus::Connected => "connected",
            HostStatus::Failed => "failed",
        }
    }
}

/// One running port-forward, as shown in the host manager's active-tunnel
/// panel. Built by the workspace from the SSH tunnel capability snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTunnelRow {
    pub host_label: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

type HostIdsFn = Rc<dyn Fn(&App) -> Vec<String>>;
type HostNameFn = Rc<dyn Fn(&str, &App) -> Option<String>>;
type PickerRowsFn = Rc<dyn Fn(&App) -> Vec<HostPickerRow>>;
type RecentPickerRowsFn = Rc<dyn Fn(usize, &App) -> Vec<HostPickerRow>>;
type SetStatusFn = Rc<dyn Fn(&str, HostStatus, &mut App)>;
type SetActiveTunnelsFn = Rc<dyn Fn(Vec<ActiveTunnelRow>, &mut App)>;

/// Narrow composition contract between the workspace and the Hosts UI.
#[derive(Clone)]
pub struct HostView {
    host_ids: HostIdsFn,
    host_name: HostNameFn,
    jump_host_label: HostNameFn,
    picker_rows: PickerRowsFn,
    recent_picker_rows: RecentPickerRowsFn,
    set_status: SetStatusFn,
    set_active_tunnels: SetActiveTunnelsFn,
}

impl HostView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_ids: impl Fn(&App) -> Vec<String> + 'static,
        host_name: impl Fn(&str, &App) -> Option<String> + 'static,
        jump_host_label: impl Fn(&str, &App) -> Option<String> + 'static,
        picker_rows: impl Fn(&App) -> Vec<HostPickerRow> + 'static,
        recent_picker_rows: impl Fn(usize, &App) -> Vec<HostPickerRow> + 'static,
        set_status: impl Fn(&str, HostStatus, &mut App) + 'static,
        set_active_tunnels: impl Fn(Vec<ActiveTunnelRow>, &mut App) + 'static,
    ) -> Self {
        Self {
            host_ids: Rc::new(host_ids),
            host_name: Rc::new(host_name),
            jump_host_label: Rc::new(jump_host_label),
            picker_rows: Rc::new(picker_rows),
            recent_picker_rows: Rc::new(recent_picker_rows),
            set_status: Rc::new(set_status),
            set_active_tunnels: Rc::new(set_active_tunnels),
        }
    }

    /// A host that knows nothing. For headless views and tests.
    pub fn disconnected() -> Self {
        Self::new(
            |_| Vec::new(),
            |_, _| None,
            |_, _| None,
            |_| Vec::new(),
            |_, _| Vec::new(),
            |_, _, _| {},
            |_, _| {},
        )
    }

    /// All known host ids (session restore).
    pub fn host_ids(&self, cx: &App) -> Vec<String> {
        (self.host_ids)(cx)
    }

    /// Display name for a host id, if known.
    pub fn host_name(&self, host_id: &str, cx: &App) -> Option<String> {
        (self.host_name)(host_id, cx)
    }

    /// The jump-host chain label for a host id, if any.
    pub fn jump_host_label(&self, host_id: &str, cx: &App) -> Option<String> {
        (self.jump_host_label)(host_id, cx)
    }

    /// Canonical host rows for command-palette / new-tab pickers.
    pub fn picker_rows(&self, cx: &App) -> Vec<HostPickerRow> {
        (self.picker_rows)(cx)
    }

    /// Canonical recent-host rows.
    pub fn recent_picker_rows(&self, n: usize, cx: &App) -> Vec<HostPickerRow> {
        (self.recent_picker_rows)(n, cx)
    }

    /// Push a host's live connection status into the host manager panel.
    pub fn set_status(&self, host_id: &str, status: HostStatus, cx: &mut App) {
        (self.set_status)(host_id, status, cx);
    }

    /// Replace the active-tunnel snapshot in the host manager panel.
    pub fn set_active_tunnels(&self, rows: Vec<ActiveTunnelRow>, cx: &mut App) {
        (self.set_active_tunnels)(rows, cx);
    }
}
