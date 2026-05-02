## Phase 11: Incremental Scene And Viewport Protocol

Goal: stop depending on full-buffer scene updates before large files, multiple clients, decorations, and modes make that path expensive.

Acceptance criteria:

- Clients report viewports and the server tracks each client's viewport independently.
- Typing in a large buffer does not require full-buffer scene updates after initial sync/resync.
- Cursor, selection, visible text, decoration, and diagnostic updates have incremental protocol paths.
- Payload-size instrumentation demonstrates reduced update size for viewport-scoped edits.

Test plan:

- Add protocol tests for viewport reporting and independent per-client viewport state.
- Add scene-update tests proving cursor, selection, text, decoration, and diagnostic changes can be sent incrementally.
- Add payload-size tests or metrics assertions comparing viewport-scoped updates with full-buffer snapshots.

Implementation tasks:

- [x] Define client viewport reporting from GPUI client to server.
- [x] Track each client's viewport independently on the server.
- [x] Split scene updates into snapshot and patch concepts.
- [x] Add cursor-only update messages.
- [x] Add selection-only update messages.
- [x] Add visible-line text update messages.
- [x] Add decoration/style-span update messages.
- [x] Add diagnostics update messages for future LSP support.
- [x] Avoid sending full buffer text after every edit.
- [x] Send only visible or required ranges to each client where practical.
- [x] Keep an initial full scene snapshot path for first render/resync.
- [x] Add resync handling if a client misses or rejects a patch.
- [x] Track IPC payload sizes before and after the protocol change.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
Typing in a large buffer does not require sending the entire buffer to every client on every edit.
```

