//! Restore policy for terminal panes whose session may still be alive in the
//! detached session host (ADR 0009, "Restore + resume").
//!
//! Restore order per pane: attach if the session is alive in the daemon, else
//! replay the persisted scrollback into a fresh shell (then the resume hook of
//! [`crate::resume`] applies). Sessions the daemon holds that no restored pane
//! claims are "recovered sessions"; they are offered, never silently leaked.

use labonair_terminal::BackgroundSession;

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
/// A session counts when it carries the pane's tag (its scrollback uuid, unique
/// per pane); the stored id is only a tie-breaker, because ids restart with
/// every daemon lifetime and the snapshot may be older than the session (the
/// app was killed before it saved). Replaying while a session with the tag is
/// alive would leave that session running unseen.
pub(crate) fn decide(
    host_session_id: Option<u64>,
    scrollback_id: Option<&str>,
    alive: &[BackgroundSession],
) -> HostRestore {
    let Some(tag) = scrollback_id else {
        return HostRestore::Replay;
    };
    alive
        .iter()
        .filter(|s| s.pane_tag.as_deref() == Some(tag))
        .max_by_key(|s| (Some(s.id) == host_session_id, !s.attached, s.id))
        .map_or(HostRestore::Replay, |s| HostRestore::Attach(s.id))
}

/// Sessions to offer as "recovered": nobody holds them (`attached` would mean
/// another window or instance does) and this app has not adopted them. A
/// session whose tag an open pane carries still counts: restore adopts at most
/// one session per tag, any twin is a leftover.
pub(crate) fn orphans<'a>(
    alive: &'a [BackgroundSession],
    adopted_ids: &[u64],
) -> Vec<&'a BackgroundSession> {
    alive
        .iter()
        .filter(|s| !s.attached && !adopted_ids.contains(&s.id))
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
    fn attaches_the_live_session_carrying_the_panes_tag() {
        let alive = [session(4, Some("sb-a"), false)];
        assert_eq!(
            decide(Some(4), Some("sb-a"), &alive),
            HostRestore::Attach(4)
        );
        // A stale snapshot (id gone or missing) still finds the session by tag.
        assert_eq!(
            decide(Some(5), Some("sb-a"), &alive),
            HostRestore::Attach(4)
        );
        assert_eq!(decide(None, Some("sb-a"), &alive), HostRestore::Attach(4));
        // Another pane's tag, no tag, or an empty daemon: replay.
        assert_eq!(decide(Some(4), Some("sb-b"), &alive), HostRestore::Replay);
        assert_eq!(decide(Some(4), None, &alive), HostRestore::Replay);
        assert_eq!(decide(Some(4), Some("sb-a"), &[]), HostRestore::Replay);
    }

    #[test]
    fn twins_prefer_the_stored_id_then_a_free_then_the_newest_session() {
        let alive = [
            session(3, Some("sb-a"), false),
            session(6, Some("sb-a"), true),
            session(7, Some("sb-a"), false),
        ];
        assert_eq!(
            decide(Some(3), Some("sb-a"), &alive),
            HostRestore::Attach(3)
        );
        assert_eq!(decide(None, Some("sb-a"), &alive), HostRestore::Attach(7));
    }

    #[test]
    fn orphans_skip_adopted_and_foreign_sessions_but_keep_twins() {
        let alive = [
            session(1, Some("open-pane"), false),
            session(2, Some("lost-pane"), false),
            session(3, None, false),
            session(4, Some("other-window"), true),
            session(5, Some("open-pane"), false),
        ];
        // 5 was adopted by the pane tagged "open-pane": its twin 1 is a leftover.
        let found = orphans(&alive, &[5]);
        let ids: Vec<u64> = found.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn found_title_counts() {
        assert_eq!(found_title(1), "1 background session found");
        assert_eq!(found_title(3), "3 background sessions found");
    }
}
