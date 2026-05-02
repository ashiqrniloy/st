use std::time::Instant;

use crate::{
    events::{PaneScene, ScenePatch, SceneUpdate, Viewport},
    protocol::{ClientId, ServerToClient},
};

use super::{
    client::{ClientConnectionState, pane_scenes},
    state::EditorServer,
};

impl EditorServer {
    pub(super) fn scene_snapshot_for_viewport(
        &self,
        viewport: Viewport,
        panes: Vec<PaneScene>,
    ) -> SceneUpdate {
        let (start_char, end_char, text) = self.visible_text_for_viewport(viewport);
        let cursor = self.editor.cursor.clamp(start_char, end_char) - start_char;
        SceneUpdate {
            background_color: self.scene_settings.background_color,
            text,
            cursor_char_index: cursor,
            cursor_visible: self.scene_settings.cursor_visible,
            panes,
        }
    }

    pub(super) fn visible_text_for_viewport(&self, viewport: Viewport) -> (usize, usize, String) {
        let total_lines = self.editor.buffer.line_count();
        let start_line = viewport.start_line.min(total_lines.saturating_sub(1));
        let end_line = viewport.end_line.max(start_line + 1).min(total_lines);
        let start_char = self.editor.buffer.line_col_to_char(start_line, 0);
        let end_char = if end_line >= total_lines {
            self.editor.buffer.char_len()
        } else {
            self.editor.buffer.line_col_to_char(end_line, 0)
        };
        let text = self.editor.buffer.slice_chars(start_char, end_char);
        (start_char, end_char, text)
    }

    pub(super) fn send_scene_snapshot_to_client(&mut self, client_id: ClientId) {
        let (viewport, panes) = self
            .clients
            .get(&client_id)
            .map(|client| (client.viewport, pane_scenes(client)))
            .unwrap_or_else(|| (Viewport::default(), vec![]));
        self.send_to_client(
            client_id,
            ServerToClient::SceneSnapshot(self.scene_snapshot_for_viewport(viewport, panes)),
        );
    }

    pub(super) fn send_scene_patch_updates(&mut self) {
        let started = Instant::now();
        let targets: Vec<(ClientId, Viewport)> = self
            .clients
            .iter()
            .filter(|(_, client)| client.state == ClientConnectionState::Connected)
            .map(|(id, client)| (*id, client.viewport))
            .collect();

        for (client_id, viewport) in targets {
            let (start_char, end_char, text) = self.visible_text_for_viewport(viewport);
            let cursor = self.editor.cursor.clamp(start_char, end_char) - start_char;
            self.send_to_client(
                client_id,
                ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate {
                    start_line: viewport.start_line,
                    end_line: viewport.end_line,
                    text,
                    cursor_char_index: cursor,
                    cursor_visible: self.scene_settings.cursor_visible,
                }),
            );
        }

        self.metrics.scene_to_client_samples += 1;
        self.metrics.scene_to_client_total += started.elapsed();
    }

    pub(super) fn send_to_client(&mut self, client_id: ClientId, message: ServerToClient) {
        if let Ok(encoded) = serde_json::to_vec(&message) {
            self.metrics.ipc_outbound_bytes += (encoded.len() + 1) as u64;
        }

        self.metrics.record_outbound_enqueue(client_id);
        let should_remove = self
            .clients
            .get(&client_id)
            .is_some_and(|client| client.tx.send(message).is_err());

        if should_remove {
            self.remove_client(client_id);
        }
    }

    pub(super) fn send_to_all(&mut self, message: ServerToClient) {
        if let Ok(encoded) = serde_json::to_vec(&message) {
            self.metrics.ipc_outbound_bytes += (encoded.len() + 1) as u64;
        }

        let dead_clients: Vec<ClientId> = self
            .clients
            .iter()
            .filter_map(|(client_id, client)| {
                if client.state == ClientConnectionState::Closing {
                    return None;
                }

                self.metrics.record_outbound_enqueue(*client_id);
                if client.tx.send(message.clone()).is_err() {
                    Some(*client_id)
                } else {
                    None
                }
            })
            .collect();

        for client_id in dead_clients {
            self.remove_client(client_id);
        }
    }
}
