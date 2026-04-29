# Step-by-Step Implementation Plan

This document is intentionally only an ordered implementation checklist. Overall architecture and design rationale live in `README.md`.

## Phase 1: Support Multiple Clients

Goal: prove the client/server model is real.

- [x] Allow multiple clients to connect simultaneously.
- [x] Broadcast editor updates to all clients initially.
- [x] Track each client's connection state.
- [x] Handle one client disconnecting without affecting others.
- [x] Defer active view/client behavior until later.

Milestone:

```text
Open two clients.
Type in one.
Both receive updates.
Closing one leaves the other and server running.
```

## Phase 2: Add Explicit Server Shutdown

Goal: provide reliable lifecycle control.

- [x] Implement `st quit` fully.
- [x] Send `ShutdownServer` over IPC.
- [x] Server notifies clients before shutdown.
- [x] Clients exit or show disconnected state.
- [x] Server shuts down Deno runtime.
- [x] Server removes socket file on exit.
- [x] Server exits cleanly.

## Phase 3: Add Optional Idle Shutdown

Goal: avoid unwanted background daemons during early development.

- [x] Add configurable idle timeout.
- [x] Track connected client count.
- [x] If no clients remain for N seconds/minutes, shutdown server.
- [x] Disable idle shutdown when launched explicitly as long-running service.

## Phase 4: Load JavaScript From Disk

Goal: stop embedding JS source in Rust.

- [ ] Create `runtime/bootstrap.js`.
- [ ] Create `runtime/editor_api.js`.
- [ ] Load bootstrap from disk in server runtime.
- [ ] Report JS syntax/runtime errors clearly.
- [ ] Keep server alive if JS fails to load.

## Phase 5: JS Commands And Keybindings

Goal: make the editor programmable while keeping Rust in control of command dispatch.

Design rule:

```text
Deno does not receive every key by default.
Deno registers capabilities with Rust.
Rust stores the active command registry and keymap.
Rust decides what each key means.
```

Target extension API shape:

```js
editor.commands.register("insert-date", () => {
  editor.commands.execute("insert-text", new Date().toISOString());
});

editor.keymap.bind("ctrl+d", "insert-date");
```

Target registration flow:

```text
Extension activates in Deno
Deno registers command with Rust server
Deno registers keybinding with Rust server
Rust stores keybinding as owned extension resource
```

Target input dispatch flow:

```text
Client sends KeyInputEvent to server
Server normalizes it into a KeyChord
Server checks Rust-owned keymap
If bound to Rust builtin command: Rust handles it directly
If bound to JS command: Rust invokes Deno command handler
If unbound printable text: Rust applies InsertText directly
If unhandled special key: ignore or pass to explicit listeners later
```

Suggested Rust-side structures:

```rust
pub enum CommandHandler {
    RustBuiltin(EditorCommand),
    JsCommand { extension_id: ExtensionId, command_id: String },
}

pub struct Keymap {
    bindings: HashMap<KeyChord, CommandHandler>,
}
```

Implementation tasks:

- [ ] Define `KeyChord` normalized from `KeyInputEvent`.
- [ ] Implement modified-key chord handling (Ctrl/Alt/Meta and multi-key chords) through Rust keymap dispatch.
- [ ] Define command registry structure in Rust server.
- [ ] Define keymap structure in Rust server.
- [ ] Define `CommandHandler::RustBuiltin`.
- [ ] Define `CommandHandler::JsCommand`.
- [ ] Expose command registration API to JS.
- [ ] Expose keybinding registration API to JS.
- [ ] Track command ownership by extension/runtime.
- [ ] Store JS-registered keybindings in Rust-owned keymap.
- [ ] Resolve incoming key input through Rust keymap before text insertion.
- [ ] Invoke Deno only for keybindings/commands registered by JS.
- [ ] Allow JS command to request a typed `EditorCommand`.
- [ ] Validate and apply requested `EditorCommand` in Rust server.
- [ ] Add tests for keymap resolution.
- [ ] Add tests for Rust builtin vs JS command dispatch.

