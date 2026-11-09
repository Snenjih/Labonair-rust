//! Multi-file diff presentation model for the workspace DiffView
//! (`docs/parity/diff-view-spec.md` §2, §4, §6).
//!
//! Pure data: `git diff` patches go in, an ordered list of file sections with
//! typed lines, per-hunk stage state, statistics, split alignment and hunk
//! navigation come out. No GPUI, no IO — the view only paints what this model
//! derives.

use std::collections::HashSet;
use std::ops::Range;

use crate::unified::{build_hunk_patch, is_whole_file_single_hunk, DiffHunk, FileDiff};

/// Where a hunk sits relative to the index (spec §2 "Hunk stage state").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkStage {
    /// Worktree change not yet in the index.
    Unstaged,
    /// Change already recorded in the index.
    Staged,
    /// Committed / stashed / branch change: no staging applies.
    ReadOnly,
}

/// How a file changed, for header coloring and ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Modified,
    Added,
    Deleted,
    Untracked,
}

impl FileKind {
    /// Ordering group: tracked changes first, new files last (spec §2).
    fn sort_group(self) -> u8 {
        match self {
            FileKind::Modified | FileKind::Deleted => 2,
            FileKind::Added | FileKind::Untracked => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Deleted,
}

/// One rendered line of a hunk. `text` excludes the `+`/`-`/` ` prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub text: String,
    /// Byte range of the changed middle when a deleted/added pair was matched
    /// (word diff). `None` for context and unpaired lines.
    pub word: Option<Range<usize>>,
}

/// One hunk with its stage tag and precomputed counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkView {
    pub stage: HunkStage,
    pub header: String,
    pub lines: Vec<DiffLine>,
    pub added: u32,
    pub removed: u32,
    /// Index of the originating patch in [`FileSection::sources`] and of the
    /// hunk inside it — enough to rebuild a standalone patch.
    source: usize,
    source_hunk: usize,
}

/// Everything the document knows about one changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSection {
    pub path: String,
    pub kind: FileKind,
    pub hunks: Vec<HunkView>,
    /// Verbatim text for files without parsable hunks (binary, untracked
    /// fallback).
    pub raw: Vec<String>,
    pub added: u32,
    pub removed: u32,
    sources: Vec<(HunkStage, FileDiff)>,
}

/// Addresses one hunk in a [`DiffDocument`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HunkId {
    pub file: usize,
    pub hunk: usize,
}

/// A staging operation derived from a hunk (spec §8: hunk staging).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkOperation {
    /// Whole-file add/delete: plain `git add` / `git restore --staged`.
    WholeFile { path: String, stage: HunkStage },
    /// A standalone single-hunk patch for `git apply`.
    Patch {
        path: String,
        patch: String,
        stage: HunkStage,
    },
}

