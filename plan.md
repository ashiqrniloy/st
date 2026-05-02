# Step-by-Step Implementation Plan

This document is intentionally only an ordered implementation checklist. Overall architecture and design rationale live in `README.md`.

## Phase 1: Support Multiple Clients

Goal: prove the client/server model is real.

- [x] Allow multiple clients to connect simultaneously.
- [x] Broadcast editor updates to all clients initially.
- [x] Track each client's connection state.
- [x] Handle one client disconnecting without affecting others.
- [x] Defer active view/client behavior until later.

Milestone:

```text
Open two clients.
Type in one.
Both receive updates.
Closing one leaves the other and server running.
```

## Phase 2: Add Explicit Server Shutdown

Goal: provide reliable lifecycle control.

- [x] Implement `st quit` fully.
- [x] Send `ShutdownServer` over IPC.
- [x] Server notifies clients before shutdown.
- [x] Clients exit or show disconnected state.
- [x] Server shuts down Deno runtime.
- [x] Server removes socket file on exit.
- [x] Server exits cleanly.

## Phase 3: Add Optional Idle Shutdown

Goal: avoid unwanted background daemons during early development.

- [x] Add configurable idle timeout.
- [x] Track connected client count.
- [x] If no clients remain for N seconds/minutes, shutdown server.
- [x] Disable idle shutdown when launched explicitly as long-running service.

## Phase 4: Emacs-Like Command Structure Foundation

Goal: create the command foundation now so all future user-visible behavior is added through a discoverable command system.

- [x] Define `CommandId`.
- [x] Define `CommandDescriptor` with structured metadata.
- [x] Include required command metadata: id, title, description, source, category/namespace, arguments, examples, related docs.
- [x] Define `CommandSource` for builtin, extension, tool, and generated commands.
- [x] Define `CommandHandler` for Rust builtin handlers first.
- [x] Add a server-owned `CommandRegistry`.
- [x] Register existing builtin editor commands through `CommandRegistry`.
- [x] Route client command requests through `CommandRegistry` instead of ad-hoc command handling where practical.
- [x] Keep Rust validation and editor mutation in the server.
- [x] Keep ordinary text input on the Rust hot path.
- [x] Prepare the registry shape so later JS/extension commands can register descriptors and handlers.
- [x] Add tests for command registration.
- [x] Add tests for duplicate command ID rejection.
- [x] Add tests for builtin command dispatch.
- [x] Add tests that public commands require metadata.
- [x] Link this phase to `documenation.md` as the detailed documentation rationale.

Milestone:

```text
User-visible operations are represented as registered commands with metadata, and future features have a single command system to plug into.
```

## Phase 5: Self-Documentation And Help Query Foundation

Goal: create the server-side documentation structure and command-accessible help path so available documentation can be shown inside the editor.

- [x] Define common documentation structs for summary, description, arguments, examples, related links, and source.
- [x] Define documentation query types.
- [x] Define documentation result types.
- [x] Add protocol messages for documentation queries and results, or a general command-based equivalent.
- [x] Add server handlers for listing commands.
- [x] Add server handlers for describing one command.
- [x] Add placeholder registry/query shapes for settings, keybindings, modes, extensions, tools, permissions, and API docs.
- [x] Add builtin help commands such as `help.commands` and `help.command`.
- [x] Ensure help commands themselves have command metadata.
- [x] Add a simple client rendering path for documentation results, even if initially text/log based.
- [x] Keep documentation sourced from live server registries.
- [x] Do not make clients scrape markdown files as the primary help source.
- [x] Add tests for command listing documentation.
- [x] Add tests for describing a command.
- [x] Add tests for missing documentation query errors.
- [x] Add tests that help commands are discoverable through the command registry.

Milestone:

```text
The editor can query the server for available command documentation and show it through the command/help path.
```

## Phase 6: Document Existing Builtin Capabilities

Goal: update existing implemented behavior so the project starts with documentation coverage instead of adding docs only for future features.

