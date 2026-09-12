//! The editor's UI-free text model.
//!
//! `EditorBuffer` is the only mutable text store in the editor capability. It
//! uses a UTF-8 rope for logarithmic edits and cheap structural clones.
//! Readers receive [`BufferSnapshot`] values; they never receive the mutable
//! rope. Readers receive [`BufferSnapshot`] values and mutations are applied
//! through validated [`Transaction`] values.

use std::fmt;
use std::ops::Range;

use ropey::{LineType, Rope};
use serde::{Deserialize, Serialize};

/// A caret / selection endpoint. `column` counts Unicode scalar values, not
/// UTF-8 bytes. `line` is zero based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Monotonically increasing revision of a buffer.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Revision(u64);

impl Revision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// A byte range in the canonical UTF-8 text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    fn as_range(self) -> Range<usize> {
        self.start..self.end
    }
}

/// Affinity determines which side of an insertion an anchor stays on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnchorAffinity {
    /// Stay before text inserted exactly at this offset.
    Before,
    /// Move after text inserted exactly at this offset.
    After,
}

/// A stable byte offset that can be mapped through edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Anchor {
    offset: usize,
    affinity: AnchorAffinity,
}

impl Anchor {
    pub const fn new(offset: usize, affinity: AnchorAffinity) -> Self {
        Self { offset, affinity }
    }

    pub const fn offset(self) -> usize {
        self.offset
    }

    pub const fn affinity(self) -> AnchorAffinity {
        self.affinity
    }

    pub fn resolve(self, snapshot: &BufferSnapshot) -> Position {
        snapshot.byte_to_position(self.offset)
    }

    /// Apply one edit to this anchor using deterministic boundary semantics.
    pub fn map_through(&mut self, range: TextRange, inserted_len: usize) {
        if range.is_empty() {
            if self.offset > range.start
                || (self.offset == range.start && self.affinity == AnchorAffinity::After)
            {
                self.offset = self.offset.saturating_add(inserted_len);
            }
            return;
        }

        if self.offset < range.start {
            return;
        }
        if self.offset > range.end {
            self.offset = self
                .offset
                .saturating_sub(range.len())
                .saturating_add(inserted_len);
            return;
        }

        self.offset = match self.affinity {
            AnchorAffinity::Before => range.start,
            AnchorAffinity::After => range.start.saturating_add(inserted_len),
        };
    }
}

/// A range represented by stable anchors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorRange {
    pub start: Anchor,
    pub end: Anchor,
}

impl AnchorRange {
    pub const fn new(start: Anchor, end: Anchor) -> Self {
        Self { start, end }
    }

    pub fn resolve(self, snapshot: &BufferSnapshot) -> TextRange {
        let a = self.start.offset().min(snapshot.byte_len());
        let b = self.end.offset().min(snapshot.byte_len());
        if a <= b {
            TextRange::new(a, b)
        } else {
            TextRange::new(b, a)
        }
    }

    pub fn map_through(&mut self, range: TextRange, inserted_len: usize) {
        self.start.map_through(range, inserted_len);
        self.end.map_through(range, inserted_len);
    }
}

/// One selection. `head` is the active caret and `anchor` is its fixed origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Selection {
    pub anchor: Anchor,
    pub head: Anchor,
}

impl Selection {
    pub const fn collapsed(at: Anchor) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    pub const fn range(self) -> AnchorRange {
        AnchorRange::new(self.anchor, self.head)
    }
}

/// A deterministic selection collection. The first entry is the primary
/// selection; multi-cursor editing can extend this without changing the
/// buffer contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary: usize,
}