/// Input for one file: its path plus every patch that describes it.
#[derive(Debug, Clone)]
pub struct FileInput {
    pub path: String,
    pub untracked: bool,
    pub diffs: Vec<(HunkStage, FileDiff)>,
    /// Fallback text when no patch has hunks.
    pub raw: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStyle {
    Unified,
    Split,
}

/// One virtualized row of the flattened document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatRow {
    File(usize),
    HunkHeader(HunkId),
    Line {
        id: HunkId,
        line: usize,
    },
    Split {
        id: HunkId,
        left: Option<usize>,
        right: Option<usize>,
    },
    Raw {
        file: usize,
        line: usize,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffDocument {
    pub files: Vec<FileSection>,
}

impl DiffDocument {
    /// Builds the ordered document. Files with several patches (staged +
    /// unstaged) get one section whose hunks interleave by index position.
    pub fn build(inputs: Vec<FileInput>) -> Self {
        let mut files: Vec<FileSection> = inputs.into_iter().map(build_section).collect();
        files.sort_by(|a, b| (a.kind.sort_group(), &a.path).cmp(&(b.kind.sort_group(), &b.path)));
        Self { files }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// `(added, removed)` over the whole document.
    pub fn totals(&self) -> (u32, u32) {
        self.files
            .iter()
            .fold((0, 0), |(a, r), f| (a + f.added, r + f.removed))
    }

    pub fn file_index(&self, path: &str) -> Option<usize> {
        self.files.iter().position(|f| f.path == path)
    }

    pub fn hunk_ids(&self) -> Vec<HunkId> {
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, f)| (0..f.hunks.len()).map(move |hunk| HunkId { file, hunk }))
            .collect()
    }

    pub fn hunk(&self, id: HunkId) -> Option<&HunkView> {
        self.files.get(id.file)?.hunks.get(id.hunk)
    }

    /// Next/previous hunk after/before `from`, wrapping across files and the
    /// document ends. `None` only when the document has no hunks.
    pub fn step_hunk(&self, from: Option<HunkId>, forward: bool) -> Option<HunkId> {
        let ids = self.hunk_ids();
        if ids.is_empty() {
            return None;
        }
        let Some(from) = from else {
            return Some(if forward { ids[0] } else { ids[ids.len() - 1] });
        };
        let pos = ids.iter().position(|id| *id >= from);
        let next = match (forward, pos) {
            (true, Some(p)) if ids[p] == from => (p + 1) % ids.len(),
            (true, Some(p)) => p,
            (true, None) => 0,
            (false, Some(0)) | (false, None) => ids.len() - 1,
            (false, Some(p)) => p - 1,
        };
        Some(ids[next])
    }

    pub fn has_stage(&self, stage: HunkStage) -> bool {
        self.files
            .iter()
            .any(|f| f.hunks.iter().any(|h| h.stage == stage))
    }

    /// Flattens the document into virtual rows. Folded files keep only their
    /// header row.
    pub fn flatten(&self, style: DiffStyle, folded: &HashSet<String>) -> Vec<FlatRow> {
        let mut rows = Vec::new();
        for (file, section) in self.files.iter().enumerate() {
            rows.push(FlatRow::File(file));
            if folded.contains(&section.path) {
                continue;
            }
            for (hunk, view) in section.hunks.iter().enumerate() {
                let id = HunkId { file, hunk };
                rows.push(FlatRow::HunkHeader(id));
                match style {
                    DiffStyle::Unified => {
                        rows.extend((0..view.lines.len()).map(|line| FlatRow::Line { id, line }))
                    }
                    DiffStyle::Split => rows.extend(
                        split_rows(&view.lines)
                            .into_iter()
                            .map(|(left, right)| FlatRow::Split { id, left, right }),
                    ),
                }
            }
            rows.extend((0..section.raw.len()).map(|line| FlatRow::Raw { file, line }));
        }
        rows
    }

    /// The staging operation for one hunk, or `None` for read-only hunks.
    pub fn hunk_operation(&self, id: HunkId) -> Option<HunkOperation> {
        let section = self.files.get(id.file)?;
        let view = section.hunks.get(id.hunk)?;
        if view.stage == HunkStage::ReadOnly {
            return None;
        }
        let (stage, source) = section.sources.get(view.source)?;
        if is_whole_file_single_hunk(source) {
            return Some(HunkOperation::WholeFile {
                path: section.path.clone(),
                stage: *stage,
            });
        }
        let hunk = source.hunks.get(view.source_hunk)?;
        Some(HunkOperation::Patch {
            path: section.path.clone(),
            patch: build_hunk_patch(source, hunk),
            stage: *stage,
        })
    }
}

fn build_section(input: FileInput) -> FileSection {
    let FileInput {
        path,
        untracked,
        diffs,
        raw,
    } = input;

    let mut hunks: Vec<(u32, HunkView)> = Vec::new();
    let mut is_new = untracked;
    let mut is_deleted = false;
    for (source, (stage, diff)) in diffs.iter().enumerate() {
        is_new |= diff.is_new_file;
        is_deleted |= diff.is_deleted_file;
        for (source_hunk, hunk) in diff.hunks.iter().enumerate() {
            let view = hunk_view(*stage, hunk, source, source_hunk);
            // Staged hunks live in index coordinates on their new side,
            // unstaged hunks on their old side — a shared ordering axis.
            let position = match stage {
                HunkStage::Staged => hunk.new_start,
                _ => hunk.old_start,
            };
            hunks.push((position, view));
        }
    }
    hunks.sort_by_key(|(position, view)| (*position, view.stage != HunkStage::Staged));
    let hunks: Vec<HunkView> = hunks.into_iter().map(|(_, h)| h).collect();

    let kind = if untracked {
        FileKind::Untracked
    } else if is_deleted {
        FileKind::Deleted
    } else if is_new {
        FileKind::Added
    } else {
        FileKind::Modified
    };

    let (mut added, mut removed) = hunks
        .iter()
        .fold((0, 0), |(a, r), h| (a + h.added, r + h.removed));
    if hunks.is_empty() && kind == FileKind::Untracked {
        added = raw.iter().filter(|l| l.starts_with('+')).count() as u32;
        removed = 0;
    }

    FileSection {
        path,
        kind,
        hunks,
        raw,
        added,
        removed,
        sources: diffs,
    }
}

fn hunk_view(stage: HunkStage, hunk: &DiffHunk, source: usize, source_hunk: usize) -> HunkView {
    let mut lines: Vec<DiffLine> = Vec::new();
    let (mut old_no, mut new_no) = (hunk.old_start, hunk.new_start);
    let (mut added, mut removed) = (0, 0);
    for raw in &hunk.lines {
        match raw.as_bytes().first() {
            Some(b'+') => {
                lines.push(DiffLine {
                    kind: LineKind::Added,
                    old_no: None,
                    new_no: Some(new_no),
                    text: raw[1..].to_string(),
                    word: None,
                });
                new_no += 1;
                added += 1;
            }
            Some(b'-') => {
                lines.push(DiffLine {
                    kind: LineKind::Deleted,
                    old_no: Some(old_no),
                    new_no: None,
                    text: raw[1..].to_string(),
                    word: None,
                });
                old_no += 1;
                removed += 1;
            }
            Some(b'\\') => {}
            _ => {
                lines.push(DiffLine {
                    kind: LineKind::Context,
                    old_no: Some(old_no),
                    new_no: Some(new_no),
                    text: raw.strip_prefix(' ').unwrap_or(raw).to_string(),
                    word: None,
                });
                old_no += 1;
                new_no += 1;
            }
        }
    }
    pair_word_diffs(&mut lines);
    HunkView {
        stage,
        header: hunk.header.clone(),
        lines,
        added,
        removed,
        source,
        source_hunk,
    }
}

/// Pairs each run of deleted lines with the added lines that follow it,
/// index-wise, and records the changed middle on both.
fn pair_word_diffs(lines: &mut [DiffLine]) {
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != LineKind::Deleted {
            i += 1;
            continue;
        }
        let del_start = i;
        while i < lines.len() && lines[i].kind == LineKind::Deleted {
            i += 1;
        }
        let add_start = i;
        while i < lines.len() && lines[i].kind == LineKind::Added {
            i += 1;
        }
        let pairs = (add_start - del_start).min(i - add_start);
        for k in 0..pairs {
            let ranges = word_diff_ranges(&lines[del_start + k].text, &lines[add_start + k].text);
            if let Some((before, after)) = ranges {
                lines[del_start + k].word = Some(before);
                lines[add_start + k].word = Some(after);
            }
        }
    }
}

