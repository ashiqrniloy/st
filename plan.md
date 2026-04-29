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

## Phase 9: Wire Commands Into EditorState

Goal: make server-owned editor state mutate through Rust commands before refining Deno dispatch.

- [x] Convert text key input into `EditorCommand::InsertText`.
- [x] Convert Backspace into `EditorCommand::Backspace`.
- [x] Convert left arrow into `EditorCommand::MoveCursorLeft`.
- [x] Convert right arrow into `EditorCommand::MoveCursorRight`.
- [x] Apply commands to server-owned `EditorState`.
- [x] Add tests for key-input-to-command translation.
- [x] Log updated buffer/cursor state after each command initially.

Milestone:

```text
Client window receives keys.
Server receives keys.
Server mutates EditorState in Rust.
Server logs buffer and cursor.
```

## Phase 10: Establish Server-First Input Dispatch Architecture

Goal: refine the Phase 9 input path so ordinary typing is handled by Rust and Deno is not placed in the mandatory hot path.

Target event forwarding flow already available from Phase 8:

```text
Client -> IPC -> Server -> JS Runtime
```

Important target architecture for input handling:

```text
OS/compositor -> focused UI client -> IPC -> Rust server core -> EditorState -> SceneUpdate -> client renderer
```

Deno should not be the mandatory hot path for every normal character typed. The client captures platform input because the OS delivers keyboard events to the focused window. The server should then handle ordinary editor behavior in Rust first. Deno participates when extensions, custom commands, keybindings, modes, or AI behavior need to observe or influence the event.

Preferred long-term flow for ordinary typing:

```text
Client captures KeyInputEvent
Server converts input to EditorCommand in Rust
Server applies EditorCommand to EditorState
Server produces SceneUpdate
Client renders SceneUpdate
```

Preferred long-term flow for extension-customized behavior:

```text
Client captures KeyInputEvent
Server checks Rust/editor keymap and extension registrations
Server forwards selected events/commands to Deno when needed
Deno requests typed EditorCommand
Server validates and applies EditorCommand
Server produces SceneUpdate
Client renders SceneUpdate
```

Implementation tasks:

- [x] Client captures focused-window key input.
- [x] Client sends `KeyInputEvent` to server over IPC.
- [x] Server receives key input before Deno.
- [x] Server can forward selected events to Deno.
- [x] Stop forwarding every ordinary `KeyInputEvent` to Deno by default.
- [x] Handle ordinary printable text through Rust command application from Phase 9.
- [x] Handle built-in editing keys through Rust command application from Phase 9.
- [x] Add an explicit temporary policy for which events are still forwarded to Deno for logging/debugging.
- [x] Document that extension keybindings later register with Rust before Deno is invoked.

## Phase 11: Send Render Updates From Server To Client

Goal: have server state drive client rendering.

- [x] Add server-to-client render/update channel per connected client.
- [x] After `EditorState` changes, produce temporary render output.
- [x] Send `ServerToClient::Render` to the active client.
- [x] Client receives render messages from IPC.
- [x] Client applies render messages to GPUI view.
- [x] Keep temporary rectangle rendering for now.
- [x] Add cursor placeholder rendering.

Milestone:

```text
Typing in client mutates server state.
Server sends render update.
Client redraws.
```

## Phase 12: Replace Temporary RenderCommand With Scene Updates

Goal: stop treating JS/client rendering as raw drawing long-term.

- [x] Define `Scene` or `SceneUpdate` type.
- [x] Represent background.
- [x] Represent text placeholders or glyph runs.
- [x] Represent cursor.
- [ ] Represent selections later.
- [x] Replace most `RenderCommand` usage with `SceneUpdate`.
- [x] Keep `RenderCommand` only if needed for temporary debugging.

## Phase 13: Render Visible Text

Goal: make the editor visibly editable.

- [x] Choose short-term text rendering approach.
- [x] Render buffer text in the client.
- [x] Render cursor position.
- [x] Handle newlines.
- [x] Keep layout simple and fixed-width initially.
- [x] Add placeholder scrolling only if necessary.

Milestone:

```text
Open client.
Type characters.
Characters appear in the window.
Backspace works.
Cursor moves left/right.
Server owns state.
```

## Phase 14: Replace Temporary Text Rendering With GPUI Native Editor View

Goal: use GPUI's native text input and layout model instead of temporary `observe_keystrokes` plus inline cursor text rendering.

Architectural rule: the client may use GPUI-native input APIs, but the server remains the canonical owner of editor state. The GPUI editor view is a focused input/rendering surface and local mirror, not the source of truth.

