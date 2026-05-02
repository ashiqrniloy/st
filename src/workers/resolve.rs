use std::time::Duration;

use tokio::{task, time::timeout};

use super::{
    cancellation::CancellationToken,
    request::BackgroundTaskResult,
};

pub(crate) async fn resolve_task_result(
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
            Ok(result) => result,
            Err(err) => BackgroundTaskResult::Failed(err.to_string()),
        }
    }
}

pub(crate) async fn resolve_async_task_result<F>(
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