## Phase 6: Manual Hot Reload

Goal: reload JS without recompiling Rust.

- [ ] Add command to reload JS runtime/extensions.
- [ ] Dispose old JS resources.
- [ ] Reload JS files from disk.
- [ ] Re-register commands/keybindings.
- [ ] Report reload errors without crashing server or clients.

## Phase 7: Extension Lifecycle

Goal: prepare for real extensions.

- [ ] Define extension activation API.
- [ ] Define optional deactivation API.
- [ ] Track extension-owned resources.
- [ ] Dispose resources on reload/unload.
- [ ] Isolate activation errors.

## Phase 8: File I/O

Goal: edit real files.

- [ ] Add open file command.
- [ ] Add save file command.
- [ ] Track buffer path.
- [ ] Track dirty state.
- [ ] Handle file read/write errors.
- [ ] Route file requests through server.

## Phase 9: Undo/Redo

Goal: make editing usable.

- [ ] Define edit transactions.
- [ ] Add undo stack.
- [ ] Add redo stack.
- [ ] Store cursor state before/after edits.
- [ ] Implement undo insert.
- [ ] Implement undo delete/backspace.
- [ ] Add tests.

## Phase 10: Better Buffer Data Structure

Goal: support larger files.

- [ ] Add tests around current `String` buffer behavior.
- [ ] Evaluate `ropey` or another text storage structure.
- [ ] Introduce buffer IDs.
- [ ] Support multiple buffers.
- [ ] Add line/column mapping.

## Phase 11: Text Editing Features In Depth

Goal: revisit editor text behavior comprehensively after core storage, file, and undo/redo primitives are stronger.

- [ ] Define canonical server-side selection model.
- [ ] Define editable vs read-only editor state.
- [ ] Make editor state not editable/read-only.
- [ ] Make editor state editable again.
- [ ] Highlight a single letter.
- [ ] Highlight a word.
- [ ] Highlight a line.
- [ ] Highlight a sentence.
- [ ] Highlight multiple words.
- [ ] Highlight multiple lines.
- [ ] Highlight multiple sentences.
- [ ] Traverse text by letter.
- [ ] Traverse text by word.
- [ ] Traverse text by line.
- [ ] Traverse text by sentence.
- [ ] Go to the beginning of a file.
- [ ] Go to the end of a file.
- [ ] Implement reliable arrow-key movement through the server-owned command path.
- [ ] Implement reliable Shift+arrow selection through the server-owned command path.
- [ ] Add Home/End and platform-specific variants.
- [ ] Add PageUp/PageDown behavior.
- [ ] Add mouse click, drag, double-click, and triple-click selection behavior.
- [ ] Add tests for movement, selection expansion, and editability/read-only behavior.

Milestone:

```text
Core text traversal, selection, and editability behavior is explicit, tested, and server-owned.
```

## Phase 12: Multi-Session Client Architecture

Goal: support multiple independent clients on one server with separate sessions and editor state, while still allowing explicit shared-session attachment later. Implement the proper architecture directly, not a temporary shared-global-state workaround.

Target ownership model:

```text
Server event loop
  owns Client registry
  owns Session registry
  owns Buffer/editor state

Per-client Tokio task
  owns socket read/write halves
  does IPC only
  never owns editor state

ClientId
  one IPC connection / one UI window

SessionId
  one independent editor workspace

BufferId
  one text buffer/file within a session
```

Target Tokio concurrency model:

```text
UnixListener accept loop
  -> accepts sockets
  -> allocates ClientId
  -> creates per-client outbound channel
  -> spawns one Tokio task per client connection
  -> sends ServerEvent::ClientConnected into central server channel

Per-client Tokio task
  -> reads ClientToServer messages from socket
  -> sends ServerEvent::ClientMessage { client_id, message } to central server channel
  -> receives ServerToClient messages from its own outbound channel
  -> writes those messages to the socket
  -> sends ServerEvent::ClientDisconnected when socket closes/errors

Central server event loop
  -> is the only owner/mutator of clients, sessions, buffers, and editor state
  -> receives ServerEvent values from client tasks
  -> mutates state synchronously and quickly
  -> sends outbound messages by enqueueing into per-client channels
  -> never awaits socket I/O while holding or mutating editor state
```

