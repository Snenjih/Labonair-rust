use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::Manager;

use super::mcp::osc133::Osc133Capture;

/// Which underlying terminal backend a call targets. Deliberately a separate
/// type from `crate::modules::mcp::SessionKind` even though it is
/// structurally identical — this module is built to compile and be reasoned
/// about fully independently of `modules::mcp` (the MCP bridge is a
/// separately shipped, already-tested feature that this module must not
/// risk regressing), at the cost of duplicating a two-variant enum.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalKind {
    Ssh,
    Local,
}

/// Keyed async-lock serializing `terminal_exec_run_command` calls against a
/// single terminal — structurally identical to
/// `modules::mcp::McpState::lock_for`, but a wholly separate map/state. A
/// BYOK `bash_run` and an MCP `run_command` hitting the same physical tab do
/// NOT serialize against each other (see module-level doc below) — keeping
/// this decoupled from `McpState` was an explicit requirement for this
/// feature, to avoid touching the already-shipped MCP bridge.
///
/// Known limitation: if an MCP-connected external agent and a BYOK chat
/// session both write into the same tab around the same time, their
/// independent OSC-133 captures can interleave and misattribute output/exit
/// codes to the wrong caller. The underlying shell writes themselves are
/// unaffected (both go through the same real PTY). This mirrors a
/// pre-existing property of MCP's own capture (already not safe against a
/// human typing into the same tab concurrently) — this just adds a second,
/// uncoordinated observer. Left unaddressed for v1; fixing it properly would
/// mean either sharing one lock/observer between MCP and BYOK (which
/// requires touching `McpState`) or a real cross-feature tab-ownership
/// model, neither of which is in scope here.
#[derive(Clone, Default)]
pub struct TerminalExecState {
    locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl TerminalExecState {
    fn lock_for(&self, key: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut map = self.locks.lock().unwrap();
        map.entry(key.to_string()).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
    }
}

#[derive(serde::Serialize)]
pub struct TerminalExecResult {
    pub output: String,
    pub exit_code: Option<i32>,
    pub still_running: bool,
}

/// Writes into an SSH session's interactive PTY write-half. Intentionally
/// duplicated from `modules::mcp::server`'s private `write_to_ssh_session`
/// helper (~9 lines) rather than widening that function's visibility and
/// importing it — the real source of truth both copies mirror is the
/// `ssh_pty_write` command itself (`modules::ssh::pty`), so the duplication
/// carries little drift risk, and it keeps this module's compile graph (and
/// future PR diffs) fully independent of `modules::mcp`.
async fn write_to_ssh(app: &tauri::AppHandle, session_id: &str, data: String) -> Result<(), String> {
    let state = app.state::<crate::modules::ssh::SshState>();
    let session = crate::get_session_arc!(state, session_id);
    let write_half = {
        let guard = session.pty.lock().await;
        guard.as_ref().map(|p| p.write_half.clone())
    }
    .ok_or_else(|| "no pty channel open".to_string())?;
    write_half.data_bytes(data).await.map_err(|e| e.to_string())
}

async fn write_to_target(
    app: &tauri::AppHandle,
    kind: TerminalKind,
    session_id: Option<&str>,
    local_pty_id: Option<u32>,
    data: String,
) -> Result<(), String> {
    match kind {
        TerminalKind::Ssh => write_to_ssh(app, session_id.ok_or("missing session_id for ssh target")?, data).await,
        TerminalKind::Local => {
            let pty_state = app.state::<crate::modules::pty::PtyState>();
            crate::modules::pty::write_raw(&pty_state, local_pty_id.ok_or("missing local_pty_id for local target")?, &data)
        }
    }
}

/// One reusable async loop: subscribe to the target's raw-output tap,
/// optionally write a command first, then poll-with-timeout through an
/// `Osc133Capture` until either the command finishes or the deadline is hit.
/// Generalizes the logic currently duplicated between `mcp::server`'s
/// `run_command` and `read_output` handlers — `write_first: None` is the
/// `read_output`-shaped peek, `write_first: Some(cmd)` is the
/// `run_command`-shaped execute-and-wait.
async fn capture_loop(
    app: &tauri::AppHandle,
    kind: TerminalKind,
    session_id: Option<&str>,
    local_pty_id: Option<u32>,
    write_first: Option<String>,
    timeout: Duration,
) -> Result<TerminalExecResult, String> {
    let mut ssh_rx = None;
    let mut local_rx = None;
    match kind {
        TerminalKind::Ssh => {
            let session_id = session_id.ok_or("missing session_id for ssh target")?;
            let ssh_state = app.state::<crate::modules::ssh::SshState>();
            let session = crate::get_session_arc!(ssh_state, session_id);
            ssh_rx = Some(session.agent_tap.subscribe());
        }
        TerminalKind::Local => {
            let pty_id = local_pty_id.ok_or("missing local_pty_id for local target")?;
            let pty_state = app.state::<crate::modules::pty::PtyState>();
            local_rx = Some(crate::modules::pty::subscribe_agent_tap(&pty_state, pty_id)?);
        }
    }

    if let Some(data) = write_first {
        write_to_target(app, kind, session_id, local_pty_id, data).await?;
    }

    let deadline = Instant::now() + timeout;
    let mut capture = Osc133Capture::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(TerminalExecResult { output: capture.clean_output().to_string(), exit_code: None, still_running: true });
        }
        let recv = async {
            if let Some(rx) = ssh_rx.as_mut() {
                rx.recv().await.map(|s| s.into_bytes())
            } else if let Some(rx) = local_rx.as_mut() {
                rx.recv().await
            } else {
                unreachable!("exactly one of ssh_rx/local_rx is always set")
            }
        };
        match tokio::time::timeout(remaining, recv).await {
            Ok(Ok(chunk)) => {
                capture.feed(&chunk);
                if let Some(code) = capture.finished() {
                    return Ok(TerminalExecResult { output: capture.clean_output().to_string(), exit_code: code, still_running: false });
                }
            }
            Ok(Err(_)) => {
                return Err("output stream interrupted (tab closed, or capture fell behind)".to_string());
            }
            Err(_elapsed) => {
                return Ok(TerminalExecResult { output: capture.clean_output().to_string(), exit_code: None, still_running: true });
            }
        }
    }
}

