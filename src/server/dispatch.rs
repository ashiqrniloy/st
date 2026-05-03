use tokio::sync::mpsc;

use crate::{
    commands::{CommandId, CommandInvocation, editor_command_to_invocation},
    events::{EditorEvent, KeyInputEvent},
    protocol::{ClientId, ClientToServer, ServerToClient},
    window_layout::{DwimSplitThresholds, SplitAxis},
};

use super::{
    key_chord::{normalize_key_input, parse_key_chord},
    key_input::{key_input_to_command_invocation, should_forward_to_js_runtime},
    keymap::{KeyResolution, RegisteredKeybinding},
    scene::TextEditOp,
    state::EditorServer,
};

impl EditorServer {
    pub(super) fn bind_config_keybinding(
        &mut self,
        chord: String,
        command_id: String,
    ) -> Result<(), String> {
        let parsed_chord = parse_key_chord(&chord)?;
        let command_id = CommandId::new(command_id)?;
        if self.command_registry.handler_for(&command_id).is_none() {
            return Err(format!(
                "cannot bind unknown command id: {}",
                command_id.as_str()
            ));
        }
        let title = self
            .command_registry
            .command_title(&command_id)
            .unwrap_or(command_id.as_str())
            .to_string();
        self.keymap.bind(
            parsed_chord,
            RegisteredKeybinding {
                command_id: command_id.clone(),
                descriptor: crate::documentation::KeybindingDescriptor {
                    chord,
                    command_id: command_id.as_str().to_string(),
                    title,
                    description: "Keybinding registered by ~/.config/st/init.js.".into(),
                    source: crate::commands::CommandSource::Builtin,
                    owner: Some("init.js".into()),
                },
            },
        );
        Ok(())
    }

    fn split_client_window(
        &mut self,
        client_id: ClientId,
        axis: Option<SplitAxis>,
    ) -> Result<(), String> {
        let client = self
            .clients
            .get_mut(&client_id)
            .ok_or_else(|| format!("unknown client id: {:?}", client_id))?;
        let result = match axis {
            Some(axis) => client
                .layout
                .split(axis, client.window_width, client.window_height),
            None => client.layout.split_dwim(
                client.window_width,
                client.window_height,
                DwimSplitThresholds::default(),
            ),
        };
        result.map(|_| ()).map_err(|err| err.to_string())
    }

    fn dispatch_command_id(
        &mut self,
        client_id: ClientId,
        command_id: CommandId,
        js_event_tx: &mpsc::UnboundedSender<EditorEvent>,
    ) {
        match self.command_registry.handler_for(&command_id).cloned() {
            Some(crate::commands::CommandHandler::RustBuiltin(kind)) => {
                let split_result = match kind {
                    crate::commands::RustBuiltinCommand::SplitWindowHorizontal => {
                        Some(self.split_client_window(client_id, Some(SplitAxis::Horizontal)))
                    }
                    crate::commands::RustBuiltinCommand::SplitWindowVertical => {
                        Some(self.split_client_window(client_id, Some(SplitAxis::Vertical)))
                    }
                    crate::commands::RustBuiltinCommand::SplitWindowDwim => {
                        Some(self.split_client_window(client_id, None))
                    }
                    _ => None,
                };

                if let Some(result) = split_result {
                    if let Err(err) = result {
                        self.send_to_client(
                            client_id,
                            ServerToClient::CommandResult {
                                command_id: command_id.as_str().to_string(),
                                success: false,
                                message: err,
                            },
                        );
                    } else {
                        self.send_to_client(
                            client_id,
                            ServerToClient::CommandResult {
                                command_id: command_id.as_str().to_string(),
                                success: true,
                                message: "split accepted".into(),
                            },
                        );
                        self.send_scene_snapshot_to_client(client_id);
                    }
                } else {
                    let invocation = CommandInvocation::new(command_id);
                    let maybe_edit = self.edit_op_for_invocation(&invocation);
                    if let Err(err) = self.command_registry.execute(invocation, &mut self.editor) {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                    } else if let Some(edit) = maybe_edit {
                        self.send_text_edit_patch_updates(&edit);
                    } else {
                        self.send_scene_patch_updates();
                    }
                }
            }
            Some(crate::commands::CommandHandler::JsCommand {
                extension_id,
                command_id,
            }) => {
                let _ = js_event_tx.send(EditorEvent::ExecuteJsCommand {
                    extension_id,
                    command_id,
                });
            }
            Some(_) => {
                self.send_to_client(
                    client_id,
                    ServerToClient::Error {
                        message: "unsupported command handler".into(),
                    },
                );
            }
            None => {
                self.send_to_client(
                    client_id,
                    ServerToClient::Error {
                        message: "unknown command id".into(),
                    },
                );
            }
        }
    }

