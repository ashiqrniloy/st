# Step-by-Step Implementation Plan

This document is intentionally only an ordered implementation checklist. Overall architecture and design rationale live in `README.md`.

## Phase 1: Stabilize Current Modular Prototype

Goal: keep the current single-process prototype working while preparing for client/server extraction.

- [x] Refactor `src/main.rs` into modules.
- [x] Add `src/app.rs`.
- [x] Add `src/events.rs`.
- [x] Add `src/editor.rs`.
- [x] Add `src/render.rs`.
- [x] Add `src/js_runtime.rs`.
- [x] Remove old duplicate rendering module.
- [x] Define temporary `RenderCommand`.
- [x] Define `KeyInputEvent`.
- [x] Define `EditorEvent`.
- [x] Define `EditorCommand`.
- [x] Define minimal `EditorState`.
- [x] Implement insert text.
- [x] Implement backspace.
- [x] Implement cursor left/right.
- [x] Implement cursor clamping.
- [x] Add basic editor unit tests.
- [ ] Decide short-term close behavior for single-process mode.
- [ ] Remove or suppress expected dead-code warnings once server/client wiring uses the types.

## Phase 2: Introduce Protocol Types

Goal: separate in-process editor events from client/server protocol messages.

- [x] Add `src/protocol.rs`.
- [x] Define `ClientId`.
- [x] Define `ClientToServer`.
- [x] Define `ServerToClient`.
- [x] Move IPC-facing message types into `protocol.rs`.
- [x] Keep editor-internal types in `events.rs` for now.
- [x] Add serde derives for protocol messages.
- [x] Add tests for serializing/deserializing protocol messages.

Initial protocol target:

```rust
pub enum ClientToServer {
    Hello,
    KeyInput(KeyInputEvent),
    Command(EditorCommand),
    CloseClient { client_id: ClientId },
    ShutdownServer,
}

pub enum ServerToClient {
    Welcome { client_id: ClientId },
    Render(RenderCommand),
    Error { message: String },
}
```

## Phase 3: Add Local IPC Layer

Goal: support process-to-process communication over a local socket.

- [x] Add `src/ipc.rs`.
- [x] Use a Unix domain socket as the first transport.
- [x] Choose socket path, preferably `$XDG_RUNTIME_DIR/st/st.sock`.
- [x] Add helper to compute socket path.
- [x] Add helper to create parent runtime directory.
- [x] Implement newline-delimited JSON message writing.
- [x] Implement newline-delimited JSON message reading.
- [x] Add client connect helper.
- [x] Add server bind/listen helper.
- [x] Handle stale socket files on server startup.
- [x] Add tests for message encoding/decoding where practical.

## Phase 4: Add CLI Routing

Goal: make the binary able to run as server, client, or utility command.

- [x] Add `src/cli.rs`.
- [x] Parse basic subcommands without adding a CLI dependency yet.
- [x] Support `st server`.
- [x] Support `st client`.
- [x] Support `st quit`.
- [x] Make bare `st` behave like `st client`.
- [x] Keep `main.rs` as top-level routing and error handling only.

Target behavior:

```bash
cargo run -- server
cargo run -- client
cargo run -- quit
cargo run
```

## Phase 5: Implement Minimal Server

Goal: run a foreground server that accepts clients and logs messages.

- [x] Add `src/server.rs`.
- [x] Define `EditorServer`.
- [x] Store `EditorState` in `EditorServer`.
- [x] Accept multiple Unix socket clients.
- [x] Assign each client a `ClientId`.
- [x] Reply to `Hello` with `Welcome`.
- [x] Log `KeyInput` messages on the server.
- [x] Handle `CloseClient` by removing the client connection.
- [x] Handle `ShutdownServer` by exiting the server loop.
- [x] Ensure closing one client does not stop the server.

Milestone:

```text
Terminal 1: cargo run -- server
Terminal 2: cargo run -- client

Client connects.
Server assigns client id.
Server logs received messages.
Closing client leaves server running.
cargo run -- quit stops server.
```

