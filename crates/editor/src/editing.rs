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
            EditIntent::Move { motion, extend } => self.multi_move(motion, extend),
            EditIntent::AddCursor(position) => self.add_cursor(position),
            EditIntent::AddCursorAbove => self.add_cursors_vertical(false),
            EditIntent::AddCursorBelow => self.add_cursors_vertical(true),
            EditIntent::SelectNextOccurrence => self.select_next_occurrence(),
            EditIntent::SelectPreviousOccurrence => self.select_previous_occurrence(),
            EditIntent::SelectAllOccurrences => self.select_all_occurrences(),
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
                    .or_else(|| line_range(&snapshot, range.start))
                    .unwrap_or(range)
            } else {
                line_range(&snapshot, range.start).unwrap_or(range)
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
                Selection::collapsed(Anchor::new(range.start, AnchorAffinity::After))
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
        for selection in selections.selections() {
            let range = selection.range().resolve(&snapshot);
            targets.push((range, format!("{ch}{close}")));
        }
        let edits = targets
            .iter()
            .map(|(range, replacement)| Edit::new(*range, replacement.clone()))
            .collect::<Vec<_>>();
        let after_offsets = targets
            .iter()
            .map(|(range, _)| {
                if range.is_empty() {
                    map_offset(range.start, &edits, AnchorAffinity::Before) + ch.len_utf8()
                } else {
                    map_offset(range.end, &edits, AnchorAffinity::After)
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
            let after = after_offsets
                .into_iter()
                .map(|offset| Selection::collapsed(Anchor::new(offset, AnchorAffinity::After)))
                .collect();
            self.set_selection_set(SelectionSet::from_selections(after, 0));
        }
        result
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
