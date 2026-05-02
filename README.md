# st

`st` is an experimental AI-native, extensible text editor with a Rust server, a Deno-powered extension runtime, and native GPUI clients.

## Architecture

The editor is being built as a client/server application from the start.

```text
st server
  - Rust editor core
  - canonical editor state
  - buffers, cursors, selections, commands
  - deno_core JavaScript runtime
  - extension/plugin lifecycle
  - file I/O and permissions later
  - local IPC listener

st client
  - GPUI native window
  - focused GPUI editor surface
  - GPUI-native text input/IME bridge
  - shaped text rendering frontend
  - sends events/commands to server
  - receives scene updates from server
```

Clients are disposable. The server is persistent.

Closing a UI window closes only that client. The server continues running until explicitly stopped, for example by `st quit` or a future systemd service stop.

A normal `st`/`st client` launch attempts to connect to the server. If no server is available, the client starts one in the background, waits for the Unix socket to become available, then opens the GPUI window.

## Ownership Model

Rust owns correctness-critical state:

- canonical buffer contents
- cursor and selection state
- command application
- undo/redo transactions
- file identity and persistence state
- extension resource tracking
- permission checks
- render scene generation

JavaScript customizes behavior through typed APIs:

- commands
- keybindings
- themes
- modes
- decorations
- AI behavior
- non-critical UI behavior

JavaScript should not own canonical buffer text. Extensions request mutations through typed commands; Rust validates and applies them.

## IPC Model

The first transport is a local Unix domain socket:

```text
$XDG_RUNTIME_DIR/st/st.sock
```

If `$XDG_RUNTIME_DIR` is unavailable, the implementation falls back to a temporary runtime directory.

Messages currently use newline-delimited JSON for easy debugging. Later this may move to a binary format.

Example message categories:

```text
ClientToServer
  - Hello
  - KeyInput
  - Command
  - CloseClient
  - ShutdownServer

ServerToClient
  - Welcome
  - SceneUpdate
  - Render debug messages
  - Error
```

The primary UI update path is now scene-driven: the server owns editor state, produces `SceneUpdate`, and each client applies that scene to its local GPUI view mirror.

## CLI Model

Target CLI shape:

```bash
st              # connect to running server, or spawn server, then open client
st client       # explicitly open a client
st server       # run server in foreground
st server --daemon
st quit         # ask server to shut down
```

When running through Cargo, put `--` before arguments meant for `st`:

```bash
cargo run -- server
cargo run -- client
cargo run -- quit
```

Do not use `cargo run --server`; Cargo interprets that as a Cargo argument, not an editor argument.

Manual server/client mode works before daemonization or systemd integration.

## Current Editor Surface

The client no longer uses global keystroke observation for ordinary text entry. It now has a custom GPUI editor view built around GPUI's native input model:

- `Focusable`
- `FocusHandle`
- `EntityInputHandler`
- `ElementInputHandler`
- `window.handle_input(...)`
- shaped text layout
- separately painted cursor
- separately painted selections

This gives the editor a path toward proper platform text behavior, including IME composition, marked text, selected text ranges, platform text replacement, text bounds for the OS, mouse-to-character mapping, clipboard integration, and focus ownership.

The client-side editor surface is still not the canonical editor state. It is a focused input/rendering surface and local mirror. Text input callbacks are converted into IPC events/commands, the server applies them to `EditorState`, and clients update from server `SceneUpdate` messages.

Currently working at the prototype level:

- opening a server and client
- client auto-starting the server when needed
- typing visible text into the GPUI window
- inserting newlines with Enter/Return
- server-owned buffer updates
- server-to-client scene updates
- Deno runtime running behind the server

Known text-editing work remains, especially robust arrow-key movement, Shift+arrow selection, richer selection semantics, and traversal by word/line/sentence. These are tracked in `plan.md` under the later text editing phase.

## GPUI Component Boundary

`gpui-component` may be adopted later for surrounding application UI, but not for the editor surface itself.

Allowed future uses include:

- command palette
- buttons
- dialogs
- settings
- panels
- tabs
- status bar
- menus
- notifications
- dock layout

The editor itself remains a custom GPUI-native, server-backed editor view so canonical buffer, cursor, selection, undo/redo, and command dispatch stay under our control.

## Long-Term Direction

The long-term design is similar in spirit to Emacs: programmable, introspectable, extensible, and customizable. The core difference is that Rust owns canonical editor correctness while JavaScript provides user programmability through controlled APIs.