- [x] Add metadata for client startup/open-client behavior where user-facing.
- [x] Add metadata for server startup behavior where user-facing.
- [x] Add metadata for explicit server shutdown / `st quit`.
- [x] Add metadata for close-client behavior.
- [x] Add metadata for insert text.
- [x] Add metadata for backspace.
- [x] Add metadata for move cursor left.
- [x] Add metadata for move cursor right.
- [x] Add metadata for current scene/render update behavior where user-facing.
- [x] Add metadata for idle shutdown configuration.
- [x] Add metadata for CLI-visible commands and flags where they intersect with in-editor help.
- [x] Add tests that all builtin commands have required metadata.
- [x] Add tests that all current user-visible settings/options have required metadata once represented in registries.
- [x] Add tests that documentation descriptors are non-empty and have stable IDs.
- [x] Add a development assertion or test helper that rejects builtin public capabilities without descriptors.

Milestone:

```text
Everything already built and visible to the user has initial structured documentation metadata and can be discovered through the help foundation.
```

## Phase 7: Configuration Foundation

Goal: establish the JavaScript-first configuration foundation before adding more user-visible behavior, extensions, tools, modes, and settings.

- [x] Define configuration architecture in code using `config.md` as the rationale.
- [x] Define default config locations, including `~/.config/st/init.js`, without making them the only supported locations.
- [x] Add a typed Rust-side settings/config registry shape.
- [x] Add setting descriptors with id, title, description, type, default, valid values/range, examples, and reload/restart behavior.
- [x] Ensure setting descriptors integrate with the self-documentation metadata model.
- [x] Add configuration source tracking, such as default, CLI, init.js, YAML, extension, or runtime override.
- [x] Define precedence rules between defaults, CLI options, init.js, YAML-loaded values, and runtime changes.
- [x] Add a JavaScript-facing config API shape for future `init.js` support.
- [x] Add YAML loading as declarative data support, not as the primary behavior engine.
- [x] Add path expansion helpers for `~`, environment variables where appropriate, and relative paths.
- [x] Add configuration validation and clear diagnostics.
- [x] Add a way to report config load errors without crashing the server when possible.
- [x] Add extension/tool directory configuration concepts without enforcing one directory structure.
- [x] Add tests for setting descriptor validation.
- [x] Add tests for configuration precedence.
- [x] Add tests for invalid configuration diagnostics.
- [x] Link this phase to `config.md` and the `st-config` skill as detailed guidance.

Milestone:

```text
The project has a documented, typed, self-documenting configuration foundation before additional behavior becomes hard-coded.
```

## Phase 8: Make Existing Behavior Configurable

Goal: audit and update current behavior so existing defaults that users may reasonably want to change are represented through the configuration foundation.

- [x] Audit existing hard-coded values and defaults.
- [x] Make editor background color configurable.
- [x] Make cursor visibility/style defaults configurable where applicable.
- [x] Make server auto-start idle timeout configurable.
- [x] Make explicit foreground server idle-timeout behavior documented/configurable through CLI/config where appropriate.
- [x] Make socket/runtime directory behavior configurable where safe, while preserving sensible defaults.
- [x] Make client/server startup behavior configurable where user-facing.
- [x] Make key handling defaults configurable through the command/keymap foundation where appropriate.
- [x] Make any hard-coded UI text/style defaults configurable if user-facing.
- [x] Add setting descriptors and documentation for each existing configurable option.
- [x] Add tests that existing configurable options have defaults and docs.
- [x] Add tests that configured values override defaults.
- [x] Ensure no new user-tunable hard-coded values are introduced without descriptors.

Milestone:

```text
Current implemented behavior has been audited, and user-tunable defaults are represented through documented configuration instead of scattered hard-coded constants.
```

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

## Phase 10: Scalable Buffer Storage And Versioned Snapshots

Goal: replace the early `String` buffer model before file, mode, LSP, and extension features depend on it.

Acceptance criteria:

- The canonical buffer representation is no longer a plain long-term `String` mutation model.
- Insert/delete operations remain efficient and preserve correct cursor behavior.
- Line/column, UTF-8, and UTF-16 mappings are available and tested.
- Snapshots include buffer versions, and stale background results can be rejected.

Test plan:

- Add storage tests for large-buffer insert/delete operations and cursor preservation.
- Add mapping tests for line/column, UTF-8 offsets, and UTF-16 offsets, including multi-byte text.
- Add snapshot/version tests proving stale worker-style results can be rejected.

Implementation tasks:

