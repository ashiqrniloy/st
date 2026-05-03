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

This document records the chosen high-level model: make the GPUI client responsible for immediate primitive text editing, while the server follows via edit transactions and remains responsible for commands, persistence, extensions, semantic services, and asynchronous intelligence features.

Most core architectural decisions are now locked in this document. Remaining concerns are implementation details and policy choices called out in the review section near the end.

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

# Chosen Authority Model

The chosen authority model is:

```text
Focused Normal-mode client owns primitive edits for the file.
Server follows by processing edit transactions.
Server does not need to be strictly canonical for immediate text edits.
Only one client can have edit ownership for a file at a time.
Other clients viewing the same file are Read-mode followers.
```

This intentionally rejects the local-echo/reconciliation model where every primitive edit remains server-authoritative and the client merely speculates. Instead, primitive editing is a legitimate client-native path in Normal mode.

## Normal Mode

Normal mode means the client has edit leadership for a file.

```text
Normal-mode client
  applies primitive edits locally
  renders immediately
  emits text/selection transactions to server
  receives async semantic/decorative/server-command results
```

Only one client may be in Normal mode for a given source file.

## Read Mode

Read mode means the client does not own edit leadership for that file.

```text
Read-mode client
  does not apply primitive edits directly
  routes input through server/config command path
  receives incremental updates from the Normal-mode owner/server stream
```

Read mode is also the default for additional clients that open a file already owned by another Normal-mode client.

## Edit Leadership / Single Writer

`st` does not intend to support collaborative multi-client simultaneous editing of one file. Therefore, the single-writer model is not a temporary compromise; it is the intended ownership model.

Rules:

- A file may have zero or one Normal-mode owner.
- A file may have any number of Read-mode viewers/followers.
- If no client owns Normal mode for a file, a client may request Normal mode.
- If a client owns Normal mode for a file, new clients open that file in Read mode.
- Ownership transfer should be explicit and conservative.
- Read-mode clients receive incremental updates and server-side results but do not locally mutate file text.

## Why This Model

Benefits:

- Best typing latency for the editing client.
- Avoids speculative local echo complexity.
- Avoids CRDT/OT/collaborative editing complexity.
- Preserves a strong server role as processor, persistence coordinator, semantic service host, extension host, and transaction receiver.
- Makes user-visible mode state meaningful and useful for future modal editing/keybinding design.

Tradeoffs:

- Server may briefly lag behind the Normal-mode client.
- Save and server-side commands must account for transaction ordering.
- Read-mode behavior and ownership transfer need explicit UX/protocol design.
- Undo/redo grouping/interleaving policies still need detailed design.

---

# Potential Issues And Design Risks

## 1. Edit Leadership And Read-Mode UX

The main multi-client concern is not collaborative editing; it is clear single-writer ownership.

Risk:

```text
A user may be confused why one client can edit while another is read-only.
```

Required design:

- Show whether the current buffer/client is in Normal mode or Read mode.
- Show which client/session owns Normal mode when a file is read-only because another client owns it.
- Provide a command to request Normal mode.
- Define explicit ownership transfer behavior.
- Ensure a Read-mode client never applies primitive edits locally.

## 2. Keymap And Routing Knowledge In The Client

The client needs enough routing knowledge to decide whether input is client-native or server-routed.

Risk:

```text
If the client runs dynamic config/TypeScript on the input hot path, typing performance regresses.
```

Required design:

```text
TypeScript/Rust config layer builds authoritative keymap/routing state
server compiles a client-safe routing snapshot
client performs fast routing decisions locally from that snapshot
```

The client must not execute TypeScript or perform slow dynamic lookup during input handling.

## 3. Hot Reload Of Keymaps And Routing

Config should be hot-reloadable, but hot reload must not compromise input performance or leave the client in a partially updated state.

Required design:

