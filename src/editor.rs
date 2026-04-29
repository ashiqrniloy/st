use crate::events::EditorCommand;

#[derive(Default, Debug)]
pub struct EditorState {
    pub buffer: String,
    pub cursor: usize,
}

impl EditorState {
    pub fn apply(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::InsertText { text } => self.insert_text(&text),
            EditorCommand::Backspace => self.backspace(),
            EditorCommand::MoveCursorLeft => self.move_cursor_left(),
            EditorCommand::MoveCursorRight => self.move_cursor_right(),
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        self.clamp_cursor();
        let byte_index = self.byte_index_at_char(self.cursor);
        self.buffer.insert_str(byte_index, text);
        self.cursor += text.chars().count();
        self.clamp_cursor();
    }

    pub fn backspace(&mut self) {
        self.clamp_cursor();
        if self.cursor == 0 {
            return;
        }

        let start = self.byte_index_at_char(self.cursor - 1);
        let end = self.byte_index_at_char(self.cursor);
        self.buffer.replace_range(start..end, "");
        self.cursor -= 1;
        self.clamp_cursor();
    }

    pub fn move_cursor_left(&mut self) {
        self.clamp_cursor();
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.clamp_cursor();
        let len = self.buffer.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn clamp_cursor(&mut self) {
        let len = self.buffer.chars().count();
        if self.cursor > len {
            self.cursor = len;
        }
    }

    fn byte_index_at_char(&self, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }

        self.buffer
            .char_indices()
            .nth(char_index)
            .map(|(idx, _)| idx)
            .unwrap_or(self.buffer.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_move_cursor() {
        let mut editor = EditorState::default();
        editor.apply(EditorCommand::InsertText { text: "abc".into() });
        assert_eq!(editor.buffer, "abc");
        assert_eq!(editor.cursor, 3);

        editor.apply(EditorCommand::MoveCursorLeft);
        assert_eq!(editor.cursor, 2);
        editor.apply(EditorCommand::MoveCursorRight);
        assert_eq!(editor.cursor, 3);
    }

    #[test]
    fn backspace_removes_previous_character() {
        let mut editor = EditorState::default();
        editor.apply(EditorCommand::InsertText { text: "ab".into() });
        editor.apply(EditorCommand::Backspace);
        assert_eq!(editor.buffer, "a");
        assert_eq!(editor.cursor, 1);
    }
}
