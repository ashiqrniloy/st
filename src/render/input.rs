use std::ops::Range;

use gpui::{Bounds, Context, EntityInputHandler, Pixels, Point, UTF16Selection, Window, point};

use super::{layout, root_view::RootView, selection};

impl EntityInputHandler for RootView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = selection::range_from_utf16(&self.content, &range_utf16);
        actual_range.replace(selection::range_to_utf16(&self.content, &range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: selection::range_to_utf16(&self.content, &self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| selection::range_to_utf16(&self.content, range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| selection::range_from_utf16(&self.content, r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        if self.marked_range.is_some() {
            self.content = format!(
                "{}{}{}",
                &self.content[..range.start],
                new_text,
                &self.content[range.end..]
            );
            let new_cursor = range.start + new_text.len();
            self.selected_range = new_cursor..new_cursor;
            self.marked_range = None;
            cx.notify();
        }
        self.send_key(new_text, Some(new_text.to_string()), false);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| selection::range_from_utf16(&self.content, r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content = format!(
            "{}{}{}",
            &self.content[..range.start],
            new_text,
            &self.content[range.end..]
        );
        self.marked_range = if new_text.is_empty() {
            None
        } else {
            Some(range.start..range.start + new_text.len())
        };

        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| selection::range_from_utf16(&self.content, r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        self.send_key(new_text, Some(new_text.to_string()), false);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = selection::range_from_utf16(&self.content, &range_utf16);
        let (start_line, start_col) = layout::line_and_col_for_byte(&self.content, range.start);
        let (end_line, end_col) = layout::line_and_col_for_byte(&self.content, range.end);
        let start_layout = self.last_layout.get(start_line)?;
        let end_layout = self.last_layout.get(end_line)?;

        Some(Bounds::from_corners(
            point(
                element_bounds.left() + start_layout.x_for_index(start_col),
                element_bounds.top() + self.line_height * start_line as f32,
            ),
            point(
                element_bounds.left() + end_layout.x_for_index(end_col),
                element_bounds.top() + self.line_height * (end_line as f32 + 1.0),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let byte = layout::index_for_mouse_position(
            &self.last_bounds,
            &self.last_layout,
            self.line_height,
            &self.content,
            point,
        );
        Some(selection::offset_to_utf16_str(&self.content, byte))
    }
}