impl SelectionSet {
    pub fn single(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary: 0,
        }
    }

    /// Build a selection set from document-order selections. Empty input is
    /// represented by one caret at the start of the document, which keeps the
    /// invariant that an editor always has a primary selection.
    pub fn from_selections(mut selections: Vec<Selection>, primary: usize) -> Self {
        if selections.is_empty() {
            return Self::default();
        }

        let primary_range = selections
            .get(primary.min(selections.len() - 1))
            .map(|selection| {
                (
                    selection.anchor.offset().min(selection.head.offset()),
                    selection.anchor.offset().max(selection.head.offset()),
                )
            });

        selections.sort_by_key(|selection| {
            (
                selection.anchor.offset().min(selection.head.offset()),
                selection.anchor.offset().max(selection.head.offset()),
            )
        });

        let mut unique = Vec::with_capacity(selections.len());
        for selection in selections {
            let range = (
                selection.anchor.offset().min(selection.head.offset()),
                selection.anchor.offset().max(selection.head.offset()),
            );
            let overlaps = unique.last().is_some_and(|previous: &Selection| {
                let previous_end = previous.anchor.offset().max(previous.head.offset());
                let previous_start = previous.anchor.offset().min(previous.head.offset());
                previous_end > range.0
                    || (previous_start == previous_end
                        && range.0 == range.1
                        && previous_start == range.0)
            });
            if overlaps {
                let previous = unique.pop().expect("overlap implies a previous selection");
                let start = previous
                    .anchor
                    .offset()
                    .min(previous.head.offset())
                    .min(range.0);
                let end = previous
                    .anchor
                    .offset()
                    .max(previous.head.offset())
                    .max(range.1);
                unique.push(Selection {
                    anchor: Anchor::new(start, AnchorAffinity::Before),
                    head: Anchor::new(end, AnchorAffinity::After),
                });
            } else {
                unique.push(selection);
            }
        }
        let primary = primary_range
            .and_then(|range| {
                unique.iter().position(|selection| {
                    selection.anchor.offset().min(selection.head.offset()) <= range.0
                        && selection.anchor.offset().max(selection.head.offset()) >= range.1
                })
            })
            .unwrap_or(0);
        Self {
            selections: unique,
            primary,
        }
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<Selection> {
        self.selections.get(index).copied()
    }

    pub fn with_primary(mut self, primary: usize) -> Self {
        if !self.selections.is_empty() {
            self.primary = primary.min(self.selections.len() - 1);
        }
        self
    }

    /// Resolve all selections against one immutable buffer snapshot.
    pub fn ranges(&self, snapshot: &BufferSnapshot) -> Vec<TextRange> {
        self.selections
            .iter()
            .map(|selection| selection.range().resolve(snapshot))
            .collect()
    }

    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    pub fn primary(&self) -> Selection {
        self.selections
            .get(self.primary)
            .copied()
            .unwrap_or_else(|| Selection::collapsed(Anchor::new(0, AnchorAffinity::After)))
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn map_through(&mut self, range: TextRange, inserted_len: usize) {
        for selection in &mut self.selections {
            selection.anchor.map_through(range, inserted_len);
            selection.head.map_through(range, inserted_len);
        }
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::single(Selection::collapsed(Anchor::new(0, AnchorAffinity::After)))
    }
}

/// The origin of an edit, used for deterministic undo grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditSource {
    Typing,
    Completion,
    Delete,
    Paste,
    Replace,
    Format,
    External,
    Undo,
    Redo,
}

/// One edit expressed in byte offsets of the transaction's base revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub range: TextRange,
    pub replacement: String,
}

impl Edit {
    pub fn new(range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            range,
            replacement: replacement.into(),
        }
    }

    pub fn insertion(offset: usize, replacement: impl Into<String>) -> Self {
        Self::new(TextRange::new(offset, offset), replacement)
    }
}

/// A validated, ordered group of edits and its selection boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    base_revision: Revision,
    edits: Vec<Edit>,
    selection_before: SelectionSet,
    selection_after: SelectionSet,
    source: EditSource,
    label: Option<String>,
}

impl Transaction {
    pub fn new(
        base_revision: Revision,
        edits: Vec<Edit>,
        selection_before: SelectionSet,
        selection_after: SelectionSet,
        source: EditSource,
        label: Option<String>,
    ) -> Result<Self, EditorError> {
        validate_edits(&edits)?;
        if edits.is_empty() {
            return Err(EditorError::EmptyTransaction);
        }
        Ok(Self {
            base_revision,
            edits,
            selection_before,
            selection_after,
            source,
            label,
        })
    }

    pub fn base_revision(&self) -> Revision {
        self.base_revision
    }

    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    pub fn selection_before(&self) -> &SelectionSet {
        &self.selection_before
    }

