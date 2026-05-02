# st Configuration System

st uses executable JavaScript for user configuration and declarative YAML for optional data loaded by that JavaScript.

## Primary config file

The primary config entrypoint is:

```text
~/.config/st/init.js
```

At server startup, the JavaScript runtime loads files in this order:

1. Built-in runtime API: `runtime/editor_api.js`
2. User config, if present: `~/.config/st/init.js`
3. Built-in runtime event loop: `runtime/bootstrap.js`

JavaScript is the supported config language. TypeScript-only syntax is intentionally not accepted until/unless the runtime grows an explicit transpilation step.

## Public config API style

Public config APIs are exposed directly as JavaScript globals or namespaces. Do not prefix them with `st.`.

Good:

```js
keymap.bind("ctrl shift h", "window.split_horizontal");
```

The `st.` prefix is reserved for internal implementation details and is not part of the public config API.

## Default config template

A starter config is checked in at:

```text
runtime/default_init.js
```

Copy it into place to test config loading and keybindings:

```bash
mkdir -p ~/.config/st
cp runtime/default_init.js ~/.config/st/init.js
```

Restart `st server` after editing config. Hot reload is planned separately.

## Keybindings

Use `keymap.bind(chord, commandId)` from `init.js`:

```js
keymap.bind("ctrl shift h", "window.split_horizontal");
keymap.bind("ctrl shift v", "window.split_vertical");
keymap.bind("ctrl shift s", "window.split_dwim");
```

The keybinding is submitted to Rust, validated against the Rust command registry, stored in the Rust-owned keymap, and exposed through keybinding documentation queries.

### Chord grammar

Key chords are space-separated segments:

```text
modifier-group key key ...
```

A modifier group uses `+` between modifiers. The modifier group applies to following key segments until another explicit modifier group/key combination is used.

Examples:

```js
keymap.bind("ctrl shift h", "window.split_horizontal");
keymap.bind("ctrl shift w h", "window.split_horizontal");
keymap.bind("ctrl x ctrl s", "window.split_dwim");
```

The second example means: hold `ctrl shift`, press `w`, then press `h`.

Supported modifier names:

| Name | Meaning |
| --- | --- |
| `ctrl` | Control modifier |
| `shift` | Shift modifier |
| `meta` | Meta / Command modifier |
| `alt` | Alt / Option modifier |

Supported named non-character keys include:

| Name |
| --- |
| `space` |
| `enter` |
| `escape` |

Printable keys such as `h`, `v`, `s`, and `1` are also valid key segments.

## Window split commands

The built-in split commands are:

| Command | Description |
| --- | --- |
| `window.split_horizontal` | Split active pane into top/bottom regions. |
| `window.split_vertical` | Split active pane into left/right regions. |
| `window.split_dwim` | Choose split direction from current dimensions and layout. |

## Runtime safety model

JavaScript config does not mutate editor state directly. It calls APIs such as `keymap.bind`, which submit typed requests to Rust. Rust owns command registration, key dispatch, pane layout state, split validation, and scene generation.

## In-program documentation

The same configuration information is registered in Rust documentation metadata. Query the API documentation registry to discover entries such as:

- `config.loading`
- `api.keymap.bind`

This markdown file is the website/GitHub-facing companion to those live docs.