- [x] Evaluate `ropey`, piece-table storage, or another scalable text storage structure.
- [x] Choose the initial scalable buffer representation.
- [x] Introduce `BufferId` if not already available.
- [x] Introduce `BufferVersion`.
- [x] Replace direct `String` mutation in `EditorState` with the chosen text storage abstraction.
- [x] Preserve efficient insert/delete operations.
- [x] Add line/column mapping.
- [x] Add UTF-8 offset mapping.
- [x] Add UTF-16 offset mapping for future LSP support.
- [x] Add range APIs for reading slices of the buffer.
- [x] Add explicit full-buffer read APIs only where needed.
- [x] Add cheap immutable snapshot support for background workers.
- [x] Include `BufferVersion` in snapshots.
- [x] Ensure background results can be discarded when their `BufferVersion` is stale.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
The editor no longer depends on a simple String as the canonical long-term buffer representation, and background work has a versioned snapshot model.
```

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

## Phase 13: Load JavaScript From Disk

Goal: stop embedding JS source in Rust.

Acceptance criteria:

- JavaScript runtime source is loaded from disk instead of embedded Rust strings.
- Syntax and runtime load errors are reported clearly.
- The server remains alive and usable when JavaScript loading fails.

Test plan:

- Add runtime-loading tests using temporary runtime files on disk.
- Add syntax/runtime error tests proving load failures are reported clearly.
- Add a failure-path test proving the server remains alive after JavaScript load failure.

Implementation tasks:

- [x] Create `runtime/bootstrap.js`.
- [x] Create `runtime/editor_api.js`.
- [x] Load bootstrap from disk in server runtime.
- [x] Report JS syntax/runtime errors clearly.
- [x] Keep server alive if JS fails to load.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

## Phase 14: JS Commands And Keybindings

Goal: make the editor programmable while keeping Rust in control of command dispatch.

Acceptance criteria:

- Key input is normalized into Rust-owned key chords and resolved through a Rust-owned keymap before text insertion.
- Rust builtin commands execute without invoking Deno.
- JS commands and keybindings can be registered with metadata and ownership tracking.
- JS command requests are validated by Rust before mutating editor state.

Test plan:

- Add key normalization and keymap-resolution tests for printable, modified, and multi-key chords.
- Add dispatch tests proving Rust builtin commands do not invoke Deno and JS commands do.
- Add registration/ownership tests for JS commands and keybindings.
- Add validation tests proving malformed or unauthorized JS command requests cannot mutate editor state.

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

editor.keymap.bind("ctrl d", "insert-date");
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

- [x] Define `KeyChord` normalized from `KeyInputEvent`.
- [x] Implement modified-key chord handling (Ctrl/Alt/Meta and multi-key chords) through Rust keymap dispatch.
- [x] Define command registry structure in Rust server.
- [x] Define keymap structure in Rust server.
- [x] Define `CommandHandler::RustBuiltin`.
- [x] Define `CommandHandler::JsCommand`.
- [x] Expose command registration API to JS.
- [x] Expose keybinding registration API to JS.
- [x] Track command ownership by extension/runtime.
- [x] Store JS-registered keybindings in Rust-owned keymap.
- [x] Resolve incoming key input through Rust keymap before text insertion.
- [x] Invoke Deno only for keybindings/commands registered by JS.
- [x] Allow JS command to request a typed `EditorCommand`.
- [x] Validate and apply requested `EditorCommand` in Rust server.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

## Phase 15: Code Review

Goal: review all code written so far and align the implementation with the project's documentation, configuration, and performance principles before adding more UI and command-center complexity.

Acceptance criteria:

- Dead code that is not useful for planned future implementation is removed.
- Code aligns with `documentation.md`, `config.md`, and `performance.md`.
- Simplifications and more elegant solutions are applied where they improve the code without weakening architecture or tests.
- All existing behavior remains covered by passing tests after cleanup.

Test plan:

- Add or update tests that protect behavior touched by cleanup before changing implementation.
- Run documentation/configuration/performance conformance checks where automated checks exist; otherwise record a focused review checklist.
- After cleanup, run the full suite to prove behavior did not regress.

Implementation tasks:

- [x] Review all code written so far.
- [x] Remove dead code that is not useful for future implementations.
- [x] Review `documentation.md`, `config.md`, and `performance.md`.
- [x] Fix any code deviation from `documentation.md`.
- [x] Fix any code deviation from `config.md`.
- [x] Fix any code deviation from `performance.md`.
- [x] Simplify implementations wherever possible without compromising architecture, correctness, documentation, configurability, or performance.
- [x] Review the implementation for more elegant solutions.
- [x] Implement more elegant solutions where they are clearly better and do not compromise project constraints.
- [x] Keep public/user-visible behavior documented through structured metadata.
- [x] Keep user-tunable behavior represented through the configuration foundation.
- [x] Keep Rust-owned hot paths and avoid introducing JavaScript-heavy bottlenecks.
- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
The codebase is cleaner, simpler, aligned with documentation/configuration/performance guidance, and ready for the next UI and command phases.
```

