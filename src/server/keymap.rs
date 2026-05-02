use std::collections::HashMap;

use crate::{
    commands::{CommandId, CommandSource},
    documentation::KeybindingDescriptor,
};

use super::key_chord::{KeyChord, SimpleChord, parse_key_chord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyResolution {
    Exact,
    Prefix,
    None,
}

#[derive(Debug, Clone)]
pub(super) struct RegisteredKeybinding {
    pub(super) command_id: CommandId,
    pub(super) descriptor: KeybindingDescriptor,
}

#[derive(Debug, Default)]
pub(super) struct Keymap {
    bindings: HashMap<KeyChord, RegisteredKeybinding>,
}

impl Keymap {
    pub(super) fn bind(&mut self, chord: KeyChord, binding: RegisteredKeybinding) {
        self.bindings.insert(chord, binding);
    }

    pub(super) fn bind_builtin(
        &mut self,
        chord_text: &str,
        command_id: CommandId,
        title: &str,
    ) -> Result<(), String> {
        let chord = parse_key_chord(chord_text)?;
        let command_id_text = command_id.as_str().to_string();
        self.bind(
            chord,
            RegisteredKeybinding {
                command_id,
                descriptor: KeybindingDescriptor {
                    chord: chord_text.into(),
                    command_id: command_id_text,
                    title: title.into(),
                    description: "Built-in editor keybinding handled by Rust.".into(),
                    source: CommandSource::Builtin,
                    owner: None,
                },
            },
        );
        Ok(())
    }

    pub(super) fn resolve_state(&self, chord: &[SimpleChord]) -> KeyResolution {
        let mut prefix = false;
        for registered in self.bindings.keys() {
            if registered.0 == chord {
                return KeyResolution::Exact;
            }
            if registered.0.starts_with(chord) {
                prefix = true;
            }
        }
        if prefix {
            KeyResolution::Prefix
        } else {
            KeyResolution::None
        }
    }

    pub(super) fn lookup(&self, chord: &[SimpleChord]) -> Option<CommandId> {
        self.bindings
            .get(&KeyChord(chord.to_vec()))
            .map(|binding| binding.command_id.clone())
    }

    pub(super) fn list_descriptors(&self) -> Vec<KeybindingDescriptor> {
        let mut bindings: Vec<_> = self
            .bindings
            .values()
            .map(|binding| binding.descriptor.clone())
            .collect();
        bindings.sort_by(|a, b| a.chord.cmp(&b.chord).then(a.command_id.cmp(&b.command_id)));
        bindings
    }
}
