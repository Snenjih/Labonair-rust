//! Module-owned keymap runtime: default/user layer composition, context-aware
//! resolution, conflict detection, and the lossless keymap-file editor model.
//! See [`adapter`], [`file`], [`runtime`], and [`management`].
//!
//! This root keeps only two pure keystroke-string helpers shared across the
//! runtime and the keymap UI. The pre-migration `ShortcutId` / `SHORTCUTS`
//! cheat-sheet model was removed in R08-007; command identity is `CommandId`
//! and default bindings are owner `CommandDescriptor` metadata.

pub mod adapter;
pub mod command_provider;
pub mod file;
pub mod management;
pub mod runtime;

/// Canonicalise a keystroke string so equivalent bindings compare equal
/// regardless of modifier order (`"shift-cmd-d"` == `"cmd-shift-d"`).
/// Chords are canonicalised one keystroke at a time and retain their order.
pub fn normalize_keystrokes(binding: &str) -> String {
    binding
        .split_whitespace()
        .map(normalize_single_keystroke)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_single_keystroke(binding: &str) -> String {
    let mut parts: Vec<&str> = binding.split('-').filter(|s| !s.is_empty()).collect();
    // Trailing "" from a literal "cmd--" (minus key) — restore it.
    let key = if binding.ends_with("--") {
        parts.pop();
        "-"
    } else {
        parts.pop().unwrap_or("")
    };
    let rank = |m: &str| match m {
        "ctrl" | "control" => 0,
        "alt" | "option" => 1,
        "shift" => 2,
        "cmd" | "super" | "platform" | "win" => 3,
        _ => 4,
    };
    parts.sort_by_key(|m| rank(m));
    parts.dedup();
    let mods = parts.join("-");
    if mods.is_empty() {
        key.to_string()
    } else {
        format!("{mods}-{key}")
    }
}

/// Split a GPUI keystroke string (`"cmd-shift-p"`, `"cmd--"`) into display
/// tokens (`["\u{2318}", "\u{21e7}", "P"]`) — the modifier glyphs the keymap
/// UI and command palette render right-aligned.
pub fn keystroke_tokens(binding: &str) -> Vec<String> {
    let mut parts: Vec<&str> = binding.split('-').filter(|s| !s.is_empty()).collect();
    let key = if binding.ends_with("--") {
        parts.pop();
        "-".to_string()
    } else {
        parts.pop().unwrap_or("").to_string()
    };
    let mut out: Vec<String> = parts
        .iter()
        .map(|m| match *m {
            "ctrl" | "control" => "\u{2303}".to_string(),
            "alt" | "option" => "\u{2325}".to_string(),
            "shift" => "\u{21e7}".to_string(),
            "cmd" | "super" | "platform" | "win" => "\u{2318}".to_string(),
            other => other.to_string(),
        })
        .collect();
    if !key.is_empty() {
        out.push(if key.chars().count() == 1 {
            key.to_uppercase()
        } else {
            key
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_orders_modifiers_regardless_of_input_order() {
        assert_eq!(normalize_keystrokes("shift-cmd-d"), "shift-cmd-d");
        assert_eq!(normalize_keystrokes("cmd-shift-d"), "shift-cmd-d");
        assert_eq!(normalize_keystrokes("cmd-k cmd-s"), "cmd-k cmd-s");
    }

    #[test]
    fn keystroke_tokens_render_modifier_glyphs() {
        assert_eq!(
            keystroke_tokens("cmd-shift-p"),
            ["\u{2318}", "\u{21e7}", "P"]
        );
        assert_eq!(keystroke_tokens("ctrl-tab"), ["\u{2303}", "tab"]);
        assert_eq!(keystroke_tokens("cmd-b"), ["\u{2318}", "B"]);
    }
}