    pub fn selection_after(&self) -> &SelectionSet {
        &self.selection_after
    }

    pub fn source(&self) -> EditSource {
        self.source
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn with_base_revision(&self, revision: Revision, source: EditSource) -> Self {
        let mut copy = self.clone();
        copy.base_revision = revision;
        copy.source = source;
        copy
    }
}

/// A single applied edit, suitable for incremental syntax/search consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDelta {
    pub range: TextRange,
    pub deleted_len: usize,
    pub inserted_len: usize,
}

/// Typed event emitted by an editor owner after a transaction is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferEvent {
    TransactionApplied {
        before: Revision,
        after: Revision,
        source: EditSource,
        deltas: Vec<EditDelta>,
    },
}

/// Errors returned by the UI-free text contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    StaleRevision {
        expected: Revision,
        actual: Revision,
    },
    InvalidRange(TextRange),
    InvalidEditOrder,
    OverlappingEdits,
    InvalidUtf8Boundary(usize),
    EmptyTransaction,
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleRevision { expected, actual } => write!(
                f,
                "transaction targets revision {} but buffer is at {}",
                expected.value(),
                actual.value()
            ),
            Self::InvalidRange(range) => {
                write!(f, "invalid text range {}..{}", range.start, range.end)
            }
            Self::InvalidEditOrder => write!(f, "transaction edits are not ordered"),
            Self::OverlappingEdits => write!(f, "transaction edits overlap"),
            Self::InvalidUtf8Boundary(offset) => {
                write!(f, "offset {offset} is not a UTF-8 boundary")
            }
            Self::EmptyTransaction => write!(f, "transaction contains no edits"),
        }
    }
}

impl std::error::Error for EditorError {}

/// Immutable result of an applied transaction.
#[derive(Debug, Clone)]
pub struct AppliedTransaction {
    pub revision_before: Revision,
    pub revision_after: Revision,
    pub inverse: Transaction,
    pub deltas: Vec<EditDelta>,
    pub event: BufferEvent,
}

/// Mutable canonical text storage.
pub struct EditorBuffer {
    rope: Rope,
    revision: Revision,
}

impl Clone for EditorBuffer {
    fn clone(&self) -> Self {
        Self {
            rope: self.rope.clone(),
            revision: self.revision,
        }
    }
}

impl EditorBuffer {
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            revision: Revision::default(),
        }
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot {
            rope: self.rope.clone(),
            revision: self.revision,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn byte_len(&self) -> usize {
        self.rope.len()
    }

    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines(LineType::LF)
    }

    /// Return a logical line without its LF or CRLF terminator.
    pub fn line(&self, index: usize) -> String {
        line_text(&self.rope, index)
    }

    pub fn line_len(&self, index: usize) -> usize {
        self.line(index).chars().count()
    }

    pub fn end(&self) -> Position {
        let line = self.line_count().saturating_sub(1);
        Position::new(line, self.line_len(line))
    }

    pub fn clamp(&self, position: Position) -> Position {
        let line = position.line.min(self.line_count().saturating_sub(1));
        Position::new(line, position.column.min(self.line_len(line)))
    }

    pub fn position_to_byte(&self, position: Position) -> usize {
        position_to_byte(&self.rope, self.clamp(position))
    }

    pub fn byte_to_position(&self, byte: usize) -> Position {
        byte_to_position(&self.rope, byte)
    }

    /// Apply a transaction against exactly its base revision.
    pub fn apply_transaction(
        &mut self,
        transaction: &Transaction,
    ) -> Result<AppliedTransaction, EditorError> {
        if transaction.base_revision != self.revision {
            return Err(EditorError::StaleRevision {
                expected: transaction.base_revision,
                actual: self.revision,
            });
        }
        validate_edits(&transaction.edits)?;
        for edit in &transaction.edits {
            validate_range(&self.rope, edit.range)?;
        }

        let before = self.revision;
        let mut inverse_edits = Vec::with_capacity(transaction.edits.len());
        let mut deltas = Vec::with_capacity(transaction.edits.len());
        let mut prior_delta: isize = 0;

        for edit in &transaction.edits {
            let deleted = self.rope.slice(edit.range.as_range()).to_string();
            let final_start = shifted_offset(edit.range.start, prior_delta);
            inverse_edits.push(Edit::new(
                TextRange::new(
                    final_start,
                    final_start.saturating_add(edit.replacement.len()),
                ),
                deleted,
            ));
            let delta = edit.replacement.len() as isize - edit.range.len() as isize;
            prior_delta = prior_delta.saturating_add(delta);
            deltas.push(EditDelta {
                range: edit.range,
                deleted_len: edit.range.len(),
                inserted_len: edit.replacement.len(),
            });
        }

        for edit in transaction.edits.iter().rev() {
            self.rope.remove(edit.range.as_range());
            if !edit.replacement.is_empty() {
                self.rope.insert(edit.range.start, &edit.replacement);
            }
        }

        let after = before.next();
        self.revision = after;
        let inverse = Transaction::new(
            after,
            inverse_edits,
            transaction.selection_after.clone(),
            transaction.selection_before.clone(),
            EditSource::Undo,
            transaction.label.clone(),
        )?;
        let event = BufferEvent::TransactionApplied {
            before,
            after,
            source: transaction.source,
            deltas: deltas.clone(),
        };
        Ok(AppliedTransaction {
            revision_before: before,
            revision_after: after,
            inverse,
            deltas,
            event,
        })
    }
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::from_text("")
    }
}

