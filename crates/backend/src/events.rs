//! In-process event plumbing that replaces Tauri's `Emitter` + `ipc::Channel`.
//!
//! [`EventBus`] is the app-wide fan-out for named events (the old
//! `window.emit("name", payload)`); [`EventChannel`] is a point-to-point
//! streaming sink (the old `ipc::Channel<T>`). The GPUI UI layer supplies the
//! concrete sinks.
//!
//! Two layers sit on the bus:
//! * [`RawEvent`] — the wire form actually carried by the broadcast channel: a
//!   name plus a JSON payload. Every ported backend call site emits this via
//!   `App::emit("name", payload)`, unchanged from the mechanical Tauri strip.
//! * [`AppEvent`] — a typed enum over the well-known backend → UI events. New
//!   code can emit it with [`EventBus::emit_event`] and subscribers can recover
//!   it from a [`RawEvent`] with [`AppEvent::from_raw`] (`T01-004`).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// A named app-wide event with a JSON payload — the form carried on the bus.
#[derive(Clone, Debug)]
pub struct RawEvent {
    pub name: String,
    pub payload: serde_json::Value,
}

/// App-wide broadcast bus. Cloneable; every clone shares one channel.
#[derive(Clone)]
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<RawEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel(1024);
        Self { tx }
    }

    /// Subscribe to every subsequently-emitted event. Decode typed events with
    /// [`AppEvent::from_raw`].
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RawEvent> {
        self.tx.subscribe()
    }

    /// Emit `payload` under `name`. Having no active subscribers is not an
    /// error — this is a fan-out bus, not a required sink.
    pub fn emit<S: Serialize>(&self, name: &str, payload: S) -> Result<(), String> {
        let payload = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        let _ = self.tx.send(RawEvent {
            name: name.to_string(),
            payload,
        });
        Ok(())
    }

    /// Emit a typed [`AppEvent`]. Serializes to the same flat `(name, payload)`
    /// wire form the equivalent string call sites use, so typed and string
    /// emitters interoperate on one channel.
    pub fn emit_event(&self, event: AppEvent) -> Result<(), String> {
        let name = event.event_name();
        // Externally-tagged: `{ "variant": { ..fields } }` — unwrap to the flat
        // field object so the payload matches the string call sites' shape.
        let payload = serde_json::to_value(&event)
            .map_err(|e| e.to_string())?
            .as_object()
            .and_then(|m| m.values().next().cloned())
            .unwrap_or(serde_json::Value::Null);
        let _ = self.tx.send(RawEvent {
            name: name.to_string(),
            payload,
        });
        Ok(())
    }
}

/// Typed backend → UI events. On the wire each variant is emitted as a flat
/// field object under its [`event_name`](AppEvent::event_name) — the same shape
/// the equivalent string call sites already emit; the name carries the
/// discriminant, exactly as Tauri's event name did.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppEvent {
    // Transfer / SFTP
    TransferProgress {
        transfer_id: String,
        file_name: String,
        transferred: u64,
        total: u64,
        direction: String,
    },
    TransferCompleted {
        transfer_id: String,
        success: bool,
        #[serde(default)]
        error: Option<String>,
    },
    FileConflict {
        transfer_id: String,
        file_name: String,
        conflict_type: String,
    },

    // SSH
    SshSessionEstablished {
        session_id: String,
        #[serde(default)]
        default_path: Option<String>,
    },
    SshAuthRequired {
        session_id: String,
        prompt_message: String,
        is_2fa: bool,
    },
    SshPassphraseRequired {
        session_id: String,
    },
    SshKnownHostsWarning {
        session_id: String,
        fingerprint: String,
        host: String,
        is_mismatch: bool,
    },
    SshConnectionLost {
        session_id: String,
    },

    // Filesystem
    DirChanged {
        path: String,
    },

    // Native menus
    MenuActivated {
        action: String,
    },

    // MCP bridge
    McpOpenTabRequest {
        request_id: String,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        host_id: Option<String>,
    },
    McpCloseTabRequest {
        request_id: String,
        #[serde(default)]
        session_id: Option<String>,
    },
    McpGrantExpired {
        tab_id: String,
    },
    /// The bridge listener failed to come up (e.g. port already in use). The
    /// backend has already rolled `enabled` back to `false`; the UI surfaces
    /// this as an error toast.
    McpServerError {
        message: String,
    },
    /// One of the four MCP action tools (`run_command` / `send_keys` /
    /// `open_tab` / `close_tab`) touched a granted tab — feeds the optional
    /// "notify on agent activity" preference (UI in T11-006).
    McpActivity {
        label: String,
        action: String,
        detail: String,
    },
}

