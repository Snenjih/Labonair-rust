//! Transaction-based undo / redo history.
//!
//! History stores forward and inverse edits, not full document snapshots.
//! Consecutive typing transactions are grouped explicitly until a caret move
//! or another edit source breaks the group; no wall-clock coalescing is used.

use crate::buffer::{EditSource, Transaction};

struct HistoryStep {
    forward: Transaction,
    inverse: Transaction,
}

struct HistoryGroup {
    steps: Vec<HistoryStep>,
}

/// Undo/redo stack for one editor document.
#[derive(Default)]
pub struct History {
    undo: Vec<HistoryGroup>,
    redo: Vec<HistoryGroup>,
    can_coalesce: bool,
}

impl History {
    /// Record one applied transaction and its inverse.
    pub fn record(&mut self, forward: Transaction, inverse: Transaction) {
        let source = forward.source();
        let should_group = self.can_coalesce
            && source == EditSource::Typing
            && self.undo.last().is_some_and(|group| {
                group
                    .steps
                    .iter()
                    .all(|step| step.forward.source() == EditSource::Typing)
            });

        if should_group {
            if let Some(group) = self.undo.last_mut() {
                group.steps.push(HistoryStep { forward, inverse });
            }
        } else {
            self.undo.push(HistoryGroup {
                steps: vec![HistoryStep { forward, inverse }],
            });
        }
        self.redo.clear();
        self.can_coalesce = source == EditSource::Typing;
    }

    /// Force the next transaction to start a new undo group.
    pub fn break_coalescing(&mut self) {
        self.can_coalesce = false;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Move the latest group to redo and return inverse transactions in apply
    /// order (last edit first).
    pub fn undo(&mut self) -> Option<Vec<Transaction>> {
        let group = self.undo.pop()?;
        let inverse = group
            .steps
            .iter()
            .rev()
            .map(|step| step.inverse.clone())
            .collect();
        self.redo.push(group);
        self.can_coalesce = false;
        Some(inverse)
    }

    /// Undo only the most recent typing transaction, keeping earlier typing
    /// in the same group available to one ordinary Undo command.
    pub fn undo_latest_typing_step(&mut self) -> Option<Transaction> {
        let group = self.undo.last_mut()?;
        if group.steps.last()?.forward.source() != EditSource::Typing {
            return None;
        }
        let step = group.steps.pop()?;
        if group.steps.is_empty() {
            self.undo.pop();
        }
        let inverse = step.inverse.clone();
        self.redo.push(HistoryGroup { steps: vec![step] });
        self.can_coalesce = self.undo.last().is_some_and(|group| {
            group
                .steps
                .iter()
                .all(|step| step.forward.source() == EditSource::Typing)
        });
        Some(inverse)
    }

    /// Move the latest group to undo and return forward transactions in apply
    /// order.
    pub fn redo(&mut self) -> Option<Vec<Transaction>> {
        let group = self.redo.pop()?;
        let forward = group
            .steps
            .iter()
            .map(|step| step.forward.clone())
            .collect();
        self.undo.push(group);
        self.can_coalesce = false;
        Some(forward)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.can_coalesce = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::{Anchor, AnchorAffinity, Edit, EditorBuffer, Selection, SelectionSet};

    fn selection() -> SelectionSet {
        SelectionSet::single(Selection::collapsed(Anchor::new(0, AnchorAffinity::After)))
    }

    fn tx(buffer: &EditorBuffer, source: EditSource, text: &str) -> Transaction {
        Transaction::new(
            buffer.revision(),
            vec![Edit::insertion(0, text)],
            selection(),
            selection(),
            source,
            None,
        )
        .expect("valid transaction")
    }

    #[test]
    fn typing_transactions_group_without_timing() {
        let buffer = EditorBuffer::default();
        let mut history = History::default();
        let first = tx(&buffer, EditSource::Typing, "a");
        let second = tx(&buffer, EditSource::Typing, "b");
        history.record(first.clone(), first);
        history.record(second.clone(), second);
        assert_eq!(history.undo().expect("group").len(), 2);
        assert!(!history.can_undo());
    }

    #[test]
    fn non_typing_source_breaks_the_group() {
        let buffer = EditorBuffer::default();
        let mut history = History::default();
        let first = tx(&buffer, EditSource::Typing, "a");
        let delete = tx(&buffer, EditSource::Delete, "b");
        history.record(first.clone(), first);
        history.record(delete.clone(), delete);
        assert_eq!(history.undo().expect("delete").len(), 1);
        assert_eq!(history.undo().expect("typing").len(), 1);
    }

    #[test]
    fn undo_latest_typing_step_keeps_earlier_group_steps_undoable() {
        let buffer = EditorBuffer::default();
        let mut history = History::default();
        let first = tx(&buffer, EditSource::Typing, "a");
        let second = tx(&buffer, EditSource::Typing, "b");
        history.record(first.clone(), first);
        history.record(second.clone(), second);

        assert!(history.undo_latest_typing_step().is_some());
        assert_eq!(history.undo().expect("remaining typing group").len(), 1);
        assert_eq!(history.redo().expect("redo remaining typing").len(), 1);
        assert_eq!(history.redo().expect("redo last typing step").len(), 1);
    }
}
