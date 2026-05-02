use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::commands::CommandSource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationArgument {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub source: CommandSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationEntry {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub arguments: Vec<DocumentationArgument>,
    pub examples: Vec<String>,
    pub related_links: Vec<String>,
    pub source: CommandSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingDescriptor {
    pub id: String,
    pub title: String,
    pub description: String,
    pub value_type: String,
    pub default_value: String,
    pub valid_values: Vec<String>,
    pub examples: Vec<String>,
    pub configured_via: Vec<String>,
    pub reload_behavior: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingDescriptor {
    pub chord: String,
    pub command_id: String,
    pub title: String,
    pub description: String,
    pub source: CommandSource,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentationQuery {
    ListCommands,
    DescribeCommand { command_id: String },
    ListSettings,
    DescribeSetting { setting_id: String },
    ListKeybindings,
    ListModes,
    ListExtensions,
    ListTools,
    ListPermissions,
    ListApis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentationResult {
    CommandList {
        commands: Vec<DocumentationSummary>,
    },
    CommandDetails {
        command: DocumentationEntry,
    },
    SettingsList {
        settings: Vec<SettingDescriptor>,
    },
    SettingDetails {
        setting: SettingDescriptor,
    },
    KeybindingsList {
        keybindings: Vec<KeybindingDescriptor>,
    },
    ApiList {
        apis: Vec<DocumentationEntry>,
    },
    EmptyPlaceholder {
        registry: DocumentationRegistryKind,
    },
    QueryError {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentationRegistryKind {
    Keybindings,
    Modes,
    Extensions,
    Tools,
    Permissions,
    Apis,
}

pub const REQUIRED_BUILTIN_CAPABILITY_IDS: &[&str] = &[
    "client.start",
    "server.start",
    "server.shutdown",
    "client.close",
    "editor.insert_text",
    "editor.backspace",
    "editor.move_cursor_left",
    "editor.move_cursor_right",
    "window.split_horizontal",
    "window.split_vertical",
    "window.split_dwim",
    "api.window_splitting",
    "render.scene_update",
    "cli.commands",
];

pub fn builtin_capability_docs() -> Vec<DocumentationEntry> {
    vec![
        capability(
            "client.start",
            "Open Client",
            "Open a client and connect to server.",
        ),
        capability(
            "server.start",
            "Start Server",
            "Start the foreground server process.",
        ),
        capability(
            "server.shutdown",
            "Shutdown Server",
            "Shutdown server via `st quit`.",
        ),
        capability(
            "client.close",
            "Close Client",
            "Close one client connection.",
        ),
        capability(
            "editor.insert_text",
            "Insert Text",
            "Insert text at the cursor.",
        ),
        capability(
            "editor.backspace",
            "Backspace",
            "Delete one character before cursor.",
        ),
        capability(
            "editor.move_cursor_left",
            "Move Cursor Left",
            "Move cursor one char left.",
        ),
        capability(
            "editor.move_cursor_right",
            "Move Cursor Right",
            "Move cursor one char right.",
        ),
        capability(
            "window.split_horizontal",
            "Split Window Horizontally",
            "Split active window pane into top/bottom regions with Rust-owned validation.",
        ),
        capability(
            "window.split_vertical",
            "Split Window Vertically",
            "Split active window pane into left/right regions with Rust-owned validation.",
        ),
        capability(
            "window.split_dwim",
            "Split Window DWIM",
            "Choose a documented split direction from window dimensions and layout state.",
        ),
        capability(
            "api.window_splitting",
            "Window Splitting JavaScript API",
            "Extensions call splitWindowHorizontal, splitWindowVertical, or splitWindowDwim to request typed Rust split commands.",
        ),
        capability(
            "render.scene_update",
            "Scene Update",
            "Push updated buffer/cursor/pane scene state to clients.",
        ),
        capability(
            "cli.commands",
            "CLI Commands",
            "User-facing CLI commands: client/server/quit and related flags.",
        ),
    ]
}

pub fn builtin_settings() -> Vec<SettingDescriptor> {
    crate::configuration::builtin_setting_descriptors()
}

pub fn builtin_api_docs() -> Vec<DocumentationEntry> {
    vec![
        DocumentationEntry {
            id: "config.loading".into(),
            title: "Configuration Loading".into(),
            summary: "JavaScript configuration is loaded from ~/.config/st/init.js.".into(),
            description: "At server startup the Deno runtime loads the built-in config API, then ~/.config/st/init.js when it exists, then the runtime event loop. Config code should register keybindings and future settings through public globals such as keymap.bind. User-facing APIs do not use an st. prefix. JavaScript is the supported config language; TypeScript syntax is not accepted unless it is also valid JavaScript.".into(),
            arguments: vec![],
            examples: vec!["keymap.bind(\"ctrl+shift s\", \"window.split_dwim\");".into()],
            related_links: vec!["docs/configuration.md".into(), "runtime/default_init.js".into()],
            source: CommandSource::Builtin,
        },
        DocumentationEntry {
            id: "api.keymap.bind".into(),
            title: "keymap.bind".into(),
            summary: "Bind a key chord to a command from JavaScript config.".into(),
            description: "Register a keybinding in Rust's keymap. The command id must already exist in the Rust command registry. Chords use an Emacs-style modifier group followed by one or more keys, such as ctrl+shift h or ctrl+shift w h. Supported modifiers are ctrl, shift, meta, and alt. Supported named keys include space, enter, and escape. The binding is introspectable through keybinding documentation queries.".into(),
            arguments: vec![
                DocumentationArgument { name: "chord".into(), description: "Key chord such as ctrl+shift h, ctrl+shift w h, or ctrl x ctrl s.".into() },
                DocumentationArgument { name: "commandId".into(), description: "Registered command id, for example window.split_horizontal.".into() },
            ],
            examples: vec!["keymap.bind(\"ctrl+shift h\", \"window.split_horizontal\");".into()],
            related_links: vec!["docs/configuration.md#keybindings".into()],
            source: CommandSource::Builtin,
        },
    ]
}

pub fn validate_builtin_docs() -> Result<(), String> {
    let capabilities = builtin_capability_docs();
    let settings = builtin_settings();

    for capability in &capabilities {
        if capability.id.trim().is_empty()
            || capability.title.trim().is_empty()
            || capability.description.trim().is_empty()
        {
            return Err("builtin capability docs contain empty required fields".into());
        }
    }

    for setting in &settings {
        if setting.id.trim().is_empty()
            || setting.title.trim().is_empty()
            || setting.description.trim().is_empty()
        {
            return Err("builtin setting docs contain empty required fields".into());
        }
    }

    let documented: HashSet<&str> = capabilities.iter().map(|d| d.id.as_str()).collect();
    for required in REQUIRED_BUILTIN_CAPABILITY_IDS {
        if !documented.contains(required) {
            return Err(format!("missing builtin capability doc: {required}"));
        }
    }

    Ok(())
}

fn capability(id: &str, title: &str, description: &str) -> DocumentationEntry {
    DocumentationEntry {
        id: id.into(),
        title: title.into(),
        summary: title.into(),
        description: description.into(),
        arguments: vec![],
        examples: vec![format!("Use capability {id}")],
        related_links: vec!["documenation.md".into()],
        source: CommandSource::Builtin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_docs_have_non_empty_stable_ids() {
        let capabilities = builtin_capability_docs();
        let ids: Vec<&str> = capabilities.iter().map(|d| d.id.as_str()).collect();
        assert!(ids.iter().all(|id| !id.trim().is_empty()));
        assert_eq!(ids, REQUIRED_BUILTIN_CAPABILITY_IDS);
    }

    #[test]
    fn builtin_docs_validator_passes() {
        validate_builtin_docs().expect("builtin docs should validate");
    }

    #[test]
    fn builtin_api_docs_include_configuration_wiki_entries() {
        let apis = builtin_api_docs();
        assert!(apis.iter().any(|api| api.id == "config.loading"));
        assert!(apis.iter().any(|api| api.id == "api.keymap.bind"));
        assert!(apis.iter().all(|api| !api.description.trim().is_empty()));
        assert!(apis.iter().all(|api| !api.examples.is_empty()));
    }
}
