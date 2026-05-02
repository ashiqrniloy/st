## Phase 7: Configuration Foundation

Goal: establish the JavaScript-first configuration foundation before adding more user-visible behavior, extensions, tools, modes, and settings.

- [x] Define configuration architecture in code using `config.md` as the rationale.
- [x] Define default config locations, including `~/.config/st/init.js`, without making them the only supported locations.
- [x] Add a typed Rust-side settings/config registry shape.
- [x] Add setting descriptors with id, title, description, type, default, valid values/range, examples, and reload/restart behavior.
- [x] Ensure setting descriptors integrate with the self-documentation metadata model.
- [x] Add configuration source tracking, such as default, CLI, init.js, YAML, extension, or runtime override.
- [x] Define precedence rules between defaults, CLI options, init.js, YAML-loaded values, and runtime changes.
- [x] Add a JavaScript-facing config API shape for future `init.js` support.
- [x] Add YAML loading as declarative data support, not as the primary behavior engine.
- [x] Add path expansion helpers for `~`, environment variables where appropriate, and relative paths.
- [x] Add configuration validation and clear diagnostics.
- [x] Add a way to report config load errors without crashing the server when possible.
- [x] Add extension/tool directory configuration concepts without enforcing one directory structure.
- [x] Add tests for setting descriptor validation.
- [x] Add tests for configuration precedence.
- [x] Add tests for invalid configuration diagnostics.
- [x] Link this phase to `config.md` and the `st-config` skill as detailed guidance.

Milestone:

```text
The project has a documented, typed, self-documenting configuration foundation before additional behavior becomes hard-coded.
```