impl fmt::Debug for EditorBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditorBuffer")
            .field("byte_len", &self.byte_len())
            .field("revision", &self.revision)
            .finish()
    }
}

/// Immutable, cheap-to-clone view of a buffer revision.
#[derive(Clone)]
pub struct BufferSnapshot {
    rope: Rope,
    revision: Revision,
}

impl BufferSnapshot {
    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Return text from a byte range in this exact snapshot revision.
    pub fn text_range(&self, range: TextRange) -> String {
        let start = range.start.min(self.rope.len());
        let end = range.end.min(self.rope.len());
        if start > end || !self.rope.is_char_boundary(start) || !self.rope.is_char_boundary(end) {
            return String::new();
        }
        self.rope.slice(start..end).to_string()
    }

    pub fn byte_len(&self) -> usize {
        self.rope.len()
    }

    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines(LineType::LF)
    }

    pub fn line(&self, index: usize) -> String {
        line_text(&self.rope, index)
    }

    pub fn line_len(&self, index: usize) -> usize {
        self.line(index).chars().count()
    }

    pub fn clamp(&self, position: Position) -> Position {
        clamp_position(&self.rope, position)
    }

    /// Return the byte offset at which a logical line starts.
    ///
    /// The offset is measured in the snapshot's canonical UTF-8 text and is
    /// therefore stable for every consumer of this revision.
    pub fn line_start_byte(&self, line: usize) -> usize {
        line_start(&self.rope, line)
    }

    /// Return the byte offset just after a line's content, excluding its line
    /// ending. CRLF is treated as one line ending and is excluded in full.
    pub fn line_end_byte(&self, line: usize) -> usize {
        line_end_without_terminator(&self.rope, line)
    }

    /// Map a Unicode-scalar position into the snapshot's UTF-8 byte space.
    pub fn position_to_byte(&self, position: Position) -> usize {
        position_to_byte(&self.rope, clamp_position(&self.rope, position))
    }

    pub fn end(&self) -> Position {
        let line = self.line_count().saturating_sub(1);
        Position::new(line, self.line_len(line))
    }

    pub fn byte_to_position(&self, byte: usize) -> Position {
        byte_to_position(&self.rope, byte)
    }
}

impl fmt::Debug for BufferSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferSnapshot")
            .field("byte_len", &self.byte_len())
            .field("revision", &self.revision)
            .finish()
    }
}

fn validate_edits(edits: &[Edit]) -> Result<(), EditorError> {
    let mut previous_end = 0;
    for (index, edit) in edits.iter().enumerate() {
        if edit.range.start > edit.range.end {
            return Err(EditorError::InvalidEditOrder);
        }
        if index > 0 && edit.range.start < previous_end {
            return Err(EditorError::OverlappingEdits);
        }
        previous_end = edit.range.end;
    }
    Ok(())
}

