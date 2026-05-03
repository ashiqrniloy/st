use serde::{Deserialize, Serialize};

use crate::{
    documentation::DocumentationQuery,
    events::{EditorCommand, KeyInputEvent, Viewport},
};

use super::ClientId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToServer {
    Hello,
    KeyInput(KeyInputEvent),
    Command(EditorCommand),
    CloseClient {
        client_id: ClientId,
    },
    ShutdownServer,
    DocumentationQuery(DocumentationQuery),
    SetViewport {
        viewport: Viewport,
    },
    RegisterJsCommand {
        extension_id: String,
        command_id: String,
        title: String,
        description: String,
        category: String,
    },
    RegisterJsKeybinding {
        extension_id: String,
        chord: String,
        command_id: String,
    },
    SetWindowDimensions {
        width: u32,
        height: u32,
    },
    ResyncScene,
}
