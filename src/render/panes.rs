use gpui::{Bounds, Pixels, point, px, size};

use crate::events::PaneScene;

pub(super) fn pane_divider_bounds(
    element_bounds: Bounds<Pixels>,
    panes: &[PaneScene],
) -> Vec<Bounds<Pixels>> {
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

pub(super) fn active_text_bounds(
    element_bounds: Bounds<Pixels>,
    panes: &[PaneScene],
) -> Bounds<Pixels> {
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
