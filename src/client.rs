use std::{
    process::{Command, Stdio},
    sync::mpsc as std_mpsc,
    thread,
    time::Duration,
};

use tokio::{
    io::BufReader,
    net::UnixStream,
    sync::mpsc::{self as tokio_mpsc, error::TrySendError},
    time::sleep,
};

use crate::{
    configuration::{DEFAULT_AUTO_STARTED_IDLE_TIMEOUT_SECS, DEFAULT_LINE_HEIGHT_PX},
    events::{EditorEvent, ScenePatch, Viewport},
    ipc::{connect_to_server, read_json_line, socket_path, write_json_line},
    protocol::{ClientId, ClientToServer, ServerToClient},
    render::{UiChannels, UiSceneEvent, run_ui_with_gpui},
};

#[derive(Debug, Clone, Copy)]
struct SceneVersionState {
    buffer_id: u64,
    buffer_version: u64,
}

fn viewport_for_height(height: u32) -> Viewport {
    let visible_lines = ((height as f32 / DEFAULT_LINE_HEIGHT_PX).floor().max(1.0)) as usize;
    Viewport {
        start_line: 0,
        end_line: visible_lines,
    }
}

fn validate_and_advance_scene_version(
    state: &mut Option<SceneVersionState>,
    patch: &ScenePatch,
) -> Result<(), String> {
    match patch {
        ScenePatch::VisibleTextUpdate {
            buffer_id,
            buffer_version,
            ..
        } => {
            *state = Some(SceneVersionState {
                buffer_id: *buffer_id,
                buffer_version: *buffer_version,
            });
            Ok(())
        }
        ScenePatch::TextEditPatch {
            buffer_id,
            base_version,
            new_version,
            ..
        } => {
            let Some(current) = state.as_mut() else {
                return Err("missing scene snapshot".into());
            };
            if current.buffer_id != *buffer_id || current.buffer_version != *base_version {
                return Err("scene version mismatch".into());
            }
            current.buffer_version = *new_version;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn run() -> Result<(), String> {
    let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<EditorEvent>();
    let (scene_tx, scene_rx) = tokio_mpsc::channel::<UiSceneEvent>(256);
    let (welcome_tx, welcome_rx) = std_mpsc::channel::<Result<ClientId, String>>();

    let ipc_thread = spawn_ipc_thread(ui_rx, scene_tx, welcome_tx);

    let client_id = welcome_rx
        .recv()
        .map_err(|err| format!("failed to receive server welcome: {err}"))??;

    println!("Client: connected as {client_id:?}");

    let ui_result = run_ui_with_gpui(UiChannels { ui_tx, scene_rx });

    if let Err(err) = ipc_thread.join() {
        return Err(format!("client IPC thread panicked: {err:?}"));
    }

    ui_result
}

fn spawn_ipc_thread(
    mut ui_rx: tokio_mpsc::UnboundedReceiver<EditorEvent>,
    scene_tx: tokio_mpsc::Sender<UiSceneEvent>,
    welcome_tx: std_mpsc::Sender<Result<ClientId, String>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                let _ = welcome_tx.send(Err(format!("failed to build client runtime: {err}")));
                return;
            }
        };

        runtime.block_on(async move {
            let stream = match connect_or_start_server().await {
                Ok(stream) => stream,
                Err(err) => {
                    let _ = welcome_tx.send(Err(err));
                    return;
                }
            };

            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            if let Err(err) = write_json_line(&mut writer, &ClientToServer::Hello).await {
                let _ = welcome_tx.send(Err(format!("failed to send hello to server: {err}")));
                return;
            }

            let client_id = match read_json_line::<_, ServerToClient>(&mut reader).await {
                Ok(Some(ServerToClient::Welcome { client_id })) => client_id,
                Ok(Some(other)) => {
                    let _ = welcome_tx.send(Err(format!(
                        "expected welcome from server, received {other:?}"
                    )));
                    return;
                }
                Ok(None) => {
                    let _ = welcome_tx.send(Err("server disconnected before welcome".into()));
                    return;
                }
                Err(err) => {
                    let _ = welcome_tx.send(Err(format!("failed to read server welcome: {err}")));
                    return;
                }
            };

            if welcome_tx.send(Ok(client_id)).is_err() {
                return;
            }

            let _ = write_json_line(
                &mut writer,
                &ClientToServer::SetViewport {
                    viewport: viewport_for_height(crate::configuration::DEFAULT_CLIENT_WINDOW_HEIGHT_PX as u32),
                },
            )
            .await;

            let mut scene_version: Option<SceneVersionState> = None;
            let mut scene_queue_dropped = 0u64;
            let mut scene_queue_coalesced = 0u64;

            loop {
                tokio::select! {
                    ui_event = ui_rx.recv() => {
                        let Some(ui_event) = ui_event else {
                            let _ = write_json_line(
                                &mut writer,
                                &ClientToServer::CloseClient { client_id },
                            ).await;
                            break;
                        };

                        match ui_event {
                            EditorEvent::KeyInput(key_event) => {
                                if let Err(err) = write_json_line(
                                    &mut writer,
                                    &ClientToServer::KeyInput(key_event),
                                ).await {
                                    eprintln!("Client: failed to send key input: {err}");
                                    break;
                                }
                            }
                            EditorEvent::ExecuteJsCommand { .. } => {
                                // UI thread never emits this; server->JS runtime path only.
                            }
                            EditorEvent::WindowDimensionsChanged { width, height } => {
                                if let Err(err) = write_json_line(
                                    &mut writer,
                                    &ClientToServer::SetWindowDimensions { width, height },
                                ).await {
                                    eprintln!("Client: failed to send window dimensions: {err}");
                                    break;
                                }
                            }
                            EditorEvent::ViewportChanged { viewport } => {
                                if let Err(err) = write_json_line(
                                    &mut writer,
                                    &ClientToServer::SetViewport { viewport },
                                ).await {
                                    eprintln!("Client: failed to send viewport: {err}");
                                    break;
                                }
                            }
                            EditorEvent::Shutdown => {
                                let _ = write_json_line(
                                    &mut writer,
                                    &ClientToServer::CloseClient { client_id },
                                ).await;
                                break;
                            }
                        }
                    }
                    server_message = read_json_line::<_, ServerToClient>(&mut reader) => {
                        match server_message {
                            Ok(Some(ServerToClient::Welcome { client_id })) => {
                                println!("Client: unexpected second welcome for {client_id:?}");
                            }
                            Ok(Some(ServerToClient::SceneSnapshot(scene))) => {
                                scene_version = Some(SceneVersionState {
                                    buffer_id: scene.buffer_id,
                                    buffer_version: scene.buffer_version,
                                });
                                if let Err(TrySendError::Full(_)) = scene_tx.try_send(UiSceneEvent::Snapshot(scene)) {
                                    scene_queue_dropped += 1;
                                    let _ = write_json_line(&mut writer, &ClientToServer::ResyncScene).await;
                                }
                            }
                            Ok(Some(ServerToClient::ScenePatch(patch))) => {
                                if validate_and_advance_scene_version(&mut scene_version, &patch).is_ok() {
                                    match scene_tx.try_send(UiSceneEvent::Patch(patch.clone())) {
                                        Ok(()) => {}
                                        Err(TrySendError::Full(_)) => {
                                            if matches!(patch, ScenePatch::CursorUpdate { .. } | ScenePatch::SelectionUpdate { .. }) {
                                                scene_queue_coalesced += 1;
                                            } else {
                                                scene_queue_dropped += 1;
                                                let _ = write_json_line(&mut writer, &ClientToServer::ResyncScene).await;
                                            }
                                        }
                                        Err(TrySendError::Closed(_)) => break,
                                    }
                                } else {
                                    let _ = write_json_line(&mut writer, &ClientToServer::ResyncScene).await;
                                }
                            }
                            Ok(Some(ServerToClient::Render(command))) => {
                                println!("Client: received debug render command: {command:?}");
                            }
                            Ok(Some(ServerToClient::Error { message })) => {
                                eprintln!("Client: server error: {message}");
                            }
                            Ok(Some(ServerToClient::ServerShuttingDown { reason })) => {
                                println!("Client: server is shutting down: {reason}");
                                break;
                            }
                            Ok(Some(ServerToClient::DocumentationResult(result))) => {
                                println!("Client help: {result:?}");
                            }
                            Ok(Some(ServerToClient::CommandResult { command_id, success, message })) => {
                                if !success {
                                    eprintln!("Client: command {command_id} failed: {message}");
                                }
                            }
                            Ok(None) => {
                                println!("Client: server disconnected");
                                break;
                            }
                            Err(err) => {
                                eprintln!("Client: failed to read server message: {err}");
                                break;
                            }
                        }
                    }
                }
            }

            if scene_queue_dropped > 0 || scene_queue_coalesced > 0 {
                println!(
                    "Client: scene queue pressure dropped={} coalesced={}",
                    scene_queue_dropped, scene_queue_coalesced
                );
            }
        });
    })
}

