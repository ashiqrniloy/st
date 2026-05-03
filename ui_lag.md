# UI Typing Lag Diagnosis And Remediation Plan

## Problem Summary

Typing in the GPUI client is visibly slow even in a fresh editor session. Latency grows as more characters are typed into the first line. Previous work in `text_lag.md` improved server/protocol simulations and some client cache behavior, but the live editor still feels slow when running:

```bash
cargo run -- server
cargo run -- client
```

This plan focuses on the GPUI/client hot path and the end-to-end user-visible key-to-paint path.

## Current Diagnosis

### 1. Normal text input has no local echo

In `src/render/input.rs`, ordinary text input sends the key to the server but does not update local content immediately:

```rust
self.send_key(new_text, Some(new_text.to_string()), false);
```

So visible text appears only after this full round trip:

```text
GPUI input
  -> client IPC thread
  -> Unix socket
  -> server key dispatch
  -> server mutation
  -> server ScenePatch
  -> Unix socket
  -> client IPC thread
  -> GPUI entity update
  -> cx.notify()
  -> GPUI prepaint/paint
```

This makes typing perceptually dependent on server, IPC, queue, and render latency.

### 2. GPUI prepaint still performs O(visible text) work every frame

`src/render/element.rs` still does this in `prepaint()`:

```rust
let content = view.content.clone();
let lines_text: Vec<String> = if content.is_empty() {
    vec![String::new()]
} else {
    content.split('\n').map(str::to_string).collect()
};
```

Every paint can still:

- clone the whole visible text,
- split the whole visible text,
- allocate one `String` per visible line,
- scan content for cursor layout,
- collect cloned `ShapedLine`s into a fresh vector.

The shaped-line cache reduces reshaping unchanged lines, but it does not eliminate whole-content clone/split/scan work.

### 3. Long-line typing still reshapes the entire growing line

The current dirty-line strategy marks one line dirty. For a single long first line, that means every keystroke reshapes the entire growing line:

```rust
shape_line(line_text.clone().into(), ...)
```

GPUI documentation for `WindowTextSystem::shape_line` confirms this shapes a single line. Shaping one dirty line is still O(line length), so a one-line file can remain slow even if shaped-line count is `1`.

### 4. Existing profiler/perf gate does not prove the real issue is fixed

Current local commands pass:

```bash
cargo run -- profiler
cargo run -- perf-gate
```

But the gate reports simulated or incomplete render metrics, e.g. `scene_to_paint_us p50=0.00`, so it is not measuring real GPUI key-to-paint behavior.

The current profiler counts shaped lines, not shaped bytes. For the actual bug, shaped bytes per key is the important metric.

## Architecture Assessment

The current server-backed architecture is not inherently impossible, but it is incomplete for responsive typing because the client lacks optimistic local echo. A server-authoritative editor can remain responsive if:

1. simple local edits are shown immediately and reconciled with the server, and
2. the GPUI render path is incremental down to visible line/chunk granularity.

Without those changes, typing remains round-trip gated and vulnerable to any client/server/render delay.

---

# Phase 0: Build A Real End-To-End Latency Harness

## Acceptance Criteria

- [ ] The editor can measure real key-to-paint latency in the GPUI client.
- [ ] Metrics distinguish GPUI input time, client send time, server receive/mutate time, server send time, client receive/apply time, `prepaint()` duration, `shape_line()` duration, and `paint()` duration.
- [ ] Long-line tests measure shaped bytes, not only shaped lines.
- [ ] Metrics can be enabled during normal client/server runs without changing editor behavior.
- [ ] A repeatable manual or automated runbook exists for reproducing typing-lag numbers.

## TODO

