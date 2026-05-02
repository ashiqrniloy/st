mod editor_command;
mod input;
mod render_command;
mod scene;

pub use editor_command::EditorCommand;
pub use input::KeyInputEvent;
pub use render_command::RenderCommand;
pub use scene::{PaneScene, ScenePatch, SceneUpdate, Viewport};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorEvent {
    KeyInput(KeyInputEvent),
    ExecuteJsCommand {
        extension_id: String,
        command_id: String,
    },
    WindowDimensionsChanged {
        width: u32,
        height: u32,
    },
    Shutdown,
}