## Phase 16: UI Window Management And Split Panes

Goal: implement server-owned window/pane layout management with manual horizontal/vertical split commands, a DWIM split command, GPUI rendering support, JS/extension APIs, and structured user-facing documentation.

Acceptance criteria:

- A client window starts as one full-window pane and can be split into at most four panes.
- Manual horizontal split divides the selected/active pane or full window along the X axis, producing top/bottom space within that split region.
- Manual vertical split divides the selected/active pane or full window along the Y axis, producing left/right space within that split region.
- From one full-window pane, one horizontal split produces two horizontal panes; one vertical split produces two vertical panes.
- Repeating the same split direction from a two-pane same-axis layout produces exactly three same-axis panes and makes that layout terminal for further splits.
- From a two-pane horizontal layout, a vertical split splits one unsplit pane; a second vertical split splits the remaining unsplit pane, producing four panes; no additional splits are allowed.
- From a two-pane vertical layout, a horizontal split splits one unsplit pane; a second horizontal split splits the remaining unsplit pane, producing four panes; no additional splits are allowed.
- Mixed split behavior is deterministic: the active pane is preferred when eligible, otherwise the next eligible unsplit pane is chosen by a documented stable ordering.
- Consecutive same-direction layouts stop at three panes, while mixed orthogonal layouts can reach four panes.
- Split requests that would exceed the allowed layout are rejected without corrupting layout state and return a clear command/protocol result.
- Pane focus/active-pane state is tracked by Rust and survives split operations deterministically.
- Rust owns canonical window, pane, and layout state; JS/extensions request typed split commands and never mutate layout state directly.
- GPUI client rendering consumes server layout state and renders pane rectangles consistently with the Rust layout calculation.
- A DWIM split command chooses the most useful split direction from current window dimensions, pane layout, and documented/configurable aspect-ratio thresholds.
- On a wide full-screen-style window, DWIM first creates a horizontal two-pane layout, then vertically splits one horizontal pane, then vertically splits the remaining horizontal pane, then stops at four panes.
- On a window that is wide but constrained to roughly half-screen height, DWIM prefers vertical splits, can create three vertical panes, and then stops.
- On a window that is tall/narrow or constrained to roughly half-screen width, DWIM prefers horizontal splits, can create three horizontal panes, and then stops.
- Manual split commands, DWIM split command, related JS APIs, and any user-tunable DWIM thresholds are registered with structured documentation metadata and discoverable through the help system.

Test plan:

- Add Rust unit tests for layout transitions from one pane to two horizontal panes, two vertical panes, three same-axis panes, mixed three-pane layouts, mixed four-pane layouts, and rejected over-limit splits.
- Add Rust tests for active-pane selection, deterministic fallback to the next eligible pane, and focus preservation after splits.
- Add Rust tests for rectangle calculation using representative window sizes, including odd dimensions and minimum-size edge cases.
- Add DWIM tests for wide full-window dimensions, wide half-height dimensions, tall/half-width dimensions, existing same-axis layouts, existing mixed layouts, and terminal layouts.
- Add protocol/command tests proving manual and DWIM split commands are routed through Rust command handlers and return clear success/error results.
- Add JS API tests proving extensions can request split-window-horizontal, split-window-vertical, and split-window-dwim through typed APIs but cannot mutate layout state directly.
- Add documentation metadata tests proving all public split commands, JS APIs, and configurable DWIM settings have descriptors.
- Add GPUI/client rendering tests or focused layout-adapter tests proving server pane rectangles are rendered as distinct panes without client-side layout divergence.

Implementation tasks:

