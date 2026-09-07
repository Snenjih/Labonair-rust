//! UI-free transfer capability contracts and lifecycle registry.
//!
//! The transfer module owns job identity, lifecycle state, progress, conflict
//! state, and history. Transport workers implement [`TransferService`], while
//! UI consumers receive immutable [`TransferSnapshot`] values from
//! [`TransferRegistry`]. No backend, GPUI, or SFTP implementation details are
//! allowed here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

impl TransferDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Download => "download",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    Queued,
    Running,
    Paused,
    Cancelled,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferJob {
    pub id: String,
    pub session_id: String,
    pub src_path: String,
    pub dest_path: String,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub bytes_total: u64,
    pub bytes_transferred: u64,
    pub speed_bps: f64,
    #[serde(default)]
    pub skipped_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRequest {
    pub session_id: String,
    pub src_path: String,
    pub dest_path: String,
    pub direction: TransferDirection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferResolution {
    Overwrite,
    Skip,
    SkipAll,
    Rename(String),
    Abort,
}

impl TransferResolution {
    pub fn worker_parts(&self) -> (&'static str, Option<&str>) {
        match self {
            Self::Overwrite => ("overwrite", None),
            Self::Skip => ("skip", None),
            Self::SkipAll => ("skip_all", None),
            Self::Rename(name) => ("rename", Some(name.as_str())),
            Self::Abort => ("abort", None),
        }
    }

    pub fn all_policy(&self) -> Option<Self> {
        match self {
            Self::Overwrite => Some(Self::Overwrite),
            Self::Skip => Some(Self::Skip),
            Self::SkipAll => Some(Self::SkipAll),
            Self::Rename(_) | Self::Abort => None,
        }
    }
}

/// Operations exposed by the transport worker to feature consumers.
pub trait TransferService: Send + Sync {
    fn enqueue<'a>(&'a self, request: TransferRequest) -> BoxFuture<'a, Result<String, String>>;

    fn cancel<'a>(&'a self, job_id: String) -> BoxFuture<'a, Result<(), String>>;

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> BoxFuture<'a, Result<(), String>>;
}

/// Messages sent to a transport-backed transfer worker.
#[derive(Debug)]
pub enum WorkerMessage {
    Enqueue(TransferJob),
    Cancel(String),
    ResolveConflict {
        job_id: String,
        resolution: String,
        new_name: Option<String>,
    },
    SessionReconnected(String),
}

pub type ConflictMap = Arc<Mutex<HashMap<String, oneshot::Sender<ConflictResolution>>>>;

#[derive(Debug)]
pub struct ConflictResolution {
    pub resolution: String,
    pub new_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransferStepPayload {
    pub job_id: String,
    pub ts: i64,
    pub message: String,
}

/// Shared queue state owned by Transfers and consumed by its transport
/// adapter. The concrete worker remains free to depend on SSH/SFTP internals.
#[derive(Clone)]
pub struct TransferWorkerState {
    pub sender: mpsc::Sender<WorkerMessage>,
    pub conflicts: ConflictMap,
    pub settings: Arc<TransferSettings>,
}

pub struct TransferSettings {
    pub max_concurrent: AtomicUsize,
    pub chunk_size: AtomicUsize,
    pub default_conflict_resolution: std::sync::Mutex<String>,
    pub on_folder_file_error: std::sync::Mutex<String>,
}

impl Default for TransferSettings {
    fn default() -> Self {
        Self {
            max_concurrent: AtomicUsize::new(2),
            chunk_size: AtomicUsize::new(65536),
            default_conflict_resolution: std::sync::Mutex::new("ask".to_string()),
            on_folder_file_error: std::sync::Mutex::new("ask".to_string()),
        }
    }
}

/// Error returned by a typed transfer event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferEventError {
    Lagged(u64),
    Closed,
}

/// Backend-independent source of worker lifecycle events.
pub trait TransferEventSource: Send + Sync {
    fn subscribe(&self) -> Box<dyn TransferEventReceiver>;
}

