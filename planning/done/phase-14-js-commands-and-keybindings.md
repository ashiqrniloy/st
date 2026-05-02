## Phase 14: JS Commands And Keybindings

Goal: make the editor programmable while keeping Rust in control of command dispatch.

Acceptance criteria:

- Key input is normalized into Rust-owned key chords and resolved through a Rust-owned keymap before text insertion.
- Rust builtin commands execute without invoking Deno.
- JS commands and keybindings can be registered with metadata and ownership tracking.
- JS command requests are validated by Rust before mutating editor state.

Test plan:

- Add key normalization and keymap-resolution tests for printable, modified, and multi-key chords.
- Add dispatch tests proving Rust builtin commands do not invoke Deno and JS commands do.
- Add registration/ownership tests for JS commands and keybindings.
- Add validation tests proving malformed or unauthorized JS command requests cannot mutate editor state.

Design rule:

```text
Deno does not receive every key by default.
Deno registers capabilities with Rust.
Rust stores the active command registry and keymap.
Rust decides what each key means.
```

Target extension API shape:

```js
editor.commands.register("insert-date", () => {
  editor.commands.execute("insert-text", new Date().toISOString());
});

editor.keymap.bind("ctrl d", "insert-date");
```

Target registration flow:

```text
Extension activates in Deno
Deno registers command with Rust server
Deno registers keybinding with Rust server
Rust stores keybinding as owned extension resource
```

Target input dispatch flow:

```text
Client sends KeyInputEvent to server
Server normalizes it into a KeyChord
Server checks Rust-owned keymap
If bound to Rust builtin command: Rust handles it directly
If bound to JS command: Rust invokes Deno command handler
If unbound printable text: Rust applies InsertText directly
If unhandled special key: ignore or pass to explicit listeners later
```

Suggested Rust-side structures:

```rust
pub enum CommandHandler {
    RustBuiltin(EditorCommand),
    JsCommand { extension_id: ExtensionId, command_id: String },
}

pub struct Keymap {
    bindings: HashMap<KeyChord, CommandHandler>,
}
```

Implementation tasks:

- [x] Define `KeyChord` normalized from `KeyInputEvent`.
- [x] Implement modified-key chord handling (Ctrl/Alt/Meta and multi-key chords) through Rust keymap dispatch.
- [x] Define command registry structure in Rust server.
- [x] Define keymap structure in Rust server.
- [x] Define `CommandHandler::RustBuiltin`.
- [x] Define `CommandHandler::JsCommand`.
- [x] Expose command registration API to JS.
- [x] Expose keybinding registration API to JS.
- [x] Track command ownership by extension/runtime.
- [x] Store JS-registered keybindings in Rust-owned keymap.
- [x] Resolve incoming key input through Rust keymap before text insertion.
- [x] Invoke Deno only for keybindings/commands registered by JS.
- [x] Allow JS command to request a typed `EditorCommand`.
- [x] Validate and apply requested `EditorCommand` in Rust server.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

