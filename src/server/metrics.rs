use std::{collections::HashMap, time::Duration};

use crate::protocol::ClientId;

#[derive(Debug, Default)]
pub(super) struct PerformanceMetrics {
    pub(super) key_to_scene_samples: u64,
    pub(super) key_to_scene_total: Duration,
    pub(super) scene_to_client_samples: u64,
    pub(super) scene_to_client_total: Duration,
    pub(super) ipc_inbound_bytes: u64,
    pub(super) ipc_outbound_bytes: u64,
    pub(super) server_event_queue_depth: usize,
    pub(super) server_event_queue_max_depth: usize,
    pub(super) per_client_outbound_queue_depth: HashMap<ClientId, usize>,
    pub(super) per_client_outbound_queue_max_depth: HashMap<ClientId, usize>,
}

impl PerformanceMetrics {
    pub(super) fn record_server_queue_enqueue(&mut self) {
        self.server_event_queue_depth += 1;
        self.server_event_queue_max_depth = self
            .server_event_queue_max_depth
            .max(self.server_event_queue_depth);
    }

    pub(super) fn record_server_queue_dequeue(&mut self) {
        self.server_event_queue_depth = self.server_event_queue_depth.saturating_sub(1);
    }

    pub(super) fn record_outbound_enqueue(&mut self, client_id: ClientId) {
        let depth = self
            .per_client_outbound_queue_depth
            .entry(client_id)
            .or_default();
        *depth += 1;
        let max_depth = self
            .per_client_outbound_queue_max_depth
            .entry(client_id)
            .or_default();
        *max_depth = (*max_depth).max(*depth);
    }

    pub(super) fn record_outbound_dequeue(&mut self, client_id: ClientId) {
        let depth = self
            .per_client_outbound_queue_depth
            .entry(client_id)
            .or_default();
        *depth = depth.saturating_sub(1);
    }
}
