use std::time::Duration;

use tokio::{fs, sync::mpsc};

use crate::editor::{BufferId, BufferVersion, TextBuffer};

use super::{
    manager::BackgroundTaskManager,
    request::{BackgroundTaskKind, BackgroundTaskRequest, BackgroundTaskResponse, BackgroundTaskResult},
    stale::should_discard_stale_response,
};

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
