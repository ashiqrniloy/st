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

