use std::{
    collections::{HashMap, HashSet},
    fs, io,
};

use tokio::{io::BufReader, net::UnixStream, sync::mpsc};

use crate::{
    editor::EditorState,
    events::{EditorCommand, EditorEvent, KeyInputEvent, RenderCommand, SceneUpdate},
    ipc::{bind_server_socket, connect_to_server, read_json_line, socket_path, write_json_line},
    js_runtime::spawn_js_runtime,
    protocol::{ClientId, ClientToServer, ServerToClient},
};

#[derive(Debug)]
pub struct EditorServer {
    editor: EditorState,
    next_client_id: u64,
    clients: HashSet<ClientId>,
    client_txs: HashMap<ClientId, mpsc::UnboundedSender<ServerToClient>>,
}

#[derive(Debug)]
enum ServerEvent {
    ClientMessage {
        client_id: ClientId,
        message: ClientToServer,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    JsRenderCommand(RenderCommand),
}

impl Default for EditorServer {
    fn default() -> Self {
        Self {
            editor: EditorState::default(),
            next_client_id: 1,
            clients: HashSet::new(),
            client_txs: HashMap::new(),
        }
    }
}

impl EditorServer {
    fn allocate_client_id(&mut self) -> ClientId {
        let id = ClientId(self.next_client_id);
        self.next_client_id += 1;
        id
    }

    fn register_client(&mut self, client_id: ClientId, tx: mpsc::UnboundedSender<ServerToClient>) {
        self.clients.insert(client_id);
        self.client_txs.insert(client_id, tx);
    }

    fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
        self.client_txs.remove(&client_id);
    }

    fn scene_update(&self) -> SceneUpdate {
        SceneUpdate {
            background_color: 0x1e1e2e,
            text: self.editor.buffer.clone(),
            cursor_char_index: self.editor.cursor,
            cursor_visible: true,
        }
    }

    fn send_scene_update(&mut self) {
        self.send_to_all(ServerToClient::Scene(self.scene_update()));
    }

    fn send_to_all(&mut self, message: ServerToClient) {
        let dead_clients: Vec<ClientId> = self
            .client_txs
            .iter()
            .filter_map(|(client_id, tx)| {
                if tx.send(message.clone()).is_err() {
                    Some(*client_id)
                } else {
                    None
                }
            })
            .collect();

        for client_id in dead_clients {
            self.remove_client(client_id);
        }
    }

    fn handle_message(
        &mut self,
        client_id: ClientId,
        message: ClientToServer,
        js_event_tx: &mpsc::UnboundedSender<EditorEvent>,
    ) -> bool {
        match message {
            ClientToServer::Hello => {
                println!("Server: client {client_id:?} said hello");
                if let Some(tx) = self.client_txs.get(&client_id) {
                    let _ = tx.send(ServerToClient::Scene(self.scene_update()));
                }
            }
            ClientToServer::KeyInput(event) => {
                println!("Server: key input from {client_id:?}: {event:?}");

                let command = key_input_to_command(&event);
                let handled_by_rust = command.is_some();

                if let Some(command) = command {
                    self.editor.apply(command);
                    println!(
                        "Server: editor state: buffer={:?}, cursor={}",
                        self.editor.buffer, self.editor.cursor
                    );
                    self.send_scene_update();
                }

                if should_forward_to_js_runtime(&event, handled_by_rust) {
                    println!("Server: forwarding key input to JS runtime (temporary policy)");
                    let _ = js_event_tx.send(EditorEvent::KeyInput(event));
                }
            }
            ClientToServer::Command(command) => {
                println!("Server: command from {client_id:?}: {command:?}");
                self.editor.apply(command);
                println!(
                    "Server: editor state: buffer={:?}, cursor={}",
                    self.editor.buffer, self.editor.cursor
                );
                self.send_scene_update();
            }
            ClientToServer::CloseClient { client_id } => {
                println!("Server: client {client_id:?} requested close");
                self.remove_client(client_id);
            }
            ClientToServer::ShutdownServer => {
                println!("Server: shutdown requested by {client_id:?}");
                return true;
            }
        }

        false
    }
}

pub fn run_foreground() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to build server runtime: {err}"))?;

    runtime
        .block_on(run_server())
        .map_err(|err| err.to_string())
}

pub fn request_shutdown() -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to build quit runtime: {err}"))?;

    runtime
        .block_on(async {
            let mut stream = connect_to_server().await?;
            write_json_line(&mut stream, &ClientToServer::ShutdownServer).await
        })
        .map_err(|err| format!("failed to request server shutdown: {err}"))
}

