//! MCP bridge event delivery into the workspace.
//!
//! The workspace consumes the typed MCP event contract. The concrete source
//! is composed by the shell and owns any translation from platform events.

use gpui::{AsyncApp, Context, Task, WeakEntity};
use labonair_mcp_core::McpEventSource;
use std::sync::Arc;

use crate::Workspace;

/// Owns the single subscription to the MCP event source and forwards typed
/// events to the [`Workspace`]. Drop stops the loop.
pub struct McpEventBridge {
    _task: Task<()>,
}

impl McpEventBridge {
    pub fn new(
        source: Arc<dyn McpEventSource>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let task = cx.spawn(async move |_this, cx: &mut AsyncApp| {
            let mut rx = source.subscribe();
            loop {
                let Some(event) = rx.recv().await else { break };
                let delivered = workspace.update(cx, |ws, cx| {
                    ws.handle_mcp_event(event, cx);
                });
                if delivered.is_err() {
                    break;
                }
            }
        });
        Self { _task: task }
    }
}
