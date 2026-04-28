# Editor Architecture and Implementation Plan

This document captures the current architecture review, code review findings, recommended improvements, and a step-by-step implementation plan for building an AI-native, infinitely extensible text editor with a Rust core and JavaScript/Deno extension layer.

## Project Vision

The editor should have:

- A robust, fast Rust core.
- A JavaScript/TypeScript extension layer powered by `deno_core`.
- Hot-reloadable user extensions.
- Strong separation between canonical editor state and user customization logic.
- A native desktop UI capable of high-quality rendering.
- A long-term architecture similar in spirit to Emacs: programmable, extensible, introspectable, and customizable.

The central design principle should be:

> Rust owns canonical editor state and correctness-critical systems. JavaScript customizes behavior through typed APIs.

---

# 1. Current State

At the moment, the project has:

- A Rust binary crate.
- `winit` window creation.
- `softbuffer` drawing a solid background.
- A `deno_core` JavaScript runtime running on a separate thread.
- Basic Rust-to-JS event passing using `tokio` channels.
- Basic JS-to-Rust render command passing.
- A window that opens successfully.
- Key presses sent from the UI thread to the JS runtime.

The current code is a good proof of concept for:

- Opening a native window.
- Embedding Deno/V8.
- Sending events from Rust to JS.
- Sending commands from JS back to Rust.

However, it is not yet an editor. The next goal should be to build the minimal editor loop:

```text
input -> editor state mutation -> render scene -> draw frame
```

---

# 2. High-Level Architecture

## 2.1 Recommended System Layers

The editor should be structured around these layers:

```text
Native platform layer
  - windowing
  - input collection
  - clipboard
  - file dialogs
  - OS integration

Editor core
  - buffers
  - cursors/selections
  - undo/redo
  - commands
  - keymaps
  - layout model
  - editor state
  - permissions
  - extension resource tracking

Extension runtime
  - deno_core
  - JS/TS APIs
  - command registration
  - event listeners
  - hot reload
  - extension lifecycle

Rendering layer
  - scene construction
  - text layout
  - glyph rendering
  - decorations
  - theme application

AI layer, later
  - model integrations
  - context extraction
  - code actions
  - agentic workflows
  - tool permissions
```

## 2.2 Ownership Boundary

Rust should own:

- Text buffers.
- Rope/text storage.
- Cursor and selection state.
- Undo/redo history.
- File identity and persistence state.
- Command execution primitives.
- Input normalization.
- Layout and visible viewport state.
- Rendering scene construction.
- Extension resource tracking.
- Permission checks.
- Crash/error isolation.

JavaScript should own:

- User configuration.
- Keybinding declarations.
- Command definitions.
- Editor modes.
- Themes.
- Plugin logic.
- Decorations and overlays.
- AI assistant behavior.
- UI behavior that is not correctness-critical.

JavaScript should not own the canonical buffer contents. Instead, JS should request mutations through typed Rust APIs.

Example future JS API:

```js
editor.commands.register("save-buffer", async () => {
  await editor.buffer.save(editor.currentBuffer());
});

editor.keymap.bind("ctrl+s", "save-buffer");

editor.commands.register("insert-hello", () => {
  editor.buffer.insert(editor.currentBuffer(), editor.cursor.position(), "hello");
});
```

Rust should validate and apply the actual operation.

---

# 3. Architectural Improvements

## 3.1 Do Not Make JS Drive Low-Level Rendering

The current prototype allows JS to send low-level render commands such as:

```rust
RenderCommand::DrawRect { x, y, w, h }
```

This is useful for proving communication, but it should not become the long-term rendering model.

Low-level JS-driven rendering can become:

- Slow.
- Hard to batch.
- Hard to optimize.
- Difficult to make deterministic.
- Difficult to synchronize with editor state.

Recommended long-term model:

```text
JS requests high-level changes:
  - set theme
  - add decoration
  - show popup
  - define mode-line content
  - register command palette item

Rust computes:
  - visible lines
  - glyph layout
  - clipping
  - dirty regions
  - final render scene

Renderer draws:
  - background
  - text
  - selections
  - cursor
  - decorations
  - overlays
```

Short-term, `RenderCommand` is okay, but it should evolve into either:

```rust
EditorCommand
```

or:

```rust
SceneUpdate
```

rather than raw drawing.

## 3.2 Introduce a Command Architecture

The editor should quickly move from ad-hoc events to a command system.

Suggested event type:

