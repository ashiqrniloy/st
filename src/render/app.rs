use gpui::{
    App, AppContext, Application, Bounds, Focusable, WindowBounds, WindowOptions, px, size,
};

use crate::{
    configuration::{
        DEFAULT_CLIENT_WINDOW_HEIGHT_PX, DEFAULT_CLIENT_WINDOW_WIDTH_PX, DEFAULT_CURSOR_VISIBLE,
        DEFAULT_EDITOR_BACKGROUND_COLOR,
    },
    events::{EditorEvent, PaneScene, SceneUpdate},
};

use super::{UiChannels, actions, root_view::RootView};

pub(super) fn run_ui_with_gpui(channels: UiChannels) -> Result<(), String> {
    Application::new().run(move |cx: &mut App| {
        actions::bind_keys(cx);

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
