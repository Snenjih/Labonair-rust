//! Editor-owned Git gutter and review bridge.
//!
//! Git remains the owner of repository state and mutations. This module only
//! turns an injected, read-only unified diff into immutable editor decorations
//! and typed intents. The workspace can adapt those intents to the existing
//! `labonair-git` and Project Diff contracts without exposing either model to
//! the editor.

use std::future::Future;
use std::pin::Pin;

use crate::display::DisplaySnapshot;
use crate::unified::{parse_diff_hunks, DiffHunk, FileDiff};
use crate::{BufferSnapshot, Position, Revision};

/// A line-level change kind suitable for a gutter marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GitHunkStatus {
    Added,
    Modified,
    Deleted,
    /// A new file that is not in the repository index yet.
    Untracked,
}

/// The availability of Git information for one editor document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitGutterState {
    Unavailable,
    Loading,
    NoRepository,
    Ready,
    Empty,
    Error(String),
}

/// Stable identity for an editor hunk action. The revision prevents an action
/// created for an old buffer/diff pairing from being applied to a new one.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GitHunkTarget {
    pub path: String,
    pub hunk_id: String,
    pub revision: Revision,
}

/// A preview required before a destructive hunk discard/revert operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitDiscardPreview {
    pub target: GitHunkTarget,
    pub summary: String,
    pub patch: String,
}

/// Typed editor intents. Git or Project Diff adapters execute these; the
/// editor never mutates the repository or index itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitHunkAction {
    Stage {
        target: GitHunkTarget,
        patch: String,
    },
    Unstage {
        target: GitHunkTarget,
        patch: String,
    },
    Discard {
        preview: GitDiscardPreview,
    },
    OpenProjectDiff {
        target: GitHunkTarget,
    },
    ShowChange {
        target: GitHunkTarget,
    },
}

/// A hunk parsed from the provider's unified diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHunk {
    pub id: String,
    pub status: GitHunkStatus,
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// New-file line numbers (zero-based) changed by `+` lines in this hunk.
    /// A pure deletion has no such line and uses the hunk's insertion anchor.
    pub changed_lines: Vec<usize>,
    /// The exact patch accepted by the Git adapter. Keeping it in the
    /// projection avoids reconstructing or reinterpreting a hunk in the UI.
    pub patch: String,
    /// Conservative intra-line decorations derived from this hunk's paired
    /// delete/add runs. The editor never reconstructs these from the buffer.
    pub inline_decorations: Vec<GitInlineDecoration>,
}

impl GitHunk {
    pub fn target(&self, path: &str, revision: Revision) -> GitHunkTarget {
        GitHunkTarget {
            path: path.to_string(),
            hunk_id: self.id.clone(),
            revision,
        }
    }

    pub fn discard_preview(&self, path: &str, revision: Revision) -> GitDiscardPreview {
        GitDiscardPreview {
            target: self.target(path, revision),
            summary: format!("{} lines changed", self.new_lines.max(self.old_lines)),
            patch: self.patch.clone(),
        }
    }

    pub fn status_label(&self) -> &'static str {
        match self.status {
            GitHunkStatus::Added => "Added",
            GitHunkStatus::Modified => "Modified",
            GitHunkStatus::Deleted => "Deleted",
            GitHunkStatus::Untracked => "Untracked",
        }
    }

    pub fn line_range_label(&self) -> String {
        if self.new_lines <= 1 {
            format!("line {}", self.new_start)
        } else {
            format!(
                "lines {}–{}",
                self.new_start,
                self.new_start + self.new_lines - 1
            )
        }
    }

    pub fn summary(&self) -> String {
        format!("{} old → {} new lines", self.old_lines, self.new_lines)
    }

    /// Keep context-menu previews compact while preserving the original patch
    /// text. The full patch remains on the typed stage/unstage/discard intent.
    pub fn patch_preview(&self, max_lines: usize) -> String {
        self.patch
            .lines()
            .skip_while(|line| !line.starts_with("@@"))
            .take(max_lines.max(1))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One logical line that receives a gutter marker. Deleted hunks point at the
/// insertion anchor (`new_start - 1`) because no new line exists to decorate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitLineDecoration {
    pub line: usize,
    pub status: GitHunkStatus,
    pub hunk_id: String,
}

