use std::{
    fs, io,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::sync::mpsc;

use crate::{
    cli::ServerOptions,
    events::{EditorEvent, RenderCommand},
    ipc::{bind_server_socket, connect_to_server, socket_path, write_json_line},
    js_runtime::{JsRuntimeCommand, spawn_js_runtime},
    protocol::{ClientId, ClientToServer, ServerToClient},
};

use super::{socket_task::handle_client, state::EditorServer};

#[derive(Debug)]
pub(super) enum ServerEvent {
    ClientMessage {
        client_id: ClientId,
        message: ClientToServer,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    JsRenderCommand(RenderCommand),
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
    let (js_command_tx, mut js_command_rx) = mpsc::unbounded_channel::<JsRuntimeCommand>();
    let js_thread = spawn_js_runtime(js_event_rx, js_render_tx, js_command_tx);
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
                let (client_tx, client_rx) = mpsc::channel::<ServerToClient>(256);
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

                match event {
                    ServerEvent::ClientMessage { client_id, message } => {
                        if server.handle_message(client_id, message, &js_event_tx) {
                            break Ok(());
                        }
                    }
                    ServerEvent::ClientDisconnected { client_id } => {
                        server.remove_client(client_id);
                        println!("Server: client {client_id:?} disconnected");
                        if server.clients.is_empty() {
                            idle_deadline = idle_timeout
                                .map(|timeout| tokio::time::Instant::now() + timeout);
                        }
                    }
                    ServerEvent::JsRenderCommand(command) => {
                        println!("Server: render command from JS runtime: {command:?}");
                    }
                }
            }
            command = js_render_rx.recv() => {
                if let Some(command) = command {
                    server_queue_depth.fetch_add(1, Ordering::Relaxed);
                    let _ = server_tx.send(ServerEvent::JsRenderCommand(command));
                }
            }
            command = js_command_rx.recv() => {
                if let Some(command) = command {
                    match command {
                        JsRuntimeCommand::EditorCommand(command) => {
                            if let Some(client_id) = server.clients.keys().next().copied() {
                                let _ = server.handle_message(client_id, ClientToServer::Command(command), &js_event_tx);
                            }
                        }
                        JsRuntimeCommand::BindKey { chord, command_id } => {
                            if let Err(err) = server.bind_config_keybinding(chord, command_id) {
                                eprintln!("Config keybinding registration failed: {err}");
                            }
                        }
                    }
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
