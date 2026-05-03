# Client/Server Editing Model Discussion

## Context

The current editor sends ordinary typing through the server before text appears in the GPUI client. This makes visible typing latency depend on the full round trip:

```text
GPUI input
  -> client IPC thread
  -> Unix socket
  -> server key dispatch
  -> server mutation
  -> server ScenePatch
  -> Unix socket
  -> client IPC thread
  -> GPUI entity update
  -> cx.notify()
  -> GPUI prepaint/paint
```

This document records the discussed alternative model: make the GPUI client responsible for immediate primitive text editing, while the server follows via edit transactions and remains responsible for commands, persistence, extensions, semantic services, and asynchronous intelligence features.

This is intentionally not yet a final decision. There are architectural choices still to make.

---

# High-Level Direction

Do not route every keystroke through the server.

Instead:

```text
Client handles ordinary text insertion/deletion directly.
Server receives resulting edit transactions and follows asynchronously.
Server handles configured commands, modifier keys, mode-specific transforms, autocomplete, syntax, diagnostics, persistence, and extension-driven behavior.
Input routing is configurable and mode-aware.
```

This differs from local echo with reconciliation:

- **Local echo model:** server is still immediate authority; client speculates and later reconciles.
- **Client-native editing model:** the client is allowed to apply a defined class of primitive edits directly; the server follows via transactions.

The client-native model is potentially cleaner for editor responsiveness because simple text editing is not speculative in the same sense. It becomes a legitimate local edit path.

---

# What Should Still Be Done Regardless

The UI/render optimization work remains necessary no matter which client/server authority model is chosen.

Even if primitive edits move client-side, that only removes the server round trip from visible typing. It does not fix GPUI/client rendering costs such as:

- cloning visible text during `prepaint()`,
- splitting visible text during render,
- reshaping a whole long line per character,
- O(n) char/byte/UTF-16 conversions,
- simulated-only performance gates.

Therefore, the following `ui_lag.md` phases are still required:

```text
Phase 0: Real end-to-end profiling mode
Phase 2: Persistent viewport line model
Phase 3: Long-line chunking / horizontal viewport shaping
Phase 4: Metadata-based char/byte/UTF-16 conversions
Phase 5: GPUI render-path cleanup
Phase 6: Real UI-lag regression gate
```

Phase 0 should be activated only in an explicit profiling mode, not during normal editor usage. Possible forms:

```bash
cargo run -- ui-profiler
cargo run -- client --profile-ui
ST_UI_PROFILE=1 cargo run -- client
```

Normal client/server runs should not pay detailed profiling overhead.

---

# Proposed Runtime Model

## Client-Native Editing Path

The client handles directly:

- unmodified printable text,
- backspace,
- delete,
- enter,
- basic cursor movement,
- basic selection movement,
- possibly tab, depending on mode/config policy.

For ordinary typing:

```text
GPUI input
  -> client mutates local buffer/model immediately
  -> client marks affected line/chunk dirty
  -> client calls cx.notify()
  -> GPUI prepaint updates only affected visible chunk(s)
  -> GPUI paint shows text immediately
  -> client sends TextEditTransaction to server asynchronously
  -> server applies same transaction
  -> server later sends syntax/highlighting/autocomplete/diagnostics/decorations as versioned async updates
```

The server receives higher-level transactions instead of raw key events for normal text edits:

```text
TextEditTransaction
SelectionTransaction
ViewportUpdate
```

## Server-Routed Path

The server handles:

- modifier chords,
- prefix chords,
- command palette commands,
- save/open/file operations,
- extension commands,
- mode-specific intercepted keys,
- autocomplete requests,
- formatting,
- LSP actions,
- LaTeX transformations/render commands,
- commands that require server-owned capabilities.

For routed input:

```text
GPUI key
  -> client consults compiled routing table/keymap/mode policy
  -> if routed, client sends CommandInvocation/KeyInput to server
  -> server executes command
  -> server sends result/edit/overlay back
```

