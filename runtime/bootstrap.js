async function mainLoop() {
  while (true) {
    const event = await globalThis.st.recvEditorEvent();
    if (event === null) {
      Deno.core.print("JS loop: editor event channel closed, stopping runtime loop.\n");
      break;
    }

    const started = Date.now();
    Deno.core.print(`JS received: ${JSON.stringify(event)}\n`);

    globalThis.st.sendRenderCommand({
      DrawRect: { x: 0.0, y: 0.0, w: 100.0, h: 100.0, color: 0xff00ff },
    });

    Deno.core.print(`JS command duration ms: ${Date.now() - started}\n`);
  }
}

mainLoop().catch((err) => {
  Deno.core.print(`JS mainLoop error: ${err?.stack ?? err}\n`);
});
