use gpui::{ClipboardItem, Context, KeyBinding, Window, actions};

use super::{layout, root_view::RootView, selection};

actions!(
    st_editor,
    [
        Backspace,
        Enter,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Copy,
        Cut
    ]
);

pub(super) fn bind_keys(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("StEditor")),
        KeyBinding::new("enter", Enter, Some("StEditor")),
        KeyBinding::new("left", Left, Some("StEditor")),
        KeyBinding::new("right", Right, Some("StEditor")),
        KeyBinding::new("up", Up, Some("StEditor")),
        KeyBinding::new("down", Down, Some("StEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("StEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("StEditor")),
        KeyBinding::new("shift-up", SelectUp, Some("StEditor")),
        KeyBinding::new("shift-down", SelectDown, Some("StEditor")),
        KeyBinding::new("cmd-a", SelectAll, Some("StEditor")),
        KeyBinding::new("cmd-c", Copy, Some("StEditor")),
        KeyBinding::new("cmd-x", Cut, Some("StEditor")),
    ]);
}

impl RootView {
    pub(super) fn insert_newline(&mut self, _: &Enter, _: &mut Window, _cx: &mut Context<Self>) {
        self.send_key("Enter", Some("\n".into()), false);
    }

    pub(super) fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(
                selection::previous_boundary(&self.content, self.cursor_offset()),
                cx,
            );
        } else {
            self.move_to(self.selected_range.start, cx);
        }
        self.send_key("ArrowLeft", None, false);
    }

    pub(super) fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(
                selection::next_boundary(&self.content, self.cursor_offset()),
                cx,
            );
        } else {
            self.move_to(self.selected_range.end, cx);
        }
        self.send_key("ArrowRight", None, false);
    }

    pub(super) fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        let offset = layout::vertical_target_offset(&self.content, self.cursor_offset(), -1);
        self.move_to(offset, cx);
    }

    pub(super) fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        let offset = layout::vertical_target_offset(&self.content, self.cursor_offset(), 1);
        self.move_to(offset, cx);
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(
            selection::previous_boundary(&self.content, self.cursor_offset()),
            cx,
        );
        self.send_key("ArrowLeft", None, true);
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(
            selection::next_boundary(&self.content, self.cursor_offset()),
            cx,
        );
        self.send_key("ArrowRight", None, true);
    }

    pub(super) fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(
            layout::vertical_target_offset(&self.content, self.cursor_offset(), -1),
            cx,
        );
    }

    pub(super) fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(
            layout::vertical_target_offset(&self.content, self.cursor_offset(), 1),
            cx,
        );
    }

    pub(super) fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(
                selection::previous_boundary(&self.content, self.cursor_offset()),
                cx,
            );
        }
        self.send_key("Backspace", None, false);
    }

    pub(super) fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    pub(super) fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub(super) fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.send_key("Backspace", None, false);
        }
    }
}
