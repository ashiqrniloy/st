const { core } = Deno;

async function mainLoop() {
  while (true) {
    const event = await core.ops.op_recv_editor_event();
    if (event === null) {
      core.print("JS loop: editor event channel closed, stopping runtime loop.\n");
      break;
    }

    const started = Date.now();
    core.print(`JS received: ${JSON.stringify(event)}\n`);

    core.ops.op_send_render_command({
      DrawRect: { x: 0.0, y: 0.0, w: 100.0, h: 100.0, color: 0xff00ff },
    });

    core.print(`JS command duration ms: ${Date.now() - started}\n`);
  }
}

mainLoop().catch((err) => {
  core.print(`JS mainLoop error: ${err?.stack ?? err}\n`);
});
