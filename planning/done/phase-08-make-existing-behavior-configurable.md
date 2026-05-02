## Phase 8: Make Existing Behavior Configurable

Goal: audit and update current behavior so existing defaults that users may reasonably want to change are represented through the configuration foundation.

- [x] Audit existing hard-coded values and defaults.
- [x] Make editor background color configurable.
- [x] Make cursor visibility/style defaults configurable where applicable.
- [x] Make server auto-start idle timeout configurable.
- [x] Make explicit foreground server idle-timeout behavior documented/configurable through CLI/config where appropriate.
- [x] Make socket/runtime directory behavior configurable where safe, while preserving sensible defaults.
- [x] Make client/server startup behavior configurable where user-facing.
- [x] Make key handling defaults configurable through the command/keymap foundation where appropriate.
- [x] Make any hard-coded UI text/style defaults configurable if user-facing.
- [x] Add setting descriptors and documentation for each existing configurable option.
- [x] Add tests that existing configurable options have defaults and docs.
- [x] Add tests that configured values override defaults.
- [x] Ensure no new user-tunable hard-coded values are introduced without descriptors.

Milestone:

```text
Current implemented behavior has been audited, and user-tunable defaults are represented through documented configuration instead of scattered hard-coded constants.
```

