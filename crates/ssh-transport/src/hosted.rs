//! Sessions hosted outside this process (the detached session host).
//!
//! Remote git, snippet runs and the MCP bridge read the in-process session map
//! directly, which cannot see a session the daemon owns. They ask this module
//! first and fall back to their in-process path when it answers `None`, so
//! app-bound sessions behave exactly as before. The composition root installs
//! the router once; without one (app mode, tests) every lookup is `None`.

use std::sync::{Arc, OnceLock};

use labonair_ssh::{SshHostedAccess, SshSessionId};

static ACCESS: OnceLock<Arc<dyn SshHostedAccess>> = OnceLock::new();

/// Install the access once at composition; later calls are ignored.
pub fn install(access: Arc<dyn SshHostedAccess>) {
    let _ = ACCESS.set(access);
}

/// The access, when `session_id` is owned by the detached host.
pub fn for_session(session_id: &str) -> Option<&'static Arc<dyn SshHostedAccess>> {
    ACCESS.get().filter(|access| access.is_hosted(session_id))
}

/// Run `command` (with optional stdin) on a hosted session as
/// `(stdout, stderr, exit code)`; `None` when the session is not hosted.
pub async fn try_exec(
    session_id: &str,
    command: String,
    stdin: Option<Vec<u8>>,
) -> Option<Result<(Vec<u8>, Vec<u8>, i32), String>> {
    let access = for_session(session_id)?;
    let result = access
        .exec(SshSessionId::new(session_id), command, stdin)
        .await;
    Some(result.map(|out| (out.stdout, out.stderr, out.exit_code)))
}

/// Live PTY output of a hosted session (the MCP `agent_tap`).
pub fn agent_tap(session_id: &str) -> Option<tokio::sync::broadcast::Receiver<String>> {
    for_session(session_id)?.agent_tap(session_id)
}

/// Type `data` into a hosted session's PTY; `None` when it is not hosted.
pub async fn try_write(session_id: &str, data: String) -> Option<Result<(), String>> {
    let access = for_session(session_id)?;
    Some(access.write_pty(SshSessionId::new(session_id), data).await)
}