Implementation tasks:

- [ ] Define `SessionId` in the protocol or a dedicated IDs module.
- [ ] Define `BufferId` in the protocol or a dedicated IDs module.
- [ ] Document ID roles: `ClientId` is a connection, `SessionId` is a workspace, `BufferId` is text/file state.
- [ ] Add `ClientConnection { session_id, state, tx }`.
- [ ] Add `ClientConnectionState` with at least `Connected` and `Closing`.
- [ ] Add `EditorSession { active_buffer_id, buffers, ... }`.
- [ ] Add `EditorBuffer { editor_state, path, dirty, ... }` or equivalent.
- [ ] Move canonical `EditorState` out of `EditorServer` root into per-session/per-buffer state.
- [ ] Store clients in `HashMap<ClientId, ClientConnection>`.
- [ ] Store sessions in `HashMap<SessionId, EditorSession>`.
- [ ] Add server counters/allocators for `ClientId`, `SessionId`, and `BufferId`.
- [ ] On client connect without an attach target, create a new `SessionId` and initial scratch `BufferId`.
- [ ] Associate that client with its new independent session.
- [ ] Send `Welcome` with enough metadata for the client to know its `ClientId` and active session once protocol supports it.
- [ ] Add explicit attach semantics in the protocol, for example `ClientToServer::AttachSession { session_id }`.
- [ ] Validate attach requests and return a protocol error if the session does not exist or is not attachable.
- [ ] Allow multiple clients to point at the same `SessionId` only through explicit attach behavior.
- [ ] Route `KeyInput` by looking up `client_id -> session_id -> active_buffer_id -> EditorState`.
- [ ] Route `Command` by looking up `client_id -> session_id -> active_buffer_id -> EditorState`.
- [ ] Generate `SceneUpdate` from the changed session/buffer only.
- [ ] Send scene updates only to clients whose `ClientConnection.session_id` matches the changed session.
- [ ] Remove global editor-state broadcasting.
- [ ] Keep independent per-client outbound `mpsc` channels.
- [ ] Make outbound sends from the central server loop non-blocking channel sends only.
- [ ] If a client's outbound channel is closed, mark/remove that client without affecting other clients.
- [ ] Do not wrap the entire server/editor state in a shared mutex used by client tasks.
- [ ] Do not let client tasks mutate sessions or buffers directly.
- [ ] Do not hold server/editor state across `.await` points for socket reads or writes.
- [ ] Decide and implement session lifecycle policy for disconnected clients.
- [ ] Preserve unattached sessions only if they are explicitly retained, named, dirty, or attached by future behavior.
- [ ] Destroy empty scratch sessions when their last client disconnects if they are not retained.
- [ ] Ensure closing one client removes only that `ClientId` and possibly its unretained empty session.
- [ ] Ensure closing one client never shuts down the server.
- [ ] Add unit tests for ID allocation and client/session association.
- [ ] Add unit tests for independent client sessions.
- [ ] Add unit tests for explicit shared-session attachment.
- [ ] Add unit tests that updates are session-scoped, not global.
- [ ] Add unit tests that a disconnected client is removed without affecting unrelated clients/sessions.
- [ ] Add an integration/manual test recipe: open two clients, type different text, verify they diverge.
- [ ] Add an integration/manual test recipe: explicitly attach a second client to an existing session, type once, verify both attached clients update.

Milestone:

```text
Open two clients.
Each gets an independent session by default.
Typing in one does not affect the other.
Explicitly attached clients share only that session's updates.
One slow or closed client does not block another client.
The central server event loop remains the only owner of canonical editor state.
Per-client Tokio tasks perform IPC only.
```

