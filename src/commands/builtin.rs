use crate::commands::{
    descriptor::{CommandArgumentDescriptor, CommandDescriptor, CommandHandler, CommandSource},
    id::CommandId,
    registry::CommandRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustBuiltinCommand {
    InsertText,
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
    HelpCommands,
    HelpCommand,
    SplitWindowHorizontal,
    SplitWindowVertical,
    SplitWindowDwim,
}

pub fn register_builtin_editor_commands(registry: &mut CommandRegistry) -> Result<(), String> {
    registry.register(
        CommandDescriptor {
            id: CommandId::new("editor.insert_text")?,
            title: "Insert Text".into(),
            description: "Insert text at the current cursor position.".into(),
            source: CommandSource::Builtin,
            category: "editor".into(),
            arguments: vec![CommandArgumentDescriptor {
                name: "text".into(),
                description: "Text to insert at the cursor".into(),
            }],
            examples: vec!["Insert typed characters into the current buffer".into()],
            related_docs: vec!["documenation.md#command-metadata".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::InsertText),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("editor.backspace")?,
            title: "Backspace".into(),
            description: "Delete one character before the cursor.".into(),
            source: CommandSource::Builtin,
            category: "editor".into(),
            arguments: vec![],
            examples: vec!["Remove one character to the left of the cursor".into()],
            related_docs: vec!["documenation.md#command-metadata".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("editor.move_cursor_left")?,
            title: "Move Cursor Left".into(),
            description: "Move the cursor one character to the left.".into(),
            source: CommandSource::Builtin,
            category: "editor".into(),
            arguments: vec![],
            examples: vec!["Navigate one character left".into()],
            related_docs: vec!["documenation.md#command-metadata".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::MoveCursorLeft),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("editor.move_cursor_right")?,
            title: "Move Cursor Right".into(),
            description: "Move the cursor one character to the right.".into(),
            source: CommandSource::Builtin,
            category: "editor".into(),
            arguments: vec![],
            examples: vec!["Navigate one character right".into()],
            related_docs: vec!["documenation.md#command-metadata".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::MoveCursorRight),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("window.split_horizontal")?,
            title: "Split Window Horizontally".into(),
            description: "Split the active pane along the X axis to create top/bottom panes. Same-axis layouts stop at three panes; mixed layouts stop at four panes.".into(),
            source: CommandSource::Builtin,
            category: "window".into(),
            arguments: vec![],
            examples: vec!["Run window.split_horizontal to split the active pane into top and bottom regions".into()],
            related_docs: vec!["planning/done/phase-16-ui-window-management-and-split-panes.md".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::SplitWindowHorizontal),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("window.split_vertical")?,
            title: "Split Window Vertically".into(),
            description: "Split the active pane along the Y axis to create left/right panes. The active unsplit pane is preferred; otherwise the next unsplit pane in stable pane order is used.".into(),
            source: CommandSource::Builtin,
            category: "window".into(),
            arguments: vec![],
            examples: vec!["Run window.split_vertical to split the active pane into left and right regions".into()],
            related_docs: vec!["planning/done/phase-16-ui-window-management-and-split-panes.md".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::SplitWindowVertical),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("window.split_dwim")?,
            title: "Split Window DWIM".into(),
            description: "Choose a split direction from the current window aspect ratio and layout. Wide full windows start with horizontal top/bottom splits, constrained wide windows prefer vertical left/right splits, and terminal layouts return a clear error.".into(),
            source: CommandSource::Builtin,
            category: "window".into(),
            arguments: vec![],
            examples: vec!["Run window.split_dwim repeatedly to fill the current window up to the documented pane limit".into()],
            related_docs: vec!["configuration setting window.split_dwim_wide_aspect_ratio".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::SplitWindowDwim),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("help.commands")?,
            title: "List Commands".into(),
            description: "List all available commands and their summaries.".into(),
            source: CommandSource::Builtin,
            category: "help".into(),
            arguments: vec![],
            examples: vec!["Show all command documentation entries".into()],
            related_docs: vec!["documenation.md#help-commands".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::HelpCommands),
    )?;

    registry.register(
        CommandDescriptor {
            id: CommandId::new("help.command")?,
            title: "Describe Command".into(),
            description: "Show detailed documentation for one command.".into(),
            source: CommandSource::Builtin,
            category: "help".into(),
            arguments: vec![CommandArgumentDescriptor {
                name: "command_id".into(),
                description: "Command id to describe, e.g. editor.insert_text".into(),
            }],
            examples: vec!["Describe editor.insert_text".into()],
            related_docs: vec!["documenation.md#help-commands".into()],
        },
        CommandHandler::RustBuiltin(RustBuiltinCommand::HelpCommand),
    )?;

    Ok(())
}
