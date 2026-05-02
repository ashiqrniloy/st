use crate::{
    documentation::DocumentationQuery,
    events::{KeyInputEvent, RenderCommand, SceneUpdate, Viewport},
    protocol::{ClientId, ClientToServer, ServerToClient},
};

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
    let decoded: ClientToServer = serde_json::from_str(&json).expect("deserialize client message");

    assert_eq!(decoded, message);
}

#[test]
fn server_to_client_round_trips_through_json() {
    let message = ServerToClient::Welcome {
        client_id: ClientId(42),
    };
    let json = serde_json::to_string(&message).expect("serialize server message");
    let decoded: ServerToClient = serde_json::from_str(&json).expect("deserialize server message");

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
    let decoded: ServerToClient = serde_json::from_str(&json).expect("deserialize render message");

    assert_eq!(decoded, message);
}

#[test]
fn scene_message_round_trips_through_json() {
    let message = ServerToClient::SceneSnapshot(SceneUpdate {
        background_color: 0x1e1e2e,
        text: "abc".into(),
        cursor_char_index: 2,
        cursor_visible: true,
        panes: vec![],
    });
    let json = serde_json::to_string(&message).expect("serialize scene message");
    let decoded: ServerToClient = serde_json::from_str(&json).expect("deserialize scene message");

    assert_eq!(decoded, message);
}

#[test]
fn shutdown_message_round_trips_through_json() {
    let message = ServerToClient::ServerShuttingDown {
        reason: "server shutdown requested".into(),
    };
    let json = serde_json::to_string(&message).expect("serialize shutdown message");
    let decoded: ServerToClient = serde_json::from_str(&json).expect("deserialize shutdown message");

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

#[test]
fn viewport_message_round_trips_through_json() {
    let message = ClientToServer::SetViewport {
        viewport: Viewport {
            start_line: 3,
            end_line: 9,
        },
    };
    let json = serde_json::to_string(&message).expect("serialize viewport message");
    let decoded: ClientToServer = serde_json::from_str(&json).expect("deserialize viewport message");
    assert_eq!(decoded, message);
}

#[test]
fn command_result_round_trips_through_json() {
    let message = ServerToClient::CommandResult {
        command_id: "window.split_dwim".into(),
        success: true,
        message: "split accepted".into(),
    };
    let json = serde_json::to_string(&message).expect("serialize command result");
    let decoded: ServerToClient = serde_json::from_str(&json).expect("deserialize command result");
    assert_eq!(decoded, message);
}

#[test]
fn window_dimensions_message_round_trips_through_json() {
    let message = ClientToServer::SetWindowDimensions {
        width: 900,
        height: 500,
    };
    let json = serde_json::to_string(&message).expect("serialize dimensions");
    let decoded: ClientToServer = serde_json::from_str(&json).expect("deserialize dimensions");
    assert_eq!(decoded, message);
}

#[test]
fn js_registration_messages_round_trip_through_json() {
    let command = ClientToServer::RegisterJsCommand {
        extension_id: "ext.test".into(),
        command_id: "ext.hello".into(),
        title: "Hello".into(),
        description: "Run hello".into(),
        category: "extension".into(),
    };
    let chord = ClientToServer::RegisterJsKeybinding {
        extension_id: "ext.test".into(),
        chord: "ctrl+d".into(),
        command_id: "ext.hello".into(),
    };

    let command_json = serde_json::to_string(&command).expect("serialize command registration");
    let chord_json = serde_json::to_string(&chord).expect("serialize keybinding registration");

    let command_decoded: ClientToServer =
        serde_json::from_str(&command_json).expect("deserialize command registration");
    let chord_decoded: ClientToServer =
        serde_json::from_str(&chord_json).expect("deserialize keybinding registration");

    assert_eq!(command_decoded, command);
    assert_eq!(chord_decoded, chord);
}