pub trait TransferEventReceiver: Send {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Result<TransferEvent, TransferEventError>>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransferEvent {
    Progress(TransferJob),
    Step {
        job_id: String,
        timestamp: i64,
        message: String,
    },
    Conflict {
        job_id: String,
        source_path: String,
        destination_path: String,
    },
    FileError {
        job_id: String,
        path: String,
        error: String,
    },
}

impl TransferEvent {
    /// Decode the legacy event-bus wire shape at the capability boundary.
    /// Producers can later publish this enum directly without changing the
    /// registry or its UI consumers.
    pub fn from_raw(name: &str, payload: &serde_json::Value) -> Option<Self> {
        match name {
            "transfer_progress" => serde_json::from_value::<TransferJob>(payload.clone())
                .ok()
                .map(Self::Progress),
            "transfer_step" => Some(Self::Step {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                timestamp: payload.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
                message: payload.get("message")?.as_str()?.to_string(),
            }),
            "file_conflict" => Some(Self::Conflict {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                source_path: payload.get("src_path")?.as_str()?.to_string(),
                destination_path: payload.get("dest_path")?.as_str()?.to_string(),
            }),
            "file_error" => Some(Self::FileError {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                path: payload.get("path")?.as_str()?.to_string(),
                error: payload.get("error")?.as_str()?.to_string(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferConflict {
    pub source_path: String,
    pub destination_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFileError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferSnapshot {
    pub job: TransferJob,
    pub steps: Vec<TransferStep>,
    pub conflict: Option<TransferConflict>,
    pub file_error: Option<TransferFileError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferStep {
    pub timestamp: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveRequest {
    pub job_id: String,
    pub resolution: TransferResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryUpdate {
    None,
    Completed {
        session_id: String,
        direction: TransferDirection,
    },
    Resolve(Vec<ResolveRequest>),
}

struct Record {
    job: TransferJob,
    steps: Vec<TransferStep>,
    conflict: Option<TransferConflict>,
    file_error: Option<TransferFileError>,
}

/// The single owner of transfer lifecycle state.
#[derive(Default)]
pub struct TransferRegistry {
    records: Vec<Record>,
    sticky: std::collections::HashMap<String, TransferResolution>,
}

impl TransferRegistry {
    pub fn apply(&mut self, event: TransferEvent) -> RegistryUpdate {
        match event {
            TransferEvent::Progress(job) => {
                let completed = matches!(job.status, TransferStatus::Completed);
                let completion = (job.session_id.clone(), job.direction);
                if let Some(record) = self.records.iter_mut().find(|r| r.job.id == job.id) {
                    record.job = job;
                    if !matches!(record.job.status, TransferStatus::Queued) {
                        record.conflict = None;
                        record.file_error = None;
                    }
                } else {
                    self.records.insert(
                        0,
                        Record {
                            job,
                            steps: Vec::new(),
                            conflict: None,
                            file_error: None,
                        },
                    );
                }
                if completed {
                    RegistryUpdate::Completed {
                        session_id: completion.0,
                        direction: completion.1,
                    }
                } else {
                    RegistryUpdate::None
                }
            }
            TransferEvent::Step {
                job_id,
                timestamp,
                message,
            } => {
                if let Some(record) = self.records.iter_mut().find(|r| r.job.id == job_id) {
                    record.steps.push(TransferStep { timestamp, message });
                }
                RegistryUpdate::None
            }
            TransferEvent::Conflict {
                job_id,
                source_path,
                destination_path,
            } => {
                let Some(record) = self.records.iter_mut().find(|r| r.job.id == job_id) else {
                    return RegistryUpdate::None;
                };
                if let Some(policy) = self
                    .sticky
                    .get(&record.job.session_id)
                    .filter(|policy| {
                        matches!(
                            policy,
                            TransferResolution::Overwrite | TransferResolution::Skip
                        )
                    })
                    .cloned()
                {
                    return RegistryUpdate::Resolve(vec![ResolveRequest {
                        job_id,
                        resolution: policy,
                    }]);
                }
                record.conflict = Some(TransferConflict {
                    source_path,
                    destination_path,
                });
                record.job.status = TransferStatus::Paused;
                RegistryUpdate::None
            }
            TransferEvent::FileError {
                job_id,
                path,
                error,
            } => {
                if let Some(policy) = self
                    .sticky
                    .get(&self.session_of(&job_id).unwrap_or_default())
                    .filter(|policy| matches!(policy, TransferResolution::SkipAll))
                    .cloned()
                {
                    return RegistryUpdate::Resolve(vec![ResolveRequest {
                        job_id,
                        resolution: policy,
                    }]);
                }
                if let Some(record) = self.records.iter_mut().find(|r| r.job.id == job_id) {
                    record.file_error = Some(TransferFileError { path, error });
                    record.job.status = TransferStatus::Paused;
                }
                RegistryUpdate::None
            }
        }
    }

    pub fn resolve(&mut self, job_id: &str, resolution: TransferResolution) -> RegistryUpdate {
        let Some(record) = self.records.iter_mut().find(|r| r.job.id == job_id) else {
            return RegistryUpdate::None;
        };
        let session_id = record.job.session_id.clone();
        let mut requests = vec![ResolveRequest {
            job_id: job_id.to_string(),
            resolution: resolution.clone(),
        }];
        if let Some(policy) = resolution.all_policy() {
            self.sticky.insert(session_id.clone(), policy.clone());
            let siblings: Vec<String> = if matches!(
                policy,
                TransferResolution::Overwrite | TransferResolution::Skip
            ) {
                self.records
                    .iter()
                    .filter(|r| {
                        r.job.session_id == session_id && r.job.id != job_id && r.conflict.is_some()
                    })
                    .map(|r| r.job.id.clone())
                    .collect()
            } else {
                Vec::new()
            };
            requests.extend(siblings.into_iter().map(|sibling| ResolveRequest {
                job_id: sibling,
                resolution: policy.clone(),
            }));
        }
        for request in &requests {
            if let Some(record) = self.records.iter_mut().find(|r| r.job.id == request.job_id) {
                record.conflict = None;
                record.file_error = None;
                record.job.status = TransferStatus::Running;
            }
        }
        RegistryUpdate::Resolve(requests)
    }

    pub fn snapshot(&self) -> Vec<TransferSnapshot> {
        self.records
            .iter()
            .map(|record| TransferSnapshot {
                job: record.job.clone(),
                steps: record.steps.clone(),
                conflict: record.conflict.clone(),
                file_error: record.file_error.clone(),
            })
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| {
                matches!(
                    record.job.status,
                    TransferStatus::Queued | TransferStatus::Running
                )
            })
            .count()
    }

    fn session_of(&self, job_id: &str) -> Option<String> {
        self.records
            .iter()
            .find(|record| record.job.id == job_id)
            .map(|record| record.job.session_id.clone())
    }

    pub fn clear_completed(&mut self) {
        self.records.retain(|record| {
            !matches!(
                record.job.status,
                TransferStatus::Completed | TransferStatus::Cancelled | TransferStatus::Failed(_)
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(id: &str, status: TransferStatus) -> TransferJob {
        TransferJob {
            id: id.into(),
            session_id: "session".into(),
            src_path: "/source".into(),
            dest_path: "/destination".into(),
            direction: TransferDirection::Upload,
            status,
            bytes_total: 10,
            bytes_transferred: 0,
            speed_bps: 0.0,
            skipped_count: 0,
        }
    }

    #[test]
    fn progress_is_retained_and_completion_is_reported() {
        let mut registry = TransferRegistry::default();
        assert_eq!(
            registry.apply(TransferEvent::Progress(job("a", TransferStatus::Queued))),
            RegistryUpdate::None
        );
        assert_eq!(registry.active_count(), 1);
        assert_eq!(
            registry.apply(TransferEvent::Progress(job("a", TransferStatus::Completed))),
            RegistryUpdate::Completed {
                session_id: "session".into(),
                direction: TransferDirection::Upload,
            }
        );
    }

    #[test]
    fn all_resolution_fans_out_to_paused_siblings() {
        let mut registry = TransferRegistry::default();
        for id in ["a", "b"] {
            registry.apply(TransferEvent::Progress(job(id, TransferStatus::Running)));
            registry.apply(TransferEvent::Conflict {
                job_id: id.into(),
                source_path: "/source".into(),
                destination_path: "/destination".into(),
            });
        }
        let RegistryUpdate::Resolve(requests) = registry.resolve("a", TransferResolution::Skip)
        else {
            panic!("expected conflict resolution requests");
        };
        assert_eq!(requests.len(), 2);
        assert_eq!(registry.active_count(), 2);
    }

    #[test]
    fn skip_all_is_scoped_to_file_errors_not_conflicts() {
        let mut registry = TransferRegistry::default();
        for id in ["a", "b"] {
            registry.apply(TransferEvent::Progress(job(id, TransferStatus::Running)));
            registry.apply(TransferEvent::FileError {
                job_id: id.into(),
                path: "/file".into(),
                error: "read failed".into(),
            });
        }

        let RegistryUpdate::Resolve(requests) = registry.resolve("a", TransferResolution::SkipAll)
        else {
            panic!("expected file-error resolution request");
        };
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].resolution, TransferResolution::SkipAll);

        let update = registry.apply(TransferEvent::FileError {
            job_id: "b".into(),
            path: "/next-file".into(),
            error: "read failed".into(),
        });
        assert_eq!(
            update,
            RegistryUpdate::Resolve(vec![ResolveRequest {
                job_id: "b".into(),
                resolution: TransferResolution::SkipAll,
            }])
        );
    }

    #[test]
    fn raw_event_decoder_rejects_unknown_names() {
        assert!(TransferEvent::from_raw("unknown", &serde_json::json!({})).is_none());
    }
}
