//! SSH connection-event bridge (T17-008).
//!
//! The workspace receives only the typed SSH connection contract. The concrete
//! source is composed by the shell and may translate a platform event stream;
//! no backend event bus or aggregate application state crosses this boundary.

use gpui::{AsyncApp, Context, Task, WeakEntity};
use labonair_ssh::SshEventSource;
use std::sync::Arc;

use crate::Workspace;

/// Owns the single subscription to the SSH event source and forwards typed
/// events to the [`Workspace`]. Drop stops the loop.
pub struct SshEventBridge {
    _task: Task<()>,
}

impl SshEventBridge {
    pub fn new(
        source: Arc<dyn SshEventSource>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let mut rx = source.subscribe();
            loop {
                let Some(event) = rx.recv().await else { break };
                let delivered = workspace.update(cx, |ws, cx| {
                    ws.handle_ssh_event(event, cx);
                });
                if delivered.is_err() {
                    // Workspace entity dropped — nothing left to feed.
                    break;
                }
            }
        });
        Self { _task: task }
    }
}