impl AppEvent {
    /// The bus event name for this variant — matches the string the equivalent
    /// pre-existing call site emits.
    pub fn event_name(&self) -> &'static str {
        match self {
            AppEvent::TransferProgress { .. } => "transfer_progress",
            AppEvent::TransferCompleted { .. } => "transfer_completed",
            AppEvent::FileConflict { .. } => "file_conflict",
            AppEvent::SshSessionEstablished { .. } => "session_established",
            AppEvent::SshAuthRequired { .. } => "auth_required",
            AppEvent::SshPassphraseRequired { .. } => "passphrase_required",
            AppEvent::SshKnownHostsWarning { .. } => "known_hosts_warning",
            AppEvent::SshConnectionLost { .. } => "ssh_connection_lost",
            AppEvent::DirChanged { .. } => "fs:dir-changed",
            AppEvent::MenuActivated { .. } => "menu:activated",
            AppEvent::McpOpenTabRequest { .. } => "mcp_open_tab_request",
            AppEvent::McpCloseTabRequest { .. } => "mcp_close_tab_request",
            AppEvent::McpGrantExpired { .. } => "mcp_grant_expired",
            AppEvent::McpServerError { .. } => "mcp_server_error",
            AppEvent::McpActivity { .. } => "mcp_activity",
        }
    }

    /// Recover a typed event from a raw bus event, or `None` if the name is not
    /// a known typed variant or the payload does not fit it.
    pub fn from_raw(raw: &RawEvent) -> Option<Self> {
        let value = |v: serde_json::Value| serde_json::from_value::<AppEvent>(v).ok();
        // Re-tag the flat payload with the variant name so serde can pick the
        // right arm, then deserialize.
        let payload = raw.payload.as_object()?.clone();
        let variant = match raw.name.as_str() {
            "transfer_progress" => "transfer_progress",
            "transfer_completed" => "transfer_completed",
            "file_conflict" => "file_conflict",
            "session_established" => "ssh_session_established",
            "auth_required" => "ssh_auth_required",
            "passphrase_required" => "ssh_passphrase_required",
            "known_hosts_warning" => "ssh_known_hosts_warning",
            "ssh_connection_lost" => "ssh_connection_lost",
            "fs:dir-changed" => "dir_changed",
            "menu:activated" => "menu_activated",
            "mcp_open_tab_request" => "mcp_open_tab_request",
            "mcp_close_tab_request" => "mcp_close_tab_request",
            "mcp_grant_expired" => "mcp_grant_expired",
            "mcp_server_error" => "mcp_server_error",
            "mcp_activity" => "mcp_activity",
            _ => return None,
        };
        value(serde_json::json!({ variant: payload }))
    }
}

/// Point-to-point streaming sink — the in-process replacement for Tauri's
/// `ipc::Channel<T>`. The concrete callback is supplied by the UI layer.
pub struct EventChannel<T> {
    sink: Arc<dyn Fn(T) -> Result<(), String> + Send + Sync>,
}

impl<T> Clone for EventChannel<T> {
    fn clone(&self) -> Self {
        Self {
            sink: self.sink.clone(),
        }
    }
}

impl<T> EventChannel<T> {
    pub fn new(f: impl Fn(T) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self { sink: Arc::new(f) }
    }

    /// A sink that discards every value — used as a placeholder before the UI
    /// wires a real one.
    pub fn null() -> Self {
        Self {
            sink: Arc::new(|_| Ok(())),
        }
    }

    pub fn send(&self, value: T) -> Result<(), String> {
        (self.sink)(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn typed_emit_round_trips_through_from_raw() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.emit_event(AppEvent::DirChanged {
            path: "/tmp/x".into(),
        })
        .unwrap();

        let raw = rx.recv().await.unwrap();
        assert_eq!(raw.name, "fs:dir-changed");
        match AppEvent::from_raw(&raw) {
            Some(AppEvent::DirChanged { path }) => assert_eq!(path, "/tmp/x"),
            other => panic!("unexpected decode: {other:?}"),
        }
    }

    #[test]
    fn from_raw_decodes_string_call_site_payload_with_extra_fields() {
        // `ssh/pty.rs` emits `{session_id, reason}` — the extra `reason` is
        // ignored, the typed variant still resolves.
        let raw = RawEvent {
            name: "ssh_connection_lost".into(),
            payload: serde_json::json!({ "session_id": "s1", "reason": "eof" }),
        };
        assert!(matches!(
            AppEvent::from_raw(&raw),
            Some(AppEvent::SshConnectionLost { session_id }) if session_id == "s1"
        ));
    }

    #[test]
    fn from_raw_decodes_mcp_open_tab_request_without_path() {
        // `mcp::server::open_tab` emits only `{request_id, host_id}` — the
        // optional `path` must not make this fail to decode.
        let raw = RawEvent {
            name: "mcp_open_tab_request".into(),
            payload: serde_json::json!({ "request_id": "r1", "host_id": "h1" }),
        };
        assert!(matches!(
            AppEvent::from_raw(&raw),
            Some(AppEvent::McpOpenTabRequest { request_id, host_id, path })
                if request_id == "r1" && host_id.as_deref() == Some("h1") && path.is_none()
        ));
    }

    #[test]
    fn from_raw_decodes_mcp_close_tab_request_and_server_error() {
        let close = RawEvent {
            name: "mcp_close_tab_request".into(),
            payload: serde_json::json!({ "request_id": "r2", "session_id": "s9" }),
        };
        assert!(matches!(
            AppEvent::from_raw(&close),
            Some(AppEvent::McpCloseTabRequest { request_id, session_id })
                if request_id == "r2" && session_id.as_deref() == Some("s9")
        ));
        let err = RawEvent {
            name: "mcp_server_error".into(),
            payload: serde_json::json!({ "message": "port in use" }),
        };
        assert!(matches!(
            AppEvent::from_raw(&err),
            Some(AppEvent::McpServerError { message }) if message == "port in use"
        ));
    }

    #[test]
    fn from_raw_returns_none_for_unknown_name() {
        let raw = RawEvent {
            name: "totally_unknown".into(),
            payload: serde_json::json!({}),
        };
        assert!(AppEvent::from_raw(&raw).is_none());
    }
}
