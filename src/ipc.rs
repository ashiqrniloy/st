use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};

const APP_RUNTIME_DIR: &str = "st";
const SOCKET_FILE_NAME: &str = "st.sock";

pub fn runtime_dir() -> PathBuf {
    env::var_os(crate::configuration::RUNTIME_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(env::temp_dir)
                .join(APP_RUNTIME_DIR)
        })
}

pub fn socket_path() -> PathBuf {
    runtime_dir().join(SOCKET_FILE_NAME)
}

pub fn ensure_runtime_dir() -> io::Result<PathBuf> {
    let dir = runtime_dir();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub async fn connect_to_server() -> io::Result<UnixStream> {
    UnixStream::connect(socket_path()).await
}

pub async fn bind_server_socket() -> io::Result<UnixListener> {
    ensure_runtime_dir()?;
    let path = socket_path();
    remove_stale_socket_file(&path).await?;
    UnixListener::bind(path)
}

async fn remove_stale_socket_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    match UnixStream::connect(path).await {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            format!("server socket is already active: {}", path.display()),
        )),
        Err(_) => fs::remove_file(path),
    }
}

pub async fn write_json_line<W, T>(writer: &mut W, message: &T) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let encoded = serde_json::to_vec(message).map_err(invalid_data)?;
    writer.write_all(&encoded).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}

pub async fn read_json_line<R, T>(reader: &mut R) -> io::Result<Option<T>>
where
    R: AsyncBufRead + Unpin,
    T: DeserializeOwned,
{
    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line).await?;

    if bytes_read == 0 {
        return Ok(None);
    }

    let message = serde_json::from_str(line.trim_end()).map_err(invalid_data)?;
    Ok(Some(message))
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use tokio::io::{BufReader, duplex};

    use super::*;
    use crate::{
        events::{KeyInputEvent, RenderCommand},
        protocol::{ClientId, ClientToServer, ServerToClient},
    };

    #[tokio::test]
    async fn writes_and_reads_client_message() {
        let (mut writer, reader) = duplex(1024);
        let mut reader = BufReader::new(reader);

        let message = ClientToServer::KeyInput(KeyInputEvent {
            logical_key: "a".into(),
            physical_key: "KeyA".into(),
            text: Some("a".into()),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            repeat: false,
        });

        write_json_line(&mut writer, &message)
            .await
            .expect("write client message");
        let decoded: ClientToServer = read_json_line(&mut reader)
            .await
            .expect("read client message")
            .expect("message should be present");

        assert_eq!(decoded, message);
    }

    #[tokio::test]
    async fn writes_and_reads_server_message() {
        let (mut writer, reader) = duplex(1024);
        let mut reader = BufReader::new(reader);

        let message = ServerToClient::Render(RenderCommand::DrawRect {
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 40.0,
            color: 0x00ff00,
        });

        write_json_line(&mut writer, &message)
            .await
            .expect("write server message");
        let decoded: ServerToClient = read_json_line(&mut reader)
            .await
            .expect("read server message")
            .expect("message should be present");

        assert_eq!(decoded, message);
    }

    #[tokio::test]
    async fn returns_none_on_eof() {
        let (writer, reader) = duplex(1024);
        drop(writer);
        let mut reader = BufReader::new(reader);

        let decoded: Option<ServerToClient> = read_json_line(&mut reader).await.expect("read EOF");

        assert_eq!(decoded, None);
    }

    #[test]
    fn socket_path_ends_with_expected_components() {
        let path = socket_path();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("st.sock")
        );
        assert_eq!(
            path.parent()
                .and_then(|dir| dir.file_name())
                .and_then(|name| name.to_str()),
            Some("st")
        );
    }

    #[test]
    fn protocol_types_are_usable_with_ipc_helpers() {
        let _ = ClientToServer::CloseClient {
            client_id: ClientId(1),
        };
        let _ = ServerToClient::Welcome {
            client_id: ClientId(1),
        };
    }
}
