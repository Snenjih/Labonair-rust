//! Bounded, in-memory history for explicit editor location jumps.

use crate::buffer::Position;

/// A cursor and viewport stop in one editor document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationLocation {
    pub position: Position,
    pub scroll_top: usize,
    pub scroll_left: usize,
}

impl NavigationLocation {
    pub const fn new(position: Position, scroll_top: usize, scroll_left: usize) -> Self {
        Self {
            position,
            scroll_top,
            scroll_left,
        }
    }
}

/// Back/forward stops for explicit jumps in an editor view.
///
/// Ordinary cursor movement does not create entries. Call
/// [`record_transition`](Self::record_transition) when a command moves the
/// editor to a distinct location. History is intentionally bounded so long
/// navigation sessions cannot grow without limit.
#[derive(Clone, Debug)]
pub struct NavigationHistory {
    entries: Vec<NavigationLocation>,
    cursor: Option<usize>,
    capacity: usize,
}

impl Default for NavigationHistory {
    fn default() -> Self {
        Self::with_capacity(100)
    }
}

impl NavigationHistory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: None,
            capacity: capacity.max(2),
        }
    }

    /// Record a jump from `from` to `to`, dropping any forward branch.
    pub fn record_transition(&mut self, from: NavigationLocation, to: NavigationLocation) {
        if from == to {
            return;
        }

        if let Some(cursor) = self.cursor {
            self.entries.truncate(cursor + 1);
        }
        if self.entries.last() != Some(&from) {
            self.entries.push(from);
        }
        if self.entries.last() != Some(&to) {
            self.entries.push(to);
        }
        self.cursor = self.entries.len().checked_sub(1);

        while self.entries.len() > self.capacity {
            self.entries.remove(0);
            self.cursor = self.cursor.map(|cursor| cursor.saturating_sub(1));
        }
    }

    pub fn back(&mut self) -> Option<NavigationLocation> {
        let cursor = self.cursor?;
        if cursor == 0 {
            return None;
        }
        let previous = cursor - 1;
        self.cursor = Some(previous);
        self.entries.get(previous).copied()
    }

    pub fn can_back(&self) -> bool {
        self.cursor.is_some_and(|cursor| cursor > 0)
    }

    pub fn forward(&mut self) -> Option<NavigationLocation> {
        let next = self.cursor?.checked_add(1)?;
        let location = self.entries.get(next).copied()?;
        self.cursor = Some(next);
        Some(location)
    }

    pub fn can_forward(&self) -> bool {
        self.cursor
            .and_then(|cursor| cursor.checked_add(1))
            .is_some_and(|next| next < self.entries.len())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(line: usize) -> NavigationLocation {
        NavigationLocation::new(Position::new(line, 2), line.saturating_sub(3), 1)
    }

    #[test]
    fn back_and_forward_restore_cursor_and_viewport_stops() {
        let mut history = NavigationHistory::default();
        assert!(!history.can_back());
        assert!(!history.can_forward());
        history.record_transition(location(2), location(12));
        history.record_transition(location(20), location(30));
        assert!(history.can_back());
        assert!(!history.can_forward());

        assert_eq!(history.back(), Some(location(20)));
        assert!(history.can_forward());
        assert_eq!(history.back(), Some(location(12)));
        assert_eq!(history.back(), Some(location(2)));
        assert_eq!(history.back(), None);
        assert_eq!(history.forward(), Some(location(12)));
        assert_eq!(history.forward(), Some(location(20)));
        assert_eq!(history.forward(), Some(location(30)));
        assert_eq!(history.forward(), None);
    }

    #[test]
    fn a_new_jump_after_back_discards_the_forward_branch() {
        let mut history = NavigationHistory::default();
        history.record_transition(location(1), location(10));
        history.record_transition(location(20), location(30));

        assert_eq!(history.back(), Some(location(20)));
        history.record_transition(location(20), location(40));

        assert_eq!(history.back(), Some(location(20)));
        assert_eq!(history.back(), Some(location(10)));
        assert_eq!(history.forward(), Some(location(20)));
        assert_eq!(history.forward(), Some(location(40)));
        assert_eq!(history.forward(), None);
    }

    #[test]
    fn duplicate_locations_do_not_create_stops_and_history_is_bounded() {
        let mut history = NavigationHistory::with_capacity(3);
        history.record_transition(location(1), location(1));
        history.record_transition(location(1), location(2));
        history.record_transition(location(2), location(3));
        history.record_transition(location(3), location(4));

        assert_eq!(history.back(), Some(location(3)));
        assert_eq!(history.back(), Some(location(2)));
        assert_eq!(history.back(), None);
    }
}
