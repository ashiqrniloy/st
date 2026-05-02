use crate::{
    commands::{
        builtin::RustBuiltinCommand, descriptor::CommandHandler, invocation::CommandInvocation,
        registry::CommandRegistry,
    },
    editor::EditorState,
    events::EditorCommand,
};

pub fn execute_command(
    registry: &CommandRegistry,
    invocation: CommandInvocation,
    editor: &mut EditorState,
) -> Result<(), String> {
    let command = registry
        .registered_command(&invocation.command_id)
        .ok_or_else(|| format!("unknown command id: {}", invocation.command_id.as_str()))?;

    match command.handler {
        CommandHandler::RustBuiltin(kind) => apply_builtin_command(kind, invocation, editor),
        CommandHandler::JsCommand { .. } | CommandHandler::Tool | CommandHandler::Generated => {
            Err(format!(
                "command handler not implemented yet for {}",
                command.descriptor.id.as_str()
            ))
        }
    }
}

pub fn apply_builtin_command(
    kind: RustBuiltinCommand,
    invocation: CommandInvocation,
    editor: &mut EditorState,
) -> Result<(), String> {
    match kind {
        RustBuiltinCommand::InsertText => {
            let text = invocation
                .text
                .ok_or_else(|| "editor.insert_text requires text".to_string())?;
            editor.apply(EditorCommand::InsertText { text });
            Ok(())
        }
        RustBuiltinCommand::Backspace => {
            editor.apply(EditorCommand::Backspace);
            Ok(())
        }
        RustBuiltinCommand::MoveCursorLeft => {
            editor.apply(EditorCommand::MoveCursorLeft);
            Ok(())
        }
        RustBuiltinCommand::MoveCursorRight => {
            editor.apply(EditorCommand::MoveCursorRight);
            Ok(())
        }
        RustBuiltinCommand::HelpCommands | RustBuiltinCommand::HelpCommand => Err(
            "help commands are query-only in this phase; use documentation query messages".into(),
        ),
        RustBuiltinCommand::SplitWindowHorizontal
        | RustBuiltinCommand::SplitWindowVertical
        | RustBuiltinCommand::SplitWindowDwim => {
            Err("window split commands require server-owned client window state".into())
        }
    }
}
