## Phase 25: Text Editing Features In Depth

Goal: revisit editor text behavior comprehensively after core storage, file, and undo/redo primitives are stronger.

Acceptance criteria:

- Selection, traversal, and editability/read-only behavior are modeled server-side.
- Movement by character, word, line, sentence, file boundary, and page is explicit and tested.
- Keyboard and mouse selection behavior is deterministic and command-routed where applicable.
- Read-only state prevents mutations while still allowing navigation and selection.

Test plan:

- Add server-side selection model tests for character, word, line, sentence, and multi-range cases where supported.
- Add traversal tests for file boundaries, arrow keys, Home/End, and PageUp/PageDown.
- Add mouse selection tests or deterministic event-model tests for click/drag/double-click/triple-click behavior.
- Add read-only/editable tests proving navigation is allowed and mutations are blocked when read-only.

Implementation tasks:

- [ ] Define canonical server-side selection model.
- [ ] Define editable vs read-only editor state.
- [ ] Make editor state not editable/read-only.
- [ ] Make editor state editable again.
- [ ] Highlight a single letter.
- [ ] Highlight a word.
- [ ] Highlight a line.
- [ ] Highlight a sentence.
- [ ] Highlight multiple words.
- [ ] Highlight multiple lines.
- [ ] Highlight multiple sentences.
- [ ] Traverse text by letter.
- [ ] Traverse text by word.
- [ ] Traverse text by line.
- [ ] Traverse text by sentence.
- [ ] Go to the beginning of a file.
- [ ] Go to the end of a file.
- [ ] Implement reliable arrow-key movement through the server-owned command path.
- [ ] Implement reliable Shift+arrow selection through the server-owned command path.
- [ ] Add Home/End and platform-specific variants.
- [ ] Add PageUp/PageDown behavior.
- [ ] Add mouse click, drag, double-click, and triple-click selection behavior.

- [ ] Write or update tests from the test plan after implementation.
- [ ] Validate that the tests prove each acceptance criterion is met.
- [ ] Run formatting and the relevant/full test suite.

Milestone:

```text
Core text traversal, selection, and editability behavior is explicit, tested, and server-owned.
```

