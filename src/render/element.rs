use gpui::{
    App, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId, IntoElement,
    LayoutId, PaintQuad, Pixels, ShapedLine, Style, Window, fill, point, px, relative, rgb, size,
};

use super::{layout, panes, root_view::RootView};

pub(super) struct TextSurface {
    pub(super) view: Entity<RootView>,
}

pub(super) struct PrepaintState {
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
        let text_bounds = panes::active_text_bounds(bounds, &view.panes);
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
            color: rgb(crate::configuration::DEFAULT_EDITOR_TEXT_COLOR).into(),
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

        let (cursor_line, cursor_col) = layout::line_and_col_for_byte(&content, cursor);
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
            let (start_line, start_col) =
                layout::line_and_col_for_byte(&content, selected_range.start);
            let (end_line, end_col) = layout::line_and_col_for_byte(&content, selected_range.end);
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
                        rgb(crate::configuration::DEFAULT_SELECTION_COLOR),
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
        let text_bounds = panes::active_text_bounds(bounds, &self.view.read(cx).panes);
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(text_bounds, self.view.clone()),
            cx,
        );

        let panes = self.view.read(cx).panes.clone();
        if panes.len() > 1 {
            for divider in panes::pane_divider_bounds(bounds, &panes) {
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
