//! Undo / redo stack.
//!
//! Stores full-document snapshots. Consecutive same-kind edits within a short
//! window coalesce into one undo step so typing a word is a single undo, but a
//! caret move or a delete starts a fresh step.

use std::time::{Duration, Instant};

use crate::buffer::{Position, TextBuffer};

const COALESCE_WINDOW: Duration = Duration::from_millis(600);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Insert,
    Delete,
    /// A change that must never coalesce (paste, external reload, replace-all).
    Barrier,
}

#[derive(Clone)]
struct Snapshot {
    buffer: TextBuffer,
    cursor: Position,
}

#[derive(Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_kind: Option<EditKind>,
    last_at: Option<Instant>,
}

impl History {
    /// Record the pre-edit state, unless it can coalesce with the previous one.
    pub fn record(&mut self, buffer: &TextBuffer, cursor: Position, kind: EditKind) {
        let now = Instant::now();
        let coalesce = kind != EditKind::Barrier
            && self.last_kind == Some(kind)
            && self
                .last_at
                .map(|t| now.duration_since(t) < COALESCE_WINDOW)
                .unwrap_or(false)
            && !self.undo.is_empty();

        if !coalesce {
            self.undo.push(Snapshot {
                buffer: buffer.clone(),
                cursor,
            });
            self.redo.clear();
        }
        self.last_kind = Some(kind);
        self.last_at = Some(now);
    }

    /// Force the next [`record`](Self::record) to start a new step.
    pub fn break_coalescing(&mut self) {
        self.last_kind = None;
        self.last_at = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Pop an undo step, pushing the current state onto the redo stack.
    pub fn undo(
        &mut self,
        current: &TextBuffer,
        cursor: Position,
    ) -> Option<(TextBuffer, Position)> {
        let snap = self.undo.pop()?;
        self.redo.push(Snapshot {
            buffer: current.clone(),
            cursor,
        });
        self.break_coalescing();
        Some((snap.buffer, snap.cursor))
    }

    pub fn redo(
        &mut self,
        current: &TextBuffer,
        cursor: Position,
    ) -> Option<(TextBuffer, Position)> {
        let snap = self.redo.pop()?;
        self.undo.push(Snapshot {
            buffer: current.clone(),
            cursor,
        });
        self.break_coalescing();
        Some((snap.buffer, snap.cursor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_coalesces_but_delete_breaks() {
        let mut h = History::default();
        let b0 = TextBuffer::from_text("");
        h.record(&b0, Position::default(), EditKind::Insert);
        let b1 = TextBuffer::from_text("a");
        h.record(&b1, Position::new(0, 1), EditKind::Insert);
        let b2 = TextBuffer::from_text("ab");
        h.record(&b2, Position::new(0, 2), EditKind::Delete);
        let b3 = TextBuffer::from_text("a");

        // one undo for the whole "ab" typing burst, one for the delete
        let (u1, _) = h.undo(&b3, Position::new(0, 1)).unwrap();
        assert_eq!(u1.text(), "ab");
        let (u2, _) = h.undo(&u1, Position::default()).unwrap();
        assert_eq!(u2.text(), "");
        assert!(!h.can_undo());
    }

    #[test]
    fn redo_restores() {
        let mut h = History::default();
        let b0 = TextBuffer::from_text("x");
        h.record(&b0, Position::default(), EditKind::Barrier);
        let b1 = TextBuffer::from_text("xy");
        let (u, _) = h.undo(&b1, Position::new(0, 2)).unwrap();
        assert_eq!(u.text(), "x");
        let (r, _) = h.redo(&u, Position::default()).unwrap();
        assert_eq!(r.text(), "xy");
    }
}