/// Runs `command` visibly inside the given terminal's real PTY (local or
/// SSH) and waits (up to `timeout_ms`, default 30s) for it to finish via the
/// OSC-133 shell-integration marker, or returns partial output with
/// `still_running: true` if it doesn't finish in time. Serialized per-target
/// via `TerminalExecState` so overlapping calls against the same tab never
/// interleave their writes/captures.
#[tauri::command]
pub async fn terminal_exec_run_command(
    kind: TerminalKind,
    session_id: Option<String>,
    local_pty_id: Option<u32>,
    command: String,
    timeout_ms: Option<u64>,
    app: tauri::AppHandle,
    state: tauri::State<'_, TerminalExecState>,
) -> Result<TerminalExecResult, String> {
    let lock_key = match kind {
        TerminalKind::Ssh => format!("ssh:{}", session_id.as_deref().ok_or("missing session_id for ssh target")?),
        TerminalKind::Local => format!("local:{}", local_pty_id.ok_or("missing local_pty_id for local target")?),
    };
    let lock = state.lock_for(&lock_key);
    let _guard = lock.lock().await;
    capture_loop(
        &app,
        kind,
        session_id.as_deref(),
        local_pty_id,
        Some(format!("{command}\n")),
        Duration::from_millis(timeout_ms.unwrap_or(30_000)),
    )
    .await
}

/// Peeks at live output from a terminal without writing anything — used to
/// check on a command that a prior `terminal_exec_run_command` call reported
/// as `still_running: true`. Deliberately takes no lock (mirrors MCP's
/// `read_output`, which is also unlocked) so a peek is never blocked by an
/// in-flight `run_command` against the same target.
#[tauri::command]
pub async fn terminal_exec_peek_output(
    kind: TerminalKind,
    session_id: Option<String>,
    local_pty_id: Option<u32>,
    wait_ms: Option<u64>,
    app: tauri::AppHandle,
) -> Result<TerminalExecResult, String> {
    capture_loop(
        &app,
        kind,
        session_id.as_deref(),
        local_pty_id,
        None,
        Duration::from_millis(wait_ms.unwrap_or(1000)),
    )
    .await
}

/// Sends raw keystrokes/control characters into a terminal without waiting
/// for a command-finished marker — for answering interactive prompts (sudo
/// password, y/n) or interrupting a stuck command (e.g. the byte 0x03 for
/// Ctrl+C). No lock, no capture — fire-and-forget, mirrors MCP's
/// `send_keys`.
#[tauri::command]
pub async fn terminal_exec_send_keys(
    kind: TerminalKind,
    session_id: Option<String>,
    local_pty_id: Option<u32>,
    data: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    write_to_target(&app, kind, session_id.as_deref(), local_pty_id, data).await
}
