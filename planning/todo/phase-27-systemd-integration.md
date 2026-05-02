## Phase 27: Systemd Integration

Goal: allow persistent background server operation.

Acceptance criteria:

- Foreground server mode and user-service operation are documented.
- Socket path creation and cleanup work correctly under systemd-style runtime environments.
- The plan clearly decides whether a daemon flag is needed or systemd is sufficient.

Test plan:

- Add runtime-dir/socket-path tests simulating systemd-style environment variables.
- Add CLI/help or documentation tests for foreground server mode and user-service instructions where practical.
- Add a validation check documenting the daemon-vs-systemd decision.

Implementation tasks:

- [ ] Add documented foreground server mode.
- [ ] Add documented user service example.
- [ ] Decide whether `st server --daemon` is needed or systemd is enough.
- [ ] Ensure socket path and cleanup work under systemd.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

