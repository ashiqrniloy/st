# Long-Term Performance Direction

This document records the intended performance direction for `st` as an Emacs-scale, extensible, do-everything editor.

## Summary

The current Rust + GPUI + server/client + Deno design can scale well if the responsibilities remain strict:

```text
Rust core
  hot path, canonical editor state, transactions, buffers, sessions, key dispatch,
  rendering model, permissions, scheduling

GPUI clients
  input and rendering only
  no canonical editor state

Deno editor runtime
  configuration, lightweight commands, keybindings, hooks, extension registration

Native/background workers
  parsing, indexing, search, syntax, project analysis, expensive CPU work

External processes
  LSP servers, formatters, linters, language tooling

Per-agent Deno runtimes/processes
  long-running AI agents and generated tools
```

Deno should be the extension/configuration orchestration layer, not the editor engine.

The editor should not evolve into:

```text
every keypress -> one shared JavaScript runtime -> editor mutation -> render update
```

That would recreate the classic single-threaded editor bottleneck.

## Core Rule

The Rust server owns the hot path.

Typing, cursor movement, selection, undo/redo, buffer mutation, key dispatch, scene generation, and permission checks should work without JavaScript participation unless a registered extension command explicitly requires it.

## Current Architecture Evaluation

The current architecture is a good base because:

- the UI client is separate from the state-owning server
- GPUI handles native input/rendering on the client side
- Rust owns canonical editor state
- Deno is already behind the server
- ordinary text input is intended to be handled by Rust first
- server/client IPC allows clients to come and go independently

The main long-term risk is allowing one shared Deno runtime to accumulate too much responsibility.

A single `deno_core::JsRuntime` has one V8 isolate/event loop. Async JavaScript can yield, but synchronous JavaScript blocks that runtime. One CPU-heavy extension or infinite loop can delay other work in the same runtime.

Therefore:

```text
one shared editor runtime is acceptable for lightweight extension/config work
one shared editor runtime is not acceptable for agents, indexing, parsing, LSP, or heavy mode logic
```

## Multithreading Strategy

Use different concurrency tools for different work.

### Tokio Async Tasks

Use for I/O-bound concurrency:

```text
IPC sockets
LSP process I/O
file watcher events
AI API streaming
network requests
runtime message passing
timers
```

Tokio helps with concurrency, but CPU-heavy work must not run on the async reactor threads for long periods.

### CPU Worker Pool / Rayon

Use for CPU-heavy parallel work:

```text
project indexing
large search
symbol extraction
batch syntax analysis
markdown backlink graph computation
note graph calculation
```

### Dedicated OS Threads

Use for long-lived components that need ownership/thread affinity or must not block the server loop:

```text
Deno runtimes
agent runtimes
long-running background subsystems
```

### External Processes

Use where process isolation or ecosystem compatibility is best:

```text
LSP servers
formatters
linters
heavy language tools
possibly untrusted/generated agents later
```

## Server Event Loop Rule

The central server event loop should be the canonical state owner and should remain fast.

It should:

- receive messages
- mutate editor state quickly
- enqueue outgoing messages
- schedule background work
- apply completed worker results if still valid

It should not:

- run JavaScript inline
- perform blocking file I/O
- perform expensive parsing/indexing
- wait synchronously for LSP, AI, or worker results
- hold server/editor state across `.await`
- write directly to slow client sockets

## Buffer And Text Storage

The current early `String` buffer is not a long-term editor data structure.

Long-term requirements:

```text
efficient insert/delete
efficient line/column mapping
UTF-8 and UTF-16 mapping for LSP
large file support
snapshots for workers
incremental parse ranges
selection ranges
undo/redo transactions
```

Likely direction:

```text
ropey or another rope/piece-table structure first
custom storage only if necessary later
```

## Versioned Snapshots

Every background computation should use versioned snapshots.

Example:

```text
BufferId = 7
BufferVersion = 123
worker receives snapshot at version 123
worker returns result for version 123
server applies result only if current version is still compatible
otherwise result is discarded as stale
```

This applies to:

```text
syntax parsing
markdown outline
note backlink indexing
search indexes
completion requests
LSP diagnostics
AI proposed edits
```

## Rendering And Scene Updates

The current `SceneUpdate` that sends the full text is acceptable only for early development.

Long-term, avoid sending full buffer text after every edit.

Move toward:

```text
viewport-based rendering
incremental scene patches
line cache
style spans
decorations
cursor updates separate from text updates
selection updates separate from text updates
diagnostics/decorations as separate update streams
```

Future protocol shape may include:

```rust
enum ServerToClient {
    SceneSnapshot(SceneSnapshot),
    ScenePatch(ScenePatch),
    CursorUpdate(...),
    SelectionUpdate(...),
    DecorationUpdate(...),
    DiagnosticsUpdate(...),
}
```

