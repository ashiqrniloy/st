## Phase 2: Add Explicit Server Shutdown

Goal: provide reliable lifecycle control.

- [x] Implement `st quit` fully.
- [x] Send `ShutdownServer` over IPC.
- [x] Server notifies clients before shutdown.
- [x] Clients exit or show disconnected state.
- [x] Server shuts down Deno runtime.
- [x] Server removes socket file on exit.
- [x] Server exits cleanly.