- [ ] Add `ST_PERF_TRACE=1` or equivalent runtime-controlled tracing.
- [ ] Timestamp GPUI input receipt.
- [ ] Timestamp client-to-server send.
- [ ] Timestamp server receive.
- [ ] Timestamp server mutation completion.
- [ ] Timestamp server patch send/enqueue.
- [ ] Timestamp client patch receive.
- [ ] Timestamp `RootView` patch/snapshot apply.
- [ ] Timestamp GPUI `prepaint()` start/end.
- [ ] Timestamp GPUI `paint()` start/end.
- [ ] Record per-line/per-chunk `shape_line()` duration.
- [ ] Record `content_clone_bytes` during render.
- [ ] Record `line_split_count` and `line_split_bytes` during render.
- [ ] Record `shaped_line_count` and `shaped_byte_count` per frame/edit.
- [ ] Record scene/UI queue depth and dropped/coalesced counts.
- [ ] Add a runbook for typing/pasting 100, 1k, 10k, and 100k characters on one line.
- [ ] Add a runbook for 1k short lines.
- [ ] Report p50/p95/p99 real key-to-paint latency.

---

# Phase 1: Add Optimistic Local Echo For Simple Typing And Backspace

## Acceptance Criteria

- [ ] Simple printable insert appears in the GPUI view immediately, before server response.
- [ ] Backspace appears in the GPUI view immediately, before server response.
- [ ] Server remains canonical.
- [ ] Client tracks pending local edits.
- [ ] Server patches confirm or reject local edits.
- [ ] Matching server confirmations do not double-apply already echoed edits.
- [ ] Version mismatch or rejected edits trigger authoritative resync.
- [ ] IME marked-text behavior remains correct.
- [ ] Local echo can be disabled via a debug/config setting.

## TODO

- [ ] Define `PendingLocalEdit` state in the GPUI client/root view.
- [ ] Add a monotonically increasing client edit id for locally echoed edits.
- [ ] Extend text edit protocol with optional `client_edit_id` or equivalent acknowledgement metadata.
- [ ] In `EntityInputHandler::replace_text_in_range`, apply simple non-marked text locally before sending to server.
- [ ] Apply local backspace immediately from the keybinding/action path.
- [ ] Mark locally edited line/chunk dirty immediately.
- [ ] Call `cx.notify()` immediately after local echo.
- [ ] Send the edit to the server after local echo.
- [ ] On matching authoritative server patch, acknowledge/advance confirmed version without reapplying text.
- [ ] On mismatch, drop speculative state and request/apply `ResyncScene`.
- [ ] Add tests proving local insert updates `RootView.content` before server response.
- [ ] Add tests proving matching server patch does not double insert.
- [ ] Add tests proving mismatch triggers resync.
- [ ] Add tests for immediate local backspace.
- [ ] Add tests preserving IME marked-text behavior.

---

# Phase 2: Replace Whole-String Rendering With A Persistent Viewport Line Model

## Acceptance Criteria

- [ ] `prepaint()` does not clone whole visible content.
- [ ] `prepaint()` does not split whole visible content.
- [ ] Cursor line/column lookup is O(log lines) or O(1) for common cursor movement.
- [ ] Dirty-line information comes from persistent client text model metadata.
- [ ] Full `String` access remains available only for IME/query fallback, not the render hot path.
- [ ] Single-character insert does not rebuild all lines.
- [ ] Cursor-only movement does not dirty or rebuild text lines.
- [ ] Selection-only movement does not dirty or rebuild text lines.

## TODO

- [ ] Introduce `VisibleTextModel` owned by `RootView`.
- [ ] Store persistent `Vec<VisibleLine>` rather than splitting `content` during every `prepaint()`.
- [ ] Store per-line text, start char, start byte, char count, byte count, UTF-16 count, dirty flag, and shaped cache state.
- [ ] Update snapshot application to populate `VisibleTextModel` once.
- [ ] Update text patch application to mutate only affected lines.
- [ ] Update newline insertion/removal to splice line metadata incrementally.
- [ ] Replace render-time `content.clone()` with a cheap render snapshot/borrow of line metadata.
- [ ] Replace render-time `content.split('\n')` with iteration over persistent visible lines.
- [ ] Replace cursor line/col scans with line metadata lookup.
- [ ] Replace selection layout scans with line metadata lookup.
- [ ] Preserve full string compatibility for GPUI input handler methods where needed.
- [ ] Add instrumentation proving `content_clone_bytes_per_key == 0` for normal typing render path.
- [ ] Add instrumentation proving `line_split_bytes_per_key == 0` for normal typing render path.

---

# Phase 3: Bound Long-Line Shaping With Chunking Or Horizontal Viewport Awareness

## Acceptance Criteria