    fn edit_op_for_invocation(&self, invocation: &CommandInvocation) -> Option<TextEditOp> {
        let base_version = self.editor.buffer.version().0;
        let cursor = self.editor.cursor;
        match invocation.command_id.as_str() {
            "editor.insert_text" => {
                let replacement = invocation.text.clone()?;
                Some(TextEditOp {
                    start_char: cursor,
                    end_char: cursor,
                    replacement,
                    base_version,
                    new_version: base_version + 1,
                })
            }
            "editor.backspace" if cursor > 0 => Some(TextEditOp {
                start_char: cursor - 1,
                end_char: cursor,
                replacement: String::new(),
                base_version,
                new_version: base_version + 1,
            }),
            _ => None,
        }
    }

    fn handle_key_input(
        &mut self,
        client_id: ClientId,
        event: KeyInputEvent,
        js_event_tx: &mpsc::UnboundedSender<EditorEvent>,
    ) {
        let key = normalize_key_input(&event);
        let mut sequence = self
            .clients
            .get(&client_id)
            .map(|c| c.pending_chord.clone())
            .unwrap_or_default();
        sequence.push(key);

        match self.keymap.resolve_state(&sequence) {
            KeyResolution::Exact => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.pending_chord.clear();
                }
                if let Some(command_id) = self.keymap.lookup(&sequence) {
                    self.dispatch_command_id(client_id, command_id, js_event_tx);
                    return;
                }
            }
            KeyResolution::Prefix => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.pending_chord = sequence;
                }
                return;
            }
            KeyResolution::None => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.pending_chord.clear();
                }
            }
        }

        if let Some(invocation) = key_input_to_command_invocation(&event) {
            let maybe_edit = self.edit_op_for_invocation(&invocation);
            if let Err(err) = self.command_registry.execute(invocation, &mut self.editor) {
                self.send_to_client(client_id, ServerToClient::Error { message: err });
            } else if let Some(edit) = maybe_edit {
                self.send_text_edit_patch_updates(&edit);
            } else {
                self.send_scene_patch_updates();
            }
            return;
        }

        if should_forward_to_js_runtime(&event, false) {
            let _ = js_event_tx.send(EditorEvent::KeyInput(event));
        }
    }

    // Guardrail: keep this sync so canonical state is not held across awaits/JS execution.
    pub(super) fn handle_message(
        &mut self,
        client_id: ClientId,
        message: ClientToServer,
        js_event_tx: &mpsc::UnboundedSender<EditorEvent>,
    ) -> bool {
        match message {
            ClientToServer::Hello => {
                println!("Server: client {client_id:?} said hello");
                self.send_scene_snapshot_to_client(client_id);
            }
            ClientToServer::KeyInput(event) => {
                self.handle_key_input(client_id, event, js_event_tx);
            }
            ClientToServer::Command(command) => {
                let invocation = match editor_command_to_invocation(command) {
                    Ok(invocation) => invocation,
                    Err(err) => {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                        return false;
                    }
                };

                let split_result = match self.command_registry.handler_for(&invocation.command_id) {
                    Some(crate::commands::CommandHandler::RustBuiltin(
                        crate::commands::RustBuiltinCommand::SplitWindowHorizontal,
                    )) => Some(self.split_client_window(client_id, Some(SplitAxis::Horizontal))),
                    Some(crate::commands::CommandHandler::RustBuiltin(
                        crate::commands::RustBuiltinCommand::SplitWindowVertical,
                    )) => Some(self.split_client_window(client_id, Some(SplitAxis::Vertical))),
                    Some(crate::commands::CommandHandler::RustBuiltin(
                        crate::commands::RustBuiltinCommand::SplitWindowDwim,
                    )) => Some(self.split_client_window(client_id, None)),
                    _ => None,
                };

                if let Some(result) = split_result {
                    if let Err(err) = result {
                        self.send_to_client(
                            client_id,
                            ServerToClient::CommandResult {
                                command_id: invocation.command_id.as_str().to_string(),
                                success: false,
                                message: err,
                            },
                        );
                    } else {
                        self.send_to_client(
                            client_id,
                            ServerToClient::CommandResult {
                                command_id: invocation.command_id.as_str().to_string(),
                                success: true,
                                message: "split accepted".into(),
                            },
                        );
                        self.send_scene_snapshot_to_client(client_id);
                    }
                } else {
                    let maybe_edit = self.edit_op_for_invocation(&invocation);
                    if let Err(err) = self.command_registry.execute(invocation, &mut self.editor) {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                    } else if let Some(edit) = maybe_edit {
                        self.send_text_edit_patch_updates(&edit);
                    } else {
                        self.send_scene_patch_updates();
                    }
                }
            }
            ClientToServer::CloseClient {
                client_id: requested_client_id,
            } => {
                if requested_client_id != client_id {
                    self.send_to_client(
                        client_id,
                        ServerToClient::Error {
                            message: format!(
                                "client {client_id:?} cannot close {requested_client_id:?}"
                            ),
                        },
                    );
                }

                println!("Server: client {client_id:?} requested close");
                self.mark_client_closing(client_id);
                self.remove_client(client_id);
            }
            ClientToServer::ShutdownServer => {
                println!("Server: shutdown requested by {client_id:?}");
                self.send_to_all(ServerToClient::ServerShuttingDown {
                    reason: "server shutdown requested".into(),
                });
                return true;
            }
            ClientToServer::DocumentationQuery(query) => {
                let result = self.handle_documentation_query(query);
                self.send_to_client(client_id, ServerToClient::DocumentationResult(result));
            }
            ClientToServer::SetViewport { viewport } => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.viewport = viewport;
                }
                self.send_scene_snapshot_to_client(client_id);
            }
            ClientToServer::SetWindowDimensions { width, height } => {
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.window_width = width.max(1);
                    client.window_height = height.max(1);
                }
                self.send_scene_snapshot_to_client(client_id);
            }
            ClientToServer::ResyncScene => {
                self.send_scene_snapshot_to_client(client_id);
            }
            ClientToServer::RegisterJsCommand {
                extension_id,
                command_id,
                title,
                description,
                category,
            } => {
                let id = match CommandId::new(command_id.clone()) {
                    Ok(id) => id,
                    Err(err) => {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                        return false;
                    }
                };

                let descriptor = crate::commands::CommandDescriptor {
                    id,
                    title,
                    description,
                    source: crate::commands::CommandSource::Extension,
                    category,
                    arguments: vec![],
                    examples: vec!["Extension-invoked command".into()],
                    related_docs: vec!["documenation.md#command-metadata".into()],
                };

                if let Err(err) = self.command_registry.register(
                    descriptor,
                    crate::commands::CommandHandler::JsCommand {
                        extension_id,
                        command_id,
                    },
                ) {
                    self.send_to_client(client_id, ServerToClient::Error { message: err });
                }
            }
            ClientToServer::RegisterJsKeybinding {
                extension_id,
                chord,
                command_id,
            } => {
                let parsed_chord = match parse_key_chord(&chord) {
                    Ok(chord) => chord,
                    Err(err) => {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                        return false;
                    }
                };
                let command_id = match CommandId::new(command_id) {
                    Ok(command_id) => command_id,
                    Err(err) => {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                        return false;
                    }
                };

                match self.command_registry.handler_for(&command_id) {
                    Some(crate::commands::CommandHandler::JsCommand {
                        extension_id: owner,
                        ..
                    }) if owner == &extension_id => {
                        let title = self
                            .command_registry
                            .command_title(&command_id)
                            .unwrap_or(command_id.as_str())
                            .to_string();
                        self.keymap.bind(
                            parsed_chord,
                            RegisteredKeybinding {
                                command_id: command_id.clone(),
                                descriptor: crate::documentation::KeybindingDescriptor {
                                    chord,
                                    command_id: command_id.as_str().to_string(),
                                    title,
                                    description: format!(
                                        "Extension keybinding registered by {extension_id}."
                                    ),
                                    source: crate::commands::CommandSource::Extension,
                                    owner: Some(extension_id),
                                },
                            },
                        );
                    }
                    Some(crate::commands::CommandHandler::JsCommand { .. }) => {
                        self.send_to_client(
                            client_id,
                            ServerToClient::Error {
                                message:
                                    "extension cannot bind a command owned by another extension"
                                        .into(),
                            },
                        );
                    }
                    Some(_) => {
                        self.send_to_client(
                            client_id,
                            ServerToClient::Error {
                                message: "JS keybindings may only target JS commands".into(),
                            },
                        );
                    }
                    None => {
                        self.send_to_client(
                            client_id,
                            ServerToClient::Error {
                                message: "cannot bind unknown command id".into(),
                            },
                        );
                    }
                }
            }
        }

        false
    }
}
