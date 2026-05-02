use super::{key, panes, selection};
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

    let active = panes::active_text_bounds(bounds, &panes);
    assert_eq!(active.left(), px(160.0));
    assert_eq!(active.top(), px(20.0));
    assert_eq!(active.size.width, px(150.0));
    assert_eq!(active.size.height, px(200.0));
}

#[test]
fn pane_dividers_only_describe_internal_split_borders() {
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(300.0), px(200.0)));
    assert!(
        panes::pane_divider_bounds(
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

    let dividers = panes::pane_divider_bounds(
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
    assert_eq!(selection::offset_from_utf16_str(s, 0), 0);
    assert_eq!(selection::offset_from_utf16_str(s, 1), 1);
    assert_eq!(selection::offset_from_utf16_str(s, 3), 5);
    assert_eq!(selection::offset_to_utf16_str(s, 0), 0);
    assert_eq!(selection::offset_to_utf16_str(s, 1), 1);
    assert_eq!(selection::offset_to_utf16_str(s, 5), 3);
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

    assert!(key::should_send_raw_key_down(&event));
    let input = key::key_down_event_to_input(&event);
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

    assert!(!key::should_send_raw_key_down(&event));
}
