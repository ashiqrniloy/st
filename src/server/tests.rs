use tokio::sync::mpsc;

use crate::{
    commands::{CommandId, CommandInvocation},
    documentation::{DocumentationQuery, DocumentationResult},
    events::{EditorCommand, EditorEvent, KeyInputEvent, ScenePatch, Viewport},
    protocol::{ClientId, ClientToServer, ServerToClient},
};

use super::{
    key_chord::{normalize_key_name, parse_key_chord},
    key_input::{key_input_to_command_invocation, should_forward_to_js_runtime},
    state::{EditorSceneSettings, EditorServer},
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
    let (client_1_tx, mut client_1_rx) = mpsc::channel(256);
    let (client_2_tx, mut client_2_rx) = mpsc::channel(256);
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
        Ok(ServerToClient::ScenePatch(ScenePatch::TextEditPatch {
            buffer_id: 1,
            base_version: 0,
            new_version: 1,
            replace_start_char: 0,
            replace_end_char: 0,
            replacement: "x".into(),
            cursor_char_index: 1,
            cursor_visible: true,
        }))
    );
    assert_eq!(
        client_1_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    );
    assert_eq!(
        client_2_rx.try_recv(),
        Ok(ServerToClient::ScenePatch(ScenePatch::TextEditPatch {
            buffer_id: 1,
            base_version: 0,
            new_version: 1,
            replace_start_char: 0,
            replace_end_char: 0,
            replacement: "x".into(),
            cursor_char_index: 1,
            cursor_visible: true,
        }))
    );
    assert_eq!(
        client_2_rx.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    );
}

#[test]
fn close_client_only_closes_the_sending_connection() {
    let mut server = EditorServer::default();
    let (client_1_tx, _client_1_rx) = mpsc::channel(256);
    let (client_2_tx, _client_2_rx) = mpsc::channel(256);
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
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
    let chord = parse_key_chord("ctrl x ctrl s").expect("parse chord");
    assert_eq!(chord.0.len(), 2);
    assert!(chord.0[0].ctrl);
    assert_eq!(chord.0[0].key, "x");
    assert!(chord.0[1].ctrl);
    assert_eq!(chord.0[1].key, "s");
}

#[test]
fn parses_modifier_group_followed_by_one_or_more_keys() {
    let chord = parse_key_chord("ctrl+shift w h").expect("parse emacs-style chord");
    assert_eq!(chord.0.len(), 2);
    assert_eq!(chord.0[0].key, "w");
    assert!(chord.0[0].ctrl);
    assert!(chord.0[0].shift);
    assert_eq!(chord.0[1].key, "h");
    assert!(chord.0[1].ctrl);
    assert!(chord.0[1].shift);
}

#[test]
fn rejects_modifier_only_chords_without_a_key() {
    let err = parse_key_chord("ctrl+shift").expect_err("key is required");
    assert!(err.contains("at least one non-modifier key"));
}

#[test]
fn normalizes_named_keys_for_keybindings() {
    let chord = parse_key_chord("ctrl space enter escape").expect("parse named keys");
    assert_eq!(
        chord
            .0
            .iter()
            .map(|simple| simple.key.as_str())
            .collect::<Vec<_>>(),
        vec!["space", "enter", "escape"]
    );
    assert!(chord.0.iter().all(|simple| simple.ctrl));
    assert_eq!(normalize_key_name("Esc"), "escape");
    assert_eq!(normalize_key_name("Return"), "enter");
}

#[test]
fn forwarding_policy_does_not_send_unregistered_keys_to_js() {
    let mut modified = key_event("a", Some("a"));
    modified.ctrl = true;
    assert!(!should_forward_to_js_runtime(&modified, false));

    let special = key_event("Escape", None);
    assert!(!should_forward_to_js_runtime(&special, false));
}

