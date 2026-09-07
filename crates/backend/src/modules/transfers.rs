//! Backend adapters for the transfer capability.

use crate::{EventBus, RawEvent};
use labonair_transfers::{
    BoxFuture, ConflictResolution, TransferEvent, TransferEventError, TransferEventReceiver,
    TransferEventSource, TransferJob, TransferRequest, TransferResolution, TransferService,
    TransferStatus, TransferWorkerState, WorkerMessage,
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct BackendTransferService {
    worker: TransferWorkerState,
}

impl BackendTransferService {
    pub fn new(worker: TransferWorkerState) -> Self {
        Self { worker }
    }
}

impl TransferService for BackendTransferService {
    fn enqueue<'a>(&'a self, request: TransferRequest) -> BoxFuture<'a, Result<String, String>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            let id = uuid::Uuid::new_v4().to_string();
            let job = TransferJob {
                id: id.clone(),
                session_id: request.session_id,
                src_path: request.src_path,
                dest_path: request.dest_path,
                direction: request.direction,
                status: TransferStatus::Queued,
                bytes_total: 0,
                bytes_transferred: 0,
                speed_bps: 0.0,
                skipped_count: 0,
            };
            worker
                .sender
                .send(WorkerMessage::Enqueue(job))
                .await
                .map_err(|error| error.to_string())?;
            Ok(id)
        })
    }

    fn cancel<'a>(&'a self, job_id: String) -> BoxFuture<'a, Result<(), String>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            worker
                .sender
                .send(WorkerMessage::Cancel(job_id))
                .await
                .map_err(|error| error.to_string())
        })
    }

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> BoxFuture<'a, Result<(), String>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            let (resolution, new_name) = resolution.worker_parts();
            let mut map = worker.conflicts.lock().await;
            if let Some(tx) = map.remove(&job_id) {
                let _ = tx.send(ConflictResolution {
                    resolution: resolution.to_string(),
                    new_name: new_name.map(str::to_string),
                });
            }
            Ok(())
        })
    }
}

pub struct BackendTransferEventSource {
    events: EventBus,
}

impl BackendTransferEventSource {
    pub fn new(events: EventBus) -> Self {
        Self { events }
    }
}

struct BackendTransferEventReceiver {
    receiver: broadcast::Receiver<RawEvent>,
}

impl TransferEventSource for BackendTransferEventSource {
    fn subscribe(&self) -> Box<dyn TransferEventReceiver> {
        Box::new(BackendTransferEventReceiver {
            receiver: self.events.subscribe(),
        })
    }
}

impl TransferEventReceiver for BackendTransferEventReceiver {
    fn recv<'a>(&'a mut self) -> BoxFuture<'a, Result<TransferEvent, TransferEventError>> {
        Box::pin(async move {
            loop {
                match self.receiver.recv().await {
                    Ok(raw) => {
                        if let Some(event) = TransferEvent::from_raw(&raw.name, &raw.payload) {
                            return Ok(event);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        return Err(TransferEventError::Lagged(skipped));
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(TransferEventError::Closed);
                    }
                }
            }
        })
    }
}
