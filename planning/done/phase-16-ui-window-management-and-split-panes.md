## Phase 16: UI Window Management And Split Panes

Goal: implement server-owned window/pane layout management with manual horizontal/vertical split commands, a DWIM split command, GPUI rendering support, JS/extension APIs, and structured user-facing documentation.

Acceptance criteria:

- A client window starts as one full-window pane and can be split into at most four panes.
- Manual horizontal split divides the selected/active pane or full window along the X axis, producing top/bottom space within that split region.
- Manual vertical split divides the selected/active pane or full window along the Y axis, producing left/right space within that split region.
- From one full-window pane, one horizontal split produces two horizontal panes; one vertical split produces two vertical panes.
- Repeating the same split direction from a two-pane same-axis layout produces exactly three same-axis panes and makes that layout terminal for further splits.
- From a two-pane horizontal layout, a vertical split splits one unsplit pane; a second vertical split splits the remaining unsplit pane, producing four panes; no additional splits are allowed.
- From a two-pane vertical layout, a horizontal split splits one unsplit pane; a second horizontal split splits the remaining unsplit pane, producing four panes; no additional splits are allowed.
- Mixed split behavior is deterministic: the active pane is preferred when eligible, otherwise the next eligible unsplit pane is chosen by a documented stable ordering.
- Consecutive same-direction layouts stop at three panes, while mixed orthogonal layouts can reach four panes.
- Split requests that would exceed the allowed layout are rejected without corrupting layout state and return a clear command/protocol result.
- Pane focus/active-pane state is tracked by Rust and survives split operations deterministically.
- Rust owns canonical window, pane, and layout state; JS/extensions request typed split commands and never mutate layout state directly.
- GPUI client rendering consumes server layout state and renders pane rectangles consistently with the Rust layout calculation.
- A DWIM split command chooses the most useful split direction from current window dimensions, pane layout, and documented/configurable aspect-ratio thresholds.
- On a wide full-screen-style window, DWIM first creates a horizontal two-pane layout, then vertically splits one horizontal pane, then vertically splits the remaining horizontal pane, then stops at four panes.
- On a window that is wide but constrained to roughly half-screen height, DWIM prefers vertical splits, can create three vertical panes, and then stops.
- On a window that is tall/narrow or constrained to roughly half-screen width, DWIM prefers horizontal splits, can create three horizontal panes, and then stops.
- Manual split commands, DWIM split command, related JS APIs, and any user-tunable DWIM thresholds are registered with structured documentation metadata and discoverable through the help system.

Test plan:

- Add Rust unit tests for layout transitions from one pane to two horizontal panes, two vertical panes, three same-axis panes, mixed three-pane layouts, mixed four-pane layouts, and rejected over-limit splits.
- Add Rust tests for active-pane selection, deterministic fallback to the next eligible pane, and focus preservation after splits.
- Add Rust tests for rectangle calculation using representative window sizes, including odd dimensions and minimum-size edge cases.
- Add DWIM tests for wide full-window dimensions, wide half-height dimensions, tall/half-width dimensions, existing same-axis layouts, existing mixed layouts, and terminal layouts.
- Add protocol/command tests proving manual and DWIM split commands are routed through Rust command handlers and return clear success/error results.
- Add JS API tests proving extensions can request split-window-horizontal, split-window-vertical, and split-window-dwim through typed APIs but cannot mutate layout state directly.
- Add documentation metadata tests proving all public split commands, JS APIs, and configurable DWIM settings have descriptors.
- Add GPUI/client rendering tests or focused layout-adapter tests proving server pane rectangles are rendered as distinct panes without client-side layout divergence.

Implementation tasks:

- [x] Audit existing Rust window split primitives and identify which layout, command, protocol, and rendering pieces can be reused.
- [x] Define canonical Rust layout model for a client window, including `WindowLayout`, `PaneId`, active pane, split axis, pane tree/grid representation, and terminal layout states.
- [x] Define split invariants: maximum four panes, three-pane terminal same-axis layouts, four-pane terminal mixed layouts, deterministic active/eligible pane selection, and clear rejection behavior.
- [x] Implement pure Rust layout transition functions for manual horizontal and vertical splits.
- [x] Implement pure Rust pane rectangle calculation from window dimensions and layout state.
- [x] Ensure layout calculations handle odd pixel/cell dimensions deterministically.
- [x] Add minimum pane-size validation or a documented policy for very small windows.
- [x] Add Rust command variants for manual horizontal split, manual vertical split, and DWIM split.
- [x] Register split commands in the Rust command registry with structured metadata.
- [x] Route split command execution through the server-owned command path.
- [x] Return typed success/error results for accepted and rejected split requests.
- [x] Add protocol messages or extend existing scene/layout updates so clients receive pane IDs, active pane ID, rectangles, and any buffer/view association needed for rendering.
- [x] Update GPUI client layout/rendering code to render server-provided pane rectangles rather than independently deciding split geometry.
- [x] Add focus movement/activation behavior needed to choose which pane manual split commands affect.
- [x] Define DWIM split heuristic using current window dimensions, current layout, active pane, aspect ratio, and eligible next layouts.
- [x] Add configurable/documented DWIM threshold settings where user-tunable, while keeping the four-pane maximum as the phase invariant.
- [x] Implement DWIM in Rust so it can be called by users, keybindings, JS extensions, and future UI affordances without JavaScript on the hot path.
- [x] Expose typed JS APIs for `splitWindowHorizontal`, `splitWindowVertical`, and `splitWindowDwim` that submit validated Rust commands.
- [x] Document split commands, arguments/results, examples, JS API usage, error cases, and DWIM settings through structured descriptors.
- [x] Add default keybinding descriptors only if default split keybindings are introduced in this phase.
- [x] Ensure split state is scoped correctly for the current client/session/window model and does not assume a single global editor window beyond the current architecture.
- [x] Preserve performance boundaries: Rust owns layout mutation and scene generation; JS only orchestrates explicit extension calls.

- [x] Write or update tests from the test plan after implementation.
- [x] Validate that the tests prove each acceptance criterion is met.
- [x] Run formatting and the relevant/full test suite.

Milestone:

```text
Users and extensions can split the active window manually or via DWIM, Rust owns and validates all pane layout state, the GPUI client renders server-provided panes, and all public split capabilities are documented and tested.
```