---

# Configurable Input Routing

Input routing should be configurable and mode-aware, not hard-coded.

The server/config runtime should remain the source of truth for user configuration. The client should receive a compiled routing table suitable for fast Rust-side decisions during input handling.

Conceptual configuration shape:

```ts
inputRouting.configure({
  printableDefault: "client",
  backspaceDefault: "client",
  enterDefault: "client",
  tabDefault: "mode",
  modifiedKeysDefault: "server",
});

inputRouting.intercept({
  mode: "latex",
  keys: ["\\", "$"],
  command: "latex.handle-special-input",
});
```

Conceptual compiled client routing table:

```rust
InputRoutingTable {
    direct_text_default: true,
    server_routed_chords: Vec<KeyChord>,
    client_native_commands: Vec<KeyChord>,
    mode_overrides: Vec<ModeRoutingOverride>,
}
```

Important rule:

```text
By default, unmodified printable text should be direct client text input.
If a mode/config wants to intercept printable text, it must explicitly register that interception.
```

This preserves fast normal typing while allowing special modes such as LaTeX to intercept selected characters or sequences.

---

# Transaction Model

Both client-native edits and server-side commands should converge on a shared transaction representation.

Conceptual shape:

```rust
TextEditTransaction {
    transaction_id,
    buffer_id,
    base_version,
    new_version,
    source_client_id,
    edits: Vec<TextEdit>,
    selection_before,
    selection_after,
    undo_group,
}
```

The server should apply client transactions and use the same transaction model for server-generated edits.

This is important for:

- undo/redo consistency,
- multi-client consistency,
- save/format/LSP correctness,
- extension hooks,
- stale-result rejection,
- deterministic replay/debugging.

---

# Possible Authority Models

The main unresolved question is: who owns canonical buffer state?

## Option A: Focused Client Owns Live Buffer; Server Follows

The focused client applies primitive edits immediately and is the live authority for those edits. The server follows by applying transaction messages.

### Pros

- Best typing latency.
- Simple visible editing path.
- Avoids speculative local echo semantics for primitive edits.

### Cons

- Server is eventually consistent, not immediately canonical.
- Save/server commands must account for pending client transactions.
- Multi-client editing needs a clear conflict policy.

## Option B: Server Remains Canonical; Client Speculates

The client locally displays edits before confirmation, but the server remains the immediate authority and can accept/reject/reorder edits.

### Pros

- Preserves strongest server-canonical architecture.
- Multi-client/server command ordering is clearer.

### Cons

- This is local echo/reconciliation, which we do not currently prefer.
- Requires pending-edit queues, double-apply prevention, rejection handling, and more complex IME/undo behavior.

## Option C: Single-Writer Lease / Edit Leadership

A middle-ground model. For each buffer/session, one client has edit leadership/write lease. That client may apply primitive edits locally. The server orders and records transactions, and other clients follow server-broadcast updates.

```text
one focused client has edit leadership for a buffer
that client may apply primitive edits locally
server receives and orders transactions
other clients receive server-broadcast transactions
leadership can transfer between clients
```

### Pros

- Keeps local typing fast.
- Avoids unconstrained multi-writer conflicts.
- Gives the server a clear ordering role.
- Easier than full collaborative editing/CRDT/OT.

### Cons

- Requires lease/leadership protocol.
- Requires transfer/revocation behavior.
- Server may still be briefly behind the leader client.

This may be the best initial model if multi-client support matters.

---

# Potential Issues And Design Risks

## 1. Multi-Client Editing

If two clients edit the same buffer, direct client editing creates conflicts unless there is a policy.

Simplest initial rule:

```text
Only one client has write leadership for a buffer at a time.
Other clients are read-only/following unless leadership transfers.
```

Full collaborative editing should be treated as a future feature, not the initial solution.

## 2. Keymap Knowledge In The Client

The client needs enough keymap/routing knowledge to decide whether input is direct or server-routed.

The client should not run TypeScript config on every keypress. Instead:

