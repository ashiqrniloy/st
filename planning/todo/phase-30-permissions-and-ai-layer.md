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