## Phase 13: Systemd Integration

Goal: allow persistent background server operation.

- [ ] Add documented foreground server mode.
- [ ] Add documented user service example.
- [ ] Decide whether `st server --daemon` is needed or systemd is enough.
- [ ] Ensure socket path and cleanup work under systemd.

## Phase 14: Adopt GPUI Component For Non-Editor UI

Goal: improve application chrome and supporting UI using `gpui-component` without replacing the custom server-backed editor surface.

Explicit boundary: `gpui-component` is not used for the editor itself. The editor remains our custom GPUI native editor view, backed by server-owned editor state.

Use `gpui-component` only for:

- [ ] Command palette.
- [ ] Buttons.
- [ ] Dialogs.
- [ ] Settings.
- [ ] Panels.
- [ ] Tabs.
- [ ] Status bar.
- [ ] Menus.
- [ ] Notifications.
- [ ] Dock layout.

Implementation tasks:

- [ ] Add `gpui-component` dependency only after editor core/client-server rendering is stable.
- [ ] Call `gpui_component::init(cx)` during GPUI app startup.
- [ ] Wrap the application root as required for overlays/dialogs/notifications.
- [ ] Keep editor buffer, cursor, selections, undo/redo, and key dispatch outside `gpui-component::InputState`.
- [ ] Document which UI surfaces are allowed to use `gpui-component`.

## Phase 15: Extension And Agent Runtime Architecture

Goal: use one JS runtime for normal editor programmability and isolated per-agent JS runtimes for long-running or blocking agent work. Implement the final runtime separation model directly so blocked agent code cannot freeze normal editor behavior.

Target runtime model:

```text
Server event loop
  owns canonical clients/sessions/buffers/editor state
  owns command registry and keymap
  validates all mutations

Editor extension runtime
  one deno_core::JsRuntime
  lightweight editor programmability only
  commands, keybindings, themes, modes, lightweight hooks
  must not run long-running agents

Agent runtime workers
  one worker per active agent
  one deno_core::JsRuntime per worker
  one V8 isolate/event loop per agent
  communicates with server through channels
  never directly mutates editor state
```

Target message flow for normal editor JS:

```text
Client input
  -> Server keymap/command dispatch
  -> Rust builtin command OR editor JS command
  -> JS command returns a typed request/result
  -> Server validates
  -> Server mutates editor/session/buffer state
  -> Server emits SceneUpdate
```

Target message flow for agents:

```text
User starts agent
  -> Server allocates AgentId
  -> Server creates AgentRuntimeWorker
  -> Worker starts its own deno_core::JsRuntime
  -> Server sends immutable context snapshots / typed tool requests
  -> Agent returns proposed edits/tool requests/logs/errors
  -> Server validates every requested mutation
  -> Server applies accepted commands
  -> Server emits SceneUpdate
```

Tokio/threading requirements:

```text
Central server event loop
  - must not execute JavaScript inline
  - must not block on agent completion
  - must communicate with runtimes through channels

Editor runtime worker
  - owns the single normal editor JsRuntime
  - processes lightweight extension requests sequentially
  - returns results to the server through channels

Agent runtime worker
  - owns exactly one agent JsRuntime
  - runs independently from the editor runtime and other agents
  - may be a Tokio task if compatible with JsRuntime ownership requirements
  - may be a dedicated OS thread if runtime !Send/!Sync constraints require it
  - communicates by mpsc/oneshot channels only
```

Implementation tasks:

