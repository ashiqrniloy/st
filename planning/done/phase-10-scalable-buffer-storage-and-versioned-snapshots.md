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

