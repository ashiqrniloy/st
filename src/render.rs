use std::ops::Range;

use gpui::{
    App, Application, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    KeyBinding, KeyDownEvent, Keystroke, LayoutId, Modifiers, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, Style, UTF16Selection,
    Window, WindowBounds, WindowOptions, actions, div, fill, point, prelude::*, px, relative, rgb,
    size,
};
use tokio::sync::mpsc as tokio_mpsc;

use crate::{
    configuration::{
        DEFAULT_CLIENT_WINDOW_HEIGHT_PX, DEFAULT_CLIENT_WINDOW_WIDTH_PX, DEFAULT_CURSOR_VISIBLE,
        DEFAULT_EDITOR_BACKGROUND_COLOR, DEFAULT_EDITOR_TEXT_COLOR, DEFAULT_LINE_HEIGHT_PX,
        DEFAULT_SELECTION_COLOR,
    },
    events::{EditorEvent, KeyInputEvent, PaneScene, SceneUpdate},
};

pub struct UiChannels {
    pub ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
    pub scene_rx: tokio_mpsc::UnboundedReceiver<SceneUpdate>,
}

pub fn run_ui_with_gpui(channels: UiChannels) -> Result<(), String> {
    GpuiRenderer.run(channels)
}

trait UiRenderer {
    fn run(self, channels: UiChannels) -> Result<(), String>;
}

struct GpuiRenderer;

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

