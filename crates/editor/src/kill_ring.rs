//! Bounded in-memory history for editor Kill Ring Cut and Yank commands.

use std::collections::VecDeque;

pub const KILL_RING_CAPACITY: usize = 16;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KillRing {
    entries: VecDeque<String>,
    next_yank: usize,
}

impl KillRing {
    pub fn push(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        self.entries.push_front(text);
        self.entries.truncate(KILL_RING_CAPACITY);
        self.next_yank = 0;
    }

    /// Return the newest entry first, then cycle through older entries.
    pub fn yank(&mut self) -> Option<String> {
        let entry = self.entries.get(self.next_yank)?.clone();
        self.next_yank = (self.next_yank + 1) % self.entries.len();
        Some(entry)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yanks_newest_first_and_cycles_through_history() {
        let mut ring = KillRing::default();
        ring.push("first".into());
        ring.push("second".into());

        assert_eq!(ring.yank().as_deref(), Some("second"));
        assert_eq!(ring.yank().as_deref(), Some("first"));
        assert_eq!(ring.yank().as_deref(), Some("second"));
    }

    #[test]
    fn ignores_empty_entries_and_keeps_a_bounded_history() {
        let mut ring = KillRing::default();
        ring.push(String::new());
        assert!(ring.is_empty());
        for entry in 0..=KILL_RING_CAPACITY {
            ring.push(entry.to_string());
        }
        assert_eq!(ring.len(), KILL_RING_CAPACITY);
        assert_eq!(ring.yank().as_deref(), Some("16"));
    }
}
