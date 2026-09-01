//! Text diffing (T06-004).
//!
//! A line-based **Myers** diff (`myers` — the classic O(ND) shortest-edit-
//! script algorithm, same family Git uses) that turns two strings into a
//! manipulation-friendly [`DiffLine`] list plus context-grouped [`Hunk`]s,
//! with [`side_by_side`] projecting a hunk into paired left/right rows.
//!
//! The input is always *two text contents* and the output is a plain data
//! structure (line list with per-line status + numbers) — deliberately not
//! the textual `git diff` format, so callers can render or mutate it freely.
//! This is the shared core the Git UI (Phase 8) and the AI diff (Phase 10)
//! build their diff panes on.

/// Per-line change classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeTag {
    /// Present unchanged in both versions.
    Equal,
    /// Present only in the old version (removed).
    Delete,
    /// Present only in the new version (added).
    Insert,
}

/// One line of a computed diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub tag: ChangeTag,
    /// 1-based line number in the old text (`None` for inserted lines).
    pub old_line: Option<usize>,
    /// 1-based line number in the new text (`None` for deleted lines).
    pub new_line: Option<usize>,
    /// The line content, without its trailing newline.
    pub text: String,
}

/// A contiguous changed region plus up to `context` surrounding equal lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// 1-based first old line covered (`0` when the old side is empty).
    pub old_start: usize,
    pub old_len: usize,
    /// 1-based first new line covered (`0` when the new side is empty).
    pub new_start: usize,
    pub new_len: usize,
    /// Every line of the hunk, in order (context + deletions + insertions).
    pub lines: Vec<DiffLine>,
}

impl Hunk {
    /// The `@@ -a,b +c,d @@` unified-diff header for this hunk.
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_len, self.new_start, self.new_len
        )
    }

    /// Number of inserted and deleted lines in this hunk.
    pub fn change_counts(&self) -> (usize, usize) {
        let ins = self
            .lines
            .iter()
            .filter(|l| l.tag == ChangeTag::Insert)
            .count();
        let del = self
            .lines
            .iter()
            .filter(|l| l.tag == ChangeTag::Delete)
            .count();
        (ins, del)
    }
}

/// A computed diff between two text contents.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Diff {
    /// Full line-by-line diff of the whole input, every line in order.
    pub lines: Vec<DiffLine>,
    /// Changed regions, each with `context` lines of surrounding context.
    /// Empty when the two inputs are identical.
    pub hunks: Vec<Hunk>,
}

impl Diff {
    /// Default number of unchanged context lines kept around each change.
    pub const DEFAULT_CONTEXT: usize = 3;

    /// Diff `old` against `new` with [`Self::DEFAULT_CONTEXT`] context lines.
    pub fn compute(old: &str, new: &str) -> Self {
        Self::compute_with_context(old, new, Self::DEFAULT_CONTEXT)
    }

    /// Diff `old` against `new`, keeping `context` unchanged lines around
    /// each change; hunks closer than `2 * context` lines apart are merged.
    pub fn compute_with_context(old: &str, new: &str, context: usize) -> Self {
        let old_lines = split_lines(old);
        let new_lines = split_lines(new);
        let ops = myers(&old_lines, &new_lines);
        let lines = build_lines(&old_lines, &new_lines, &ops);
        let hunks = build_hunks(&lines, context);
        Self { lines, hunks }
    }

    /// `true` when the two inputs are identical (no hunks).
    pub fn is_unchanged(&self) -> bool {
        self.hunks.is_empty()
    }

    /// `(insertions, deletions)` across the whole diff.
    pub fn stats(&self) -> (usize, usize) {
        let ins = self
            .lines
            .iter()
            .filter(|l| l.tag == ChangeTag::Insert)
            .count();
        let del = self
            .lines
            .iter()
            .filter(|l| l.tag == ChangeTag::Delete)
            .count();
        (ins, del)
    }
}

/// Splits text into lines, dropping the trailing empty element a final
/// newline would produce (`"a\nb\n"` → `["a", "b"]`, `"a\n\n"` → `["a", ""]`).
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// One edit-script step.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Equal,
    Delete,
    Insert,
}

