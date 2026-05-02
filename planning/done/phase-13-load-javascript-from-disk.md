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

