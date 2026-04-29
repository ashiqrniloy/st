# Configuration And Extension System Direction

This document records the intended direction for `st` configuration, extensions, and AI-created tools.

## Decision

Use a hybrid model:

```text
TypeScript-first configuration
+
optional YAML declarative files
+
optional manifests for packaged extensions/tools
```

This combines the flexibility of Emacs-style executable configuration with YAML's usefulness for structured data, manifests, and AI-generated metadata.

## Core Principle

Rust exposes safe editor capabilities. TypeScript composes those capabilities. YAML describes data.

```text
Rust core
  owns editor state, rendering protocol, buffers, sessions, transactions, permissions

TypeScript config/runtime
  composes capabilities exposed by Rust
  registers commands, keybindings, tools, hooks, extensions

YAML files
  declarative data only
  settings, extension manifests, tool manifests, keybinding tables, profiles
```

YAML should not become the primary extension language. If YAML becomes responsible for conditionals, variables, hooks, command composition, and dynamic behavior, it will effectively turn into a weaker custom programming language.

## Primary Config Entrypoint

The primary user configuration file should be executable TypeScript:

```text
~/.config/st/init.ts
```

Example:

```ts
import { editor, keymap, extensions, tools, config } from "st";

editor.set({
  theme: "catppuccin",
  fontSize: 14,
});

keymap.bind("ctrl+s", "file.save");
keymap.bind("ctrl+p", "commandPalette.open");

await extensions.loadDir("~/.config/st/extensions");
await tools.loadDir("~/.config/st/tools");

const settings = await config.loadYaml("~/.config/st/settings.yml");
editor.set(settings.editor ?? {});
```

## Optional YAML Files

YAML files are supported as declarative data and may be loaded by `init.ts`.

Conventional but non-mandatory examples:

```text
~/.config/st/settings.yml
~/.config/st/keybindings.yml
~/.config/st/extensions.yml
~/.config/st/tools.yml
```

Example:

```yaml
editor:
  theme: catppuccin
  font_size: 14

extension_dirs:
  - ~/.config/st/extensions
  - ~/.local/share/st/extensions

tool_dirs:
  - ~/.config/st/tools
  - ~/.local/share/st/tools
```

Then `init.ts` can decide how to apply it:

```ts
const cfg = await config.loadYaml("~/.config/st/settings.yml");

editor.set(cfg.editor ?? {});

for (const dir of cfg.extension_dirs ?? []) {
  await extensions.loadDir(dir);
}

for (const dir of cfg.tool_dirs ?? []) {
  await tools.loadDir(dir);
}
```

## Do Not Enforce One Filesystem Structure

Provide useful defaults, but do not make them mandatory.

Users should be able to do any of these:

```ts
await import("./my-custom-editor.ts");
await config.loadYamlConfig("./config.yml");
await extensions.loadDir("~/work/editor-extensions");
await tools.loadDir("~/ai-generated-tools");
```

This preserves the Emacs-like property that users can shape the editor around their own workflow.

## Extension Formats

Support both lightweight script extensions and packaged extensions.

### Single-file extension

```text
~/.config/st/extensions/insert-date.ts
```

```ts
import { commands, editor } from "st";

commands.register("user.insertDate", () => {
  editor.insertText(new Date().toISOString());
});
```

### Packaged extension

```text
my-extension/
  st.extension.yml
  main.ts
```

Manifest:

```yaml
id: user.my-extension
name: My Extension
entry: main.ts

contributes:
  commands:
    - id: user.my-extension.insertDate
      title: Insert Date
  keybindings:
    - key: ctrl+d
      command: user.my-extension.insertDate
```

Entrypoint:

```ts
import { commands, editor } from "st";

commands.register("user.my-extension.insertDate", () => {
  editor.insertText(new Date().toISOString());
});
```

Loading remains user-controlled:

```ts
await extensions.load("~/.config/st/extensions/my-extension");
await extensions.loadDir("~/.config/st/extensions");
```

## AI-Created Tools

AI agents should be able to create tools as TypeScript implementation plus YAML/JSON metadata.

Example:

```text
~/.config/st/tools/summarize-selection/
  tool.yml
  main.ts
```

`tool.yml`:

```yaml
id: user.summarizeSelection
name: Summarize Selection
entry: main.ts
permissions:
  read_selection: true
  network: ask
```

`main.ts`:

```ts
import { tools, editor } from "st";

tools.register("user.summarizeSelection", async () => {
  const text = await editor.selection.getText();
  const summary = await tools.ai.complete({
    prompt: `Summarize this:\n${text}`,
  });

  await editor.insertText(summary);
});
```

The user decides whether to load generated tools:

```ts
await tools.loadDir("~/.config/st/tools");
```

or via YAML consumed by `init.ts`:

```yaml
tool_dirs:
  - ~/.config/st/tools
```

## Security And Permissions

Executable configuration is powerful and should be treated as trusted user code.

Packaged extensions and AI-created tools should pass through a permission system before accessing sensitive capabilities.

Examples of gated capabilities:

```text
file read/write
network
subprocess execution
workspace inspection
selection access
buffer mutation
AI provider access
```

The Rust server remains the authority:

```text
Extension/tool requests action
  -> Rust server checks permissions and validates request
  -> Rust server applies typed command/transaction if allowed
  -> Rust server emits updates
```

Extensions and tools should not directly mutate editor state.

## Runtime Direction

Normal editor configuration and lightweight extensions use the normal editor JS runtime.

Long-running AI agents use isolated per-agent runtimes later:

```text
normal editor JS runtime
  commands, keybindings, themes, modes, lightweight hooks

per-agent JS runtime
  one runtime per active agent
  long-running or blocking agent work
```

This prevents blocked agent code from freezing normal editor behavior.

## Recommended Implementation Path

1. Add `~/.config/st/init.ts` as the primary config entrypoint.
2. Expose a small TypeScript API from Rust/Deno.
3. Add YAML loading as a helper API, not the primary behavior engine.
4. Add extension loading from explicit files/directories.
5. Add optional extension manifests.
6. Add tool loading from explicit files/directories.
7. Add optional tool manifests.
8. Add permissions around sensitive capabilities.
9. Add package/discovery UI later.

## Summary

The intended model is:

```text
init.ts is authoritative
YAML is declarative data
manifests describe reusable extensions/tools
Rust validates and applies all state changes
users control where code and config are loaded from
```

This gives `st` Emacs-like flexibility without inventing a YAML programming language, while still supporting structured configuration and AI-generated tools.
