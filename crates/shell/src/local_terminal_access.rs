//! Shell-composed adapter exposing the terminal registry to UI-free MCP code.

use std::sync::Arc;

use labonair_mcp_core::{BoxFuture, LocalTerminalAccess, LocalTerminalOutputReceiver};
use labonair_terminal::TerminalRegistry;

pub(crate) struct TerminalRegistryAccess {
    registry: Arc<TerminalRegistry>,
}

impl TerminalRegistryAccess {
    pub(crate) fn new(registry: Arc<TerminalRegistry>) -> Self {
        Self { registry }
    }

    fn session(&self, session_id: &str) -> Result<labonair_terminal::SessionHandle, String> {
        let id = session_id
            .parse::<u64>()
            .map_err(|_| format!("invalid local terminal session id: {session_id}"))?;
        self.registry
            .handle(id)
            .ok_or_else(|| format!("local terminal session not found: {session_id}"))
    }
}

struct TerminalOutputReceiver {
    receiver: tokio::sync::broadcast::Receiver<Vec<u8>>,
}

impl LocalTerminalOutputReceiver for TerminalOutputReceiver {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Result<Vec<u8>, String>> {
        Box::pin(async move {
            self.receiver.recv().await.map_err(|error| match error {
                tokio::sync::broadcast::error::RecvError::Closed => {
                    "local terminal output stream closed".to_string()
                }
                tokio::sync::broadcast::error::RecvError::Lagged(skipped) => {
                    format!("local terminal output stream lagged by {skipped} chunks")
                }
            })
        })
    }
}

impl LocalTerminalAccess for TerminalRegistryAccess {
    fn write(&self, session_id: &str, data: String) -> Result<(), String> {
        self.session(session_id)?.write(data.as_bytes())
    }

    fn subscribe(&self, session_id: &str) -> Result<Box<dyn LocalTerminalOutputReceiver>, String> {
        Ok(Box::new(TerminalOutputReceiver {
            receiver: self.session(session_id)?.subscribe_agent_output()?,
        }))
    }
}
