use std::{collections::HashMap, sync::atomic::{AtomicU64, Ordering}, time::{Duration, Instant}};

use tokio::{fs, sync::mpsc, task};

use super::{
    cancellation::{CancellationToken, TaskHandle},
    metrics::WorkerMetrics,
    request::{
        BackgroundRequestId, BackgroundTaskKind, BackgroundTaskRequest, BackgroundTaskResponse,
        BackgroundTaskResult,
    },
    resolve::{resolve_async_task_result, resolve_task_result},
};

#[derive(Debug)]
pub struct BackgroundTaskManager {
    next_request_id: AtomicU64,
    pending: HashMap<BackgroundRequestId, TaskHandle>,
    response_tx: mpsc::UnboundedSender<BackgroundTaskResponse>,
    pub metrics: WorkerMetrics,
}

impl BackgroundTaskManager {
    pub fn new(response_tx: mpsc::UnboundedSender<BackgroundTaskResponse>) -> Self {
        Self {
            next_request_id: AtomicU64::new(1),
            pending: HashMap::new(),
            response_tx,
            metrics: WorkerMetrics::default(),
        }
    }

    pub fn next_request_id(&self) -> BackgroundRequestId {
        BackgroundRequestId(self.next_request_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn schedule(&mut self, request: BackgroundTaskRequest) -> TaskHandle {
        let token = CancellationToken::new();
        let handle = TaskHandle::new(request.request_id, token.clone());
        self.pending.insert(request.request_id, handle.clone());

        let response_tx = self.response_tx.clone();
        let metrics = self.metrics.clone();
        metrics.enqueue();

        match request.kind.clone() {
            BackgroundTaskKind::CpuCountChars { snapshot } => {
                task::spawn(async move {
                    let started = Instant::now();
                    let token_for_task = token.clone();
                    let task_fut = task::spawn_blocking(move || {
                        for _ in 0..3 {
                            if token_for_task.is_cancelled() {
                                return BackgroundTaskResult::Cancelled;
                            }
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        BackgroundTaskResult::CharCount(snapshot.rope.len_chars())
                    });

                    let result = resolve_task_result(request.timeout, task_fut, &token).await;
                    let _ = response_tx.send(BackgroundTaskResponse {
                        request_id: request.request_id,
                        buffer_id: request.buffer_id,
                        buffer_version: request.buffer_version,
                        result,
                    });
                    metrics.observe_duration(started.elapsed());
                    metrics.dequeue();
                });
            }
            BackgroundTaskKind::IoReadFile { path } => {
                task::spawn(async move {
                    let started = Instant::now();
                    let token_for_task = token.clone();
                    let task_fut = async move {
                        if token_for_task.is_cancelled() {
                            return BackgroundTaskResult::Cancelled;
                        }
                        match fs::read(path).await {
                            Ok(bytes) => BackgroundTaskResult::FileBytes(bytes.len()),
                            Err(err) => BackgroundTaskResult::Failed(err.to_string()),
                        }
                    };

                    let result =
                        resolve_async_task_result(request.timeout, task_fut, &token).await;
                    let _ = response_tx.send(BackgroundTaskResponse {
                        request_id: request.request_id,
                        buffer_id: request.buffer_id,
                        buffer_version: request.buffer_version,
                        result,
                    });
                    metrics.observe_duration(started.elapsed());
                    metrics.dequeue();
                });
            }
        }

        handle
    }

    #[allow(dead_code)]
    pub fn cancel(&mut self, request_id: BackgroundRequestId) -> bool {
        if let Some(handle) = self.pending.remove(&request_id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn complete(&mut self, response: &BackgroundTaskResponse) {
        self.pending.remove(&response.request_id);
    }
}
