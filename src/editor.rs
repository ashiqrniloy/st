use ropey::Rope;

use crate::events::EditorCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferVersion(pub u64);

#[derive(Debug, Clone)]
pub struct BufferSnapshot {
    pub buffer_id: BufferId,
    pub version: BufferVersion,
    pub rope: Rope,
}

impl BufferSnapshot {
    pub fn is_stale_for(&self, current: BufferVersion) -> bool {
        self.version != current
    }
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    id: BufferId,
    version: BufferVersion,
    rope: Rope,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self {
            id: BufferId(1),
            version: BufferVersion(0),
            rope: Rope::new(),
        }
    }
}

impl TextBuffer {
    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn version(&self) -> BufferVersion {
        self.version
    }

    pub fn char_len(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn full_text(&self) -> String {
        self.rope.to_string()
    }

    pub fn insert(&mut self, char_index: usize, text: &str) {
        self.rope.insert(char_index, text);
        self.bump_version();
    }

    pub fn remove_char_range(&mut self, start_char: usize, end_char: usize) {
        self.rope.remove(start_char..end_char);
        self.bump_version();
    }

    #[allow(dead_code)]
    pub fn slice_chars(&self, start_char: usize, end_char: usize) -> String {
        self.rope.slice(start_char..end_char).to_string()
    }

    pub fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        self.rope.line_to_char(line) + col
    }

    pub fn char_to_line_col(&self, char_index: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(char_index);
        let line_start = self.rope.line_to_char(line);
        (line, char_index.saturating_sub(line_start))
    }

    pub fn char_to_utf8_offset(&self, char_index: usize) -> usize {
        self.rope.char_to_byte(char_index)
    }

    pub fn utf8_offset_to_char(&self, utf8_offset: usize) -> usize {
        self.rope.byte_to_char(utf8_offset)
    }

    pub fn char_to_utf16_offset(&self, char_index: usize) -> usize {
        self.rope
            .slice(..char_index)
            .chars()
            .map(|ch| ch.len_utf16())
            .sum()
    }

    pub fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot {
            buffer_id: self.id,
            version: self.version,
            rope: self.rope.clone(),
        }
    }

    fn bump_version(&mut self) {
        self.version.0 += 1;
    }
}

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
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        self.clamp_cursor();
        let len = self.buffer.char_len();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn clamp_cursor(&mut self) {
        let len = self.buffer.char_len();
        if self.cursor > len {
            self.cursor = len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_move_cursor() {
        let mut editor = EditorState::default();
        editor.apply(EditorCommand::InsertText { text: "abc".into() });
        assert_eq!(editor.buffer.full_text(), "abc");
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
        assert_eq!(editor.buffer.full_text(), "a");
        assert_eq!(editor.cursor, 1);
    }

    #[test]
    fn supports_utf8_utf16_and_line_column_mappings() {
        let mut buffer = TextBuffer::default();
        buffer.insert(0, "a\n€😀b");

        assert_eq!(buffer.char_to_utf8_offset(2), 2);
        assert_eq!(buffer.char_to_utf8_offset(3), 5);
        assert_eq!(buffer.utf8_offset_to_char(5), 3);

        assert_eq!(buffer.char_to_utf16_offset(0), 0);
        assert_eq!(buffer.char_to_utf16_offset(3), 3);
        assert_eq!(buffer.char_to_utf16_offset(4), 5);

        assert_eq!(buffer.char_to_line_col(0), (0, 0));
        assert_eq!(buffer.char_to_line_col(2), (1, 0));
        assert_eq!(buffer.line_col_to_char(1, 2), 4);
    }

    #[test]
    fn snapshot_version_marks_stale_results() {
        let mut buffer = TextBuffer::default();
        let snapshot = buffer.snapshot();
        buffer.insert(0, "hello");

        assert!(snapshot.is_stale_for(buffer.version()));
        assert_eq!(snapshot.buffer_id, buffer.id());
        assert_eq!(snapshot.rope.to_string(), "");
    }
}
