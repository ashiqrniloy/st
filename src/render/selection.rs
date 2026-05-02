use std::ops::Range;

pub(super) fn offset_from_utf16_str(content: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;
    for ch in content.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

pub(super) fn offset_to_utf16_str(content: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in content.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }
    utf16_offset
}

pub(super) fn range_to_utf16(content: &str, range: &Range<usize>) -> Range<usize> {
    offset_to_utf16_str(content, range.start)..offset_to_utf16_str(content, range.end)
}

pub(super) fn range_from_utf16(content: &str, range_utf16: &Range<usize>) -> Range<usize> {
    offset_from_utf16_str(content, range_utf16.start)
        ..offset_from_utf16_str(content, range_utf16.end)
}

pub(super) fn char_to_byte_index(content: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    content
        .char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(content.len())
}

pub(super) fn previous_boundary(content: &str, offset: usize) -> usize {
    content[..offset]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

pub(super) fn next_boundary(content: &str, offset: usize) -> usize {
    if offset >= content.len() {
        return content.len();
    }
    content[offset..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| offset + i)
        .unwrap_or(content.len())
}
