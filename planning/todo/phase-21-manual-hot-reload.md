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

