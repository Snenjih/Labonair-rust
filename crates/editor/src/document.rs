//! An editable document: the canonical editor buffer plus selection, undo
//! history, and the on-disk baseline used for dirty-state tracking.
//!
//! All document edits go through [`Transaction`] and [`EditorBuffer`]. The
//! public `cursor`/`anchor` fields remain as a compatibility view for the
//! existing GPUI adapter, while transaction selections are the durable
//! undo/redo state.

use std::path::PathBuf;

use crate::buffer::{
    Anchor, AnchorAffinity, BufferSnapshot, Edit, EditSource, EditorBuffer, Position, Selection,
    SelectionSet, TextRange, Transaction,
};
use crate::history::History;
use crate::language::Language;
use crate::lifecycle::{
    FileCapabilities, FileIdentity, FileLifecycle, FileLifecycleError, FileLifecycleEvent,
    FileSessionMetadata, FileSnapshot, FileState, ReloadDecision, SaveIntent,
};
use crate::search::{self, SearchError, SearchQuery};

/// Caret movement primitives (keyboard navigation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
    WordLeft,
    WordRight,
    PageUp(usize),
    PageDown(usize),
}

pub struct Document {
    /// The editor's canonical mutable buffer. All mutations stay inside this
    /// owner and are applied through typed transactions.
    buffer: EditorBuffer,
    pub cursor: Position,
    /// Selection origin; the selection is `anchor..cursor` when set.
    pub anchor: Option<Position>,
    pub path: Option<PathBuf>,
    pub language: Language,
    /// Editor-owned file lifecycle and accepted disk baseline.
    lifecycle: FileLifecycle,
    history: History,
    /// Canonical selection state. The public cursor/anchor fields below are
    /// kept as a one-selection compatibility view for the existing adapters.
    selections: SelectionSet,
    /// Preferred column for vertical motion (sticky like every real editor).
    goal_column: Option<usize>,
}

impl Default for Document {
    fn default() -> Self {
        Self::empty()
    }
}

impl Document {
    pub fn empty() -> Self {
        Self {
            buffer: EditorBuffer::default(),
            cursor: Position::default(),
            anchor: None,
            path: None,
            language: Language::PlainText,
            lifecycle: FileLifecycle::new(),
            history: History::default(),
            selections: SelectionSet::default(),
            goal_column: None,
        }
    }

    pub fn loading(path: PathBuf) -> Self {
        let language = Language::from_path(&path);
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().expect("New -> Loading is valid");
        Self {
            buffer: EditorBuffer::default(),
            cursor: Position::default(),
            anchor: None,
            path: Some(path),
            language,
            lifecycle,
            history: History::default(),
            selections: SelectionSet::default(),
            goal_column: None,
        }
    }

    pub fn from_file(path: PathBuf, content: &str, mtime: u64) -> Self {
        let identity = FileIdentity::from_bytes(path.clone(), mtime, content.as_bytes());
        let snapshot = FileSnapshot::new(
            identity,
            crate::lifecycle::normalize_line_endings(content),
            crate::lifecycle::detect_line_ending(content),
            crate::lifecycle::Encoding::Utf8,
            crate::lifecycle::Bom::None,
            crate::lifecycle::FileCapabilities::editable(),
        );
        Self::from_file_snapshot(snapshot)
    }

    pub fn from_file_snapshot(snapshot: FileSnapshot) -> Self {
        let path = snapshot.identity.path.clone();
        let content = snapshot.text.clone();
        let language = Language::from_path(&path);
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().expect("New -> Loading is valid");
        lifecycle
            .accept_loaded(snapshot)
            .expect("Loading -> Clean/ReadOnly is valid");
        Self {
            buffer: EditorBuffer::from_text(&content),
            cursor: Position::default(),
            anchor: None,
            path: Some(path),
            language,
            lifecycle,
            history: History::default(),
            selections: SelectionSet::default(),
            goal_column: None,
        }
    }

