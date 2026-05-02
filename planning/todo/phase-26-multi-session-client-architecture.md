## Phase 26: Multi-Session Client Architecture

Goal: support multiple independent clients on one server with separate sessions and editor state, while still allowing explicit shared-session attachment later. Implement the proper architecture directly, not a temporary shared-global-state workaround.

Acceptance criteria:

- Each client receives an independent session by default.
- Clients share updates only through explicit attach semantics.
- The central server event loop remains the only mutator of clients, sessions, buffers, and editor state.
- Disconnecting or slowing one client does not block or corrupt unrelated clients/sessions.

Test plan:

- Add ID allocation and client/session/buffer association tests.
- Add independent-session tests proving two default clients diverge.
- Add explicit attach tests proving attached clients share updates and unattached clients do not.
- Add disconnect/slow-client tests proving unrelated clients and sessions are unaffected.

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

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

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

