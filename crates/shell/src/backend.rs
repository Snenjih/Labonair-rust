//! Composition-only state for the native application.
//!
//! This bundle exists at the shell boundary so `labonair-backend` can remain a
//! package of concrete platform adapters instead of exposing a broad
//! application facade. Feature code receives the individual capabilities it
//! needs from [`crate::bootstrap`].

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};

use labonair_backend::events::EventBus;
use labonair_backend::modules::mcp::McpState;
use labonair_backend::modules::snippets::exec::SnippetRunState;
use labonair_backend::modules::ssh::tunnels::TunnelState;
use labonair_backend::modules::ssh::{SshState, TrustState};
use labonair_persistence::Database;
use labonair_secrets::SecretsState;
use labonair_transfers::{ConflictMap, TransferSettings, TransferWorkerState, WorkerMessage};

pub struct BackendCompositionInner {
    pub(crate) events: EventBus,
    pub(crate) db: Database,
    pub(crate) secrets: Arc<SecretsState>,
    pub(crate) ssh: SshState,
    pub(crate) trust: TrustState,
    pub(crate) tunnels: TunnelState,
    pub(crate) snippet_run: Arc<SnippetRunState>,
    pub(crate) mcp: McpState,
    pub(crate) transfer: TransferWorkerState,
    worker_rx: StdMutex<Option<tokio::sync::mpsc::Receiver<WorkerMessage>>>,
}

/// Cloneable composition state owned by the application shell.
#[derive(Clone)]
pub struct BackendComposition(Arc<BackendCompositionInner>);

impl std::ops::Deref for BackendComposition {
    type Target = BackendCompositionInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BackendComposition {
    /// Builds the concrete backend capabilities rooted at `data_dir`.
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

        Ok(Self(Arc::new(BackendCompositionInner {
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

    /// Returns the raw event transport for composition-only diagnostics.
    pub fn events(&self) -> EventBus {
        self.events.clone()
    }

    /// Starts the development-only raw event trace at the composition root.
    #[cfg(debug_assertions)]
    pub fn spawn_event_logger(&self) {
        let mut receiver = self.events.subscribe();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(raw) => {
                        tracing::debug!(name = %raw.name, payload = ?raw.payload, "backend event")
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
            let ssh = self.ssh.clone();
            let events = self.events.clone();
            let conflicts = self.transfer.conflicts.clone();
            let settings = self.transfer.settings.clone();
            tokio::spawn(async move {
                labonair_backend::modules::sftp::worker::run_worker(
                    receiver, ssh, events, conflicts, settings,
                )
                .await;
            });
        }
        labonair_backend::modules::mcp::spawn_auto_revoke_sweeper(
            self.events.clone(),
            self.mcp.clone(),
        );
    }
}
