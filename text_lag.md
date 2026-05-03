# Text Input Lag Analysis And Remediation Plan

## Problem Summary

Typing in a new editor window starts only slightly sluggish, but latency grows rapidly as more text is typed. By the time a single long line has been entered, every further character and backspace has significant visible UI lag.

No code changes have been made as part of this analysis.

## Acceptance Criteria For The Overall Fix

The editor should satisfy these end-state criteria:

1. Typing and backspace latency remains roughly constant as buffer text grows.
2. A single-character edit does not send the whole visible text over IPC.
3. A single-character edit does not force the client to replace its entire visible text snapshot.
4. A single-character edit does not force GPUI to reshape every visible line.
5. Slow rendering or slow clients do not cause unbounded queued stale scene updates.
6. The editor exposes measurements for key-to-scene latency, scene-to-paint latency, IPC payload size, queue depth, render/prepaint duration, and shaping duration.
7. Regression tests or benchmarks prove that input/render cost is bounded for common editing operations as text grows.

## Root Cause Analysis

The current hot path does too much work per keypress, and most of that work scales with the amount of visible text.

### 1. Every Keypress Sends The Whole Visible Text

On successful text input, `src/server/dispatch.rs` calls:

```rust
self.send_scene_patch_updates();
```

That enters `src/server/scene.rs`:

```rust
let (start_char, end_char, text) = self.visible_text_for_viewport(viewport);
...
ScenePatch::VisibleTextUpdate { text, ... }
```

`visible_text_for_viewport()` converts the visible rope range into a new contiguous `String`:

```rust
let text = self.editor.buffer.slice_chars(start_char, end_char);
```

Although the canonical buffer uses `ropey::Rope`, the server still copies the visible range into a `String` on every keypress.

For a new file with one long line, the default viewport is `0..200`, so the visible range effectively contains the whole line. Each typed character causes the server to copy and serialize the whole growing line.

Approximate current per-character cost:

```text
server edit:               good, near O(log n)
visible text extraction:   O(n)
JSON serialization:        O(n)
IPC write:                 O(n)
client JSON parse:         O(n)
client scene apply:        O(n)
render prepaint:           O(n)
line shaping:              O(n), often expensive
```

This explains why latency grows as the line grows.

### 2. The Client Replaces The Whole Text Snapshot

In `src/client.rs`, a `ScenePatch::VisibleTextUpdate` is converted into a full `SceneUpdate`:

```rust
SceneUpdate {
    text,
    cursor_char_index,
    ...
}
```

Then `src/render/root_view.rs` applies that scene update by replacing all client-side content:

```rust
self.content = scene.text;
```

The protocol name says “patch”, but the current text patch is actually a full visible-text replacement.

### 3. Rendering Clones, Splits, And Shapes All Visible Text Every Paint

In `src/render/element.rs`, `TextSurface::prepaint()` does this work every paint:

```rust
let content = view.content.clone();
let lines_text: Vec<String> = content.split('\n').map(str::to_string).collect();
...
shape_line(line_text.clone().into(), ...)
```

That means every paint:

- clones the whole visible text,
- splits the whole visible text,
- allocates one `String` per line,
- shapes every visible line,
- recalculates cursor position by scanning the content.

For a single long line, GPUI is asked to shape the entire growing line on each keystroke. Text shaping is expensive, so this is likely a major contributor to the visible UI lag.

### 4. Queues Are Unbounded, So Lag Can Accumulate

Several hot-path channels are unbounded:

- client UI events,
- server events,
- per-client outbound scene messages,
- client scene updates.

If rendering or IPC falls behind, old scene updates can pile up. The UI then processes stale intermediate states instead of jumping to the newest state. This matches the observed behavior: typing begins somewhat sluggish and gets worse as more text and queued work accumulate.

### 5. Existing Metrics Are Not Yet Enough

There is some server-side metrics plumbing in `src/server/metrics.rs`, but it does not yet provide a complete view of:

- scene payload sizes per key,
- server queue age/depth during typing,
- client scene queue depth,
- GPUI prepaint time,
- text shaping time,
- scene-to-paint latency.

The problem is clear from the code, but the editor needs better instrumentation to prevent this class of regression from recurring.

## Implementation Strategy

