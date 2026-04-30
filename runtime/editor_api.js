const { core } = Deno;

globalThis.st = {
  sendRenderCommand(command) {
    core.ops.op_send_render_command(command);
  },
  async recvEditorEvent() {
    return await core.ops.op_recv_editor_event();
  },
};