For multiple clients, each client can have a different viewport. The server should eventually send only the visible/needed data to each client.

## Extension API Performance Rules

Extension APIs should avoid accidental slow paths.

Avoid making this the normal path:

```ts
const text = await editor.getFullText();
```

Prefer range/viewport APIs:

```ts
await editor.buffer.getRange(range);
await editor.buffer.getVisibleRanges();
await editor.buffer.getLine(lineNumber);
await editor.buffer.edit(transaction);
```

If full-buffer access exists, it should be explicit and possibly permission-gated:

```ts
await editor.buffer.getFullText({ allowLarge: true });
```

Subscriptions should be filterable and debounceable:

```ts
editor.onDidChangeText({
  filePattern: "*.md",
  debounceMs: 100,
  ranges: "changed",
}, callback);
```

Do not encourage every extension to run on every keystroke.

## Modes

Modes should register capabilities and configuration. Heavy work should be handled by Rust/native services or background workers.

Good:

```ts
modes.register("markdown", {
  filePatterns: ["*.md"],
  syntax: { parser: "tree-sitter-markdown" },
  outline: { provider: "builtin.markdownHeadings" },
});
```

Bad:

```ts
editor.onEveryKeystroke(text => parseEntireDocumentInJavaScript(text));
```

Markdown, notes, and coding modes may need:

```text
incremental parsing
syntax highlighting
outline extraction
folding
links/backlinks
tag extraction
full-text search
completion
LSP diagnostics
```

Those should be incremental, cancellable, and versioned.

## LSP

LSP should generally be managed by Rust and external LSP processes.

TypeScript should configure LSP:

```ts
lsp.register({
  language: "rust",
  command: "rust-analyzer",
});
```

But Rust should handle:

```text
process lifecycle
async request/response routing
incremental didChange
diagnostics cache
completion cache
stale response handling
```

Do not route every LSP message through JavaScript unless explicitly needed.

## Autocomplete

Autocomplete should be asynchronous, cancellable, and stale-result safe.

Flow:

```text
user types
  -> Rust updates buffer immediately
  -> completion request scheduled/debounced
  -> providers run async/in workers
  -> stale results are cancelled or discarded
  -> UI receives completion update later
```

Every completion request should include:

```text
BufferId
BufferVersion
cursor position
request id
```

## AI Agents

AI agents are long-running and unpredictable.

Rules:

```text
agent never owns editor state
agent never blocks server event loop
agent works from snapshots
agent proposes edits/tool calls
server validates and applies accepted edits
```

Use one runtime per active agent, and eventually consider one process per active agent for stronger isolation.

## JavaScript, TypeScript, And JIT

`deno_core` runs JavaScript in V8. TypeScript must be transpiled to JavaScript before execution.

V8 already JIT-compiles JavaScript internally. Additional custom JIT work is generally not needed.

Important caveats:

- V8 JIT does not make one JS runtime parallel.
- V8 JIT does not make TypeScript typechecking free.
- V8 JIT does not fix large text copying or bad protocol design.
- JS should still not own editor hot paths.

Recommended TypeScript strategy:

```text
support TypeScript source
transpile/cache compiled JavaScript
allow packaged extensions to ship precompiled JavaScript optionally
avoid typechecking on every startup unless requested
provide explicit extension check/build commands later
```

Potential commands later:

```text
st extension check
st extension build
st extension cache clear
```

## IPC

Newline-delimited JSON is acceptable early.

Long-term bottlenecks are more likely to come from payload size than JSON itself. Avoid sending full buffers frequently.

If needed later, consider:

```text
MessagePack
bincode/postcard
Cap'n Proto/FlatBuffers
shared memory for large payloads
```

Do not optimize the wire format before reducing payload size and measuring.

## Measurement Requirements

Add tracing/metrics before performance problems become mysterious.

Track:

```text
key-to-scene latency
scene-to-paint latency
IPC message sizes
server event-loop queue depth
per-client outbound queue depth
Deno command duration
worker task duration
parse/index duration
LSP response duration
completion latency
memory per buffer
```

## Development Rules

1. Rust owns the hot path.
2. JS registers behavior; Rust dispatches and validates.
3. No full-buffer updates by default.
4. Every background result is versioned.
5. Every expensive task is cancellable.
6. No state lock across `.await`.
7. Extensions cannot mutate state directly.
8. One blocked extension/agent must not block the editor.
9. Measure early.
10. Prefer incremental everything.

## Final Recommendation

Keep the current architecture, but evolve it with strict performance boundaries:

```text
Rust server
  canonical state + hot path + scheduling

GPUI clients
  native rendering/input

Deno editor runtime
  lightweight programmable behavior

Native workers
  CPU-heavy editor services

External processes
  LSP/language tools

Per-agent runtimes/processes
  AI agents and generated tools
```

This preserves Emacs-like extensibility while avoiding Emacs-style single-thread bottlenecks.