```text
TypeScript/Rust config layer builds authoritative keymap/routing state
server sends compiled routing table to client
client performs fast routing decisions locally
```

## 3. Printable Keys Can Be Commands

Emacs-like editors can bind printable keys to commands. If all printable keys can be intercepted by default, typing may become slow or ambiguous.

Recommended default:

```text
unmodified printable text -> direct client edit
explicit mode/config intercepts -> server-routed
modified/prefix chords -> server-routed by default
```

## 4. Prefix Keymaps

Prefix keys such as `ctrl+x` need client-side routing awareness.

Recommended:

- modifier/prefix chords are routable by default,
- printable-prefix interception requires explicit mode/config opt-in.

## 5. Undo/Redo

If primitive edits are client-native, the undo stack must be transaction-based and consistent with server-side mutations.

Both client edits and server commands should produce the same transaction shape.

Undo grouping should be represented explicitly in transactions.

## 6. Server-Side Features Lag Behind Text

This is acceptable for many features:

- syntax highlighting,
- autocomplete,
- diagnostics,
- semantic overlays,
- LaTeX rendering/transformation previews.

But all async results must be versioned:

```text
result includes buffer_id + buffer_version/request metadata
client applies only if still current
stale results are discarded
```

## 7. Save And Commands Requiring Fresh Text

If the server follows asynchronously, server-side commands that require current text must flush or wait for pending client transactions first.

Examples:

- save,
- format,
- compile,
- LSP request,
- extension command that reads buffer text,
- search/index operations that require current contents.

Possible rule:

```text
Before commands requiring fresh buffer state, client flushes pending transactions and server applies through latest known version, then command executes.
```

Commands may need metadata such as:

```text
requires_fresh_buffer: true
```

## 8. Extension Hooks Must Not Become Synchronous Per Character

Extensions should not be able to accidentally place JavaScript or slow server work back into the normal typing path.

Recommended:

```text
default: extensions observe async versioned edit stream
optional: modes/config may register explicit input interceptors
interceptors are opt-in and documented as latency-affecting
```

---

# LaTeX Example

For a LaTeX mode:

```text
normal LaTeX text typing -> client-native direct edit
syntax highlighting -> async server result
autocomplete -> async server request/result
math rendering/preview -> async server decoration/render update
special input transformations -> explicit server-routed interceptors or async commands
```

Example behavior:

- Typing ordinary text is immediate.
- Highlighting may lag slightly.
- Completion may lag slightly.
- Rendering `\alpha` or math regions into visual formula previews may lag slightly.
- If a mode wants `$`, `\`, or another key to trigger special behavior, it must explicitly configure that routing.

---

# Recommended Initial Direction

The chosen direction from this discussion:

```text
Client is authoritative for immediate primitive editing in the focused session.
Server acts as a processor/follower for primitive edits.
Server remains responsible for command execution, persistence, indexing, extensions, semantic features, and transaction ordering/processing.
Input routing is configurable and mode-aware.
All async server results are versioned.
```

`st` does not intend to support multiple clients simultaneously editing the same file. If multiple clients view the same file, the intended model is single-writer/edit-leadership:

```text
only one client can edit a file at a time
that client is in Normal mode for that file
other clients are in Read mode for that file
Read-mode clients receive incremental updates from the Normal-mode client/server stream
```

---

# Closed Decisions And Remaining Design Details

## 1. Canonical Authority

Decision:

```text
The server does not need to remain strictly canonical for primitive edits.
The focused Normal-mode client can own immediate edits.
The server acts as a processor/follower for those edits.
```

Implication:

- Ordinary text editing should not wait for server acknowledgement before appearing.
- The server receives edit transactions after the client applies them.
- Server-side features process those transactions and produce async results.

## 2. Single Writer / Edit Leadership

Decision:

```text
Use single-writer/edit-leadership per source file.
```

`st` does not support simultaneous multi-client editing of one file. If a file is open in multiple clients, only one client may be in the editing mode for that file.

## 3. Normal Mode And Read Mode

Decision:

Introduce two basic editor modes with ownership implications:

```text
Normal mode
  client can edit the file
  client owns primitive edit path
  client sends edit transactions to server

