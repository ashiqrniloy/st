use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct WorkerMetrics {
    pub queue_depth: Arc<AtomicUsize>,
    pub max_queue_depth: Arc<AtomicUsize>,
    pub task_samples: Arc<AtomicU64>,
    pub task_total_nanos: Arc<AtomicU64>,
}

impl WorkerMetrics {
    pub(crate) fn enqueue(&self) {
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

    pub(crate) fn dequeue(&self) {
        self.queue_depth.fetch_sub(1, Ordering::Relaxed);
    }

    pub(crate) fn observe_duration(&self, elapsed: Duration) {
        self.task_samples.fetch_add(1, Ordering::Relaxed);
        self.task_total_nanos
            .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
    }
}
