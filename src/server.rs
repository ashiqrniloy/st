use std::{collections::HashSet, fs, io};

use tokio::{io::BufReader, net::UnixStream, sync::mpsc};

use crate::{
    editor::EditorState,
    events::{EditorEvent, RenderCommand},
    ipc::{bind_server_socket, connect_to_server, read_json_line, socket_path, write_json_line},
    js_runtime::spawn_js_runtime,
    protocol::{ClientId, ClientToServer, ServerToClient},
};

#[derive(Debug)]
pub struct EditorServer {
    editor: EditorState,
    next_client_id: u64,
    clients: HashSet<ClientId>,
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
        }
    }
}

impl EditorServer {
    fn allocate_client_id(&mut self) -> ClientId {
        let id = ClientId(self.next_client_id);
        self.next_client_id += 1;
        self.clients.insert(id);
        id
    }

    fn remove_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
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
            }
            ClientToServer::KeyInput(event) => {
                println!("Server: key input from {client_id:?}: {event:?}");
                let _ = js_event_tx.send(EditorEvent::KeyInput(event));
            }
            ClientToServer::Command(command) => {
                println!("Server: command from {client_id:?}: {command:?}");
                self.editor.apply(command);
                println!(
                    "Server: editor state: buffer={:?}, cursor={}",
                    self.editor.buffer, self.editor.cursor
                );
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
                println!("Server: client {client_id:?} connected");

                tokio::spawn(handle_client(client_id, stream, server_tx.clone()));
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

async fn handle_client(
    client_id: ClientId,
    stream: UnixStream,
    server_tx: mpsc::UnboundedSender<ServerEvent>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    if let Err(err) = write_json_line(&mut writer, &ServerToClient::Welcome { client_id }).await {
        eprintln!("Server: failed to welcome {client_id:?}: {err}");
        let _ = server_tx.send(ServerEvent::ClientDisconnected { client_id });
        return;
    }

    loop {
        match read_json_line::<_, ClientToServer>(&mut reader).await {
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

    let _ = server_tx.send(ServerEvent::ClientDisconnected { client_id });
}