- Server watches/reloads config or handles manual reload.
- Server validates config and builds a new generation.
- Server sends a full compiled config/routing snapshot to clients.
- Client atomically swaps the snapshot.
- Failed reload keeps the last-known-good config active.
- In-progress prefix chords are canceled or safely handled across generation changes.

## 4. Printable Keys Can Be Commands

The default should optimize for normal typing:

```text
Normal mode: common typing/edit/navigation keys -> client-native
Read mode: input routes through server/config command path
Mode overrides: explicit printable interceptors may route to server
```

Risk:

```text
If broad printable categories route through the server, typing latency can return.
```

Required design:

- Default key routing is represented in configuration, not hard-coded.
- Users/modes can override individual keys or categories.
- Documentation warns that routing printable/common typing keys through the server can degrade performance.
- Runtime diagnostics may warn when a mode routes broad printable categories through the server.

## 5. Prefix Keymaps

Prefix keys such as `ctrl+x` require client-side routing awareness.

Required design:

- Compiled routing table includes prefix information.
- Modifier/prefix chords route according to config.
- Printable prefix interception is explicit and mode/config controlled.
- If config generation changes mid-prefix, initial behavior should cancel the prefix sequence.

## 6. Undo/Redo

Undo/redo remains an important unresolved design choice.

Risk:

```text
Client-owned primitive edits and server-generated edits can produce inconsistent undo behavior unless both use a shared transaction model.
```

Required design:

- All text mutations use a common transaction format.
- Undo grouping is explicit.
- Server-generated text edits can enter the same history model as client-native edits.
- Final decision needed: client-owned undo, server-owned undo, or shared transaction log.

## 7. Server-Side Features Lag Behind Text

This is acceptable and expected for many features:

- syntax highlighting,
- autocomplete,
- diagnostics,
- semantic overlays,
- LaTeX rendering/transformation previews.

Risk:

```text
Async server results may refer to stale text.
```

Required design:

```text
result includes request id and/or buffer version/transaction metadata
client applies only if still current
stale results are discarded
```

The selected strategy is request id + buffer version + cancellation/supersession with complete snapshots per scope.

## 8. Save And Commands Requiring Fresh Text

Because the server follows asynchronously, save and some server-side commands must not run against stale text.

Save decision:

```text
Save is requested by the client.
Save request includes latest client transaction id/version.
Server applies all transactions through that id/version before writing file.
```

Other commands may also require fresh server buffer state, especially commands that read current buffer text.

Examples:

- format,
- LSP request,
- compile/build current buffer,
- extension command that reads current buffer content,
- search/index operation requiring latest edits.

Required design:

- Commands that require current text should declare `requires_fresh_buffer`.
- Before executing such commands, server must apply transactions through the request's specified transaction id/version.

## 9. Extension Hooks Must Not Become Synchronous Per Character

Extensions should not be able to accidentally put JavaScript or slow server work back into the normal typing path.

Required design:

```text
default: extensions observe async versioned edit stream
optional: modes/config may register explicit input interceptors
interceptors are opt-in and documented as latency-affecting
```

## 10. Transaction Ordering And Backpressure

The server follows Normal-mode client edits by receiving transactions.

Risks:

- transaction messages could lag behind rapid local editing,
- save could arrive before earlier edit transactions are applied,
- a slow server could accumulate stale work.

Required design:

- Transactions are monotonically numbered per client/file.
- Save and fresh-buffer commands include the required transaction id/version.
- Server applies transactions in order.
- Server detects missing transactions and requests resync if necessary.
- Queues remain bounded/monitored.

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

## 9. Text Mutation Transaction Format

Decision:

```text
Text mutation transactions are char-range authoritative everywhere.
Do not include byte hints or line/column hints in the core transaction format.
Keep the transaction payload light and canonical.
```

Rationale:

- The server/editor model naturally uses char indices.
- A single canonical coordinate system keeps the protocol simpler.
- Byte-range transactions introduce UTF-8 boundary hazards.
- Server-to-client byte conversion would be unnecessary if the client already maintains char/byte metadata.
- The client must maintain efficient char/byte/UTF-16 metadata anyway for GPUI input, cursor placement, selection, and rendering.
- Therefore, optimize client metadata instead of adding extra coordinate hints to the protocol.

