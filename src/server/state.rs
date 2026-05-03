use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::mpsc;

use crate::{
    commands::{CommandId, CommandRegistry},
    configuration::{DEFAULT_CURSOR_VISIBLE, DEFAULT_EDITOR_BACKGROUND_COLOR},
    editor::EditorState,
    protocol::{ClientId, ServerToClient},
};

use super::{
    client::{ClientConnection, ClientConnectionState},
    keymap::Keymap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EditorSceneSettings {
    pub(super) background_color: u32,
    pub(super) cursor_visible: bool,
}

impl Default for EditorSceneSettings {
    fn default() -> Self {
        Self {
            background_color: DEFAULT_EDITOR_BACKGROUND_COLOR,
            cursor_visible: DEFAULT_CURSOR_VISIBLE,
        }
    }
}

#[derive(Debug, Default)]
pub struct QueueStats {
    pub outbound_dropped: Arc<AtomicU64>,
    pub outbound_coalesced: Arc<AtomicU64>,
}

impl QueueStats {
    pub fn inc_dropped(&self) {
        self.outbound_dropped.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_coalesced(&self) {
        self.outbound_coalesced.fetch_add(1, Ordering::Relaxed);
    }
}

pub struct EditorServer {
    // Performance guardrail: Rust owns typing/cursor/edit/scene hot paths.
    pub(super) editor: EditorState,
    pub(super) scene_settings: EditorSceneSettings,
    pub(super) command_registry: CommandRegistry,
    pub(super) keymap: Keymap,
    pub(super) next_client_id: u64,
    pub(super) clients: HashMap<ClientId, ClientConnection>,
    pub(super) queue_stats: QueueStats,
}

impl Default for EditorServer {
    fn default() -> Self {
        crate::documentation::validate_builtin_docs()
            .expect("builtin documentation registration should be valid");

        let mut keymap = Keymap::default();
        keymap
            .bind_builtin(
                "backspace",
                CommandId::new("editor.backspace").expect("builtin id"),
                "Backspace",
            )
            .expect("builtin keybinding should be valid");

        Self {
            editor: EditorState::default(),
            scene_settings: EditorSceneSettings::default(),
            command_registry: CommandRegistry::with_builtin_commands()
                .expect("builtin command registration should be valid"),
            keymap,
            next_client_id: 1,
            clients: HashMap::new(),
            queue_stats: QueueStats::default(),
        }
    }
}

impl EditorServer {
    pub(super) fn allocate_client_id(&mut self) -> ClientId {
        let id = ClientId(self.next_client_id);
        self.next_client_id += 1;
        id
    }

    pub(super) fn register_client(
        &mut self,
        client_id: ClientId,
        tx: mpsc::Sender<ServerToClient>,
    ) {
        self.clients.insert(client_id, ClientConnection::new(tx));
    }

    pub(super) fn mark_client_closing(&mut self, client_id: ClientId) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.state = ClientConnectionState::Closing;
        }
    }

    pub(super) fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }
}