async fn run_server() -> io::Result<()> {
    let listener = bind_server_socket().await?;
    let path = socket_path();
    let (server_tx, mut server_rx) = mpsc::unbounded_channel::<ServerEvent>();
    let (js_event_tx, js_event_rx) = mpsc::unbounded_channel::<EditorEvent>();
    let (js_render_tx, mut js_render_rx) = mpsc::unbounded_channel::<RenderCommand>();
    let js_thread = spawn_js_runtime(js_event_rx, js_render_tx);
    let mut server = EditorServer::default();

    println!("Server: listening on {}", path.display());

    let result = loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _) = match accept_result {
                    Ok(value) => value,
                    Err(err) => break Err(err),
                };

                let client_id = server.allocate_client_id();
                let (client_tx, client_rx) = mpsc::unbounded_channel::<ServerToClient>();
                server.register_client(client_id, client_tx);

                println!("Server: client {client_id:?} connected");

                tokio::spawn(handle_client(client_id, stream, server_tx.clone(), client_rx));
            }
            event = server_rx.recv() => {
                let Some(event) = event else {
                    break Ok(());
                };

                match event {
                    ServerEvent::ClientMessage { client_id, message } => {
                        if server.handle_message(client_id, message, &js_event_tx) {
                            break Ok(());
                        }
                    }
                    ServerEvent::ClientDisconnected { client_id } => {
                        server.remove_client(client_id);
                        println!("Server: client {client_id:?} disconnected");
                    }
                    ServerEvent::JsRenderCommand(command) => {
                        println!("Server: render command from JS runtime: {command:?}");
                    }
                }
            }
            command = js_render_rx.recv() => {
                if let Some(command) = command {
                    let _ = server_tx.send(ServerEvent::JsRenderCommand(command));
                }
            }
        }
    };

    drop(js_event_tx);

    if let Err(err) = js_thread.join() {
        eprintln!("Server: JS runtime thread panicked: {err:?}");
    }

    if let Err(err) = fs::remove_file(&path)
        && err.kind() != io::ErrorKind::NotFound
    {
        eprintln!(
            "Server: failed to remove socket file {}: {err}",
            path.display()
        );
    }

    result
}

fn key_input_to_command(event: &KeyInputEvent) -> Option<EditorCommand> {
    if event.ctrl || event.alt || event.meta {
        return None;
    }

    if event.logical_key.eq_ignore_ascii_case("backspace") {
        return Some(EditorCommand::Backspace);
    }

    if event.logical_key.eq_ignore_ascii_case("arrowleft")
        || event.logical_key.eq_ignore_ascii_case("left")
    {
        return Some(EditorCommand::MoveCursorLeft);
    }

    if event.logical_key.eq_ignore_ascii_case("arrowright")
        || event.logical_key.eq_ignore_ascii_case("right")
    {
        return Some(EditorCommand::MoveCursorRight);
    }

    let text = event.text.as_deref()?;
    if text.is_empty() {
        return None;
    }

    Some(EditorCommand::InsertText { text: text.into() })
}

fn should_forward_to_js_runtime(event: &KeyInputEvent, handled_by_rust: bool) -> bool {
    if handled_by_rust {
        return false;
    }

    // Temporary Phase 10 policy:
    // - Rust handles ordinary text/backspace/left/right directly.
    // - Unhandled keys (including modified chords) are forwarded to JS for logging/debugging.
    // - Later, extension keybindings will register with Rust and drive selective JS invocation.
    event.ctrl || event.alt || event.meta || event.shift || event.text.is_none()
}

async fn handle_client(
    client_id: ClientId,
    stream: UnixStream,
    server_tx: mpsc::UnboundedSender<ServerEvent>,
    mut outbound_rx: mpsc::UnboundedReceiver<ServerToClient>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    if let Err(err) = write_json_line(&mut writer, &ServerToClient::Welcome { client_id }).await {
        eprintln!("Server: failed to welcome {client_id:?}: {err}");
        let _ = server_tx.send(ServerEvent::ClientDisconnected { client_id });
        return;
    }

    loop {
        tokio::select! {
            incoming = read_json_line::<_, ClientToServer>(&mut reader) => {
                match incoming {
                    Ok(Some(message)) => {
                        let should_disconnect = matches!(
                            message,
                            ClientToServer::CloseClient { client_id: close_id } if close_id == client_id
                        );

                        if server_tx
                            .send(ServerEvent::ClientMessage { client_id, message })
                            .is_err()
                        {
                            break;
                        }

                        if should_disconnect {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(err) => {
                        eprintln!("Server: client {client_id:?} read error: {err}");
                        break;
                    }
                }
            }
            outbound = outbound_rx.recv() => {
                let Some(message) = outbound else {
                    break;
                };

                if let Err(err) = write_json_line(&mut writer, &message).await {
                    eprintln!("Server: failed to send message to {client_id:?}: {err}");
                    break;
                }
            }
        }
    }

    let _ = server_tx.send(ServerEvent::ClientDisconnected { client_id });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_event(logical_key: &str, text: Option<&str>) -> KeyInputEvent {
        KeyInputEvent {
            logical_key: logical_key.into(),
            physical_key: String::new(),
            text: text.map(str::to_string),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            repeat: false,
        }
    }

    #[test]
    fn translates_text_input_to_insert_text_command() {
        let event = key_event("a", Some("a"));
        assert_eq!(
            key_input_to_command(&event),
            Some(EditorCommand::InsertText { text: "a".into() })
        );
    }

    #[test]
    fn translates_backspace_and_arrows() {
        assert_eq!(
            key_input_to_command(&key_event("Backspace", None)),
            Some(EditorCommand::Backspace)
        );
        assert_eq!(
            key_input_to_command(&key_event("ArrowLeft", None)),
            Some(EditorCommand::MoveCursorLeft)
        );
        assert_eq!(
            key_input_to_command(&key_event("ArrowRight", None)),
            Some(EditorCommand::MoveCursorRight)
        );
    }

    #[test]
    fn ignores_modified_keys_for_now() {
        let mut event = key_event("a", Some("a"));
        event.ctrl = true;
        assert_eq!(key_input_to_command(&event), None);
    }

    #[test]
    fn forwarding_policy_does_not_forward_rust_handled_keys() {
        let event = key_event("a", Some("a"));
        assert!(!should_forward_to_js_runtime(&event, true));
    }

    #[test]
    fn forwarding_policy_forwards_unhandled_modified_or_special_keys() {
        let mut modified = key_event("a", Some("a"));
        modified.ctrl = true;
        assert!(should_forward_to_js_runtime(&modified, false));

        let special = key_event("Escape", None);
        assert!(should_forward_to_js_runtime(&special, false));
    }
}
