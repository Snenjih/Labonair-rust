use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Tracks in-flight snippet runs so `snippet_run_cancel` can reach back into
/// them — SSH runs by the split-off write half of their exec channel (same
/// `Arc<ChannelWriteHalf<..>>`, no-lock-needed shape as `ssh::PtyChannelState`,
/// since all of its methods take `&self`). `cancelled` records which
/// `run_id`s were cancelled so the owning task can report a distinct
/// "cancelled" outcome instead of treating the resulting kill/close as a
/// plain failure — only ever inserted *after* a cancel attempt is confirmed
/// to have actually reached a still-running process/channel, so a cancel
/// racing a process's own natural exit can't mislabel a successful run.
#[derive(Default)]
pub struct SnippetRunState {
    ssh: RwLock<HashMap<String, Arc<russh::ChannelWriteHalf<russh::client::Msg>>>>,
    cancelled: RwLock<HashSet<String>>,
}

/// Cancels a running snippet started via `snippet_run_local` or
/// `snippet_run_ssh`. Kills the local child process or closes the SSH exec
/// channel, whichever is registered under `run_id`. The owning command
/// notices the resulting exit/close and emits `snippet_run_done` with
/// `cancelled: true` so the frontend can show a distinct "Cancelled" status.
///
/// The standalone local runner owns its process registry and signals the
/// process directly by PID. This adapter only selects the local or SSH path.
pub async fn snippet_run_cancel(run_id: String, state: &SnippetRunState) -> Result<(), String> {
    let ssh_write_half = state.ssh.read().unwrap().get(&run_id).cloned();
    if let Some(write_half) = ssh_write_half {
        // Same natural-completion race as the local path: only mark
        // cancelled if this call is the one that actually closed the
        // channel, not one that raced a channel already closed by the
        // command finishing on its own.
        write_half.close().await.map_err(|e| e.to_string())?;
        state.cancelled.write().unwrap().insert(run_id);
        return Ok(());
    }

    Err("no running snippet with this run id".to_string())
}

/// Runs a command on an existing SSH session and streams output as Tauri events.
/// Requires an active SSH session (opened via ssh_connect). Uses a fresh exec
/// channel so it does not disturb the interactive PTY.
pub async fn snippet_run_ssh(
    app: crate::App,
    run_id: String,
    session_id: String,
    command: String,
    ssh_state: &crate::modules::ssh::SshState,
    run_state: &SnippetRunState,
) -> Result<(), String> {
    let session = crate::get_session_arc!(ssh_state, &session_id);

    let channel = session
        .handle
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| e.to_string())?;

    // Split so the write half (needed by `snippet_run_cancel` to close the
    // channel from another task) can be registered while this task keeps
    // exclusive ownership of the read half's message loop below.
    let (mut read_half, write_half) = channel.split();
    let write_half = Arc::new(write_half);
    run_state
        .ssh
        .write()
        .unwrap()
        .insert(run_id.clone(), write_half);

    // One loop interleaves stdout/stderr as they arrive off the same message
    // stream, streaming BOTH live via `snippet_run_output` as each message
    // comes in — unlike the old sequential `read()`-loop-then-full-stderr-dump
    // pattern, which only streamed stdout live and buffered stderr until the
    // stdout side had fully closed.
    let mut exit_code: i32 = -1;
    while let Some(msg) = read_half.wait().await {
        match msg {
            russh::ChannelMsg::Data { data } => {
                let chunk = String::from_utf8_lossy(&data).into_owned();
                let _ = app.emit(
                    "snippet_run_output",
                    serde_json::json!({ "runId": run_id, "data": chunk, "stream": "stdout" }),
                );
            }
            russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                let chunk = String::from_utf8_lossy(&data).into_owned();
                let _ = app.emit(
                    "snippet_run_output",
                    serde_json::json!({ "runId": run_id, "data": chunk, "stream": "stderr" }),
                );
            }
            russh::ChannelMsg::ExtendedData { .. } => {}
            // `ExitStatus` arrives *after* `Eof` (and before `Close`), so
            // breaking on Eof/Close here would discard it and leave
            // `exit_code` stuck at -1 forever — matches russh's own
            // client_exec_simple.rs example, which explicitly warns against
            // leaving the loop early. `read_half.wait()` returns `None` on its
            // own once the channel is fully closed, ending the loop naturally
            // (including when `snippet_run_cancel` force-closes it).
            russh::ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            _ => {}
        }
    }

    run_state.ssh.write().unwrap().remove(&run_id);
    let cancelled = run_state.cancelled.write().unwrap().remove(&run_id);

    let _ = app.emit(
        "snippet_run_done",
        serde_json::json!({ "runId": run_id, "exitCode": exit_code, "cancelled": cancelled }),
    );

    Ok(())
}
