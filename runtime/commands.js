function sendEditorCommand(command) {
  Deno.core.ops.op_send_editor_command(command);
}

globalThis.__stCommands = {
  splitWindowHorizontal() {
    sendEditorCommand("SplitWindowHorizontal");
    return { commandId: "window.split_horizontal", requested: true };
  },
  splitWindowVertical() {
    sendEditorCommand("SplitWindowVertical");
    return { commandId: "window.split_vertical", requested: true };
  },
  splitWindowDwim() {
    sendEditorCommand("SplitWindowDwim");
    return { commandId: "window.split_dwim", requested: true };
  },
};
