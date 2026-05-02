pub fn clamp(cursor: &mut usize, max: usize) {
    if *cursor > max {
        *cursor = max;
    }
}

pub fn move_left(cursor: &mut usize) {
    if *cursor > 0 {
        *cursor -= 1;
    }
}

pub fn move_right(cursor: &mut usize, max: usize) {
    if *cursor < max {
        *cursor += 1;
    }
}
