use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyInputEvent {
    pub logical_key: String,
    pub physical_key: String,
    pub text: Option<String>,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorEvent {
    KeyInput(KeyInputEvent),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorCommand {
    InsertText { text: String },
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneUpdate {
    pub background_color: u32,
    pub text: String,
    pub cursor_char_index: usize,
    pub cursor_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderCommand {
    DrawRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
}
