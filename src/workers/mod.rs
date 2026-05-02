#![allow(unused_imports)]

mod cancellation;
mod manager;
mod metrics;
mod request;
mod resolve;
mod stale;

#[cfg(test)]
mod tests;

pub use cancellation::{CancellationToken, TaskHandle};
pub use manager::BackgroundTaskManager;
pub use metrics::WorkerMetrics;
pub use request::{
    BackgroundRequestId, BackgroundTaskKind, BackgroundTaskRequest, BackgroundTaskResponse,
    BackgroundTaskResult,
};
pub use stale::should_discard_stale_response;