Canonical transaction shape:

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
    start_char: u64,
    end_char: u64,
    replacement: String,
}
```

Rules:

- `start_char..end_char` is authoritative for every text mutation.
- Client-to-server primitive edit transactions use this format.
- Server-to-client follower updates use this format.
- Server-generated text edits use this format.
- Undo/redo transactions use this format.
- Extension-observed text edits use this format unless a future low-level API explicitly requires another representation.

Client performance requirement:

```text
The client must maintain line/chunk metadata so char-range application does not require whole-content scanning.
```

Required client metadata concept:

```rust
struct VisibleLine {
    start_char: u64,
    start_byte: u64,
    start_utf16: u64,
    char_len: u32,
    byte_len: u32,
    utf16_len: u32,
    chunks: Vec<VisibleChunk>,
}

struct VisibleChunk {
    start_char: u64,
    start_byte: u64,
    start_utf16: u64,
    char_len: u32,
    byte_len: u32,
    utf16_len: u32,
}
```

Application flow on the client:

```text
receive TextEdit { start_char, end_char, replacement }
  -> locate line/chunk by char metadata
  -> convert char range to local byte range inside bounded chunk
  -> apply replacement to client text model
  -> update line/chunk metadata incrementally
  -> mark affected lines/chunks dirty
```

GPUI input flow on the Normal-mode client:

```text
GPUI UTF-16 range
  -> locate line/chunk by UTF-16 metadata
  -> convert to local byte range for immediate edit
  -> derive char range from metadata
  -> emit TextEditTransaction with start_char/end_char
```

Performance rule:

```text
Naive whole-string char-to-byte scans are not acceptable in the hot path.
Char-to-byte and UTF-16-to-byte conversions must be line/chunk-metadata based.
```

## 10. Undo/Redo Ownership

Decision:

```text
Use a shared transaction log.
The Normal-mode owning client may execute undo/redo locally for transactions it owns.
The server processes/records the same undo/redo transaction in the shared log.
```

Rationale:

- Preserves fast interactive undo/redo for the editing client.
- Keeps one transaction model for client-native edits, server-generated edits, undo, and redo.
- Allows the server to inspect/process/persist the same transaction history.
- Allows Read-mode followers to receive undo/redo as ordinary transaction updates.
- Avoids making every undo/redo wait for server round trip.

Conceptual model:

```text
Normal-mode client owns primitive edits
  -> appends local transaction to local transaction log
  -> sends transaction to server
  -> server appends/processes transaction in shared log

Normal-mode client invokes undo for owned local group
  -> computes inverse transaction locally
  -> applies inverse immediately
  -> sends undo transaction to server
  -> server appends/processes undo transaction
  -> Read-mode clients receive the undo transaction as an update
```

Transaction requirements:

- Every text mutation is represented as a transaction.
- Undo/redo operations are also represented as transactions.
- Transactions include source information.
- Transactions include undo-group identity.
- Transactions include selection/cursor before and after where relevant.
- Server-generated edits enter the same transaction stream.
- Read-mode clients apply transactions from the shared stream and do not independently perform local undo.

Open implementation details:

- Exact undo group coalescing rules for typing bursts.
- How server-generated edits interleave with client-owned undo groups.
- Whether server-generated edits are undoable by the Normal-mode client by default.
- How to expose transaction history to extensions safely.

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

## 13. Async Semantic Result Versioning

Decision:

```text
Use request id + buffer version + cancellation/supersession.
Latest request wins per semantic feature/scope.
Semantic results should be complete snapshots for their scope, not fragile deltas.
The client keeps the last valid semantic state until a newer valid snapshot replaces it.
```

This applies to async features that should never block the typing hot path, including:

- syntax highlighting,
- diagnostics,
- autocomplete,
- semantic tokens,
- LaTeX previews,
- inline hints,
- code lenses,
- AI suggestions,
- async search/index results,
- extension-provided semantic/decorative results.

## 13.1 Core Problem

The Normal-mode client changes text immediately. The server or a background worker may compute semantic information from an older text version. By the time the result is ready, the buffer may have changed.

Example:

```text
version 100: user typed "\\alp"
server starts autocomplete/highlight request 55 for version 100

