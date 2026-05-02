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