- [ ] Audit existing Rust window split primitives and identify which layout, command, protocol, and rendering pieces can be reused.
- [ ] Define canonical Rust layout model for a client window, including `WindowLayout`, `PaneId`, active pane, split axis, pane tree/grid representation, and terminal layout states.
- [ ] Define split invariants: maximum four panes, three-pane terminal same-axis layouts, four-pane terminal mixed layouts, deterministic active/eligible pane selection, and clear rejection behavior.
- [ ] Implement pure Rust layout transition functions for manual horizontal and vertical splits.
- [ ] Implement pure Rust pane rectangle calculation from window dimensions and layout state.
- [ ] Ensure layout calculations handle odd pixel/cell dimensions deterministically.
- [ ] Add minimum pane-size validation or a documented policy for very small windows.
- [ ] Add Rust command variants for manual horizontal split, manual vertical split, and DWIM split.
- [ ] Register split commands in the Rust command registry with structured metadata.
- [ ] Route split command execution through the server-owned command path.
- [ ] Return typed success/error results for accepted and rejected split requests.
- [ ] Add protocol messages or extend existing scene/layout updates so clients receive pane IDs, active pane ID, rectangles, and any buffer/view association needed for rendering.
- [ ] Update GPUI client layout/rendering code to render server-provided pane rectangles rather than independently deciding split geometry.
- [ ] Add focus movement/activation behavior needed to choose which pane manual split commands affect.
- [ ] Define DWIM split heuristic using current window dimensions, current layout, active pane, aspect ratio, and eligible next layouts.
- [ ] Add configurable/documented DWIM threshold settings where user-tunable, while keeping the four-pane maximum as the phase invariant.
- [ ] Implement DWIM in Rust so it can be called by users, keybindings, JS extensions, and future UI affordances without JavaScript on the hot path.
- [ ] Expose typed JS APIs for `splitWindowHorizontal`, `splitWindowVertical`, and `splitWindowDwim` that submit validated Rust commands.
- [ ] Document split commands, arguments/results, examples, JS API usage, error cases, and DWIM settings through structured descriptors.
- [ ] Add default keybinding descriptors only if default split keybindings are introduced in this phase.
- [ ] Ensure split state is scoped correctly for the current client/session/window model and does not assume a single global editor window beyond the current architecture.
- [ ] Preserve performance boundaries: Rust owns layout mutation and scene generation; JS only orchestrates explicit extension calls.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

Milestone:

```text
Users and extensions can split the active window manually or via DWIM, Rust owns and validates all pane layout state, the GPUI client renders server-provided panes, and all public split capabilities are documented and tested.
```

## Phase 17: UI Design Command Center

Goal: placeholder for command center UI design. 

- [ ] Introduce GPUI-components for overlays

## Phase 18: Fuzzy Search Implementation

Goal: placeholder for fuzzy search implementation.

- [ ] Placeholder: details to be added later.

## Phase 19: Execute Command Function

Goal: placeholder for execute-command functionality.

- [ ] Placeholder: details to be added later.

## Phase 20: View Documentation Function

Goal: placeholder for view-documentation functionality.

- [ ] Placeholder: details to be added later.

## Phase 21: Manual Hot Reload

Goal: reload JS without recompiling Rust.

Acceptance criteria:

- A user-visible command can reload JS/runtime extension state without recompiling Rust.
- Old JS-owned resources are disposed before replacement resources are registered.
- Reload errors are reported without crashing the server or connected clients.
- Commands/keybindings after reload match the newly loaded runtime state.

Test plan:

- Add reload-command tests for successful runtime reload without Rust recompilation.
- Add resource-disposal tests proving old JS-owned commands/keybindings/resources are removed before replacement.
- Add failure-path tests proving reload errors are reported and clients/server remain alive.
- Add post-reload registry tests proving commands/keybindings reflect newly loaded runtime state.

Implementation tasks:

- [ ] Add command to reload JS runtime/extensions.
- [ ] Dispose old JS resources.
- [ ] Reload JS files from disk.
- [ ] Re-register commands/keybindings.
- [ ] Report reload errors without crashing server or clients.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 22: Extension Lifecycle

Goal: prepare for real extensions.

Acceptance criteria:

- Extensions have explicit activation and optional deactivation lifecycle hooks.
- Extension-owned resources are tracked and disposed on reload/unload.
- Activation/deactivation errors are isolated and reported without corrupting server state.
- Lifecycle behavior is documented through structured metadata where user-visible.

Test plan:

