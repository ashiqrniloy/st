use std::collections::BTreeSet;

#[derive(Debug, Clone, Default)]
pub struct VisibleTextCache {
    line_start_bytes: Vec<usize>,
    dirty_lines: BTreeSet<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PatchDirtyStats {
    pub dirty_line_count: usize,
}

impl VisibleTextCache {
    pub fn from_content(content: &str) -> Self {
        let line_start_bytes = compute_line_start_bytes(content);
        let mut dirty_lines = BTreeSet::new();
        for i in 0..line_start_bytes.len().max(1) {
            dirty_lines.insert(i);
        }
        Self {
            line_start_bytes,
            dirty_lines,
        }
    }

    pub fn replace_char_range(
        &mut self,
        content: &str,
        replace_start_char: usize,
        replace_end_char: usize,
        replacement: &str,
    ) -> PatchDirtyStats {
        let start_byte = char_to_byte_index(content, replace_start_char);
        let end_byte = char_to_byte_index(content, replace_end_char);
        self.replace_byte_range(content, start_byte, end_byte, replacement)
    }

    pub fn replace_byte_range(
        &mut self,
        content: &str,
        start_byte: usize,
        end_byte: usize,
        replacement: &str,
    ) -> PatchDirtyStats {
        let start_line = self.line_for_byte(start_byte);
        let end_line_before = self.line_for_byte(end_byte);

        let old_newline_count = content[start_byte..end_byte]
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
        let new_newline_count = replacement.bytes().filter(|b| *b == b'\n').count();

        let replaced_len = end_byte.saturating_sub(start_byte);
        let delta = replacement.len() as isize - replaced_len as isize;

        let end_line_after = if old_newline_count == new_newline_count {
            // Fast path: no line-structure change. Keep line starts and shift suffix by byte delta.
            if delta != 0 {
                for start in self.line_start_bytes.iter_mut().skip(end_line_before + 1) {
                    *start = start.saturating_add_signed(delta);
                }
            }
            end_line_before
        } else {
            // Slow path: line structure changed (newline inserted/removed).
            self.line_start_bytes =
                compute_line_start_bytes_after_replace(content, start_byte, end_byte, replacement);
            self.line_for_byte(start_byte + replacement.len())
        };

        for line in start_line..=end_line_after.max(start_line) {
            self.dirty_lines.insert(line);
        }

        PatchDirtyStats {
            dirty_line_count: end_line_after.max(start_line) - start_line + 1,
        }
    }

    pub fn dirty_line_count(&self) -> usize {
        self.dirty_lines.len()
    }

    pub fn take_dirty_lines(&mut self) -> Vec<usize> {
        let lines = self.dirty_lines.iter().copied().collect::<Vec<_>>();
        self.dirty_lines.clear();
        lines
    }

    fn line_for_byte(&self, byte_index: usize) -> usize {
        match self.line_start_bytes.binary_search(&byte_index) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        }
    }
}

fn compute_line_start_bytes(content: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    if starts.is_empty() {
        starts.push(0);
    }
    starts
}

fn compute_line_start_bytes_after_replace(
    content: &str,
    start_byte: usize,
    end_byte: usize,
    replacement: &str,
) -> Vec<usize> {
    let mut new_content = String::with_capacity(content.len() - (end_byte - start_byte) + replacement.len());
    new_content.push_str(&content[..start_byte]);
    new_content.push_str(replacement);
    new_content.push_str(&content[end_byte..]);
    compute_line_start_bytes(&new_content)
}

pub fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}
