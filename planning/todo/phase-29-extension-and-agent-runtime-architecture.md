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

