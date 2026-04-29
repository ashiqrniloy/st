---
name: st-documentation
description: Use this skill whenever working on the st editor codebase and adding or changing user-visible commands, settings, keybindings, modes, extensions, tools, permissions, help UI, protocol messages, or TypeScript/Rust APIs. It enforces the project's self-documenting Emacs-like documentation architecture so every public capability has structured metadata and can be shown to users from inside the editor.
---

# st Documentation Skill

Use this skill before adding or changing public/user-visible capability in `st`.

If available, also read the project root `documenation.md` for full guidelines. A bundled copy may exist at `references/documenation.md`.

## Non-negotiables

1. **No public capability without metadata.** Commands, settings, keybindings, modes, extensions, tools, permissions, and APIs must be registered with structured documentation.
2. **Documentation lives beside registration.** Do not add behavior in one place and leave docs as a later task.
3. **The server is the source of truth.** User-facing help should come from live registries owned/known by the Rust server, not from clients scraping files.
4. **Docs must be introspectable.** Prefer typed descriptors over markdown-only prose.
5. **Help is command-driven.** Documentation should be accessible through commands such as describe-command, describe-setting, describe-keybinding, describe-mode, describe-extension, describe-tool, describe-permission, and describe-api.
6. **Extensions and AI-created tools need docs too.** Extension/tool loading should validate descriptors and warn or fail when metadata is missing.
7. **Tests should enforce builtin documentation coverage.** Builtin commands/settings/permissions should not silently exist without docs.
8. **Configurable behavior must document how to configure it.** Settings and config APIs must include type/default/valid values, examples, and where the user can set them.

## Required metadata pattern

When adding a command, do not register only an ID and handler.

Bad:

```ts
commands.register("file.save", handler);
```

Good:

```ts
commands.register({
  id: "file.save",
  title: "Save File",
  description: "Save the active buffer to disk.",
  category: "File",
  defaultKeybindings: ["ctrl+s"],
  arguments: [],
  examples: ["Use `ctrl+s` to save the current file."],
}, handler);
```

Rust builtins should use equivalent descriptors.

## Minimum fields

For public descriptors, require or strongly prefer:

```text
id
title
description
source
category or namespace
arguments with descriptions, if any
examples, when useful
related docs/commands, when useful
```

Settings/configuration additionally need:

```text
type
default
valid range or enum values when applicable
where/how to configure it, such as init.ts API or YAML field
example configuration
whether restart/reload is required
```

Permissions additionally need:

```text
risk level
allowed operations
prompt/deny behavior
```

Tools additionally need:

```text
inputs
outputs when applicable
permissions
examples
```

## Registries to preserve

Design toward central registries:

```text
CommandRegistry
SettingRegistry
KeymapRegistry
ModeRegistry
ExtensionRegistry
ToolRegistry
PermissionRegistry
ApiRegistry
```

Each registry stores behavior plus documentation metadata.

## Implementation checks

Before finishing a change, ask:

- Did I add/change a user-visible command, setting, keybinding, mode, extension, tool, permission, config option, or API?
- If yes, did I add/update its descriptor?
- If it is configurable, did I document how to configure it and its default value?
- Can the user discover it through the command/help system?
- Did I add/update tests for required metadata if this is builtin?
- Did I avoid putting docs only in README/prose when runtime metadata is needed?

## If metadata infrastructure does not exist yet

Add TODOs only as a last resort. Prefer introducing the descriptor/registry foundation first if the change adds public capability.

At minimum, place metadata in a structure that can later be moved into the registry without rewriting behavior.
