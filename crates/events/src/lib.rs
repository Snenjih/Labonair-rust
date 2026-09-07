//! Small in-process event primitives shared by capability adapters.
//!
//! `EventBus` is intentionally only a transport primitive. Capability owners
//! translate raw events into their own typed contracts at the adapter edge;
//! this crate does not define product events or application state.

use std::sync::Arc;

use serde::Serialize;

/// A named event with a JSON payload carried by the in-process bus.
#[derive(Clone, Debug)]
pub struct RawEvent {
    pub name: String,
    pub payload: serde_json::Value,
}

/// Cloneable broadcast transport for adapter-level events.
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

    /// Subscribe to events emitted after the subscription is created.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<RawEvent> {
        self.tx.subscribe()
    }

    /// Emit a serializable payload. A missing subscriber is not an error.
    pub fn emit<S: Serialize>(&self, name: &str, payload: S) -> Result<(), String> {
        let payload = serde_json::to_value(payload).map_err(|error| error.to_string())?;
        let _ = self.tx.send(RawEvent {
            name: name.to_string(),
            payload,
        });
        Ok(())
    }
}

/// Point-to-point streaming sink for adapter-to-UI output.
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

    /// A sink that discards values until a concrete UI sink is wired.
    pub fn null() -> Self {
        Self {
            sink: Arc::new(|_| Ok(())),
        }
    }

    pub fn send(&self, value: T) -> Result<(), String> {
        (self.sink)(value)
    }
}
