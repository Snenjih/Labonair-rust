//! Local command execution owned by the snippets capability.
//!
//! The runner has no knowledge of GPUI, the application event bus, or the
//! backend facade. Consumers provide a typed event sink and decide how those
//! events are presented or bridged to another transport.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, RwLock};

use tokio::io::{AsyncBufReadExt, BufReader};

/// Which process stream produced an output event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Typed lifecycle events emitted by a local snippet process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalRunEvent {
    Output {
        run_id: String,
        data: String,
        stream: OutputStream,
    },
    Done {
        run_id: String,
        exit_code: i32,
        cancelled: bool,
    },
}

/// Event sink supplied by the owning UI or application adapter.
pub type LocalRunEventSink = Arc<dyn Fn(LocalRunEvent) + Send + Sync>;

/// Tracks local snippet processes that can be cancelled by run ID.
#[derive(Debug, Default)]
pub struct LocalRunRegistry {
    pids: RwLock<HashMap<String, u32>>,
    cancelled: RwLock<HashMap<String, ()>>,
}

impl LocalRunRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel a known process.
    ///
    /// `Ok(Some(true))` means the signal reached a live process,
    /// `Ok(Some(false))` means the process was already gone, and `Ok(None)`
    /// means this registry does not own the run ID.
    pub async fn cancel(&self, run_id: &str) -> Result<Option<bool>, String> {
        let Some(pid) = self
            .pids
            .read()
            .map_err(|e| e.to_string())?
            .get(run_id)
            .copied()
        else {
            return Ok(None);
        };

        // SAFETY: the PID was returned by tokio immediately after spawning
        // this process. A stale PID is handled through the return value.
        let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
        if result == 0 {
            self.cancelled
                .write()
                .map_err(|e| e.to_string())?
                .insert(run_id.to_string(), ());
            Ok(Some(true))
        } else {
            Ok(Some(false))
        }
    }

    fn register(&self, run_id: String, pid: u32) -> Result<(), String> {
        self.pids
            .write()
            .map_err(|e| e.to_string())?
            .insert(run_id, pid);
        Ok(())
    }

    fn finish(&self, run_id: &str) -> Result<bool, String> {
        self.pids.write().map_err(|e| e.to_string())?.remove(run_id);
        Ok(self
            .cancelled
            .write()
            .map_err(|e| e.to_string())?
            .remove(run_id)
            .is_some())
    }
}

/// Run a shell command locally and stream both output streams through `sink`.
pub async fn run_local(
    run_id: String,
    command: String,
    working_dir: Option<String>,
    registry: &LocalRunRegistry,
    sink: LocalRunEventSink,
) -> Result<(), String> {
    let trimmed = command.trim().to_string();
    if trimmed.is_empty() {
        return Err("empty command".into());
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut command = tokio::process::Command::new(&shell);
    command
        .args(["-c", &trimmed])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = working_dir.as_deref().filter(|value| !value.is_empty()) {
        command.current_dir(dir);
    }

    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().map(BufReader::new);
    let stderr = child.stderr.take().map(BufReader::new);
    if let Some(pid) = child.id() {
        registry.register(run_id.clone(), pid)?;
    }

    let output_sink = sink.clone();
    let output_run_id = run_id.clone();
    let stdout_task = if let Some(reader) = stdout {
        let mut lines = reader.lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                output_sink(LocalRunEvent::Output {
                    run_id: output_run_id.clone(),
                    data: line + "\n",
                    stream: OutputStream::Stdout,
                });
            }
        })
    } else {
        tokio::spawn(async {})
    };

    let error_sink = sink.clone();
    let error_run_id = run_id.clone();
    let stderr_task = if let Some(reader) = stderr {
        let mut lines = reader.lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                error_sink(LocalRunEvent::Output {
                    run_id: error_run_id.clone(),
                    data: line + "\n",
                    stream: OutputStream::Stderr,
                });
            }
        })
    } else {
        tokio::spawn(async {})
    };

    let _ = tokio::join!(stdout_task, stderr_task);
    let exit_code = child
        .wait()
        .await
        .map(|status| status.code().unwrap_or(-1))
        .unwrap_or(-1);
    let cancelled = registry.finish(&run_id)?;
    sink(LocalRunEvent::Done {
        run_id,
        exit_code,
        cancelled,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[tokio::test]
    async fn runs_and_streams_stdout_and_stderr() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = events.clone();
        run_local(
            "run-1".into(),
            "printf out; printf err >&2".into(),
            None,
            &LocalRunRegistry::new(),
            Arc::new(move |event| received.lock().unwrap().push(event)),
        )
        .await
        .unwrap();

        let events = events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            LocalRunEvent::Output {
                stream: OutputStream::Stdout,
                data,
                ..
            } if data == "out\n"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            LocalRunEvent::Output {
                stream: OutputStream::Stderr,
                data,
                ..
            } if data == "err\n"
        )));
        assert!(matches!(
            events.last(),
            Some(LocalRunEvent::Done {
                exit_code: 0,
                cancelled: false,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn rejects_empty_commands_without_events() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let result = run_local(
            "run-1".into(),
            "  ".into(),
            None,
            &LocalRunRegistry::new(),
            Arc::new({
                let events = events.clone();
                move |event| events.lock().unwrap().push(event)
            }),
        )
        .await;
        assert_eq!(result, Err("empty command".into()));
        assert!(events.lock().unwrap().is_empty());
    }
}