- Add activation/deactivation lifecycle tests for successful extensions.
- Add resource-tracking tests proving extension-owned resources are disposed on reload/unload.
- Add error-isolation tests for activation and deactivation failures.
- Add documentation metadata tests for user-visible lifecycle commands/settings where applicable.

Implementation tasks:

- [ ] Define extension activation API.
- [ ] Define optional deactivation API.
- [ ] Track extension-owned resources.
- [ ] Dispose resources on reload/unload.
- [ ] Isolate activation errors.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 23: File I/O

Goal: edit real files.

Acceptance criteria:

- Files can be opened into server-owned buffers and saved through server-routed commands.
- Buffer path and dirty state are tracked accurately.
- Read/write errors are reported clearly without losing editor state.
- File commands participate in the command/documentation/configuration model where user-visible.

Test plan:

- Add open/save command tests using temporary files.
- Add dirty-state and buffer-path tests covering open, edit, save, and save failure paths.
- Add read/write error tests proving errors are surfaced without losing server-owned state.
- Add documentation/configuration metadata tests for user-visible file commands where applicable.

Implementation tasks:

- [ ] Add open file command.
- [ ] Add save file command.
- [ ] Track buffer path.
- [ ] Track dirty state.
- [ ] Handle file read/write errors.
- [ ] Route file requests through server.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 24: Undo/Redo

Goal: make editing usable.

Acceptance criteria:

- Edits are represented as transactions with before/after cursor state.
- Undo and redo correctly handle insert and delete/backspace operations.
- Undo/redo integrates with dirty-state and future transaction-based systems.
- Tests cover transaction ordering, cursor restoration, and redo invalidation after new edits.

Test plan:

- Add transaction tests covering before/after cursor state and edit ordering.
- Add undo/redo tests for insert, delete, and backspace operations.
- Add redo invalidation tests after new edits.
- Add dirty-state integration tests where file state exists.

Implementation tasks:

- [ ] Define edit transactions.
- [ ] Add undo stack.
- [ ] Add redo stack.
- [ ] Store cursor state before/after edits.
- [ ] Implement undo insert.
- [ ] Implement undo delete/backspace.


- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 25: Text Editing Features In Depth

Goal: revisit editor text behavior comprehensively after core storage, file, and undo/redo primitives are stronger.

Acceptance criteria:

- Selection, traversal, and editability/read-only behavior are modeled server-side.
- Movement by character, word, line, sentence, file boundary, and page is explicit and tested.
- Keyboard and mouse selection behavior is deterministic and command-routed where applicable.
- Read-only state prevents mutations while still allowing navigation and selection.

Test plan:

- Add server-side selection model tests for character, word, line, sentence, and multi-range cases where supported.
- Add traversal tests for file boundaries, arrow keys, Home/End, and PageUp/PageDown.
- Add mouse selection tests or deterministic event-model tests for click/drag/double-click/triple-click behavior.
- Add read-only/editable tests proving navigation is allowed and mutations are blocked when read-only.

Implementation tasks:

- [ ] Define canonical server-side selection model.
- [ ] Define editable vs read-only editor state.
- [ ] Make editor state not editable/read-only.
- [ ] Make editor state editable again.
- [ ] Highlight a single letter.
- [ ] Highlight a word.
- [ ] Highlight a line.
- [ ] Highlight a sentence.
- [ ] Highlight multiple words.
- [ ] Highlight multiple lines.
- [ ] Highlight multiple sentences.
- [ ] Traverse text by letter.
- [ ] Traverse text by word.
- [ ] Traverse text by line.
- [ ] Traverse text by sentence.
- [ ] Go to the beginning of a file.
- [ ] Go to the end of a file.
- [ ] Implement reliable arrow-key movement through the server-owned command path.
- [ ] Implement reliable Shift+arrow selection through the server-owned command path.
- [ ] Add Home/End and platform-specific variants.
- [ ] Add PageUp/PageDown behavior.
- [ ] Add mouse click, drag, double-click, and triple-click selection behavior.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

Milestone:

```text
Core text traversal, selection, and editability behavior is explicit, tested, and server-owned.
```

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

## Phase 27: Systemd Integration

Goal: allow persistent background server operation.

Acceptance criteria:

- Foreground server mode and user-service operation are documented.
- Socket path creation and cleanup work correctly under systemd-style runtime environments.
- The plan clearly decides whether a daemon flag is needed or systemd is sufficient.

