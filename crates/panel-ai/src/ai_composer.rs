//! AI composer power-user affordances: `/`-slash commands and `@`-file mentions.
//!
//! Ports `reference-src/src/modules/ai/lib/slashCommands.ts` plus the composer
//! `@` / `/` autocomplete-popover behaviour from
//! `reference-src/src/modules/ai/components/AiInputBar.tsx`.

use labonair_command_palette::{match_score, SearchMode};

/// The `/init` prompt body (verbatim from `slashCommands.ts`).
pub const INIT_PROMPT: &str =
    "Scan this workspace and produce LABONAIR.md at the workspace root with:\n\n\
- One-paragraph project description.\n\
- Build / test / dev commands.\n\
- Architecture overview (subsystems, data flow, key dirs).\n\
- Conventions worth knowing (naming, patterns, gotchas).\n\
- Paths to entry points.\n\n\
Use grep/glob/list_directory/read_file to explore. Cap LABONAIR.md under 200 lines. \
Use write_file to create it (will go through normal approval).";

/// A composer slash command (`SLASH_COMMANDS` in the reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCommand {
    pub name: &'static str,
    pub invocation: &'static str,
    pub label: &'static str,
}

pub const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "init",
        invocation: "/init",
        label: "Initialize workspace",
    },
    SlashCommand {
        name: "plan",
        invocation: "/plan",
        label: "Toggle plan mode",
    },
];

/// Outcome of intercepting a slash command from the composer
/// (`SlashOutcome` in the reference).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashOutcome {
    /// Command ran inline; do NOT send a chat message. Carries a status line.
    Handled(String),
    /// Replace the user's text with `prompt` and send, tagged with `command`.
    SendPrompt {
        prompt: String,
        command: &'static str,
    },
    /// Not a slash command — behave as usual.
    None,
}

/// Prepend the command marker the reference wraps `/init` prompts with
/// (`wrapWithCommandMarker`).
pub fn wrap_with_command_marker(prompt: &str, name: &str) -> String {
    format!("<labonair-command name=\"{name}\" />\n\n{prompt}")
}

/// Port of `tryRunSlashCommand`. `plan_active` is the current plan-mode flag;
/// the returned `bool` is the flag's new value (so the caller can apply it).
pub fn parse_slash(input: &str, plan_active: bool) -> (SlashOutcome, bool) {
    let trimmed = input.trim();
    let lead = trimmed.chars().next();
    if lead != Some('/') && lead != Some('#') {
        return (SlashOutcome::None, plan_active);
    }
    let mut parts = trimmed[1..].split_whitespace();
    let head = parts.next().unwrap_or("");
    let tail = parts.collect::<Vec<_>>().join(" ");
    let known = SLASH_COMMANDS.iter().any(|c| c.name == head);
    // `#foo` is only a slash command when `foo` is a known command; otherwise it
    // is a `#directive` token and must fall through untouched.
    if lead == Some('#') && !known {
        return (SlashOutcome::None, plan_active);
    }
    match head {
        "plan" => {
            let next = if tail == "off" || tail == "exit" {
                false
            } else {
                !plan_active
            };
            let msg = if next {
                "Plan mode on"
            } else {
                "Plan mode off"
            };
            (SlashOutcome::Handled(msg.to_string()), next)
        }
        "init" => (
            SlashOutcome::SendPrompt {
                prompt: INIT_PROMPT.to_string(),
                command: "init",
            },
            plan_active,
        ),
        _ => (SlashOutcome::None, plan_active),
    }
}

/// Which autocomplete popover, if any, the current composer text should open.
/// The cursor is assumed to be at the end of `text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerPopup {
    Slash { query: String },
    File { query: String },
}

/// Decide the popover from the composer text.
pub fn detect_popup(text: &str) -> Option<ComposerPopup> {
    // `/word` at the very start, no whitespace yet.
    if let Some(rest) = text.strip_prefix('/') {
        if !rest.contains(char::is_whitespace) {
            return Some(ComposerPopup::Slash {
                query: rest.to_string(),
            });
        }
    }
    // Last whitespace-delimited token starts with `@`.
    let token = text
        .split(|c: char| c.is_whitespace())
        .next_back()
        .unwrap_or("");
    if let Some(rest) = token.strip_prefix('@') {
        if !rest.contains('@') {
            return Some(ComposerPopup::File {
                query: rest.to_string(),
            });
        }
    }
    None
}