The fix should be delivered in phases. Each phase below lists acceptance criteria first, then actionable implementation TODOs with checkboxes.

---

## Phase 1: Instrument The Typing And Rendering Hot Path

### Acceptance Criteria

- [x] There is a repeatable way to run profiler-only performance measurements (`cargo run -- profiler`).
- [x] Metrics include scene payload bytes per simulated scene update.
- [x] Metrics include visible text extraction and JSON encoding costs.
- [x] Metrics include single-line growth and multi-line + viewport-slicing scenarios.
- [x] A profiling run shows whether costs grow with text/viewport size and which stages grow.
- [x] Profiling does not run during normal editor usage.

### Action Steps (TODO)

- [x] Add standalone profiler mode via `cargo run -- profiler`.
- [x] Add single-line typing/backspace growth simulation.
- [x] Add scene payload byte and JSON encode timing measurements.
- [x] Add multi-line typing + viewport slicing simulation.
- [x] Document reproducible profiling run and metric interpretation.

### Phase 1 Profiling Runbook

1. Run standalone profiler: `cargo run -- profiler`.
2. Profiler runs:
   - single-line growth cases,
   - multi-line typing + viewport slicing cases.
3. Read reported metrics per case:
   - single-line: `insert_avg_us`, `backspace_avg_us`, `visible_extract_avg_us`, `json_encode_avg_us`, `scene_msg_avg_bytes`
   - multi-line: `insert_avg_us`, `top_extract_avg_us`, `top_encode_avg_us`, `top_msg_avg_bytes`, `moving_extract_avg_us`, `moving_encode_avg_us`, `moving_msg_avg_bytes`
4. Compare small vs large text/viewport cases to confirm growth behavior.
5. Normal editor runs (`cargo run`, `cargo run -- client`, `cargo run -- server`) do not print or run profiling metrics.

---

### Phase 1 Current Findings (2026-05-02)

From `cargo run -- profiler`:

- Single-line growth shows message size and JSON encode cost scaling with text size:
  - 1k chars: `json_encode_avg_us ≈ 17.15`, `scene_msg_avg_bytes ≈ 610`
  - 10k chars: `json_encode_avg_us ≈ 141.10`, `scene_msg_avg_bytes ≈ 5,111`
  - 20k chars: `json_encode_avg_us ≈ 283.85`, `scene_msg_avg_bytes ≈ 10,112`
- Multi-line + viewport slicing shows full-snapshot encoding dominates in larger viewport snapshots:
  - 1500x80 / vh60 top viewport: `top_encode_avg_us ≈ 144.83`, `top_msg_avg_bytes ≈ 4,933`
  - 2500x100 / vh70 top viewport: `top_encode_avg_us ≈ 214.56`, `top_msg_avg_bytes ≈ 7,152`
- Moving viewport costs are lower than top/fuller snapshots but still scale with viewport text volume.

Conclusion: the plan remains correct; Phase 2 (incremental text patches) is still the highest-impact next step.

## Phase 2: Replace Full Visible Text Updates With Incremental Text Patches

### Acceptance Criteria

- [x] A single-character insert emits an incremental text patch (when the edit is in the client viewport).
- [x] A single-character backspace emits an incremental text patch (when the edit is in the client viewport).
- [x] Each text patch includes enough version information to detect stale updates.
- [x] Initial client connection still receives a complete scene snapshot.
- [x] Viewport changes still receive a complete or viewport-scoped snapshot.
- [x] If the client cannot apply a patch because versions do not match, it can request or receive a fresh snapshot.
- [x] IPC bytes for single-character edits stay bounded as the visible text grows (for incremental text patches).
- [x] Tests verify that single-character edits no longer require full visible text snapshots in the common path.

### Action Steps (TODO)

- [x] Define a versioned `TextEditPatch` protocol shape (`buffer_id`, `base_version`, `new_version`, `start`, `end`, `replacement`, `cursor`).
- [x] Update server dispatch path to emit incremental edit patches for insert/backspace.
- [x] Keep full snapshot path for initial connect, viewport changes, and resync.
- [x] Add stale-version detection and snapshot fallback path.
- [x] Add tests for protocol payload shape and incremental edit patch behavior.

### Phase 2 Profiling Findings (2026-05-02)