- [ ] Typing at column N does not require shaping all bytes from the beginning of a long line when most text is offscreen.
- [ ] Shaped bytes per keystroke are bounded by visible viewport width plus a small context margin.
- [ ] Long-line typing latency stays roughly flat from 100 chars to 100k chars.
- [ ] Cursor x-position remains correct for the default monospace editor path.
- [ ] Complex text correctness has a documented safe fallback.
- [ ] Cursor movement in a long line does not reshape text.
- [ ] Horizontal scrolling invalidates only newly visible chunks/ranges.

## TODO

- [ ] Decide primary strategy: monospace fast path, chunked shaping, horizontal-viewport shaping, or hybrid.
- [ ] Implement default monospace ASCII fast path for cursor x and visible column calculation.
- [ ] Track horizontal scroll/visible column range in the client viewport state.
- [ ] Shape only visible substring plus context margin for long lines.
- [ ] Add line chunk metadata, e.g. fixed-size chunks around 512 bytes/chars.
- [ ] Cache shaped chunks separately from logical lines.
- [ ] Invalidate only affected chunk(s) on insert/delete.
- [ ] Maintain prefix widths or fast column-width calculation for cursor/selection positioning.
- [ ] Add fallback path for complex scripts, ligatures, bidi, or proportional font cases.
- [ ] Add tests for 100k-character line insert with bounded shaped bytes.
- [ ] Add tests for 100k-character line backspace with bounded shaped bytes.
- [ ] Add tests proving cursor movement in a long line does not reshape.
- [ ] Add tests for horizontal scroll changing only visible chunk set.
- [ ] Add tests for non-ASCII fallback correctness.

---

# Phase 4: Remove O(n) Char/Byte/UTF-16 Conversions From The Hot Path

## Acceptance Criteria

- [ ] Patch char-to-byte conversion uses line/chunk metadata instead of scanning from content start.
- [ ] Cursor line/column lookup does not scan whole visible content.
- [ ] UTF-16 conversions for GPUI input are cached or line-scoped.
- [ ] Patch apply time remains bounded from 10k to 100k line length for simple ASCII edits.
- [ ] Multi-byte character behavior remains correct.

## TODO

- [ ] Store per-line `start_char`, `start_byte`, `char_count`, `byte_count`, and `utf16_count`.
- [ ] Implement viewport char index to line lookup via binary search over line starts.
- [ ] Implement line-local char-to-byte conversion.
- [ ] Add ASCII fast path where char index equals byte index.
- [ ] Add UTF-16 prefix metadata per line or per chunk.
- [ ] Replace `selection::char_to_byte_index(&self.content, ...)` in hot patch paths.
- [ ] Replace `layout::line_and_col_for_byte(&content, ...)` hot-path scans.
- [ ] Replace `layout::byte_from_line_col(&content, ...)` hot-path scans.
- [ ] Update `EntityInputHandler` range conversion to use line-scoped metadata where possible.
- [ ] Add tests comparing 10k and 100k patch apply time/growth.
- [ ] Add tests for multi-byte UTF-8 and UTF-16 selection correctness.

---

# Phase 5: Clean Up GPUI Render-Path Hygiene

## Acceptance Criteria

- [ ] `prepaint()` only updates dirty visible lines/chunks.
- [ ] `paint()` only paints cached shapes and simple quads.
- [ ] Tight paint loops do not repeatedly call `self.view.read(cx)`.
- [ ] No fresh `Vec<ShapedLine>` clone of the whole viewport is created every frame.
- [ ] Style/font/theme changes trigger explicit cache invalidation.
- [ ] GPUI input remains installed through `Window::handle_input()` in paint as expected by GPUI docs.

## TODO

- [ ] Replace repeated `self.view.read(cx)` calls in paint loops with a single render snapshot/read.
- [ ] Remove `let content = view.content.clone()` from `prepaint()`.
- [ ] Remove render-time line splitting from `prepaint()`.
- [ ] Avoid collecting all shaped lines into a fresh vector for every frame.
- [ ] Store paint-ready line/chunk references or lightweight snapshot data.
- [ ] Separate cursor/selection quad calculation from text shaping invalidation.
- [ ] Ensure cursor-only updates do not touch text shape cache.
- [ ] Ensure selection-only updates do not touch text shape cache.
- [ ] Ensure font/style/theme change invalidates all relevant shaped caches.
- [ ] Keep `window.handle_input(&focus_handle, ElementInputHandler::new(...), cx)` in paint.
- [ ] Add render instrumentation assertions for clone/split/shaped-byte budgets.

