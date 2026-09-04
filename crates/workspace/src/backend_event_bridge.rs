//! Backend → UI event bridge (T17-008).
//!
//! [`labonair_backend::EventBus`] is a `tokio::sync::broadcast` fan-out that
//! backend modules push named events onto (SFTP transfer progress, SSH connect
//! flow, MCP tab requests, FS-watcher changes, …). This entity is the single
//! place the GPUI layer subscribes to it: it runs one `cx.spawn` loop on the
//! **foreground** executor, decodes each [`RawEvent`](labonair_backend::RawEvent)
//! into a typed [`AppEvent`] / [`TransferBusEvent`], and pushes it straight into
//! the [`Workspace`] entity via `entity.update`. No `tokio::spawn`, no
//! intermediate `std::sync::mpsc`, no per-frame poll drain.
//!
//! `tokio::sync::broadcast::Receiver::recv` is runtime-agnostic (the `sync`
//! primitives do not need an active tokio runtime), so awaiting it on the GPUI
//! foreground executor is sound.

use gpui::{AsyncApp, Context, Task, WeakEntity};
use labonair_backend::{App as Backend, AppEvent};
use tokio::sync::broadcast::error::RecvError;

use crate::transfers::TransferBusEvent;
use crate::Workspace;

/// Owns the single subscription to the backend event bus and forwards decoded
/// events to the [`Workspace`]. Drop stops the loop.
pub struct BackendEventBridge {
    _task: Task<()>,
}

impl BackendEventBridge {
    pub fn new(backend: Backend, workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let mut rx = backend.events.subscribe();
            loop {
                match rx.recv().await {
                    Ok(raw) => {
                        let delivered = workspace.update(cx, |ws, cx| {
                            if let Some(tev) = TransferBusEvent::from_raw(&raw.name, &raw.payload) {
                                ws.apply_transfer_bus_event(tev, cx);
                            } else if let Some(ev) = AppEvent::from_raw(&raw) {
                                ws.handle_ssh_event(ev, cx);
                            }
                        });
                        if delivered.is_err() {
                            // Workspace entity dropped — nothing left to feed.
                            break;
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        // Resync rather than panic: the next `recv` yields the
                        // oldest still-buffered event.
                        tracing::warn!(skipped, "backend event bridge lagged; resyncing");
                        continue;
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });
        Self { _task: task }
    }
}