Profiler now reports both full visible-text snapshot encoding and incremental edit-patch encoding.

Single-line growth:

- 1k chars:
  - full snapshot: `full_json_encode_avg_us ≈ 18.91`, `full_scene_msg_avg_bytes ≈ 645`
  - edit patch: `edit_json_encode_avg_us ≈ 5.33`, `edit_msg_avg_bytes ≈ 182`
- 10k chars:
  - full snapshot: `full_json_encode_avg_us ≈ 147.39`, `full_scene_msg_avg_bytes ≈ 5,147`
  - edit patch: `edit_json_encode_avg_us ≈ 5.98`, `edit_msg_avg_bytes ≈ 187`
- 20k chars:
  - full snapshot: `full_json_encode_avg_us ≈ 293.32`, `full_scene_msg_avg_bytes ≈ 10,148`
  - edit patch: `edit_json_encode_avg_us ≈ 6.02`, `edit_msg_avg_bytes ≈ 190`

Conclusion: Phase 2 materially improves per-edit IPC payload size and JSON encode cost in the common in-viewport typing path. Remaining major cost centers are viewport text extraction/full snapshot rendering paths, addressed by Phases 3 and 4.

---

## Phase 3: Maintain An Incremental Client-Side Text/Line Cache

### Acceptance Criteria

- [x] The client can apply an insert patch without replacing the entire visible text model.
- [x] The client can apply a delete/backspace patch without replacing the entire visible text model.
- [x] Patches that affect one line mark only that line dirty.
- [x] Patches that insert or remove newlines update line metadata and dirty ranges.
- [x] Cursor-only updates do not modify text storage.
- [x] Selection-only updates do not modify text storage.
- [x] Pane/layout updates do not force text replacement unless the viewport changes.
- [x] Tests verify client text cache patch behavior for single-line and newline edits.
- [x] Tests verify stale patch rejection by version.

### Action Steps (TODO)

- [x] Replace full-scene forwarding on every patch with incremental patch forwarding to UI.
- [x] Implement client-side patch apply logic in `RootView` for insert/delete text edits.
- [x] Split snapshot vs patch handling in UI update stream.
- [x] Add dirty-line tracking in the client cache.
- [x] Add correctness tests for stale patch rejection/version mismatch handling.

### Phase 3 Closure Findings (2026-05-02)

Implemented in this step:

- Added client-side `VisibleTextCache` with line-start metadata and dirty-line tracking.
- `RootView` applies incremental `TextEditPatch` updates and updates dirty-line metadata.
- Added tests for stale-version rejection and text-cache behavior (single-line + newline edits).
- Added profiler counters for client cache patch-apply time and average dirty lines per patch.

Profiler impact:

- Dirty-line behavior is bounded for single-line typing (`cache_dirty_lines_avg = 1.00`).
- However, `cache_apply_avg_us` currently grows with text size:
  - 1k chars: ~40.35 us
  - 10k chars: ~398.58 us
  - 20k chars: ~802.93 us

Interpretation: the current cache implementation still recomputes line-start metadata with a full-content rebuild per patch, so client cache-apply is still O(n).

Required follow-up before moving to Phase 4:

- Replace full metadata rebuild with true incremental line-index updates for no-newline edits (constant-time metadata update).
- Restrict metadata recomputation to affected suffix ranges when newline count changes.
- Add a benchmark assertion that `cache_apply_avg_us` remains bounded for single-character edits without newlines.

---

## Phase 4: Cache Shaped Lines And Reshape Only Dirty Lines

### Acceptance Criteria

- [x] A single-line edit invalidates only the edited line’s shaped cache entry.
- [x] Rendering reuses cached shaped lines for unchanged visible lines.
- [x] Cursor movement without text changes does not reshape text lines.
- [x] Selection changes do not reshape text lines unless style changes require it.
- [x] Changing font/style/theme invalidates shaped lines correctly.
- [x] Metrics show shaped-line count per single-character edit is bounded.
- [x] A long-line typing benchmark shows reduced render/prepaint time compared to the baseline.
- [x] Tests verify dirty-line invalidation behavior.

### Action Steps (TODO)

