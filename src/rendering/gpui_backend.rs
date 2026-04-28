use gpui::{
    App, Application, Bounds, Context, Window, WindowBounds, WindowOptions, canvas, div, fill,
    point, prelude::*, px, rgb, size,
};

use crate::{
    editor_core::{RenderCommand, UiEvent},
    rendering::{UiChannels, UiRenderer},
};

pub struct GpuiRenderer;

#[derive(Clone, Debug)]
struct RectPrimitive {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
}

struct RootView {
    primitives: Vec<RectPrimitive>,
}

impl RootView {
    fn apply_render_command(&mut self, command: RenderCommand, cx: &mut Context<Self>) {
        match command {
            RenderCommand::DrawRect { x, y, w, h, color } => {
                self.primitives.push(RectPrimitive { x, y, w, h, color });
                cx.notify();
            }
        }
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let primitives = self.primitives.clone();

        div().size_full().bg(rgb(0x1e1e2e)).child(canvas(
            |_, _, _| {},
            move |_bounds, _, window, _| {
                for primitive in &primitives {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(px(primitive.x), px(primitive.y)),
                            size(px(primitive.w), px(primitive.h)),
                        ),
                        rgb(primitive.color),
                    ));
                }
            },
        ))
    }
}

impl UiRenderer for GpuiRenderer {
    fn run(self, channels: UiChannels) -> Result<(), String> {
        Application::new().run(move |cx: &mut App| {
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
            let ui_tx = channels.ui_tx.clone();
            let mut render_rx = channels.render_rx;

            let window = match cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |_window, cx| {
                    cx.new(|_cx| RootView {
                        primitives: Vec::new(),
                    })
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

            cx.observe_keystrokes(move |ev, _window, _cx| {
                if let Some(key_char) = ev.keystroke.key_char.as_ref()
                    && let Some(c) = key_char.chars().next()
                {
                    let _ = ui_tx.send(UiEvent::KeyPress(c));
                }
            })
            .detach();

            cx.spawn(async move |cx| {
                while let Some(command) = render_rx.recv().await {
                    if view
                        .update(cx, |view, cx| view.apply_render_command(command, cx))
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