## Phase 6: Implement Minimal Client

Goal: move the GPUI frontend into a client process connected to the server.

- [x] Add `src/client.rs`.
- [x] Move current UI startup from `app.rs` into client flow.
- [x] Connect to running server over IPC.
- [x] Send `Hello` on connect.
- [x] Store assigned `ClientId` from `Welcome`.
- [x] Send `KeyInput` messages to server instead of directly to Deno.
- [x] Send `CloseClient` when the window closes.
- [x] Exit the client process when the window closes.
- [x] Keep server running after client exit.

## Phase 7: Auto-Start Server From Client

Goal: make `st` convenient for normal use.

- [x] On client startup, try to connect to the server socket.
- [x] If connect fails, spawn `st server` in the background.
- [x] Wait for socket availability with timeout.
- [x] Connect once the server is ready.
- [x] Report clear error if server cannot be started.
- [x] Avoid spawning duplicate servers when one already exists.

Milestone:

```text
cargo run

If no server exists:
  server starts automatically
  client connects
  window opens
```

## Phase 8: Move Deno Runtime Behind Server

Goal: ensure the extension runtime belongs to the server, not the UI client.

- [x] Update `js_runtime.rs` to receive server-side editor events.
- [x] Start Deno runtime from `server.rs`.
- [x] Remove direct UI-to-Deno channel wiring from client code.
- [x] Route client key input through server first.
- [x] Forward relevant events from server to JS runtime.
- [x] Keep JS runtime alive as long as the server is alive.
- [x] Add graceful JS runtime shutdown when server shuts down.

Target flow:

```text
Client -> IPC -> Server -> JS Runtime
```

## Phase 9: Wire Commands Into EditorState

Goal: make server-owned editor state mutate through commands.

- [ ] Convert text key input into `EditorCommand::InsertText`.
- [ ] Convert Backspace into `EditorCommand::Backspace`.
- [ ] Convert left arrow into `EditorCommand::MoveCursorLeft`.
- [ ] Convert right arrow into `EditorCommand::MoveCursorRight`.
- [ ] Apply commands to server-owned `EditorState`.
- [ ] Add tests for key-input-to-command translation.
- [ ] Log updated buffer/cursor state after each command initially.

Milestone:

```text
Client window receives keys.
Server receives keys.
Server mutates EditorState.
Server logs buffer and cursor.
```

## Phase 10: Send Render Updates From Server To Client

Goal: have server state drive client rendering.

- [ ] Add server-to-client render/update channel per connected client.
- [ ] After `EditorState` changes, produce temporary render output.
- [ ] Send `ServerToClient::Render` to the active client.
- [ ] Client receives render messages from IPC.
- [ ] Client applies render messages to GPUI view.
- [ ] Keep temporary rectangle rendering for now.
- [ ] Add cursor placeholder rendering.

Milestone:

```text
Typing in client mutates server state.
Server sends render update.
Client redraws.
```

## Phase 11: Replace Temporary RenderCommand With Scene Updates

Goal: stop treating JS/client rendering as raw drawing long-term.

- [ ] Define `Scene` or `SceneUpdate` type.
- [ ] Represent background.
- [ ] Represent text placeholders or glyph runs.
- [ ] Represent cursor.
- [ ] Represent selections later.
- [ ] Replace most `RenderCommand` usage with `SceneUpdate`.
- [ ] Keep `RenderCommand` only if needed for temporary debugging.

## Phase 12: Render Visible Text

Goal: make the editor visibly editable.

- [ ] Choose short-term text rendering approach.
- [ ] Render buffer text in the client.
- [ ] Render cursor position.
- [ ] Handle newlines.
- [ ] Keep layout simple and fixed-width initially.
- [ ] Add placeholder scrolling only if necessary.

Milestone:

```text
Open client.
Type characters.
Characters appear in the window.
Backspace works.
Cursor moves left/right.
Server owns state.
```