Test plan:

- Add runtime-dir/socket-path tests simulating systemd-style environment variables.
- Add CLI/help or documentation tests for foreground server mode and user-service instructions where practical.
- Add a validation check documenting the daemon-vs-systemd decision.

Implementation tasks:

- [ ] Add documented foreground server mode.
- [ ] Add documented user service example.
- [ ] Decide whether `st server --daemon` is needed or systemd is enough.
- [ ] Ensure socket path and cleanup work under systemd.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 28: Adopt GPUI Component For Non-Editor UI

Goal: improve application chrome and supporting UI using `gpui-component` without replacing the custom server-backed editor surface.

Acceptance criteria:

- `gpui-component` is used only for non-editor UI surfaces.
- The custom server-backed editor surface remains independent of `gpui-component::InputState`.
- Application startup initializes component support only after core rendering boundaries are stable.
- Allowed component-backed UI surfaces are documented.

Test plan:

- Add boundary tests or compile-time structure checks proving editor input/state does not depend on gpui-component InputState.
- Add startup tests or smoke tests for component initialization where practical.
- Add documentation tests/checks for allowed gpui-component UI surfaces.

Explicit boundary: `gpui-component` is not used for the editor itself. The editor remains our custom GPUI native editor view, backed by server-owned editor state.

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

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

## Phase 29: Extension And Agent Runtime Architecture

Goal: use one JS runtime for normal editor programmability and isolated per-agent JS runtimes for long-running or blocking agent work. Implement the final runtime separation model directly so blocked agent code cannot freeze normal editor behavior.

Acceptance criteria:

- Normal editor extensions run in a lightweight editor runtime separate from agent runtimes.
- Each active agent has an isolated runtime worker and cannot block normal editor commands or other agents.
- All runtime communication uses typed request/response channels and immutable snapshots or typed requests.
- All agent-requested mutations are validated by Rust and routed through normal transactions/commands.

Test plan:

- Add request/response dispatch tests for the normal editor runtime worker.
- Add independent agent runtime tests proving blocked agents do not block editor JS or other agents.
- Add typed snapshot/request tests proving runtimes do not receive mutable editor state.
- Add validation tests proving invalid agent mutations are rejected and accepted edits route through normal transactions/commands.

Target runtime model:

```text
Server event loop
  owns canonical clients/sessions/buffers/editor state
  owns command registry and keymap
  validates all mutations

Editor extension runtime
  one deno_core::JsRuntime
  lightweight editor programmability only
  commands, keybindings, themes, modes, lightweight hooks
  must not run long-running agents

Agent runtime workers
  one worker per active agent
  one deno_core::JsRuntime per worker
  one V8 isolate/event loop per agent
  communicates with server through channels
  never directly mutates editor state
```

Target message flow for normal editor JS:

```text
Client input
  -> Server keymap/command dispatch
  -> Rust builtin command OR editor JS command
  -> JS command returns a typed request/result
  -> Server validates
  -> Server mutates editor/session/buffer state
  -> Server emits SceneUpdate
```

Target message flow for agents:

```text
User starts agent
  -> Server allocates AgentId
  -> Server creates AgentRuntimeWorker
  -> Worker starts its own deno_core::JsRuntime
  -> Server sends immutable context snapshots / typed tool requests
  -> Agent returns proposed edits/tool requests/logs/errors
  -> Server validates every requested mutation
  -> Server applies accepted commands
  -> Server emits SceneUpdate
```

Tokio/threading requirements:

```text
Central server event loop
  - must not execute JavaScript inline
  - must not block on agent completion
  - must communicate with runtimes through channels

Editor runtime worker
  - owns the single normal editor JsRuntime
  - processes lightweight extension requests sequentially
  - returns results to the server through channels

Agent runtime worker
  - owns exactly one agent JsRuntime
  - runs independently from the editor runtime and other agents
  - may be a Tokio task if compatible with JsRuntime ownership requirements
  - may be a dedicated OS thread if runtime !Send/!Sync constraints require it
  - communicates by mpsc/oneshot channels only
```

Implementation tasks:

