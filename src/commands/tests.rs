use super::*;
use crate::editor::EditorState;

fn valid_descriptor(id: &str) -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(id).expect("valid id"),
        title: "Title".into(),
        description: "Description".into(),
        source: CommandSource::Builtin,
        category: "editor".into(),
        arguments: vec![],
        examples: vec!["Example".into()],
        related_docs: vec!["documenation.md".into()],
    }
}

#[test]
fn rejects_duplicate_command_id_registration() {
    let mut registry = CommandRegistry::new();
    registry
        .register(
            valid_descriptor("test.command"),
            CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
        )
        .expect("first registration should pass");

    let err = registry
        .register(
            valid_descriptor("test.command"),
            CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
        )
        .expect_err("duplicate command id should fail");

    assert!(err.contains("already registered"));
}

#[test]
fn rejects_missing_required_metadata() {
    let mut registry = CommandRegistry::new();
    let mut descriptor = valid_descriptor("test.command");
    descriptor.description.clear();

    let err = registry
        .register(
            descriptor,
            CommandHandler::RustBuiltin(RustBuiltinCommand::Backspace),
        )
        .expect_err("missing metadata should fail");

    assert!(err.contains("missing description"));
}

#[test]
fn dispatches_builtin_command() {
    let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
    let mut editor = EditorState::default();

    registry
        .execute(
            CommandInvocation::with_text(CommandId::new("editor.insert_text").expect("id"), "abc"),
            &mut editor,
        )
        .expect("dispatch should succeed");

    assert_eq!(editor.buffer.full_text(), "abc");
    assert_eq!(editor.cursor, 3);
}

#[test]
fn builtin_help_commands_are_discoverable() {
    let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
    let summaries = registry.list_command_summaries();
    assert!(summaries.iter().any(|s| s.id == "help.commands"));
    assert!(summaries.iter().any(|s| s.id == "help.command"));
}

#[test]
fn all_builtin_commands_have_required_metadata() {
    let registry = CommandRegistry::with_builtin_commands().expect("builtin registry");
    for summary in registry.list_command_summaries() {
        let details = registry
            .describe_command(&summary.id)
            .expect("summary id should be describable");
        assert!(!details.id.trim().is_empty());
        assert!(!details.title.trim().is_empty());
        assert!(!details.description.trim().is_empty());
        assert!(!details.examples.is_empty());
    }
}