version 101: user types "h"
version 102: user types "a"

request 55 returns for version 100
current client text is version 102
```

The client must not apply stale information that no longer matches the visible text.

## 13.2 Request Metadata

Every async semantic request should include metadata like:

```rust
struct SemanticRequestMeta {
    request_id: u64,
    buffer_id: u64,
    buffer_version: u64,
    transaction_id: u64,
    kind: SemanticKind,
    scope: SemanticScope,
    supersedes: Option<RequestId>,
}
```

The important identity is:

```text
(buffer_id, kind, scope)
```

Examples:

```text
(buffer 1, SyntaxHighlight, visible lines 0..80)
(buffer 1, Diagnostics, whole buffer)
(buffer 1, Autocomplete, cursor/request scope)
(buffer 1, LatexPreview, math block scope)
```

## 13.3 Latest-Request-Wins Rule

For each `(buffer_id, kind, scope)`, only the latest request matters.

Server maintains:

```rust
latest_request_by_scope: HashMap<SemanticScopeKey, RequestId>
```

When a new request is scheduled:

```text
request 56 supersedes request 55
request 55 cancellation token is set
latest_request_by_scope[key] = 56
```

When a result returns on the server:

```text
if result.request_id != latest_request_by_scope[key]:
    discard
else:
    send to client
```

The client also validates defensively:

```text
if result.request_id is older than latest known request for scope:
    discard
if result.buffer_version is incompatible with current buffer version:
    discard
otherwise:
    apply atomically for that scope
```

## 13.4 Snapshot Results Per Scope

Initial semantic results should be scope snapshots, not patch streams.

Good:

```text
SyntaxHighlightResult for visible lines 0..80 replaces all syntax highlighting for lines 0..80.
DiagnosticsResult for whole buffer replaces diagnostics for that diagnostic source.
LatexPreviewResult for math block replaces preview for that block.
```

Avoid initially:

```text
add highlight A
remove highlight B
modify highlight C
```

Reason:

```text
If request 55 is canceled and request 56 completes, request 56 must contain everything the client needs for its scope.
Canceled intermediate requests must not leave holes.
```

This is the carry-forward rule:

```text
The client keeps the last valid semantic state.
The server sends a complete replacement snapshot for the next valid state.
Canceled intermediate results do not clear anything and do not need to be replayed.
```

## 13.5 Client Semantic State

The client maintains semantic state per scope:

```rust
struct SemanticClientState {
    latest_request_by_scope: HashMap<SemanticScopeKey, RequestId>,
    applied_generation_by_scope: HashMap<SemanticScopeKey, RequestId>,
    pending_results: HashMap<SemanticScopeKey, VecDeque<SemanticResult>>,
}
```

Queues should be bounded. For most semantic scopes, capacity can be `1`:

```text
for the same buffer/kind/scope:
  keep newest pending result
  drop older pending result
```

Client application rule:

```text
result arrives
  -> if stale request id, drop
  -> if stale/incompatible buffer version, drop
  -> if newer than applied generation, replace semantic state for that scope
```

Text transactions are sequential and must not be skipped. Semantic results are not text transactions. Because semantic results are snapshots by scope, they do not require replaying every older semantic result.

## 13.6 Server Cancellation, Debounce, And Coalescing

The server should not schedule expensive semantic work directly on every keystroke without control.

Use debounce/coalescing per kind/scope:

```text
text changes
  -> mark semantic scope dirty
  -> debounce/coalesce briefly
  -> schedule latest request for that scope
