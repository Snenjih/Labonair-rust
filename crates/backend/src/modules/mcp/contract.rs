//! Backend adapter for the UI-free MCP session-access contract.

use labonair_mcp_core::{
    BoxFuture, McpSessionAccessService, McpTabOperationService, SessionGrantRequest, TabOpResult,
};

use super::{mcp_set_session_grant, mcp_tab_op_response};

/// Bridges aggregate backend MCP state to the narrow service consumed by the
/// workspace's agent-access mirror. Construction stays in the shell; the
/// workspace never receives the backend application just to grant a tab.
#[derive(Clone)]
pub struct BackendMcpSessionAccess {
    app: crate::App,
}

impl BackendMcpSessionAccess {
    pub fn new(app: crate::App) -> Self {
        Self { app }
    }
}

impl McpSessionAccessService for BackendMcpSessionAccess {
    fn set_session_grant(&self, request: SessionGrantRequest) -> BoxFuture<'_, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(async move {
            mcp_set_session_grant(
                request.tab_id,
                request.session_id,
                request.granted,
                request.label,
                request.kind,
                request.local_pty_id,
                request.host_id,
                app.clone(),
                &app.mcp,
            )
            .await
        })
    }
}

impl McpTabOperationService for BackendMcpSessionAccess {
    fn respond_tab_operation(
        &self,
        request_id: String,
        result: TabOpResult,
    ) -> BoxFuture<'_, Result<(), String>> {
        let app = self.app.clone();
        Box::pin(async move { mcp_tab_op_response(request_id, result, &app.mcp).await })
    }
}
