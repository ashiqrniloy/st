use super::*;
use crate::events::EditorCommand;

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