Read mode
  client cannot directly edit the file
  input is routed through the server / command layer
  client receives incremental updates produced by the Normal-mode owner
```

Rules:

- Multiple clients cannot have Normal mode enabled for the same source file at the same time.
- If one client has Normal mode for a file, other clients opening that file automatically enter Read mode.
- A new client opening a file already owned by another Normal-mode client opens in Read mode.
- The user may request a mode change, but Normal-mode acquisition must respect single-writer ownership.
- Mode state has major implications for keybinding behavior and should be part of the input routing model.

Open implementation detail:

- Exact UX/protocol for requesting Normal mode from Read mode still needs design.
- Ownership transfer can initially be explicit and conservative.

## 4. Default Client-Native Primitive Edits

Decision:

By default, common typing/editing/navigation inputs are client-native in Normal mode and should not be passed through the server as raw key events.

Default client-native categories include:

- QWERTY alphabet keys `A` through `Z` / `a` through `z`,
- shift-modified alphabet keys for uppercase text,
- number keys `0` through `9`, including numpad numbers,
- ordinary punctuation and typing characters such as `+`, `-`, `:`, `'`, `"`, `,`, `.`, `/`, `\\`, `;`, `[`, `]`, `(`, `)`, etc.,
- enter,
- tab,
- backspace,
- delete,
- arrow keys up/down/left/right,
- basic mouse events used for cursor placement and selection.

Modifier combinations are treated differently:

- `shift` used only to type an uppercase or shifted character remains part of client-native text input.
- combinations involving `ctrl`, `alt`, `meta`, or multi-modifier chords such as `ctrl+shift` are routed according to the configured keymap/routing table.

Important configuration rule:

```text
This default behavior must not be hard-coded.
It must be expressed as default configuration that users and modes can override.
```

Reason:

- Users may want modal editing extensions.
- Users may want printable keys to route through server commands in specific modes.
- Read mode should route differently from Normal mode.
- Future modes can define their own key routing policy.

## 5. Tab Default

Decision:

```text
Tab is client-native by default in Normal mode.
```

But it must remain configurable and mode-overridable. For example, a language mode may route Tab through indentation/completion logic if explicitly configured.

## 6. Printable Key Interceptors And Key Categories In Config

Decision:

Printable key interception is represented through configurable per-mode input routing. The default config should enumerate key categories and allow individual keys or categories to be rerouted.

Conceptual key categories:

```text
text.alpha
text.number
text.numpad_number
text.punctuation
text.whitespace
edit.backspace
edit.delete
edit.enter
edit.tab
navigation.arrow
mouse.selection
mouse.cursor
modifier.ctrl
modifier.alt
modifier.meta
modifier.shift_chord
chord.prefix
mode.interceptor
```

Conceptual config shape:

```ts
inputRouting.defineMode("normal", {
  defaultRoute: "server",
  routes: [
    { category: "text.alpha", route: "client" },
    { category: "text.number", route: "client" },
    { category: "text.numpad_number", route: "client" },
    { category: "text.punctuation", route: "client" },
    { category: "text.whitespace", route: "client" },
    { category: "edit.backspace", route: "client" },
    { category: "edit.delete", route: "client" },
    { category: "edit.enter", route: "client" },
    { category: "edit.tab", route: "client" },
    { category: "navigation.arrow", route: "client" },
    { category: "mouse.cursor", route: "client" },
    { category: "mouse.selection", route: "client" },
    { modifiers: ["ctrl"], route: "server" },
    { modifiers: ["alt"], route: "server" },
    { modifiers: ["meta"], route: "server" },
  ],
});

inputRouting.defineMode("read", {
  defaultRoute: "server",
});

inputRouting.defineMode("latex", {
  extends: "normal",
  routes: [
    { key: "\\\\", route: "server", command: "latex.handle-backslash" },
    { key: "$", route: "server", command: "latex.handle-dollar" },
  ],
});
```