/// Myers O(ND) diff: the shortest edit script turning `a` into `b`.
fn myers(a: &[&str], b: &[&str]) -> Vec<Op> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    if n == 0 && m == 0 {
        return Vec::new();
    }
    let max = n + m;
    let offset = max;
    let mut v = vec![0isize; (2 * max + 1) as usize];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut d_final = 0isize;

    'search: for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let mut x = if k == -d
                || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
            {
                v[(k + 1 + offset) as usize]
            } else {
                v[(k - 1 + offset) as usize] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[(k + offset) as usize] = x;
            if x >= n && y >= m {
                d_final = d;
                break 'search;
            }
            k += 2;
        }
    }

    // Backtrack through the trace to reconstruct the ops.
    let mut ops = Vec::new();
    let mut x = n;
    let mut y = m;
    for d in (0..=d_final).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let prev_k =
            if k == -d || (k != d && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize]) {
                k + 1
            } else {
                k - 1
            };
        let prev_x = v[(prev_k + offset) as usize];
        let prev_y = prev_x - prev_k;
        while x > prev_x && y > prev_y {
            ops.push(Op::Equal);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            if x == prev_x {
                ops.push(Op::Insert);
                y -= 1;
            } else {
                ops.push(Op::Delete);
                x -= 1;
            }
        }
    }
    ops.reverse();
    ops
}

fn build_lines(a: &[&str], b: &[&str], ops: &[Op]) -> Vec<DiffLine> {
    let mut out = Vec::with_capacity(ops.len());
    let mut oi = 0usize;
    let mut ni = 0usize;
    for op in ops {
        match op {
            Op::Equal => {
                out.push(DiffLine {
                    tag: ChangeTag::Equal,
                    old_line: Some(oi + 1),
                    new_line: Some(ni + 1),
                    text: a[oi].to_string(),
                });
                oi += 1;
                ni += 1;
            }
            Op::Delete => {
                out.push(DiffLine {
                    tag: ChangeTag::Delete,
                    old_line: Some(oi + 1),
                    new_line: None,
                    text: a[oi].to_string(),
                });
                oi += 1;
            }
            Op::Insert => {
                out.push(DiffLine {
                    tag: ChangeTag::Insert,
                    old_line: None,
                    new_line: Some(ni + 1),
                    text: b[ni].to_string(),
                });
                ni += 1;
            }
        }
    }
    out
}

fn build_hunks(lines: &[DiffLine], context: usize) -> Vec<Hunk> {
    let changed: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.tag != ChangeTag::Equal)
        .map(|(i, _)| i)
        .collect();
    if changed.is_empty() {
        return Vec::new();
    }

    // Group changed lines whose context windows touch or overlap.
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut start = changed[0];
    let mut prev = changed[0];
    for &c in &changed[1..] {
        if c - prev > context * 2 + 1 {
            groups.push((start, prev));
            start = c;
        }
        prev = c;
    }
    groups.push((start, prev));

    groups
        .into_iter()
        .map(|(lo, hi)| {
            let s = lo.saturating_sub(context);
            let e = (hi + context + 1).min(lines.len());
            let slice = &lines[s..e];
            let old_start = slice.iter().find_map(|l| l.old_line).unwrap_or(0);
            let new_start = slice.iter().find_map(|l| l.new_line).unwrap_or(0);
            let old_len = slice.iter().filter(|l| l.old_line.is_some()).count();
            let new_len = slice.iter().filter(|l| l.new_line.is_some()).count();
            Hunk {
                old_start,
                old_len,
                new_start,
                new_len,
                lines: slice.to_vec(),
            }
        })
        .collect()
}

/// The row kind of a side-by-side diff row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// Unchanged line shown on both sides.
    Context,
    /// Line removed (left only).
    Delete,
    /// Line added (right only).
    Insert,
    /// A deletion paired with an insertion (changed line, both sides).
    Replace,
}

/// One cell of a side-by-side row: a 1-based line number and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCell {
    pub line: usize,
    pub text: String,
}

/// One row of a side-by-side diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideRow {
    pub kind: RowKind,
    pub left: Option<SideCell>,
    pub right: Option<SideCell>,
}

