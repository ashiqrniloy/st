use crate::events::EditorCommand;

use super::{cursor, TextBuffer};

#[derive(Default, Debug)]
pub struct EditorState {
    pub buffer: TextBuffer,
    pub cursor: usize,
}

impl EditorState {
    pub fn apply(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::InsertText { text } => self.insert_text(&text),
            EditorCommand::Backspace => self.backspace(),
            EditorCommand::MoveCursorLeft => self.move_cursor_left(),
            EditorCommand::MoveCursorRight => self.move_cursor_right(),
            EditorCommand::SplitWindowHorizontal
            | EditorCommand::SplitWindowVertical
            | EditorCommand::SplitWindowDwim => {}
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.clamp_cursor();
        self.buffer.insert(self.cursor, text);
        self.cursor += text.chars().count();
        self.clamp_cursor();
    }

    pub fn backspace(&mut self) {
        self.clamp_cursor();
        if self.cursor == 0 {
            return;
        }

        self.buffer.remove_char_range(self.cursor - 1, self.cursor);
        self.cursor -= 1;
        self.clamp_cursor();
    }

    pub fn move_cursor_left(&mut self) {
        self.clamp_cursor();
        cursor::move_left(&mut self.cursor);
    }

    pub fn move_cursor_right(&mut self) {
        self.clamp_cursor();
        cursor::move_right(&mut self.cursor, self.buffer.char_len());
    }

    pub fn clamp_cursor(&mut self) {
        cursor::clamp(&mut self.cursor, self.buffer.char_len());
    }
}