    pub fn from_uneditable(
        path: PathBuf,
        state: FileState,
        identity: Option<FileIdentity>,
        capabilities: FileCapabilities,
    ) -> Self {
        let language = Language::from_path(&path);
        let mut lifecycle = FileLifecycle::new();
        lifecycle.begin_loading().expect("New -> Loading is valid");
        if let Some(identity) = identity {
            lifecycle
                .accept_uneditable(state, identity, capabilities)
                .expect("loading can enter a terminal non-editable state");
        } else {
            lifecycle.fail(state).expect("loading can fail");
        }
        Self {
            buffer: EditorBuffer::default(),
            cursor: Position::default(),
            anchor: None,
            path: Some(path),
            language,
            lifecycle,
            history: History::default(),
            selections: SelectionSet::default(),
            goal_column: None,
        }
    }

    // ── Status ──────────────────────────────────────────────────────────────

    /// Read-only view of one consistent buffer revision.
    pub fn snapshot(&self) -> BufferSnapshot {
        self.buffer.snapshot()
    }

    pub fn is_dirty(&self) -> bool {
        self.lifecycle.is_dirty(&self.buffer.text())
    }

    pub fn file_state(&self) -> &FileState {
        self.lifecycle.state()
    }

    pub fn file_snapshot(&self) -> Option<&FileSnapshot> {
        self.lifecycle.accepted_snapshot()
    }

    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.lifecycle.path()
    }

    pub fn file_capabilities(&self) -> Option<crate::lifecycle::FileCapabilities> {
        self.file_snapshot().map(|snapshot| snapshot.capabilities)
    }

    pub fn can_edit(&self) -> bool {
        self.lifecycle.can_edit()
    }

    pub fn session_file_metadata(&self) -> Option<FileSessionMetadata> {
        self.lifecycle.session_metadata()
    }

    pub fn observed_identity(&self) -> Option<&FileIdentity> {
        self.lifecycle.observed_identity()
    }

    pub fn observe_file(
        &mut self,
        identity: Option<FileIdentity>,
    ) -> Result<Option<FileLifecycleEvent>, FileLifecycleError> {
        self.lifecycle.observe_external(identity)
    }

    /// Record a storage-side lifecycle failure without exposing the lifecycle
    /// implementation to UI adapters.
    pub fn fail_file(&mut self, state: FileState) -> Result<(), FileLifecycleError> {
        self.lifecycle.fail(state)
    }

    pub fn keep_buffer(&mut self) -> Result<FileLifecycleEvent, FileLifecycleError> {
        self.lifecycle.keep_buffer()
    }

    pub fn prepare_save(
        &self,
        current_identity: &FileIdentity,
        intent: SaveIntent,
    ) -> Result<(), FileLifecycleError> {
        self.lifecycle.prepare_save(current_identity, intent)
    }

    pub fn accept_saved(
        &mut self,
        snapshot: FileSnapshot,
    ) -> Result<FileLifecycleEvent, FileLifecycleError> {
        self.lifecycle.accept_saved(snapshot)
    }

    pub fn reload_snapshot(
        &mut self,
        snapshot: FileSnapshot,
        decision: ReloadDecision,
    ) -> Result<FileLifecycleEvent, FileLifecycleError> {
        self.buffer = EditorBuffer::from_text(&snapshot.text);
        let event = self.lifecycle.reload_decision(snapshot, decision)?;
        self.cursor = self.buffer.clamp(self.cursor);
        self.anchor = None;
        self.selections =
            SelectionSet::single(Selection::collapsed(Anchor::new(0, AnchorAffinity::After)));
        self.history.clear();
        self.goal_column = None;
        Ok(event)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Current buffer contents (for saving).
    pub fn text(&self) -> String {
        self.buffer.text()
    }

    // ── Selection ───────────────────────────────────────────────────────────

    pub fn selection(&self) -> Option<(Position, Position)> {
        let a = self.anchor?;
        if a == self.cursor {
            return None;
        }
        Some(if a <= self.cursor {
            (a, self.cursor)
        } else {
            (self.cursor, a)
        })
    }

    pub fn selected_text(&self) -> Option<String> {
        let (s, e) = self.selection()?;
        let snapshot = self.snapshot();
        let range = TextRange::new(snapshot.position_to_byte(s), snapshot.position_to_byte(e));
        Some(snapshot.text_range(range))
    }

    /// Return the canonical selection set, including all secondary cursors.
    /// Single-selection callers continue to observe the compatibility cursor
    /// and anchor fields, including when Vim updates those fields directly.
    pub fn selection_set(&self) -> SelectionSet {
        if self.selections.len() > 1 {
            self.selections.clone()
        } else {
            self.compatibility_selection_set()
        }
    }

    /// Replace the canonical selection set without changing document text.
    pub fn set_selection_set(&mut self, selections: SelectionSet) {
        self.selections = selections;
        self.restore_selection(&self.selections.clone());
        self.history.break_coalescing();
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(Position::default());
        self.cursor = self.buffer.end();
        self.selections = self.compatibility_selection_set();
        self.history.break_coalescing();
    }

    fn begin_or_extend(&mut self, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
    }

    /// Convert the compatibility cursor/anchor view into transaction anchors.
    fn compatibility_selection_set(&self) -> SelectionSet {
        let head = self.buffer.position_to_byte(self.cursor);
        let anchor = self
            .anchor
            .map(|position| self.buffer.position_to_byte(position));
        selection_set_from_offsets(anchor, head)
    }

    /// Restore the compatibility cursor/anchor view from a transaction
    /// selection after its transaction has been applied.
    fn restore_selection(&mut self, selections: &SelectionSet) {
        self.selections = selections.clone();
        let selection = selections.primary();
        let snapshot = self.buffer.snapshot();
        self.cursor = selection.head.resolve(&snapshot);
        let anchor = selection.anchor.resolve(&snapshot);
        self.anchor = (selection.anchor.offset() != selection.head.offset()).then_some(anchor);
        self.cursor = self.buffer.clamp(self.cursor);
        self.anchor = self.anchor.map(|position| self.buffer.clamp(position));
    }

    fn selection_offsets(&self) -> (usize, usize) {
        let cursor = self.buffer.position_to_byte(self.cursor);
        let anchor = self
            .anchor
            .map(|position| self.buffer.position_to_byte(position));
        match anchor {
            Some(anchor) if anchor != cursor => (anchor.min(cursor), anchor.max(cursor)),
            _ => (cursor, cursor),
        }
    }

    // ── Editing ─────────────────────────────────────────────────────────────

    /// Apply one transaction and make its inverse available to History.
    /// `selection_after` is expressed in offsets in the resulting buffer.
    fn apply_edit(
        &mut self,
        edits: Vec<Edit>,
        selection_after: SelectionSet,
        source: EditSource,
        label: Option<String>,
    ) -> bool {
        if edits.is_empty() || !self.lifecycle.can_edit() {
            return false;
        }
        let selection_before = self.selection_set();
        let transaction = match Transaction::new(
            self.buffer.revision(),
            edits,
            selection_before,
            selection_after.clone(),
            source,
            label,
        ) {
            Ok(transaction) => transaction,
            Err(_) => return false,
        };
        let applied = match self.buffer.apply_transaction(&transaction) {
            Ok(applied) => applied,
            Err(_) => return false,
        };
        self.history.record(transaction, applied.inverse);
        self.restore_selection(&selection_after);
        let _ = self.lifecycle.sync_buffer(&self.buffer.text());
        self.goal_column = None;
        true
    }

    /// Apply a typed multi-range transaction. Ranges are expressed in the
    /// current buffer revision and must be ordered and non-overlapping.
    pub fn apply_edits(
        &mut self,
        edits: Vec<Edit>,
        selection_after: SelectionSet,
        source: EditSource,
        label: Option<String>,
    ) -> bool {
        self.apply_edit(edits, selection_after, source, label)
    }

    /// Insert text at the caret, replacing any selection.
    pub fn insert(&mut self, text: &str) {
        let (start, end) = self.selection_offsets();
        let edit = Edit::new(TextRange::new(start, end), text);
        let after = selection_set_from_offsets(None, start.saturating_add(text.len()));
        let source = if text.chars().count() == 1 {
            EditSource::Typing
        } else {
            EditSource::Paste
        };
        self.apply_edit(vec![edit], after, source, None);
    }

    /// Backspace: delete the selection, or one char to the left.
    pub fn backspace(&mut self) {
        let (start, end) = match self.selection() {
            Some(_) => self.selection_offsets(),
            None => {
                let from = self.prev_position(self.cursor);
                if from == self.cursor {
                    return;
                }
                (
                    self.buffer.position_to_byte(from),
                    self.buffer.position_to_byte(self.cursor),
                )
            }
        };
        let after = selection_set_from_offsets(None, start);
        self.apply_edit(
            vec![Edit::new(TextRange::new(start, end), "")],
            after,
            EditSource::Delete,
            None,
        );
    }

    /// Forward delete: selection, or one char to the right.
    pub fn delete_forward(&mut self) {
        let (start, end) = match self.selection() {
            Some(_) => self.selection_offsets(),
            None => {
                let to = self.next_position(self.cursor);
                if to == self.cursor {
                    return;
                }
                (
                    self.buffer.position_to_byte(self.cursor),
                    self.buffer.position_to_byte(to),
                )
            }
        };
        let after = selection_set_from_offsets(None, start);
        self.apply_edit(
            vec![Edit::new(TextRange::new(start, end), "")],
            after,
            EditSource::Delete,
            None,
        );
    }

    pub fn undo(&mut self) {
        let Some(transactions) = self.history.undo() else {
            return;
        };
        let mut final_selection = None;
        for transaction in transactions {
            let rebased = transaction.with_base_revision(self.buffer.revision(), EditSource::Undo);
            match self.buffer.apply_transaction(&rebased) {
                Ok(_) => final_selection = Some(rebased.selection_after().clone()),
                Err(_) => return,
            }
        }
        if let Some(selection) = final_selection {
            self.restore_selection(&selection);
            self.goal_column = None;
        }
        let _ = self.lifecycle.sync_buffer(&self.buffer.text());
    }

    pub fn redo(&mut self) {
        let Some(transactions) = self.history.redo() else {
            return;
        };
        let mut final_selection = None;
        for transaction in transactions {
            let rebased = transaction.with_base_revision(self.buffer.revision(), EditSource::Redo);
            match self.buffer.apply_transaction(&rebased) {
                Ok(_) => final_selection = Some(rebased.selection_after().clone()),
                Err(_) => return,
            }
        }
        if let Some(selection) = final_selection {
            self.restore_selection(&selection);
            self.goal_column = None;
        }
        let _ = self.lifecycle.sync_buffer(&self.buffer.text());
    }

    /// Replace every match of `query` with `replacement`; returns the count.
    pub fn replace_all(&mut self, query: &SearchQuery, replacement: &str) -> usize {
        self.replace_all_checked(query, replacement)
            .unwrap_or_default()
    }

    /// Replace every match as one undoable transaction and preserve regex
    /// validation errors for the in-buffer search UI.
    pub fn replace_all_checked(
        &mut self,
        query: &SearchQuery,
        replacement: &str,
    ) -> Result<usize, SearchError> {
        let snapshot = self.buffer.snapshot();
        let edits = search::replace_all_checked(&snapshot, query, replacement)?;
        if edits.is_empty() {
            return Ok(0);
        }
        let edit_count = edits.len();
        let cursor = self.buffer.position_to_byte(self.cursor);
        let after_cursor = map_offset_through_edits(cursor, &edits, AnchorAffinity::After);
        let after = selection_set_from_offsets(None, after_cursor);
        if self.apply_edit(
            edits,
            after,
            EditSource::Replace,
            Some("Replace all".to_string()),
        ) {
            Ok(edit_count)
        } else {
            Ok(0)
        }
    }

    /// Replace the indexed match as one undoable transaction. The index is in
    /// the order returned by the immutable search snapshot.
    pub fn replace_one(
        &mut self,
        query: &SearchQuery,
        replacement: &str,
        match_index: usize,
    ) -> Result<bool, SearchError> {
        let snapshot = self.buffer.snapshot();
        let Some(edit) = search::replace_one(&snapshot, query, replacement, match_index)? else {
            return Ok(false);
        };
        let after = selection_set_from_offsets(
            None,
            edit.range.start.saturating_add(edit.replacement.len()),
        );
        Ok(self.apply_edit(
            vec![edit],
            after,
            EditSource::Replace,
            Some("Replace one".to_string()),
        ))
    }

    // ── Motion ──────────────────────────────────────────────────────────────

    fn prev_position(&self, p: Position) -> Position {
        if p.column > 0 {
            Position::new(p.line, p.column - 1)
        } else if p.line > 0 {
            Position::new(p.line - 1, self.buffer.line_len(p.line - 1))
        } else {
            p
        }
    }

    fn next_position(&self, p: Position) -> Position {
        if p.column < self.buffer.line_len(p.line) {
            Position::new(p.line, p.column + 1)
        } else if p.line + 1 < self.buffer.line_count() {
            Position::new(p.line + 1, 0)
        } else {
            p
        }
    }

    fn word_boundary(&self, p: Position, forward: bool) -> Position {
        let chars: Vec<char> = self.buffer.line(p.line).chars().collect();
        let is_w = |c: char| c.is_alphanumeric() || c == '_';
        if forward {
            let mut c = p.column;
            while c < chars.len() && !is_w(chars[c]) {
                c += 1;
            }
            while c < chars.len() && is_w(chars[c]) {
                c += 1;
            }
            if c == p.column && p.line + 1 < self.buffer.line_count() {
                return Position::new(p.line + 1, 0);
            }
            Position::new(p.line, c)
        } else {
            if p.column == 0 {
                return self.prev_position(p);
            }
            let mut c = p.column;
            while c > 0 && !is_w(chars[c - 1]) {
                c -= 1;
            }
            while c > 0 && is_w(chars[c - 1]) {
                c -= 1;
            }
            Position::new(p.line, c)
        }
    }

    pub fn move_caret(&mut self, motion: Motion, extend: bool) {
        self.history.break_coalescing();
        self.begin_or_extend(extend);
        let vertical = matches!(
            motion,
            Motion::Up | Motion::Down | Motion::PageUp(_) | Motion::PageDown(_)
        );
        if !vertical {
            self.goal_column = None;
        }

        let last_line = self.buffer.line_count().saturating_sub(1);
        let new = match motion {
            Motion::Left => {
                if let Some((s, _)) = self.selection().filter(|_| !extend) {
                    s
                } else {
                    self.prev_position(self.cursor)
                }
            }
            Motion::Right => {
                if let Some((_, e)) = self.selection().filter(|_| !extend) {
                    e
                } else {
                    self.next_position(self.cursor)
                }
            }
            Motion::Up | Motion::Down | Motion::PageUp(_) | Motion::PageDown(_) => {
                let goal = *self.goal_column.get_or_insert(self.cursor.column);
                let target = match motion {
                    Motion::Up => self.cursor.line.saturating_sub(1),
                    Motion::Down => (self.cursor.line + 1).min(last_line),
                    Motion::PageUp(n) => self.cursor.line.saturating_sub(n),
                    Motion::PageDown(n) => (self.cursor.line + n).min(last_line),
                    _ => unreachable!(),
                };
                Position::new(target, goal.min(self.buffer.line_len(target)))
            }
            Motion::LineStart => Position::new(self.cursor.line, 0),
            Motion::LineEnd => {
                Position::new(self.cursor.line, self.buffer.line_len(self.cursor.line))
            }
            Motion::DocStart => Position::default(),
            Motion::DocEnd => self.buffer.end(),
            Motion::WordLeft => self.word_boundary(self.cursor, false),
            Motion::WordRight => self.word_boundary(self.cursor, true),
        };
        self.cursor = self.buffer.clamp(new);
        if !extend {
            self.anchor = None;
        }
        self.selections = self.compatibility_selection_set();
    }

    /// Place the caret at `pos` (mouse click). `extend` keeps the anchor.
    pub fn set_caret(&mut self, pos: Position, extend: bool) {
        self.history.break_coalescing();
        self.begin_or_extend(extend);
        self.cursor = self.buffer.clamp(pos);
        self.goal_column = None;
        if !extend {
            self.anchor = None;
        }
        self.selections = self.compatibility_selection_set();
    }
}

