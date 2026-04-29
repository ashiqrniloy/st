# st

`st` is an experimental AI-native, extensible text editor with a Rust server, a Deno-powered extension runtime, and native GPUI clients.

## Architecture

The editor is being built as a client/server application from the start.

```text
st server
  - Rust editor core
  - canonical editor state
  - buffers, cursors, selections, commands
  - deno_core JavaScript/TypeScript runtime
  - extension/plugin lifecycle
  - file I/O and permissions later
  - local IPC listener

st client
  - GPUI native window
  - input collection
  - rendering frontend
  - sends events/commands to server
  - receives render/scene updates from server
```

Clients are disposable. The server is persistent.

Closing a UI window should close only that client. The server should continue running until explicitly stopped, for example by `st quit` or a systemd service stop.

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

JavaScript/TypeScript customizes behavior through typed APIs:

- commands
- keybindings
- themes
- modes
- decorations
- AI behavior
- non-critical UI behavior

JavaScript should not own canonical buffer text. Extensions request mutations through typed commands; Rust validates and applies them.

## IPC Model

The first transport should be a local Unix domain socket:

```text
$XDG_RUNTIME_DIR/st/st.sock
```

Initial messages can use newline-delimited JSON for easy debugging. Later this may move to a binary format.

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
  - Render/SceneUpdate
  - EditorStateChanged
  - Error
```

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

Manual server/client mode should work before daemonization or systemd integration.

## Long-Term Direction

The long-term design is similar in spirit to Emacs: programmable, introspectable, extensible, and customizable. The core difference is that Rust owns canonical editor correctness while JavaScript/TypeScript provides user programmability through controlled APIs.