```rust
pub enum EditorEvent {
    KeyInput(KeyInputEvent),
    MouseInput(MouseInputEvent),
    WindowResized { width: u32, height: u32 },
    BufferChanged { buffer_id: BufferId },
    CommandRequested { command: String },
    Shutdown,
}
```

Suggested command type:

```rust
pub enum EditorCommand {
    InsertText { buffer_id: BufferId, position: usize, text: String },
    DeleteRange { buffer_id: BufferId, start: usize, end: usize },
    MoveCursor { view_id: ViewId, motion: CursorMotion },
    SaveBuffer { buffer_id: BufferId },
    SetTheme { theme_id: String },
    RequestRedraw,
}
```

This will make the editor:

- Easier to test.
- Easier to extend.
- Easier to connect to JS.
- Easier to inspect and debug.
- Easier to integrate with AI agents later.

## 3.3 Hot Reload Requires Extension Lifecycle Management

Hot reload should not simply re-run JavaScript. Extensions need lifecycle boundaries.

Recommended JS shape:

```js
export function activate(ctx) {
  const disposable = ctx.commands.register("hello", () => {
    ctx.log.info("hello");
  });

  ctx.subscriptions.push(disposable);
}

export function deactivate() {
  // Optional explicit cleanup.
}
```

Rust should track all resources created by an extension:

- Commands.
- Keybindings.
- Event listeners.
- Timers.
- Decorations.
- File watchers.
- Status bar items.
- Panels.
- AI tools.

On reload:

1. Disable old extension instance.
2. Dispose resources registered by that extension.
3. Clear event listeners and timers.
4. Load new JS module.
5. Run `activate()`.
6. Report any activation errors without crashing the editor.

## 3.4 Add Permissions Early

Since the editor is intended to be infinitely extensible, extensions should not get unrestricted access by default.

Eventually support permissions for:

- File read.
- File write.
- Network access.
- Subprocess execution.
- Environment variables.
- Clipboard.
- AI/model access.
- Workspace access.

Example future manifest:

```json
{
  "name": "example-extension",
  "permissions": {
    "fs.read": ["workspace"],
    "fs.write": ["workspace"],
    "network": false,
    "subprocess": false
  }
}
```

Deno/V8 gives useful primitives, but the editor should define its own permission policy around exposed ops.

## 3.5 Use Event-Driven Wakeups Instead of Busy Polling

The current code uses:

```rust
event_loop.set_control_flow(ControlFlow::Poll);
```

This keeps the UI loop running continuously and can waste CPU.

Recommended approach:

```rust
event_loop.set_control_flow(ControlFlow::Wait);
```

Then use `EventLoopProxy` or another wakeup mechanism to notify the UI thread when JS/core has work ready.

This should be fixed soon, because editors spend a lot of time idle.

## 3.6 Treat Rendering as a Separate Subsystem

`softbuffer` is acceptable for the first prototype, but a serious editor needs high-quality text rendering.

Potential future rendering stack:

- `winit` for windowing/input.
- `wgpu`, `vello`, `skia-safe`, or another renderer for drawing.
- `cosmic-text`, `swash`, `rustybuzz`, or similar for text shaping/layout.
- A custom scene representation for editor UI.

Short-term rendering goals:

1. Draw background.
2. Draw text.
3. Draw cursor.
4. Draw selection.
5. Draw line numbers or gutters.
6. Draw simple decorations.

Long-term rendering goals:

1. High-DPI correctness.
2. Font fallback.
3. Ligatures.
4. Bidirectional text.
5. IME support.
6. Incremental redraw.
7. Smooth scrolling.
8. GPU acceleration.

---

# 4. Code Review Findings

## 4.1 `cargo check`

The project currently passes:

```bash
cargo check
```

## 4.2 `cargo clippy`

The project currently fails:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

The current failures are minor collapsible `if` warnings in the keyboard input handling.

Current shape:

```rust
if key_event.state == ElementState::Pressed {
    if let Key::Character(ch) = &key_event.logical_key {
        if let Some(c) = ch.chars().next() {
            println!("UI thread: key '{}'", c);
            let _ = self.ui_tx.send(UiEvent::KeyPress(c));
        }
    }
}
```

This can be collapsed or rewritten when the input event type is improved.

## 4.3 Current `UiEvent` Is Too Small

Current type:

```rust
pub enum UiEvent {
    KeyPress(char),
}
```

This loses important input information:

