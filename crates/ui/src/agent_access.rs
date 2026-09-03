//! Client-side mirror of the MCP bridge's per-tab agent-access grants
//! (T11-006).
//!
//! Port of the reference `src/modules/tabs/store/agentAccessStore.ts`. The
//! authoritative grant map lives in the Rust `McpState`
//! (`labonair_backend::modules::mcp`); this store mirrors it so the tab
//! context-menu checkbox and the header badge can read grant state
//! synchronously without round-tripping through the backend on every render.
//!
//! Grants are keyed by **tab id** (not session id) — an SSH tab can rebind to
//! a fresh `session_id` across a jump-host reconnect while remaining the same
//! tab the user granted access to.

use std::collections::BTreeMap;

use gpui::Context;
use tokio::runtime::Handle as TokioHandle;

use labonair_backend::modules::mcp::{mcp_set_session_grant, SessionKind};
use labonair_backend::App as Backend;

use labonair_notifications::{notification_center, Notification};

/// One tab the user has granted MCP agent access to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentAccessEntry {
    pub tab_id: u64,
    /// Backend session id (the SSH `ssh_id` UUID for SSH tabs, empty for
    /// local tabs which are addressed by `local_pty_id` instead).
    pub session_id: String,
    pub label: String,
}

/// Shared entity: the local grant mirror + bridge-enabled / notify flags.
pub struct AgentAccessStore {
    entries: BTreeMap<u64, AgentAccessEntry>,
    bridge_enabled: bool,
    notify_on_activity: bool,
    backend: Backend,
    tokio: TokioHandle,
}

impl AgentAccessStore {
    pub fn new(backend: Backend, tokio: TokioHandle) -> Self {
        Self {
            entries: BTreeMap::new(),
            bridge_enabled: false,
            notify_on_activity: false,
            backend,
            tokio,
        }
    }

    /// Mirror the persisted preferences (called once at startup, after they
    /// have been pushed to `McpState`).
    pub fn hydrate(
        &mut self,
        bridge_enabled: bool,
        notify_on_activity: bool,
        cx: &mut Context<Self>,
    ) {
        self.bridge_enabled = bridge_enabled;
        self.notify_on_activity = notify_on_activity;
        if !bridge_enabled {
            self.entries.clear();
        }
        cx.notify();
    }

    pub fn bridge_enabled(&self) -> bool {
        self.bridge_enabled
    }

    pub fn notify_on_activity(&self) -> bool {
        self.notify_on_activity
    }

    /// Reflect a bridge enable/disable that already happened Rust-side.
    /// Disabling drops the whole local mirror (Rust cleared its grants too).
    pub fn set_bridge_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.bridge_enabled == enabled {
            return;
        }
        self.bridge_enabled = enabled;
        if !enabled {
            self.entries.clear();
        }
        cx.notify();
    }

    pub fn set_notify_on_activity(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.notify_on_activity != on {
            self.notify_on_activity = on;
            cx.notify();
        }
    }

    pub fn is_granted(&self, tab_id: u64) -> bool {
        self.entries.contains_key(&tab_id)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Granted tabs, in tab order.
    pub fn entries(&self) -> Vec<AgentAccessEntry> {
        self.entries.values().cloned().collect()
    }

    /// Grant or revoke agent access for one tab: pushes the change to the Rust
    /// bridge (`mcp_set_session_grant`) and mirrors it locally. The local
    /// mirror is applied optimistically and rolled back with an error toast if
    /// the backend rejects the grant (e.g. the host has "Block AI Agent
    /// Access" set).
    #[allow(clippy::too_many_arguments)]
    pub fn set_grant(
        &mut self,
        tab_id: u64,
        session_id: String,
        granted: bool,
        label: String,
        kind: SessionKind,
        host_id: Option<String>,
        local_pty_id: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let app = self.backend.clone();
        let (sid, lbl, hid) = (session_id.clone(), label.clone(), host_id.clone());
        let task = self.tokio.spawn(async move {
            mcp_set_session_grant(
                tab_id.to_string(),
                sid,
                granted,
                lbl,
                kind,
                local_pty_id,
                hid,
                app.clone(),
                &app.mcp,
            )
            .await
        });

        if granted {
            self.entries.insert(
                tab_id,
                AgentAccessEntry {
                    tab_id,
                    session_id,
                    label,
                },
            );
        } else {
            self.entries.remove(&tab_id);
        }
        cx.notify();

        cx.spawn(async move |this, cx| {
            if let Ok(Err(err)) = task.await {
                let _ = this.update(cx, |this, cx| {
                    if granted {
                        this.entries.remove(&tab_id);
                        cx.notify();
                    }
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(
                            Notification::error(
                                if granted {
                                    "Failed to grant AI agent access"
                                } else {
                                    "Failed to revoke AI agent access"
                                },
                                err,
                            ),
                            cx,
                        );
                    });
                });
            }
        })
        .detach();
    }

    /// Drop the local mirror for one tab without a backend call — used when the
    /// tab is closed, or when Rust reports the grant expired
    /// (`mcp_grant_expired`: auto-revoke sweep or a host's block flag).
    pub fn clear_local(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        if self.entries.remove(&tab_id).is_some() {
            cx.notify();
        }
    }

    #[cfg(test)]
    fn insert_for_test(&mut self, tab_id: u64, label: &str) {
        self.entries.insert(
            tab_id,
            AgentAccessEntry {
                tab_id,
                session_id: String::new(),
                label: label.to_string(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};

    fn store(cx: &mut TestAppContext) -> (gpui::Entity<AgentAccessStore>, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let dir = std::env::temp_dir().join(format!("labonair-aa-{}", uuid::Uuid::new_v4()));
        let backend = Backend::new(&dir).unwrap();
        let handle = rt.handle().clone();
        let entity = cx.update(|cx| cx.new(|_| AgentAccessStore::new(backend, handle)));
        (entity, rt)
    }

    #[gpui::test]
    fn hydrate_and_disable_clears_mirror(cx: &mut TestAppContext) {
        let (entity, _rt) = store(cx);
        entity.update(cx, |s, cx| {
            s.hydrate(true, true, cx);
            assert!(s.bridge_enabled());
            assert!(s.notify_on_activity());
            s.insert_for_test(1, "web-01");
            s.insert_for_test(2, "db-01");
            assert_eq!(s.count(), 2);
            assert!(s.is_granted(1));

            // Disabling the bridge drops the whole local mirror.
            s.set_bridge_enabled(false, cx);
            assert!(!s.bridge_enabled());
            assert_eq!(s.count(), 0);
        });
    }

    #[gpui::test]
    fn clear_local_and_notify_toggle(cx: &mut TestAppContext) {
        let (entity, _rt) = store(cx);
        entity.update(cx, |s, cx| {
            s.insert_for_test(7, "t");
            s.clear_local(7, cx);
            assert!(!s.is_granted(7));

            assert!(!s.notify_on_activity());
            s.set_notify_on_activity(true, cx);
            assert!(s.notify_on_activity());
        });
    }
}