/// The kind of source-side text change represented by an inline decoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GitInlineChangeKind {
    Inserted,
    Deleted,
    Replaced,
}

/// A conservative intra-line change on the new-file side of a hunk.
///
/// `start_column..end_column` is measured in Unicode scalar values, not UTF-8
/// bytes. A deletion has a zero-width range at its insertion anchor; the UI
/// may render that as a small marker without inventing text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitInlineDecoration {
    pub line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub kind: GitInlineChangeKind,
    /// Exact changed source text on the new side. It is empty for a deletion.
    pub source_text: String,
    pub hunk_id: String,
}

/// The immutable, revision-bound editor projection of Git state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitGutterSnapshot {
    pub path: String,
    pub revision: Revision,
    pub state: GitGutterState,
    /// Whether this projection compares the index with `HEAD` rather than
    /// the worktree with the index. The repository decides this before the
    /// diff is requested; the editor only uses it to label actions.
    pub staged: bool,
    pub hunks: Vec<GitHunk>,
    pub decorations: Vec<GitLineDecoration>,
    pub inline_decorations: Vec<GitInlineDecoration>,
}

impl GitGutterSnapshot {
    pub fn unavailable(path: impl Into<String>, revision: Revision) -> Self {
        Self::with_state(path, revision, GitGutterState::Unavailable)
    }

    pub fn loading(path: impl Into<String>, revision: Revision) -> Self {
        Self::with_state(path, revision, GitGutterState::Loading)
    }

    pub fn no_repository(path: impl Into<String>, revision: Revision) -> Self {
        Self::with_state(path, revision, GitGutterState::NoRepository)
    }

    pub fn error(path: impl Into<String>, revision: Revision, message: impl Into<String>) -> Self {
        Self::with_state(path, revision, GitGutterState::Error(message.into()))
    }

    pub fn empty(path: impl Into<String>, revision: Revision) -> Self {
        Self::with_state(path, revision, GitGutterState::Empty)
    }

    fn with_state(path: impl Into<String>, revision: Revision, state: GitGutterState) -> Self {
        Self {
            path: path.into(),
            revision,
            state,
            staged: false,
            hunks: Vec::new(),
            decorations: Vec::new(),
            inline_decorations: Vec::new(),
        }
    }

    /// Attach the repository's comparison side to an already parsed
    /// projection. This keeps staging knowledge outside the editor parser.
    pub fn with_staged(mut self, staged: bool) -> Self {
        self.staged = staged;
        self
    }

    /// Build a snapshot for one file from the existing unified-diff parser.
    /// The parser is shared with Project Diff; no Git state is copied here.
    pub fn from_unified_diff(
        path: impl Into<String>,
        revision: Revision,
        diff: &str,
    ) -> Result<Self, GitGutterError> {
        let path = path.into();
        let files = parse_diff_hunks(diff);
        let file = files
            .iter()
            .find(|file| same_path(&file.path, &path))
            .or_else(|| (files.len() == 1).then(|| &files[0]))
            .ok_or_else(|| {
                if diff.is_empty() {
                    GitGutterError::EmptyDiff
                } else if diff.contains("[diff truncated") || diff.contains("[diff too large]") {
                    GitGutterError::InvalidDiff("diff output was truncated".to_string())
                } else {
                    GitGutterError::FileNotFound(path.clone())
                }
            })?;
        Ok(Self::from_file_diff(path, revision, file))
    }