- Modifier keys.
- Physical key.
- Logical key.
- Repeat status.
- Special keys.
- IME/composition state.
- Dead keys.
- Alt/meta/control combinations.

Recommended future type:

```rust
pub struct KeyInputEvent {
    pub logical_key: String,
    pub physical_key: String,
    pub text: Option<String>,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub repeat: bool,
}
```

Then:

```rust
pub enum EditorEvent {
    KeyInput(KeyInputEvent),
    WindowResized { width: u32, height: u32 },
    CloseRequested,
}
```

## 4.4 Render Commands Are Received But Ignored

The current code receives render commands:

```rust
while let Ok(command) = self.ui_rx.try_recv() {
    println!("UI thread: render command from JS: {:?}", command);
    if let Some(window) = self.window.as_ref() {
        window.request_redraw();
    }
}
```

But `RedrawRequested` only clears the background:

```rust
buf.fill(0x00_1e_1e_2e);
```

Immediate improvement:

- Store render commands or scene state in `App`.
- Draw them during `RedrawRequested`.

Better improvement:

- Replace raw `RenderCommand` with high-level editor state and scene generation.

## 4.5 `RenderCommand::DrawRect` Is Incomplete

Current type:

```rust
pub enum RenderCommand {
    DrawRect { x: f32, y: f32, w: f32, h: f32 },
}
```

It lacks:

- Color.
- Coordinate semantics.
- Z-order.
- Clipping.
- Target surface/view.

If kept temporarily, change to:

```rust
pub enum RenderCommand {
    DrawRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
    },
}
```

## 4.6 Unbounded Channels Can Grow Without Limit

Current code uses:

```rust
tokio_mpsc::unbounded_channel()
```

This is acceptable for a prototype, but risky long-term.

Potential problems:

- Broken extension floods render commands.
- Mouse move events accumulate.
- Redraw requests pile up.
- Memory grows without limit.

Recommended future approach:

- Use bounded channels for most message types.
- Coalesce high-frequency messages.
- Keep only latest mouse position or latest resize.
- Deduplicate redraw requests.

## 4.7 Deno Thread Lifecycle Is Not Managed

Current code spawns the Deno thread but does not retain its handle:

```rust
thread::spawn(move || {
    runtime.block_on(async {
        // ...
    });
});
```

Issues:

- No clean shutdown protocol.
- No thread join.
- No extension cleanup.
- No graceful cancellation.

Recommended future behavior:

1. UI receives close request.
2. UI sends shutdown message to runtime/core.
3. JS runtime receives cancellation/shutdown event.
4. Extensions deactivate.
5. Runtime exits event loop.
6. Main thread joins runtime thread.
7. Process exits cleanly.

## 4.8 JavaScript Is Embedded in Rust

Current code embeds JS in a string:

```rust
let js_code = r#"
    const { core } = Deno;
    // ...
"#;
```

This is okay for the first proof of concept, but hot reload requires JS files on disk.

Recommended structure:

```text
runtime/
  bootstrap.js
  editor_api.js
extensions/
  init.js
```

The Rust runtime should load these files from disk and eventually support reload.

## 4.9 Resize Events Are Not Exposed

The current redraw code reads window size, but JS/editor logic is not informed about resize events.

Add an event:

```rust
EditorEvent::WindowResized { width, height }
```

This will matter once layout, panels, and decorations exist.

## 4.10 Too Many `unwrap()` Calls

The prototype uses several `unwrap()` calls:

```rust
event_loop.create_window(attrs).unwrap();
softbuffer::Context::new(window.clone()).unwrap();
surface.buffer_mut().unwrap();
buf.present().unwrap();
```

This is fine for the initial setup, but the editor core should eventually use structured errors.

Recommended approach:

- Keep `unwrap()` only for truly unrecoverable initialization failures.
- Use clear error messages for setup failures.
- Avoid panicking during normal rendering.
- Log render failures and attempt recovery where possible.

## 4.11 Tokio Dependency Is Too Broad

Current dependency:

```toml
tokio = { version = "1.52.1", features = ["full"] }
```

This is convenient but pulls in more than needed.

Later, reduce to specific features, for example:

```toml
tokio = { version = "1", features = ["rt", "sync", "macros", "time"] }
```

Only do this after the runtime shape stabilizes.

---

# 5. Recommended Implementation Plan

## Phase 1: Clean Up the Prototype

Goal: make the current proof of concept cleaner and easier to extend.

Tasks:

