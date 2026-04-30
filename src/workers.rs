use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::{fs, sync::mpsc, task, time::timeout};

use crate::editor::{BufferId, BufferSnapshot, BufferVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackgroundRequestId(pub u64);

#[derive(Debug, Clone)]
pub enum BackgroundTaskKind {
    CpuCountChars { snapshot: BufferSnapshot },
    IoReadFile { path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct BackgroundTaskRequest {
    pub request_id: BackgroundRequestId,
    pub buffer_id: BufferId,
    pub buffer_version: BufferVersion,
    pub timeout: Option<Duration>,
    pub kind: BackgroundTaskKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskResult {
    CharCount(usize),
    FileBytes(usize),
    Cancelled,
    TimedOut,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundTaskResponse {
    pub request_id: BackgroundRequestId,
    pub buffer_id: BufferId,
    pub buffer_version: BufferVersion,
    pub result: BackgroundTaskResult,
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub request_id: BackgroundRequestId,
    token: CancellationToken,
}

impl TaskHandle {
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

#[derive(Debug, Default, Clone)]
pub struct WorkerMetrics {
    pub queue_depth: Arc<AtomicUsize>,
    pub max_queue_depth: Arc<AtomicUsize>,
    pub task_samples: Arc<AtomicU64>,
    pub task_total_nanos: Arc<AtomicU64>,
}

impl WorkerMetrics {
    fn enqueue(&self) {
        let depth = self.queue_depth.fetch_add(1, Ordering::Relaxed) + 1;
        let mut current_max = self.max_queue_depth.load(Ordering::Relaxed);
        while depth > current_max {
            match self.max_queue_depth.compare_exchange(
                current_max,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(updated) => current_max = updated,
            }
        }
    }

    fn dequeue(&self) {
        self.queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    fn observe_duration(&self, elapsed: Duration) {
        self.task_samples.fetch_add(1, Ordering::Relaxed);
        self.task_total_nanos
            .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }
}

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
        let handle = TaskHandle {
            request_id: request.request_id,
            token: token.clone(),
        };
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

                    let result = resolve_async_task_result(request.timeout, task_fut, &token).await;
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

pub fn should_discard_stale_response(
    response: &BackgroundTaskResponse,
    current: BufferVersion,
) -> bool {
    response.buffer_version != current
}

async fn resolve_task_result(
    maybe_timeout: Option<Duration>,
    fut: task::JoinHandle<BackgroundTaskResult>,
    token: &CancellationToken,
) -> BackgroundTaskResult {
    if token.is_cancelled() {
        return BackgroundTaskResult::Cancelled;
    }

    if let Some(deadline) = maybe_timeout {
        match timeout(deadline, fut).await {
            Ok(Ok(result)) => result,
            Ok(Err(err)) => BackgroundTaskResult::Failed(err.to_string()),
            Err(_) => BackgroundTaskResult::TimedOut,
        }
    } else {
        match fut.await {
            Ok(result) => result.into(),
            Err(err) => BackgroundTaskResult::Failed(err.to_string()),
        }
    }
}

async fn resolve_async_task_result<F>(
    maybe_timeout: Option<Duration>,
    fut: F,
    token: &CancellationToken,
) -> BackgroundTaskResult
where
    F: std::future::Future<Output = BackgroundTaskResult>,
{
    if token.is_cancelled() {
        return BackgroundTaskResult::Cancelled;
    }

    if let Some(deadline) = maybe_timeout {
        match timeout(deadline, fut).await {
            Ok(result) => result,
            Err(_) => BackgroundTaskResult::TimedOut,
        }
    } else {
        fut.await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::TextBuffer;

    #[tokio::test]
    async fn cpu_task_uses_snapshot_and_returns_versioned_response() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut manager = BackgroundTaskManager::new(tx);
        let mut buffer = TextBuffer::default();
        buffer.insert(0, "abc🙂");
        let request = BackgroundTaskRequest {
            request_id: manager.next_request_id(),
            buffer_id: buffer.id(),
            buffer_version: buffer.version(),
            timeout: Some(Duration::from_secs(1)),
            kind: BackgroundTaskKind::CpuCountChars {
                snapshot: buffer.snapshot(),
            },
        };

        manager.schedule(request.clone());
        let response = rx.recv().await.expect("response");

        assert_eq!(response.request_id, request.request_id);
        assert_eq!(response.buffer_id, request.buffer_id);
        assert_eq!(response.buffer_version, request.buffer_version);
        assert_eq!(response.result, BackgroundTaskResult::CharCount(4));
    }

    #[tokio::test]
    async fn can_cancel_running_task() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut manager = BackgroundTaskManager::new(tx);
        let buffer = TextBuffer::default();
        let request_id = manager.next_request_id();
        let request = BackgroundTaskRequest {
            request_id,
            buffer_id: buffer.id(),
            buffer_version: buffer.version(),
            timeout: None,
            kind: BackgroundTaskKind::CpuCountChars {
                snapshot: buffer.snapshot(),
            },
        };

        let handle = manager.schedule(request);
        handle.cancel();

        let response = rx.recv().await.expect("response");
        assert_eq!(response.result, BackgroundTaskResult::Cancelled);
    }

    #[tokio::test]
    async fn timed_out_tasks_are_observable() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut manager = BackgroundTaskManager::new(tx);
        let buffer = TextBuffer::default();
        let request = BackgroundTaskRequest {
            request_id: manager.next_request_id(),
            buffer_id: buffer.id(),
            buffer_version: buffer.version(),
            timeout: Some(Duration::from_nanos(1)),
            kind: BackgroundTaskKind::CpuCountChars {
                snapshot: buffer.snapshot(),
            },
        };

        manager.schedule(request);
        let response = rx.recv().await.expect("response");
        assert_eq!(response.result, BackgroundTaskResult::TimedOut);
    }

    #[tokio::test]
    async fn stale_results_can_be_discarded_by_buffer_version() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let manager = BackgroundTaskManager::new(tx);
        let mut buffer = TextBuffer::default();
        let request = BackgroundTaskRequest {
            request_id: manager.next_request_id(),
            buffer_id: buffer.id(),
            buffer_version: buffer.version(),
            timeout: None,
            kind: BackgroundTaskKind::CpuCountChars {
                snapshot: buffer.snapshot(),
            },
        };

        buffer.insert(0, "newer");

        let stale = BackgroundTaskResponse {
            request_id: request.request_id,
            buffer_id: request.buffer_id,
            buffer_version: request.buffer_version,
            result: BackgroundTaskResult::CharCount(0),
        };

        assert!(should_discard_stale_response(&stale, buffer.version()));
    }

    #[tokio::test]
    async fn io_task_runs_outside_server_loop_path() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut manager = BackgroundTaskManager::new(tx);

        let path = std::env::temp_dir().join(format!("st-worker-test-{}.txt", std::process::id()));
        fs::write(&path, b"hello").await.expect("write temp file");

        let request = BackgroundTaskRequest {
            request_id: manager.next_request_id(),
            buffer_id: BufferId(1),
            buffer_version: BufferVersion(0),
            timeout: Some(Duration::from_secs(1)),
            kind: BackgroundTaskKind::IoReadFile { path: path.clone() },
        };

        manager.schedule(request);
        let response = rx.recv().await.expect("response");
        assert_eq!(response.result, BackgroundTaskResult::FileBytes(5));

        let _ = fs::remove_file(path).await;
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