    pub fn from_file_diff(path: impl Into<String>, revision: Revision, file: &FileDiff) -> Self {
        let path = path.into();
        let hunks = file
            .hunks
            .iter()
            .enumerate()
            .map(|(index, hunk)| make_hunk(file, hunk, index))
            .collect::<Vec<_>>();
        let mut decorations = Vec::new();
        let mut inline_decorations = Vec::new();
        for hunk in &hunks {
            let status = hunk.status;
            if hunk.changed_lines.is_empty() {
                decorations.push(GitLineDecoration {
                    line: hunk.new_start.saturating_sub(1) as usize,
                    status,
                    hunk_id: hunk.id.clone(),
                });
            } else {
                decorations.extend(hunk.changed_lines.iter().copied().map(|line| {
                    GitLineDecoration {
                        line,
                        status,
                        hunk_id: hunk.id.clone(),
                    }
                }));
            }
            inline_decorations.extend(hunk.inline_decorations.iter().cloned());
        }
        Self {
            path,
            revision,
            state: if hunks.is_empty() {
                GitGutterState::Empty
            } else {
                GitGutterState::Ready
            },
            staged: false,
            hunks,
            decorations,
            inline_decorations,
        }
    }

    pub fn is_current(&self, revision: Revision) -> bool {
        self.revision == revision
    }

    /// Return only the decorations safe to use for this buffer revision.
    pub fn decorations_for_revision(&self, revision: Revision) -> &[GitLineDecoration] {
        if self.is_current(revision) {
            self.decorations.as_slice()
        } else {
            &[]
        }
    }

    pub fn inline_decorations_for_revision(&self, revision: Revision) -> &[GitInlineDecoration] {
        if self.is_current(revision) {
            self.inline_decorations.as_slice()
        } else {
            &[]
        }
    }

    pub fn hunk(&self, id: &str, revision: Revision) -> Result<&GitHunk, GitGutterError> {
        if !self.is_current(revision) {
            return Err(GitGutterError::StaleRevision {
                expected: revision,
                actual: self.revision,
            });
        }
        self.hunks
            .iter()
            .find(|hunk| hunk.id == id)
            .ok_or_else(|| GitGutterError::HunkNotFound(id.to_string()))
    }

    /// Navigate changed lines with wrap-around, like a normal editor change
    /// navigator. Adjacent lines in one hunk are collapsed to one target.
    pub fn next_change_line(
        &self,
        current_line: usize,
        forward: bool,
        revision: Revision,
    ) -> Option<usize> {
        if !self.is_current(revision) {
            return None;
        }
        let mut lines = self
            .decorations
            .iter()
            .map(|decoration| decoration.line)
            .collect::<Vec<_>>();
        lines.sort_unstable();
        lines.dedup();
        if forward {
            lines
                .iter()
                .copied()
                .find(|line| *line > current_line)
                .or_else(|| lines.first().copied())
        } else {
            lines
                .iter()
                .rev()
                .copied()
                .find(|line| *line < current_line)
                .or_else(|| lines.last().copied())
        }
    }

    pub fn hunk_at_line(&self, line: usize, revision: Revision) -> Option<&GitHunk> {
        self.is_current(revision).then(|| {
            self.decorations
                .iter()
                .find(|decoration| decoration.line == line)
                .and_then(|decoration| self.hunks.iter().find(|hunk| hunk.id == decoration.hunk_id))
        })?
    }

    pub fn action(
        &self,
        hunk_id: &str,
        revision: Revision,
        kind: GitHunkActionKind,
    ) -> Result<GitHunkAction, GitGutterError> {
        let hunk = self.hunk(hunk_id, revision)?;
        let target = hunk.target(&self.path, revision);
        Ok(match kind {
            GitHunkActionKind::Stage => GitHunkAction::Stage {
                target,
                patch: hunk.patch.clone(),
            },
            GitHunkActionKind::Unstage => GitHunkAction::Unstage {
                target,
                patch: hunk.patch.clone(),
            },
            GitHunkActionKind::Discard => GitHunkAction::Discard {
                preview: hunk.discard_preview(&self.path, revision),
            },
            GitHunkActionKind::OpenProjectDiff => GitHunkAction::OpenProjectDiff { target },
            GitHunkActionKind::ShowChange => GitHunkAction::ShowChange { target },
        })
    }

