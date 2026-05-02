## Phase 1: Support Multiple Clients

Goal: prove the client/server model is real.

- [x] Allow multiple clients to connect simultaneously.
- [x] Broadcast editor updates to all clients initially.
- [x] Track each client's connection state.
- [x] Handle one client disconnecting without affecting others.
- [x] Defer active view/client behavior until later.

Milestone:

```text
Open two clients.
Type in one.
Both receive updates.
Closing one leaves the other and server running.
```