/// Side-by-side alignment of one hunk: each run of deletions/additions is
/// zipped, the shorter side padded with `None` filler (spec §6).
pub fn split_rows(lines: &[DiffLine]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut rows = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        match lines[i].kind {
            LineKind::Context => {
                rows.push((Some(i), Some(i)));
                i += 1;
            }
            _ => {
                let mut dels = Vec::new();
                let mut adds = Vec::new();
                while i < lines.len() && lines[i].kind != LineKind::Context {
                    match lines[i].kind {
                        LineKind::Deleted => dels.push(i),
                        _ => adds.push(i),
                    }
                    i += 1;
                }
                for k in 0..dels.len().max(adds.len()) {
                    rows.push((dels.get(k).copied(), adds.get(k).copied()));
                }
            }
        }
    }
    rows
}

/// Byte ranges of the changed middle after preserving the longest common
/// prefix and suffix; `None` when the lines are equal.
pub fn word_diff_ranges(before: &str, after: &str) -> Option<(Range<usize>, Range<usize>)> {
    let prefix_chars = before
        .chars()
        .zip(after.chars())
        .take_while(|(l, r)| l == r)
        .count();
    let prefix_bytes = before
        .char_indices()
        .nth(prefix_chars)
        .map_or(before.len(), |(index, _)| index);
    if prefix_bytes == before.len() && prefix_bytes == after.len() {
        return None;
    }
    let suffix_chars = before[prefix_bytes..]
        .chars()
        .rev()
        .zip(after[prefix_bytes..].chars().rev())
        .take_while(|(l, r)| l == r)
        .count();
    let suffix_bytes = |text: &str| {
        text.chars()
            .rev()
            .take(suffix_chars)
            .map(char::len_utf8)
            .sum::<usize>()
    };
    Some((
        prefix_bytes..before.len() - suffix_bytes(before),
        prefix_bytes..after.len() - suffix_bytes(after),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unified::parse_diff_hunks;

    fn patch(lines: &[&str]) -> Vec<FileDiff> {
        parse_diff_hunks(&(lines.join("\n") + "\n"))
    }

    fn modified(path: &str, body: &[&str]) -> Vec<FileDiff> {
        let mut lines = vec![
            format!("diff --git a/{path} b/{path}"),
            "index 111..222 100644".to_string(),
            format!("--- a/{path}"),
            format!("+++ b/{path}"),
        ];
        lines.extend(body.iter().map(|s| s.to_string()));
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        patch(&refs)
    }

    fn input(path: &str, diffs: Vec<(HunkStage, FileDiff)>) -> FileInput {
        FileInput {
            path: path.into(),
            untracked: false,
            diffs,
            raw: Vec::new(),
        }
    }

    fn one(path: &str, stage: HunkStage, body: &[&str]) -> FileInput {
        let diff = modified(path, body).remove(0);
        input(path, vec![(stage, diff)])
    }

    #[test]
    fn lines_carry_numbers_and_counts() {
        let doc = DiffDocument::build(vec![one(
            "a.rs",
            HunkStage::Unstaged,
            &["@@ -10,3 +10,3 @@", " keep", "-old", "+new", " tail"],
        )]);
        let hunk = &doc.files[0].hunks[0];
        assert_eq!((hunk.added, hunk.removed), (1, 1));
        assert_eq!(hunk.lines[0].old_no, Some(10));
        assert_eq!(hunk.lines[1].old_no, Some(11));
        assert_eq!(hunk.lines[1].new_no, None);
        assert_eq!(hunk.lines[2].new_no, Some(11));
        assert_eq!(hunk.lines[3].old_no, Some(12));
        assert_eq!(hunk.lines[3].new_no, Some(12));
        assert_eq!(doc.totals(), (1, 1));
    }

    #[test]
    fn replacement_pairs_get_word_ranges() {
        let doc = DiffDocument::build(vec![one(
            "a.rs",
            HunkStage::Unstaged,
            &["@@ -1 +1 @@", "-let count = 1;", "+let count = 42;"],
        )]);
        let lines = &doc.files[0].hunks[0].lines;
        assert_eq!(&lines[0].text[lines[0].word.clone().unwrap()], "1");
        assert_eq!(&lines[1].text[lines[1].word.clone().unwrap()], "42");
        assert_eq!(word_diff_ranges("same", "same"), None);
        let (before, after) = word_diff_ranges("α = old;", "α = new;").unwrap();
        assert_eq!(&"α = old;"[before], "old");
        assert_eq!(&"α = new;"[after], "new");
    }

    #[test]
    fn staged_and_unstaged_hunks_interleave_by_index_position() {
        let staged = modified("a.rs", &["@@ -20,1 +20,1 @@", "-x", "+y"]).remove(0);
        let unstaged = modified("a.rs", &["@@ -3,1 +3,1 @@", "-p", "+q"]).remove(0);
        let doc = DiffDocument::build(vec![input(
            "a.rs",
            vec![(HunkStage::Staged, staged), (HunkStage::Unstaged, unstaged)],
        )]);
        let stages: Vec<_> = doc.files[0].hunks.iter().map(|h| h.stage).collect();
        assert_eq!(stages, [HunkStage::Unstaged, HunkStage::Staged]);
        assert!(doc.has_stage(HunkStage::Staged));
    }

    #[test]
    fn files_sort_tracked_before_new_then_by_path() {
        let mut new_file = one("a_new.rs", HunkStage::Unstaged, &["@@ -0,0 +1 @@", "+x"]);
        new_file.untracked = true;
        let doc = DiffDocument::build(vec![
            new_file,
            one("z.rs", HunkStage::Unstaged, &["@@ -1 +1 @@", "-a", "+b"]),
            one("b.rs", HunkStage::Unstaged, &["@@ -1 +1 @@", "-a", "+b"]),
        ]);
        let order: Vec<_> = doc.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(order, ["b.rs", "z.rs", "a_new.rs"]);
        assert_eq!(doc.files[2].kind, FileKind::Untracked);
    }

    #[test]
    fn split_rows_pad_the_shorter_side() {
        let doc = DiffDocument::build(vec![one(
            "a.rs",
            HunkStage::Unstaged,
            &["@@ -1,3 +1,4 @@", " c", "-a", "+b", "+extra", " d"],
        )]);
        let rows = split_rows(&doc.files[0].hunks[0].lines);
        assert_eq!(
            rows,
            [
                (Some(0), Some(0)),
                (Some(1), Some(2)),
                (None, Some(3)),
                (Some(4), Some(4)),
            ]
        );
    }

    #[test]
    fn flatten_keeps_only_the_header_of_folded_files() {
        let doc = DiffDocument::build(vec![
            one("a.rs", HunkStage::Unstaged, &["@@ -1 +1 @@", "-a", "+b"]),
            one("b.rs", HunkStage::Unstaged, &["@@ -1 +1 @@", "-a", "+b"]),
        ]);
        let all = doc.flatten(DiffStyle::Unified, &HashSet::new());
        assert_eq!(all.len(), 2 * (1 + 1 + 2));
        let folded = doc.flatten(DiffStyle::Unified, &HashSet::from(["a.rs".to_string()]));
        assert_eq!(folded.len(), 1 + 4);
        assert_eq!(folded[0], FlatRow::File(0));
    }

    #[test]
    fn hunk_navigation_wraps_across_files() {
        let doc = DiffDocument::build(vec![
            one("a.rs", HunkStage::Unstaged, &["@@ -1 +1 @@", "-a", "+b"]),
            one(
                "b.rs",
                HunkStage::Unstaged,
                &["@@ -1 +1 @@", "-a", "+b", "@@ -9 +9 @@", "-c", "+d"],
            ),
        ]);
        let first = HunkId { file: 0, hunk: 0 };
        let last = HunkId { file: 1, hunk: 1 };
        assert_eq!(doc.step_hunk(None, true), Some(first));
        assert_eq!(
            doc.step_hunk(Some(first), true),
            Some(HunkId { file: 1, hunk: 0 })
        );
        assert_eq!(doc.step_hunk(Some(last), true), Some(first));
        assert_eq!(doc.step_hunk(Some(first), false), Some(last));
        assert_eq!(DiffDocument::default().step_hunk(None, true), None);
    }

    #[test]
    fn hunk_operation_picks_patch_or_whole_file() {
        let doc = DiffDocument::build(vec![
            one("a.rs", HunkStage::Staged, &["@@ -1 +1 @@", "-a", "+b"]),
            FileInput {
                path: "n.rs".into(),
                untracked: false,
                diffs: vec![(
                    HunkStage::Unstaged,
                    patch(&[
                        "diff --git a/n.rs b/n.rs",
                        "new file mode 100644",
                        "--- /dev/null",
                        "+++ b/n.rs",
                        "@@ -0,0 +1 @@",
                        "+x",
                    ])
                    .remove(0),
                )],
                raw: Vec::new(),
            },
        ]);
        assert!(matches!(
            doc.hunk_operation(HunkId { file: 0, hunk: 0 }),
            Some(HunkOperation::Patch {
                stage: HunkStage::Staged,
                ..
            })
        ));
        assert!(matches!(
            doc.hunk_operation(HunkId { file: 1, hunk: 0 }),
            Some(HunkOperation::WholeFile {
                stage: HunkStage::Unstaged,
                ..
            })
        ));
        let read_only = DiffDocument::build(vec![one(
            "r.rs",
            HunkStage::ReadOnly,
            &["@@ -1 +1 @@", "-a", "+b"],
        )]);
        assert_eq!(read_only.hunk_operation(HunkId { file: 0, hunk: 0 }), None);
    }
}
