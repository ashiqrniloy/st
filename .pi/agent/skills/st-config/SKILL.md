---
name: st-config
description: Use this skill whenever working on the st editor codebase and adding or changing defaults, settings, paths, commands, keybindings, modes, extensions, tools, runtime behavior, UI behavior, permissions, LSP behavior, AI/tool behavior, or anything users/extensions/agents may need to modify. It enforces st's TypeScript-first configuration architecture with optional YAML data and manifests, and prevents hard-coded behavior that should be configurable.
---

# st Config Skill

Use this skill before adding or changing behavior that users, extensions, or AI-created tools may want to configure.

If available, also read project root `config.md` for full rationale. A bundled copy may exist at `references/config.md`.

## Non-negotiables

1. **Do not hard-code user-tunable behavior.** If a value affects user workflow, UI, editing behavior, modes, extensions, tools, permissions, runtime behavior, paths, LSP, AI, or performance policy, design a configuration path.
2. **TypeScript is the primary configuration layer.** The authoritative config entrypoint is intended to be `~/.config/st/init.ts`.
3. **YAML is declarative data only.** YAML may describe settings, manifests, keybindings, profiles, extension/tool dirs, etc. Do not turn YAML into a programming language.
4. **Do not enforce one filesystem structure.** Provide defaults, but allow users to load config/extensions/tools from arbitrary paths.
5. **Rust exposes safe capabilities; TypeScript composes them.** Configuration should call typed APIs that the Rust server validates.
6. **Configurable things must be documented.** Whenever adding a setting/config option, also update documentation metadata per the `st-documentation` skill.
7. **Extensions/tools should be configurable too.** Users must decide what extension/tool dirs/files/manifests to load.
8. **AI-created tools need explicit loading and permissions.** Do not auto-trust generated tools without config/permission flow.

## Preferred model

```text
Rust core
  owns editor state, rendering protocol, buffers, sessions, transactions, permissions
  exposes safe typed capabilities

TypeScript config/runtime
  composes capabilities
  registers commands, keybindings, modes, extensions, tools, hooks
  loads optional YAML files

YAML files
  declarative data only: settings, manifests, keybindings, profiles, directories
```

## Primary config shape

Default entrypoint:

```text
~/.config/st/init.ts
```

Example:

```ts
import { editor, keymap, extensions, tools, config } from "st";

editor.set({ theme: "catppuccin", fontSize: 14 });
keymap.bind("ctrl+s", "file.save");

const cfg = await config.loadYaml("~/.config/st/settings.yml");
editor.set(cfg.editor ?? {});

for (const dir of cfg.extension_dirs ?? []) await extensions.loadDir(dir);
for (const dir of cfg.tool_dirs ?? []) await tools.loadDir(dir);
```

## What should be configurable

Default to configurable when a choice affects users or future extension/tool behavior:

```text
paths and directories
socket/runtime paths where practical
idle timeout and lifecycle policy
font, theme, colors, cursor style
keybindings
commands and command aliases
mode activation rules
file patterns
editor behavior and text editing preferences
rendering preferences
extension load paths
extension enable/disable state
tool load paths
AI provider/model/tool permissions
LSP server command/env/settings
formatters/linters
performance policies, timeouts, debounce intervals, queue limits
```

Not every option needs a full UI immediately, but it should have a typed setting/descriptor path if public or user-tunable.

## Implementation checks

Before finishing a code change, ask:

- Did I introduce a constant/default that users may want to change?
- Did I introduce a path, timeout, keybinding, mode rule, command behavior, UI preference, permission, provider, or tool behavior?
- Should this be a typed setting, a TypeScript API, a YAML field, or an extension/tool manifest field?
- Is the default documented?
- Is the config option discoverable through the self-documentation system?
- Can extensions or AI tools configure this without editing Rust code?
- Did I avoid hard-coding one directory layout?

## If config infrastructure does not exist yet

Prefer creating or extending the configuration foundation rather than adding more hard-coded behavior.

If immediate full config support is too large, isolate the default in a named setting descriptor or config struct so it can move into the registry without behavior rewrites.
