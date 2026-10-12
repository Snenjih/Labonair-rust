//! Typed editing intents and editor-owned editing behavior.
//!
//! The implementation operates on [`Document`] transactions. It never keeps
//! a second text representation, so multi-cursor edits, undo, and lifecycle
//! checks use the same canonical buffer as the existing editor adapter.

use crate::buffer::{
    Anchor, AnchorAffinity, BufferSnapshot, Edit, EditSource, Position, Selection, SelectionSet,
    TextRange,
};
use crate::document::{Document, Motion};
use crate::language::Language;
use crate::search::{find_all, SearchQuery};
use crate::splits::{EditorGroupId, EditorSplitTree, SplitError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditIntent {
    Insert(String),
    /// Replace each selection with its matching text, or broadcast one value
    /// when only one is provided. Used for multi-cursor paste.
    InsertMany(Vec<String>),
    DeleteBackward,
    DeleteForward,
    /// Remove non-empty selections without deleting at empty carets.
    DeleteSelections,
    DeleteLine,
    JoinLines,
    DeleteTo(DeleteTarget),
    KillToLineEnd,
    Rewrap {
        columns: usize,
    },
    Move {
        motion: Motion,
        extend: bool,
    },
    AddCursor(Position),
    AddCursorAbove,
    AddCursorBelow,
    SelectNextOccurrence,
    SelectPreviousOccurrence,
    SelectAllOccurrences,
    SelectLine,
    SelectAll,
    UndoSelection,
    RedoSelection,
    DuplicateSelection,
    ExpandSelection,
    ShrinkSelection,
    DuplicateLines,
    MoveLines {
        down: bool,
    },
    Transpose,
    ToggleComment,
    Indent {
        unit: String,
    },
    Outdent {
        unit: String,
    },
    AutoIndent {
        unit: String,
    },
    TypeBracket(char),
    TypeTagClose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteTarget {
    LineStart,
    LineEnd,
    PreviousWordStart,
    NextWordEnd,
    PreviousSubwordStart,
    NextSubwordEnd,
}

/// Named compatibility type for a single entry in a multi-selection.
pub type MultiSelection = Selection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitIntent {
    Right,
    Down,
    FocusNext,
    FocusPrevious,
    Close,
    CloseOthers,
    Resize { delta_milli: i16 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorViewEvent {
    EditApplied {
        source: EditSource,
        edit_count: usize,
    },
    SelectionChanged,
    SplitChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorCommandResult {
    pub changed: bool,
    pub events: Vec<EditorViewEvent>,
}

impl EditorCommandResult {
    fn unchanged() -> Self {
        Self {
            changed: false,
            events: Vec::new(),
        }
    }

    fn changed(event: EditorViewEvent) -> Self {
        Self {
            changed: true,
            events: vec![event],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommand {
    Edit(EditIntent),
    Split(SplitIntent),
}

/// The smallest typed command boundary needed by a workspace/editor view.
pub trait EditorCommandSink {
    fn execute(&mut self, command: EditorCommand) -> EditorCommandResult;
}

pub trait EditorSnapshotStream {
    fn snapshot(&self) -> EditorViewSnapshot;
}

#[derive(Clone, Debug)]
pub struct EditorViewSnapshot {
    pub buffer: BufferSnapshot,
    pub selections: SelectionSet,
    pub splits: EditorSplitTree,
}

/// Editor-owned state for a tab or standalone editor surface. Workspace can
/// host this value through the typed command/snapshot traits without owning
/// either document selections or the internal split tree.
pub struct EditorState {
    document: Document,
    splits: EditorSplitTree,
}

impl EditorState {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            splits: EditorSplitTree::new(EditorGroupId::new(1)),
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    pub fn split_tree(&self) -> &EditorSplitTree {
        &self.splits
    }

    pub fn split_tree_mut(&mut self) -> &mut EditorSplitTree {
        &mut self.splits
    }
}

impl EditorSnapshotStream for EditorState {
    fn snapshot(&self) -> EditorViewSnapshot {
        EditorViewSnapshot {
            buffer: self.document.snapshot(),
            selections: self.document.selection_set(),
            splits: self.splits.clone(),
        }
    }
}

impl EditorCommandSink for EditorState {
    fn execute(&mut self, command: EditorCommand) -> EditorCommandResult {
        match command {
            EditorCommand::Edit(intent) => self.document.execute_edit(intent),
            EditorCommand::Split(intent) => self.execute_split(intent),
        }
    }
}

impl EditorState {
    fn execute_split(&mut self, intent: SplitIntent) -> EditorCommandResult {
        let before = self.splits.clone();
        let result = match intent {
            SplitIntent::Right => self.splits.split_right().map(|_| ()),
            SplitIntent::Down => self.splits.split_down().map(|_| ()),
            SplitIntent::FocusNext => {
                self.splits.focus_next();
                Ok(())
            }
            SplitIntent::FocusPrevious => {
                self.splits.focus_previous();
                Ok(())
            }
            SplitIntent::Close => self.splits.close_focused().map(|_| ()),
            SplitIntent::CloseOthers => self.splits.close_other_groups(),
            SplitIntent::Resize { delta_milli } => self
                .splits
                .resize(self.splits.focused_group(), f32::from(delta_milli) / 1000.0),
        };
        match result {
            Ok(()) if self.splits != before => {
                EditorCommandResult::changed(EditorViewEvent::SplitChanged)
            }
            Ok(()) => EditorCommandResult::unchanged(),
            Err(SplitError::CannotCloseLastGroup | SplitError::GroupNotFound(_)) => {
                EditorCommandResult::unchanged()
            }
        }
    }
}

impl Document {
    /// Execute one semantic editing intent against the canonical buffer.
    pub fn execute_edit(&mut self, intent: EditIntent) -> EditorCommandResult {
        match intent {
            EditIntent::Insert(text) => self.multi_insert(&text),
            EditIntent::InsertMany(texts) => self.multi_insert_many(&texts),
            EditIntent::DeleteBackward => self.multi_delete(false),
            EditIntent::DeleteForward => self.multi_delete(true),
            EditIntent::DeleteSelections => self.delete_selections(),
            EditIntent::DeleteLine => self.delete_lines(),
            EditIntent::JoinLines => self.join_lines(),
            EditIntent::DeleteTo(target) => self.delete_to(target),
            EditIntent::KillToLineEnd => self.kill_to_line_end(),
            EditIntent::Rewrap { columns } => self.rewrap(columns),
            EditIntent::Move { motion, extend } => self.multi_move(motion, extend),
            EditIntent::AddCursor(position) => self.add_cursor(position),
            EditIntent::AddCursorAbove => self.add_cursors_vertical(false),
            EditIntent::AddCursorBelow => self.add_cursors_vertical(true),
            EditIntent::SelectNextOccurrence => self.select_next_occurrence(),
            EditIntent::SelectPreviousOccurrence => self.select_previous_occurrence(),
            EditIntent::SelectAllOccurrences => self.select_all_occurrences(),
            EditIntent::SelectLine => self.select_lines(),
            EditIntent::SelectAll => self.select_all_intent(),
            EditIntent::UndoSelection => self.selection_history_step(false),
            EditIntent::RedoSelection => self.selection_history_step(true),
            EditIntent::DuplicateSelection => self.duplicate_selection(),
            EditIntent::ExpandSelection => self.expand_selection(),
            EditIntent::ShrinkSelection => self.shrink_selection(),
            EditIntent::DuplicateLines => self.duplicate_lines(),
            EditIntent::MoveLines { down } => self.move_lines(down),
            EditIntent::Transpose => self.transpose(),
            EditIntent::ToggleComment => self.toggle_comment(),
            EditIntent::Indent { unit } => self.indent_lines(&unit, false),
            EditIntent::Outdent { unit } => self.indent_lines(&unit, true),
            EditIntent::AutoIndent { unit } => self.insert_auto_indent(&unit),
            EditIntent::TypeBracket(ch) => self.type_bracket(ch),
            EditIntent::TypeTagClose => self.type_tag_close(),
        }
    }

    fn multi_insert(&mut self, text: &str) -> EditorCommandResult {
        if text.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let selections = self.selection_set();
        let snapshot = self.snapshot();
        let targets = selections
            .ranges(&snapshot)
            .into_iter()
            .map(|range| (range, text.to_string()))
            .collect::<Vec<_>>();
        self.apply_targets(targets, text.chars().count() == 1, EditSource::Typing, None)
    }

    fn multi_insert_many(&mut self, texts: &[String]) -> EditorCommandResult {
        if texts.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let selections = self.selection_set();
        if texts.len() != 1 && texts.len() != selections.len() {
            return EditorCommandResult::unchanged();
        }
        let snapshot = self.snapshot();
        let targets = selections
            .ranges(&snapshot)
            .into_iter()
            .enumerate()
            .map(|(index, range)| {
                let replacement = if texts.len() == 1 {
                    texts[0].clone()
                } else {
                    texts[index].clone()
                };
                (range, replacement)
            })
            .collect();
        self.apply_targets(targets, false, EditSource::Paste, Some("Paste".into()))
    }

    fn delete_selections(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let targets = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .filter(|range| !range.is_empty())
            .map(|range| (range, String::new()))
            .collect();
        self.apply_targets(targets, false, EditSource::Delete, Some("Cut".into()))
    }

    fn delete_lines(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut lines = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .flat_map(|range| {
                let start = snapshot.byte_to_position(range.start).line;
                let end_position = snapshot.byte_to_position(range.end);
                let end =
                    if !range.is_empty() && end_position.column == 0 && end_position.line > start {
                        end_position.line - 1
                    } else {
                        end_position.line
                    };
                start..=end.max(start)
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        let Some(first_line) = lines.first().copied() else {
            return EditorCommandResult::unchanged();
        };

        let mut groups = Vec::new();
        let mut group_start = first_line;
        let mut group_end = first_line;
        for line in lines.into_iter().skip(1) {
            if line == group_end + 1 {
                group_end = line;
            } else {
                groups.push((group_start, group_end));
                group_start = line;
                group_end = line;
            }
        }
        groups.push((group_start, group_end));

        let last_document_line = snapshot.line_count().saturating_sub(1);
        let targets = groups
            .into_iter()
            .map(|(first, last)| {
                let start = if last == last_document_line && first > 0 {
                    snapshot.line_end_byte(first - 1)
                } else {
                    snapshot.line_start_byte(first)
                };
                let end = if last < last_document_line {
                    snapshot.line_start_byte(last + 1)
                } else {
                    snapshot.line_end_byte(last)
                };
                (TextRange::new(start, end), String::new())
            })
            .filter(|(range, _)| !range.is_empty())
            .collect();
        self.apply_targets(
            targets,
            false,
            EditSource::Delete,
            Some("Delete lines".into()),
        )
    }

    fn join_lines(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let last_line = snapshot.line_count().saturating_sub(1);
        let mut boundaries = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .flat_map(|range| {
                let start = snapshot.byte_to_position(range.start).line;
                let end = snapshot.byte_to_position(range.end).line;
                let end_exclusive = if range.is_empty() || start == end {
                    start.saturating_add(1)
                } else {
                    end
                };
                start..end_exclusive.min(last_line)
            })
            .collect::<Vec<_>>();
        boundaries.sort_unstable();
        boundaries.dedup();
        let targets = boundaries
            .into_iter()
            .map(|line| {
                let left = snapshot.line(line);
                let right = snapshot.line(line + 1);
                let trimmed_right = right.trim_start();
                let indentation_bytes = right.len() - trimmed_right.len();
                let replacement = if !left.is_empty() && !trimmed_right.is_empty() {
                    " "
                } else {
                    ""
                };
                let start = snapshot.line_end_byte(line);
                let end = snapshot.line_start_byte(line + 1) + indentation_bytes;
                (TextRange::new(start, end), replacement.to_string())
            })
            .collect();
        self.apply_targets(
            targets,
            false,
            EditSource::Typing,
            Some("Join lines".into()),
        )
    }

    fn delete_to(&mut self, target: DeleteTarget) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let text = snapshot.text();
        let ranges = self
            .selection_set()
            .selections()
            .iter()
            .filter_map(|selection| {
                let range = selection.range().resolve(&snapshot);
                if !range.is_empty() {
                    return Some(range);
                }
                let position = snapshot.byte_to_position(range.start);
                let boundary = match target {
                    DeleteTarget::LineStart => snapshot.line_start_byte(position.line),
                    DeleteTarget::LineEnd => snapshot.line_end_byte(position.line),
                    DeleteTarget::PreviousWordStart => {
                        word_delete_boundary(&text, range.start, false, false)
                    }
                    DeleteTarget::NextWordEnd => {
                        word_delete_boundary(&text, range.start, true, false)
                    }
                    DeleteTarget::PreviousSubwordStart => {
                        word_delete_boundary(&text, range.start, false, true)
                    }
                    DeleteTarget::NextSubwordEnd => {
                        word_delete_boundary(&text, range.start, true, true)
                    }
                };
                let range = TextRange::new(range.start.min(boundary), range.start.max(boundary));
                (!range.is_empty()).then_some(range)
            })
            .collect();
        let targets = merge_ranges(ranges)
            .into_iter()
            .map(|range| (range, String::new()))
            .collect();
        let label = match target {
            DeleteTarget::LineStart => "Delete to line start",
            DeleteTarget::LineEnd => "Delete to line end",
            DeleteTarget::PreviousWordStart => "Delete to previous word start",
            DeleteTarget::NextWordEnd => "Delete to next word end",
            DeleteTarget::PreviousSubwordStart => "Delete to previous subword start",
            DeleteTarget::NextSubwordEnd => "Delete to next subword end",
        };
        self.apply_targets(targets, false, EditSource::Delete, Some(label.into()))
    }

    /// Resolve the ranges affected by Kill Ring Cut for the current selection
    /// set. The Editor view uses these exact ranges to capture text before it
    /// dispatches [`EditIntent::KillToLineEnd`].
    pub fn kill_to_line_end_ranges(&self) -> Vec<TextRange> {
        let snapshot = self.snapshot();
        let ranges = self
            .selection_set()
            .selections()
            .iter()
            .filter_map(|selection| {
                let range = selection.range().resolve(&snapshot);
                if !range.is_empty() {
                    return Some(range);
                }
                let position = snapshot.byte_to_position(range.start);
                let mut end = snapshot.line_end_byte(position.line);
                if end == range.start && position.line + 1 < snapshot.line_count() {
                    end = snapshot.line_start_byte(position.line + 1);
                }
                (end > range.start).then_some(TextRange::new(range.start, end))
            })
            .collect();
        merge_ranges(ranges)
    }

    fn kill_to_line_end(&mut self) -> EditorCommandResult {
        let targets = self
            .kill_to_line_end_ranges()
            .into_iter()
            .map(|range| (range, String::new()))
            .collect();
        self.apply_targets(
            targets,
            false,
            EditSource::Delete,
            Some("Kill to line end".into()),
        )
    }

    fn rewrap(&mut self, columns: usize) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let line_count = snapshot.line_count();
        let mut spans = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .filter_map(|range| {
                let start_position = snapshot.byte_to_position(range.start);
                let mut first = start_position.line;
                let mut last = snapshot.byte_to_position(range.end).line;
                if !range.is_empty()
                    && last > first
                    && snapshot.byte_to_position(range.end).column == 0
                {
                    last -= 1;
                }

                if range.is_empty() {
                    if snapshot.line(first).trim().is_empty() {
                        return None;
                    }
                    while first > 0 && !snapshot.line(first - 1).trim().is_empty() {
                        first -= 1;
                    }
                    while last + 1 < line_count && !snapshot.line(last + 1).trim().is_empty() {
                        last += 1;
                    }
                } else {
                    if !snapshot.line(first).trim().is_empty() {
                        while first > 0 && !snapshot.line(first - 1).trim().is_empty() {
                            first -= 1;
                        }
                    }
                    if !snapshot.line(last).trim().is_empty() {
                        while last + 1 < line_count && !snapshot.line(last + 1).trim().is_empty() {
                            last += 1;
                        }
                    }
                }
                Some((first, last))
            })
            .collect::<Vec<_>>();
        spans.sort_unstable();
        let mut merged_spans: Vec<(usize, usize)> = Vec::new();
        for (first, last) in spans {
            if let Some((_, previous_last)) = merged_spans.last_mut() {
                if first <= *previous_last {
                    *previous_last = (*previous_last).max(last);
                    continue;
                }
            }
            merged_spans.push((first, last));
        }

        let mut targets = Vec::new();
        for (first, last) in merged_spans {
            let mut paragraph_start = None;
            for line in first..=last.saturating_add(1).min(line_count) {
                let is_end = line >= last.saturating_add(1).min(line_count);
                let is_blank = !is_end && snapshot.line(line).trim().is_empty();
                if is_blank || is_end {
                    if let Some(start_line) = paragraph_start.take() {
                        let end_line = line - 1;
                        let lines = (start_line..=end_line)
                            .map(|line| snapshot.line(line).to_string())
                            .collect::<Vec<_>>();
                        let Some(replacement) = rewrap_lines(&lines, columns) else {
                            continue;
                        };
                        let start = snapshot.line_start_byte(start_line);
                        let end = snapshot.line_end_byte(end_line);
                        let range = TextRange::new(start, end);
                        if snapshot.text_range(range) != replacement {
                            targets.push((range, replacement));
                        }
                    }
                } else if paragraph_start.is_none() {
                    paragraph_start = Some(line);
                }
            }
        }

        self.apply_targets(
            targets,
            false,
            EditSource::Typing,
            Some("Rewrap paragraphs".into()),
        )
    }

    fn multi_delete(&mut self, forward: bool) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut targets = Vec::new();
        for selection in self.selection_set().selections() {
            let range = selection.range().resolve(&snapshot);
            let range = if !range.is_empty() {
                range
            } else if forward {
                next_char_range(&snapshot, range.start)
            } else {
                previous_char_range(&snapshot, range.start)
            };
            if !range.is_empty() {
                targets.push((range, String::new()));
            }
        }
        self.apply_targets(targets, false, EditSource::Delete, None)
    }

    fn multi_move(&mut self, motion: Motion, extend: bool) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let mut moved = Vec::with_capacity(current.len());
        for selection in current.selections() {
            let head = selection.head.resolve(&snapshot);
            let next = move_position(&snapshot, head, motion);
            let anchor = if extend {
                selection.anchor.resolve(&snapshot)
            } else {
                next
            };
            moved.push(Selection {
                anchor: Anchor::new(snapshot.position_to_byte(anchor), AnchorAffinity::Before),
                head: Anchor::new(snapshot.position_to_byte(next), AnchorAffinity::After),
            });
        }
        let next = SelectionSet::from_selections(moved, current.primary_index());
        if next == current {
            return EditorCommandResult::unchanged();
        }
        self.set_selection_set(next);
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn add_cursor(&mut self, position: Position) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut selections = self.selection_set().selections().to_vec();
        let at = snapshot.position_to_byte(position);
        if selections
            .iter()
            .any(|selection| selection.anchor.offset() == at && selection.head.offset() == at)
        {
            return EditorCommandResult::unchanged();
        }
        selections.push(Selection::collapsed(Anchor::new(at, AnchorAffinity::After)));
        let next = SelectionSet::from_selections(selections, self.selection_set().primary_index());
        self.set_selection_set(next);
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn add_cursors_vertical(&mut self, below: bool) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let line_count = snapshot.line_count();
        let mut selections = current.selections().to_vec();
        for selection in current.selections() {
            let anchor = selection.anchor.resolve(&snapshot);
            let head = selection.head.resolve(&snapshot);
            let (Some(anchor_line), Some(head_line)) = (
                vertical_target_line(anchor.line, below, line_count),
                vertical_target_line(head.line, below, line_count),
            ) else {
                continue;
            };
            let anchor_column = anchor
                .column
                .min(snapshot.line(anchor_line).chars().count());
            let head_column = head.column.min(snapshot.line(head_line).chars().count());
            selections.push(Selection {
                anchor: Anchor::new(
                    snapshot.position_to_byte(Position::new(anchor_line, anchor_column)),
                    selection.anchor.affinity(),
                ),
                head: Anchor::new(
                    snapshot.position_to_byte(Position::new(head_line, head_column)),
                    selection.head.affinity(),
                ),
            });
        }
        let next = SelectionSet::from_selections(selections, current.primary_index());
        if next == current {
            return EditorCommandResult::unchanged();
        }
        self.set_selection_set(next);
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn select_all_occurrences(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let primary = self.selection_set().primary().range().resolve(&snapshot);
        let query_text = if primary.is_empty() {
            word_at(&snapshot, primary.start).unwrap_or_default()
        } else {
            snapshot.text_range(primary)
        };
        if query_text.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let matches = find_all(
            &snapshot,
            &SearchQuery {
                text: query_text,
                ..Default::default()
            },
        );
        let selections = matches
            .iter()
            .map(|matched| Selection {
                anchor: Anchor::new(
                    snapshot.position_to_byte(matched.start),
                    AnchorAffinity::Before,
                ),
                head: Anchor::new(
                    snapshot.position_to_byte(matched.end),
                    AnchorAffinity::After,
                ),
            })
            .collect::<Vec<_>>();
        if selections.is_empty() {
            return EditorCommandResult::unchanged();
        }
        self.set_selection_set(SelectionSet::from_selections(selections, 0));
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn select_all_intent(&mut self) -> EditorCommandResult {
        let before = self.selection_set();
        self.select_all();
        if self.selection_set() == before {
            EditorCommandResult::unchanged()
        } else {
            EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
        }
    }

    fn select_lines(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let line_count = snapshot.line_count();
        let selections = current
            .ranges(&snapshot)
            .into_iter()
            .map(|range| {
                let start = snapshot.byte_to_position(range.start);
                let mut last_line = snapshot.byte_to_position(range.end).line;
                let end_position = snapshot.byte_to_position(range.end);
                if !range.is_empty() && end_position.column == 0 && last_line > start.line {
                    last_line -= 1;
                }
                let selection_start = snapshot.line_start_byte(start.line);
                let selection_end = if last_line + 1 < line_count {
                    snapshot.line_start_byte(last_line + 1)
                } else {
                    snapshot.line_end_byte(last_line)
                };
                Selection {
                    anchor: Anchor::new(selection_start, AnchorAffinity::Before),
                    head: Anchor::new(selection_end, AnchorAffinity::After),
                }
            })
            .collect::<Vec<_>>();
        let next = SelectionSet::from_selections(selections, current.primary_index());
        if next == current {
            return EditorCommandResult::unchanged();
        }
        self.set_selection_set(next);
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn select_next_occurrence(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let primary = current.primary().range().resolve(&snapshot);
        let query_text = if primary.is_empty() {
            word_at(&snapshot, primary.start).unwrap_or_default()
        } else {
            snapshot.text_range(primary)
        };
        if query_text.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let matches = find_all(
            &snapshot,
            &SearchQuery {
                text: query_text,
                ..Default::default()
            },
        );
        let Some(next) = matches
            .iter()
            .copied()
            .find(|matched| matched.text_range(&snapshot).start > primary.end)
            .or_else(|| matches.first().copied())
        else {
            return EditorCommandResult::unchanged();
        };
        let next_selection = Selection {
            anchor: Anchor::new(
                snapshot.position_to_byte(next.start),
                AnchorAffinity::Before,
            ),
            head: Anchor::new(snapshot.position_to_byte(next.end), AnchorAffinity::After),
        };
        if current
            .selections()
            .iter()
            .any(|selection| selection.range().resolve(&snapshot) == next.text_range(&snapshot))
        {
            return EditorCommandResult::unchanged();
        }
        let mut selections = current.selections().to_vec();
        selections.push(next_selection);
        self.set_selection_set(SelectionSet::from_selections(
            selections,
            current.primary_index().saturating_add(1),
        ));
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn select_previous_occurrence(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let primary = current.primary().range().resolve(&snapshot);
        let query_text = if primary.is_empty() {
            word_at(&snapshot, primary.start).unwrap_or_default()
        } else {
            snapshot.text_range(primary)
        };
        if query_text.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let matches = find_all(
            &snapshot,
            &SearchQuery {
                text: query_text,
                ..Default::default()
            },
        );
        let Some(previous) = matches
            .iter()
            .copied()
            .rev()
            .find(|matched| matched.text_range(&snapshot).end <= primary.start)
            .or_else(|| matches.last().copied())
        else {
            return EditorCommandResult::unchanged();
        };
        if current
            .selections()
            .iter()
            .any(|selection| selection.range().resolve(&snapshot) == previous.text_range(&snapshot))
        {
            return EditorCommandResult::unchanged();
        }
        let mut selections = current.selections().to_vec();
        selections.push(Selection {
            anchor: Anchor::new(
                snapshot.position_to_byte(previous.start),
                AnchorAffinity::Before,
            ),
            head: Anchor::new(
                snapshot.position_to_byte(previous.end),
                AnchorAffinity::After,
            ),
        });
        self.set_selection_set(SelectionSet::from_selections(
            selections,
            current.primary_index().saturating_add(1),
        ));
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn selection_history_step(&mut self, redo: bool) -> EditorCommandResult {
        let changed = if redo {
            self.redo_selection()
        } else {
            self.undo_selection()
        };
        if !changed {
            return EditorCommandResult::unchanged();
        }
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn expand_selection(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let current = self.selection_set();
        let mut expanded = Vec::with_capacity(current.len());
        for selection in current.selections() {
            let range = selection.range().resolve(&snapshot);
            let next = if range.is_empty() {
                word_range(&snapshot, range.start)
                    .or_else(|| enclosing_bracket_range(&snapshot, range))
                    .or_else(|| line_range(&snapshot, range.start))
                    .unwrap_or(range)
            } else {
                enclosing_bracket_range(&snapshot, range)
                    .or_else(|| line_range(&snapshot, range.start))
                    .unwrap_or(range)
            };
            expanded.push(Selection {
                anchor: Anchor::new(next.start, AnchorAffinity::Before),
                head: Anchor::new(next.end, AnchorAffinity::After),
            });
        }
        let next = SelectionSet::from_selections(expanded, current.primary_index());
        if next == current {
            return EditorCommandResult::unchanged();
        }
        self.set_selection_set(next);
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn shrink_selection(&mut self) -> EditorCommandResult {
        let current = self.selection_set();
        if current
            .selections()
            .iter()
            .all(|selection| selection.anchor.offset() == selection.head.offset())
        {
            return EditorCommandResult::unchanged();
        }
        let snapshot = self.snapshot();
        let selections = current
            .selections()
            .iter()
            .map(|selection| {
                let range = selection.range().resolve(&snapshot);
                if let Some(inner) = inner_bracket_range(&snapshot, range) {
                    Selection {
                        anchor: Anchor::new(inner.start, AnchorAffinity::Before),
                        head: Anchor::new(inner.end, AnchorAffinity::After),
                    }
                } else {
                    Selection::collapsed(Anchor::new(range.start, AnchorAffinity::After))
                }
            })
            .collect();
        self.set_selection_set(SelectionSet::from_selections(
            selections,
            current.primary_index(),
        ));
        EditorCommandResult::changed(EditorViewEvent::SelectionChanged)
    }

    fn duplicate_lines(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut lines = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .map(|range| {
                let start_line = snapshot.byte_to_position(range.start).line;
                let end_line = snapshot.byte_to_position(range.end).line;
                (start_line, end_line)
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        let mut targets = Vec::new();
        for (_, end_line) in lines {
            let end = snapshot.line_end_byte(end_line);
            let line = snapshot.line(end_line);
            targets.push((TextRange::new(end, end), format!("\n{line}")));
        }
        self.apply_targets(
            targets,
            false,
            EditSource::Typing,
            Some("Duplicate lines".into()),
        )
    }

    fn duplicate_selection(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let ranges = self.selection_set().ranges(&snapshot);
        let selected = ranges
            .into_iter()
            .filter(|range| !range.is_empty())
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return self.duplicate_lines();
        }
        let edits = selected
            .iter()
            .map(|range| {
                Edit::new(
                    TextRange::new(range.end, range.end),
                    snapshot.text_range(*range),
                )
            })
            .collect::<Vec<_>>();
        let after = selected
            .iter()
            .map(|range| {
                Selection::collapsed(Anchor::new(
                    map_offset(range.end, &edits, AnchorAffinity::After),
                    AnchorAffinity::After,
                ))
            })
            .collect::<Vec<_>>();
        if !self.apply_edits(
            edits.clone(),
            SelectionSet::from_selections(after, 0),
            EditSource::Paste,
            Some("Duplicate selection".into()),
        ) {
            return EditorCommandResult::unchanged();
        }
        EditorCommandResult {
            changed: true,
            events: vec![EditorViewEvent::EditApplied {
                source: EditSource::Paste,
                edit_count: edits.len(),
            }],
        }
    }

    fn move_lines(&mut self, down: bool) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let range = self.selection_set().primary().range().resolve(&snapshot);
        let first = snapshot.byte_to_position(range.start).line;
        let last = snapshot.byte_to_position(range.end).line;
        let line_count = snapshot.line_count();
        let (region, replacement) = if down {
            if last + 1 >= line_count {
                return EditorCommandResult::unchanged();
            }
            let start = snapshot.line_start_byte(first);
            let next_end = snapshot.line_end_byte(last + 1);
            let block = snapshot.text_range(TextRange::new(start, snapshot.line_end_byte(last)));
            let next =
                snapshot.text_range(TextRange::new(snapshot.line_start_byte(last + 1), next_end));
            (TextRange::new(start, next_end), format!("{next}\n{block}"))
        } else {
            if first == 0 {
                return EditorCommandResult::unchanged();
            }
            let start = snapshot.line_start_byte(first - 1);
            let end = snapshot.line_end_byte(last);
            let previous = snapshot.line(first - 1);
            let block = snapshot.text_range(TextRange::new(snapshot.line_start_byte(first), end));
            (TextRange::new(start, end), format!("{block}\n{previous}"))
        };
        self.apply_targets(
            vec![(region, replacement)],
            false,
            EditSource::Typing,
            Some("Move lines".into()),
        )
    }

    fn insert_auto_indent(&mut self, unit: &str) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let text = snapshot.text();
        let targets = self
            .selection_set()
            .selections()
            .iter()
            .map(|selection| {
                let range = selection.range().resolve(&snapshot);
                if self.language == Language::Markdown && range.is_empty() {
                    if let Some(target) = markdown_list_continuation(&snapshot, range.start) {
                        return target;
                    }
                }
                let indentation = auto_indent(&text, range.start, unit);
                (range, format!("\n{indentation}"))
            })
            .collect();
        self.apply_targets(
            targets,
            true,
            EditSource::Typing,
            Some("Enter with auto indent".into()),
        )
    }

    fn transpose(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut targets = Vec::new();
        for selection in self.selection_set().selections() {
            let range = selection.range().resolve(&snapshot);
            if !range.is_empty() || range.start == 0 {
                continue;
            }
            let previous = previous_char_range(&snapshot, range.start);
            let next = next_char_range(&snapshot, range.start);
            if next.is_empty() {
                continue;
            }
            let a = snapshot.text_range(previous);
            let b = snapshot.text_range(next);
            targets.push((TextRange::new(previous.start, next.end), format!("{b}{a}")));
        }
        self.apply_targets(targets, false, EditSource::Typing, Some("Transpose".into()))
    }

    fn toggle_comment(&mut self) -> EditorCommandResult {
        let snapshot = self.snapshot();
        let mut lines = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .flat_map(|range| {
                let a = snapshot.byte_to_position(range.start).line;
                let b = snapshot.byte_to_position(range.end).line;
                a..=b
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        let uncomment = lines.iter().all(|line| {
            let text = snapshot.line(*line);
            text.trim_start().starts_with("//")
        });
        let targets = lines
            .into_iter()
            .filter_map(|line| {
                let text = snapshot.line(line);
                let start = snapshot.line_start_byte(line);
                if uncomment {
                    let whitespace = text.len() - text.trim_start().len();
                    let rest = text[whitespace..].strip_prefix("//")?;
                    let remove = if rest.starts_with(' ') { 3 } else { 2 };
                    Some((
                        TextRange::new(start + whitespace, start + whitespace + remove),
                        String::new(),
                    ))
                } else {
                    let whitespace = text.len() - text.trim_start().len();
                    Some((
                        TextRange::new(start + whitespace, start + whitespace),
                        "// ".into(),
                    ))
                }
            })
            .collect();
        self.apply_targets(
            targets,
            false,
            EditSource::Typing,
            Some("Toggle comment".into()),
        )
    }

    fn indent_lines(&mut self, unit: &str, outdent: bool) -> EditorCommandResult {
        if unit.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let snapshot = self.snapshot();
        let mut lines = self
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .flat_map(|range| {
                let a = snapshot.byte_to_position(range.start).line;
                let b = snapshot.byte_to_position(range.end).line;
                a..=b
            })
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        let targets = lines
            .into_iter()
            .filter_map(|line| {
                let text = snapshot.line(line);
                let start = snapshot.line_start_byte(line);
                if outdent {
                    let count = if text.starts_with(unit) {
                        unit.len()
                    } else if text.starts_with('\t') || text.starts_with(' ') {
                        text.chars()
                            .take_while(|c| *c == ' ' || *c == '\t')
                            .count()
                            .min(unit.chars().count())
                    } else {
                        0
                    };
                    (count > 0).then(|| (TextRange::new(start, start + count), String::new()))
                } else {
                    Some((TextRange::new(start, start), unit.to_string()))
                }
            })
            .collect();
        self.apply_targets(targets, false, EditSource::Typing, Some("Indent".into()))
    }

    fn type_bracket(&mut self, ch: char) -> EditorCommandResult {
        if closing_bracket(ch).is_some() {
            let snapshot = self.snapshot();
            let selections = self.selection_set();
            if selections.selections().iter().all(|selection| {
                let range = selection.range().resolve(&snapshot);
                range.is_empty()
                    && snapshot.text_range(next_char_range(&snapshot, range.start))
                        == ch.to_string()
            }) {
                let moved = selections
                    .selections()
                    .iter()
                    .map(|selection| {
                        let at = selection.head.resolve(&snapshot);
                        let next = next_char_position(&snapshot, at);
                        Selection::collapsed(Anchor::new(
                            snapshot.position_to_byte(next),
                            AnchorAffinity::After,
                        ))
                    })
                    .collect();
                self.set_selection_set(SelectionSet::from_selections(
                    moved,
                    selections.primary_index(),
                ));
                return EditorCommandResult::changed(EditorViewEvent::SelectionChanged);
            }
        }
        let Some(close) = matching_pair(ch) else {
            return self.multi_insert(&ch.to_string());
        };
        let snapshot = self.snapshot();
        let selections = self.selection_set();
        let mut targets = Vec::new();
        let mut inner_ranges = Vec::new();
        for selection in selections.selections() {
            let range = selection.range().resolve(&snapshot);
            let selected = snapshot.text_range(range);
            inner_ranges.push((range, selected.len()));
            targets.push((range, format!("{ch}{selected}{close}")));
        }
        let edits = targets
            .iter()
            .map(|(range, replacement)| Edit::new(*range, replacement.clone()))
            .collect::<Vec<_>>();
        let after_selections = inner_ranges
            .into_iter()
            .map(|(range, inner_len)| {
                let start = map_offset(range.start, &edits, AnchorAffinity::Before) + ch.len_utf8();
                if inner_len == 0 {
                    Selection::collapsed(Anchor::new(start, AnchorAffinity::After))
                } else {
                    Selection {
                        anchor: Anchor::new(start, AnchorAffinity::Before),
                        head: Anchor::new(start + inner_len, AnchorAffinity::After),
                    }
                }
            })
            .collect::<Vec<_>>();
        let result = self.apply_targets(
            targets,
            false,
            EditSource::Typing,
            Some("Auto close bracket".into()),
        );
        if result.changed {
            self.set_selection_set(SelectionSet::from_selections(after_selections, 0));
        }
        result
    }

    fn type_tag_close(&mut self) -> EditorCommandResult {
        if !matches!(self.language, Language::Html | Language::Xml) {
            return self.multi_insert(">");
        }
        let snapshot = self.snapshot();
        let selections = self.selection_set();
        let mut targets = Vec::with_capacity(selections.len());
        for selection in selections.selections() {
            let range = selection.range().resolve(&snapshot);
            let closing_tag = range
                .is_empty()
                .then(|| opening_tag_name_before(&snapshot, range.start, self.language))
                .flatten();
            let replacement = closing_tag
                .map(|name| format!("></{name}>"))
                .unwrap_or_else(|| ">".to_string());
            targets.push((range, replacement));
        }
        targets.sort_by_key(|(range, _)| (range.start, range.end));
        targets.dedup_by(|left, right| left.0 == right.0);
        let edits = targets
            .iter()
            .map(|(range, replacement)| Edit::new(*range, replacement.clone()))
            .collect::<Vec<_>>();
        if edits.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let after = targets
            .iter()
            .map(|(range, _)| {
                Selection::collapsed(Anchor::new(
                    map_offset(range.start, &edits, AnchorAffinity::Before).saturating_add(1),
                    AnchorAffinity::After,
                ))
            })
            .collect::<Vec<_>>();
        let edit_count = edits.len();
        if !self.apply_edits(
            edits,
            SelectionSet::from_selections(after, selections.primary_index()),
            EditSource::Typing,
            Some("Auto close tag".into()),
        ) {
            return EditorCommandResult::unchanged();
        }
        EditorCommandResult {
            changed: true,
            events: vec![EditorViewEvent::EditApplied {
                source: EditSource::Typing,
                edit_count,
            }],
        }
    }

    fn apply_targets(
        &mut self,
        mut targets: Vec<(TextRange, String)>,
        typing: bool,
        source: EditSource,
        label: Option<String>,
    ) -> EditorCommandResult {
        targets.sort_by_key(|(range, _)| (range.start, range.end));
        targets.dedup_by(|a, b| a.0 == b.0);
        let edits = targets
            .iter()
            .map(|(range, replacement)| Edit::new(*range, replacement.clone()))
            .collect::<Vec<_>>();
        if edits.is_empty() {
            return EditorCommandResult::unchanged();
        }
        let before = self.selection_set();
        let after = mapped_selections(&before, &edits);
        let source = if typing { EditSource::Typing } else { source };
        if !self.apply_edits(edits.clone(), after, source, label) {
            return EditorCommandResult::unchanged();
        }
        EditorCommandResult {
            changed: true,
            events: vec![EditorViewEvent::EditApplied {
                source,
                edit_count: edits.len(),
            }],
        }
    }
}

fn word_delete_boundary(text: &str, offset: usize, forward: bool, subword: bool) -> usize {
    let characters = text.char_indices().collect::<Vec<_>>();
    let mut index = characters.partition_point(|(start, _)| *start < offset);
    if forward {
        while index < characters.len() && !is_word_character(characters[index].1) {
            index += 1;
        }
        if index == characters.len() {
            return text.len();
        }
        index += 1;
        while index < characters.len()
            && is_word_character(characters[index].1)
            && (!subword || !is_subword_boundary(&characters, index))
        {
            index += 1;
        }
        characters
            .get(index)
            .map_or(text.len(), |(start, _)| *start)
    } else {
        while index > 0 && !is_word_character(characters[index - 1].1) {
            index -= 1;
        }
        while index > 0
            && is_word_character(characters[index - 1].1)
            && (!subword || !is_subword_boundary(&characters, index))
        {
            index -= 1;
        }
        characters.get(index).map_or(0, |(start, _)| *start)
    }
}

fn merge_ranges(mut ranges: Vec<TextRange>) -> Vec<TextRange> {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<TextRange> = Vec::with_capacity(ranges.len());
    for range in ranges.into_iter().filter(|range| !range.is_empty()) {
        if let Some(previous) = merged.last_mut() {
            if range.start <= previous.end {
                previous.end = previous.end.max(range.end);
                continue;
            }
        }
        merged.push(range);
    }
    merged
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn is_subword_boundary(characters: &[(usize, char)], index: usize) -> bool {
    let Some((_, right)) = characters.get(index) else {
        return false;
    };
    let Some((_, left)) = index.checked_sub(1).and_then(|index| characters.get(index)) else {
        return false;
    };
    if !is_word_character(*left) || !is_word_character(*right) || *left == '_' || *right == '_' {
        return true;
    }
    if left.is_numeric() != right.is_numeric() || (left.is_lowercase() && right.is_uppercase()) {
        return true;
    }
    left.is_uppercase()
        && right.is_uppercase()
        && characters
            .get(index + 1)
            .is_some_and(|(_, next)| next.is_lowercase())
}

fn rewrap_lines(lines: &[String], columns: usize) -> Option<String> {
    let first = lines.first()?;
    let indentation_len = first.len() - first.trim_start().len();
    let indentation = &first[..indentation_len];
    let first_content = &first[indentation_len..];
    let comment_marker = ["///", "//", "#", "*", "--", ";", "%"]
        .into_iter()
        .find(|marker| first_content.starts_with(marker));
    let repeat_comment_marker = comment_marker.is_some_and(|marker| {
        lines.iter().all(|line| {
            let content = line.trim_start();
            content.starts_with(marker)
        })
    });
    let marker = repeat_comment_marker.then_some(comment_marker).flatten();
    let marker_spacing = marker
        .filter(|marker| {
            first_content[marker.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        })
        .map(|_| " ")
        .unwrap_or("");
    let prefix = match marker {
        Some(marker) => format!("{indentation}{marker}{marker_spacing}"),
        None => indentation.to_string(),
    };
    let words = lines
        .iter()
        .flat_map(|line| {
            let mut content = line.trim_start();
            if let Some(marker) = marker {
                content = content.strip_prefix(marker).unwrap_or(content).trim_start();
            }
            content.split_whitespace()
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        return None;
    }

    let prefix_columns = prefix.chars().count();
    let width = columns.max(prefix_columns.saturating_add(1)).max(1);
    let mut output = Vec::new();
    let mut current = prefix.clone();
    let mut current_columns = prefix_columns;
    for word in words {
        let word_columns = word.chars().count();
        let separator_columns = usize::from(current_columns > prefix_columns);
        if current_columns + separator_columns + word_columns > width
            && current_columns > prefix_columns
        {
            output.push(current);
            current = prefix.clone();
            current_columns = prefix_columns;
        }
        if current_columns > prefix_columns {
            current.push(' ');
            current_columns += 1;
        }
        current.push_str(word);
        current_columns += word_columns;
    }
    output.push(current);
    Some(output.join("\n"))
}

fn mapped_selections(before: &SelectionSet, edits: &[Edit]) -> SelectionSet {
    let selections = before
        .selections()
        .iter()
        .map(|selection| {
            let empty = selection.anchor.offset() == selection.head.offset();
            let affinity = if empty {
                AnchorAffinity::After
            } else {
                AnchorAffinity::Before
            };
            let anchor = map_offset(selection.anchor.offset(), edits, affinity);
            let head = map_offset(selection.head.offset(), edits, AnchorAffinity::After);
            Selection {
                anchor: Anchor::new(anchor, AnchorAffinity::Before),
                head: Anchor::new(head, AnchorAffinity::After),
            }
        })
        .collect::<Vec<_>>();
    SelectionSet::from_selections(selections, before.primary_index())
}

fn vertical_target_line(line: usize, below: bool, line_count: usize) -> Option<usize> {
    let target = if below {
        line.checked_add(1)?
    } else {
        line.checked_sub(1)?
    };
    (target < line_count).then_some(target)
}

fn map_offset(mut offset: usize, edits: &[Edit], affinity: AnchorAffinity) -> usize {
    let mut delta = 0isize;
    for edit in edits {
        let start = if delta < 0 {
            edit.range.start.saturating_sub(delta.unsigned_abs())
        } else {
            edit.range.start.saturating_add(delta as usize)
        };
        let end = start.saturating_add(edit.range.len());
        let mut anchor = Anchor::new(offset, affinity);
        anchor.map_through(TextRange::new(start, end), edit.replacement.len());
        offset = anchor.offset();
        delta = delta.saturating_add(edit.replacement.len() as isize - edit.range.len() as isize);
    }
    offset
}

fn previous_char_range(snapshot: &BufferSnapshot, at: usize) -> TextRange {
    if at == 0 {
        return TextRange::new(at, at);
    }
    let text = snapshot.text();
    let start = text[..at]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index);
    TextRange::new(start, at)
}

fn next_char_range(snapshot: &BufferSnapshot, at: usize) -> TextRange {
    let text = snapshot.text();
    let Some(ch) = text[at..].chars().next() else {
        return TextRange::new(at, at);
    };
    TextRange::new(at, at + ch.len_utf8())
}

fn move_position(snapshot: &BufferSnapshot, position: Position, motion: Motion) -> Position {
    let line_len = snapshot.line_len(position.line);
    match motion {
        Motion::Left => previous_char_position(snapshot, position),
        Motion::Right => next_char_position(snapshot, position),
        Motion::Up => Position::new(position.line.saturating_sub(1), position.column),
        Motion::Down => Position::new(
            (position.line + 1).min(snapshot.line_count().saturating_sub(1)),
            position.column,
        ),
        Motion::LineStart => Position::new(position.line, 0),
        Motion::LineEnd => Position::new(position.line, line_len),
        Motion::DocStart => Position::default(),
        Motion::DocEnd => snapshot.end(),
        Motion::WordLeft => word_position(snapshot, position, false),
        Motion::WordRight => word_position(snapshot, position, true),
        Motion::PageUp(lines) => {
            Position::new(position.line.saturating_sub(lines), position.column)
        }
        Motion::PageDown(lines) => Position::new(
            (position.line + lines).min(snapshot.line_count().saturating_sub(1)),
            position.column,
        ),
    }
}

fn previous_char_position(snapshot: &BufferSnapshot, position: Position) -> Position {
    if position.column > 0 {
        Position::new(position.line, position.column - 1)
    } else if position.line > 0 {
        Position::new(position.line - 1, snapshot.line_len(position.line - 1))
    } else {
        position
    }
}

fn next_char_position(snapshot: &BufferSnapshot, position: Position) -> Position {
    if position.column < snapshot.line_len(position.line) {
        Position::new(position.line, position.column + 1)
    } else if position.line + 1 < snapshot.line_count() {
        Position::new(position.line + 1, 0)
    } else {
        position
    }
}

fn word_position(snapshot: &BufferSnapshot, position: Position, forward: bool) -> Position {
    let chars = snapshot.line(position.line).chars().collect::<Vec<_>>();
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    if forward {
        let mut column = position.column;
        while column < chars.len() && !is_word(chars[column]) {
            column += 1;
        }
        while column < chars.len() && is_word(chars[column]) {
            column += 1;
        }
        Position::new(position.line, column)
    } else {
        let mut column = position.column;
        while column > 0 && !is_word(chars[column - 1]) {
            column -= 1;
        }
        while column > 0 && is_word(chars[column - 1]) {
            column -= 1;
        }
        Position::new(position.line, column)
    }
}

fn word_at(snapshot: &BufferSnapshot, at: usize) -> Option<String> {
    let text = snapshot.text();
    let position = snapshot.byte_to_position(at);
    let chars = snapshot.line(position.line).chars().collect::<Vec<_>>();
    if position.column >= chars.len()
        || !(chars[position.column].is_alphanumeric() || chars[position.column] == '_')
    {
        return None;
    }
    let mut start = position.column;
    let mut end = position.column + 1;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    let line_start = snapshot.line_start_byte(position.line);
    Some(
        text[line_start..]
            .chars()
            .skip(start)
            .take(end - start)
            .collect(),
    )
}

fn word_range(snapshot: &BufferSnapshot, at: usize) -> Option<TextRange> {
    let position = snapshot.byte_to_position(at);
    let line = snapshot.line(position.line);
    let chars = line.chars().collect::<Vec<_>>();
    if position.column >= chars.len()
        || !(chars[position.column].is_alphanumeric() || chars[position.column] == '_')
    {
        return None;
    }
    let mut start = position.column;
    let mut end = position.column + 1;
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    let base = snapshot.line_start_byte(position.line);
    let start_byte = base
        + line
            .char_indices()
            .nth(start)
            .map_or(line.len(), |(b, _)| b);
    let end_byte = base + line.char_indices().nth(end).map_or(line.len(), |(b, _)| b);
    Some(TextRange::new(start_byte, end_byte))
}

fn line_range(snapshot: &BufferSnapshot, at: usize) -> Option<TextRange> {
    let line = snapshot.byte_to_position(at).line;
    (line < snapshot.line_count()).then(|| {
        let start = snapshot.line_start_byte(line);
        let end = snapshot.line_end_byte(line);
        TextRange::new(start, end)
    })
}

fn enclosing_bracket_range(snapshot: &BufferSnapshot, range: TextRange) -> Option<TextRange> {
    let text = snapshot.text();
    let mut best = None;
    for (offset, character) in text[..range.start].char_indices() {
        if !matches!(character, '(' | '[' | '{') {
            continue;
        }
        let Some(close) = matching_bracket(&text, offset) else {
            continue;
        };
        let candidate = TextRange::new(offset, close + 1);
        if candidate.start <= range.start
            && candidate.end >= range.end
            && candidate.len() > range.len()
            && best.is_none_or(|current: TextRange| candidate.len() < current.len())
        {
            best = Some(candidate);
        }
    }
    best
}

fn inner_bracket_range(snapshot: &BufferSnapshot, range: TextRange) -> Option<TextRange> {
    if range.len() < 2 {
        return None;
    }
    let text = snapshot.text();
    let open = *text.as_bytes().get(range.start)? as char;
    if !matches!(open, '(' | '[' | '{')
        || matching_bracket(&text, range.start) != Some(range.end - 1)
    {
        return None;
    }
    Some(TextRange::new(range.start + 1, range.end - 1))
}

pub fn matching_bracket(text: &str, offset: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let bracket = *bytes.get(offset)? as char;
    let (close, direction) = match bracket {
        '(' => (')', 1isize),
        '[' => (']', 1),
        '{' => ('}', 1),
        ')' => ('(', -1),
        ']' => ('[', -1),
        '}' => ('{', -1),
        _ => return None,
    };
    let mut depth = 0usize;
    let mut index = offset as isize + direction;
    while index >= 0 && (index as usize) < bytes.len() {
        let current = bytes[index as usize] as char;
        if current == bracket {
            depth += 1;
        }
        if current == close {
            if depth == 0 {
                return Some(index as usize);
            }
            depth -= 1;
        }
        index += direction;
    }
    None
}

pub fn auto_indent(text: &str, offset: usize, unit: &str) -> String {
    let end = offset.min(text.len());
    let end = (0..=end)
        .rev()
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(0);
    let before = &text[..end];
    let line = before.rsplit('\n').next().unwrap_or_default();
    let leading = line
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .collect::<String>();
    let trimmed = line.trim_end();
    if trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(') {
        format!("{leading}{unit}")
    } else {
        leading
    }
}

fn markdown_list_continuation(
    snapshot: &BufferSnapshot,
    caret: usize,
) -> Option<(TextRange, String)> {
    let position = snapshot.byte_to_position(caret);
    if caret != snapshot.line_end_byte(position.line) {
        return None;
    }
    let line = snapshot.line(position.line);
    let indentation_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let indentation = &line[..indentation_len];
    let content = &line[indentation_len..];
    let (marker, item_text) = markdown_list_marker(content)?;
    if item_text.trim().is_empty() {
        let marker_start = snapshot.line_start_byte(position.line) + indentation_len;
        return Some((
            TextRange::new(marker_start, caret),
            format!("\n{indentation}"),
        ));
    }
    Some((
        TextRange::new(caret, caret),
        format!("\n{indentation}{marker}"),
    ))
}

fn markdown_list_marker(content: &str) -> Option<(String, &str)> {
    for marker in ["- ", "* ", "+ ", "> "] {
        if let Some(item_text) = content.strip_prefix(marker) {
            return Some((marker.to_string(), item_text));
        }
    }
    if matches!(content, "-" | "*" | "+" | ">") {
        return Some((format!("{content} "), ""));
    }

    let digits_end = content
        .char_indices()
        .find_map(|(index, character)| (!character.is_ascii_digit()).then_some(index))
        .unwrap_or(content.len());
    if digits_end == 0 {
        return None;
    }
    let digits = &content[..digits_end];
    let suffix = &content[digits_end..];
    for ending in [". ", ") "] {
        if let Some(item_text) = suffix.strip_prefix(ending) {
            let number = digits.parse::<u64>().ok()?;
            return Some((format!("{}{}", number.saturating_add(1), ending), item_text));
        }
    }
    for ending in ['.', ')'] {
        if suffix == ending.to_string() {
            let number = digits.parse::<u64>().ok()?;
            return Some((format!("{}{} ", number.saturating_add(1), ending), ""));
        }
    }
    None
}

fn matching_pair(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '\'' => Some('\''),
        '"' => Some('"'),
        _ => None,
    }
}

fn closing_bracket(ch: char) -> Option<char> {
    matches!(ch, ')' | ']' | '}' | '\'' | '"').then_some(ch)
}

fn opening_tag_name_before(
    snapshot: &BufferSnapshot,
    offset: usize,
    language: Language,
) -> Option<String> {
    let text = snapshot.text();
    let offset = offset.min(text.len());
    let prefix = &text[..offset];
    let open = prefix.rfind('<')?;
    let tag = &prefix[open + 1..];
    if tag.contains('>')
        || tag
            .trim_start()
            .chars()
            .next()
            .is_some_and(|character| matches!(character, '/' | '!' | '?'))
    {
        return None;
    }
    let name_end = tag
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_alphanumeric() && !matches!(character, '-' | '_' | ':' | '.'))
                .then_some(index)
        })
        .unwrap_or(tag.len());
    let name = &tag[..name_end];
    if name.is_empty() || !name.as_bytes()[0].is_ascii_alphabetic() {
        return None;
    }
    let attributes = &tag[name_end..];
    if attributes.trim_end().ends_with('/') || has_unclosed_attribute_quote(attributes) {
        return None;
    }
    if language == Language::Html && is_void_html_tag(name) {
        return None;
    }
    Some(name.to_string())
}

fn has_unclosed_attribute_quote(attributes: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for character in attributes.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
        }
    }
    quote.is_some()
}

fn is_void_html_tag(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// A small typed snippet session. Placeholders are `${1:default}` or `${1}`;
/// the editor can advance through ranges without embedding snippet state in
/// the buffer or history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetSession {
    pub text: String,
    pub placeholders: Vec<TextRange>,
    pub active: usize,
}

impl SnippetSession {
    pub fn parse(template: &str) -> Self {
        let mut text = String::new();
        let mut placeholders = Vec::new();
        let mut rest = template;
        while let Some(start) = rest.find("${") {
            text.push_str(&rest[..start]);
            let Some(end) = rest[start + 2..].find('}') else {
                text.push_str(&rest[start..]);
                break;
            };
            let body = &rest[start + 2..start + 2 + end];
            let default = body.split_once(':').map_or("", |(_, value)| value);
            let offset = text.len();
            text.push_str(default);
            placeholders.push(TextRange::new(offset, text.len()));
            rest = &rest[start + 3 + end..];
        }
        text.push_str(rest);
        Self {
            text,
            placeholders,
            active: 0,
        }
    }

    pub fn active_placeholder(&self) -> Option<TextRange> {
        self.placeholders.get(self.active).copied()
    }

    pub fn advance(&mut self) -> Option<TextRange> {
        if self.active + 1 < self.placeholders.len() {
            self.active += 1;
        }
        self.active_placeholder()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> Document {
        Document::from_file("test.rs".into(), text, 0)
    }

    #[test]
    fn multi_cursor_insert_delete_and_undo_are_transactional() {
        let mut document = doc("a a");
        document.set_caret(Position::new(0, 1), false);
        document.execute_edit(EditIntent::AddCursor(Position::new(0, 3)));
        document.execute_edit(EditIntent::Insert("x".into()));
        assert_eq!(document.text(), "ax ax");
        document.execute_edit(EditIntent::DeleteBackward);
        assert_eq!(document.text(), "a a");
        document.undo();
        assert_eq!(document.text(), "ax ax");
    }

    #[test]
    fn add_cursor_above_and_below_preserves_columns_and_clamps_short_lines() {
        let mut document = doc("abc\nxy\n1234");
        document.set_caret(Position::new(1, 2), false);
        document.execute_edit(EditIntent::AddCursorAbove);
        assert_eq!(document.selection_set().len(), 2);
        assert!(document
            .selection_set()
            .ranges(&document.snapshot())
            .iter()
            .any(|range| range.start == 2));

        document.execute_edit(EditIntent::AddCursorBelow);
        let snapshot = document.snapshot();
        let positions = document
            .selection_set()
            .selections()
            .iter()
            .map(|selection| selection.head.resolve(&snapshot))
            .collect::<Vec<_>>();
        assert!(positions.contains(&Position::new(2, 2)));
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn previous_occurrence_wraps_and_selection_undo_redo_are_independent() {
        let mut document = doc("foo foo foo");
        document.set_caret(Position::new(0, 0), false);
        document.execute_edit(EditIntent::SelectPreviousOccurrence);
        let snapshot = document.snapshot();
        assert!(document
            .selection_set()
            .ranges(&snapshot)
            .contains(&TextRange::new(8, 11)));

        document.execute_edit(EditIntent::UndoSelection);
        assert_eq!(document.selection_set().len(), 1);
        assert_eq!(document.text(), "foo foo foo");
        document.execute_edit(EditIntent::RedoSelection);
        assert_eq!(document.selection_set().len(), 2);

        document.execute_edit(EditIntent::Insert("x".into()));
        assert_eq!(document.text(), "xfoo foo x");
        let selection_after_edit = document.selection_set();
        assert!(!document.execute_edit(EditIntent::UndoSelection).changed);
        assert_eq!(document.selection_set(), selection_after_edit);
    }

    #[test]
    fn select_all_and_duplicate_selection_are_transactional() {
        let mut document = doc("abcd");
        document.set_caret(Position::new(0, 1), false);
        document.set_caret(Position::new(0, 3), true);
        document.execute_edit(EditIntent::DuplicateSelection);
        assert_eq!(document.text(), "abcbcd");
        assert_eq!(document.cursor, Position::new(0, 5));
        document.undo();
        assert_eq!(document.text(), "abcd");

        document.execute_edit(EditIntent::SelectAll);
        assert_eq!(document.selected_text().as_deref(), Some("abcd"));
    }

    #[test]
    fn select_line_includes_line_endings_and_supports_multiple_cursors() {
        let mut document = doc("one\ntwo\nlast");
        document.set_caret(Position::new(0, 1), false);
        document.execute_edit(EditIntent::AddCursor(Position::new(2, 2)));

        let result = document.execute_edit(EditIntent::SelectLine);
        assert!(result.changed);
        let snapshot = document.snapshot();
        let selected = document
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .map(|range| snapshot.text_range(range))
            .collect::<Vec<_>>();
        assert_eq!(selected, vec!["one\n", "last"]);
        assert_eq!(document.selection_set().len(), 2);

        let mut final_line = doc("one\ntwo");
        final_line.set_caret(Position::new(1, 2), false);
        final_line.execute_edit(EditIntent::SelectLine);
        assert_eq!(final_line.selected_text().as_deref(), Some("two"));
    }

    #[test]
    fn occurrences_comments_indent_and_brackets_work_on_unicode_text() {
        let mut document = doc("λ + λ\nvalue");
        document.set_caret(Position::new(0, 0), false);
        document.execute_edit(EditIntent::SelectAllOccurrences);
        assert_eq!(document.selection_set().len(), 2);
        document.execute_edit(EditIntent::ToggleComment);
        assert_eq!(document.text(), "// λ + λ\nvalue");
        assert_eq!(matching_bracket("(λ)", 0), Some(3));
        assert_eq!(auto_indent("if (x) {", 8, "  "), "  ");
    }

    #[test]
    fn expansion_transpose_and_snippets_are_typed_operations() {
        let mut document = doc("hello");
        document.set_caret(Position::new(0, 2), false);
        document.execute_edit(EditIntent::ExpandSelection);
        assert_eq!(document.selected_text().as_deref(), Some("hello"));
        document.set_caret(Position::new(0, 4), false);
        document.execute_edit(EditIntent::Transpose);
        assert_eq!(document.text(), "helol");
        let session = SnippetSession::parse("fn ${1:name}(${2})");
        assert_eq!(session.text, "fn name()");
        assert_eq!(session.placeholders.len(), 2);
    }

    #[test]
    fn selection_expands_through_enclosing_brackets_and_shrinks_to_inner_text() {
        let mut document = doc("call(alpha)");
        document.set_caret(Position::new(0, 7), false);

        document.execute_edit(EditIntent::ExpandSelection);
        assert_eq!(document.selected_text().as_deref(), Some("alpha"));
        document.execute_edit(EditIntent::ExpandSelection);
        assert_eq!(document.selected_text().as_deref(), Some("(alpha)"));
        document.execute_edit(EditIntent::ShrinkSelection);
        assert_eq!(document.selected_text().as_deref(), Some("alpha"));
        document.execute_edit(EditIntent::ShrinkSelection);
        assert!(document.selected_text().is_none());
        assert_eq!(document.cursor, Position::new(0, 5));
    }

    #[test]
    fn line_movement_and_bracket_skip_over_keep_the_caret_typed() {
        let mut document = doc("one\ntwo\nthree");
        document.set_caret(Position::new(1, 0), false);
        document.execute_edit(EditIntent::MoveLines { down: true });
        assert_eq!(document.text(), "one\nthree\ntwo");

        let mut document = doc("");
        document.execute_edit(EditIntent::TypeBracket('('));
        assert_eq!(document.text(), "()");
        assert_eq!(document.cursor, Position::new(0, 1));
        document.execute_edit(EditIntent::TypeBracket(')'));
        assert_eq!(document.cursor, Position::new(0, 2));
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn typing_an_open_bracket_wraps_and_preserves_selected_text() {
        let mut document = doc("one two");
        let snapshot = document.snapshot();
        document.set_selection_set(SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: Anchor::new(0, AnchorAffinity::Before),
                    head: Anchor::new(3, AnchorAffinity::After),
                },
                Selection {
                    anchor: Anchor::new(4, AnchorAffinity::Before),
                    head: Anchor::new(snapshot.byte_len(), AnchorAffinity::After),
                },
            ],
            0,
        ));

        let result = document.execute_edit(EditIntent::TypeBracket('('));
        assert!(result.changed);
        assert_eq!(document.text(), "(one) (two)");
        let snapshot = document.snapshot();
        let selected = document
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .map(|range| snapshot.text_range(range))
            .collect::<Vec<_>>();
        assert_eq!(selected, vec!["one", "two"]);

        document.undo();
        assert_eq!(document.text(), "one two");
        assert_eq!(document.selected_text().as_deref(), Some("one"));
    }

    #[test]
    fn typing_tag_close_completes_html_and_xml_opening_tags() {
        let mut html = Document::from_file("index.html".into(), "<div", 0);
        html.set_caret(Position::new(0, 4), false);
        assert!(html.execute_edit(EditIntent::TypeTagClose).changed);
        assert_eq!(html.text(), "<div></div>");
        assert_eq!(html.cursor, Position::new(0, 5));
        html.undo();
        assert_eq!(html.text(), "<div");

        let mut xml = Document::from_file("node.xml".into(), "<Node", 0);
        xml.set_caret(Position::new(0, 5), false);
        xml.execute_edit(EditIntent::TypeTagClose);
        assert_eq!(xml.text(), "<Node></Node>");
        assert_eq!(xml.cursor, Position::new(0, 6));
    }

    #[test]
    fn typing_tag_close_skips_void_closing_self_closing_and_quoted_tags() {
        let mut void = Document::from_file("index.html".into(), "<img", 0);
        void.set_caret(Position::new(0, 4), false);
        void.execute_edit(EditIntent::TypeTagClose);
        assert_eq!(void.text(), "<img>");

        let mut self_closing = Document::from_file("index.html".into(), "<img/", 0);
        self_closing.set_caret(Position::new(0, 5), false);
        self_closing.execute_edit(EditIntent::TypeTagClose);
        assert_eq!(self_closing.text(), "<img/>");

        let mut closing = Document::from_file("index.html".into(), "</div", 0);
        closing.set_caret(Position::new(0, 5), false);
        closing.execute_edit(EditIntent::TypeTagClose);
        assert_eq!(closing.text(), "</div>");

        let mut quoted = Document::from_file("index.html".into(), "<div title=\"x", 0);
        quoted.set_caret(Position::new(0, 13), false);
        quoted.execute_edit(EditIntent::TypeTagClose);
        assert_eq!(quoted.text(), "<div title=\"x>");
    }

    #[test]
    fn typing_tag_close_uses_each_html_cursor_in_one_transaction() {
        let mut document = Document::from_file("index.html".into(), "<div\n<span", 0);
        document.set_caret(Position::new(0, 4), false);
        document.execute_edit(EditIntent::AddCursor(Position::new(1, 5)));

        document.execute_edit(EditIntent::TypeTagClose);

        assert_eq!(document.text(), "<div></div>\n<span></span>");
        let snapshot = document.snapshot();
        let cursors = document
            .selection_set()
            .selections()
            .iter()
            .map(|selection| selection.head.resolve(&snapshot))
            .collect::<Vec<_>>();
        assert_eq!(cursors, [Position::new(0, 5), Position::new(1, 6)]);
        document.undo();
        assert_eq!(document.text(), "<div\n<span");
    }

    #[test]
    fn delete_line_removes_the_current_line_and_undoes_as_one_transaction() {
        let mut document = doc("one\ntwo\nthree");
        document.set_caret(Position::new(1, 2), false);
        let result = document.execute_edit(EditIntent::DeleteLine);
        assert!(result.changed);
        assert_eq!(document.text(), "one\nthree");
        assert_eq!(document.cursor, Position::new(1, 0));
        document.undo();
        assert_eq!(document.text(), "one\ntwo\nthree");
    }

    #[test]
    fn delete_line_handles_first_last_empty_and_multi_cursor_lines() {
        let mut first = doc("one\ntwo");
        first.set_caret(Position::new(0, 0), false);
        first.execute_edit(EditIntent::DeleteLine);
        assert_eq!(first.text(), "two");

        let mut last = doc("one\ntwo");
        last.set_caret(Position::new(1, 0), false);
        last.execute_edit(EditIntent::DeleteLine);
        assert_eq!(last.text(), "one");

        let mut empty = doc("");
        assert!(!empty.execute_edit(EditIntent::DeleteLine).changed);
        assert_eq!(empty.text(), "");

        let mut multiple = doc("one\ntwo\nthree");
        multiple.set_caret(Position::new(0, 0), false);
        multiple.execute_edit(EditIntent::AddCursor(Position::new(2, 0)));
        multiple.execute_edit(EditIntent::DeleteLine);
        assert_eq!(multiple.text(), "two");
    }

    #[test]
    fn delete_line_treats_a_selection_ending_at_line_start_as_exclusive() {
        let mut document = doc("one\ntwo\nthree");
        let snapshot = document.snapshot();
        document.set_selection_set(SelectionSet::from_selections(
            vec![Selection {
                anchor: Anchor::new(
                    snapshot.position_to_byte(Position::new(0, 0)),
                    AnchorAffinity::Before,
                ),
                head: Anchor::new(
                    snapshot.position_to_byte(Position::new(1, 0)),
                    AnchorAffinity::After,
                ),
            }],
            0,
        ));
        document.execute_edit(EditIntent::DeleteLine);
        assert_eq!(document.text(), "two\nthree");
    }

    #[test]
    fn delete_to_line_boundaries_supports_multiple_cursors_and_undo() {
        let mut document = doc("ab cd\nef gh");
        document.set_caret(Position::new(0, 1), false);
        document.execute_edit(EditIntent::AddCursor(Position::new(1, 1)));
        document.execute_edit(EditIntent::DeleteTo(DeleteTarget::LineEnd));
        assert_eq!(document.text(), "a\ne");
        document.undo();
        assert_eq!(document.text(), "ab cd\nef gh");

        let mut start = doc("alpha beta");
        start.set_caret(Position::new(0, 6), false);
        start.execute_edit(EditIntent::DeleteTo(DeleteTarget::LineStart));
        assert_eq!(start.text(), "beta");
    }

    #[test]
    fn delete_to_word_and_subword_boundaries_handles_unicode_and_selections() {
        let mut next_word = doc("one, two three");
        next_word.set_caret(Position::new(0, 0), false);
        next_word.execute_edit(EditIntent::DeleteTo(DeleteTarget::NextWordEnd));
        assert_eq!(next_word.text(), ", two three");

        let mut previous_word = doc("one two");
        previous_word.set_caret(Position::new(0, 7), false);
        previous_word.execute_edit(EditIntent::DeleteTo(DeleteTarget::PreviousWordStart));
        assert_eq!(previous_word.text(), "one ");

        let mut next_subword = doc("parseHTTPValue42");
        next_subword.set_caret(Position::new(0, 0), false);
        next_subword.execute_edit(EditIntent::DeleteTo(DeleteTarget::NextSubwordEnd));
        assert_eq!(next_subword.text(), "HTTPValue42");

        let mut previous_subword = doc("λValue42_part");
        let end = previous_subword.snapshot().end();
        previous_subword.set_caret(end, false);
        previous_subword.execute_edit(EditIntent::DeleteTo(DeleteTarget::PreviousSubwordStart));
        assert_eq!(previous_subword.text(), "λValue42_");

        let mut selected = doc("delete this");
        selected.set_caret(Position::new(0, 0), false);
        selected.set_caret(Position::new(0, 6), true);
        selected.execute_edit(EditIntent::DeleteTo(DeleteTarget::NextWordEnd));
        assert_eq!(selected.text(), " this");
    }

    #[test]
    fn kill_to_line_end_captures_line_breaks_and_merges_overlapping_cursors() {
        let mut newline = doc("one\ntwo");
        newline.set_caret(Position::new(0, 3), false);
        let snapshot = newline.snapshot();
        let ranges = newline.kill_to_line_end_ranges();
        assert_eq!(ranges.len(), 1);
        assert_eq!(snapshot.text_range(ranges[0]), "\n");
        newline.execute_edit(EditIntent::KillToLineEnd);
        assert_eq!(newline.text(), "onetwo");
        newline.undo();
        assert_eq!(newline.text(), "one\ntwo");

        let mut overlapping = doc("abcdef");
        overlapping.set_caret(Position::new(0, 1), false);
        overlapping.execute_edit(EditIntent::AddCursor(Position::new(0, 3)));
        let ranges = overlapping.kill_to_line_end_ranges();
        assert_eq!(ranges.len(), 1);
        assert_eq!(overlapping.snapshot().text_range(ranges[0]), "bcdef");
        overlapping.execute_edit(EditIntent::KillToLineEnd);
        assert_eq!(overlapping.text(), "a");
    }

    #[test]
    fn join_lines_removes_the_line_ending_and_indentation_as_one_transaction() {
        let mut document = doc("hello\n  world");
        document.set_caret(Position::new(0, 2), false);

        let result = document.execute_edit(EditIntent::JoinLines);
        assert!(result.changed);
        assert_eq!(document.text(), "hello world");
        document.undo();
        assert_eq!(document.text(), "hello\n  world");
    }

    #[test]
    fn join_lines_handles_last_line_empty_lines_and_multiple_cursors() {
        let mut last_line = doc("one\ntwo");
        last_line.set_caret(Position::new(1, 0), false);
        assert!(!last_line.execute_edit(EditIntent::JoinLines).changed);
        assert_eq!(last_line.text(), "one\ntwo");

        let mut empty = doc("one\n\ntwo");
        empty.set_caret(Position::new(0, 0), false);
        empty.execute_edit(EditIntent::JoinLines);
        assert_eq!(empty.text(), "one\ntwo");

        let mut multiple = doc("a\nb\nc");
        multiple.set_caret(Position::new(0, 0), false);
        multiple.execute_edit(EditIntent::AddCursor(Position::new(1, 0)));
        multiple.execute_edit(EditIntent::JoinLines);
        assert_eq!(multiple.text(), "a b c");
        multiple.undo();
        assert_eq!(multiple.text(), "a\nb\nc");
    }

    #[test]
    fn join_lines_joins_each_line_boundary_covered_by_a_selection() {
        let mut document = doc("a\nb\nc");
        let snapshot = document.snapshot();
        document.set_selection_set(SelectionSet::from_selections(
            vec![Selection {
                anchor: Anchor::new(
                    snapshot.position_to_byte(Position::new(0, 0)),
                    AnchorAffinity::Before,
                ),
                head: Anchor::new(
                    snapshot.position_to_byte(Position::new(2, 0)),
                    AnchorAffinity::After,
                ),
            }],
            0,
        ));

        document.execute_edit(EditIntent::JoinLines);
        assert_eq!(document.text(), "a b c");
        document.undo();
        assert_eq!(document.text(), "a\nb\nc");
    }

    #[test]
    fn rewrap_formats_the_current_paragraph_to_the_requested_width_and_undoes() {
        let mut document = doc("alpha beta gamma delta");
        document.set_caret(Position::new(0, 7), false);

        let result = document.execute_edit(EditIntent::Rewrap { columns: 12 });
        assert!(result.changed);
        assert_eq!(document.text(), "alpha beta\ngamma delta");
        document.undo();
        assert_eq!(document.text(), "alpha beta gamma delta");
    }

    #[test]
    fn rewrap_preserves_comment_prefixes_and_blank_paragraph_boundaries() {
        let mut comment = doc("  // alpha beta gamma delta");
        comment.set_caret(Position::new(0, 0), false);
        comment.execute_edit(EditIntent::Rewrap { columns: 16 });
        assert_eq!(comment.text(), "  // alpha beta\n  // gamma delta");

        let mut paragraphs = doc("alpha beta gamma\n\nsecond paragraph words");
        paragraphs.execute_edit(EditIntent::SelectAll);
        paragraphs.execute_edit(EditIntent::Rewrap { columns: 11 });
        assert_eq!(
            paragraphs.text(),
            "alpha beta\ngamma\n\nsecond\nparagraph\nwords"
        );
        paragraphs.undo();
        assert_eq!(
            paragraphs.text(),
            "alpha beta gamma\n\nsecond paragraph words"
        );
    }

    #[test]
    fn enter_inserts_newline_and_auto_indents_the_caret() {
        let mut document = doc("  if {");
        document.set_caret(Position::new(0, 6), false);
        document.execute_edit(EditIntent::AutoIndent { unit: "  ".into() });

        assert_eq!(document.text(), "  if {\n    ");
        assert_eq!(document.cursor, Position::new(1, 4));
        document.undo();
        assert_eq!(document.text(), "  if {");
    }

    #[test]
    fn markdown_enter_continues_bullets_and_increments_numbered_items() {
        let mut bullet = Document::from_file("notes.md".into(), "  - first", 0);
        bullet.set_caret(Position::new(0, 9), false);
        bullet.execute_edit(EditIntent::AutoIndent { unit: "  ".into() });
        assert_eq!(bullet.text(), "  - first\n  - ");
        assert_eq!(bullet.cursor, Position::new(1, 4));

        let mut numbered = Document::from_file("notes.md".into(), "8. item", 0);
        numbered.set_caret(Position::new(0, 7), false);
        numbered.execute_edit(EditIntent::AutoIndent { unit: "  ".into() });
        assert_eq!(numbered.text(), "8. item\n9. ");
        assert_eq!(numbered.cursor, Position::new(1, 3));
    }

    #[test]
    fn markdown_enter_exits_an_empty_list_item() {
        let mut document = Document::from_file("notes.md".into(), "  - ", 0);
        document.set_caret(Position::new(0, 4), false);
        document.execute_edit(EditIntent::AutoIndent { unit: "  ".into() });
        assert_eq!(document.text(), "  \n  ");
        assert_eq!(document.cursor, Position::new(1, 2));

        let mut plain_text = Document::from_file("notes.txt".into(), "- item", 0);
        plain_text.set_caret(Position::new(0, 6), false);
        plain_text.execute_edit(EditIntent::AutoIndent { unit: "  ".into() });
        assert_eq!(plain_text.text(), "- item\n");
    }

    #[test]
    fn paste_distributes_or_broadcasts_replacements_across_selections() {
        let mut document = doc("a b");
        document.set_caret(Position::new(0, 1), false);
        document.execute_edit(EditIntent::AddCursor(Position::new(0, 3)));

        document.execute_edit(EditIntent::InsertMany(vec!["x".into(), "y".into()]));
        assert_eq!(document.text(), "ax by");
        document.undo();
        assert_eq!(document.text(), "a b");

        document.execute_edit(EditIntent::InsertMany(vec!["!".into()]));
        assert_eq!(document.text(), "a! b!");
        document.undo();
        assert_eq!(document.text(), "a b");
    }

    #[test]
    fn cut_deletes_only_non_empty_selections() {
        let mut document = doc("one two");
        document.set_selection_set(SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: Anchor::new(0, AnchorAffinity::Before),
                    head: Anchor::new(3, AnchorAffinity::After),
                },
                Selection::collapsed(Anchor::new(4, AnchorAffinity::After)),
            ],
            0,
        ));

        document.execute_edit(EditIntent::DeleteSelections);
        assert_eq!(document.text(), " two");
        document.undo();
        assert_eq!(document.text(), "one two");
    }
}