fn offset_from_utf16_str(content: &str, offset: usize) -> usize {
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

fn offset_to_utf16_str(content: &str, offset: usize) -> usize {
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

struct RootView {
    ui_tx: tokio_mpsc::UnboundedSender<EditorEvent>,
    focus_handle: FocusHandle,
    content: String,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Vec<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    line_height: Pixels,
    background_color: u32,
    cursor_visible: bool,
    panes: Vec<PaneScene>,
    last_reported_dimensions: Option<(u32, u32)>,
}

impl RootView {
    fn from_scene(
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
            line_height: px(DEFAULT_LINE_HEIGHT_PX),
            background_color: scene.background_color,
            cursor_visible: scene.cursor_visible,
            panes: scene.panes,
            last_reported_dimensions: None,
        };
        let cursor = s.char_to_byte_index(scene.cursor_char_index);
        s.selected_range = cursor..cursor;
        s
    }

    fn apply_scene_update(&mut self, scene: SceneUpdate, cx: &mut Context<Self>) {
        self.background_color = scene.background_color;
        self.cursor_visible = scene.cursor_visible;
        self.panes = scene.panes;
        if self.marked_range.is_none() && !self.is_selecting {
            self.content = scene.text;
            let cursor = self.char_to_byte_index(scene.cursor_char_index);
            self.selected_range = cursor..cursor;
            self.selection_reversed = false;
        }
        cx.notify();
    }

    fn send_key(&self, logical_key: &str, text: Option<String>, shift: bool) {
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

    fn send_gpui_key_down(&self, event: &KeyDownEvent) {
        if !should_send_raw_key_down(event) {
            return;
        }
        let _ = self
            .ui_tx
            .send(EditorEvent::KeyInput(key_down_event_to_input(event)));
    }

    fn report_window_dimensions_if_changed(&mut self, bounds: Bounds<Pixels>) {
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

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content[..offset]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        if offset >= self.content.len() {
            return self.content.len();
        }
        self.content[offset..]
            .char_indices()
            .nth(1)
            .map(|(i, _)| offset + i)
            .unwrap_or(self.content.len())
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
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

    fn offset_from_utf16(&self, offset: usize) -> usize {
        offset_from_utf16_str(&self.content, offset)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        offset_to_utf16_str(&self.content, offset)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn char_to_byte_index(&self, char_index: usize) -> usize {
        if char_index == 0 {
            return 0;
        }
        self.content
            .char_indices()
            .nth(char_index)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len())
    }

    fn line_and_col_for_byte(&self, byte_index: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for (i, ch) in self.content.char_indices() {
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

    fn byte_from_line_col(&self, target_line: usize, target_col: usize) -> usize {
        let mut line = 0;
        let mut col = 0;
        let mut line_start = 0;
        for (i, ch) in self.content.char_indices() {
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
            line_start + target_col.min(self.content[line_start..].len())
        } else {
            self.content.len()
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if self.last_layout.is_empty() {
            return 0;
        }
        let y = position.y - bounds.top();
        let line_idx = ((y / self.line_height).floor() as isize).max(0) as usize;
        let line_idx = line_idx.min(self.last_layout.len().saturating_sub(1));
        let x = position.x - bounds.left();
        let col = self.last_layout[line_idx].closest_index_for_x(x);
        self.byte_from_line_col(line_idx, col)
    }

    fn vertical_target_offset(&self, line_delta: isize) -> usize {
        let (line, col) = self.line_and_col_for_byte(self.cursor_offset());
        let target_line = (line as isize + line_delta).max(0) as usize;
        self.byte_from_line_col(target_line, col)
    }

    fn insert_newline(&mut self, _: &Enter, _: &mut Window, _cx: &mut Context<Self>) {
        self.send_key("Enter", Some("\n".into()), false);
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
        self.send_key("ArrowLeft", None, false);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
        self.send_key("ArrowRight", None, false);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.vertical_target_offset(-1), cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.vertical_target_offset(1), cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        self.send_key("ArrowLeft", None, true);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
        self.send_key("ArrowRight", None, true);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target_offset(-1), cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.vertical_target_offset(1), cx);
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.send_key("Backspace", None, false);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        self.send_gpui_key_down(event);
    }

    fn copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.send_key("Backspace", None, false);
        }
    }
}

impl EntityInputHandler for RootView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
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
            .map(|range| self.range_to_utf16(range))
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
            .map(|r| self.range_from_utf16(r))
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
            .map(|r| self.range_from_utf16(r))
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
            .map(|r| self.range_from_utf16(r))
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
        let range = self.range_from_utf16(&range_utf16);
        let (start_line, start_col) = self.line_and_col_for_byte(range.start);
        let (end_line, end_col) = self.line_and_col_for_byte(range.end);
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
        let byte = self.index_for_mouse_position(point);
        Some(self.offset_to_utf16(byte))
    }
}

struct TextSurface {
    view: Entity<RootView>,
}

fn key_name_from_keystroke(keystroke: &Keystroke) -> String {
    match keystroke.key.to_ascii_lowercase().as_str() {
        " " | "spacebar" => "space".into(),
        "return" => "enter".into(),
        "esc" => "escape".into(),
        other => other.into(),
    }
}

fn should_send_raw_key_down(event: &KeyDownEvent) -> bool {
    let modifiers = event.keystroke.modifiers;
    !event.is_held && (modifiers.control || modifiers.alt || modifiers.platform)
}

fn key_down_event_to_input(event: &KeyDownEvent) -> KeyInputEvent {
    let Modifiers {
        control,
        alt,
        shift,
        platform,
        ..
    } = event.keystroke.modifiers;
    KeyInputEvent {
        logical_key: key_name_from_keystroke(&event.keystroke),
        physical_key: String::new(),
        text: None,
        ctrl: control,
        alt,
        shift,
        meta: platform,
        repeat: event.is_held,
    }
}

fn pane_divider_bounds(element_bounds: Bounds<Pixels>, panes: &[PaneScene]) -> Vec<Bounds<Pixels>> {
    let mut dividers = Vec::new();
    let mut seen = Vec::<(u32, u32, u32, u32)>::new();

    for (index, a) in panes.iter().enumerate() {
        for b in panes.iter().skip(index + 1) {
            if a.x + a.width == b.x || b.x + b.width == a.x {
                let x = if a.x + a.width == b.x { b.x } else { a.x };
                let y_start = a.y.max(b.y);
                let y_end = (a.y + a.height).min(b.y + b.height);
                if y_start < y_end {
                    let key = (x, y_start, x, y_end);
                    if !seen.contains(&key) {
                        seen.push(key);
                        dividers.push(Bounds::new(
                            point(
                                element_bounds.left() + px(x as f32),
                                element_bounds.top() + px(y_start as f32),
                            ),
                            size(px(1.0), px((y_end - y_start) as f32)),
                        ));
                    }
                }
            }

            if a.y + a.height == b.y || b.y + b.height == a.y {
                let y = if a.y + a.height == b.y { b.y } else { a.y };
                let x_start = a.x.max(b.x);
                let x_end = (a.x + a.width).min(b.x + b.width);
                if x_start < x_end {
                    let key = (x_start, y, x_end, y);
                    if !seen.contains(&key) {
                        seen.push(key);
                        dividers.push(Bounds::new(
                            point(
                                element_bounds.left() + px(x_start as f32),
                                element_bounds.top() + px(y as f32),
                            ),
                            size(px((x_end - x_start) as f32), px(1.0)),
                        ));
                    }
                }
            }
        }
    }

    dividers
}

fn active_text_bounds(element_bounds: Bounds<Pixels>, panes: &[PaneScene]) -> Bounds<Pixels> {
    panes
        .iter()
        .find(|pane| pane.active)
        .or_else(|| panes.first())
        .map(|pane| {
            Bounds::new(
                point(
                    element_bounds.left() + px(pane.x as f32),
                    element_bounds.top() + px(pane.y as f32),
                ),
                size(px(pane.width as f32), px(pane.height as f32)),
            )
        })
        .unwrap_or(element_bounds)
}

struct PrepaintState {
    lines: Vec<ShapedLine>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
}

impl IntoElement for TextSurface {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextSurface {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (_window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let view = self.view.read(cx);
        let text_bounds = active_text_bounds(bounds, &view.panes);
        let content = view.content.clone();
        let selected_range = view.selected_range.clone();
        let cursor = view.cursor_offset();
        let lines_text: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.split('\n').map(str::to_string).collect()
        };

        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let run = gpui::TextRun {
            len: 0,
            font: style.font(),
            color: rgb(DEFAULT_EDITOR_TEXT_COLOR).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let shaped_lines: Vec<ShapedLine> = lines_text
            .iter()
            .map(|line_text| {
                let mut r = run.clone();
                r.len = line_text.len();
                window
                    .text_system()
                    .shape_line(line_text.clone().into(), font_size, &[r], None)
            })
            .collect();

        let (cursor_line, cursor_col) = view.line_and_col_for_byte(cursor);
        let cursor_x = shaped_lines
            .get(cursor_line)
            .map(|line| line.x_for_index(cursor_col))
            .unwrap_or(px(0.0));

        let cursor_quad = Some(fill(
            Bounds::new(
                point(
                    text_bounds.left() + cursor_x,
                    text_bounds.top() + view.line_height * cursor_line as f32,
                ),
                size(px(2.0), view.line_height),
            ),
            gpui::blue(),
        ));

        let selections = if selected_range.is_empty() {
            Vec::new()
        } else {
            let (start_line, start_col) = view.line_and_col_for_byte(selected_range.start);
            let (end_line, end_col) = view.line_and_col_for_byte(selected_range.end);
            (start_line..=end_line)
                .filter_map(|line_idx| {
                    let line = shaped_lines.get(line_idx)?;
                    let line_text = lines_text.get(line_idx)?;
                    let start_x = if line_idx == start_line {
                        line.x_for_index(start_col)
                    } else {
                        px(0.0)
                    };
                    let end_x = if line_idx == end_line {
                        line.x_for_index(end_col)
                    } else {
                        line.x_for_index(line_text.len())
                    };
                    Some(fill(
                        Bounds::from_corners(
                            point(
                                text_bounds.left() + start_x,
                                text_bounds.top() + view.line_height * line_idx as f32,
                            ),
                            point(
                                text_bounds.left() + end_x,
                                text_bounds.top() + view.line_height * (line_idx as f32 + 1.0),
                            ),
                        ),
                        rgb(DEFAULT_SELECTION_COLOR),
                    ))
                })
                .collect()
        };

        PrepaintState {
            lines: shaped_lines,
            cursor: cursor_quad,
            selections,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.view.update(cx, |view, _cx| {
            view.report_window_dimensions_if_changed(bounds);
        });

        let focus_handle = self.view.read(cx).focus_handle.clone();
        let text_bounds = active_text_bounds(bounds, &self.view.read(cx).panes);
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(text_bounds, self.view.clone()),
            cx,
        );

        let panes = self.view.read(cx).panes.clone();
        if panes.len() > 1 {
            for divider in pane_divider_bounds(bounds, &panes) {
                window.paint_quad(fill(divider, rgb(0x6c7086)));
            }
        }

        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }

        for (i, line) in prepaint.lines.iter().enumerate() {
            let origin = point(
                text_bounds.left(),
                text_bounds.top() + self.view.read(cx).line_height * i as f32,
            );
            let _ = line.paint(origin, self.view.read(cx).line_height, window, cx);
        }

        if focus_handle.is_focused(window)
            && self.view.read(cx).cursor_visible
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.view.update(cx, |view, _cx| {
            view.last_layout = std::mem::take(&mut prepaint.lines);
            view.last_bounds = Some(text_bounds);
        });
    }
}

impl Focusable for RootView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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

#[cfg(test)]
mod tests {
    use super::{
        active_text_bounds, key_down_event_to_input, offset_from_utf16_str, offset_to_utf16_str,
        pane_divider_bounds, should_send_raw_key_down,
    };
    use crate::{events::PaneScene, window_layout::PaneId};
    use gpui::{Bounds, KeyDownEvent, Keystroke, Modifiers, point, px, size};

    #[test]
    fn active_text_bounds_uses_server_provided_active_pane_rectangle() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(300.0), px(200.0)));
        let panes = vec![
            PaneScene {
                pane_id: PaneId(1),
                x: 0,
                y: 0,
                width: 150,
                height: 200,
                active: false,
            },
            PaneScene {
                pane_id: PaneId(2),
                x: 150,
                y: 0,
                width: 150,
                height: 200,
                active: true,
            },
        ];

        let active = active_text_bounds(bounds, &panes);
        assert_eq!(active.left(), px(160.0));
        assert_eq!(active.top(), px(20.0));
        assert_eq!(active.size.width, px(150.0));
        assert_eq!(active.size.height, px(200.0));
    }

    #[test]
    fn pane_dividers_only_describe_internal_split_borders() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(300.0), px(200.0)));
        assert!(
            pane_divider_bounds(
                bounds,
                &[PaneScene {
                    pane_id: PaneId(1),
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 200,
                    active: true,
                }]
            )
            .is_empty()
        );

        let dividers = pane_divider_bounds(
            bounds,
            &[
                PaneScene {
                    pane_id: PaneId(1),
                    x: 0,
                    y: 0,
                    width: 150,
                    height: 200,
                    active: true,
                },
                PaneScene {
                    pane_id: PaneId(2),
                    x: 150,
                    y: 0,
                    width: 150,
                    height: 200,
                    active: false,
                },
            ],
        );
        assert_eq!(dividers.len(), 1);
        assert_eq!(dividers[0].left(), px(150.0));
        assert_eq!(dividers[0].top(), px(0.0));
        assert_eq!(dividers[0].size.width, px(1.0));
        assert_eq!(dividers[0].size.height, px(200.0));
    }

    #[test]
    fn utf16_utf8_offset_conversion_handles_multibyte_chars() {
        let s = "a🙂b";
        assert_eq!(offset_from_utf16_str(s, 0), 0);
        assert_eq!(offset_from_utf16_str(s, 1), 1);
        assert_eq!(offset_from_utf16_str(s, 3), 5);
        assert_eq!(offset_to_utf16_str(s, 0), 0);
        assert_eq!(offset_to_utf16_str(s, 1), 1);
        assert_eq!(offset_to_utf16_str(s, 5), 3);
    }

    #[test]
    fn modified_gpui_key_down_becomes_server_key_input() {
        let event = KeyDownEvent {
            keystroke: Keystroke {
                modifiers: Modifiers {
                    control: true,
                    shift: true,
                    ..Default::default()
                },
                key: "h".into(),
                key_char: Some("H".into()),
            },
            is_held: false,
        };

        assert!(should_send_raw_key_down(&event));
        let input = key_down_event_to_input(&event);
        assert_eq!(input.logical_key, "h");
        assert!(input.ctrl);
        assert!(input.shift);
        assert!(!input.alt);
        assert!(!input.meta);
        assert!(input.text.is_none());
    }

    #[test]
    fn unmodified_text_key_down_stays_with_text_input_handler() {
        let event = KeyDownEvent {
            keystroke: Keystroke {
                modifiers: Modifiers::default(),
                key: "h".into(),
                key_char: Some("h".into()),
            },
            is_held: false,
        };

        assert!(!should_send_raw_key_down(&event));
    }
}

