use crate::documentation::{DocumentationQuery, DocumentationRegistryKind, DocumentationResult};

use super::state::EditorServer;

impl EditorServer {
    pub(super) fn handle_documentation_query(
        &self,
        query: DocumentationQuery,
    ) -> DocumentationResult {
        match query {
            DocumentationQuery::ListCommands => DocumentationResult::CommandList {
                commands: self.command_registry.list_command_summaries(),
            },
            DocumentationQuery::DescribeCommand { command_id } => {
                match self.command_registry.describe_command(&command_id) {
                    Some(command) => DocumentationResult::CommandDetails { command },
                    None => DocumentationResult::QueryError {
                        message: format!("unknown command id: {command_id}"),
                    },
                }
            }
            DocumentationQuery::ListSettings => DocumentationResult::SettingsList {
                settings: crate::documentation::builtin_settings(),
            },
            DocumentationQuery::DescribeSetting { setting_id } => {
                match crate::documentation::builtin_settings()
                    .into_iter()
                    .find(|setting| setting.id == setting_id)
                {
                    Some(setting) => DocumentationResult::SettingDetails { setting },
                    None => DocumentationResult::QueryError {
                        message: format!("unknown setting id: {setting_id}"),
                    },
                }
            }
            DocumentationQuery::ListKeybindings => DocumentationResult::KeybindingsList {
                keybindings: self.keymap.list_descriptors(),
            },
            DocumentationQuery::ListModes => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Modes,
            },
            DocumentationQuery::ListExtensions => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Extensions,
            },
            DocumentationQuery::ListTools => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Tools,
            },
            DocumentationQuery::ListPermissions => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Permissions,
            },
            DocumentationQuery::ListApis => DocumentationResult::ApiList {
                apis: crate::documentation::builtin_api_docs(),
            },
        }
    }
}