Performance warning requirement:

- Routing printable/common typing keys through the server can degrade typing latency.
- Config documentation should explicitly warn about this.

## 7. Compiled Routing Table Sent To Clients

Decision:

The server/config runtime remains the source of truth. The client receives a compiled routing table optimized for fast key decisions.

Conceptual compiled format:

```rust
enum InputRoute {
    ClientEdit(ClientEditKind),
    ServerKey,
    ServerCommand(CommandId),
    Ignore,
}

struct CompiledInputRoutingTable {
    generation: u64,
    active_mode: ModeId,
    default_route: InputRoute,
    key_routes: HashMap<NormalizedKey, InputRoute>,
    category_routes: Vec<(KeyCategory, InputRoute)>,
    modifier_routes: Vec<(ModifierPattern, InputRoute)>,
    prefix_routes: PrefixTrie<InputRoute>,
}
```

Key requirements:

- The client must not run TypeScript on the hot path.
- The client must be able to decide direct-client vs server-routed input synchronously and cheaply.
- Routing tables are versioned/generation-tagged.
- Mode changes send a new compiled routing table or activate an existing compiled table.

## 8. Hot Reload Of Config And Compiled Routing

Decision:

```text
The server/config runtime remains authoritative for configuration.
The client holds only a versioned compiled routing/config snapshot.
Config hot reload rebuilds server-side registries, recompiles client-safe routing tables, and sends an atomic snapshot update to clients.
```

Desired flow:

```text
init.ts / config file changes
  -> server detects change or receives manual reload command
  -> server debounces reload events
  -> server reloads/evaluates config
  -> server validates config
  -> server rebuilds typed registries/keymaps/mode routing tables
  -> server compiles client-safe routing snapshot
  -> server publishes new generation
  -> server sends config/routing snapshot to clients
  -> clients atomically swap routing table
```

The client must never interpret TypeScript or run config logic while handling input. The hot input path uses only the current compiled snapshot.

Conceptual compiled snapshot:

```rust
struct CompiledClientConfig {
    generation: u64,
    modes: Vec<CompiledModeRouting>,
    active_mode_by_buffer: HashMap<BufferId, ModeId>,
    keymap: CompiledKeymap,
    input_routing: CompiledInputRoutingTable,
}
```

Client state:

```rust
struct ClientInputState {
    current_config_generation: u64,
    compiled_config: Arc<CompiledClientConfig>,
}
```

Protocol concepts:

```rust
enum ServerToClient {
    ClientConfigSnapshot {
        generation: u64,
        config: CompiledClientConfig,
    },
    ConfigReloaded {
        generation: u64,
    },
    ConfigReloadFailed {
        attempted_generation: u64,
        errors: Vec<ConfigDiagnostic>,
    },
    ActiveModeChanged {
        generation: u64,
        buffer_id: BufferId,
        mode_id: ModeId,
    },
}

enum ClientToServer {
    RequestConfigReload,
    SetMode {
        buffer_id: BufferId,
        requested_mode: ModeId,
    },
}
```

Initial implementation should prefer full snapshots rather than diffs:

```text
ClientConfigSnapshot replaces the whole client-side compiled config atomically.
```

Diffs can be introduced later if routing/config snapshots become large.

### Last-Known-Good Config

Failed reloads must not break editing.

Rule:

```text
If config reload fails, keep the last known-good compiled config active.
```

The server should report diagnostics to the client:

```text
ConfigReloadFailed { attempted_generation, errors }
```

The user can keep editing using the previous working config while fixing `init.ts` or related config files.

### Atomic Client Swap

The client should never partially apply keymap/mode/routing updates.

Bad:

```text
mode table updated
keymap table not updated
routing table stale
```

Good:

```text
one compiled snapshot contains all related input state
client swaps Arc<CompiledClientConfig> in one operation
```

