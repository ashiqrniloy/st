use gpui::{KeyDownEvent, Keystroke, Modifiers};

use crate::events::KeyInputEvent;

fn key_name_from_keystroke(keystroke: &Keystroke) -> String {
    match keystroke.key.to_ascii_lowercase().as_str() {
        " " | "spacebar" => "space".into(),
        "return" => "enter".into(),
        "esc" => "escape".into(),
        other => other.into(),
    }
}

pub(super) fn should_send_raw_key_down(event: &KeyDownEvent) -> bool {
    let modifiers = event.keystroke.modifiers;
    !event.is_held && (modifiers.control || modifiers.alt || modifiers.platform)
}

pub(super) fn key_down_event_to_input(event: &KeyDownEvent) -> KeyInputEvent {
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