    /// Project a visible editor hunk onto E3's visual rows. A stale snapshot
    /// intentionally projects to nothing instead of painting misleading data.
    pub fn display_decorations(
        &self,
        buffer: &BufferSnapshot,
        display: &DisplaySnapshot,
    ) -> Vec<DisplayGitDecoration> {
        if !self.is_current(buffer.revision()) {
            return Vec::new();
        }
        self.decorations_for_revision(buffer.revision())
            .iter()
            .filter_map(|decoration| {
                display
                    .position_to_display(Position::new(decoration.line, 0))
                    .map(|(visual_row, _)| DisplayGitDecoration {
                        visual_row,
                        line: decoration.line,
                        status: decoration.status,
                        hunk_id: decoration.hunk_id.clone(),
                    })
            })
            .collect()
    }

    /// Project intra-line decorations onto visible display rows. A stale
    /// snapshot intentionally paints nothing, and wrapped lines are split into
    /// row-local ranges rather than drawing across a visual-row boundary.
    pub fn display_inline_decorations(
        &self,
        buffer: &BufferSnapshot,
        display: &DisplaySnapshot,
    ) -> Vec<DisplayGitInlineDecoration> {
        if !self.is_current(buffer.revision()) {
            return Vec::new();
        }
        self.inline_decorations_for_revision(buffer.revision())
            .iter()
            .flat_map(|decoration| {
                display.rows.iter().filter_map(move |row| {
                    let crate::display::DisplayRowKind::Text {
                        logical_line,
                        start_column,
                        end_column,
                    } = row.kind
                    else {
                        return None;
                    };
                    if logical_line != decoration.line {
                        return None;
                    }
                    let start = decoration.start_column.max(start_column);
                    let end = decoration.end_column.min(end_column);
                    if start < end {
                        return Some(DisplayGitInlineDecoration {
                            visual_row: row.visual_row,
                            start_column: start
                                .saturating_sub(start_column)
                                .saturating_sub(display.viewport.left_column),
                            end_column: end
                                .saturating_sub(start_column)
                                .saturating_sub(display.viewport.left_column),
                            kind: decoration.kind,
                            source_text: decoration.source_text.clone(),
                            hunk_id: decoration.hunk_id.clone(),
                        });
                    }
                    if decoration.start_column == decoration.end_column
                        && decoration.start_column >= start_column
                        && decoration.start_column <= end_column
                    {
                        let column = decoration
                            .start_column
                            .saturating_sub(start_column)
                            .saturating_sub(display.viewport.left_column);
                        return Some(DisplayGitInlineDecoration {
                            visual_row: row.visual_row,
                            start_column: column,
                            end_column: column,
                            kind: decoration.kind,
                            source_text: decoration.source_text.clone(),
                            hunk_id: decoration.hunk_id.clone(),
                        });
                    }
                    None
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitHunkActionKind {
    Stage,
    Unstage,
    Discard,
    OpenProjectDiff,
    ShowChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayGitDecoration {
    pub visual_row: usize,
    pub line: usize,
    pub status: GitHunkStatus,
    pub hunk_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayGitInlineDecoration {
    pub visual_row: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub kind: GitInlineChangeKind,
    pub source_text: String,
    pub hunk_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitGutterError {
    EmptyDiff,
    InvalidDiff(String),
    FileNotFound(String),
    HunkNotFound(String),
    StaleRevision {
        expected: Revision,
        actual: Revision,
    },
    Provider(String),
    InvalidActionState(GitGutterState),
}

impl std::fmt::Display for GitGutterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDiff => write!(f, "Git returned no diff"),
            Self::InvalidDiff(message) => write!(f, "invalid Git diff: {message}"),
            Self::FileNotFound(path) => write!(f, "Git diff does not contain {path}"),
            Self::HunkNotFound(id) => write!(f, "Git hunk {id} does not exist"),
            Self::StaleRevision { expected, actual } => write!(
                f,
                "Git hunk targets revision {}, but the buffer is at {}",
                expected.value(),
                actual.value()
            ),
            Self::Provider(message) => write!(f, "Git provider failed: {message}"),
            Self::InvalidActionState(state) => {
                write!(f, "Git hunk action unavailable in {state:?}")
            }
        }
    }
}

impl std::error::Error for GitGutterError {}

/// Input for an asynchronous decoration refresh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitDecorationRequest {
    pub path: String,
    pub revision: Revision,
    pub session_id: Option<String>,
}

pub type GitProviderFuture<T> = Pin<Box<dyn Future<Output = Result<T, GitGutterError>> + Send>>;

/// Read-only Git provider boundary. Implementations belong to the Git/workspace
/// composition layer and must not block GPUI while obtaining a snapshot.
pub trait GitDecorationProvider: Send + Sync {
    fn refresh(&self, request: GitDecorationRequest) -> GitProviderFuture<GitGutterSnapshot>;
}

/// Mutation boundary for stage/unstage/discard/review intents.
pub trait HunkActionSink: Send + Sync {
    fn submit(&self, action: GitHunkAction) -> GitProviderFuture<()>;
}

fn same_path(left: &str, right: &str) -> bool {
    let left = left.strip_prefix("./").unwrap_or(left);
    let right = right.strip_prefix("./").unwrap_or(right);
    left == right
        || std::path::Path::new(left).file_name() == std::path::Path::new(right).file_name()
}

fn make_hunk(file: &FileDiff, hunk: &DiffHunk, index: usize) -> GitHunk {
    let status = if file.is_new_file {
        GitHunkStatus::Untracked
    } else if hunk.old_lines == 0 {
        GitHunkStatus::Added
    } else if hunk.new_lines == 0 {
        GitHunkStatus::Deleted
    } else {
        GitHunkStatus::Modified
    };
    let mut new_line = hunk.new_start.saturating_sub(1) as usize;
    let mut changed_lines = Vec::new();
    for line in &hunk.lines {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        match kind {
            '+' if !line.starts_with("+++") => {
                changed_lines.push(new_line);
                new_line = new_line.saturating_add(1);
            }
            ' ' => new_line = new_line.saturating_add(1),
            '-' | '\\' => {}
            _ => {}
        }
    }
    let id = format!("{}:{index}:{}", file.path, hunk.new_start);
    let inline_decorations = inline_decorations_for_hunk(hunk, &id);
    GitHunk {
        id,
        status,
        header: hunk.header.clone(),
        old_start: hunk.old_start,
        old_lines: hunk.old_lines,
        new_start: hunk.new_start,
        new_lines: hunk.new_lines,
        changed_lines,
        patch: crate::unified::build_hunk_patch(file, hunk),
        inline_decorations,
    }
}

/// The result of aligning one old/new line pair in Unicode scalar columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineDiff {
    pub kind: GitInlineChangeKind,
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
    pub old_text: String,
    pub new_text: String,
}

/// Find a unique contiguous change while retaining the largest equal prefix
/// and suffix. Multiple equally-good alignments are rejected deliberately:
/// repeated characters are not worth painting a potentially misleading range.
pub fn inline_diff(old: &str, new: &str) -> Option<InlineDiff> {
    let old_chars = old.chars().collect::<Vec<_>>();
    let new_chars = new.chars().collect::<Vec<_>>();
    let mut best_score = 0usize;
    let mut candidates = Vec::new();
    for prefix in 0..=old_chars.len().min(new_chars.len()) {
        if old_chars[..prefix] != new_chars[..prefix] {
            continue;
        }
        for suffix in 0..=old_chars.len().min(new_chars.len()) - prefix {
            if suffix > old_chars.len() - prefix || suffix > new_chars.len() - prefix {
                continue;
            }
            if old_chars[old_chars.len() - suffix..] != new_chars[new_chars.len() - suffix..] {
                continue;
            }
            let score = prefix + suffix;
            if score > best_score {
                best_score = score;
                candidates.clear();
            }
            if score == best_score {
                candidates.push((prefix, suffix));
            }
        }
    }
    if candidates.len() != 1 {
        return None;
    }
    let (prefix, suffix) = candidates[0];
    let old_end = old_chars.len() - suffix;
    let new_end = new_chars.len() - suffix;
    if prefix == old_end && prefix == new_end {
        return None;
    }
    let kind = match (old_end == prefix, new_end == prefix) {
        (true, false) => GitInlineChangeKind::Inserted,
        (false, true) => GitInlineChangeKind::Deleted,
        (false, false) => GitInlineChangeKind::Replaced,
        (true, true) => return None,
    };
    Some(InlineDiff {
        kind,
        old_start: prefix,
        old_end,
        new_start: prefix,
        new_end,
        old_text: old_chars[prefix..old_end].iter().collect(),
        new_text: new_chars[prefix..new_end].iter().collect(),
    })
}

fn inline_decorations_for_hunk(hunk: &DiffHunk, hunk_id: &str) -> Vec<GitInlineDecoration> {
    let mut result = Vec::new();
    let mut new_line = hunk.new_start.saturating_sub(1) as usize;
    let mut index = 0;
    while index < hunk.lines.len() {
        if !hunk.lines[index].starts_with('-') || hunk.lines[index].starts_with("---") {
            if (hunk.lines[index].starts_with('+') && !hunk.lines[index].starts_with("+++"))
                || hunk.lines[index].starts_with(' ')
            {
                new_line += 1;
            }
            index += 1;
            continue;
        }
        let delete_start = index;
        while index < hunk.lines.len()
            && hunk.lines[index].starts_with('-')
            && !hunk.lines[index].starts_with("---")
        {
            index += 1;
        }
        let delete_end = index;
        let add_start = index;
        while index < hunk.lines.len()
            && hunk.lines[index].starts_with('+')
            && !hunk.lines[index].starts_with("+++")
        {
            index += 1;
        }
        let add_end = index;
        let delete_count = delete_end - delete_start;
        let add_count = add_end - add_start;
        if add_count > 0 && delete_count == add_count {
            let mut run_decorations = Vec::with_capacity(add_count);
            let mut ambiguous = false;
            for offset in 0..add_count {
                let old = diff_content(&hunk.lines[delete_start + offset]);
                let new = diff_content(&hunk.lines[add_start + offset]);
                let Some(alignment) = inline_diff(old, new) else {
                    ambiguous = true;
                    continue;
                };
                run_decorations.push(GitInlineDecoration {
                    line: new_line + offset,
                    start_column: alignment.new_start,
                    end_column: alignment.new_end,
                    kind: alignment.kind,
                    source_text: alignment.new_text,
                    hunk_id: hunk_id.to_string(),
                });
            }
            new_line += add_count;
            if !ambiguous {
                result.extend(run_decorations);
            }
        } else {
            new_line += add_count;
        }
    }
    result
}

fn diff_content(line: &str) -> &str {
    line.get(1..)
        .unwrap_or_default()
        .strip_suffix('\r')
        .unwrap_or_else(|| line.get(1..).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DisplayConfig, DisplayMap, EditorBuffer, Viewport};

    fn diff() -> String {
        [
            "diff --git a/src/lib.rs b/src/lib.rs",
            "index 1111111..2222222 100644",
            "--- a/src/lib.rs",
            "+++ b/src/lib.rs",
            "@@ -1,2 +1,3 @@",
            " fn main() {}",
            "+added();",
            " second();",
            "@@ -7,1 +7,0 @@",
            " old();",
            "-removed();",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn parses_hunks_and_projects_added_modified_and_deleted_lines() {
        let snapshot =
            GitGutterSnapshot::from_unified_diff("src/lib.rs", Revision::default(), &diff())
                .expect("fixture parses");
        assert_eq!(snapshot.state, GitGutterState::Ready);
        assert_eq!(snapshot.hunks.len(), 2);
        assert_eq!(snapshot.hunks[0].status, GitHunkStatus::Modified);
        assert_eq!(snapshot.hunks[1].status, GitHunkStatus::Deleted);
        assert_eq!(snapshot.decorations.len(), 2);
        assert_eq!(snapshot.decorations[0].line, 1);
        assert_eq!(snapshot.decorations[1].line, 6);
        assert!(snapshot.hunks[0].patch.contains("+added();"));
    }

    #[test]
    fn maps_unicode_crlf_buffer_lines_without_byte_math() {
        let buffer = EditorBuffer::from_text("α\r\nβ\r\nγ\r\n");
        let display = DisplayMap::build(
            &buffer.snapshot(),
            Viewport {
                height_rows: 8,
                ..Default::default()
            },
            DisplayConfig::default(),
            &[],
        );
        let file = parse_diff_hunks(
            "diff --git a/u.txt b/u.txt\n--- a/u.txt\n+++ b/u.txt\n@@ -1,3 +1,3 @@\n α\r\n-β\r\n+δ\r\n γ\r\n",
        )[0]
            .clone();
        let snapshot =
            GitGutterSnapshot::from_file_diff("u.txt", buffer.snapshot().revision(), &file);
        let projected = snapshot.display_decorations(&buffer.snapshot(), &display);
        assert_eq!(projected[0].line, 1);
        assert_eq!(projected[0].visual_row, 1);
    }

    #[test]
    fn change_navigation_wraps_and_collapses_duplicate_lines() {
        let snapshot =
            GitGutterSnapshot::from_unified_diff("src/lib.rs", Revision::default(), &diff())
                .expect("fixture parses");
        assert_eq!(
            snapshot.next_change_line(0, true, Revision::default()),
            Some(1)
        );
        assert_eq!(
            snapshot.next_change_line(99, true, Revision::default()),
            Some(1)
        );
        assert_eq!(
            snapshot.next_change_line(0, false, Revision::default()),
            Some(6)
        );
    }

    #[test]
    fn stale_revision_rejects_projection_navigation_and_actions() {
        let snapshot =
            GitGutterSnapshot::from_unified_diff("src/lib.rs", Revision::default(), &diff())
                .expect("fixture parses");
        let newer = Revision::new(1);
        assert!(snapshot.decorations_for_revision(newer).is_empty());
        assert_eq!(snapshot.next_change_line(0, true, newer), None);
        assert!(matches!(
            snapshot.action(&snapshot.hunks[0].id, newer, GitHunkActionKind::Stage),
            Err(GitGutterError::StaleRevision { .. })
        ));
    }

    #[test]
    fn actions_carry_typed_targets_and_discard_preview() {
        let snapshot =
            GitGutterSnapshot::from_unified_diff("src/lib.rs", Revision::default(), &diff())
                .expect("fixture parses");
        let id = &snapshot.hunks[0].id;
        let stage = snapshot
            .action(id, Revision::default(), GitHunkActionKind::Stage)
            .expect("stage action");
        assert!(
            matches!(stage, GitHunkAction::Stage { ref target, ref patch } if target.path == "src/lib.rs" && patch.contains("+added();"))
        );
        let discard = snapshot
            .action(id, Revision::default(), GitHunkActionKind::Discard)
            .expect("discard action");
        assert!(
            matches!(discard, GitHunkAction::Discard { ref preview } if preview.summary.contains("lines") && preview.patch.contains("@@"))
        );
    }

    #[test]
    fn empty_and_unavailable_states_are_explicit() {
        let empty = GitGutterSnapshot::from_unified_diff("x.rs", Revision::default(), "")
            .expect_err("empty diff is not fake ready data");
        assert_eq!(empty, GitGutterError::EmptyDiff);
        assert_eq!(
            GitGutterSnapshot::no_repository("x.rs", Revision::default()).state,
            GitGutterState::NoRepository
        );
        assert_eq!(
            GitGutterSnapshot::loading("x.rs", Revision::default()).state,
            GitGutterState::Loading
        );
    }

    #[test]
    fn untracked_file_has_explicit_untracked_markers() {
        let file = parse_diff_hunks(
            "diff --git a/new.rs b/new.rs\nnew file mode 100644\n--- /dev/null\n+++ b/new.rs\n@@ -0,0 +1,2 @@\n+fn main() {}\n+α\n",
        )[0]
            .clone();
        let snapshot = GitGutterSnapshot::from_file_diff("new.rs", Revision::default(), &file);
        assert_eq!(snapshot.hunks[0].status, GitHunkStatus::Untracked);
        assert!(snapshot
            .decorations
            .iter()
            .all(|d| d.status == GitHunkStatus::Untracked));
    }

    #[test]
    fn inline_diff_preserves_equal_prefix_and_suffix() {
        let change = inline_diff("prefix old suffix", "prefix new suffix").expect("change");
        assert_eq!(change.kind, GitInlineChangeKind::Replaced);
        assert_eq!((change.new_start, change.new_end), (7, 10));
        assert_eq!(change.new_text, "new");
    }

    #[test]
    fn inline_diff_represents_insertions_and_deletions() {
        let inserted = inline_diff("left:right", "leftXYZ:right").expect("insertion");
        assert_eq!(inserted.kind, GitInlineChangeKind::Inserted);
        assert_eq!(inserted.new_text, "XYZ");
        let deleted = inline_diff("leftXYZ:right", "left:right").expect("deletion");
        assert_eq!(deleted.kind, GitInlineChangeKind::Deleted);
        assert_eq!(deleted.new_text, "");
        assert_eq!(deleted.new_start, deleted.new_end);
    }

    #[test]
    fn inline_diff_uses_unicode_scalar_columns() {
        let change = inline_diff("α旧🙂", "α新🙂").expect("unicode change");
        assert_eq!((change.new_start, change.new_end), (1, 2));
        assert_eq!(change.new_text, "新");
    }

    #[test]
    fn inline_diff_rejects_ambiguous_repeated_alignment() {
        assert!(inline_diff("a", "aa").is_none());
        assert!(inline_diff("aaaa", "aaa").is_none());
    }

    #[test]
    fn inline_decorations_pair_only_compatible_adjacent_runs() {
        let file = parse_diff_hunks(
            "diff --git a/x.rs b/x.rs\n--- a/x.rs\n+++ b/x.rs\n@@ -1,3 +1,3 @@\n-before old\n+before new\n context\n-a\n-b\n+x\n",
        )[0]
            .clone();
        let snapshot = GitGutterSnapshot::from_file_diff("x.rs", Revision::default(), &file);
        assert_eq!(snapshot.inline_decorations.len(), 1);
        assert_eq!(snapshot.inline_decorations[0].source_text, "new");
        assert_eq!(snapshot.inline_decorations[0].line, 0);
    }

    #[test]
    fn inline_display_projection_is_revision_safe_and_unicode_safe() {
        let buffer = EditorBuffer::from_text("α新🙂\n");
        let display = DisplayMap::build(
            &buffer.snapshot(),
            Viewport {
                height_rows: 8,
                ..Default::default()
            },
            DisplayConfig::default(),
            &[],
        );
        let mut snapshot = GitGutterSnapshot::empty("x.rs", buffer.snapshot().revision());
        snapshot.inline_decorations.push(GitInlineDecoration {
            line: 0,
            start_column: 1,
            end_column: 2,
            kind: GitInlineChangeKind::Replaced,
            source_text: "新".into(),
            hunk_id: "x:0:1".into(),
        });
        let projected = snapshot.display_inline_decorations(&buffer.snapshot(), &display);
        assert_eq!((projected[0].start_column, projected[0].end_column), (1, 2));
        assert!(snapshot
            .display_inline_decorations(&buffer.snapshot(), &display)
            .iter()
            .all(|decoration| decoration.visual_row == 0));
        let stale = GitGutterSnapshot::empty("x.rs", Revision::new(1));
        assert!(stale
            .display_inline_decorations(&buffer.snapshot(), &display)
            .is_empty());
    }
}
