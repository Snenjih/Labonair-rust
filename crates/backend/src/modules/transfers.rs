//! Backend adapters for the transfer capability.

use crate::{EventBus, RawEvent};
use labonair_transfers::{
    BoxFuture, TransferEvent, TransferEventError, TransferEventReceiver, TransferEventSource,
    TransferRequest, TransferResolution, TransferService,
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct BackendTransferService {
    worker: crate::modules::sftp::TransferWorkerState,
}

impl BackendTransferService {
    pub fn new(worker: crate::modules::sftp::TransferWorkerState) -> Self {
        Self { worker }
    }
}

impl TransferService for BackendTransferService {
    fn enqueue<'a>(&'a self, request: TransferRequest) -> BoxFuture<'a, Result<String, String>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            crate::modules::sftp::commands::enqueue_transfer(
                request.session_id,
                request.src_path,
                request.dest_path,
                request.direction.as_str().to_string(),
                &worker,
            )
            .await
        })
    }

    fn cancel<'a>(&'a self, job_id: String) -> BoxFuture<'a, Result<(), String>> {
        let worker = self.worker.clone();
        Box::pin(
            async move { crate::modules::sftp::commands::cancel_transfer(job_id, &worker).await },
        )
    }

    fn resolve<'a>(
        &'a self,
        job_id: String,
        resolution: TransferResolution,
    ) -> BoxFuture<'a, Result<(), String>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            let (resolution, new_name) = resolution.worker_parts();
            crate::modules::sftp::commands::resolve_conflict(
                job_id,
                resolution.to_string(),
                new_name.map(str::to_string),
                &worker,
            )
            .await
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