impl UiRenderer for GpuiRenderer {
    fn run(self, channels: UiChannels) -> Result<(), String> {
        Application::new().run(move |cx: &mut App| {
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

            let close_tx = channels.ui_tx.clone();
            cx.on_window_closed(move |cx| {
                if cx.windows().is_empty() {
                    let _ = close_tx.send(EditorEvent::Shutdown);
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(
                None,
                size(
                    px(DEFAULT_CLIENT_WINDOW_WIDTH_PX as f32),
                    px(DEFAULT_CLIENT_WINDOW_HEIGHT_PX as f32),
                ),
                cx,
            );
            let mut scene_rx = channels.scene_rx;
            let ui_tx = channels.ui_tx.clone();

            let window = match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |window, cx| {
                    let initial = SceneUpdate {
                        background_color: DEFAULT_EDITOR_BACKGROUND_COLOR,
                        text: String::new(),
                        cursor_char_index: 0,
                        cursor_visible: DEFAULT_CURSOR_VISIBLE,
                        panes: vec![PaneScene {
                            pane_id: crate::window_layout::PaneId(1),
                            x: 0,
                            y: 0,
                            width: DEFAULT_CLIENT_WINDOW_WIDTH_PX as u32,
                            height: DEFAULT_CLIENT_WINDOW_HEIGHT_PX as u32,
                            active: true,
                        }],
                    };
                    let entity = cx.new(|cx| RootView::from_scene(initial, ui_tx.clone(), cx));
                    window.focus(&entity.read(cx).focus_handle(cx));
                    entity
                },
            ) {
                Ok(window) => window,
                Err(err) => {
                    eprintln!("Failed to open GPUI window: {err}");
                    cx.quit();
                    return;
                }
            };

            let view = match window.update(cx, |_view, _window, cx| cx.entity()) {
                Ok(view) => view,
                Err(err) => {
                    eprintln!("Failed to capture GPUI root view: {err}");
                    cx.quit();
                    return;
                }
            };

            cx.spawn(async move |cx| {
                while let Some(scene) = scene_rx.recv().await {
                    if view
                        .update(cx, |view, cx| view.apply_scene_update(scene, cx))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();

            cx.activate(true);
        });

        Ok(())
    }
}
