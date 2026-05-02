## Phase 12: Background Worker Architecture

Goal: create the worker foundation needed for modes, parsing, indexing, search, LSP coordination, and autocomplete without blocking the server event loop.

Acceptance criteria:

- Expensive CPU and I/O work is scheduled outside the central server event loop.
- Workers receive immutable snapshots or typed requests, never mutable editor state.
- Worker responses include request/session/buffer/version metadata and stale results are discarded.
- Cancellation, timeout, queue-depth, and duration behavior is observable and tested.

Test plan:

- Add tests that long-running CPU/I/O tasks do not block typing or central event-loop message handling.
- Add tests that workers receive immutable snapshots or typed requests and that stale versioned responses are discarded.
- Add tests for cancellation, timeout behavior, queue-depth metrics, and task-duration metrics.

Implementation tasks:

- [x] Define a background task request/response model.
- [x] Define worker request IDs for correlating responses.
- [x] Define cancellable task handles.
- [x] Add a CPU worker pool for CPU-heavy tasks.
- [x] Use Tokio tasks for I/O-heavy background work.
- [x] Use dedicated OS threads where a subsystem has ownership or thread-affinity requirements.
- [x] Ensure the central server event loop schedules work but does not execute expensive work inline.
- [x] Ensure workers receive immutable snapshots or typed requests, not mutable editor state.
- [x] Ensure worker responses include relevant session/buffer/version metadata.
- [x] Discard stale worker responses based on `BufferVersion` or request cancellation.
- [x] Add cancellation support for parse/search/completion-style tasks.
- [x] Add timeout support for worker requests where appropriate.
- [x] Add metrics for worker queue depth and task duration.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
The server can schedule expensive work in parallel without blocking client IPC, typing, rendering updates, or unrelated background tasks.
```

