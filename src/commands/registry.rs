use std::collections::HashMap;

use crate::{
    commands::{
        builtin::register_builtin_editor_commands,
        descriptor::{CommandDescriptor, CommandHandler},
        execute::execute_command,
        id::CommandId,
        invocation::CommandInvocation,
        validation::validate_descriptor,
    },
    documentation::{DocumentationArgument, DocumentationEntry, DocumentationSummary},
    editor::EditorState,
};

#[derive(Debug, Clone)]
pub(crate) struct RegisteredCommand {
    pub(crate) descriptor: CommandDescriptor,
    pub(crate) handler: CommandHandler,
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
        execute_command(self, invocation, editor)
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

    pub fn command_title(&self, command_id: &CommandId) -> Option<&str> {
        self.commands
            .get(command_id)
            .map(|registered| registered.descriptor.title.as_str())
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

    pub(crate) fn registered_command(&self, command_id: &CommandId) -> Option<&RegisteredCommand> {
        self.commands.get(command_id)
    }
}
