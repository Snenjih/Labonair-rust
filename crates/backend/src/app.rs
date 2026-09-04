//! The process-wide application state — the in-process replacement for Tauri's
//! managed-state registry. Every ported backend command that used to take
//! `tauri::State<'_, T>` / `tauri::AppHandle` now takes `&T` and/or `&App`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};

use serde::Serialize;

use crate::events::{AppEvent, EventBus};
use crate::modules::fs::watcher::WatcherState;
use crate::modules::hosts::{self, HostsDb};
use crate::modules::mcp::McpState;
use crate::modules::pty::PtyState;
use crate::modules::secrets::SecretsState;
use crate::modules::settings::{BarItemPlacementLock, StatusBarPlacementLock};
use crate::modules::sftp::{ConflictMap, TransferSettings, TransferWorkerState, WorkerMessage};
use crate::modules::shell::ShellState;
use crate::modules::snippets::exec::SnippetRunState;
use crate::modules::ssh::tunnels::TunnelState;
use crate::modules::ssh::{SshState, TrustState};
use crate::modules::terminal_exec::TerminalExecState;

pub struct AppInner {
    pub events: EventBus,
    pub db: HostsDb,
    pub secrets: SecretsState,
    pub ssh: SshState,
    pub trust: TrustState,
    pub tunnels: TunnelState,
    pub pty: PtyState,
    pub shell: ShellState,
    pub snippet_run: SnippetRunState,
    pub terminal_exec: TerminalExecState,
    pub watcher: WatcherState,
    pub mcp: McpState,
    pub transfer: TransferWorkerState,
    pub bar_item_lock: BarItemPlacementLock,
    pub status_bar_lock: StatusBarPlacementLock,
    worker_rx: StdMutex<Option<tokio::sync::mpsc::Receiver<WorkerMessage>>>,
}

/// Cheap-to-clone handle to the whole backend. All clones share one `AppInner`.
#[derive(Clone)]
pub struct App(Arc<AppInner>);

/// Alias kept for the roadmap's `AppState` naming.
pub type AppState = App;

impl std::ops::Deref for App {
    type Target = AppInner;
    fn deref(&self) -> &AppInner {
        &self.0
    }
}

impl App {
    /// Builds the full backend state rooted at `data_dir` (SQLite lives at
    /// `<data_dir>/labonair.db`). Does not start any background task — call
    /// [`App::spawn_workers`] from inside a tokio runtime for that.
    pub fn new(data_dir: &Path) -> Result<App, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        let conn = hosts::db::initialize_db(data_dir.to_path_buf())?;
        let db = HostsDb(std::sync::Mutex::new(conn));

        let (tx, rx) = tokio::sync::mpsc::channel::<WorkerMessage>(100);
        let conflicts: ConflictMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let settings = Arc::new(TransferSettings::default());
        let transfer = TransferWorkerState {
            sender: tx,
            conflicts,
            settings,
        };

        Ok(App(Arc::new(AppInner {
            events: EventBus::new(),
            db,
            secrets: SecretsState::default(),
            ssh: SshState::default(),
            trust: TrustState::default(),
            tunnels: TunnelState::default(),
            pty: PtyState::default(),
            shell: ShellState::default(),
            snippet_run: SnippetRunState::default(),
            terminal_exec: TerminalExecState::default(),
            watcher: WatcherState::default(),
            mcp: McpState::default(),
            transfer,
            bar_item_lock: BarItemPlacementLock::default(),
            status_bar_lock: StatusBarPlacementLock::default(),
            worker_rx: StdMutex::new(Some(rx)),
        })))
    }

    /// Spawns the SFTP transfer worker and the MCP auto-revoke sweeper. Safe
    /// to call once; a second call is a no-op for the transfer worker.
    pub fn spawn_workers(&self) {
        if let Some(rx) = self.0.worker_rx.lock().unwrap().take() {
            let ssh = self.ssh.clone();
            let app = self.clone();
            let conflicts = self.transfer.conflicts.clone();
            let settings = self.transfer.settings.clone();
            tokio::spawn(async move {
                crate::modules::sftp::worker::run_worker(rx, ssh, app, conflicts, settings).await;
            });
        }
        crate::modules::mcp::spawn_auto_revoke_sweeper(self.clone(), self.mcp.clone());
    }

    /// Emit an app-wide event (replaces `window.emit`).
    pub fn emit<S: Serialize>(&self, name: &str, payload: S) -> Result<(), String> {
        self.events.emit(name, payload)
    }

    /// Emit a typed app-wide event.
    pub fn emit_event(&self, event: AppEvent) -> Result<(), String> {
        self.events.emit_event(event)
    }
}
