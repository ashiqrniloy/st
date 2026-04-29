use serde::{Deserialize, Serialize};

use crate::{
    documentation::{DocumentationQuery, DocumentationResult},
    events::{EditorCommand, KeyInputEvent, RenderCommand, SceneUpdate},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientId(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientToServer {
    Hello,
    KeyInput(KeyInputEvent),
    Command(EditorCommand),
    CloseClient { client_id: ClientId },
    ShutdownServer,
    DocumentationQuery(DocumentationQuery),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerToClient {
    Welcome { client_id: ClientId },
    Scene(SceneUpdate),
    Render(RenderCommand),
    Error { message: String },
    ServerShuttingDown { reason: String },
    DocumentationResult(DocumentationResult),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key_input() -> KeyInputEvent {
        KeyInputEvent {
            logical_key: "a".into(),
            physical_key: "KeyA".into(),
            text: Some("a".into()),
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
            repeat: false,
        }
    }

    #[test]
    fn client_to_server_round_trips_through_json() {
        let message = ClientToServer::KeyInput(sample_key_input());
        let json = serde_json::to_string(&message).expect("serialize client message");
        let decoded: ClientToServer =
            serde_json::from_str(&json).expect("deserialize client message");

        assert_eq!(decoded, message);
    }

    #[test]
    fn server_to_client_round_trips_through_json() {
        let message = ServerToClient::Welcome {
            client_id: ClientId(42),
        };
        let json = serde_json::to_string(&message).expect("serialize server message");
        let decoded: ServerToClient =
            serde_json::from_str(&json).expect("deserialize server message");

        assert_eq!(decoded, message);
    }

    #[test]
    fn render_message_round_trips_through_json() {
        let message = ServerToClient::Render(RenderCommand::DrawRect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
            color: 0xff00ff,
        });
        let json = serde_json::to_string(&message).expect("serialize render message");
        let decoded: ServerToClient =
            serde_json::from_str(&json).expect("deserialize render message");

        assert_eq!(decoded, message);
    }

    #[test]
    fn scene_message_round_trips_through_json() {
        let message = ServerToClient::Scene(SceneUpdate {
            background_color: 0x1e1e2e,
            text: "abc".into(),
            cursor_char_index: 2,
            cursor_visible: true,
        });
        let json = serde_json::to_string(&message).expect("serialize scene message");
        let decoded: ServerToClient =
            serde_json::from_str(&json).expect("deserialize scene message");

        assert_eq!(decoded, message);
    }

    #[test]
    fn shutdown_message_round_trips_through_json() {
        let message = ServerToClient::ServerShuttingDown {
            reason: "server shutdown requested".into(),
        };
        let json = serde_json::to_string(&message).expect("serialize shutdown message");
        let decoded: ServerToClient =
            serde_json::from_str(&json).expect("deserialize shutdown message");

        assert_eq!(decoded, message);
    }

    #[test]
    fn documentation_query_round_trips_through_json() {
        let message = ClientToServer::DocumentationQuery(DocumentationQuery::DescribeCommand {
            command_id: "editor.insert_text".into(),
        });
        let json = serde_json::to_string(&message).expect("serialize docs query");
        let decoded: ClientToServer = serde_json::from_str(&json).expect("deserialize docs query");
        assert_eq!(decoded, message);
    }
}
