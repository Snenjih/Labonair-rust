//! In-process event plumbing that replaces Tauri's `Emitter` + `ipc::Channel`.
//!
//! [`EventBus`] is the app-wide fan-out for named events (the old
//! `window.emit("name", payload)`); [`EventChannel`] is a point-to-point
//! streaming sink (the old `ipc::Channel<T>`). The GPUI UI layer supplies the
//! concrete sinks; `T01-004` builds the typed routing on top of this.

use std::sync::Arc;

use serde::Serialize;

/// A named app-wide event with a JSON payload.
#[derive(Clone, Debug)]
pub struct AppEvent {
    pub name: String,
    pub payload: serde_json::Value,
}

/// App-wide broadcast bus. Cloneable; every clone shares one channel.
#[derive(Clone)]
pub struct EventBus {
    tx: tokio::sync::broadcast::Sender<AppEvent>,
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

    /// Subscribe to every subsequently-emitted event.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }

    /// Emit `payload` under `name`. Having no active subscribers is not an
    /// error — this is a fan-out bus, not a required sink.
    pub fn emit<S: Serialize>(&self, name: &str, payload: S) -> Result<(), String> {
        let payload = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        let _ = self.tx.send(AppEvent {
            name: name.to_string(),
            payload,
        });
        Ok(())
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