fn validate_range(rope: &Rope, range: TextRange) -> Result<(), EditorError> {
    if range.start > range.end || range.end > rope.len() {
        return Err(EditorError::InvalidRange(range));
    }
    if !rope.is_char_boundary(range.start) {
        return Err(EditorError::InvalidUtf8Boundary(range.start));
    }
    if !rope.is_char_boundary(range.end) {
        return Err(EditorError::InvalidUtf8Boundary(range.end));
    }
    Ok(())
}

fn shifted_offset(offset: usize, delta: isize) -> usize {
    if delta.is_negative() {
        offset.saturating_sub(delta.unsigned_abs())
    } else {
        offset.saturating_add(delta as usize)
    }
}

fn line_start(rope: &Rope, line: usize) -> usize {
    rope.line_to_byte_idx(line.min(rope.len_lines(LineType::LF)), LineType::LF)
}

fn line_end_without_terminator(rope: &Rope, line: usize) -> usize {
    let start = line_start(rope, line);
    let next = line_start(rope, line.saturating_add(1));
    let mut end = next.min(rope.len());
    if end > start && rope.byte(end.saturating_sub(1)) == b'\n' {
        end -= 1;
        if end > start && rope.byte(end.saturating_sub(1)) == b'\r' {
            end -= 1;
        }
    }
    end
}

fn line_text(rope: &Rope, line: usize) -> String {
    if line >= rope.len_lines(LineType::LF) {
        return String::new();
    }
    let start = line_start(rope, line);
    let end = line_end_without_terminator(rope, line);
    rope.slice(start..end).to_string()
}

fn position_to_byte(rope: &Rope, position: Position) -> usize {
    let position = clamp_position(rope, position);
    let line = position.line;
    let start = line_start(rope, line);
    let end = line_end_without_terminator(rope, line);
    let start_char = rope.byte_to_char_idx(start);
    let end_char = rope.byte_to_char_idx(end);
    let char_index = start_char.saturating_add(position.column).min(end_char);
    rope.char_to_byte_idx(char_index)
}

fn clamp_position(rope: &Rope, position: Position) -> Position {
    let line = position
        .line
        .min(rope.len_lines(LineType::LF).saturating_sub(1));
    let start = line_start(rope, line);
    let end = line_end_without_terminator(rope, line);
    let start_char = rope.byte_to_char_idx(start);
    let end_char = rope.byte_to_char_idx(end);
    Position::new(
        line,
        position.column.min(end_char.saturating_sub(start_char)),
    )
}