fn selection_set_from_offsets(anchor: Option<usize>, head: usize) -> SelectionSet {
    let head = Anchor::new(head, AnchorAffinity::After);
    let anchor = anchor
        .map(|offset| Anchor::new(offset, AnchorAffinity::Before))
        .unwrap_or(head);
    SelectionSet::single(Selection { anchor, head })
}

fn map_offset_through_edits(offset: usize, edits: &[Edit], affinity: AnchorAffinity) -> usize {
    let mut anchor = Anchor::new(offset, affinity);
    let mut delta = 0isize;
    for edit in edits {
        let start = if delta.is_negative() {
            edit.range.start.saturating_sub(delta.unsigned_abs())
        } else {
            edit.range.start.saturating_add(delta as usize)
        };
        let end = start.saturating_add(edit.range.len());
        anchor.map_through(TextRange::new(start, end), edit.replacement.len());
        let change = edit.replacement.len() as isize - edit.range.len() as isize;
        delta = delta.saturating_add(change);
    }
    anchor.offset()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(path: &str, text: &str, mtime: u64) -> FileSnapshot {
        FileSnapshot::new(
            FileIdentity::from_bytes(path, mtime, text.as_bytes()),
            crate::lifecycle::normalize_line_endings(text),
            crate::lifecycle::detect_line_ending(text),
            crate::lifecycle::Encoding::Utf8,
            crate::lifecycle::Bom::None,
            crate::lifecycle::FileCapabilities::editable(),
        )
    }

    #[test]
    fn dirty_tracks_saved_baseline() {
        let mut d = Document::from_file("f.txt".into(), "hello", 1);
        assert!(!d.is_dirty());
        d.move_caret(Motion::DocEnd, false);
        d.insert("!");
        assert!(d.is_dirty());
        d.accept_saved(snapshot("f.txt", &d.text(), 2)).unwrap();
        assert!(!d.is_dirty());
        assert_eq!(d.text(), "hello!");
    }

    #[test]
    fn snapshot_boundary_is_read_only_and_revision_consistent() {
        let mut d = Document::from_file("f".into(), "before", 0);
        let before = d.snapshot();
        assert_eq!(before.text(), "before");

        d.set_caret(before.end(), false);
        d.insert(" after");

        assert_eq!(before.text(), "before");
        assert_eq!(before.revision().value(), 0);
        let after = d.snapshot();
        assert_eq!(after.text(), "before after");
        assert_eq!(after.revision().value(), 1);
    }

    #[test]
    fn undo_redo_roundtrip_restores_transaction_selections() {
        let mut d = Document::from_file("f".into(), "", 0);
        d.insert("h");
        d.insert("i");
        d.move_caret(Motion::Left, false);
        d.insert("X");
        assert_eq!(d.text(), "hXi");
        assert_eq!(d.cursor, Position::new(0, 2));
        d.undo();
        assert_eq!(d.text(), "hi");
        assert_eq!(d.cursor, Position::new(0, 1));
        d.undo();
        assert_eq!(d.text(), "");
        assert_eq!(d.cursor, Position::default());
        d.redo();
        assert_eq!(d.text(), "hi");
        assert_eq!(d.cursor, Position::new(0, 2));
    }

    #[test]
    fn selection_replace_on_insert() {
        let mut d = Document::from_file("f".into(), "abcdef", 0);
        d.set_caret(Position::new(0, 1), false);
        d.set_caret(Position::new(0, 4), true);
        assert_eq!(d.selected_text().as_deref(), Some("bcd"));
        d.insert("X");
        assert_eq!(d.text(), "aXef");
        assert_eq!(d.cursor, Position::new(0, 2));
        d.undo();
        assert_eq!(d.text(), "abcdef");
        assert_eq!(
            d.selection(),
            Some((Position::new(0, 1), Position::new(0, 4)))
        );
    }

    #[test]
    fn backspace_merges_lines_and_undoes() {
        let mut d = Document::from_file("f".into(), "ab\ncd", 0);
        d.set_caret(Position::new(1, 0), false);
        d.backspace();
        assert_eq!(d.text(), "abcd");
        assert_eq!(d.cursor, Position::new(0, 2));
        d.undo();
        assert_eq!(d.text(), "ab\ncd");
        assert_eq!(d.cursor, Position::new(1, 0));
    }

    #[test]
    fn delete_forward_and_replace_all_are_transactional() {
        let mut d = Document::from_file("f".into(), "one one", 0);
        d.set_caret(Position::new(0, 3), false);
        d.delete_forward();
        assert_eq!(d.text(), "oneone");
        d.undo();
        assert_eq!(d.text(), "one one");
        let count = d.replace_all(
            &SearchQuery {
                text: "one".into(),
                ..Default::default()
            },
            "two",
        );
        assert_eq!(count, 2);
        assert_eq!(d.text(), "two two");
        d.undo();
        assert_eq!(d.text(), "one one");
        d.redo();
        assert_eq!(d.text(), "two two");
    }

    #[test]
    fn reload_resets_revision_history_and_baseline() {
        let mut d = Document::from_file("f".into(), "old", 1);
        d.set_caret(Position::new(0, 3), false);
        d.insert("!");
        assert!(d.can_undo());
        d.observe_file(Some(FileIdentity::from_bytes("f", 2, b"new\r\ntext")))
            .unwrap();
        d.reload_snapshot(
            snapshot("f", "new\r\ntext", 2),
            ReloadDecision::DiscardBuffer,
        )
        .unwrap();
        assert!(!d.is_dirty());
        assert!(!d.can_undo());
        assert_eq!(d.text(), "new\ntext");
        let snapshot = d.snapshot();
        assert_eq!(snapshot.line(0), "new");
        assert_eq!(snapshot.line(1), "text");
    }

    #[test]
    fn unicode_and_eof_edit_boundaries_are_safe() {
        let mut d = Document::from_file("f".into(), "é終", 0);
        d.set_caret(Position::new(0, 2), false);
        d.insert("λ");
        assert_eq!(d.text(), "é終λ");
        d.set_caret(d.snapshot().end(), false);
        d.backspace();
        assert_eq!(d.text(), "é終");
        d.set_caret(Position::default(), false);
        d.backspace();
        assert_eq!(d.text(), "é終");
    }

    #[test]
    fn vertical_motion_keeps_goal_column() {
        let mut d = Document::from_file("f".into(), "longline\nab\nlongline", 0);
        d.set_caret(Position::new(0, 7), false);
        d.move_caret(Motion::Down, false);
        assert_eq!(d.cursor, Position::new(1, 2));
        d.move_caret(Motion::Down, false);
        assert_eq!(d.cursor, Position::new(2, 7));
    }
}
