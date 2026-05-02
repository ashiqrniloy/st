# Step-by-Step Implementation Plan

This document is intentionally only an ordered implementation checklist. Overall architecture and design rationale live in `README.md`.

## Phase files

Phases are split into ordered files under:

- `done/` for completed phases
- `todo/` for phases still to be implemented

The `phase-XX-...md` prefix preserves execution order when sorted by file name.

- Phase 1: [Support Multiple Clients](done/phase-01-support-multiple-clients.md) — `done`
- Phase 2: [Add Explicit Server Shutdown](done/phase-02-add-explicit-server-shutdown.md) — `done`
- Phase 3: [Add Optional Idle Shutdown](done/phase-03-add-optional-idle-shutdown.md) — `done`
- Phase 4: [Emacs-Like Command Structure Foundation](done/phase-04-emacs-like-command-structure-foundation.md) — `done`
- Phase 5: [Self-Documentation And Help Query Foundation](done/phase-05-self-documentation-and-help-query-foundation.md) — `done`
- Phase 6: [Document Existing Builtin Capabilities](done/phase-06-document-existing-builtin-capabilities.md) — `done`
- Phase 7: [Configuration Foundation](done/phase-07-configuration-foundation.md) — `done`
- Phase 8: [Make Existing Behavior Configurable](done/phase-08-make-existing-behavior-configurable.md) — `done`
- Phase 9: [Performance Guardrails And Instrumentation](done/phase-09-performance-guardrails-and-instrumentation.md) — `done`
- Phase 10: [Scalable Buffer Storage And Versioned Snapshots](done/phase-10-scalable-buffer-storage-and-versioned-snapshots.md) — `done`
- Phase 11: [Incremental Scene And Viewport Protocol](done/phase-11-incremental-scene-and-viewport-protocol.md) — `done`
- Phase 12: [Background Worker Architecture](done/phase-12-background-worker-architecture.md) — `done`
- Phase 13: [Load JavaScript From Disk](done/phase-13-load-javascript-from-disk.md) — `done`
- Phase 14: [JS Commands And Keybindings](done/phase-14-js-commands-and-keybindings.md) — `done`
- Phase 15: [Code Review](done/phase-15-code-review.md) — `done`
- Phase 16: [UI Window Management And Split Panes](todo/phase-16-ui-window-management-and-split-panes.md) — `todo`
- Phase 17: [UI Design Command Center](todo/phase-17-ui-design-command-center.md) — `todo`
- Phase 18: [Fuzzy Search Implementation](todo/phase-18-fuzzy-search-implementation.md) — `todo`
- Phase 19: [Execute Command Function](todo/phase-19-execute-command-function.md) — `todo`
- Phase 20: [View Documentation Function](todo/phase-20-view-documentation-function.md) — `todo`
- Phase 21: [Manual Hot Reload](todo/phase-21-manual-hot-reload.md) — `todo`
- Phase 22: [Extension Lifecycle](todo/phase-22-extension-lifecycle.md) — `todo`
- Phase 23: [File I/O](todo/phase-23-file-i-o.md) — `todo`
- Phase 24: [Undo/Redo](todo/phase-24-undo-redo.md) — `todo`
- Phase 25: [Text Editing Features In Depth](todo/phase-25-text-editing-features-in-depth.md) — `todo`
- Phase 26: [Multi-Session Client Architecture](todo/phase-26-multi-session-client-architecture.md) — `todo`
- Phase 27: [Systemd Integration](todo/phase-27-systemd-integration.md) — `todo`
- Phase 28: [Adopt GPUI Component For Non-Editor UI](todo/phase-28-adopt-gpui-component-for-non-editor-ui.md) — `todo`
- Phase 29: [Extension And Agent Runtime Architecture](todo/phase-29-extension-and-agent-runtime-architecture.md) — `todo`
- Phase 30: [Permissions And AI Layer](todo/phase-30-permissions-and-ai-layer.md) — `todo`