- [ ] Fix clippy warnings.
- [ ] Replace nested keyboard `if` statements with cleaner matching.
- [ ] Add `color` to `RenderCommand::DrawRect` if keeping it temporarily.
- [ ] Store received render commands or render scene in `App`.
- [ ] Actually draw rectangles during `RedrawRequested`.
- [ ] Add basic error messages instead of blind `unwrap()` where easy.
- [ ] Add comments documenting the intended Rust/JS ownership boundary.

Suggested result:

```text
Window opens.
Key press goes to JS.
JS sends DrawRect with color.
Rust draws that rectangle.
```

## Phase 2: Split the Code Into Modules

Goal: avoid letting `main.rs` become the whole editor.

Suggested structure:

```text
src/
  main.rs
  app.rs
  editor.rs
  events.rs
  render.rs
  js_runtime.rs
```

Responsibilities:

```text
main.rs
  - startup wiring
  - channel creation
  - event loop creation

app.rs
  - winit ApplicationHandler
  - window lifecycle
  - native event collection

editor.rs
  - EditorState
  - buffers
  - cursors
  - command application

events.rs
  - EditorEvent
  - EditorCommand
  - KeyInputEvent
  - shared message types

render.rs
  - scene representation
  - softbuffer drawing
  - color helpers

js_runtime.rs
  - deno_core setup
  - op registration
  - JS event loop
```

Suggested result:

```text
Same behavior as current app, but with clean module boundaries.
```

## Phase 3: Add Minimal Editor State

Goal: turn the prototype into the smallest real editor.

Start with:

```rust
pub struct EditorState {
    pub buffer: String,
    pub cursor: usize,
}
```

Implement:

- [ ] Insert character.
- [ ] Backspace.
- [ ] Enter/newline.
- [ ] Move cursor left.
- [ ] Move cursor right.
- [ ] Clamp cursor to valid positions.

Then evolve toward:

```rust
pub struct EditorState {
    pub buffers: Vec<Buffer>,
    pub active_buffer: BufferId,
    pub views: Vec<View>,
    pub active_view: ViewId,
}
```

Suggested result:

```text
Typing mutates Rust EditorState.
Backspace works.
Cursor position is tracked.
```

## Phase 4: Improve Input Events

Goal: stop losing important keyboard information.

Replace:

```rust
UiEvent::KeyPress(char)
```

with something closer to:

```rust
pub enum EditorEvent {
    KeyInput(KeyInputEvent),
    WindowResized { width: u32, height: u32 },
    CloseRequested,
}
```

Implement `KeyInputEvent` with:

- [ ] Logical key.
- [ ] Physical key.
- [ ] Text, if available.
- [ ] Ctrl modifier.
- [ ] Alt modifier.
- [ ] Shift modifier.
- [ ] Meta/super modifier.
- [ ] Repeat flag.

Suggested result:

```text
Special keys and modifier combinations can be represented.
```

## Phase 5: Render Text

Goal: visible text editing.

Minimum viable rendering:

- [ ] Clear background.
- [ ] Render buffer text.
- [ ] Render cursor.
- [ ] Handle newlines.
- [ ] Handle basic scrolling later.

If text rendering is too much immediately, a temporary placeholder is acceptable:

- Draw one rectangle per character.
- Draw a differently colored cursor rectangle.

But do not stay at rectangle-only rendering for long.

Potential text rendering libraries to evaluate later:

- `cosmic-text`
- `swash`
- `rustybuzz`
- `fontdue`
- `vello`
- `wgpu`
- `skia-safe`

Suggested result:

```text
Typing characters makes characters visible in the window.
Cursor is visible.
```

## Phase 6: Add a Command System

Goal: move behavior behind explicit commands.

Introduce:

```rust
pub enum EditorCommand {
    InsertText { text: String },
    Backspace,
    MoveCursorLeft,
    MoveCursorRight,
    SaveBuffer,
    RequestRedraw,
}
```

Implement:

- [ ] Command application in Rust.
- [ ] Unit tests for command behavior.
- [ ] Key events translated into commands.
- [ ] JS able to request commands.

Suggested result:

```text
Input produces commands.
Commands mutate EditorState.
EditorState produces render scene.
```

## Phase 7: Make JS Register Commands and Keybindings

Goal: make the editor programmable.

Expose JS APIs like:

```js
editor.commands.register("insert-date", () => {
  editor.buffer.insert(new Date().toISOString());
});

editor.keymap.bind("ctrl+d", "insert-date");
```