/// Slash commands matching `query`, best match first.
pub fn filter_slash(query: &str) -> Vec<&'static SlashCommand> {
    let mut hits: Vec<(i64, &'static SlashCommand)> = SLASH_COMMANDS
        .iter()
        .filter_map(|c| match_score(SearchMode::Fuzzy, c.name, query).map(|s| (s, c)))
        .collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.0));
    hits.into_iter().map(|(_, c)| c).collect()
}

/// Fuzzy-rank `paths` against `query`, capped at `limit`.
pub fn filter_files(query: &str, paths: &[String], limit: usize) -> Vec<String> {
    if query.is_empty() {
        return paths.iter().take(limit).cloned().collect();
    }
    let mut hits: Vec<(i64, &String)> = paths
        .iter()
        .filter_map(|p| match_score(SearchMode::Fuzzy, p, query).map(|s| (s, p)))
        .collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.0));
    hits.into_iter()
        .take(limit)
        .map(|(_, p)| p.clone())
        .collect()
}

/// Replace the trailing `@…` mention token in `text` with `@<path> `.
pub fn apply_file_mention(text: &str, path: &str) -> String {
    let cut = text.rfind('@').unwrap_or(text.len());
    let mut out = text[..cut].to_string();
    out.push('@');
    out.push_str(path);
    out.push(' ');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_slash_init_and_plan() {
        let (o, plan) = parse_slash("/init", false);
        assert!(matches!(
            o,
            SlashOutcome::SendPrompt {
                command: "init",
                ..
            }
        ));
        assert!(!plan);

        let (o, plan) = parse_slash("/plan", false);
        assert_eq!(o, SlashOutcome::Handled("Plan mode on".into()));
        assert!(plan);
        let (o, plan) = parse_slash("/plan", true);
        assert_eq!(o, SlashOutcome::Handled("Plan mode off".into()));
        assert!(!plan);
        let (_, plan) = parse_slash("/plan off", true);
        assert!(!plan);
    }

    #[test]
    fn parse_slash_passthrough() {
        assert_eq!(parse_slash("hello world", false).0, SlashOutcome::None);
        // `#directive` tokens are not slash commands.
        assert_eq!(parse_slash("#deploy now", false).0, SlashOutcome::None);
        // but `#plan` (a known command name) is intercepted.
        assert!(matches!(
            parse_slash("#plan", false).0,
            SlashOutcome::Handled(_)
        ));
        assert_eq!(parse_slash("/unknowncmd", false).0, SlashOutcome::None);
    }

    #[test]
    fn detect_popup_cases() {
        assert_eq!(
            detect_popup("/pl"),
            Some(ComposerPopup::Slash { query: "pl".into() })
        );
        assert_eq!(
            detect_popup("/"),
            Some(ComposerPopup::Slash {
                query: String::new()
            })
        );
        // whitespace after the command closes the slash popover
        assert_eq!(detect_popup("/plan "), None);
        assert_eq!(
            detect_popup("look at @src/ma"),
            Some(ComposerPopup::File {
                query: "src/ma".into()
            })
        );
        assert_eq!(detect_popup("plain text"), None);
    }

    #[test]
    fn filter_helpers() {
        let slash = filter_slash("pl");
        assert_eq!(slash[0].name, "plan");
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
        ];
        let hits = filter_files("mainrs", &files, 10);
        assert_eq!(hits[0], "src/main.rs");
        assert_eq!(filter_files("", &files, 2).len(), 2);
    }

    #[test]
    fn apply_file_mention_replaces_token() {
        assert_eq!(
            apply_file_mention("look at @src/ma", "src/main.rs"),
            "look at @src/main.rs "
        );
        assert_eq!(apply_file_mention("@", "a.rs"), "@a.rs ");
    }

    #[test]
    fn wrap_marker() {
        assert_eq!(
            wrap_with_command_marker("do it", "init"),
            "<labonair-command name=\"init\" />\n\ndo it"
        );
    }
}
