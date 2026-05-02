## Phase 9: Performance Guardrails And Instrumentation

Goal: establish performance rules early so the editor does not mature around unscalable paths.

Acceptance criteria:

- Typing, cursor movement, selection-ready paths, undo/redo-ready paths, and scene generation remain Rust-owned and do not require JavaScript.
- Instrumentation exposes key-to-scene latency, scene-to-client latency, IPC payload sizes, event-loop queue depth, outbound queue depth, and runtime/worker timings where applicable.
- Server/editor state is never held across socket I/O awaits or JavaScript execution.
- Tests or assertions guard the non-blocking dispatch and state-ownership rules that can be checked at this phase.

Test plan:

- Add unit tests or assertions for Rust-owned hot-path dispatch where practical.
- Add instrumentation tests or focused assertions for latency counters, payload-size counters, queue-depth counters, and runtime/worker timing hooks.
- Add a code-structure/regression test or review checklist item proving server/editor state is not held across socket I/O awaits or JavaScript execution.

Implementation tasks:

- [x] Document in code comments that Rust owns the editor hot path.
- [x] Ensure ordinary typing, cursor movement, selection, undo/redo, and scene generation do not require JavaScript.
- [x] Ensure JavaScript registers behavior while Rust dispatches and validates commands.
- [x] Add tracing or timing hooks around key-to-scene latency.
- [x] Add tracing or timing hooks around scene-to-client latency.
- [x] Add tracing or counters for IPC message sizes.
- [x] Add tracing or counters for central server event-loop queue depth.
- [x] Add tracing or counters for per-client outbound queue depth.
- [x] Add timing around Deno command execution.
- [x] Add timing around background worker task execution once workers exist.
- [x] Add a rule that server/editor state is never held across socket I/O awaits.
- [x] Add a rule that server/editor state is never held while executing JavaScript.
- [x] Add a rule that extensions request typed commands/transactions instead of mutating editor state directly.
- [x] Link this phase to `performance.md` as the detailed rationale.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
The project has explicit performance guardrails and basic latency/message-size visibility before extension and mode complexity grows.
```