- [ ] Define `AgentId`.
- [ ] Define `RuntimeRequestId` for correlating async runtime requests/responses.
- [ ] Define `EditorRuntimeRequest` and `EditorRuntimeResponse`.
- [ ] Define `AgentRuntimeRequest` and `AgentRuntimeResponse`.
- [ ] Define typed agent outputs: proposed edit, command request, tool request, log, error, completion.
- [ ] Define immutable context snapshot types for sessions/buffers/selections.
- [ ] Keep one normal editor `deno_core::JsRuntime` for lightweight extension operations.
- [ ] Restrict the normal editor runtime to commands, keybindings, themes, modes, and lightweight hooks.
- [ ] Explicitly reject or route long-running agent work away from the normal editor runtime.
- [ ] Add an `EditorRuntimeWorker` abstraction that owns the normal editor runtime.
- [ ] Start the editor runtime worker outside the central server event loop.
- [ ] Communicate with the editor runtime worker through request/response channels.
- [ ] Ensure the central server event loop does not call `JsRuntime::run_event_loop` directly.
- [ ] Ensure the central server event loop never waits synchronously for JS to finish.
- [ ] Add an `AgentRuntimeWorker` abstraction.
- [ ] Start one `AgentRuntimeWorker` per active agent.
- [ ] Start one `deno_core::JsRuntime` inside each agent worker.
- [ ] Ensure each agent runtime has its own V8 isolate and event loop.
- [ ] Store active agents in `HashMap<AgentId, AgentHandle>` on the server.
- [ ] `AgentHandle` should contain request sender, lifecycle state, and cancellation handle.
- [ ] Send only immutable snapshots or typed requests from server state into agent workers.
- [ ] Require agents to return typed proposed changes instead of mutating state directly.
- [ ] Validate all agent-requested edits/commands in Rust before applying them.
- [ ] Route accepted agent edits through the same transaction/undo command path as user edits.
- [ ] Never hold server/editor/session/buffer state while executing JavaScript.
- [ ] Never expose direct mutable Rust editor state references to JS.
- [ ] Ensure blocked normal editor JS cannot block socket IPC tasks.
- [ ] Ensure blocked normal editor JS cannot corrupt server state.
- [ ] Ensure blocked agent JS cannot block the normal editor runtime.
- [ ] Ensure blocked agent JS cannot block unrelated agent runtimes.
- [ ] Ensure blocked agent JS cannot block client IPC tasks.
- [ ] Add cancellation messages for running agents.
- [ ] Add timeout handling for agent requests and tool calls.
- [ ] Add runtime cleanup when an agent completes normally.
- [ ] Add runtime cleanup when an agent errors.
- [ ] Add runtime cleanup when an agent is cancelled.
- [ ] Add runtime cleanup when the server shuts down.
- [ ] Add structured logging for runtime start, request, response, error, cancellation, and shutdown.
- [ ] Add resource accounting hooks for CPU time, memory, tool usage, and token usage where practical.
- [ ] Add a policy hook for maximum concurrent agents.
- [ ] Add a policy hook for maximum runtime duration per agent.
- [ ] Add a policy hook for maximum pending requests per agent.
- [ ] Define the future migration path from per-agent runtime workers to per-agent OS processes for stronger isolation.
- [ ] Add tests for normal editor runtime request/response dispatch.
- [ ] Add tests for independent agent runtime request/response dispatch.
- [ ] Add tests that a blocked agent does not block normal editor JS commands.
- [ ] Add tests that a blocked agent does not block another agent.
- [ ] Add tests that cancelling one agent does not cancel unrelated agents.
- [ ] Add tests that agent-proposed invalid edits are rejected by the server.
- [ ] Add tests that accepted agent edits produce normal transactions and scene updates.

Milestone:

```text
Normal editor extensions share one lightweight JS runtime.
Each active agent has an isolated JS runtime with its own V8 isolate/event loop.
The server event loop never executes JavaScript inline.
Blocked or slow agent code does not block editor commands, clients, or other agents.
All mutations still go through validated Rust server commands and normal edit transactions.
```

## Phase 16: Permissions And AI Layer

Goal: add advanced features safely after the editor core works.

- [ ] Define permission model for extensions.
- [ ] Gate file/network/subprocess access.
- [ ] Add AI command/tool API.
- [ ] Route AI edits through normal command/transaction system.
- [ ] Make AI edits previewable/reversible.
