use crate::{
    commands::{CommandId, CommandInvocation},
    events::KeyInputEvent,
};

pub(super) fn key_input_to_command_invocation(event: &KeyInputEvent) -> Option<CommandInvocation> {
    if event.ctrl || event.alt || event.meta {
        return None;
    }

    if event.logical_key.eq_ignore_ascii_case("backspace") {
        return Some(CommandInvocation::new(
            CommandId::new("editor.backspace").ok()?,
        ));
    }

    if event.logical_key.eq_ignore_ascii_case("arrowleft")
        || event.logical_key.eq_ignore_ascii_case("left")
    {
        return Some(CommandInvocation::new(
            CommandId::new("editor.move_cursor_left").ok()?,
        ));
    }

    if event.logical_key.eq_ignore_ascii_case("arrowright")
        || event.logical_key.eq_ignore_ascii_case("right")
    {
        return Some(CommandInvocation::new(
            CommandId::new("editor.move_cursor_right").ok()?,
        ));
    }

    let text = event.text.as_deref()?;
    if text.is_empty() {
        return None;
    }

    Some(CommandInvocation::with_text(
        CommandId::new("editor.insert_text").ok()?,
        text,
    ))
}

pub(super) fn should_forward_to_js_runtime(_event: &KeyInputEvent, _handled_by_rust: bool) -> bool {
    // Deno does not receive every key by default. JavaScript is invoked only through
    // capabilities registered in Rust-owned registries/keymaps.
    false
}
