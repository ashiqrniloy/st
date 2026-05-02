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

