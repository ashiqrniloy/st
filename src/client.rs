use std::{
    process::{Command, Stdio},
    sync::mpsc as std_mpsc,
    thread,
    time::Duration,
};

use tokio::{io::BufReader, net::UnixStream, sync::mpsc as tokio_mpsc, time::sleep};

use crate::{
    events::{EditorEvent, SceneUpdate},
    ipc::{connect_to_server, read_json_line, socket_path, write_json_line},
    protocol::{ClientId, ClientToServer, ServerToClient},
    render::{UiChannels, run_ui_with_gpui},
};

pub fn run() -> Result<(), String> {
    let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<EditorEvent>();
    let (scene_tx, scene_rx) = tokio_mpsc::unbounded_channel::<SceneUpdate>();
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
    scene_tx: tokio_mpsc::UnboundedSender<SceneUpdate>,
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
                            Ok(Some(ServerToClient::Scene(scene))) => {
                                let _ = scene_tx.send(scene);
                            }
                            Ok(Some(ServerToClient::Render(command))) => {
                                println!("Client: received debug render command: {command:?}");
                            }
                            Ok(Some(ServerToClient::Error { message })) => {
                                eprintln!("Client: server error: {message}");
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
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|err| format!("failed to start server process: {err}"))
}