```

Example debounce policies:

```text
syntax visible range: 8-16ms or next frame-ish
autocomplete: 30-80ms
diagnostics: 150-500ms
LaTeX preview: 100-300ms
```

When a newer request supersedes an older one:

```text
cancel old token
drop queued unsent old request
ensure new request is a complete snapshot for its scope
```

Server queues must be bounded. On overflow:

```text
drop/coalesce older pending work for the same scope
keep newest request
never let semantic work compete with typing/editor hot path
```

## 13.7 Dirty Scopes

Semantic work should be scheduled by dirty scope, not always by whole file.

Examples:

```text
user types in visible line 20
  -> schedule syntax highlight for visible range / affected visible region
  -> schedule autocomplete if relevant at cursor
  -> schedule diagnostics after longer debounce for whole buffer
  -> schedule LaTeX preview only if a math block is affected
```

Each result is still a complete snapshot for its chosen scope.

## 13.8 Version Compatibility Rule

Initial strict rule:

```text
Apply semantic result only if result.buffer_version == client.current_buffer_version for that scope's text state.
```

If stale:

```text
do not apply stale result
do not clear existing semantic state
keep previous valid highlights/diagnostics/previews visible until a newer valid result arrives
```

This prevents flicker and avoids semantic holes while maintaining correctness.

## 13.9 Example Sequence

```text
version 100
request 55 syntax visible lines 0..60 scheduled

user types -> version 101
request 56 syntax visible lines 0..60 scheduled
request 55 canceled

request 55 completes anyway
server sees latest request is 56 -> discard

client still displays last valid syntax from request 54

request 56 completes
server sends full syntax snapshot for lines 0..60 at version 101

client checks:
  request 56 is latest
  version 101 == current
