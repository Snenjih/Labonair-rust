use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use labonair_snippets::exec::{
    ExecutionFuture, OutputStream, SnippetRunEvent, SnippetRunEventSink, SshCommandExecutor,
};

use labonair_events::EventBus;

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

/// SSH implementation of the snippets capability's execution contract. The
/// panel only sees `SshCommandExecutor`; russh and session storage remain
/// private to this adapter.
pub struct SshSnippetExecutor {
    ssh_state: labonair_ssh_transport::SshState,
    run_state: Arc<SnippetRunState>,
}

impl SshSnippetExecutor {
    pub fn new(
        ssh_state: labonair_ssh_transport::SshState,
        run_state: Arc<SnippetRunState>,
    ) -> Self {
        Self {
            ssh_state,
            run_state,
        }
    }
}

impl SshCommandExecutor for SshSnippetExecutor {
    fn execute(
        &self,
        run_id: String,
        session_id: String,
        command: String,
        sink: SnippetRunEventSink,
    ) -> ExecutionFuture {
        Box::pin(run_ssh_with_sink(
            run_id,
            session_id,
            command,
            self.ssh_state.clone(),
            self.run_state.clone(),
            sink,
        ))
    }

    fn cancel(&self, run_id: String) -> ExecutionFuture {
        let run_state = self.run_state.clone();
        Box::pin(async move { snippet_run_cancel(run_id, &run_state).await })
    }
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
    run_id: String,
    session_id: String,
    command: String,
    ssh_state: &labonair_ssh_transport::SshState,
    run_state: &Arc<SnippetRunState>,
    events: EventBus,
) -> Result<(), String> {
    let sink: SnippetRunEventSink = Arc::new(move |event| match event {
        SnippetRunEvent::Output {
            run_id,
            data,
            stream,
        } => {
            let _ = events.emit(
                "snippet_run_output",
                serde_json::json!({
                    "runId": run_id,
                    "data": data,
                    "stream": match stream {
                        OutputStream::Stdout => "stdout",
                        OutputStream::Stderr => "stderr",
                    }
                }),
            );
        }
        SnippetRunEvent::Done {
            run_id,
            exit_code,
            cancelled,
        } => {
            let _ = events.emit(
                "snippet_run_done",
                serde_json::json!({
                    "runId": run_id,
                    "exitCode": exit_code,
                    "cancelled": cancelled
                }),
            );
        }
    });

    run_ssh_with_sink(
        run_id,
        session_id,
        command,
        ssh_state.clone(),
        run_state.clone(),
        sink,
    )
    .await
}

async fn run_ssh_with_sink(
    run_id: String,
    session_id: String,
    command: String,
    ssh_state: labonair_ssh_transport::SshState,
    run_state: Arc<SnippetRunState>,
    sink: SnippetRunEventSink,
) -> Result<(), String> {
    let session = labonair_ssh_transport::get_session_arc!(&ssh_state, &session_id);
    let channel = session
        .handle
        .channel_open_session()
        .await
        .map_err(|e| e.to_string())?;
    channel
        .exec(true, command)
        .await
        .map_err(|e| e.to_string())?;

    let (mut read_half, write_half) = channel.split();
    run_state
        .ssh
        .write()
        .unwrap()
        .insert(run_id.clone(), Arc::new(write_half));

    let mut exit_code: i32 = -1;
    while let Some(msg) = read_half.wait().await {
        match msg {
            russh::ChannelMsg::Data { data } => sink(SnippetRunEvent::Output {
                run_id: run_id.clone(),
                data: String::from_utf8_lossy(&data).into_owned(),
                stream: OutputStream::Stdout,
            }),
            russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                sink(SnippetRunEvent::Output {
                    run_id: run_id.clone(),
                    data: String::from_utf8_lossy(&data).into_owned(),
                    stream: OutputStream::Stderr,
                });
            }
            russh::ChannelMsg::ExtendedData { .. } => {}
            russh::ChannelMsg::ExitStatus { exit_status } => exit_code = exit_status as i32,
            _ => {}
        }
    }

    run_state.ssh.write().unwrap().remove(&run_id);
    let cancelled = run_state.cancelled.write().unwrap().remove(&run_id);
    sink(SnippetRunEvent::Done {
        run_id,
        exit_code,
        cancelled,
    });
    Ok(())
}
