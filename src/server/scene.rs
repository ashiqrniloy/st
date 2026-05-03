use crate::{
    events::{PaneScene, ScenePatch, SceneUpdate, Viewport},
    protocol::{ClientId, ServerToClient},
};

use super::{
    client::{ClientConnectionState, pane_scenes},
    state::EditorServer,
};

#[derive(Debug, Clone)]
pub(super) struct TextEditOp {
    pub(super) start_char: usize,
    pub(super) end_char: usize,
    pub(super) replacement: String,
    pub(super) base_version: u64,
    pub(super) new_version: u64,
}

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
            buffer_id: self.editor.buffer.id().0,
            buffer_version: self.editor.buffer.version().0,
            viewport_start_line: viewport.start_line,
            viewport_end_line: viewport.end_line,
            cursor_char_index: cursor,
            cursor_visible: self.scene_settings.cursor_visible,
            panes,
        }
    }

    pub(super) fn visible_text_for_viewport(&self, viewport: Viewport) -> (usize, usize, String) {
        let (start_char, end_char) = self.visible_char_range_for_viewport(viewport);
        let text = self.editor.buffer.slice_chars(start_char, end_char);
        (start_char, end_char, text)
    }

    pub(super) fn visible_char_range_for_viewport(&self, viewport: Viewport) -> (usize, usize) {
        let total_lines = self.editor.buffer.line_count();
        let start_line = viewport.start_line.min(total_lines.saturating_sub(1));
        let end_line = viewport.end_line.max(start_line + 1).min(total_lines);
        let start_char = self.editor.buffer.line_col_to_char(start_line, 0);
        let end_char = if end_line >= total_lines {
            self.editor.buffer.char_len()
        } else {
            self.editor.buffer.line_col_to_char(end_line, 0)
        };
        (start_char, end_char)
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
                    buffer_id: self.editor.buffer.id().0,
                    buffer_version: self.editor.buffer.version().0,
                    cursor_char_index: cursor,
                    cursor_visible: self.scene_settings.cursor_visible,
                }),
            );
        }
    }

    pub(super) fn send_text_edit_patch_updates(&mut self, edit: &TextEditOp) {
        let targets: Vec<(ClientId, Viewport)> = self
            .clients
            .iter()
            .filter(|(_, client)| client.state == ClientConnectionState::Connected)
            .map(|(id, client)| (*id, client.viewport))
            .collect();

        for (client_id, viewport) in targets {
            let (start_char, end_char) = self.visible_char_range_for_viewport(viewport);

            let edit_in_view = edit.start_char >= start_char
                && edit.end_char <= end_char
                && self.editor.cursor >= start_char
                && self.editor.cursor <= end_char;

            if !edit_in_view {
                self.send_scene_snapshot_to_client(client_id);
                continue;
            }

            let cursor = self.editor.cursor - start_char;
            self.send_to_client(
                client_id,
                ServerToClient::ScenePatch(ScenePatch::TextEditPatch {
                    buffer_id: self.editor.buffer.id().0,
                    base_version: edit.base_version,
                    new_version: edit.new_version,
                    replace_start_char: edit.start_char - start_char,
                    replace_end_char: edit.end_char - start_char,
                    replacement: edit.replacement.clone(),
                    cursor_char_index: cursor,
                    cursor_visible: self.scene_settings.cursor_visible,
                }),
            );
        }
    }

    pub(super) fn send_to_client(&mut self, client_id: ClientId, message: ServerToClient) {
        let Some(client) = self.clients.get(&client_id) else {
            return;
        };
        match client.tx.try_send(message.clone()) {
            Ok(()) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                self.remove_client(client_id);
            }
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                let drop_safe = matches!(
                    message,
                    ServerToClient::ScenePatch(ScenePatch::CursorUpdate { .. })
                        | ServerToClient::ScenePatch(ScenePatch::SelectionUpdate { .. })
                );
                if drop_safe {
                    self.queue_stats.inc_coalesced();
                } else {
                    self.queue_stats.inc_dropped();
                    self.remove_client(client_id);
                }
            }
        }
    }

    pub(super) fn send_to_all(&mut self, message: ServerToClient) {
        let dead_clients: Vec<ClientId> = self
            .clients
            .iter()
            .filter_map(|(client_id, client)| {
                if client.state == ClientConnectionState::Closing {
                    return None;
                }

                if client.tx.try_send(message.clone()).is_err() {
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
