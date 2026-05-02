use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::request::BackgroundRequestId;

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
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
    pub(crate) fn new(request_id: BackgroundRequestId, token: CancellationToken) -> Self {
        Self { request_id, token }
    }

    pub fn cancel(&self) {
        self.token.cancel();
    }
}
