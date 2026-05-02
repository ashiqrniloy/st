use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tokio::{io::BufReader, net::UnixStream, sync::mpsc};

use crate::{
    ipc::{read_json_line, write_json_line},
    protocol::{ClientId, ClientToServer, ServerToClient},
};

use super::r#loop::ServerEvent;

pub(super) async fn handle_client(
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