---

# Phase 6: Replace Current Perf Gate With A Real UI-Lag Regression Gate

## Acceptance Criteria

- [ ] The gate fails on the current unfixed GPUI hot path.
- [ ] The gate passes only when real key-to-paint latency and shaped-byte behavior are bounded.
- [ ] Tests cover first-line typing at 100, 1k, 10k, and 100k characters.
- [ ] Tests cover many short lines.
- [ ] Metrics include real key-to-paint p50/p95/p99.
- [ ] Metrics include real `prepaint()` p50/p95/p99.
- [ ] Metrics include real `shape_line()` p50/p95/p99.
- [ ] Metrics include shaped bytes per key.
- [ ] Metrics include content clone bytes and line split bytes per key.

## TODO

- [ ] Add a UI perf command or test mode that drives the GPUI client with scripted input.
- [ ] Add scenario: type to 100 chars on first line.
- [ ] Add scenario: type to 1k chars on first line.
- [ ] Add scenario: type to 10k chars on first line.
- [ ] Add scenario: type to 100k chars on first line.
- [ ] Add scenario: type/edit across 1k short lines.
- [ ] Assert `content_clone_bytes_per_key == 0` on normal typing render path.
- [ ] Assert `line_split_bytes_per_key == 0` on normal typing render path.
- [ ] Assert shaped bytes per key is bounded by viewport text bytes plus margin.
- [ ] Assert key-to-paint p95 stays below an agreed threshold.
- [ ] Assert prepaint p95 stays below an agreed threshold.
- [ ] Assert queue depths remain bounded under rapid input.
- [ ] Make the gate easy to run locally, e.g. `cargo run -- ui-perf-gate`.
- [ ] Document how to interpret failures.

---

# Alternative Architecture Paths

## Path A: Keep Server/Client Architecture And Add Optimistic Local Echo

Recommended path.

### TODO

- [ ] Keep Rust server canonical.
- [ ] Add speculative client local edits.
- [ ] Add acknowledgement/reconciliation protocol.
- [ ] Keep extensions and agents submitting typed requests to Rust.
- [ ] Preserve multi-client correctness through versioned resync.

## Path B: Move Canonical Editing Hot Path Into GPUI Client

Lower latency but conflicts with current server-owned architecture.

### TODO

- [ ] Evaluate whether server ownership remains a hard requirement.
- [ ] Prototype client-owned buffer mutation.
- [ ] Make server a follower/replicator.
- [ ] Reassess extension, multi-client, undo/redo, and command architecture.

## Path C: Single-Process Default With Optional External Server

May reduce IPC latency but does not solve render/shaping costs by itself.

### TODO

- [ ] Prototype same-process server/client mode.
- [ ] Measure latency difference versus Unix socket mode.
- [ ] Still implement persistent line model and bounded shaping.
- [ ] Decide whether operational complexity is justified.

---

# Recommended Work Order

- [ ] Phase 0: Real end-to-end latency harness.
- [ ] Phase 1: Optimistic local echo.
- [ ] Phase 2: Persistent viewport line model.
- [ ] Phase 3: Long-line chunking / horizontal viewport shaping.
- [ ] Phase 4: Metadata-based char/byte/UTF-16 conversions.
- [ ] Phase 5: GPUI render-path cleanup.
- [ ] Phase 6: Real UI-lag regression gate.

## Final Target State

The fixed typing path should look like this:

```text
GPUI input
  -> immediate speculative local model update
  -> mark affected line/chunk dirty
  -> cx.notify()
  -> prepaint updates only affected visible chunk(s)
  -> paint cached text + cursor
  -> server receives typed edit in parallel
  -> server confirms version or requests reconciliation
```

The hot path must avoid:

```text
server round-trip before visible feedback
whole visible text clone per paint
whole visible text split per paint
whole long-line shape per key
whole-content char/byte scans per patch
unmeasured simulated-only perf gates
```
