## Phase 4: Emacs-Like Command Structure Foundation

Goal: create the command foundation now so all future user-visible behavior is added through a discoverable command system.

- [x] Define `CommandId`.
- [x] Define `CommandDescriptor` with structured metadata.
- [x] Include required command metadata: id, title, description, source, category/namespace, arguments, examples, related docs.
- [x] Define `CommandSource` for builtin, extension, tool, and generated commands.
- [x] Define `CommandHandler` for Rust builtin handlers first.
- [x] Add a server-owned `CommandRegistry`.
- [x] Register existing builtin editor commands through `CommandRegistry`.
- [x] Route client command requests through `CommandRegistry` instead of ad-hoc command handling where practical.
- [x] Keep Rust validation and editor mutation in the server.
- [x] Keep ordinary text input on the Rust hot path.
- [x] Prepare the registry shape so later JS/extension commands can register descriptors and handlers.
- [x] Add tests for command registration.
- [x] Add tests for duplicate command ID rejection.
- [x] Add tests for builtin command dispatch.
- [x] Add tests that public commands require metadata.
- [x] Link this phase to `documenation.md` as the detailed documentation rationale.

Milestone:

```text
User-visible operations are represented as registered commands with metadata, and future features have a single command system to plug into.
```