Rust-side resources to track:

- Registered commands.
- Registered keybindings.
- Owning extension/module.

Initial implementation can be simple:

- One JS runtime.
- One global command registry.
- No extension isolation yet.

Suggested result:

```text
A JS file can register a command.
A keybinding can invoke that command.
The command can request a Rust buffer mutation.
```

## Phase 8: Load JavaScript From Disk

Goal: prepare for hot reload.

Create:

```text
runtime/
  bootstrap.js
  editor_api.js
extensions/
  init.js
```

Implement:

- [ ] Load JS source from disk.
- [ ] Execute bootstrap file.
- [ ] Report JS syntax/runtime errors clearly.
- [ ] Keep editor alive if JS fails to load.

Suggested result:

```text
Editor starts by loading runtime/bootstrap.js from disk.
JS changes no longer require recompiling Rust.
```

## Phase 9: Add Manual Hot Reload

Goal: prove extension reload.

Start with a manual keybinding, for example:

```text
Ctrl+R reloads JS runtime/extensions.
```

Implement:

- [ ] Shutdown old JS runtime or deactivate old extension context.
- [ ] Dispose commands/keybindings/listeners from old runtime.
- [ ] Load JS again from disk.
- [ ] Re-register commands/keybindings.
- [ ] Display reload errors without crashing.

Suggested result:

```text
Edit JS file.
Press reload key.
New command behavior takes effect.
```

## Phase 10: Add Proper Extension Lifecycle

Goal: support real extensions.

Define extension API:

```js
export function activate(ctx) {
  ctx.subscriptions.push(
    ctx.commands.register("hello", () => ctx.log.info("hello"))
  );
}

export function deactivate() {
}
```

Rust should provide:

- Extension ID.
- Resource registry.
- Disposable handles.
- Activation errors.
- Deactivation errors.
- Reload behavior.

Suggested result:

```text
Extensions can be loaded, unloaded, and reloaded without leaking commands or listeners.
```

## Phase 11: Replace Busy Polling

Goal: reduce idle CPU usage.

Current:

```rust
event_loop.set_control_flow(ControlFlow::Poll);
```

Target:

```rust
event_loop.set_control_flow(ControlFlow::Wait);
```

Use a wakeup mechanism such as `EventLoopProxy` so non-UI threads can notify the UI thread.

Implement:

- [ ] Custom user event type for winit.
- [ ] EventLoopProxy passed to runtime/core where needed.
- [ ] JS/core sends wake event when redraw/work is ready.
- [ ] UI drains pending messages after wake.

Suggested result:

```text
Editor uses little/no CPU while idle.
JS/core can still wake the UI thread.
```

## Phase 12: Add Real Buffer Data Structures

Goal: prepare for large files.

A plain `String` is okay at first, but a serious editor should use a text data structure designed for editing.

Evaluate:

- `ropey`
- custom gap buffer
- custom piece table
- custom rope

Recommended likely path:

- Start with `String`.
- Add tests around buffer operations.
- Move to `ropey` or a custom rope when needed.

Implement eventually:

- [ ] Buffer IDs.
- [ ] Multiple buffers.
- [ ] Dirty state.
- [ ] File path association.
- [ ] Encoding assumptions.
- [ ] Line/column mapping.
- [ ] Efficient insertion/deletion.
- [ ] Undo/redo.

## Phase 13: Add Undo/Redo

Goal: make editing usable.

Recommended design:

- Commands generate edit transactions.
- Transactions are applied to buffers.
- Transactions are stored in undo stack.
- Related edits can be grouped.

Example:

```rust
pub struct EditTransaction {
    pub buffer_id: BufferId,
    pub edits: Vec<TextEdit>,
    pub before_cursor: CursorState,
    pub after_cursor: CursorState,
}
```

Implement:

- [ ] Undo stack.
- [ ] Redo stack.
- [ ] Insert undo.
- [ ] Delete undo.
- [ ] Cursor restoration.
- [ ] Transaction grouping.

## Phase 14: Add File I/O

Goal: edit real files.

Implement commands:

- [ ] Open file.
- [ ] Save file.
- [ ] Save as.
- [ ] New buffer.
- [ ] Close buffer.

Design considerations:

- File permissions.
- Unsaved changes.
- Encoding.
- Large file loading.
- External file changes.
- Error reporting.

## Phase 15: Add AI-Native Architecture

Do this after the basic editor works.

AI should integrate with the same command and permission system.

