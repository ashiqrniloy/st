use serde::{Deserialize, Serialize};

use crate::window_layout::PaneId;

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
    pub buffer_id: u64,
    pub buffer_version: u64,
    pub viewport_start_line: usize,
    pub viewport_end_line: usize,
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
        buffer_id: u64,
        buffer_version: u64,
        cursor_char_index: usize,
        cursor_visible: bool,
    },
    TextEditPatch {
        buffer_id: u64,
        base_version: u64,
        new_version: u64,
        replace_start_char: usize,
        replace_end_char: usize,
        replacement: String,
        cursor_char_index: usize,
        cursor_visible: bool,
    },
    DecorationUpdate,
    DiagnosticsUpdate,
}
