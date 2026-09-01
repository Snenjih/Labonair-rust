//! An editable document: buffer + caret + selection + undo history + the
//! on-disk baseline used for the dirty flag and (later) diffing.

use std::path::PathBuf;

use crate::buffer::{Position, TextBuffer};
use crate::history::{EditKind, History};
use crate::language::Language;
use crate::search::{self, SearchQuery};

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
    pub buffer: TextBuffer,
    pub cursor: Position,
    /// Selection origin; the selection is `anchor..cursor` when set.
    pub anchor: Option<Position>,
    pub path: Option<PathBuf>,
    pub language: Language,
    /// Contents last known to match disk — the dirty baseline.
    saved_text: String,
    /// Disk mtime (ms) at last load/save, for external-change detection.
    pub disk_mtime: u64,
    /// Set when the file changed on disk since we loaded it.
    pub external_change: bool,
    history: History,
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
            buffer: TextBuffer::default(),
            cursor: Position::default(),
            anchor: None,
            path: None,
            language: Language::PlainText,
            saved_text: String::new(),
            disk_mtime: 0,
            external_change: false,
            history: History::default(),
            goal_column: None,
        }
    }

    pub fn from_file(path: PathBuf, content: &str, mtime: u64) -> Self {
        let language = Language::from_path(&path);
        Self {
            buffer: TextBuffer::from_text(content),
            cursor: Position::default(),
            anchor: None,
            path: Some(path),
            language,
            saved_text: content.to_string(),
            disk_mtime: mtime,
            external_change: false,
            history: History::default(),
            goal_column: None,
        }
    }

    // ── Status ──────────────────────────────────────────────────────────────

    pub fn is_dirty(&self) -> bool {
        self.buffer.text() != self.saved_text
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

    /// Mark the current contents as persisted to disk.
    pub fn mark_saved(&mut self, mtime: u64) {
        self.saved_text = self.buffer.text();
        self.disk_mtime = mtime;
        self.external_change = false;
    }

    /// Note that disk changed under us (watcher / activation re-stat).
    pub fn note_disk_mtime(&mut self, mtime: u64) {
        if mtime != self.disk_mtime {
            self.external_change = true;
        }
    }

    /// Replace the whole buffer with fresh disk contents (external reload).
    pub fn reload(&mut self, content: &str, mtime: u64) {
        self.history
            .record(&self.buffer, self.cursor, EditKind::Barrier);
        self.buffer = TextBuffer::from_text(content);
        self.saved_text = content.to_string();
        self.disk_mtime = mtime;
        self.external_change = false;
        self.cursor = self.buffer.clamp(self.cursor);
        self.anchor = None;
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
        let mut clone = self.buffer.clone();
        Some(clone.delete(s, e))
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(Position::default());
        self.cursor = self.buffer.end();
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

    // ── Editing ─────────────────────────────────────────────────────────────

    fn delete_selection(&mut self) -> bool {
        if let Some((s, e)) = self.selection() {
            self.buffer.delete(s, e);
            self.cursor = s;
            self.anchor = None;
            true
        } else {
            false
        }
    }

    /// Insert text at the caret, replacing any selection.
    pub fn insert(&mut self, text: &str) {
        let kind = if text.contains('\n') || text.chars().count() > 1 {
            EditKind::Barrier
        } else {
            EditKind::Insert
        };
        self.history.record(&self.buffer, self.cursor, kind);
        self.delete_selection();
        self.cursor = self.buffer.insert(self.cursor, text);
        self.goal_column = None;
    }

    /// Backspace: delete the selection, or one char to the left.
    pub fn backspace(&mut self) {
        self.history
            .record(&self.buffer, self.cursor, EditKind::Delete);
        if self.delete_selection() {
            self.goal_column = None;
            return;
        }
        let from = self.prev_position(self.cursor);
        if from != self.cursor {
            self.buffer.delete(from, self.cursor);
            self.cursor = from;
        }
        self.goal_column = None;
    }

    /// Forward delete: selection, or one char to the right.
    pub fn delete_forward(&mut self) {
        self.history
            .record(&self.buffer, self.cursor, EditKind::Delete);
        if self.delete_selection() {
            self.goal_column = None;
            return;
        }
        let to = self.next_position(self.cursor);
        if to != self.cursor {
            self.buffer.delete(self.cursor, to);
        }
        self.goal_column = None;
    }

    pub fn undo(&mut self) {
        if let Some((buf, cur)) = self.history.undo(&self.buffer, self.cursor) {
            self.buffer = buf;
            self.cursor = self.buffer.clamp(cur);
            self.anchor = None;
            self.goal_column = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some((buf, cur)) = self.history.redo(&self.buffer, self.cursor) {
            self.buffer = buf;
            self.cursor = self.buffer.clamp(cur);
            self.anchor = None;
            self.goal_column = None;
        }
    }

    /// Replace every match of `query` with `replacement`; returns the count.
    pub fn replace_all(&mut self, query: &SearchQuery, replacement: &str) -> usize {
        self.history
            .record(&self.buffer, self.cursor, EditKind::Barrier);
        let n = search::replace_all(&mut self.buffer, query, replacement);
        self.cursor = self.buffer.clamp(self.cursor);
        self.anchor = None;
        n
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_tracks_saved_baseline() {
        let mut d = Document::from_file("f.txt".into(), "hello", 1);
        assert!(!d.is_dirty());
        d.move_caret(Motion::DocEnd, false);
        d.insert("!");
        assert!(d.is_dirty());
        d.mark_saved(2);
        assert!(!d.is_dirty());
        assert_eq!(d.text(), "hello!");
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut d = Document::from_file("f".into(), "", 0);
        d.insert("h");
        d.insert("i");
        d.move_caret(Motion::Left, false);
        d.insert("X");
        assert_eq!(d.text(), "hXi");
        d.undo();
        assert_eq!(d.text(), "hi");
        d.undo();
        assert_eq!(d.text(), "");
        d.redo();
        assert_eq!(d.text(), "hi");
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
    }

    #[test]
    fn backspace_merges_lines() {
        let mut d = Document::from_file("f".into(), "ab\ncd", 0);
        d.set_caret(Position::new(1, 0), false);
        d.backspace();
        assert_eq!(d.text(), "abcd");
        assert_eq!(d.cursor, Position::new(0, 2));
    }

    #[test]
    fn external_change_flag() {
        let mut d = Document::from_file("f".into(), "x", 10);
        d.note_disk_mtime(10);
        assert!(!d.external_change);
        d.note_disk_mtime(20);
        assert!(d.external_change);
        d.reload("y", 20);
        assert!(!d.external_change);
        assert_eq!(d.text(), "y");
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