- [ ] Define `AgentId`.
- [ ] Define `RuntimeRequestId` for correlating async runtime requests/responses.
- [ ] Define `EditorRuntimeRequest` and `EditorRuntimeResponse`.
- [ ] Define `AgentRuntimeRequest` and `AgentRuntimeResponse`.
- [ ] Define typed agent outputs: proposed edit, command request, tool request, log, error, completion.
- [ ] Define immutable context snapshot types for sessions/buffers/selections.
- [ ] Keep one normal editor `deno_core::JsRuntime` for lightweight extension operations.
- [ ] Restrict the normal editor runtime to commands, keybindings, themes, modes, and lightweight hooks.
- [ ] Explicitly reject or route long-running agent work away from the normal editor runtime.
- [ ] Add an `EditorRuntimeWorker` abstraction that owns the normal editor runtime.
- [ ] Start the editor runtime worker outside the central server event loop.
- [ ] Communicate with the editor runtime worker through request/response channels.
- [ ] Ensure the central server event loop does not call `JsRuntime::run_event_loop` directly.
- [ ] Ensure the central server event loop never waits synchronously for JS to finish.
- [ ] Add an `AgentRuntimeWorker` abstraction.
- [ ] Start one `AgentRuntimeWorker` per active agent.
- [ ] Start one `deno_core::JsRuntime` inside each agent worker.
- [ ] Ensure each agent runtime has its own V8 isolate and event loop.
- [ ] Store active agents in `HashMap<AgentId, AgentHandle>` on the server.
- [ ] `AgentHandle` should contain request sender, lifecycle state, and cancellation handle.
- [ ] Send only immutable snapshots or typed requests from server state into agent workers.
- [ ] Require agents to return typed proposed changes instead of mutating state directly.
- [ ] Validate all agent-requested edits/commands in Rust before applying them.
- [ ] Route accepted agent edits through the same transaction/undo command path as user edits.
- [ ] Never hold server/editor/session/buffer state while executing JavaScript.
- [ ] Never expose direct mutable Rust editor state references to JS.
- [ ] Ensure blocked normal editor JS cannot block socket IPC tasks.
- [ ] Ensure blocked normal editor JS cannot corrupt server state.
- [ ] Ensure blocked agent JS cannot block the normal editor runtime.
- [ ] Ensure blocked agent JS cannot block unrelated agent runtimes.
- [ ] Ensure blocked agent JS cannot block client IPC tasks.
- [ ] Add cancellation messages for running agents.
- [ ] Add timeout handling for agent requests and tool calls.
- [ ] Add runtime cleanup when an agent completes normally.
- [ ] Add runtime cleanup when an agent errors.
- [ ] Add runtime cleanup when an agent is cancelled.
- [ ] Add runtime cleanup when the server shuts down.
- [ ] Add structured logging for runtime start, request, response, error, cancellation, and shutdown.
- [ ] Add resource accounting hooks for CPU time, memory, tool usage, and token usage where practical.
- [ ] Add a policy hook for maximum concurrent agents.
- [ ] Add a policy hook for maximum runtime duration per agent.
- [ ] Add a policy hook for maximum pending requests per agent.
- [ ] Define the future migration path from per-agent runtime workers to per-agent OS processes for stronger isolation.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

Milestone:

```text
Normal editor extensions share one lightweight JS runtime.
Each active agent has an isolated JS runtime with its own V8 isolate/event loop.
The server event loop never executes JavaScript inline.
Blocked or slow agent code does not block editor commands, clients, or other agents.
All mutations still go through validated Rust server commands and normal edit transactions.
```

## Phase 30: Permissions And AI Layer

Goal: add advanced features safely after the editor core works.

Acceptance criteria:

- Extension permissions gate file, network, subprocess, AI, and tool access where applicable.
- AI/tool edits are routed through normal command/transaction paths.
- AI edits are previewable, reversible, and permission-checked.
- Permission and AI capabilities are discoverable through structured documentation.

Test plan:

- Add permission-policy tests for file, network, subprocess, AI, and tool access gates.
- Add AI/tool edit tests proving edits route through normal command/transaction paths.
- Add preview/reversal tests for AI edits.
- Add documentation metadata tests proving permission and AI capabilities are discoverable.

Implementation tasks:

- [ ] Define permission model for extensions.
- [ ] Gate file/network/subprocess access.
- [ ] Add AI command/tool API.
- [ ] Route AI edits through normal command/transaction system.
- [ ] Make AI edits previewable/reversible.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.