/// Projects a [`Hunk`] into aligned left/right rows: runs of deletions and
/// the insertions that follow them are paired index-wise into `Replace`
/// rows, with any leftover shown as pure `Delete` / `Insert` rows.
pub fn side_by_side(hunk: &Hunk) -> Vec<SideRow> {
    let mut rows = Vec::new();
    let mut dels: Vec<&DiffLine> = Vec::new();
    let mut inss: Vec<&DiffLine> = Vec::new();

    for line in &hunk.lines {
        match line.tag {
            ChangeTag::Delete => dels.push(line),
            ChangeTag::Insert => inss.push(line),
            ChangeTag::Equal => {
                flush_side(&mut rows, &mut dels, &mut inss);
                rows.push(SideRow {
                    kind: RowKind::Context,
                    left: Some(SideCell {
                        line: line.old_line.unwrap_or(0),
                        text: line.text.clone(),
                    }),
                    right: Some(SideCell {
                        line: line.new_line.unwrap_or(0),
                        text: line.text.clone(),
                    }),
                });
            }
        }
    }
    flush_side(&mut rows, &mut dels, &mut inss);
    rows
}

fn flush_side(rows: &mut Vec<SideRow>, dels: &mut Vec<&DiffLine>, inss: &mut Vec<&DiffLine>) {
    let n = dels.len().max(inss.len());
    for i in 0..n {
        let del = dels.get(i).copied();
        let ins = inss.get(i).copied();
        let kind = match (del.is_some(), ins.is_some()) {
            (true, true) => RowKind::Replace,
            (true, false) => RowKind::Delete,
            (false, true) => RowKind::Insert,
            (false, false) => unreachable!(),
        };
        rows.push(SideRow {
            kind,
            left: del.map(|l| SideCell {
                line: l.old_line.unwrap_or(0),
                text: l.text.clone(),
            }),
            right: ins.map(|l| SideCell {
                line: l.new_line.unwrap_or(0),
                text: l.text.clone(),
            }),
        });
    }
    dels.clear();
    inss.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(diff: &Diff) -> Vec<ChangeTag> {
        diff.lines.iter().map(|l| l.tag).collect()
    }

    #[test]
    fn identical_inputs_produce_no_hunks() {
        let d = Diff::compute("a\nb\nc\n", "a\nb\nc\n");
        assert!(d.is_unchanged());
        assert_eq!(d.stats(), (0, 0));
        assert!(d.lines.iter().all(|l| l.tag == ChangeTag::Equal));
    }

    #[test]
    fn pure_insertion_is_detected() {
        let d = Diff::compute("a\nc\n", "a\nb\nc\n");
        assert_eq!(
            tags(&d),
            vec![ChangeTag::Equal, ChangeTag::Insert, ChangeTag::Equal]
        );
        assert_eq!(d.stats(), (1, 0));
        let inserted = d.lines.iter().find(|l| l.tag == ChangeTag::Insert).unwrap();
        assert_eq!(inserted.text, "b");
        assert_eq!(inserted.new_line, Some(2));
        assert_eq!(inserted.old_line, None);
    }

    #[test]
    fn pure_deletion_is_detected() {
        let d = Diff::compute("a\nb\nc\n", "a\nc\n");
        assert_eq!(
            tags(&d),
            vec![ChangeTag::Equal, ChangeTag::Delete, ChangeTag::Equal]
        );
        assert_eq!(d.stats(), (0, 1));
    }

    #[test]
    fn modification_is_a_delete_then_insert() {
        let d = Diff::compute("a\nB\nc\n", "a\nb\nc\n");
        assert_eq!(
            tags(&d),
            vec![
                ChangeTag::Equal,
                ChangeTag::Delete,
                ChangeTag::Insert,
                ChangeTag::Equal
            ]
        );
        assert_eq!(d.stats(), (1, 1));
    }

    #[test]
    fn line_numbers_track_both_sides() {
        let d = Diff::compute("keep\nold1\nold2\ntail\n", "keep\nnew1\ntail\n");
        for line in &d.lines {
            match line.tag {
                ChangeTag::Equal => {
                    assert!(line.old_line.is_some() && line.new_line.is_some());
                }
                ChangeTag::Delete => {
                    assert!(line.old_line.is_some() && line.new_line.is_none());
                }
                ChangeTag::Insert => {
                    assert!(line.old_line.is_none() && line.new_line.is_some());
                }
            }
        }
        let tail = d.lines.last().unwrap();
        assert_eq!(tail.text, "tail");
        assert_eq!(tail.old_line, Some(4));
        assert_eq!(tail.new_line, Some(3));
    }

    #[test]
    fn distant_changes_split_into_separate_hunks() {
        let old: String = (1..=20).map(|i| format!("line{i}\n")).collect();
        let mut new_lines: Vec<String> = (1..=20).map(|i| format!("line{i}")).collect();
        new_lines[1] = "CHANGED2".into();
        new_lines[17] = "CHANGED18".into();
        let new = format!("{}\n", new_lines.join("\n"));

        let d = Diff::compute(&old, &new);
        assert_eq!(d.hunks.len(), 2);
        // Each hunk carries 3 lines of context on each side of its change.
        assert!(d.hunks[0].lines.iter().any(|l| l.text == "CHANGED2"));
        assert!(d.hunks[1].lines.iter().any(|l| l.text == "CHANGED18"));
        assert_eq!(d.hunks[0].header(), "@@ -1,5 +1,5 @@");
    }

    #[test]
    fn nearby_changes_merge_into_one_hunk() {
        let old = "a\nb\nc\nd\ne\nf\n";
        let new = "a\nB\nc\nd\ne\nF\n";
        let d = Diff::compute(old, new);
        assert_eq!(d.hunks.len(), 1);
    }

    #[test]
    fn hunk_header_matches_covered_ranges() {
        let d = Diff::compute("a\nb\nc\n", "a\nx\ny\nz\nc\n");
        assert_eq!(d.hunks.len(), 1);
        let h = &d.hunks[0];
        assert_eq!(h.old_start, 1);
        assert_eq!(h.old_len, 3);
        assert_eq!(h.new_start, 1);
        assert_eq!(h.new_len, 5);
        assert_eq!(h.header(), "@@ -1,3 +1,5 @@");
    }

    #[test]
    fn from_empty_to_content_is_all_inserts() {
        let d = Diff::compute("", "a\nb\n");
        assert_eq!(tags(&d), vec![ChangeTag::Insert, ChangeTag::Insert]);
        assert_eq!(d.hunks.len(), 1);
        assert_eq!(d.hunks[0].old_start, 0);
        assert_eq!(d.hunks[0].old_len, 0);
    }

    #[test]
    fn side_by_side_pairs_changes_and_keeps_line_numbers() {
        let d = Diff::compute("a\nB\nc\n", "a\nb\nc\n");
        let rows = side_by_side(&d.hunks[0]);
        let replace = rows.iter().find(|r| r.kind == RowKind::Replace).unwrap();
        assert_eq!(replace.left.as_ref().unwrap().text, "B");
        assert_eq!(replace.left.as_ref().unwrap().line, 2);
        assert_eq!(replace.right.as_ref().unwrap().text, "b");
        assert_eq!(replace.right.as_ref().unwrap().line, 2);
        // Context rows are present on both sides.
        assert!(rows
            .iter()
            .any(|r| r.kind == RowKind::Context && r.left.is_some() && r.right.is_some()));
    }

    #[test]
    fn side_by_side_unbalanced_runs_fall_back_to_single_sided_rows() {
        let d = Diff::compute("a\nx\ny\nz\nb\n", "a\nX\nb\n");
        let rows = side_by_side(&d.hunks[0]);
        assert_eq!(
            rows.iter().filter(|r| r.kind == RowKind::Replace).count(),
            1
        );
        assert_eq!(rows.iter().filter(|r| r.kind == RowKind::Delete).count(), 2);
        assert!(rows.iter().all(|r| r.kind != RowKind::Insert));
    }

    #[test]
    fn blank_lines_are_preserved() {
        let d = Diff::compute("a\n\nb\n", "a\nb\n");
        let deleted: Vec<_> = d
            .lines
            .iter()
            .filter(|l| l.tag == ChangeTag::Delete)
            .collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].text, "");
    }
}
