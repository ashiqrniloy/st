use crate::commands::id::CommandId;

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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandHandler {
    RustBuiltin(crate::commands::builtin::RustBuiltinCommand),
    JsCommand {
        extension_id: String,
        command_id: String,
    },
    Tool,
    Generated,
}
