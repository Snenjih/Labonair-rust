//! Resume-on-restore policy (Option D of ADR 0009): which command a restored
//! terminal pane should offer or run to re-open its former foreground program.
//!
//! The command is resolved at snapshot time and stored in the session file;
//! `terminal.resumeOnRestore` is evaluated at restore time, so changing the
//! setting between save and restore takes effect on the next restore.

use std::collections::BTreeMap;

use labonair_settings::content::terminal::ResumeOnRestore;
use labonair_terminal::resume::resume_command;

/// What a restoring pane does with its stored resume command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resume {
    None,
    /// Show a one-click banner; the user decides.
    Offer(String),
    /// Type the command into the fresh shell right away.
    Run(String),
}

pub(crate) fn decide(mode: ResumeOnRestore, stored: Option<&str>) -> Resume {
    let Some(command) = stored.map(str::trim).filter(|c| !c.is_empty()) else {
        return Resume::None;
    };
    match mode {
        ResumeOnRestore::Off => Resume::None,
        ResumeOnRestore::Ask => Resume::Offer(command.to_string()),
        ResumeOnRestore::Auto => Resume::Run(command.to_string()),
    }
}

/// The resume command to persist for a pane at snapshot time. `foreground` is
/// the name of the job holding the tty (`None` while the shell is idle).
/// Remote panes never get one, and neither does a pane without a known cwd:
/// tools like `claude --continue` resume per directory, so the re-spawned
/// shell would otherwise continue the wrong conversation.
pub(crate) fn snapshot_command(
    remote: bool,
    cwd: Option<&str>,
    foreground: Option<&str>,
    user_commands: &BTreeMap<String, String>,
) -> Option<String> {
    if remote || cwd.is_none_or(|c| c.trim().is_empty()) {
        return None;
    }
    resume_command(foreground?, user_commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn mode_decides_what_happens_to_the_stored_command() {
        let c = Some("claude --continue");
        assert_eq!(decide(ResumeOnRestore::Off, c), Resume::None);
        assert_eq!(
            decide(ResumeOnRestore::Ask, c),
            Resume::Offer("claude --continue".into())
        );
        assert_eq!(
            decide(ResumeOnRestore::Auto, c),
            Resume::Run("claude --continue".into())
        );
        for mode in [ResumeOnRestore::Ask, ResumeOnRestore::Auto] {
            assert_eq!(decide(mode, None), Resume::None);
            assert_eq!(decide(mode, Some("  ")), Resume::None);
        }
    }

    #[test]
    fn snapshot_stores_the_resolved_command_only() {
        assert_eq!(
            snapshot_command(false, Some("/p"), Some("claude"), &none()).as_deref(),
            Some("claude --continue")
        );
        // Idle shell, or an unknown program.
        assert_eq!(snapshot_command(false, Some("/p"), None, &none()), None);
        assert_eq!(
            snapshot_command(false, Some("/p"), Some("vim"), &none()),
            None
        );
    }

    #[test]
    fn snapshot_skips_remote_and_cwd_less_panes() {
        assert_eq!(
            snapshot_command(true, Some("/p"), Some("claude"), &none()),
            None
        );
        assert_eq!(snapshot_command(false, None, Some("claude"), &none()), None);
        assert_eq!(
            snapshot_command(false, Some(" "), Some("claude"), &none()),
            None
        );
    }

    #[test]
    fn snapshot_honours_user_overrides_and_disables() {
        let mut u = none();
        u.insert("claude".into(), String::new());
        u.insert("codex".into(), "codex resume".into());
        assert_eq!(
            snapshot_command(false, Some("/p"), Some("claude"), &u),
            None
        );
        assert_eq!(
            snapshot_command(false, Some("/p"), Some("codex"), &u).as_deref(),
            Some("codex resume")
        );
    }
}
