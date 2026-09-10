//! Turn captured `gpui::Keystroke`s into Labonair's binding-string form.
//!
//! The keymap surface records the keys the user actually presses; the keymap
//! file and every validation/merge/conflict path speak the textual form
//! (`"cmd-shift-p"`, chord segments space-joined). This module is the single
//! adapter between the two. Modifiers are emitted in `labonair_keymap`'s
//! canonical order (ctrl, alt, shift, cmd) so the produced string is already
//! normalized and round-trips through [`gpui::Keystroke::parse`].

use gpui::Keystroke;

/// Serialize a captured chord into its binding string.
pub fn to_binding_string(keys: &[Keystroke]) -> String {
    keys.iter().map(one_keystroke).collect::<Vec<_>>().join(" ")
}

fn one_keystroke(keystroke: &Keystroke) -> String {
    let modifiers = &keystroke.modifiers;
    let mut out = String::with_capacity(keystroke.key.len() + 12);
    if modifiers.control {
        out.push_str("ctrl-");
    }
    if modifiers.alt {
        out.push_str("alt-");
    }
    if modifiers.shift {
        out.push_str("shift-");
    }
    if modifiers.platform {
        out.push_str("cmd-");
    }
    if modifiers.function {
        out.push_str("fn-");
    }
    out.push_str(&keystroke.key);
    out
}

/// Whether a keystroke carries only a modifier key (the user is still
/// composing). Such an event is never a complete binding and is dropped during
/// capture.
pub fn is_modifier_only(keystroke: &Keystroke) -> bool {
    matches!(
        keystroke.key.as_str(),
        "" | "control"
            | "ctrl"
            | "alt"
            | "option"
            | "shift"
            | "platform"
            | "cmd"
            | "super"
            | "win"
            | "fn"
            | "function"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_gpui_parse() {
        for input in [
            "cmd-b",
            "cmd-shift-p",
            "ctrl-alt-shift-cmd-k",
            "ctrl-tab",
            "escape",
            "f5",
            "up",
            "cmd--",
            "shift-a",
        ] {
            let parsed = Keystroke::parse(input).unwrap();
            let serialized = to_binding_string(std::slice::from_ref(&parsed));
            assert_eq!(
                Keystroke::parse(&serialized).unwrap(),
                parsed,
                "{input} -> {serialized}"
            );
        }
    }

    #[test]
    fn joins_chord_segments_with_spaces() {
        let a = Keystroke::parse("cmd-k").unwrap();
        let b = Keystroke::parse("cmd-s").unwrap();
        assert_eq!(to_binding_string(&[a, b]), "cmd-k cmd-s");
    }

    #[test]
    fn modifier_only_keystrokes_are_rejected() {
        assert!(is_modifier_only(&Keystroke::parse("cmd").unwrap()));
        assert!(is_modifier_only(&Keystroke::parse("shift").unwrap()));
        assert!(!is_modifier_only(&Keystroke::parse("cmd-p").unwrap()));
    }
}