Potential AI concepts:

- AI commands are normal commands.
- AI tools are registered capabilities.
- AI can request edits, but Rust applies them as transactions.
- AI actions should be previewable/reversible.
- AI should operate on structured editor context, not raw global state.

Example future API:

```js
editor.ai.registerTool("replace-selection", async ({ text }) => {
  editor.buffer.replaceSelection(text);
});
```

Security considerations:

- AI tools need permission checks.
- File writes should be explicit.
- Shell/subprocess access should be gated.
- Large context extraction should be controlled.

---

# 6. Suggested Near-Term Milestone

The next concrete milestone should be:

```text
Open window.
Type characters.
Characters appear in the editor.
Backspace works.
Cursor moves left/right.
JS receives key events.
JS can request a command.
Rust applies command to EditorState.
```

This validates the important architecture without overbuilding.

Recommended immediate order:

1. Split code into modules.
2. Add `EditorState` with a `String` buffer and cursor.
3. Add `EditorEvent` and `EditorCommand`.
4. Make typed key input produce commands.
5. Render visible buffer contents.
6. Replace `ControlFlow::Poll` with event-driven wakeups.
7. Load JS from disk.
8. Add manual hot reload.
9. Add extension lifecycle and resource cleanup.

---

# 7. Testing Strategy

Add tests as soon as editor state exists.

Start with pure Rust unit tests for:

- Insert text.
- Delete/backspace.
- Cursor movement.
- Newline insertion.
- Cursor clamping.
- Command application.
- Undo/redo once implemented.

Example target:

```rust
#[test]
fn insert_text_moves_cursor() {
    let mut editor = EditorState::default();
    editor.apply(EditorCommand::InsertText { text: "a".into() });
    assert_eq!(editor.buffer, "a");
    assert_eq!(editor.cursor, 1);
}
```

Keep as much editor logic as possible independent from `winit` and `deno_core` so it can be tested easily.

---

# 8. Key Design Rules

1. Rust owns canonical editor state.
2. JS customizes behavior through typed APIs.
3. Extensions must be disposable/reloadable.
4. Rendering should be driven by editor state, not ad-hoc JS draw calls.
5. Use explicit commands for all meaningful mutations.
6. Avoid unbounded queues for high-frequency events.
7. Avoid busy polling.
8. Treat permissions as a core architectural concern.
9. Keep editor logic testable without a window.
10. Add AI features through the same command/permission/edit transaction model.

---

# 9. Immediate TODO Checklist

- [ ] Refactor `src/main.rs` into modules.
- [ ] Define `EditorState`.
- [ ] Define `EditorEvent`.
- [ ] Define `EditorCommand`.
- [ ] Replace `UiEvent::KeyPress(char)` with richer key input.
- [ ] Implement insert/backspace/cursor movement.
- [ ] Render current buffer text or placeholder character rectangles.
- [ ] Add cursor rendering.
- [ ] Make render commands affect actual drawing if keeping them temporarily.
- [ ] Fix clippy warnings.
- [ ] Add unit tests for editor state mutations.
- [ ] Replace `ControlFlow::Poll` with `ControlFlow::Wait` plus wakeups.
- [ ] Move JS code out of Rust string and into files.
- [ ] Add manual JS reload.
- [ ] Add extension lifecycle management.

---

# 10. Long-Term TODO Checklist

- [ ] Multiple buffers.
- [ ] Multiple views/windows/splits.
- [ ] Real text rendering.
- [ ] Font shaping and fallback.
- [ ] High-DPI rendering.
- [ ] Selection support.
- [ ] Mouse support.
- [ ] Clipboard support.
- [ ] Undo/redo.
- [ ] File open/save.
- [ ] Command palette.
- [ ] Status line/modeline.
- [ ] Minibuffer or command input.
- [ ] Extension manifests.
- [ ] Extension permissions.
- [ ] Extension marketplace/local packages.
- [ ] AI tool API.
- [ ] AI edit previews.
- [ ] Sandboxed subprocess/network access.
- [ ] Crash/error reporting for extensions.
- [ ] Plugin profiling and diagnostics.

---

# 11. Final Recommendation

The project should now move from infrastructure proof-of-concept to editor-core proof-of-concept.

Do not start with a large plugin system yet. First make a tiny editor that can:

- Store text.
- Mutate text through commands.
- Render text.
- Let JS influence commands/keybindings.

Once that works, hot reload and extension lifecycle will have something meaningful to reload.