## Phase 13: Support Multiple Clients

Goal: prove the client/server model is real.

- [ ] Allow multiple clients to connect simultaneously.
- [ ] Broadcast editor updates to all clients initially.
- [ ] Track each client's connection state.
- [ ] Handle one client disconnecting without affecting others.
- [ ] Decide active view/client behavior later.

Milestone:

```text
Open two clients.
Type in one.
Both receive updates.
Closing one leaves the other and server running.
```

## Phase 14: Add Explicit Server Shutdown

Goal: provide reliable lifecycle control.

- [ ] Implement `st quit` fully.
- [ ] Send `ShutdownServer` over IPC.
- [ ] Server notifies clients before shutdown.
- [ ] Clients exit or show disconnected state.
- [ ] Server shuts down Deno runtime.
- [ ] Server removes socket file on exit.
- [ ] Server exits cleanly.

## Phase 15: Add Optional Idle Shutdown

Goal: avoid unwanted background daemons during early development.

- [ ] Add configurable idle timeout.
- [ ] Track connected client count.
- [ ] If no clients remain for N seconds/minutes, shutdown server.
- [ ] Disable idle shutdown when launched explicitly as long-running service.

## Phase 16: Load JavaScript From Disk

Goal: stop embedding JS source in Rust.

- [ ] Create `runtime/bootstrap.js`.
- [ ] Create `runtime/editor_api.js`.
- [ ] Load bootstrap from disk in server runtime.
- [ ] Report JS syntax/runtime errors clearly.
- [ ] Keep server alive if JS fails to load.

## Phase 17: JS Commands And Keybindings

Goal: make the editor programmable.

- [ ] Expose command registration API to JS.
- [ ] Expose keybinding registration API to JS.
- [ ] Track command ownership.
- [ ] Allow JS command to request an `EditorCommand`.
- [ ] Apply requested command in Rust server.

## Phase 18: Manual Hot Reload

Goal: reload JS without recompiling Rust.

- [ ] Add command to reload JS runtime/extensions.
- [ ] Dispose old JS resources.
- [ ] Reload JS files from disk.
- [ ] Re-register commands/keybindings.
- [ ] Report reload errors without crashing server or clients.

## Phase 19: Extension Lifecycle

Goal: prepare for real extensions.

- [ ] Define extension activation API.
- [ ] Define optional deactivation API.
- [ ] Track extension-owned resources.
- [ ] Dispose resources on reload/unload.
- [ ] Isolate activation errors.

## Phase 20: File I/O

Goal: edit real files.

- [ ] Add open file command.
- [ ] Add save file command.
- [ ] Track buffer path.
- [ ] Track dirty state.
- [ ] Handle file read/write errors.
- [ ] Route file requests through server.

## Phase 21: Undo/Redo

Goal: make editing usable.

- [ ] Define edit transactions.
- [ ] Add undo stack.
- [ ] Add redo stack.
- [ ] Store cursor state before/after edits.
- [ ] Implement undo insert.
- [ ] Implement undo delete/backspace.
- [ ] Add tests.

## Phase 22: Better Buffer Data Structure

Goal: support larger files.

- [ ] Add tests around current `String` buffer behavior.
- [ ] Evaluate `ropey` or another text storage structure.
- [ ] Introduce buffer IDs.
- [ ] Support multiple buffers.
- [ ] Add line/column mapping.

## Phase 23: Systemd Integration

Goal: allow persistent background server operation.

- [ ] Add documented foreground server mode.
- [ ] Add documented user service example.
- [ ] Decide whether `st server --daemon` is needed or systemd is enough.
- [ ] Ensure socket path and cleanup work under systemd.

## Phase 24: Permissions And AI Layer

Goal: add advanced features safely after the editor core works.

- [ ] Define permission model for extensions.
- [ ] Gate file/network/subprocess access.
- [ ] Add AI command/tool API.
- [ ] Route AI edits through normal command/transaction system.
- [ ] Make AI edits previewable/reversible.