async fn connect_or_start_server() -> Result<UnixStream, String> {
    if let Ok(stream) = connect_to_server().await {
        return Ok(stream);
    }

    start_server_process()?;

    let attempts = 50;
    let delay = Duration::from_millis(100);
    let mut last_error = None;

    for _ in 0..attempts {
        match connect_to_server().await {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                last_error = Some(err);
                sleep(delay).await;
            }
        }
    }

    Err(format!(
        "server was started but did not become ready at {}: {}",
        socket_path().display(),
        last_error
            .map(|err| err.to_string())
            .unwrap_or_else(|| "unknown connection error".into())
    ))
}

fn start_server_process() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("failed to determine current executable: {err}"))?;

    Command::new(current_exe)
        .arg("server")
        .arg("--idle-timeout-secs")
        .arg(DEFAULT_AUTO_STARTED_IDLE_TIMEOUT_SECS.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to start server process: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_state_advances_on_valid_text_edit_patch() {
        let mut state = Some(SceneVersionState {
            buffer_id: 1,
            buffer_version: 2,
        });

        validate_and_advance_scene_version(
            &mut state,
            &ScenePatch::TextEditPatch {
                buffer_id: 1,
                base_version: 2,
                new_version: 3,
                replace_start_char: 1,
                replace_end_char: 2,
                replacement: "Z".into(),
                cursor_char_index: 2,
                cursor_visible: true,
            },
        )
        .expect("patch applies");

        let state = state.expect("state remains");
        assert_eq!(state.buffer_id, 1);
        assert_eq!(state.buffer_version, 3);
    }

    #[test]
    fn version_state_rejects_mismatch() {
        let mut state = Some(SceneVersionState {
            buffer_id: 1,
            buffer_version: 2,
        });

        let result = validate_and_advance_scene_version(
            &mut state,
            &ScenePatch::TextEditPatch {
                buffer_id: 1,
                base_version: 1,
                new_version: 2,
                replace_start_char: 0,
                replace_end_char: 0,
                replacement: "a".into(),
                cursor_char_index: 1,
                cursor_visible: true,
            },
        );

        assert!(result.is_err());
    }
}