- [x] Create a custom GPUI editor view for the text surface.
- [x] Implement `Focusable` for the editor view.
- [x] Store and use a `FocusHandle` for proper focus ownership.
- [x] Replace global `observe_keystrokes` text editing with focused editor input handling.
- [x] Implement `EntityInputHandler` for the editor view or backing input entity.
- [x] Use `ElementInputHandler` during element paint via `window.handle_input(...)`.
- [x] Support IME composition.
- [x] Support marked text.
- [x] Support selected text ranges.
- [x] Support platform text replacement callbacks.
- [x] Support clipboard semantics through GPUI/platform input paths.
- [x] Provide text bounds for the OS via `bounds_for_range`.
- [x] Provide mouse-to-character mapping via `character_index_for_point`.
- [x] Use shaped text layout for visible text.
- [x] Paint cursor separately from text content.
- [x] Paint selections separately from text content.
- [x] Stop rendering cursor by inserting a cursor character into display text.
- [x] Convert GPUI input callbacks into IPC events/commands sent to the server.
- [x] Apply server `SceneUpdate` responses back into the local editor view mirror.
- [x] Add tests for client-side offset conversion helpers where practical.
- [x] Keep server-owned `EditorState` as the canonical buffer.

Milestone:

```text
Client editor surface owns focus correctly.
Typing still goes through the server.
IME/marked text/platform replacement paths are represented.
Cursor and selections are painted separately.
Server remains canonical state owner.
```

## Phase 15: Support Multiple Clients

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

## Phase 16: Add Explicit Server Shutdown

Goal: provide reliable lifecycle control.

- [ ] Implement `st quit` fully.
- [ ] Send `ShutdownServer` over IPC.
- [ ] Server notifies clients before shutdown.
- [ ] Clients exit or show disconnected state.
- [ ] Server shuts down Deno runtime.
- [ ] Server removes socket file on exit.
- [ ] Server exits cleanly.

## Phase 17: Add Optional Idle Shutdown

Goal: avoid unwanted background daemons during early development.

- [ ] Add configurable idle timeout.
- [ ] Track connected client count.
- [ ] If no clients remain for N seconds/minutes, shutdown server.
- [ ] Disable idle shutdown when launched explicitly as long-running service.

## Phase 18: Load JavaScript From Disk

Goal: stop embedding JS source in Rust.

- [ ] Create `runtime/bootstrap.js`.
- [ ] Create `runtime/editor_api.js`.
- [ ] Load bootstrap from disk in server runtime.
- [ ] Report JS syntax/runtime errors clearly.
- [ ] Keep server alive if JS fails to load.

## Phase 19: JS Commands And Keybindings

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

## Phase 20: Manual Hot Reload

Goal: reload JS without recompiling Rust.

- [ ] Add command to reload JS runtime/extensions.
- [ ] Dispose old JS resources.
- [ ] Reload JS files from disk.
- [ ] Re-register commands/keybindings.
- [ ] Report reload errors without crashing server or clients.

## Phase 21: Extension Lifecycle

Goal: prepare for real extensions.

- [ ] Define extension activation API.
- [ ] Define optional deactivation API.
- [ ] Track extension-owned resources.
- [ ] Dispose resources on reload/unload.
- [ ] Isolate activation errors.

## Phase 22: File I/O

Goal: edit real files.

- [ ] Add open file command.
- [ ] Add save file command.
- [ ] Track buffer path.
- [ ] Track dirty state.
- [ ] Handle file read/write errors.
- [ ] Route file requests through server.

## Phase 23: Undo/Redo

Goal: make editing usable.

- [ ] Define edit transactions.
- [ ] Add undo stack.
- [ ] Add redo stack.
- [ ] Store cursor state before/after edits.
- [ ] Implement undo insert.
- [ ] Implement undo delete/backspace.
- [ ] Add tests.

## Phase 24: Better Buffer Data Structure

Goal: support larger files.

- [ ] Add tests around current `String` buffer behavior.
- [ ] Evaluate `ropey` or another text storage structure.
- [ ] Introduce buffer IDs.
- [ ] Support multiple buffers.
- [ ] Add line/column mapping.

## Phase 25: Systemd Integration

Goal: allow persistent background server operation.

- [ ] Add documented foreground server mode.
- [ ] Add documented user service example.
- [ ] Decide whether `st server --daemon` is needed or systemd is enough.
- [ ] Ensure socket path and cleanup work under systemd.

## Phase 26: Adopt GPUI Component For Non-Editor UI

Goal: improve application chrome and supporting UI using `gpui-component` without replacing the custom server-backed editor surface.

Explicit boundary: `gpui-component` is not used for the editor itself. The editor remains our custom GPUI native editor view from Phase 14, backed by server-owned editor state.

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

## Phase 27: Permissions And AI Layer

Goal: add advanced features safely after the editor core works.

- [ ] Define permission model for extensions.
- [ ] Gate file/network/subprocess access.
- [ ] Add AI command/tool API.
- [ ] Route AI edits through normal command/transaction system.
- [ ] Make AI edits previewable/reversible.
