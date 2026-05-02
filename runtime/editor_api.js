const { core } = Deno;

function sendEditorCommand(command) {
  core.ops.op_send_editor_command(command);
}

function splitWindowHorizontal() {
  sendEditorCommand("SplitWindowHorizontal");
  return { commandId: "window.split_horizontal", requested: true };
}

function splitWindowVertical() {
  sendEditorCommand("SplitWindowVertical");
  return { commandId: "window.split_vertical", requested: true };
}

function splitWindowDwim() {
  sendEditorCommand("SplitWindowDwim");
  return { commandId: "window.split_dwim", requested: true };
}

const keymap = {
  bind(chord, commandId) {
    core.ops.op_bind_keybinding(String(chord), String(commandId));
    return { chord: String(chord), commandId: String(commandId), registered: true };
  },
};

globalThis.keymap = keymap;
globalThis.splitWindowHorizontal = splitWindowHorizontal;
globalThis.splitWindowVertical = splitWindowVertical;
globalThis.splitWindowDwim = splitWindowDwim;

// Internal runtime namespace used by bootstrap.js. User config should use the
// public globals above, such as keymap.bind(...), without an st. prefix.
globalThis.__stRuntime = {
  sendRenderCommand(command) {
    core.ops.op_send_render_command(command);
  },
  async recvEditorEvent() {
    return await core.ops.op_recv_editor_event();
  },
};
