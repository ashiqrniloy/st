use ropey::Rope;

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

    pub fn line_count(&self) -> usize {
        self.rope.len_lines().max(1)
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
