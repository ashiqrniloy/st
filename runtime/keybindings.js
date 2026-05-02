globalThis.__stKeybindings = {
  keymap: {
    bind(chord, commandId) {
      const normalizedChord = String(chord);
      const normalizedCommandId = String(commandId);
      Deno.core.ops.op_bind_keybinding(normalizedChord, normalizedCommandId);
      return { chord: normalizedChord, commandId: normalizedCommandId, registered: true };
    },
  },
};
