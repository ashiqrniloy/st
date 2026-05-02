use crate::{commands::id::CommandId, events::EditorCommand};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    pub command_id: CommandId,
    pub text: Option<String>,
}

impl CommandInvocation {
    pub fn new(command_id: CommandId) -> Self {
        Self {
            command_id,
            text: None,
        }
    }

    pub fn with_text(command_id: CommandId, text: impl Into<String>) -> Self {
        Self {
            command_id,
            text: Some(text.into()),
        }
    }
}

pub fn editor_command_to_invocation(command: EditorCommand) -> Result<CommandInvocation, String> {
    match command {
        EditorCommand::InsertText { text } => Ok(CommandInvocation::with_text(
            CommandId::new("editor.insert_text")?,
            text,
        )),
        EditorCommand::Backspace => Ok(CommandInvocation::new(CommandId::new("editor.backspace")?)),
        EditorCommand::MoveCursorLeft => Ok(CommandInvocation::new(CommandId::new(
            "editor.move_cursor_left",
        )?)),
        EditorCommand::MoveCursorRight => Ok(CommandInvocation::new(CommandId::new(
            "editor.move_cursor_right",
        )?)),
        EditorCommand::SplitWindowHorizontal => Ok(CommandInvocation::new(CommandId::new(
            "window.split_horizontal",
        )?)),
        EditorCommand::SplitWindowVertical => Ok(CommandInvocation::new(CommandId::new(
            "window.split_vertical",
        )?)),
        EditorCommand::SplitWindowDwim => {
            Ok(CommandInvocation::new(CommandId::new("window.split_dwim")?))
        }
    }
}