### Config Generation Rules

- Every compiled config snapshot has a monotonically increasing `generation`.
- Client ignores snapshots older than its current generation.
- Input handling reads the current generation/snapshot cheaply.
- After a new generation is installed, the next key uses the new routing behavior.
- Already-applied local edits are not retroactively reinterpreted.

### Prefix Chord Edge Case

If config changes while a key chord/prefix sequence is in progress, the simplest initial rule is:

```text
cancel the in-progress prefix sequence when config generation changes
```

The UI may show a small status message:

```text
Key sequence canceled because keymap reloaded.
```

This avoids mixing keymap generations inside a single chord.

### Active Mode Removed By Reload

If a config reload removes the currently active mode, the client/server should fall back to a valid mode:

```text
Normal mode if this client owns edit leadership for the file.
Read mode if another client owns edit leadership or if Normal mode is unavailable.
```

### Performance Warning

Config may route broad printable categories through the server, but documentation and runtime diagnostics should warn that this can degrade typing latency.

Possible warning condition:

```text
mode routes text.alpha/text.number/text.punctuation/text.whitespace to server
```

### Hot Reload TODO

- [ ] Define typed Rust descriptors for config, keymap, modes, and input routing.
- [ ] Define `CompiledClientConfig` and `CompiledInputRoutingTable`.
- [ ] Send initial `ClientConfigSnapshot` during client connection.
- [ ] Make client input routing use only the compiled snapshot.
- [ ] Add manual config reload command/request.
- [ ] Add config file watcher with debounce.
- [ ] Rebuild config into a new generation after successful reload.
- [ ] Keep last-known-good compiled config on reload failure.
- [ ] Send `ConfigReloadFailed` diagnostics without changing active client config.
- [ ] Atomically swap client config snapshots.
- [ ] Cancel in-progress prefix chords when config generation changes.
- [ ] Define fallback behavior if active mode disappears.
- [ ] Add performance warnings for server-routed broad printable categories.
- [ ] Add tests: reload changes route for a printable key.
- [ ] Add tests: failed reload preserves previous route.
- [ ] Add tests: generation swap is atomic.
- [ ] Add tests: new client receives latest generation.
- [ ] Add tests: prefix chord is canceled or handled safely across generation change.

## 9. Text Mutation Transaction Format Options

A final transaction format still needs to be selected. The format should optimize for correctness, compactness, and fast application.

### Option A: Absolute byte-range transaction

```rust
struct TextEditTransaction {
    transaction_id: u64,
    buffer_id: u64,
    base_version: u64,
    new_version: u64,
    source_client_id: ClientId,
    edits: Vec<ByteRangeEdit>,
    selection_before: Selection,
    selection_after: Selection,
    undo_group: UndoGroupId,
}

struct ByteRangeEdit {
    start_byte: u64,
    end_byte: u64,
    replacement: String,
}
```

Pros:

- Fast for Rust string/rope mutation when byte offsets are already known.
- Compact.
- Avoids repeated char-to-byte conversion on the server.

Cons:

- Requires strict UTF-8 boundary guarantees.
- Less convenient for protocol/debugging than line/column or char offsets.
- Offset validity depends on exact base version.

### Option B: Absolute char-range transaction

```rust
struct TextEditTransaction {
    transaction_id: u64,
    buffer_id: u64,
    base_version: u64,
    new_version: u64,
    source_client_id: ClientId,
    edits: Vec<CharRangeEdit>,
    selection_before: Selection,
    selection_after: Selection,
    undo_group: UndoGroupId,
}

struct CharRangeEdit {
    start_char: u64,
    end_char: u64,
    replacement: String,
}
```

Pros:

- Aligns with current server/editor char-index model.
- Easier to reason about Unicode scalar positions than bytes.
- Good fit for rope APIs that operate on char indices.

Cons:

- Client/server may need char/byte conversion metadata.
- Long-line char conversion must be optimized by Phase 4.
- UTF-16 GPUI offsets still need separate conversion.

