//! Composition-only state for the native application.
//!
//! This bundle exists at the application composition boundary. Feature code
//! receives the individual capabilities it needs from [`crate::bootstrap`].

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};

use labonair_events::EventBus;
use labonair_mcp_server::McpState;
use labonair_persistence::Database;
use labonair_secrets::SecretsState;
use labonair_snippets_ssh::exec::SnippetRunState;
use labonair_ssh_transport::tunnels::TunnelState;
use labonair_ssh_transport::{SshState, TrustState};
use labonair_transfers::{ConflictMap, TransferSettings, TransferWorkerState, WorkerMessage};

struct AppCompositionInner {
    events: EventBus,
    db: Database,
    secrets: Arc<SecretsState>,
    ssh: SshState,
    trust: TrustState,
    tunnels: TunnelState,
    snippet_run: Arc<SnippetRunState>,
    mcp: McpState,
    transfer: TransferWorkerState,
    worker_rx: StdMutex<Option<tokio::sync::mpsc::Receiver<WorkerMessage>>>,
}

/// Cloneable composition state owned by the application shell.
///
/// This is a construction bundle, not a service facade: it exposes each
/// concrete capability through an explicit accessor so a feature constructor
/// can only be handed the one capability it needs, never the whole bundle
/// (R08-006 — the old blanket `Deref<Target = AppCompositionInner>` is gone).
#[derive(Clone)]
pub struct AppComposition(Arc<AppCompositionInner>);

impl AppComposition {
    /// The application event transport.
    pub fn events(&self) -> &EventBus {
        &self.0.events
    }
    /// The shared foundation database handle.
    pub fn db(&self) -> &Database {
        &self.0.db
    }
    /// The secret store.
    pub fn secrets(&self) -> &Arc<SecretsState> {
        &self.0.secrets
    }
    /// SSH connection state.
    pub fn ssh(&self) -> &SshState {
        &self.0.ssh
    }
    /// SSH host-key trust state.
    pub fn trust(&self) -> &TrustState {
        &self.0.trust
    }
    /// SSH tunnel state.
    pub fn tunnels(&self) -> &TunnelState {
        &self.0.tunnels
    }
    /// Silent snippet-run state.
    pub fn snippet_run(&self) -> &Arc<SnippetRunState> {
        &self.0.snippet_run
    }
    /// MCP server state.
    pub fn mcp(&self) -> &McpState {
        &self.0.mcp
    }
    /// Transfer worker channel + shared queue state.
    pub fn transfer(&self) -> &TransferWorkerState {
        &self.0.transfer
    }
}

impl AppComposition {
    /// Builds the concrete application capabilities rooted at `data_dir`.
    pub fn new(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
        let connection = labonair_persistence::initialize_database(data_dir.to_path_buf())?;
        let db = Database(Arc::new(std::sync::Mutex::new(connection)));

        let (sender, receiver) = tokio::sync::mpsc::channel::<WorkerMessage>(100);
        let conflicts: ConflictMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let settings = Arc::new(TransferSettings::default());
        let transfer = TransferWorkerState {
            sender,
            conflicts,
            settings,
        };

        Ok(Self(Arc::new(AppCompositionInner {
            events: EventBus::new(),
            db,
            secrets: Arc::new(SecretsState::new(data_dir.to_path_buf())),
            ssh: SshState::default(),
            trust: TrustState::default(),
            tunnels: TunnelState::default(),
            snippet_run: Arc::new(SnippetRunState::default()),
            mcp: McpState::default(),
            transfer,
            worker_rx: StdMutex::new(Some(receiver)),
        })))
    }

    /// Starts the development-only raw event trace at the composition root.
    #[cfg(debug_assertions)]
    pub fn spawn_event_logger(&self) {
        let mut receiver = self.0.events.subscribe();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(raw) => {
                        tracing::debug!(name = %raw.name, payload = ?raw.payload, "application event")
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(skipped, "event bus subscriber lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    /// Starts the concrete background workers once at the composition root.
    pub fn spawn_workers(&self) {
        if let Some(receiver) = self.0.worker_rx.lock().unwrap().take() {
            let ssh = self.0.ssh.clone();
            let events = self.0.events.clone();
            let conflicts = self.0.transfer.conflicts.clone();
            let settings = self.0.transfer.settings.clone();
            tokio::spawn(async move {
                labonair_transfers_ssh::run_worker(receiver, ssh, events, conflicts, settings)
                    .await;
            });
        }
        labonair_mcp_server::spawn_auto_revoke_sweeper(self.0.events.clone(), self.0.mcp.clone());
    }
}
