## Phase 6: Document Existing Builtin Capabilities

Goal: update existing implemented behavior so the project starts with documentation coverage instead of adding docs only for future features.

- [x] Add metadata for client startup/open-client behavior where user-facing.
- [x] Add metadata for server startup behavior where user-facing.
- [x] Add metadata for explicit server shutdown / `st quit`.
- [x] Add metadata for close-client behavior.
- [x] Add metadata for insert text.
- [x] Add metadata for backspace.
- [x] Add metadata for move cursor left.
- [x] Add metadata for move cursor right.
- [x] Add metadata for current scene/render update behavior where user-facing.
- [x] Add metadata for idle shutdown configuration.
- [x] Add metadata for CLI-visible commands and flags where they intersect with in-editor help.
- [x] Add tests that all builtin commands have required metadata.
- [x] Add tests that all current user-visible settings/options have required metadata once represented in registries.
- [x] Add tests that documentation descriptors are non-empty and have stable IDs.
- [x] Add a development assertion or test helper that rejects builtin public capabilities without descriptors.

Milestone:

```text
Everything already built and visible to the user has initial structured documentation metadata and can be discovered through the help foundation.
```