### Option C: Line/column transaction

```rust
struct TextEditTransaction {
    transaction_id: u64,
    buffer_id: u64,
    base_version: u64,
    new_version: u64,
    source_client_id: ClientId,
    edits: Vec<LineColumnEdit>,
    selection_before: Selection,
    selection_after: Selection,
    undo_group: UndoGroupId,
}

struct LineColumnEdit {
    start: LineColumn,
    end: LineColumn,
    replacement: String,
}
```

Pros:

- Human-readable and debug-friendly.
- Good for diagnostics/logging.
- Natural for editor UI concepts.

Cons:

- More expensive to apply unless line metadata is excellent.
- Ambiguous around tabs/columns unless columns are clearly byte/char/UTF-16 based.
- Less compact.

### Option D: Hybrid transaction with primary byte range plus debug positions

```rust
struct TextEditTransaction {
    transaction_id: u64,
    buffer_id: u64,
    base_version: u64,
    new_version: u64,
    source_client_id: ClientId,
    edits: Vec<TextEdit>,
    selection_before: Selection,
    selection_after: Selection,
    undo_group: UndoGroupId,
}

struct TextEdit {
    start_byte: u64,
    end_byte: u64,
    start_char: Option<u64>,
    end_char: Option<u64>,
    start_line_col: Option<LineColumn>,
    end_line_col: Option<LineColumn>,
    replacement: String,
}
```

Pros:

- Fast primary application via byte range.
- Optional char/line positions improve debugging and validation.
- Can validate client metadata during development/profiling.

Cons:

- Larger payload.
- More fields and invariants.
- Risk of inconsistent metadata if not carefully validated.

Recommendation to choose from:

```text
Option B if we want consistency with current rope/editor char-index design.
Option D if we want maximum performance with validation/debug metadata.
```

## 10. Undo/Redo Ownership Options

Final ownership still needs to be selected.

### Option A: Client-owned undo/redo

Pros:

- Fastest interactive undo/redo.
- Matches client-owned primitive editing.
- UI can group typing edits without server round trips.

Cons:

- Server commands that mutate text must be inserted into client undo history.
- Read-mode/follower clients need server-provided history or no undo.
- Persistence/debugging of undo history is harder.

### Option B: Server-owned undo/redo

Pros:

- One authoritative undo history.
- Server commands and extension edits are naturally included.
- Easier to serialize, inspect, and expose to extensions.

Cons:

- Undo/redo becomes server-routed and may have latency.
- Conflicts with client-owned immediate primitive edit philosophy.
- Client must wait for server to apply undo result.

### Option C: Shared transaction log, client executes local undo for owned transactions

Pros:

- One transaction log format for client and server.
- Client can undo local typing immediately in Normal mode.
- Server can process/record the same undo transaction.
- Server-generated edits can be inserted into the shared log.

Cons:

- Most complex design.
- Requires careful undo grouping and source tracking.
- Requires clear rules when server-generated edits interleave with client edits.

Recommendation to choose from:

```text
Option C is architecturally strongest.
Option A is simplest/fastest initially.
Option B is simplest for server consistency but worst for typing responsiveness.
```

## 11. Save Semantics

Decision:

```text
Save is requested by the client and executed by the server.
When save is requested, the server waits until all transactions up to that save request have been received and applied, then writes the file.
```

Conceptual sequence:

```text
client applies edits locally
client sends tx 101
client sends tx 102
client sends SaveRequest(after_transaction=102)
server applies tx 101
server applies tx 102
server executes save
server sends SaveResult
```

Requirements:

- Save request includes the latest client transaction id/version known to the client.
- Server must not save an older version if pending transactions from that client precede the save request.
- Save result should include saved buffer version / transaction id.

## 12. Commands Requiring Fresh Server Buffer State

Clarification:

This question was asked because, if the server follows asynchronously, a server-side command may otherwise run against stale text.