#[test]
fn documentation_query_lists_commands() {
    let mut server = EditorServer::default();
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
fn documentation_query_lists_keybindings_from_live_keymap() {
    let mut server = EditorServer::default();
    let (client_tx, mut client_rx) = mpsc::channel(256);
    let (js_tx, _js_rx) = mpsc::unbounded_channel();
    let client_id = ClientId(1);
    server.register_client(client_id, client_tx);

    assert!(!server.handle_message(
        client_id,
        ClientToServer::DocumentationQuery(DocumentationQuery::ListKeybindings),
        &js_tx,
    ));

    let ServerToClient::DocumentationResult(DocumentationResult::KeybindingsList { keybindings }) =
        client_rx.try_recv().expect("docs result")
    else {
        panic!("expected keybindings list");
    };

    assert!(keybindings.iter().any(|binding| {
        binding.chord == "backspace" && binding.command_id == "editor.backspace"
    }));
}

#[test]
fn documentation_query_lists_builtin_api_docs() {
    let mut server = EditorServer::default();
    let (client_tx, mut client_rx) = mpsc::channel(256);
    let (js_tx, _js_rx) = mpsc::unbounded_channel();
    let client_id = ClientId(1);
    server.register_client(client_id, client_tx);

    assert!(!server.handle_message(
        client_id,
        ClientToServer::DocumentationQuery(DocumentationQuery::ListApis),
        &js_tx,
    ));

    let ServerToClient::DocumentationResult(DocumentationResult::ApiList { apis }) =
        client_rx.try_recv().expect("docs result")
    else {
        panic!("expected api list");
    };

    assert!(apis.iter().any(|api| api.id == "config.loading"));
    assert!(apis.iter().any(|api| api.id == "api.keymap.bind"));
}

#[test]
fn scene_update_uses_configurable_scene_settings() {
    let mut server = EditorServer::default();
    server.scene_settings = EditorSceneSettings {
        background_color: 0x112233,
        cursor_visible: false,
    };

    let scene = server.scene_snapshot_for_viewport(Viewport::default(), vec![]);
    assert_eq!(scene.background_color, 0x112233);
    assert!(!scene.cursor_visible);
}

#[test]
fn rust_builtin_key_dispatch_does_not_invoke_js_runtime() {
    let mut server = EditorServer::default();
    let (client_tx, _client_rx) = mpsc::channel(256);
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
    let (client_tx, _client_rx) = mpsc::channel(256);
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
    let (client_tx, mut client_rx) = mpsc::channel(256);
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
fn js_keybinding_registration_rejects_unknown_command() {
    let mut server = EditorServer::default();
    let (client_tx, mut client_rx) = mpsc::channel(256);
    let (js_tx, _js_rx) = mpsc::unbounded_channel();
    let client_id = ClientId(1);
    server.register_client(client_id, client_tx);

    assert!(!server.handle_message(
        client_id,
        ClientToServer::RegisterJsKeybinding {
            extension_id: "ext.test".into(),
            chord: "ctrl+d".into(),
            command_id: "ext.missing".into(),
        },
        &js_tx,
    ));

    assert!(matches!(
        client_rx.try_recv(),
        Ok(ServerToClient::Error { message }) if message.contains("unknown command")
    ));
}

#[test]
fn per_client_viewports_receive_independent_visible_text_updates() {
    let mut server = EditorServer::default();
    let (client_1_tx, mut client_1_rx) = mpsc::channel(256);
    let (client_2_tx, mut client_2_rx) = mpsc::channel(256);
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
        ServerToClient::SceneSnapshot(scene) => {
            assert!(scene.text.contains("line1"));
        }
        ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate { text, .. }) => {
            assert!(text.contains("line1"));
        }
        other => panic!("expected snapshot or visible text update for client1, got {other:?}"),
    }

    match c2 {
        ServerToClient::SceneSnapshot(scene) => {
            assert!(scene.text.contains("line2"));
        }
        ServerToClient::ScenePatch(ScenePatch::VisibleTextUpdate { text, .. }) => {
            assert!(text.contains("line2"));
        }
        other => panic!("expected snapshot or visible text update for client2, got {other:?}"),
    }
}
