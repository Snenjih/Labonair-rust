//! `WorkspaceLiveBridge` — the real [`labonair_ai::LiveBridge`] backing the AI
//! agent's terminal/workspace tools (`terminal_read`, `terminal_write`,
//! `suggest_command`, and relative-path resolution for `bash_run` / file ops).
//!
//! The agent tools run on Tokio blocking threads and call the bridge
//! synchronously, but the live UI state lives in a GPUI `Workspace` entity on
//! the main thread. This bridges the gap with two `Send + Sync` shared cells:
//!
//! - a **snapshot** (`LiveSnapshot`) that `AppShell` writes every render from
//!   the workspace — read-only fields (cwd, root, buffer, ssh tab id);
//! - a **command queue** that bridge writes push onto and `AppShell` drains in
//!   its update cycle (same pattern as `AiChatEvent::RunInTerminal`).

use std::sync::{Arc, Mutex};

use labonair_ai::LiveBridge;

/// Read-only view of the live UI, refreshed by `AppShell` each render.
#[derive(Debug, Default, Clone)]
pub struct LiveSnapshot {
    pub cwd: Option<String>,
    pub workspace_root: Option<String>,
    /// Recent lines of the active terminal buffer (oldest first).
    pub terminal_lines: Vec<String>,
    /// Session id of the active SSH terminal tab, if remote.
    pub ssh_tab_id: Option<String>,
    /// Whether there is an active terminal to write into.
    pub has_terminal: bool,
}

/// A pending write to the active terminal, drained by `AppShell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCommand {
    pub text: String,
    /// `true` = execute (newline appended); `false` = type at the prompt only.
    pub execute: bool,
}

/// Shared handle. Cheap to clone — all clones see the same cells.
#[derive(Clone, Default)]
pub struct WorkspaceLiveBridge {
    snapshot: Arc<Mutex<LiveSnapshot>>,
    queue: Arc<Mutex<Vec<LiveCommand>>>,
}

impl WorkspaceLiveBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the snapshot (called by `AppShell` each render).
    pub fn set_snapshot(&self, s: LiveSnapshot) {
        if let Ok(mut g) = self.snapshot.lock() {
            *g = s;
        }
    }

    /// Take every queued command (called by `AppShell` in its update cycle).
    pub fn drain_commands(&self) -> Vec<LiveCommand> {
        self.queue
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }

    fn snap(&self) -> LiveSnapshot {
        self.snapshot.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn enqueue(&self, text: &str, execute: bool) -> bool {
        if !self.snap().has_terminal {
            return false;
        }
        self.queue
            .lock()
            .map(|mut g| {
                g.push(LiveCommand {
                    text: text.to_string(),
                    execute,
                });
            })
            .is_ok()
    }
}

impl LiveBridge for WorkspaceLiveBridge {
    fn cwd(&self) -> Option<String> {
        self.snap().cwd
    }

    fn workspace_root(&self) -> Option<String> {
        self.snap().workspace_root
    }

    fn terminal_context(&self, max_lines: usize) -> Option<String> {
        let lines = self.snap().terminal_lines;
        if lines.is_empty() {
            return None;
        }
        let start = lines.len().saturating_sub(max_lines);
        Some(lines[start..].join("\n"))
    }

    fn active_ssh_tab_id(&self) -> Option<String> {
        self.snap().ssh_tab_id
    }

    fn inject_into_active_pty(&self, text: &str) -> bool {
        self.enqueue(text, false)
    }

    fn send_to_active_terminal(&self, command: &str) -> bool {
        self.enqueue(command, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reads_through() {
        let b = WorkspaceLiveBridge::new();
        assert_eq!(b.cwd(), None);
        assert!(b.terminal_context(10).is_none());
        b.set_snapshot(LiveSnapshot {
            cwd: Some("/tmp/x".into()),
            workspace_root: Some("/tmp".into()),
            terminal_lines: vec!["a".into(), "b".into(), "c".into()],
            ssh_tab_id: Some("s1".into()),
            has_terminal: true,
        });
        assert_eq!(b.cwd().as_deref(), Some("/tmp/x"));
        assert_eq!(b.workspace_root().as_deref(), Some("/tmp"));
        assert_eq!(b.active_ssh_tab_id().as_deref(), Some("s1"));
        assert_eq!(b.terminal_context(2).as_deref(), Some("b\nc"));
        assert_eq!(b.terminal_context(50).as_deref(), Some("a\nb\nc"));
    }

    #[test]
    fn writes_gate_on_active_terminal_and_queue() {
        let b = WorkspaceLiveBridge::new();
        // No terminal → refused, nothing queued.
        assert!(!b.send_to_active_terminal("ls"));
        assert!(b.drain_commands().is_empty());

        b.set_snapshot(LiveSnapshot {
            has_terminal: true,
            ..Default::default()
        });
        assert!(b.send_to_active_terminal("ls -la"));
        assert!(b.inject_into_active_pty("git status"));
        let drained = b.drain_commands();
        assert_eq!(
            drained,
            vec![
                LiveCommand {
                    text: "ls -la".into(),
                    execute: true
                },
                LiveCommand {
                    text: "git status".into(),
                    execute: false
                },
            ]
        );
        // Drained once → empty.
        assert!(b.drain_commands().is_empty());
    }
}
