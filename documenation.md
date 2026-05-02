# Self-Documentation Guidelines

This document records the documentation architecture for `st`.

## Goal

`st` should be self-documenting in an Emacs-like way.

If a user can invoke, configure, load, bind, extend, permit, or inspect something, then the editor should be able to describe it from inside the program.

```text
If a command, setting, keybinding, mode, extension, tool, permission, or API exists,
the editor can introspect it and show documentation for it.
```

Documentation is not separate prose added later. Documentation is part of runtime capability registration.

## Core Principle

Every public capability must be registered with structured metadata.

The metadata should describe:

```text
what the capability is
where it came from
how the user invokes it
what arguments/settings/permissions it uses
examples of use
related capabilities
```

The help system should be generated from live registries, not from static markdown alone.

## Emacs-Like Help Model

Emacs has commands such as:

```text
describe-function
describe-variable
describe-key
```

`st` should eventually provide equivalents:

```text
describe-command
describe-setting
describe-keybinding
describe-mode
describe-extension
describe-tool
describe-permission
describe-api
```

Users should be able to access documentation through the command system and command palette.

Example user-facing commands:

```text
help.open
help.commands
help.command
help.keybindings
help.settings
help.modes
help.extensions
help.tools
help.permissions
help.api
```

## Required Registries

The Rust server should eventually own central registries for public capabilities:

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

Each registry stores both behavior and documentation metadata.

The client asks the server for documentation. The client should not scrape random files to build help.

## Command Metadata

Commands should not be registered as only an ID and handler.

Bad:

```js
commands.register("file.save", handler);
```

Good:

```js
commands.register({
  id: "file.save",
  title: "Save File",
  description: "Save the active buffer to disk.",
  category: "File",
  defaultKeybindings: ["ctrl+s"],
  arguments: [],
  examples: [
    "Use `ctrl+s` to save the current file."
  ],
}, handler);
```

Rust builtins should follow the same rule:

```rust
registry.register(
    CommandDescriptor {
        id: "file.save".into(),
        title: "Save File".into(),
        description: "Save the active buffer to disk.".into(),
        category: "File".into(),
        source: CapabilitySource::Builtin,
        arguments: vec![],
        default_keybindings: vec![...],
        examples: vec![...],
        related: vec![...],
    },
    CommandHandler::RustBuiltin(...),
);
```

## Setting And Configuration Metadata

Settings and configuration options should be registered with type/default/range documentation and should explain how the user configures them.

```js
settings.register({
  id: "editor.fontSize",
  title: "Font Size",
  description: "Controls the editor text size in pixels.",
  type: "number",
  default: 14,
  minimum: 6,
  maximum: 72,
  configureWith: {
    javascript: "editor.set({ fontSize: 14 })",
    yaml: "editor:\n  font_size: 14",
  },
  requiresRestart: false,
});
```

Every configurable behavior should document:

```text
type
default
valid range or enum values
how to configure it from init.js
YAML field if one exists
example configuration
whether reload/restart is required
```

## Mode Metadata

Modes should describe file patterns, behavior, contributed commands, and related services.

```js
modes.register({
  id: "markdown",
  title: "Markdown Mode",
  description: "Editing support for Markdown files.",
  filePatterns: ["*.md"],
  commands: ["markdown.toggleCheckbox"],
});
```

## Tool Metadata

AI tools and user-created tools must include metadata and permission information.

```js
tools.register({
  id: "notes.summarizeSelection",
  title: "Summarize Selection",
  description: "Summarizes the current selection using the configured AI provider.",
  permissions: ["read_selection", "network"],
  inputs: [
    { name: "tone", type: "string", description: "Summary tone." }
  ],
  examples: [
    "Summarize the selected paragraph in a concise tone."
  ],
}, handler);
```

## Permission Metadata

Permissions should be visible and explainable.

Each permission should document:

```text
id
title
description
risk level
what grants it
what operations it allows
whether it can be denied or prompted
```

## Structured Documentation

Prefer structured documentation over free-form markdown-only docs.

A descriptor may contain markdown descriptions, but the core fields should be typed.

Suggested common fields:

```rust
struct Documentation {
    summary: String,
    description: String,
    arguments: Vec<ArgumentDoc>,
    examples: Vec<Example>,
    related: Vec<DocLink>,
}
```

This allows rendering documentation as:

```text
help panel
command palette preview
plain text
markdown
HTML export
AI context
```

## Documentation Protocol

Documentation should be exposed through the client/server protocol.

Possible specific protocol messages:

```rust
ClientToServer::ListCommands
ClientToServer::DescribeCommand { command_id }
ClientToServer::ListSettings
ClientToServer::DescribeSetting { setting_id }
```

Or a generic query model:

```rust
ClientToServer::DocumentationQuery(DocumentationQuery)
ServerToClient::DocumentationResult(DocumentationResult)
```

The server is the source of truth because it owns the active runtime registries.

## Enforcement

Public capabilities should not be allowed without documentation metadata.

Required fields for public descriptors:

```text
id
title
description
source
category or namespace when applicable
argument descriptions when arguments exist
setting default/type when settings exist
permission description/risk when permissions exist
```

Tests should enforce builtin coverage:

```rust
#[test]
fn all_builtin_commands_have_documentation() {}

#[test]
fn all_builtin_settings_have_documentation() {}

#[test]
fn all_permissions_have_documentation() {}
```

Extension validation should warn or fail for missing docs.

Suggested policy:

```text
development/test mode: fail when public capability lacks docs
normal user mode: warn and show extension load diagnostics
strict mode: fail extension load
```

## Documentation For Existing Code

As existing systems are converted to the command/registry model, add descriptors for all already-built functionality:

```text
client startup
server startup
server shutdown / quit
close client
insert text
backspace
move cursor left/right
scene updates / rendering behavior where user-facing
configuration entrypoints when implemented
```

## Documentation For Extension API

The JavaScript extension API should also be documented.

Long-term approach:

```text
JavaScript declarations
  -> generated structured API docs
  -> loaded into help registry
  -> shown through help commands
```

Until generation exists, manually register API namespace/method descriptors.

## AI Agent Rule

AI agents adding code must also add or update documentation descriptors when they add public capabilities.

If an AI agent creates a user-visible command, setting, tool, permission, mode, or extension API, it must add metadata in the same change.

## Summary

The intended model is:

```text
no public capability without metadata
metadata lives beside capability registration
server registries are the source of truth
help commands query live registries
tests enforce builtin documentation coverage
extension validation enforces extension documentation quality
```

This gives `st` Emacs-like self-documentation with structured, runtime-introspectable metadata.
