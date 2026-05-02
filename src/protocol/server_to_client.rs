use serde::{Deserialize, Serialize};

use crate::{
    documentation::DocumentationResult,
    events::{RenderCommand, ScenePatch, SceneUpdate},
};

use super::ClientId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerToClient {
    Welcome {
        client_id: ClientId,
    },
    SceneSnapshot(SceneUpdate),
    ScenePatch(ScenePatch),
    Render(RenderCommand),
    Error {
        message: String,
    },
    ServerShuttingDown {
        reason: String,
    },
    DocumentationResult(DocumentationResult),
    CommandResult {
        command_id: String,
        success: bool,
        message: String,
    },
}
