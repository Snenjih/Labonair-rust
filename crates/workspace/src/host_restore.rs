//! Restore policy for terminal panes whose session may still be alive in the
//! detached session host (ADR 0009, "Restore + resume").
//!
//! Restore order per pane: attach if the session is alive in the daemon, else
//! replay the persisted scrollback into a fresh shell (then the resume hook of
//! [`crate::resume`] applies). Sessions the daemon holds that no restored pane
//! claims are "recovered sessions"; they are offered, never silently leaked.

use labonair_terminal::BackgroundSession;

use crate::session::HostKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostRestore {
    /// The session is alive in the daemon: attach it (no resume banner, the
    /// process is still running).
    Attach(u64),
    /// Fall back to scrollback replay in a fresh session.
    Replay,
}

/// Decide how a pane restores. `alive` is the daemon's session list.
///
/// The stored id only counts when the daemon's session carries the pane's tag
/// (its scrollback uuid): ids restart with every daemon lifetime, so a bare id
/// could name somebody else's session after the daemon was restarted.
pub(crate) fn decide(
    kind: HostKind,
    host_session_id: Option<u64>,
    scrollback_id: Option<&str>,
    alive: &[BackgroundSession],
) -> HostRestore {
    let (HostKind::Detached, Some(id), Some(tag)) = (kind, host_session_id, scrollback_id) else {
        return HostRestore::Replay;
    };
    if alive
        .iter()
        .any(|s| s.id == id && s.pane_tag.as_deref() == Some(tag))
    {
        HostRestore::Attach(id)
    } else {
        HostRestore::Replay
    }
}

/// Sessions to offer as "recovered": nobody holds them (`attached` would mean
/// another window or instance does), no open pane carries their tag and this
/// app has not adopted them already.
pub(crate) fn orphans<'a>(
    alive: &'a [BackgroundSession],
    known_tags: &[String],
    adopted_ids: &[u64],
) -> Vec<&'a BackgroundSession> {
    alive
        .iter()
        .filter(|s| !s.attached && !adopted_ids.contains(&s.id))
        .filter(|s| {
            s.pane_tag
                .as_ref()
                .is_none_or(|tag| !known_tags.contains(tag))
        })
        .collect()
}

/// Title of the "found" notification.
pub(crate) fn found_title(count: usize) -> String {
    match count {
        1 => "1 background session found".to_string(),
        n => format!("{n} background sessions found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: u64, tag: Option<&str>, attached: bool) -> BackgroundSession {
        BackgroundSession {
            id,
            pane_tag: tag.map(str::to_string),
            label: None,
            cwd: None,
            title: None,
            exited: false,
            attached,
            foreground: None,
        }
    }

    #[test]
    fn attaches_only_a_live_session_with_the_panes_tag() {
        let alive = [session(4, Some("sb-a"), false)];
        let d = HostKind::Detached;
        assert_eq!(
            decide(d, Some(4), Some("sb-a"), &alive),
            HostRestore::Attach(4)
        );
        // Gone, or the id now belongs to another pane's session.
        assert_eq!(
            decide(d, Some(5), Some("sb-a"), &alive),
            HostRestore::Replay
        );
        assert_eq!(
            decide(d, Some(4), Some("sb-b"), &alive),
            HostRestore::Replay
        );
        // Nothing to verify against.
        assert_eq!(decide(d, Some(4), None, &alive), HostRestore::Replay);
        assert_eq!(decide(d, None, Some("sb-a"), &alive), HostRestore::Replay);
    }

    #[test]
    fn app_panes_and_an_empty_daemon_always_replay() {
        let alive = [session(4, Some("sb-a"), false)];
        assert_eq!(
            decide(HostKind::App, Some(4), Some("sb-a"), &alive),
            HostRestore::Replay
        );
        assert_eq!(
            decide(HostKind::Detached, Some(4), Some("sb-a"), &[]),
            HostRestore::Replay
        );
    }

    #[test]
    fn orphans_skip_claimed_adopted_and_foreign_sessions() {
        let alive = [
            session(1, Some("open-pane"), false),
            session(2, Some("lost-pane"), false),
            session(3, None, false),
            session(4, Some("other-window"), true),
            session(5, Some("adopted"), false),
        ];
        let found = orphans(&alive, &["open-pane".to_string()], &[5]);
        let ids: Vec<u64> = found.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![2, 3]);
    }

    #[test]
    fn found_title_counts() {
        assert_eq!(found_title(1), "1 background session found");
        assert_eq!(found_title(3), "3 background sessions found");
    }
}
