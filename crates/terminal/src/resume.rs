//! Pure lookup of the command that re-opens a terminal's former foreground
//! program after a session restore (Option D of ADR 0009).
//!
//! Only the process name is looked up here; whether and when to run the
//! result is the restoring workspace's decision.

use std::collections::BTreeMap;

/// Built-in `process name -> resume command` entries. Only tools whose resume
/// flags are known exactly belong here; anything else is user-configurable via
/// `terminal.resumeCommands`.
const DEFAULTS: &[(&str, &str)] = &[
    ("claude", "claude --continue"),
    ("codex", "codex resume --last"),
];

/// Reduce an OS-reported process name or path to the key used in the table:
/// basename, lower-case, without a `@version` / `-1.2.3` suffix or a script /
/// `.exe` extension (`/opt/bin/Claude-2.1.0` -> `claude`).
pub fn normalize_process_name(raw: &str) -> String {
    let base = raw
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut end = base.find('@').unwrap_or(base.len());
    if let Some(i) = base
        .match_indices('-')
        .find(|(i, _)| base[i + 1..].starts_with(|c: char| c.is_ascii_digit()))
        .map(|(i, _)| i)
    {
        end = end.min(i);
    }
    let name = &base[..end];
    match name.rsplit_once('.') {
        Some((stem, "exe" | "js" | "mjs" | "cjs" | "py" | "sh")) if !stem.is_empty() => {
            stem.to_string()
        }
        _ => name.to_string(),
    }
}

/// The command that resumes `process_name`, from the defaults merged with the
/// user's `resumeCommands` (user entries win; an empty value disables that
/// entry). `None` when nothing is configured for the process or the command
/// contains control characters (it is typed into a shell as one line).
pub fn resume_command(
    process_name: &str,
    user_commands: &BTreeMap<String, String>,
) -> Option<String> {
    let key = normalize_process_name(process_name);
    if key.is_empty() {
        return None;
    }
    let user = user_commands
        .iter()
        .find(|(k, _)| normalize_process_name(k) == key)
        .map(|(_, v)| v.trim());
    let command = match user {
        Some(cmd) => cmd,
        None => DEFAULTS.iter().find(|(k, _)| *k == key)?.1,
    };
    (!command.is_empty() && !command.chars().any(char::is_control)).then(|| command.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn normalises_paths_versions_and_extensions() {
        for (raw, want) in [
            ("claude", "claude"),
            ("/usr/local/bin/Claude", "claude"),
            ("claude-2.1.0", "claude"),
            ("codex@0.5.1", "codex"),
            ("codex.exe", "codex"),
            ("claude.js", "claude"),
            ("C:\\bin\\codex.exe", "codex"),
            ("git-lfs", "git-lfs"),
            ("2.1.45", "2.1.45"),
            ("", ""),
        ] {
            assert_eq!(normalize_process_name(raw), want, "{raw}");
        }
    }

    #[test]
    fn defaults_resolve_after_normalisation() {
        let none = BTreeMap::new();
        assert_eq!(
            resume_command("/opt/claude-2.1.0", &none).as_deref(),
            Some("claude --continue")
        );
        assert_eq!(
            resume_command("codex", &none).as_deref(),
            Some("codex resume --last")
        );
        assert_eq!(resume_command("vim", &none), None);
        assert_eq!(resume_command("", &none), None);
    }

    #[test]
    fn user_entry_overrides_default_and_adds_new_ones() {
        let u = user(&[
            ("claude", " claude -c "),
            ("Aider", "aider --restore-chat-history"),
        ]);
        assert_eq!(resume_command("claude", &u).as_deref(), Some("claude -c"));
        assert_eq!(
            resume_command("aider", &u).as_deref(),
            Some("aider --restore-chat-history")
        );
        assert_eq!(
            resume_command("codex", &u).as_deref(),
            Some("codex resume --last")
        );
    }

    #[test]
    fn empty_user_entry_disables_a_default() {
        let u = user(&[("claude", "")]);
        assert_eq!(resume_command("claude", &u), None);
        assert!(resume_command("codex", &u).is_some());
    }

    #[test]
    fn multi_line_commands_are_rejected() {
        let u = user(&[("evil", "ls\nrm -rf ~")]);
        assert_eq!(resume_command("evil", &u), None);
    }
}