fn byte_to_position(rope: &Rope, byte: usize) -> Position {
    let byte = byte.min(rope.len());
    let line = rope.byte_to_line_idx(byte, LineType::LF);
    let start = line_start(rope, line);
    let end = line_end_without_terminator(rope, line);
    let clamped = byte.min(end);
    Position::new(
        line,
        rope.byte_to_char_idx(clamped)
            .saturating_sub(rope.byte_to_char_idx(start)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_at(buffer: &EditorBuffer, position: Position) -> SelectionSet {
        SelectionSet::single(Selection::collapsed(Anchor::new(
            buffer.position_to_byte(position),
            AnchorAffinity::After,
        )))
    }

    fn transaction(buffer: &EditorBuffer, edits: Vec<Edit>) -> Transaction {
        let selection = selection_at(buffer, Position::default());
        Transaction::new(
            buffer.revision(),
            edits,
            selection.clone(),
            selection,
            EditSource::Typing,
            None,
        )
        .expect("test transaction is valid")
    }

    #[test]
    fn empty_and_trailing_newline_have_stable_lines() {
        for source in ["", "a\n", "a\r\n", "\r\n\r\n"] {
            let buffer = EditorBuffer::from_text(source);
            assert_eq!(buffer.text(), source);
            assert_eq!(buffer.line_count(), source.matches('\n').count() + 1);
        }
    }

    #[test]
    fn crlf_line_content_excludes_only_the_terminator() {
        let buffer = EditorBuffer::from_text("one\r\ntwo\nthree");
        assert_eq!(buffer.line(0), "one");
        assert_eq!(buffer.line(1), "two");
        assert_eq!(buffer.line(2), "three");
        assert_eq!(buffer.position_to_byte(Position::new(1, 0)), 5);
    }

    #[test]
    fn unicode_position_conversion_uses_scalar_values() {
        let buffer = EditorBuffer::from_text("é•x\n終");
        let position = Position::new(0, 2);
        let byte = buffer.position_to_byte(position);
        assert_eq!(byte, "é•".len());
        assert_eq!(buffer.byte_to_position(byte), position);
    }

    #[test]
    fn snapshot_is_immutable_and_revisioned() {
        let mut buffer = EditorBuffer::from_text("abc");
        let snapshot = buffer.snapshot();
        let edit = Transaction::new(
            buffer.revision(),
            vec![Edit::insertion(
                buffer.position_to_byte(Position::new(0, 1)),
                "X",
            )],
            selection_at(&buffer, Position::new(0, 1)),
            selection_at(&buffer, Position::new(0, 2)),
            EditSource::Typing,
            None,
        )
        .expect("insertion transaction is valid");
        buffer
            .apply_transaction(&edit)
            .expect("transaction applies");
        assert_eq!(snapshot.text(), "abc");
        assert_eq!(snapshot.revision(), Revision::default());
        assert_eq!(buffer.revision(), Revision::new(1));
    }

    #[test]
    fn anchor_affinity_is_deterministic_at_insertions() {
        let mut before = Anchor::new(1, AnchorAffinity::Before);
        let mut after = Anchor::new(1, AnchorAffinity::After);
        let insertion = TextRange::new(1, 1);
        before.map_through(insertion, 2);
        after.map_through(insertion, 2);
        assert_eq!(before.offset(), 1);
        assert_eq!(after.offset(), 3);
    }

    #[test]
    fn replacement_maps_inside_anchors_to_the_selected_side() {
        let mut before = Anchor::new(2, AnchorAffinity::Before);
        let mut after = Anchor::new(2, AnchorAffinity::After);
        let edit = TextRange::new(1, 4);
        before.map_through(edit, 2);
        after.map_through(edit, 2);
        assert_eq!(before.offset(), 1);
        assert_eq!(after.offset(), 3);
    }

    #[test]
    fn transaction_uses_deltas_and_inverse_instead_of_snapshots() {
        let mut buffer = EditorBuffer::from_text("abcdef");
        let applied = buffer
            .apply_transaction(&transaction(
                &buffer,
                vec![Edit::new(TextRange::new(1, 3), "XYZ")],
            ))
            .expect("transaction applies");
        assert_eq!(buffer.text(), "aXYZdef");
        let inverse = applied
            .inverse
            .with_base_revision(buffer.revision(), EditSource::Undo);
        buffer.apply_transaction(&inverse).expect("inverse applies");
        assert_eq!(buffer.text(), "abcdef");
    }

    #[test]
    fn multiple_edits_are_applied_back_to_front() {
        let mut buffer = EditorBuffer::from_text("0123456789");
        let tx = transaction(
            &buffer,
            vec![
                Edit::new(TextRange::new(1, 3), "A"),
                Edit::new(TextRange::new(7, 9), "B"),
            ],
        );
        buffer
            .apply_transaction(&tx)
            .expect("non-overlapping edits");
        assert_eq!(buffer.text(), "0A3456B9");
    }

    #[test]
    fn large_text_snapshot_and_edit_keep_content_correct() {
        let source = "0123456789\n".repeat(20_000);
        let mut buffer = EditorBuffer::from_text(&source);
        let snapshot = buffer.snapshot();
        let offset = buffer.position_to_byte(Position::new(10_000, 3));
        let tx = Transaction::new(
            buffer.revision(),
            vec![Edit::insertion(offset, "λ")],
            selection_at(&buffer, Position::new(10_000, 3)),
            selection_at(&buffer, Position::new(10_000, 4)),
            EditSource::Typing,
            None,
        )
        .expect("large edit is valid");
        buffer.apply_transaction(&tx).expect("large edit applies");
        assert_eq!(snapshot.text(), source);
        assert_eq!(buffer.line_count(), 20_001);
        assert_eq!(buffer.line(10_000), "012λ3456789");
    }
}
