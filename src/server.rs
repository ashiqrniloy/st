use std::{
    collections::HashMap,
    fs, io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::{io::BufReader, net::UnixStream, sync::mpsc};

use crate::{
    cli::ServerOptions,
    commands::{CommandId, CommandInvocation, CommandRegistry, editor_command_to_invocation},
    configuration::{DEFAULT_CURSOR_VISIBLE, DEFAULT_EDITOR_BACKGROUND_COLOR},
    documentation::{DocumentationQuery, DocumentationRegistryKind, DocumentationResult},
    editor::EditorState,
    events::{EditorEvent, KeyInputEvent, RenderCommand, ScenePatch, SceneUpdate, Viewport},
    ipc::{bind_server_socket, connect_to_server, read_json_line, socket_path, write_json_line},
    js_runtime::spawn_js_runtime,
    protocol::{ClientId, ClientToServer, ServerToClient},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientConnectionState {
    Connected,
    Closing,
}

#[derive(Debug)]
struct ClientConnection {
    state: ClientConnectionState,
    tx: mpsc::UnboundedSender<ServerToClient>,
    viewport: Viewport,
    pending_chord: Vec<SimpleChord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EditorSceneSettings {
    background_color: u32,
    cursor_visible: bool,
}

impl Default for EditorSceneSettings {
    fn default() -> Self {
        Self {
            background_color: DEFAULT_EDITOR_BACKGROUND_COLOR,
            cursor_visible: DEFAULT_CURSOR_VISIBLE,
        }
    }
}

#[derive(Debug)]
pub struct EditorServer {
    // Performance guardrail: Rust owns typing/cursor/edit/scene hot paths.
    editor: EditorState,
    scene_settings: EditorSceneSettings,
    command_registry: CommandRegistry,
    keymap: Keymap,
    next_client_id: u64,
    clients: HashMap<ClientId, ClientConnection>,
    metrics: PerformanceMetrics,
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
    OutboundDequeued {
        client_id: ClientId,
    },
    JsRenderCommand(RenderCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SimpleChord {
    key: String,
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct KeyChord(Vec<SimpleChord>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyResolution {
    Exact,
    Prefix,
    None,
}

#[derive(Debug, Default)]
struct Keymap {
    bindings: HashMap<KeyChord, CommandId>,
}

impl Keymap {
    fn bind(&mut self, chord: KeyChord, command_id: CommandId) {
        self.bindings.insert(chord, command_id);
    }

    fn resolve_state(&self, chord: &[SimpleChord]) -> KeyResolution {
        let mut prefix = false;
        for registered in self.bindings.keys() {
            if registered.0 == chord {
                return KeyResolution::Exact;
            }
            if registered.0.starts_with(chord) {
                prefix = true;
            }
        }
        if prefix {
            KeyResolution::Prefix
        } else {
            KeyResolution::None
        }
    }

    fn lookup(&self, chord: &[SimpleChord]) -> Option<CommandId> {
        self.bindings.get(&KeyChord(chord.to_vec())).cloned()
    }
}

#[derive(Debug, Default)]
struct PerformanceMetrics {
    key_to_scene_samples: u64,
    key_to_scene_total: Duration,
    scene_to_client_samples: u64,
    scene_to_client_total: Duration,
    ipc_inbound_bytes: u64,
    ipc_outbound_bytes: u64,
    server_event_queue_depth: usize,
    server_event_queue_max_depth: usize,
    per_client_outbound_queue_depth: HashMap<ClientId, usize>,
    per_client_outbound_queue_max_depth: HashMap<ClientId, usize>,
}

impl PerformanceMetrics {
    fn record_server_queue_enqueue(&mut self) {
        self.server_event_queue_depth += 1;
        self.server_event_queue_max_depth = self
            .server_event_queue_max_depth
            .max(self.server_event_queue_depth);
    }

    fn record_server_queue_dequeue(&mut self) {
        self.server_event_queue_depth = self.server_event_queue_depth.saturating_sub(1);
    }

    fn record_outbound_enqueue(&mut self, client_id: ClientId) {
        let depth = self
            .per_client_outbound_queue_depth
            .entry(client_id)
            .or_default();
        *depth += 1;
        let max_depth = self
            .per_client_outbound_queue_max_depth
            .entry(client_id)
            .or_default();
        *max_depth = (*max_depth).max(*depth);
    }

    fn record_outbound_dequeue(&mut self, client_id: ClientId) {
        let depth = self
            .per_client_outbound_queue_depth
            .entry(client_id)
            .or_default();
        *depth = depth.saturating_sub(1);
    }
}

impl Default for EditorServer {
    fn default() -> Self {
        crate::documentation::validate_builtin_docs()
            .expect("builtin documentation registration should be valid");

        let mut keymap = Keymap::default();
        keymap.bind(
            KeyChord(vec![SimpleChord {
                key: "backspace".into(),
                ctrl: false,
                alt: false,
                shift: false,
                meta: false,
            }]),
            CommandId::new("editor.backspace").expect("builtin id"),
        );

        Self {
            editor: EditorState::default(),
            scene_settings: EditorSceneSettings::default(),
            command_registry: CommandRegistry::with_builtin_commands()
                .expect("builtin command registration should be valid"),
            keymap,
            next_client_id: 1,
            clients: HashMap::new(),
            metrics: PerformanceMetrics::default(),
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
        self.clients.insert(
            client_id,
            ClientConnection {
                state: ClientConnectionState::Connected,
                tx,
                viewport: Viewport::default(),
                pending_chord: Vec::new(),
            },
        );
    }

    fn mark_client_closing(&mut self, client_id: ClientId) {
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.state = ClientConnectionState::Closing;
        }
    }

    fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
    }

    fn scene_snapshot_for_viewport(&self, viewport: Viewport) -> SceneUpdate {
        let (start_char, end_char, text) = self.visible_text_for_viewport(viewport);
        let cursor = self.editor.cursor.clamp(start_char, end_char) - start_char;
        SceneUpdate {
            background_color: self.scene_settings.background_color,
            text,
            cursor_char_index: cursor,
            cursor_visible: self.scene_settings.cursor_visible,
        }
    }

    fn visible_text_for_viewport(&self, viewport: Viewport) -> (usize, usize, String) {
        let total_lines = self.editor.buffer.full_text().lines().count().max(1);
        let start_line = viewport.start_line.min(total_lines.saturating_sub(1));
        let end_line = viewport.end_line.max(start_line + 1).min(total_lines);
        let start_char = self.editor.buffer.line_col_to_char(start_line, 0);
        let end_char = if end_line >= total_lines {
            self.editor.buffer.char_len()
        } else {
            self.editor.buffer.line_col_to_char(end_line, 0)
        };
        let text = self.editor.buffer.slice_chars(start_char, end_char);
        (start_char, end_char, text)
    }

    fn send_scene_snapshot_to_client(&mut self, client_id: ClientId) {
        let viewport = self
            .clients
            .get(&client_id)
            .map(|client| client.viewport)
            .unwrap_or_default();
        self.send_to_client(
            client_id,
            ServerToClient::SceneSnapshot(self.scene_snapshot_for_viewport(viewport)),
        );
    }

    fn send_scene_patch_updates(&mut self) {
        let started = Instant::now();
        let targets: Vec<(ClientId, Viewport)> = self
            .clients
            .iter()
            .filter(|(_, client)| client.state == ClientConnectionState::Connected)
            .map(|(id, client)| (*id, client.viewport))
            .collect();

        for (client_id, viewport) in targets {
            let (start_char, end_char, text) = self.visible_text_for_viewport(viewport);
            let cursor = self.editor.cursor.clamp(start_char, end_char) - start_char;
            self.send_to_client(
                client_id,
                ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate {
                    start_line: viewport.start_line,
                    end_line: viewport.end_line,
                    text,
                    cursor_char_index: cursor,
                    cursor_visible: self.scene_settings.cursor_visible,
                }),
            );
        }

        self.metrics.scene_to_client_samples += 1;
        self.metrics.scene_to_client_total += started.elapsed();
    }

    fn send_to_client(&mut self, client_id: ClientId, message: ServerToClient) {
        if let Ok(encoded) = serde_json::to_vec(&message) {
            self.metrics.ipc_outbound_bytes += (encoded.len() + 1) as u64;
        }

        self.metrics.record_outbound_enqueue(client_id);
        let should_remove = self
            .clients
            .get(&client_id)
            .is_some_and(|client| client.tx.send(message).is_err());

        if should_remove {
            self.remove_client(client_id);
        }
    }

    fn send_to_all(&mut self, message: ServerToClient) {
        if let Ok(encoded) = serde_json::to_vec(&message) {
            self.metrics.ipc_outbound_bytes += (encoded.len() + 1) as u64;
        }

        let dead_clients: Vec<ClientId> = self
            .clients
            .iter()
            .filter_map(|(client_id, client)| {
                if client.state == ClientConnectionState::Closing {
                    return None;
                }

                self.metrics.record_outbound_enqueue(*client_id);
                if client.tx.send(message.clone()).is_err() {
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

    fn dispatch_command_id(
        &mut self,
        client_id: ClientId,
        command_id: CommandId,
        js_event_tx: &mpsc::UnboundedSender<EditorEvent>,
    ) {
        match self.command_registry.handler_for(&command_id).cloned() {
            Some(crate::commands::CommandHandler::RustBuiltin(_)) => {
                if let Err(err) = self
                    .command_registry
                    .execute(CommandInvocation::new(command_id), &mut self.editor)
                {
                    self.send_to_client(client_id, ServerToClient::Error { message: err });
                } else {
                    self.send_scene_patch_updates();
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
            if let Err(err) = self.command_registry.execute(invocation, &mut self.editor) {
                self.send_to_client(client_id, ServerToClient::Error { message: err });
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
    fn handle_message(
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
                let key_to_scene_started = Instant::now();
                self.handle_key_input(client_id, event, js_event_tx);
                self.metrics.key_to_scene_samples += 1;
                self.metrics.key_to_scene_total += key_to_scene_started.elapsed();
            }
            ClientToServer::Command(command) => {
                let key_to_scene_started = Instant::now();
                let invocation = match editor_command_to_invocation(command) {
                    Ok(invocation) => invocation,
                    Err(err) => {
                        self.send_to_client(client_id, ServerToClient::Error { message: err });
                        return false;
                    }
                };

                if let Err(err) = self.command_registry.execute(invocation, &mut self.editor) {
                    self.send_to_client(client_id, ServerToClient::Error { message: err });
                } else {
                    self.send_scene_patch_updates();
                    self.metrics.key_to_scene_samples += 1;
                    self.metrics.key_to_scene_total += key_to_scene_started.elapsed();
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
                extension_id: _,
                chord,
                command_id,
            } => match (parse_key_chord(&chord), CommandId::new(command_id)) {
                (Ok(chord), Ok(command_id)) => self.keymap.bind(chord, command_id),
                (Err(err), _) | (_, Err(err)) => {
                    self.send_to_client(client_id, ServerToClient::Error { message: err });
                }
            },
        }

        false
    }

    fn handle_documentation_query(&self, query: DocumentationQuery) -> DocumentationResult {
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
            DocumentationQuery::ListKeybindings => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Keybindings,
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
            DocumentationQuery::ListApis => DocumentationResult::EmptyPlaceholder {
                registry: DocumentationRegistryKind::Apis,
            },
        }
    }
}

pub fn run_foreground(options: ServerOptions) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to build server runtime: {err}"))?;

    runtime
        .block_on(run_server(options))
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

async fn run_server(options: ServerOptions) -> io::Result<()> {
    let listener = bind_server_socket().await?;
    let idle_timeout = options.idle_timeout_secs.map(Duration::from_secs);
    let path = socket_path();
    let (server_tx, mut server_rx) = mpsc::unbounded_channel::<ServerEvent>();
    let server_queue_depth = Arc::new(AtomicUsize::new(0));
    let (js_event_tx, js_event_rx) = mpsc::unbounded_channel::<EditorEvent>();
    let (js_render_tx, mut js_render_rx) = mpsc::unbounded_channel::<RenderCommand>();
    let js_thread = spawn_js_runtime(js_event_rx, js_render_tx);
    let mut server = EditorServer::default();

    println!("Server: listening on {}", path.display());
    if let Some(timeout) = idle_timeout {
        println!(
            "Server: idle shutdown enabled after {} seconds with no connected clients",
            timeout.as_secs()
        );
    }

    let mut idle_deadline: Option<tokio::time::Instant> = None;

    let result = loop {
        tokio::select! {
            _ = async {
                if let Some(deadline) = idle_deadline {
                    tokio::time::sleep_until(deadline).await;
                }
            }, if idle_deadline.is_some() => {
                println!("Server: idle timeout reached with no clients; shutting down");
                break Ok(());
            }
            accept_result = listener.accept() => {
                let (stream, _) = match accept_result {
                    Ok(value) => value,
                    Err(err) => break Err(err),
                };

                let client_id = server.allocate_client_id();
                let (client_tx, client_rx) = mpsc::unbounded_channel::<ServerToClient>();
                server.register_client(client_id, client_tx);

                println!("Server: client {client_id:?} connected");
                idle_deadline = None;

                tokio::spawn(handle_client(
                    client_id,
                    stream,
                    server_tx.clone(),
                    server_queue_depth.clone(),
                    client_rx,
                ));
            }
            event = server_rx.recv() => {
                let Some(event) = event else {
                    break Ok(());
                };

                server_queue_depth.fetch_sub(1, Ordering::Relaxed);
                server.metrics.record_server_queue_dequeue();

                match event {
                    ServerEvent::ClientMessage { client_id, message } => {
                        if let Ok(encoded) = serde_json::to_vec(&message) {
                            server.metrics.ipc_inbound_bytes += (encoded.len() + 1) as u64;
                        }
                        if server.handle_message(client_id, message, &js_event_tx) {
                            break Ok(());
                        }
                    }
                    ServerEvent::ClientDisconnected { client_id } => {
                        server.metrics.per_client_outbound_queue_depth.remove(&client_id);
                        server.metrics.per_client_outbound_queue_max_depth.remove(&client_id);
                        server.remove_client(client_id);
                        println!("Server: client {client_id:?} disconnected");
                        if server.clients.is_empty() {
                            idle_deadline = idle_timeout
                                .map(|timeout| tokio::time::Instant::now() + timeout);
                        }
                    }
                    ServerEvent::OutboundDequeued { client_id } => {
                        server.metrics.record_outbound_dequeue(client_id);
                    }
                    ServerEvent::JsRenderCommand(command) => {
                        println!("Server: render command from JS runtime: {command:?}");
                    }
                }
            }
            command = js_render_rx.recv() => {
                if let Some(command) = command {
                    server_queue_depth.fetch_add(1, Ordering::Relaxed);
                    server.metrics.record_server_queue_enqueue();
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

fn normalize_key_input(event: &KeyInputEvent) -> SimpleChord {
    SimpleChord {
        key: event.logical_key.to_ascii_lowercase(),
        ctrl: event.ctrl,
        alt: event.alt,
        shift: event.shift,
        meta: event.meta,
    }
}

fn parse_key_chord(chord: &str) -> Result<KeyChord, String> {
    let mut parts = Vec::new();
    for part in chord.split_whitespace() {
        let mut simple = SimpleChord {
            key: String::new(),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        };

        for piece in part.split('+') {
            match piece.to_ascii_lowercase().as_str() {
                "ctrl" => simple.ctrl = true,
                "alt" => simple.alt = true,
                "shift" => simple.shift = true,
                "meta" | "cmd" => simple.meta = true,
                key => simple.key = key.to_string(),
            }
        }

        if simple.key.is_empty() {
            return Err(format!("invalid key chord segment: {part}"));
        }
        parts.push(simple);
    }

    if parts.is_empty() {
        return Err("key chord must not be empty".into());
    }

    Ok(KeyChord(parts))
}

fn key_input_to_command_invocation(event: &KeyInputEvent) -> Option<CommandInvocation> {
    if event.ctrl || event.alt || event.meta {
        return None;
    }

    if event.logical_key.eq_ignore_ascii_case("backspace") {
        return Some(CommandInvocation::new(
            CommandId::new("editor.backspace").ok()?,
        ));
    }

    if event.logical_key.eq_ignore_ascii_case("arrowleft")
        || event.logical_key.eq_ignore_ascii_case("left")
    {
        return Some(CommandInvocation::new(
            CommandId::new("editor.move_cursor_left").ok()?,
        ));
    }

    if event.logical_key.eq_ignore_ascii_case("arrowright")
        || event.logical_key.eq_ignore_ascii_case("right")
    {
        return Some(CommandInvocation::new(
            CommandId::new("editor.move_cursor_right").ok()?,
        ));
    }

    let text = event.text.as_deref()?;
    if text.is_empty() {
        return None;
    }

    Some(CommandInvocation::with_text(
        CommandId::new("editor.insert_text").ok()?,
        text,
    ))
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
    server_queue_depth: Arc<AtomicUsize>,
    mut outbound_rx: mpsc::UnboundedReceiver<ServerToClient>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    if let Err(err) = write_json_line(&mut writer, &ServerToClient::Welcome { client_id }).await {
        eprintln!("Server: failed to welcome {client_id:?}: {err}");
        server_queue_depth.fetch_add(1, Ordering::Relaxed);
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

                        server_queue_depth.fetch_add(1, Ordering::Relaxed);
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

                server_queue_depth.fetch_add(1, Ordering::Relaxed);
                let _ = server_tx.send(ServerEvent::OutboundDequeued { client_id });

                if let Err(err) = write_json_line(&mut writer, &message).await {
                    eprintln!("Server: failed to send message to {client_id:?}: {err}");
                    break;
                }
            }
        }
    }

    server_queue_depth.fetch_add(1, Ordering::Relaxed);
    let _ = server_tx.send(ServerEvent::ClientDisconnected { client_id });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        documentation::DocumentationQuery,
        events::{EditorCommand, EditorEvent},
    };

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
            key_input_to_command_invocation(&event),
            Some(CommandInvocation::with_text(
                CommandId::new("editor.insert_text").expect("id"),
                "a"
            ))
        );
    }

    #[test]
    fn translates_backspace_and_arrows() {
        assert_eq!(
            key_input_to_command_invocation(&key_event("Backspace", None)),
            Some(CommandInvocation::new(
                CommandId::new("editor.backspace").expect("id")
            ))
        );
        assert_eq!(
            key_input_to_command_invocation(&key_event("ArrowLeft", None)),
            Some(CommandInvocation::new(
                CommandId::new("editor.move_cursor_left").expect("id")
            ))
        );
        assert_eq!(
            key_input_to_command_invocation(&key_event("ArrowRight", None)),
            Some(CommandInvocation::new(
                CommandId::new("editor.move_cursor_right").expect("id")
            ))
        );
    }

    #[test]
    fn ignores_modified_keys_for_now() {
        let mut event = key_event("a", Some("a"));
        event.ctrl = true;
        assert_eq!(key_input_to_command_invocation(&event), None);
    }

    #[test]
    fn forwarding_policy_does_not_forward_rust_handled_keys() {
        let event = key_event("a", Some("a"));
        assert!(!should_forward_to_js_runtime(&event, true));
    }

    #[test]
    fn broadcasts_editor_updates_to_all_connected_clients() {
        let mut server = EditorServer::default();
        let (client_1_tx, mut client_1_rx) = mpsc::unbounded_channel();
        let (client_2_tx, mut client_2_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();

        let client_1 = ClientId(1);
        let client_2 = ClientId(2);
        server.register_client(client_1, client_1_tx);
        server.register_client(client_2, client_2_tx);

        assert!(!server.handle_message(
            client_1,
            ClientToServer::Command(EditorCommand::InsertText { text: "x".into() }),
            &js_tx,
        ));

        assert_eq!(
            client_1_rx.try_recv(),
            Ok(ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate {
                start_line: 0,
                end_line: 200,
                text: "x".into(),
                cursor_char_index: 1,
                cursor_visible: true,
            }))
        );
        assert_eq!(client_1_rx.try_recv(), Err(mpsc::error::TryRecvError::Empty));
        assert_eq!(
            client_2_rx.try_recv(),
            Ok(ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate {
                start_line: 0,
                end_line: 200,
                text: "x".into(),
                cursor_char_index: 1,
                cursor_visible: true,
            }))
        );
        assert_eq!(client_2_rx.try_recv(), Err(mpsc::error::TryRecvError::Empty));
    }

    #[test]
    fn close_client_only_closes_the_sending_connection() {
        let mut server = EditorServer::default();
        let (client_1_tx, _client_1_rx) = mpsc::unbounded_channel();
        let (client_2_tx, _client_2_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();

        let client_1 = ClientId(1);
        let client_2 = ClientId(2);
        server.register_client(client_1, client_1_tx);
        server.register_client(client_2, client_2_tx);

        assert!(!server.handle_message(
            client_1,
            ClientToServer::CloseClient {
                client_id: client_2,
            },
            &js_tx,
        ));

        assert!(!server.clients.contains_key(&client_1));
        assert!(server.clients.contains_key(&client_2));
    }

    #[test]
    fn shutdown_notifies_connected_clients() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();

        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(server.handle_message(client_id, ClientToServer::ShutdownServer, &js_tx));
        assert_eq!(
            client_rx.try_recv(),
            Ok(ServerToClient::ServerShuttingDown {
                reason: "server shutdown requested".into(),
            })
        );
    }

    #[test]
    fn parses_multi_key_chords() {
        let chord = parse_key_chord("ctrl+x ctrl+s").expect("parse chord");
        assert_eq!(chord.0.len(), 2);
        assert!(chord.0[0].ctrl);
        assert_eq!(chord.0[0].key, "x");
        assert!(chord.0[1].ctrl);
        assert_eq!(chord.0[1].key, "s");
    }

    #[test]
    fn forwarding_policy_forwards_unhandled_modified_or_special_keys() {
        let mut modified = key_event("a", Some("a"));
        modified.ctrl = true;
        assert!(should_forward_to_js_runtime(&modified, false));

        let special = key_event("Escape", None);
        assert!(should_forward_to_js_runtime(&special, false));
    }

    #[test]
    fn documentation_query_lists_commands() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::DocumentationQuery(DocumentationQuery::ListCommands),
            &js_tx,
        ));

        let ServerToClient::DocumentationResult(DocumentationResult::CommandList { commands }) =
            client_rx.try_recv().expect("docs result")
        else {
            panic!("expected command list");
        };

        assert!(commands.iter().any(|c| c.id == "editor.insert_text"));
    }

    #[test]
    fn documentation_query_describes_command() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::DocumentationQuery(DocumentationQuery::DescribeCommand {
                command_id: "editor.insert_text".into(),
            }),
            &js_tx,
        ));

        let ServerToClient::DocumentationResult(DocumentationResult::CommandDetails { command }) =
            client_rx.try_recv().expect("docs result")
        else {
            panic!("expected command details");
        };

        assert_eq!(command.id, "editor.insert_text");
    }

    #[test]
    fn documentation_query_returns_error_for_unknown_command() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::DocumentationQuery(DocumentationQuery::DescribeCommand {
                command_id: "missing.command".into(),
            }),
            &js_tx,
        ));

        let ServerToClient::DocumentationResult(DocumentationResult::QueryError { message }) =
            client_rx.try_recv().expect("docs result")
        else {
            panic!("expected query error");
        };

        assert!(message.contains("unknown command id"));
    }

    #[test]
    fn documentation_query_lists_builtin_settings() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::DocumentationQuery(DocumentationQuery::ListSettings),
            &js_tx,
        ));

        let ServerToClient::DocumentationResult(DocumentationResult::SettingsList { settings }) =
            client_rx.try_recv().expect("docs result")
        else {
            panic!("expected settings list");
        };

        assert!(settings.iter().any(|s| s.id == "server.idle_timeout_secs"));
    }

    #[test]
    fn scene_update_uses_configurable_scene_settings() {
        let mut server = EditorServer::default();
        server.scene_settings = EditorSceneSettings {
            background_color: 0x112233,
            cursor_visible: false,
        };

        let scene = server.scene_snapshot_for_viewport(Viewport::default());
        assert_eq!(scene.background_color, 0x112233);
        assert!(!scene.cursor_visible);
    }

    #[test]
    fn rust_handled_key_updates_key_to_scene_metrics() {
        let mut server = EditorServer::default();
        let (client_tx, _client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::KeyInput(key_event("a", Some("a"))),
            &js_tx,
        ));

        assert_eq!(server.metrics.key_to_scene_samples, 1);
    }

    #[test]
    fn outbound_queue_depth_metrics_track_enqueue_and_dequeue() {
        let mut metrics = PerformanceMetrics::default();
        let client_id = ClientId(7);

        metrics.record_outbound_enqueue(client_id);
        metrics.record_outbound_enqueue(client_id);
        metrics.record_outbound_dequeue(client_id);

        assert_eq!(
            metrics
                .per_client_outbound_queue_depth
                .get(&client_id)
                .copied(),
            Some(1)
        );
        assert_eq!(
            metrics
                .per_client_outbound_queue_max_depth
                .get(&client_id)
                .copied(),
            Some(2)
        );
    }

    #[test]
    fn rust_builtin_key_dispatch_does_not_invoke_js_runtime() {
        let mut server = EditorServer::default();
        let (client_tx, _client_rx) = mpsc::unbounded_channel();
        let (js_tx, mut js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::KeyInput(key_event("a", Some("a"))),
            &js_tx,
        ));

        assert!(js_rx.try_recv().is_err());
    }

    #[test]
    fn js_keybinding_dispatch_invokes_js_runtime() {
        let mut server = EditorServer::default();
        let (client_tx, _client_rx) = mpsc::unbounded_channel();
        let (js_tx, mut js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        assert!(!server.handle_message(
            client_id,
            ClientToServer::RegisterJsCommand {
                extension_id: "ext.test".into(),
                command_id: "ext.say_hello".into(),
                title: "Say Hello".into(),
                description: "Test command".into(),
                category: "extension".into(),
            },
            &js_tx,
        ));

        assert!(!server.handle_message(
            client_id,
            ClientToServer::RegisterJsKeybinding {
                extension_id: "ext.test".into(),
                chord: "ctrl+d".into(),
                command_id: "ext.say_hello".into(),
            },
            &js_tx,
        ));

        let mut event = key_event("d", None);
        event.ctrl = true;
        assert!(!server.handle_message(client_id, ClientToServer::KeyInput(event), &js_tx));

        assert_eq!(
            js_rx.try_recv(),
            Ok(EditorEvent::ExecuteJsCommand {
                extension_id: "ext.test".into(),
                command_id: "ext.say_hello".into(),
            })
        );
    }

    #[test]
    fn malformed_js_keybinding_registration_cannot_mutate_editor_state() {
        let mut server = EditorServer::default();
        let (client_tx, mut client_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();
        let client_id = ClientId(1);
        server.register_client(client_id, client_tx);

        let before = server.editor.buffer.full_text();
        assert!(!server.handle_message(
            client_id,
            ClientToServer::RegisterJsKeybinding {
                extension_id: "ext.test".into(),
                chord: "ctrl+".into(),
                command_id: "ext.missing".into(),
            },
            &js_tx,
        ));

        assert_eq!(server.editor.buffer.full_text(), before);
        assert!(matches!(
            client_rx.try_recv(),
            Ok(ServerToClient::Error { .. })
        ));
    }

    #[test]
    fn per_client_viewports_receive_independent_visible_text_updates() {
        let mut server = EditorServer::default();
        let (client_1_tx, mut client_1_rx) = mpsc::unbounded_channel();
        let (client_2_tx, mut client_2_rx) = mpsc::unbounded_channel();
        let (js_tx, _js_rx) = mpsc::unbounded_channel();

        let client_1 = ClientId(1);
        let client_2 = ClientId(2);
        server.register_client(client_1, client_1_tx);
        server.register_client(client_2, client_2_tx);
        server.editor.insert_text("line1\nline2\nline3\n");

        assert!(!server.handle_message(
            client_1,
            ClientToServer::SetViewport {
                viewport: Viewport {
                    start_line: 0,
                    end_line: 1,
                },
            },
            &js_tx,
        ));
        assert!(!server.handle_message(
            client_2,
            ClientToServer::SetViewport {
                viewport: Viewport {
                    start_line: 1,
                    end_line: 2,
                },
            },
            &js_tx,
        ));

        let _ = client_1_rx.try_recv();
        let _ = client_2_rx.try_recv();

        assert!(!server.handle_message(
            client_1,
            ClientToServer::Command(EditorCommand::InsertText { text: "x".into() }),
            &js_tx,
        ));

        let c1 = client_1_rx.try_recv().expect("client1 patch");
        let c2 = client_2_rx.try_recv().expect("client2 patch");

        match c1 {
            ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate { text, .. }) => {
                assert!(text.contains("line1"));
            }
            other => panic!("expected visible text patch for client1, got {other:?}"),
        }

        match c2 {
            ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate { text, .. }) => {
                assert!(text.contains("line2"));
            }
            other => panic!("expected visible text patch for client2, got {other:?}"),
        }
    }
}
