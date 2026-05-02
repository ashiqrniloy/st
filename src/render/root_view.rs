use std::ops::Range;

use gpui::{
    App, Context, CursorStyle, FocusHandle, Focusable, InteractiveElement, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Render,
    ShapedLine, Styled, Window, div, px, rgb,
};
use tokio::sync::mpsc as tokio_mpsc;

use crate::events::{EditorEvent, KeyInputEvent, PaneScene, SceneUpdate};

use super::{element::TextSurface, key, layout, selection};

pub(super) struct RootView {
    pub(super) ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
    pub(super) focus_handle: FocusHandle,
    pub(super) content: String,
    pub(super) selected_range: Range<usize>,
    pub(super) selection_reversed: bool,
    pub(super) marked_range: Option<Range<usize>>,
    pub(super) last_layout: Vec<ShapedLine>,
    pub(super) last_bounds: Option<gpui::Bounds<Pixels>>,
    pub(super) is_selecting: bool,
    pub(super) line_height: Pixels,
    pub(super) background_color: u32,
    pub(super) cursor_visible: bool,
    pub(super) panes: Vec<PaneScene>,
    pub(super) last_reported_dimensions: Option<(u32, u32)>,
}

impl RootView {
    pub(super) fn from_scene(
        scene: SceneUpdate,
        ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut s = Self {
            ui_tx,
            focus_handle: cx.focus_handle(),
            content: scene.text,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: Vec::new(),
            last_bounds: None,
            is_selecting: false,
            line_height: px(crate::configuration::DEFAULT_LINE_HEIGHT_PX),
            background_color: scene.background_color,
            cursor_visible: scene.cursor_visible,
            panes: scene.panes,
            last_reported_dimensions: None,
        };
        let cursor = selection::char_to_byte_index(&s.content, scene.cursor_char_index);
        s.selected_range = cursor..cursor;
        s
    }

    pub(super) fn apply_scene_update(&mut self, scene: SceneUpdate, cx: &mut Context<Self>) {
        self.background_color = scene.background_color;
        self.cursor_visible = scene.cursor_visible;
        self.panes = scene.panes;
        if self.marked_range.is_none() && !self.is_selecting {
            self.content = scene.text;
            let cursor = selection::char_to_byte_index(&self.content, scene.cursor_char_index);
            self.selected_range = cursor..cursor;
            self.selection_reversed = false;
        }
        cx.notify();
    }

    pub(super) fn send_key(&self, logical_key: &str, text: Option<String>, shift: bool) {
        let _ = self.ui_tx.send(EditorEvent::KeyInput(KeyInputEvent {
            logical_key: logical_key.into(),
            physical_key: String::new(),
            text,
            ctrl: false,
            alt: false,
            shift,
            meta: false,
            repeat: false,
        }));
    }

    pub(super) fn send_gpui_key_down(&self, event: &KeyDownEvent) {
        if key::should_send_raw_key_down(event) {
            let _ = self
                .ui_tx
                .send(EditorEvent::KeyInput(key::key_down_event_to_input(event)));
        }
    }

    pub(super) fn report_window_dimensions_if_changed(&mut self, bounds: gpui::Bounds<Pixels>) {
        let width = f32::from(bounds.size.width).max(1.0).round() as u32;
        let height = f32::from(bounds.size.height).max(1.0).round() as u32;
        let dimensions = (width, height);
        if self.last_reported_dimensions == Some(dimensions) {
            return;
        }
        self.last_reported_dimensions = Some(dimensions);
        let _ = self
            .ui_tx
            .send(EditorEvent::WindowDimensionsChanged { width, height });
    }

    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        let idx = layout::index_for_mouse_position(
            &self.last_bounds,
            &self.last_layout,
            self.line_height,
            &self.content,
            event.position,
        );
        if event.modifiers.shift {
            self.select_to(idx, cx);
        } else {
            self.move_to(idx, cx);
        }
    }

    pub(super) fn on_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            let idx = layout::index_for_mouse_position(
                &self.last_bounds,
                &self.last_layout,
                self.line_height,
                &self.content,
                event.position,
            );
            self.select_to(idx, cx);
        }
    }

    pub(super) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.send_gpui_key_down(event);
    }
}

impl Focusable for RootView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div()
            .size_full()
            .bg(rgb(self.background_color))
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle(cx))
            .key_context("StEditor")
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::insert_newline))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .p_4()
            .child(TextSurface { view: cx.entity() })
    }
}
