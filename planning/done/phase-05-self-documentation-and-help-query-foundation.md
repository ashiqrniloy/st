## Phase 5: Self-Documentation And Help Query Foundation

Goal: create the server-side documentation structure and command-accessible help path so available documentation can be shown inside the editor.

- [x] Define common documentation structs for summary, description, arguments, examples, related links, and source.
- [x] Define documentation query types.
- [x] Define documentation result types.
- [x] Add protocol messages for documentation queries and results, or a general command-based equivalent.
- [x] Add server handlers for listing commands.
- [x] Add server handlers for describing one command.
- [x] Add placeholder registry/query shapes for settings, keybindings, modes, extensions, tools, permissions, and API docs.
- [x] Add builtin help commands such as `help.commands` and `help.command`.
- [x] Ensure help commands themselves have command metadata.
- [x] Add a simple client rendering path for documentation results, even if initially text/log based.
- [x] Keep documentation sourced from live server registries.
- [x] Do not make clients scrape markdown files as the primary help source.
- [x] Add tests for command listing documentation.
- [x] Add tests for describing a command.
- [x] Add tests for missing documentation query errors.
- [x] Add tests that help commands are discoverable through the command registry.

Milestone:

```text
The editor can query the server for available command documentation and show it through the command/help path.
```

