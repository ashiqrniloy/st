use std::collections::HashMap;

use crate::{
    documentation::{DocumentationArgument, DocumentationEntry, DocumentationSummary},
    editor::EditorState,
    events::EditorCommand,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("command id must not be empty".into());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandArgumentDescriptor {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: CommandId,
    pub title: String,
    pub description: String,
    pub source: CommandSource,
    pub category: String,
    pub arguments: Vec<CommandArgumentDescriptor>,
    pub examples: Vec<String>,
    pub related_docs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommandSource {
    Builtin,
    Extension,
    Tool,
    Generated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustBuiltinCommand {
    InsertText,
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
    HelpCommands,
    HelpCommand,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandHandler {
    RustBuiltin(RustBuiltinCommand),
    JsCommand {
        extension_id: String,
        command_id: String,
    },
    Tool,
    Generated,
}

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

#[derive(Debug, Clone)]
struct RegisteredCommand {
    descriptor: CommandDescriptor,
    handler: CommandHandler,
}

#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<CommandId, RegisteredCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_commands() -> Result<Self, String> {
        let mut registry = Self::new();
        register_builtin_editor_commands(&mut registry)?;
        Ok(registry)
    }

    pub fn register(
        &mut self,
        descriptor: CommandDescriptor,
        handler: CommandHandler,
    ) -> Result<(), String> {
        validate_descriptor(&descriptor)?;

        if self.commands.contains_key(&descriptor.id) {
            return Err(format!(
                "command id already registered: {}",
                descriptor.id.as_str()
            ));
        }

        self.commands.insert(
            descriptor.id.clone(),
            RegisteredCommand {
                descriptor,
                handler,
            },
        );

        Ok(())
    }

    pub fn execute(
        &self,
        invocation: CommandInvocation,
        editor: &mut EditorState,
    ) -> Result<(), String> {
        let command = self
            .commands
            .get(&invocation.command_id)
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

    pub fn list_command_summaries(&self) -> Vec<DocumentationSummary> {
        let mut commands: Vec<DocumentationSummary> = self
            .commands
            .values()
            .map(|registered| DocumentationSummary {
                id: registered.descriptor.id.as_str().to_string(),
                title: registered.descriptor.title.clone(),
                category: registered.descriptor.category.clone(),
                source: registered.descriptor.source,
            })
            .collect();

        commands.sort_by(|a, b| a.id.cmp(&b.id));
        commands
    }

    pub fn handler_for(&self, command_id: &CommandId) -> Option<&CommandHandler> {
        self.commands
            .get(command_id)
            .map(|registered| &registered.handler)
    }

    pub fn describe_command(&self, command_id: &str) -> Option<DocumentationEntry> {
        let id = CommandId::new(command_id).ok()?;
        self.commands.get(&id).map(|registered| DocumentationEntry {
            id: registered.descriptor.id.as_str().to_string(),
            title: registered.descriptor.title.clone(),
            summary: registered.descriptor.title.clone(),
            description: registered.descriptor.description.clone(),
            arguments: registered
                .descriptor
                .arguments
                .iter()
                .map(|argument| DocumentationArgument {
                    name: argument.name.clone(),
                    description: argument.description.clone(),
                })
                .collect(),
            examples: registered.descriptor.examples.clone(),
            related_links: registered.descriptor.related_docs.clone(),
            source: registered.descriptor.source,
        })
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
    }
}

fn apply_builtin_command(
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
    }
}

fn validate_descriptor(descriptor: &CommandDescriptor) -> Result<(), String> {
    if descriptor.id.as_str().trim().is_empty() {
        return Err("command id must not be empty".into());
    }
    if descriptor.title.trim().is_empty() {
        return Err(format!(
            "command {} is missing title",
            descriptor.id.as_str()
        ));
    }
    if descriptor.description.trim().is_empty() {
        return Err(format!(
            "command {} is missing description",
            descriptor.id.as_str()
        ));
    }
    if descriptor.category.trim().is_empty() {
        return Err(format!(
            "command {} is missing category",
            descriptor.id.as_str()
        ));
    }
    if descriptor.examples.is_empty() {
        return Err(format!(
            "command {} must include at least one example",
            descriptor.id.as_str()
        ));
    }

    for argument in &descriptor.arguments {
        if argument.name.trim().is_empty() || argument.description.trim().is_empty() {
            return Err(format!(
                "command {} has invalid argument metadata",
                descriptor.id.as_str()
            ));
        }
    }

    Ok(())
}

fn register_builtin_editor_commands(registry: &mut CommandRegistry) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_descriptor(id: &str) -> CommandDescriptor {
        CommandDescriptor {
            id: CommandId::new(id).expect("valid id"),
            title: "Title".into(),
            description: "Description".into(),
            source: CommandSource::Builtin,
            category: "editor".into(),
            arguments: vec![],
            examples: vec!["Example".into()],
            related_docs: vec!["documenation.md".into()],
        }
    }

    #[test]
    fn rejects_duplicate_command_id_registration() {
        let mut registry = CommandRegistry::new();
        registry
            .register(
                valid_descriptor("test.command"),
                CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
            )
            .expect("first registration should pass");

        let err = registry
            .register(
                valid_descriptor("test.command"),
                CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
            )
            .expect_err("duplicate command id should fail");

        assert!(err.contains("already registered"));
    }

    #[test]
    fn rejects_missing_required_metadata() {
        let mut registry = CommandRegistry::new();
        let mut descriptor = valid_descriptor("test.command");
        descriptor.description.clear();

        let err = registry
            .register(
                descriptor,
                CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
            )
            .expect_err("missing metadata should fail");

        assert!(err.contains("missing description"));
    }

    #[test]
    fn dispatches_builtin_command() {
        let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
        let mut editor = EditorState::default();

        registry
            .execute(
                CommandInvocation::with_text(
                    CommandId::new("editor.insert_text").expect("id"),
                    "abc",
                ),
                &mut editor,
            )
            .expect("dispatch should succeed");

        assert_eq!(editor.buffer.full_text(), "abc");
        assert_eq!(editor.cursor, 3);
    }

    #[test]
    fn builtin_help_commands_are_discoverable() {
        let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
        let summaries = registry.list_command_summaries();
        assert!(summaries.iter().any(|s| s.id == "help.commands"));
        assert!(summaries.iter().any(|s| s.id == "help.command"));
    }

    #[test]
    fn all_builtin_commands_have_required_metadata() {
        let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
        for summary in registry.list_command_summaries() {
            let details = registry
                .describe_command(&summary.id)
                .expect("summary id should be describable");
            assert!(!details.id.trim().is_empty());
            assert!(!details.title.trim().is_empty());
            assert!(!details.description.trim().is_empty());
            assert!(!details.examples.is_empty());
        }
    }
}
