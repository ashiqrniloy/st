## Phase 28: Adopt GPUI Component For Non-Editor UI

Goal: improve application chrome and supporting UI using `gpui-component` without replacing the custom server-backed editor surface.

Acceptance criteria:

- `gpui-component` is used only for non-editor UI surfaces.
- The custom server-backed editor surface remains independent of `gpui-component::InputState`.
- Application startup initializes component support only after core rendering boundaries are stable.
- Allowed component-backed UI surfaces are documented.

Test plan:

- Add boundary tests or compile-time structure checks proving editor input/state does not depend on gpui-component InputState.
- Add startup tests or smoke tests for component initialization where practical.
- Add documentation tests/checks for allowed gpui-component UI surfaces.

Explicit boundary: `gpui-component` is not used for the editor itself. The editor remains our custom GPUI native editor view, backed by server-owned editor state.

Use `gpui-component` only for:

- [ ] Command palette.
- [ ] Buttons.
- [ ] Dialogs.
- [ ] Settings.
- [ ] Panels.
- [ ] Tabs.
- [ ] Status bar.
- [ ] Menus.
- [ ] Notifications.
- [ ] Dock layout.

Implementation tasks:

- [ ] Add `gpui-component` dependency only after editor core/client-server rendering is stable.
- [ ] Call `gpui_component::init(cx)` during GPUI app startup.
- [ ] Wrap the application root as required for overlays/dialogs/notifications.
- [ ] Keep editor buffer, cursor, selections, undo/redo, and key dispatch outside `gpui-component::InputState`.
- [ ] Document which UI surfaces are allowed to use `gpui-component`.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

