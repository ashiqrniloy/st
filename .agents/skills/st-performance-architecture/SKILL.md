---
name: st-performance-architecture
description: Use this skill whenever working on the st editor codebase, especially when changing Rust server/client architecture, Deno/TypeScript runtimes, editor state, buffers, rendering, IPC, extensions, modes, LSP, autocomplete, AI agents, or background work. It enforces the project's non-negotiable performance rules so st remains a scalable, multithreaded, Emacs-like editor instead of growing around single-threaded or JavaScript-heavy bottlenecks.
---

# st Performance Architecture

Use this skill before designing or editing performance-sensitive code in `st`.

If available, also read the project root `performance.md` for full rationale. A bundled copy may exist at `references/performance.md`.

## Non-negotiables

1. **Rust owns the hot path.** Typing, cursor movement, selection, undo/redo, buffer mutation, key dispatch, scene generation, permissions, and transaction application must work without JavaScript unless an explicit registered extension command requires JS.
2. **Deno is orchestration, not the editor engine.** Use Deno/TypeScript for config, lightweight commands, keybindings, hooks, mode registration, and extension orchestration.
3. **Never make one shared JS runtime responsible for heavy work.** Parsing, indexing, LSP routing, autocomplete computation, AI agents, and expensive mode logic must not accumulate in the normal editor runtime.
4. **The central server event loop must stay fast.** It may mutate canonical state, enqueue messages, and schedule work. It must not block on JavaScript, socket I/O, LSP, AI, file scanning, parsing, indexing, or CPU-heavy work.
5. **No editor/server state across `.await`.** Do not hold canonical state while awaiting socket I/O, file I/O, JS runtime execution, LSP responses, AI calls, or worker completion.
6. **Extensions and agents never mutate editor state directly.** They submit typed command/transaction requests; Rust validates and applies them.
7. **Background work is snapshot-based and versioned.** Worker/LSP/AI/completion results must include buffer/session/version/request metadata and stale results must be discarded.
8. **Prefer incremental and viewport-scoped updates.** Avoid sending or copying full buffers by default.
9. **One blocked component must not block unrelated work.** A blocked client, extension, agent, parser, LSP, or worker must not block typing, server dispatch, other clients, or other agents.
10. **Measure early.** Add/maintain timing and counters for latency, IPC sizes, queue depth, JS duration, worker duration, and memory-sensitive paths.

## Preferred architecture patterns

### Server/client concurrency

```text
per-client Tokio task
  owns socket read/write only
  sends messages to central server channel
  receives outbound messages from per-client channel

central server event loop
  owns canonical clients/sessions/buffers/editor state
  applies quick mutations
  enqueues outbound messages
  schedules background work
```

Do not introduce `Arc<Mutex<EditorServer>>` patterns where client tasks lock and mutate editor state directly.

### Runtime separation

```text
normal editor Deno runtime
  config, commands, keybindings, themes, modes, lightweight hooks

per-agent runtime workers
  one runtime per active long-running agent

native/background workers
  parsing, indexing, search, syntax analysis, autocomplete computation

external processes
  LSP servers, formatters, linters, heavy language tools
```

### Worker pattern

```text
server creates immutable snapshot/request
server sends it to worker
worker computes in parallel
worker returns result with BufferVersion/RequestId
server applies only if still current and valid
```

## When adding code, check these questions

- Is this on the typing/rendering hot path? If yes, keep it in Rust and fast.
- Could this block for more than a tiny bounded time? If yes, schedule it outside the server event loop.
- Does this require full-buffer text? If yes, can it use a range, viewport, line, or snapshot instead?
- Is this result stale-sensitive? If yes, include `BufferVersion` or request metadata.
- Could multiple clients/sessions exist? If yes, do not assume global editor state.
- Could one extension/agent/tool misbehave? If yes, isolate it through workers, channels, cancellation, and timeouts.
- Is JavaScript necessary for this path? If not, keep dispatch and mutation in Rust.

## Required recommendations for future-facing code

- Use typed messages and commands instead of ad-hoc strings for state mutation.
- Prefer bounded or monitored queues for high-volume paths.
- Track per-client outbound queue behavior so slow clients are detectable.
- Design extension APIs around ranges, visible regions, filtered subscriptions, and explicit full-buffer access.
- Add cancellation for completions, parsing, search, indexing, AI, and tool calls.
- Add timeout/supervision for runtime and worker tasks.
- Keep LSP management in Rust with external LSP processes; TypeScript should configure it, not route every message by default.
- Cache/transpile TypeScript; remember V8 already JITs JavaScript but does not make one runtime parallel.

## If a requested change violates these rules

Stop and explain the performance risk. Propose an architecture-compatible alternative before editing.
