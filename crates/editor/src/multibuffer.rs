//! Editable excerpts from several source documents in one virtual view.
//!
//! A Multi-Buffer keeps each file's text and history in its original coordinate
//! space. The rendered text is only a projection: edits are routed back to the
//! owning file, excerpts follow those edits, and one multi-file edit is one
//! undo/redo step.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};

use crate::buffer::{
    Anchor, AnchorAffinity, BufferSnapshot, Edit, EditSource, EditorBuffer, EditorError, Position,
    Revision, SelectionSet, TextRange, Transaction,
};
use crate::search::{map_range_through_edits, ProjectSearchHit};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExcerptId(u64);

impl ExcerptId {
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// One source file and the ranges shown from it in the virtual buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiBufferSource {
    pub path: PathBuf,
    pub text: String,
    pub excerpt_ranges: Vec<TextRange>,
    pub can_edit: bool,
}

impl MultiBufferSource {
    pub fn whole_file(path: impl Into<PathBuf>, text: impl Into<String>, can_edit: bool) -> Self {
        let text = text.into();
        let range = TextRange::new(0, text.len());
        Self {
            path: path.into(),
            text,
            excerpt_ranges: vec![range],
            can_edit,
        }
    }
}

/// Full text loaded by the Workspace adapter for a project-search preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiBufferFileContent {
    pub path: PathBuf,
    pub text: String,
    pub can_edit: bool,
}

/// An edit expressed relative to one excerpt's current source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiBufferEdit {
    pub excerpt: ExcerptId,
    pub range: TextRange,
    pub replacement: String,
}

impl MultiBufferEdit {
    pub fn new(excerpt: ExcerptId, range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            excerpt,
            range,
            replacement: replacement.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MultiBufferError {
    DuplicateFile(PathBuf),
    UnknownFile(PathBuf),
    UnknownExcerpt(ExcerptId),
    InvalidExcerptRange {
        path: PathBuf,
        range: TextRange,
    },
    ReadOnly(PathBuf),
    StaleRevision {
        path: PathBuf,
        expected: Revision,
        actual: Revision,
    },
    Buffer(EditorError),
}

impl fmt::Display for MultiBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFile(path) => write!(f, "{} appears more than once", path.display()),
            Self::UnknownFile(path) => write!(f, "{} is not in this Multi-Buffer", path.display()),
            Self::UnknownExcerpt(id) => write!(f, "excerpt {} is no longer available", id.value()),
            Self::InvalidExcerptRange { path, range } => write!(
                f,
                "invalid excerpt range {}..{} for {}",
                range.start,
                range.end,
                path.display()
            ),
            Self::ReadOnly(path) => write!(f, "{} is read-only", path.display()),
            Self::StaleRevision {
                path,
                expected,
                actual,
            } => write!(
                f,
                "{} changed from revision {} to {}",
                path.display(),
                expected.value(),
                actual.value()
            ),
            Self::Buffer(error) => write!(f, "buffer transaction failed: {error:?}"),
        }
    }
}

impl std::error::Error for MultiBufferError {}

impl From<EditorError> for MultiBufferError {
    fn from(error: EditorError) -> Self {
        Self::Buffer(error)
    }
}

#[derive(Debug)]
struct FileBuffer {
    path: PathBuf,
    buffer: EditorBuffer,
    saved_text: String,
    can_edit: bool,
}

#[derive(Clone, Debug)]
struct Excerpt {
    id: ExcerptId,
    file_index: usize,
    start: Anchor,
    end: Anchor,
}

#[derive(Clone, Debug)]
struct HistoryFrame {
    transactions: Vec<(usize, Transaction)>,
}

/// Mutable text from several files, presented as independently editable excerpts.
pub struct MultiBuffer {
    files: Vec<FileBuffer>,
    file_indices: HashMap<PathBuf, usize>,
    excerpts: Vec<Excerpt>,
    next_excerpt_id: u64,
    revision: u64,
    undo: Vec<HistoryFrame>,
    redo: Vec<HistoryFrame>,
}