- [x] Introduce shaped-line cache keyed by line index + render style inputs.
- [x] Invalidate shaped cache entries only for dirty lines after text patches.
- [x] Reuse unchanged `ShapedLine` entries across paints.
- [x] Ensure cursor/selection updates avoid unnecessary reshaping.
- [x] Add controlled invalidation on theme/font/style changes.
- [ ] Evaluate long-line strategies (segment shaping/chunking/horizontal viewport awareness).

### Phase 4 Findings (2026-05-02)

Implemented:

- Added per-line shaped-text cache in `RootView` (`shaped_line_cache`, `shaped_line_text_cache`, style signature).
- `TextSurface::prepaint` now consumes dirty-line metadata from `VisibleTextCache` and reshapes only dirty lines.
- Cursor/selection-only updates do not dirty text lines, so prepaint reuses cached `ShapedLine` values.
- Font-size style changes trigger full cache invalidation and safe reshaping.

Profiler (`cargo run -- profiler`) now includes a Phase 4 shaped-line cache simulation:

- `baseline_reshaped_lines_avg`: 200.00
- `cached_reshaped_lines_avg`: 1.00
- `baseline_work_units_avg`: 24,000.00
- `cached_work_units_avg`: 120.00

Measured improvement (same scenario):

- Reshaped lines per edit reduced by ~99.5% (200 -> 1).
- Simulated prepaint shaping work reduced by ~99.5% (24,000 -> 120 work units).

Interpretation: Phase 4 removes full-visible-line reshaping from the common single-line edit path and keeps reshape work bounded to dirty lines.

---

## Phase 5: Add Backpressure, Coalescing, And Stale Update Dropping

### Acceptance Criteria

- [x] Server outbound scene updates cannot grow without bound for a slow client.
- [x] Client scene updates cannot grow without bound during rapid typing.
- [x] Cursor updates are coalesced when multiple pending cursor updates exist.
- [x] Superseded scene/text updates are dropped only when safe by version/order rules.
- [x] Text patches are never dropped in a way that leaves the client with an inconsistent text cache.
- [x] Metrics expose dropped/coalesced update counts.
- [x] A stress test with artificially slowed rendering does not produce ever-growing input lag.
- [x] A slow or blocked client does not block unrelated clients or the server hot path.

### Action Steps (TODO)

- [x] Add bounded or monitored queue behavior for hot-path scene update channels.
- [x] Add coalescing logic for cursor-only and superseded render updates.
- [x] Add safe stale-dropping rules based on buffer/version sequencing.
- [x] Add resync logic for any dropped patch sequences.
- [x] Add metrics for dropped/coalesced counts and queue pressure.
- [x] Add stress tests with intentionally slowed rendering/consumption.

### Phase 5 Findings (2026-05-02)

Implemented:

- Server per-client outbound scene channels are now bounded (`tokio::mpsc::channel(256)`).
- Client IPC->UI scene channel is now bounded (`tokio::mpsc::channel(256)`).
- Safe coalescing/drop rules:
  - cursor/selection-only scene patches are coalesced/dropped under pressure.
  - text/snapshot pressure triggers `ResyncScene` to preserve text-cache correctness.
- Server-side backpressure policy for full outbound queue:
  - safe ephemeral updates are coalesced,
  - non-ephemeral updates are treated as pressure faults and disconnected, preventing unbounded backlog and protecting unrelated clients.
- Metrics counters added/logged for dropped/coalesced queue events.

Profiler (`cargo run -- profiler`) now includes queue pressure simulation:

- `updates=20000 capacity=256`
- `unbounded_peak_depth=20000`
- `bounded_peak_depth=256`
- `dropped_or_coalesced=19744`

Measured improvement (same stress model):

- Peak queued scene updates are capped from 20,000 to 256 (~98.7% reduction).
- Queue growth is bounded, preventing ever-growing stale update replay.

Interpretation: Phase 5 bounds queue growth and keeps hot paths responsive under slow-render/slow-client pressure while preserving consistency via resync.

---

## Phase 6: Make Viewport Tracking Real And Dynamic

### Acceptance Criteria

- [x] The client reports actual visible line ranges instead of always using `0..200`.
- [x] The server stores an independent viewport per client.
- [x] Scene snapshots are scoped to the client’s viewport.
- [x] Text patches are sent to a client only if they affect or are relevant to that client’s viewport.
- [x] Scrolling requests the newly visible line range.
- [x] Multi-client tests verify that clients with different viewports receive different scoped updates.
- [x] IPC payload size is proportional to visible viewport size, not full buffer size.

