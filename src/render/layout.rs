use gpui::{Bounds, Pixels, Point, ShapedLine};

pub(super) fn line_and_col_for_byte(content: &str, byte_index: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, ch) in content.char_indices() {
        if i >= byte_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf8();
        }
    }
    (line, col)
}

pub(super) fn byte_from_line_col(content: &str, target_line: usize, target_col: usize) -> usize {
    let mut line = 0;
    let mut col = 0;
    let mut line_start = 0;
    for (i, ch) in content.char_indices() {
        if line == target_line && col >= target_col {
            return i;
        }
        if ch == '\n' {
            if line == target_line {
                return i;
            }
            line += 1;
            col = 0;
            line_start = i + 1;
        } else {
            col += ch.len_utf8();
        }
    }
    if line == target_line {
        line_start + target_col.min(content[line_start..].len())
    } else {
        content.len()
    }
}

pub(super) fn index_for_mouse_position(
    last_bounds: &Option<Bounds<Pixels>>,
    last_layout: &[ShapedLine],
    line_height: Pixels,
    content: &str,
    position: Point<Pixels>,
) -> usize {
    let Some(bounds) = last_bounds.as_ref() else {
        return 0;
    };
    if last_layout.is_empty() {
        return 0;
    }
    let y = position.y - bounds.top();
    let line_idx = ((y / line_height).floor() as isize).max(0) as usize;
    let line_idx = line_idx.min(last_layout.len().saturating_sub(1));
    let x = position.x - bounds.left();
    let col = last_layout[line_idx].closest_index_for_x(x);
    byte_from_line_col(content, line_idx, col)
}

pub(super) fn vertical_target_offset(content: &str, cursor: usize, line_delta: isize) -> usize {
    let (line, col) = line_and_col_for_byte(content, cursor);
    let target_line = (line as isize + line_delta).max(0) as usize;
    byte_from_line_col(content, target_line, col)
}