Examples that could require fresh server buffer state:

- save,
- format document/range,
- LSP go-to-definition/completion/rename/code action if based on current buffer text,
- compile/build current buffer,
- run tests for current buffer,
- extension command that reads current buffer content,
- search/index actions that must include latest edits.

Decision for now:

```text
Commands that need current buffer content should declare that they require fresh buffer state.
Before executing such a command, the client/server must flush/apply transactions through the request's specified transaction id/version.
```

Even if the initial set is small, the metadata is useful because many future commands/extensions will read buffer contents.

Conceptual command metadata:

```rust
struct CommandDescriptor {
    id: CommandId,
    title: String,
    requires_fresh_buffer: bool,
}
```

## 13. Async Semantic Result Versioning Options

A final stale-result strategy still needs to be selected.

### Option A: Buffer version tagging

Every async result includes:

```rust
buffer_id
buffer_version
```

Client applies only if the current buffer version matches.

Pros:

- Simple.
- Good for syntax highlighting, diagnostics, decorations.
- Easy to test.

Cons:

- Results are discarded after any later edit, even if the result still partially applies.
- Can waste work during rapid typing.

### Option B: Transaction range/version tagging

Every async result includes:

```rust
buffer_id
base_version
observed_transaction_range
affected_ranges
```

Client can apply if affected ranges are still valid or can be mapped forward.

Pros:

- More results can survive unrelated edits.
- Better for large files and expensive analyses.

Cons:

- Requires range mapping through transactions.
- More complex correctness model.

### Option C: Request id + buffer version + cancellation

Every async request has:

```rust
request_id
buffer_id
buffer_version
kind
```

Newer requests cancel older requests of the same kind/scope. Client applies only latest matching result.

Pros:

- Practical for autocomplete, syntax passes, diagnostics, LaTeX previews.
- Avoids applying stale work.
- Bounded background waste if cancellation is real.

Cons:

- Coarser than range mapping.
- Some useful partial results may be discarded.

### Option D: Hybrid

Use:

```text
request_id + buffer_version + cancellation by default
range mapping only for specific expensive features that need it
```

Pros:

- Simple default.
- Allows sophistication where justified.
- Good performance/correctness tradeoff.

Cons:

- Two systems to maintain if range mapping is added later.

Recommendation to choose from:

```text
Option C for initial implementation.
Option D as the long-term direction.
```

## 14. Tests And Benchmarks To Prove Improved Typing Latency

Required first tests/benchmarks:

- [ ] Unit test: Normal-mode printable key routes to `ClientEdit`, not `ServerKey`.
- [ ] Unit test: Read-mode printable key routes to `ServerKey`.
- [ ] Unit test: Ctrl/Alt/Meta chord routes according to config.
- [ ] Unit test: Tab routes to client by default in Normal mode.
- [ ] Unit test: Mode override can route a printable key to a server command.
- [ ] Unit test: Only one client can hold Normal mode/edit leadership for a file.
- [ ] Unit test: second client opening an owned file enters Read mode.
- [ ] Unit test: Normal-mode client emits `TextEditTransaction` after local edit.
- [ ] Unit test: SaveRequest includes latest transaction id/version.
- [ ] Integration test: typing a character updates client model before any server response is processed.
- [ ] Integration test: Read-mode client receives incremental updates from Normal-mode edits.
- [ ] Integration test: save waits for transactions through requested transaction id.
- [ ] UI profiler benchmark: key-to-visible-client-update latency for client-native typing.
- [ ] UI profiler benchmark: key-to-paint latency for 100, 1k, 10k, and 100k character first-line typing.
- [ ] UI profiler benchmark: verify no server round trip is on the Normal-mode primitive typing path.
- [ ] UI profiler benchmark: shaped bytes per key are bounded after long-line shaping work.
- [ ] Regression gate: fail if Normal-mode printable typing routes through server by default.
- [ ] Regression gate: fail if key-to-paint p95 exceeds agreed threshold in client-native path.