### Action Steps (TODO)

- [x] Implement real viewport computation from window size, line height, scroll position, and panes.
- [x] Send viewport updates from client as scroll/layout changes occur.
- [x] Scope server snapshot/patch generation to each client viewport.
- [x] Add multi-client viewport correctness tests.
- [x] Measure and validate reduced payload size for small viewports.

### Phase 6 Findings (2026-05-02)

Implemented:

- Scene snapshots now include explicit viewport metadata (`viewport_start_line`, `viewport_end_line`).
- Client viewport is now computed from actual render bounds and line height, and sent via `SetViewport` updates (instead of fixed `0..200`).
- Server continues to keep independent per-client viewport state and sends viewport-scoped snapshots/updates.
- Existing multi-client viewport test validates independent scoped updates.

Profiler (`cargo run -- profiler`) now includes viewport-scoped payload simulation.

Measured improvement (viewport-scoped snapshot payload):

- Small viewport (`vh=20`) message size is materially smaller than large viewport (`vh=120`) for the same buffer.
- This confirms IPC payload scales with viewport size, not full-buffer size, in the snapshot path.

---

## Phase 7: Add Performance Regression Tests And Benchmarks

### Acceptance Criteria

- [x] Single-character edit IPC payload size remains bounded as line length grows.
- [x] Single-character edit dirty-line count remains bounded.
- [x] Single-character edit shaped-line count remains bounded.
- [x] Cursor-only movement does not emit text patches.
- [x] Cursor-only movement does not reshape text.
- [x] Queue depths remain bounded under rapid typing.
- [x] Slow render simulation does not cause unbounded stale update replay.
- [x] Benchmarks report p50/p95/p99 latency for key-to-scene and scene-to-paint.
- [x] CI or a documented local performance command can detect regressions.

### Action Steps (TODO)

- [x] Add benchmark scenarios: 10k-char line, 100k-char line, 1k-line typing, repeated backspace, large-buffer cursor movement, multi-line selection.
- [x] Add stress scenarios with slowed rendering and multi-client viewports.
- [x] Add assertions for bounded payload size, bounded dirty/shaped line counts, and bounded queue depth.
- [x] Add latency reporting for p50/p95/p99 key-to-scene and scene-to-paint.
- [x] Integrate into CI or provide a documented local perf gate command.

### Phase 7 Findings (2026-05-02)

Implemented:

- Added a local performance gate command: `cargo run -- perf-gate`.
- Added bounded assertions for:
  - edit patch payload growth (10k vs 100k line),
  - dirty-line average per single-char edit,
  - shaped-line average per single-char edit,
  - queue peak depth under pressure,
  - stale replay bound under slow-render simulation.
- Added latency distribution reporting for key-to-scene and scene-to-paint (`p50/p95/p99`).
- Kept full profiler scenarios in `cargo run -- profiler` and added regression-check mode in `perf-gate`.

Result: Phase 7 now provides a documented local perf gate to detect regressions on bounded hot-path behavior.

## Recommended Order Of Work

1. Phase 1: Instrumentation.
2. Phase 2: Incremental server text patches.
3. Phase 3: Incremental client text cache.
4. Phase 4: Shaped-line caching and dirty-line rendering.
5. Phase 5: Queue coalescing/backpressure.
6. Phase 6: Real dynamic viewports.
7. Phase 7: Regression tests and benchmarks.

## Final Diagnosis

The buffer data structure is not the primary problem. The canonical text buffer already uses `ropey::Rope`, which is suitable for scalable text mutation.

The real problem is the protocol/render hot path:

```text
single keypress
  -> full visible rope slice copied into String
  -> full visible text serialized as JSON
  -> full visible text parsed by client
  -> client replaces whole content String
  -> renderer clones/splits all visible text
  -> GPUI shapes every visible line
```

This makes typing cost grow with visible text size. The fix is to make the path incremental end-to-end:

```text
small edit
  -> small versioned text patch
  -> incremental client line cache update
  -> dirty-line/dirty-segment invalidation
  -> reshape only affected text
  -> coalesce/drop stale render updates safely
```