then replaces syntax for lines 0..60
```

No hole, no flicker from cancellation, no stale replay, and no need to apply request 55.

## 13.10 Patch-Based Semantic Results Are Deferred

Some future semantic features may want patch-based updates. If introduced, they must carry semantic generation metadata:

```rust
struct SemanticPatchMeta {
    base_semantic_generation: u64,
    new_semantic_generation: u64,
    request_id: u64,
    buffer_id: u64,
    buffer_version: u64,
    kind: SemanticKind,
    scope: SemanticScope,
}
```

Client may apply a semantic patch only if:

```text
client.applied_generation_for_scope == base_semantic_generation
```

If not, request or wait for a full snapshot.

Initial rule:

```text
Do not use patch-based semantic results initially.
Use complete snapshots per scope.
```

## 13.11 Responsibilities

Server responsibilities:

- Maintain latest request id per `(buffer, kind, scope)`.
- Cancel superseded requests.
- Coalesce/debounce semantic work.
- Keep semantic work queues bounded.
- Send only latest valid results.
- Make initial results complete snapshots for their scope.
- Never require a canceled result to have been applied.
- Include request/version metadata on every result.

Client responsibilities:

- Maintain current buffer version/transaction id.
- Maintain latest known/applied request id per semantic scope.
- Drop stale results.
- Keep last valid semantic state until replaced.
- Apply semantic snapshots atomically per scope.
- Bound pending semantic result queues.
- Never let semantic result application block typing/render hot path.

# Remaining Concerns And Pending Decisions For Review

This section summarizes what still needs decisions or deeper design after reviewing the full model.

## A. Normal/Read Mode UX And Ownership Transfer

Core decision is locked:

```text
one Normal-mode owner per file
other clients are Read-mode followers
```

Still pending:

- [ ] Exact command names for requesting/releasing Normal mode.
- [ ] Whether a Read-mode client can request ownership while another client owns Normal mode.
- [ ] Whether ownership transfer requires confirmation from the current owner.
- [ ] What happens if the Normal-mode client disconnects, crashes, or becomes unresponsive.
- [ ] How the UI displays Normal mode vs Read mode.
- [ ] How the UI displays which client/session owns Normal mode.
- [ ] Whether Read mode is purely read-only or can still execute non-mutating commands locally/server-side.

Suggested initial policy:

```text
Ownership transfer is explicit and conservative.
If the owner disconnects cleanly, ownership becomes available.
If the owner disappears unexpectedly, server marks ownership stale and allows a new client to acquire after validating latest transaction state or resyncing from disk/server state.
```

## B. Transaction Ordering, Acknowledgement, And Resync

Core decision is locked:

```text
Normal-mode client applies primitive edits locally and sends ordered transactions to server.
Server processes transactions in order.
```

Still pending:

- [ ] Exact transaction id scheme: per client/file, per buffer, or global server sequence.
- [ ] Whether server sends acknowledgements for every transaction or coalesced acknowledgements.
- [ ] What client does if server detects a missing transaction.
- [ ] What server does if it receives out-of-order transactions.
- [ ] What resync protocol is used if transaction logs diverge.
- [ ] How much pending transaction history the client retains for resend/resync.
- [ ] How much applied transaction history the server retains for followers/semantic versioning.

Suggested initial policy:

```text
Use monotonically increasing per-buffer transaction ids from the Normal-mode owner.
Server applies in order and sends latest_applied_transaction acknowledgements.
If a gap or mismatch is detected, server requests resync from the owner or falls back to a full buffer snapshot.
```

## C. Transaction Format Details

Core decision is locked:

```text
Text edits are char-range authoritative and carry no byte/line hints.
```

Still pending:

- [ ] Exact `Selection` representation in transactions.
- [ ] Whether selections are char ranges, anchor/head pairs, or richer multi-cursor structures.
- [ ] Whether transactions can contain multiple edits and how overlapping edits are ordered/validated.
- [ ] Whether replacement text in multi-edit transactions is applied in ascending or descending range order.
- [ ] How transaction inversion is represented for undo/redo.
- [ ] Whether non-text state changes, such as mode changes or cursor-only moves, are transactions or separate events.

Suggested initial policy:

```text
Start with single-cursor/single-selection transactions but design structs to allow multi-cursor later.
Multi-edit transactions must declare deterministic ordering and reject overlapping ranges unless explicitly supported.
```

## D. Undo/Redo Detailed Policy

Core decision is locked:

```text
Use shared transaction log.
Normal-mode owner may execute local undo/redo for owned transactions.
Server records/processes the same undo/redo transaction.
```

Still pending:

- [ ] Typing coalescing rules: time-based, boundary-based, command-based, or all three.
- [ ] Whether server-generated edits are undoable by the Normal-mode client by default.
- [ ] How server-generated edits interleave with local typing undo groups.
- [ ] Whether undo stops at server-generated edits or can undo across them.
- [ ] Whether Read-mode clients can request undo or only observe undo transactions.
- [ ] How transaction history is exposed to extensions without allowing unsafe mutation.

Suggested initial policy:

```text
Coalesce contiguous typing into undo groups until cursor jump, selection change, command boundary, mode change, or timeout.
Server-generated edits create explicit undo boundaries until a more advanced policy is designed.
Read-mode clients do not perform undo directly.
```

## E. Compiled Routing Table Details

Core decision is locked:

```text
Server/config runtime is authoritative.
Client receives versioned compiled routing snapshots.
Client never runs TypeScript on input hot path.
```

Still pending:

- [ ] Exact normalized key representation.
- [ ] Exact key category taxonomy and QWERTY/default key list.
- [ ] How keyboard layouts beyond QWERTY are represented.
- [ ] How numpad keys are normalized.
- [ ] How IME/composition input bypasses or participates in routing.
- [ ] Whether mouse events are in the same routing table or a separate input policy table.
- [ ] How mode-specific routing inheritance works.
- [ ] How route conflicts are diagnosed.

Suggested initial policy:

```text
Define a normalized physical/logical key model explicitly.
Keep default Normal/Read routing in typed built-in config descriptors.
Emit diagnostics for ambiguous routes and for broad printable server-routing.
```

## F. Config Hot Reload Policy

Core decision is locked:

```text
Hot reload produces a new compiled config generation.
Client atomically swaps snapshots.
Failed reload keeps last-known-good config.
```

Still pending:

- [ ] Manual reload command name and behavior.
- [ ] File watcher debounce duration.
- [ ] Whether auto reload is enabled by default.
- [ ] How config errors are shown in UI.
- [ ] Whether reload can change active Normal/Read ownership.
- [ ] Whether reload can remove current mode and how fallback is displayed.
- [ ] Whether config generation changes cancel all prefix states or only affected ones.

Suggested initial policy:

```text
Provide manual reload first, then add auto reload with debounce.
Cancel in-progress prefix chords on any config generation change.
Show reload errors without changing active config.
```

## G. Semantic Result Scope Definitions

Core decision is locked:

```text
Async semantic results use latest-request-wins, cancellation/supersession, and full snapshots per scope.
```

Still pending:

- [ ] Exact `SemanticScope` variants.
- [ ] Scope granularity for syntax highlighting: visible range, line range, whole buffer, or parser region.
- [ ] Scope granularity for diagnostics: whole buffer vs source-specific sets.
- [ ] Scope identity for LaTeX previews: char range, stable block id, or discovered region id.
- [ ] How semantic state is cleared when a scope disappears.
- [ ] Whether strict exact buffer-version matching is too strict for some scopes after initial implementation.
- [ ] Default debounce values per semantic kind.

Suggested initial policy:

```text
Start with conservative scopes: visible line range for syntax, whole-buffer per diagnostic source, cursor scope for autocomplete, and math-block scope for LaTeX preview.
Use exact version matching initially.
```

## H. Save And Fresh-Buffer Command Semantics

Core decision is locked:

```text
Save request includes latest transaction id/version.
Server applies through that point before saving.
Commands that require current text declare requires_fresh_buffer.
```

Still pending:

- [ ] Exact `SaveRequest` protocol shape.
- [ ] How server waits for missing transactions without blocking unrelated work.
- [ ] Timeout/error behavior if required transactions never arrive.
- [ ] Which initial built-in commands set `requires_fresh_buffer`.
- [ ] Whether extension commands can declare `requires_fresh_buffer`.
- [ ] Whether LSP requests always require fresh buffer or can use async snapshots.

Suggested initial policy:

```text
Save waits for a specific transaction id with timeout.
On timeout, report failure rather than saving stale text.
Fresh-buffer command metadata is part of command descriptors and extension command descriptors.
```

## I. Client Text Model And GPUI Performance Work

Core decision is locked:

```text
Client must maintain line/chunk metadata for bounded char/byte/UTF-16 conversion.
Rendering must not clone/split whole visible text or shape whole long lines per key.
```

Still pending:

- [ ] Exact `VisibleTextModel`, `VisibleLine`, and `VisibleChunk` data structures.
- [ ] Chunk size and invalidation strategy.
- [ ] Long-line horizontal viewport behavior.
- [ ] How IME marked text interacts with chunk metadata.
- [ ] How selections/multi-cursor will be represented.
- [ ] How GPUI shaped-line/chunk cache invalidates on style/font/theme changes.
- [ ] How UI profiler measures actual key-to-paint latency.

Suggested initial policy:

```text
Implement the client text model before relying on char-range transactions for performance.
Add real UI profiler mode before and after the model change to prove improvement.
```

## J. Documentation And Configuration Metadata

Because routing, modes, fresh-buffer commands, and performance-sensitive behavior are user-visible/configurable, implementation must include structured metadata.

Still pending:

- [ ] Setting descriptors for routing/performance policies.
- [ ] Mode descriptors for Normal and Read mode.
- [ ] Command descriptors for Normal/Read mode switching and config reload.
- [ ] Documentation for performance risks of server-routed printable keys.
- [ ] Tests ensuring built-in settings/commands/modes have documentation metadata.

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
