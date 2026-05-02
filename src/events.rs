use serde::{Deserialize, Serialize};

use crate::window_layout::PaneId;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EditorCommand {
    InsertText { text: String },
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
    SplitWindowHorizontal,
    SplitWindowVertical,
    SplitWindowDwim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub start_line: usize,
    pub end_line: usize,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            start_line: 0,
            end_line: 200,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneScene {
    pub pane_id: PaneId,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneUpdate {
    pub background_color: u32,
    pub text: String,
    pub cursor_char_index: usize,
    pub cursor_visible: bool,
    pub panes: Vec<PaneScene>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScenePatch {
    CursorUpdate {
        cursor_char_index: usize,
    },
    SelectionUpdate {
        start_char_index: usize,
        end_char_index: usize,
    },
    VisibleTextUpdate {
        start_line: usize,
        end_line: usize,
        text: String,
        cursor_char_index: usize,
        cursor_visible: bool,
    },
    DecorationUpdate,
    DiagnosticsUpdate,
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
