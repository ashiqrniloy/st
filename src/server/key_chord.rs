use crate::events::KeyInputEvent;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SimpleChord {
    pub(super) key: String,
    pub(super) ctrl: bool,
    pub(super) alt: bool,
    pub(super) shift: bool,
    pub(super) meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct KeyChord(pub(super) Vec<SimpleChord>);

pub(super) fn normalize_key_name(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        " " | "spacebar" => "space".into(),
        "return" => "enter".into(),
        "esc" => "escape".into(),
        other => other.into(),
    }
}

pub(super) fn normalize_key_input(event: &KeyInputEvent) -> SimpleChord {
    SimpleChord {
        key: normalize_key_name(&event.logical_key),
        ctrl: event.ctrl,
        alt: event.alt,
        shift: event.shift,
        meta: event.meta,
    }
}

pub(super) fn parse_key_chord(chord: &str) -> Result<KeyChord, String> {
    let mut parts = Vec::new();
    let mut active_modifiers = KeyModifiers::default();

    for segment in chord.split_whitespace() {
        let parsed = parse_key_chord_segment(segment, active_modifiers)?;
        match parsed {
            ParsedChordSegment::Modifiers(modifiers) => active_modifiers = modifiers,
            ParsedChordSegment::Key(simple) => parts.push(simple),
        }
    }

    if parts.is_empty() {
        return Err("key chord must include at least one non-modifier key".into());
    }

    Ok(KeyChord(parts))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct KeyModifiers {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

enum ParsedChordSegment {
    Modifiers(KeyModifiers),
    Key(SimpleChord),
}

fn parse_key_chord_segment(
    segment: &str,
    active_modifiers: KeyModifiers,
) -> Result<ParsedChordSegment, String> {
    let mut modifiers = active_modifiers;
    let mut explicit_modifier_seen = false;
    let mut key: Option<String> = None;

    for piece in segment.split('+') {
        if piece.is_empty() {
            return Err(format!(
                "invalid empty key chord piece in segment: {segment}"
            ));
        }
        match piece.to_ascii_lowercase().as_str() {
            "ctrl" => {
                modifiers.ctrl = true;
                explicit_modifier_seen = true;
            }
            "alt" => {
                modifiers.alt = true;
                explicit_modifier_seen = true;
            }
            "shift" => {
                modifiers.shift = true;
                explicit_modifier_seen = true;
            }
            "meta" | "cmd" => {
                modifiers.meta = true;
                explicit_modifier_seen = true;
            }
            key_piece => {
                if key.replace(normalize_key_name(key_piece)).is_some() {
                    return Err(format!(
                        "key chord segment may contain at most one non-modifier key: {segment}"
                    ));
                }
            }
        }
    }

    if let Some(key) = key {
        Ok(ParsedChordSegment::Key(SimpleChord {
            key,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            meta: modifiers.meta,
        }))
    } else if explicit_modifier_seen {
        Ok(ParsedChordSegment::Modifiers(modifiers))
    } else {
        Err(format!("invalid key chord segment: {segment}"))
    }
}
