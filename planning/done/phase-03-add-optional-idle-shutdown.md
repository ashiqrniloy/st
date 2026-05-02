## Phase 3: Add Optional Idle Shutdown

Goal: avoid unwanted background daemons during early development.

- [x] Add configurable idle timeout.
- [x] Track connected client count.
- [x] If no clients remain for N seconds/minutes, shutdown server.
- [x] Disable idle shutdown when launched explicitly as long-running service.