#[derive(Clone, Debug)]
pub struct MultiBufferFileSnapshot {
    pub path: PathBuf,
    pub snapshot: BufferSnapshot,
    pub can_edit: bool,
    pub is_dirty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiBufferExcerptSnapshot {
    pub id: ExcerptId,
    pub path: PathBuf,
    pub source_range: TextRange,
    /// Range occupied by source text in the rendered Multi-Buffer projection.
    pub view_range: TextRange,
    source_file_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiBufferLocation {
    pub excerpt: ExcerptId,
    pub path: PathBuf,
    pub source_offset: usize,
    pub position: Position,
    pub revision: Revision,
}

#[derive(Clone, Debug)]
pub struct MultiBufferSnapshot {
    revision: u64,
    text: String,
    excerpts: Vec<MultiBufferExcerptSnapshot>,
    source_snapshots: Vec<BufferSnapshot>,
}

impl MultiBufferSnapshot {
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn excerpts(&self) -> &[MultiBufferExcerptSnapshot] {
        &self.excerpts
    }

    /// Map a virtual byte offset to the source file and position it displays.
    /// Headers and separators intentionally have no source location.
    pub fn location_at(&self, offset: usize) -> Option<MultiBufferLocation> {
        let excerpt = self.excerpts.iter().find(|excerpt| {
            excerpt.view_range.start <= offset && offset <= excerpt.view_range.end
        })?;
        let snapshot = self.source_snapshots.get(excerpt.source_file_index)?;
        let relative = offset
            .saturating_sub(excerpt.view_range.start)
            .min(excerpt.source_range.len());
        let source_offset = excerpt.source_range.start.saturating_add(relative);
        Some(MultiBufferLocation {
            excerpt: excerpt.id,
            path: excerpt.path.clone(),
            source_offset,
            position: snapshot.byte_to_position(source_offset),
            revision: snapshot.revision(),
        })
    }
}

impl MultiBuffer {
    pub fn new(sources: Vec<MultiBufferSource>) -> Result<Self, MultiBufferError> {
        let mut files = Vec::with_capacity(sources.len());
        let mut file_indices = HashMap::with_capacity(sources.len());
        let mut excerpts = Vec::new();
        let mut next_excerpt_id = 1_u64;
        for source in sources {
            if file_indices.contains_key(&source.path) {
                return Err(MultiBufferError::DuplicateFile(source.path));
            }
            for range in &source.excerpt_ranges {
                if !valid_utf8_range(&source.text, *range) {
                    return Err(MultiBufferError::InvalidExcerptRange {
                        path: source.path,
                        range: *range,
                    });
                }
            }
            let file_index = files.len();
            let saved_text = source.text.clone();
            files.push(FileBuffer {
                path: source.path.clone(),
                buffer: EditorBuffer::from_text(&source.text),
                saved_text,
                can_edit: source.can_edit,
            });
            file_indices.insert(source.path.clone(), file_index);
            for range in source.excerpt_ranges {
                excerpts.push(Excerpt {
                    id: ExcerptId(next_excerpt_id),
                    file_index,
                    start: Anchor::new(range.start, AnchorAffinity::Before),
                    end: Anchor::new(range.end, AnchorAffinity::After),
                });
                next_excerpt_id = next_excerpt_id.saturating_add(1);
            }
        }
        Ok(Self {
            files,
            file_indices,
            excerpts,
            next_excerpt_id,
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
        })
    }

    /// Build context excerpts around project-search hits from loaded file text.
    /// Each unique source file is stored once even when it contributes many hits.
    pub fn from_project_search(
        hits: &[ProjectSearchHit],
        files: Vec<MultiBufferFileContent>,
        context_lines: usize,
    ) -> Result<Self, MultiBufferError> {
        let hit_paths = hits
            .iter()
            .map(|hit| hit.path.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let mut sources = Vec::new();
        let mut loaded_paths = std::collections::BTreeSet::new();
        for file in files {
            let path_key = file.path.to_string_lossy().into_owned();
            if !hit_paths.contains(&path_key) {
                continue;
            }
            let snapshot = EditorBuffer::from_text(&file.text).snapshot();
            let mut ranges = hits
                .iter()
                .filter(|hit| hit.path == path_key)
                .map(|hit| {
                    let line_index =
                        usize::try_from(hit.line.saturating_sub(1)).unwrap_or(usize::MAX);
                    let start_line = line_index.saturating_sub(context_lines);
                    let end_line = line_index
                        .saturating_add(context_lines)
                        .saturating_add(1)
                        .min(snapshot.line_count());
                    let start = snapshot.position_to_byte(Position::new(start_line, 0));
                    let end = if end_line >= snapshot.line_count() {
                        snapshot.byte_len()
                    } else {
                        snapshot.position_to_byte(Position::new(end_line, 0))
                    };
                    TextRange::new(start, end)
                })
                .collect::<Vec<_>>();
            ranges.sort_by_key(|range| (range.start, range.end));
            let mut merged_ranges: Vec<TextRange> = Vec::with_capacity(ranges.len());
            for range in ranges {
                if let Some(previous) = merged_ranges.last_mut() {
                    if range.start <= previous.end {
                        previous.end = previous.end.max(range.end);
                        continue;
                    }
                }
                merged_ranges.push(range);
            }
            if !merged_ranges.is_empty() {
                loaded_paths.insert(path_key);
                sources.push(MultiBufferSource {
                    path: file.path,
                    text: file.text,
                    excerpt_ranges: merged_ranges,
                    can_edit: file.can_edit,
                });
            }
        }
        if let Some(missing) = hit_paths.difference(&loaded_paths).next() {
            return Err(MultiBufferError::UnknownFile(PathBuf::from(missing)));
        }
        Self::new(sources)
    }

    pub fn snapshot(&self) -> MultiBufferSnapshot {
        let source_snapshots = self
            .files
            .iter()
            .map(|file| file.buffer.snapshot())
            .collect::<Vec<_>>();
        let mut text = String::new();
        let mut snapshots = Vec::with_capacity(self.excerpts.len());
        for excerpt in &self.excerpts {
            let file = &self.files[excerpt.file_index];
            let source = &source_snapshots[excerpt.file_index];
            let source_range = TextRange::new(excerpt.start.offset(), excerpt.end.offset());
            let line = source
                .byte_to_position(source_range.start)
                .line
                .saturating_add(1);
            let label = sanitize_header(&file.path.to_string_lossy());
            text.push_str(&format!("--- {label}:{line} ---\n"));
            let view_start = text.len();
            text.push_str(&source.text_range(source_range));
            let view_end = text.len();
            if !text.ends_with('\n') {
                text.push('\n');
            }
            snapshots.push(MultiBufferExcerptSnapshot {
                id: excerpt.id,
                path: file.path.clone(),
                source_range,
                view_range: TextRange::new(view_start, view_end),
                source_file_index: excerpt.file_index,
            });
        }
        MultiBufferSnapshot {
            revision: self.revision,
            text,
            excerpts: snapshots,
            source_snapshots,
        }
    }

    pub fn file_snapshots(&self) -> Vec<MultiBufferFileSnapshot> {
        self.files
            .iter()
            .map(|file| {
                let snapshot = file.buffer.snapshot();
                MultiBufferFileSnapshot {
                    path: file.path.clone(),
                    is_dirty: snapshot.text() != file.saved_text,
                    snapshot,
                    can_edit: file.can_edit,
                }
            })
            .collect()
    }

    pub fn excerpt_count(&self) -> usize {
        self.excerpts.len()
    }

    pub fn is_dirty(&self, path: &Path) -> Result<bool, MultiBufferError> {
        let file_index = self.file_index(path)?;
        let file = &self.files[file_index];
        Ok(file.buffer.text() != file.saved_text)
    }

    /// Add another view onto an existing source file without duplicating its buffer.
    pub fn add_excerpt(
        &mut self,
        path: &Path,
        range: TextRange,
    ) -> Result<ExcerptId, MultiBufferError> {
        let file_index = self.file_index(path)?;
        let text = self.files[file_index].buffer.text();
        if !valid_utf8_range(&text, range) {
            return Err(MultiBufferError::InvalidExcerptRange {
                path: path.to_path_buf(),
                range,
            });
        }
        let id = ExcerptId(self.next_excerpt_id);
        self.next_excerpt_id = self.next_excerpt_id.saturating_add(1);
        self.excerpts.push(Excerpt {
            id,
            file_index,
            start: Anchor::new(range.start, AnchorAffinity::Before),
            end: Anchor::new(range.end, AnchorAffinity::After),
        });
        self.revision = self.revision.saturating_add(1);
        Ok(id)
    }

    /// Apply edits in their excerpt-local coordinates as one multi-file transaction.
    pub fn apply_edits(&mut self, edits: Vec<MultiBufferEdit>) -> Result<usize, MultiBufferError> {
        if edits.is_empty() {
            return Ok(0);
        }
        let mut grouped: BTreeMap<usize, Vec<Edit>> = BTreeMap::new();
        for edit in edits {
            if edit.range.is_empty() && edit.replacement.is_empty() {
                continue;
            }
            let excerpt = self
                .excerpts
                .iter()
                .find(|excerpt| excerpt.id == edit.excerpt)
                .ok_or(MultiBufferError::UnknownExcerpt(edit.excerpt))?;
            let file = &self.files[excerpt.file_index];
            if !file.can_edit {
                return Err(MultiBufferError::ReadOnly(file.path.clone()));
            }
            let excerpt_range = TextRange::new(excerpt.start.offset(), excerpt.end.offset());
            if edit.range.start > edit.range.end || edit.range.end > excerpt_range.len() {
                return Err(MultiBufferError::InvalidExcerptRange {
                    path: file.path.clone(),
                    range: edit.range,
                });
            }
            grouped
                .entry(excerpt.file_index)
                .or_default()
                .push(Edit::new(
                    TextRange::new(
                        excerpt_range.start.saturating_add(edit.range.start),
                        excerpt_range.start.saturating_add(edit.range.end),
                    ),
                    edit.replacement,
                ));
        }
        let mut transactions = Vec::with_capacity(grouped.len());
        let mut replacements = 0;
        for (file_index, mut file_edits) in grouped {
            file_edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
            let file = &self.files[file_index];
            let source_text = file.buffer.text();
            if let Some(edit) = file_edits
                .iter()
                .find(|edit| !valid_utf8_range(&source_text, edit.range))
            {
                return Err(MultiBufferError::InvalidExcerptRange {
                    path: file.path.clone(),
                    range: edit.range,
                });
            }
            let transaction = Transaction::new(
                file.buffer.revision(),
                file_edits.clone(),
                SelectionSet::default(),
                SelectionSet::default(),
                EditSource::External,
                Some("Edit Multi-Buffer excerpts".to_string()),
            )?;
            replacements += file_edits.len();
            transactions.push((file_index, transaction));
        }
        if transactions.is_empty() {
            return Ok(0);
        }

        let mut inverse_transactions = Vec::with_capacity(transactions.len());
        for (file_index, transaction) in &transactions {
            match self.files[*file_index]
                .buffer
                .apply_transaction(transaction)
            {
                Ok(applied) => {
                    self.map_excerpts(*file_index, transaction.edits());
                    inverse_transactions.push((*file_index, applied.inverse));
                }
                Err(error) => {
                    for (rollback_index, inverse) in inverse_transactions.into_iter().rev() {
                        let inverse_edits = inverse.edits().to_vec();
                        let _ = self.files[rollback_index]
                            .buffer
                            .apply_transaction(&inverse);
                        self.map_excerpts(rollback_index, &inverse_edits);
                    }
                    return Err(error.into());
                }
            }
        }
        self.undo.push(HistoryFrame {
            transactions: inverse_transactions,
        });
        self.redo.clear();
        self.revision = self.revision.saturating_add(1);
        Ok(replacements)
    }

    pub fn undo(&mut self) -> Result<bool, MultiBufferError> {
        let Some(frame) = self.undo.pop() else {
            return Ok(false);
        };
        match self.apply_history_frame(frame) {
            Ok(redo_frame) => {
                self.redo.push(redo_frame);
                self.revision = self.revision.saturating_add(1);
                Ok(true)
            }
            Err((frame, error)) => {
                self.undo.push(frame);
                Err(error)
            }
        }
    }

    pub fn redo(&mut self) -> Result<bool, MultiBufferError> {
        let Some(frame) = self.redo.pop() else {
            return Ok(false);
        };
        match self.apply_history_frame(frame) {
            Ok(undo_frame) => {
                self.undo.push(undo_frame);
                self.revision = self.revision.saturating_add(1);
                Ok(true)
            }
            Err((frame, error)) => {
                self.redo.push(frame);
                Err(error)
            }
        }
    }

    /// Mark one file saved only if the write corresponds to its current revision.
    pub fn mark_saved(&mut self, path: &Path, revision: Revision) -> Result<(), MultiBufferError> {
        let file_index = self.file_index(path)?;
        let file = &mut self.files[file_index];
        let actual = file.buffer.revision();
        if actual != revision {
            return Err(MultiBufferError::StaleRevision {
                path: path.to_path_buf(),
                expected: revision,
                actual,
            });
        }
        file.saved_text = file.buffer.text();
        Ok(())
    }

    fn apply_history_frame(
        &mut self,
        frame: HistoryFrame,
    ) -> Result<HistoryFrame, (HistoryFrame, MultiBufferError)> {
        for (file_index, transaction) in &frame.transactions {
            let file = &self.files[*file_index];
            if transaction.base_revision() != file.buffer.revision() {
                let error = MultiBufferError::StaleRevision {
                    path: file.path.clone(),
                    expected: transaction.base_revision(),
                    actual: file.buffer.revision(),
                };
                return Err((frame, error));
            }
        }
        let mut inverse_transactions = Vec::with_capacity(frame.transactions.len());
        for (file_index, transaction) in &frame.transactions {
            let edits = transaction.edits().to_vec();
            match self.files[*file_index]
                .buffer
                .apply_transaction(transaction)
            {
                Ok(applied) => {
                    self.map_excerpts(*file_index, &edits);
                    inverse_transactions.push((*file_index, applied.inverse));
                }
                Err(error) => {
                    return Err((frame, MultiBufferError::Buffer(error)));
                }
            }
        }
        Ok(HistoryFrame {
            transactions: inverse_transactions,
        })
    }

    fn map_excerpts(&mut self, file_index: usize, edits: &[Edit]) {
        for excerpt in self
            .excerpts
            .iter_mut()
            .filter(|excerpt| excerpt.file_index == file_index)
        {
            let mapped = map_range_through_edits(
                TextRange::new(excerpt.start.offset(), excerpt.end.offset()),
                edits,
            );
            excerpt.start = Anchor::new(mapped.start, AnchorAffinity::Before);
            excerpt.end = Anchor::new(mapped.end, AnchorAffinity::After);
        }
    }

    fn file_index(&self, path: &Path) -> Result<usize, MultiBufferError> {
        self.file_indices
            .get(path)
            .copied()
            .ok_or_else(|| MultiBufferError::UnknownFile(path.to_path_buf()))
    }
}

fn valid_utf8_range(text: &str, range: TextRange) -> bool {
    range.start <= range.end
        && range.end <= text.len()
        && text.is_char_boundary(range.start)
        && text.is_char_boundary(range.end)
}

fn sanitize_header(path: &str) -> String {
    path.replace('\n', "\\n").replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Vec<MultiBufferSource> {
        vec![
            MultiBufferSource::whole_file("/repo/a.rs", "first\nneedle\nlast\n", true),
            MultiBufferSource::whole_file("/repo/b.rs", "before\nneedle\nafter\n", true),
        ]
    }

    #[test]
    fn snapshot_maps_virtual_offsets_back_to_file_positions() {
        let buffer = MultiBuffer::new(sources()).expect("valid sources");
        let snapshot = buffer.snapshot();
        let excerpt = snapshot
            .excerpts()
            .iter()
            .find(|excerpt| excerpt.path == Path::new("/repo/b.rs"))
            .expect("second file excerpt");
        let location = snapshot
            .location_at(excerpt.view_range.start + "before\n".len())
            .expect("source location");
        assert_eq!(location.path, Path::new("/repo/b.rs"));
        assert_eq!(location.position, Position::new(1, 0));
        assert_eq!(location.source_offset, "before\n".len());
        assert!(snapshot.text().contains("--- /repo/a.rs:1 ---"));
        assert!(snapshot.text().contains("--- /repo/b.rs:1 ---"));
    }

    #[test]
    fn edits_across_files_undo_and_redo_as_one_transaction() {
        let mut buffer = MultiBuffer::new(sources()).expect("valid sources");
        let before = buffer.snapshot();
        let edits = before
            .excerpts()
            .iter()
            .map(|excerpt| {
                let start = excerpt.source_range.start;
                let file_text = buffer
                    .file_snapshots()
                    .into_iter()
                    .find(|file| file.path == excerpt.path)
                    .expect("file snapshot")
                    .snapshot
                    .text();
                let match_start = file_text[start..].find("needle").expect("match") + start;
                MultiBufferEdit::new(
                    excerpt.id,
                    TextRange::new(
                        match_start - excerpt.source_range.start,
                        match_start - excerpt.source_range.start + "needle".len(),
                    ),
                    "updated",
                )
            })
            .collect();
        assert_eq!(buffer.apply_edits(edits).expect("apply"), 2);
        assert!(buffer.is_dirty(Path::new("/repo/a.rs")).expect("file"));
        assert!(buffer.snapshot().text().contains("updated"));
        assert!(buffer.undo().expect("undo"));
        assert!(buffer.snapshot().text().contains("needle"));
        assert!(!buffer.snapshot().text().contains("updated"));
        assert!(buffer.redo().expect("redo"));
        assert!(buffer.snapshot().text().contains("updated"));
        assert!(buffer.undo().expect("undo after redo"));
        assert!(!buffer.undo().expect("empty undo"));
    }

    #[test]
    fn edits_are_rejected_for_read_only_files_and_invalid_utf8_ranges() {
        let source = MultiBufferSource {
            path: PathBuf::from("/repo/readonly.rs"),
            text: "aλb".to_string(),
            excerpt_ranges: vec![TextRange::new(0, 4)],
            can_edit: false,
        };
        let mut buffer = MultiBuffer::new(vec![source]).expect("valid source");
        let excerpt = buffer.snapshot().excerpts()[0].id;
        assert!(matches!(
            buffer.apply_edits(vec![MultiBufferEdit::new(
                excerpt,
                TextRange::new(1, 1),
                "x"
            )]),
            Err(MultiBufferError::ReadOnly(_))
        ));

        assert!(matches!(
            MultiBuffer::new(vec![MultiBufferSource {
                path: PathBuf::from("/repo/invalid.rs"),
                text: "aλb".to_string(),
                excerpt_ranges: vec![TextRange::new(2, 3)],
                can_edit: true,
            }]),
            Err(MultiBufferError::InvalidExcerptRange { .. })
        ));

        let mut editable = MultiBuffer::new(vec![MultiBufferSource::whole_file(
            "/repo/editable.rs",
            "aλb",
            true,
        )])
        .expect("valid source");
        let excerpt = editable.snapshot().excerpts()[0].id;
        assert!(matches!(
            editable.apply_edits(vec![MultiBufferEdit::new(
                excerpt,
                TextRange::new(1, 2),
                "x"
            )]),
            Err(MultiBufferError::InvalidExcerptRange { .. })
        ));
        assert!(!editable
            .is_dirty(Path::new("/repo/editable.rs"))
            .expect("file"));
    }

    #[test]
    fn excerpt_anchors_follow_insertions_before_the_excerpt_and_saved_revisions() {
        let source = MultiBufferSource {
            path: PathBuf::from("/repo/anchor.rs"),
            text: "prefix needle suffix".to_string(),
            excerpt_ranges: vec![TextRange::new(0, 20), TextRange::new(7, 13)],
            can_edit: true,
        };
        let mut buffer = MultiBuffer::new(vec![source]).expect("valid source");
        let before = buffer.snapshot();
        let excerpt = before.excerpts()[0].id;
        buffer
            .apply_edits(vec![MultiBufferEdit::new(
                excerpt,
                TextRange::new(0, 0),
                "inserted ",
            )])
            .expect("insert");
        let after = buffer.snapshot();
        let excerpt = after
            .excerpts()
            .iter()
            .find(|excerpt| excerpt.source_range.start == 16)
            .expect("following excerpt moved with its source");
        assert_eq!(excerpt.source_range, TextRange::new(16, 22));
        let file = buffer.file_snapshots().remove(0);
        assert!(file.is_dirty);
        buffer
            .mark_saved(Path::new("/repo/anchor.rs"), file.snapshot.revision())
            .expect("mark saved");
        assert!(!buffer.is_dirty(Path::new("/repo/anchor.rs")).expect("file"));
    }

    #[test]
    fn multiple_edits_in_one_file_share_the_same_undo_step() {
        let mut buffer = MultiBuffer::new(vec![MultiBufferSource::whole_file(
            "/repo/repeat.rs",
            "needle + needle",
            true,
        )])
        .expect("valid source");
        let snapshot = buffer.snapshot();
        let excerpt = snapshot.excerpts()[0].id;
        let edits = [0, 9]
            .into_iter()
            .map(|start| MultiBufferEdit::new(excerpt, TextRange::new(start, start + 6), "updated"))
            .collect();
        assert_eq!(buffer.apply_edits(edits).expect("apply"), 2);
        assert_eq!(
            buffer.file_snapshots()[0].snapshot.text(),
            "updated + updated"
        );
        assert!(buffer.undo().expect("undo"));
        assert_eq!(
            buffer.file_snapshots()[0].snapshot.text(),
            "needle + needle"
        );
        assert!(buffer.redo().expect("redo"));
        assert_eq!(
            buffer.file_snapshots()[0].snapshot.text(),
            "updated + updated"
        );
    }

    #[test]
    fn project_search_builds_context_excerpts_and_deduplicates_files() {
        let path = Path::new("/repo/a.rs");
        let hits = vec![
            ProjectSearchHit::with_relative_path(path.to_string_lossy(), "a.rs", 2, "needle", 0, 6),
            ProjectSearchHit::with_relative_path(path.to_string_lossy(), "a.rs", 5, "needle", 0, 6),
        ];
        let buffer = MultiBuffer::from_project_search(
            &hits,
            vec![MultiBufferFileContent {
                path: path.to_path_buf(),
                text: "one\nneedle\nthree\nfour\nneedle\nsix\n".to_string(),
                can_edit: true,
            }],
            0,
        )
        .expect("search hits resolve to excerpts");
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.excerpts().len(), 2);
        assert_eq!(snapshot.excerpts()[0].source_range, TextRange::new(4, 11));
        assert_eq!(snapshot.excerpts()[1].source_range, TextRange::new(22, 29));
        assert_eq!(buffer.file_snapshots().len(), 1);
        assert_eq!(snapshot.text().matches("--- /repo/a.rs:").count(), 2);
        assert!(snapshot.text().contains("needle\n"));
    }
}
