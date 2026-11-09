//! Unified-diff (`git diff` textual output) parsing.
//!
//! Moved here from `labonair-panel-scm` in the Zed-parity Phase 4 redesign so
//! that both the Source-Control panel and the workspace-level Project Diff item
//! parse `git diff` output through one implementation. Port of the reference web
//! app's `source-control/lib/diffHunks.ts`; the byte-for-byte body lines are
//! kept verbatim so a re-applied hunk patch round-trips (CRLF included).

/// One `@@ … @@` block of a unified diff, body lines kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The raw `@@ -a,b +c,d @@ …` line including trailing context.
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// Raw body lines (` `/`+`/`-`/`\ No newline…`), unmodified, in order.
    pub lines: Vec<String>,
}

/// Per-file view of a (possibly multi-file) unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// b-side path (post-rename for renames).
    pub path: String,
    /// Lines from `diff --git …` up to (excluding) the first hunk header.
    pub header_lines: Vec<String>,
    pub hunks: Vec<DiffHunk>,
    pub is_new_file: bool,
    pub is_deleted_file: bool,
    /// Every body entry of the file when it was requested with whole-file
    /// context and regrouped by [`regroup_full_context`]. One entry per line;
    /// a `\ No newline` marker is joined onto its line with `\n`.
    pub full_lines: Option<Vec<String>>,
    /// For each (regrouped) hunk, its range in `full_lines`.
    pub hunk_spans: Vec<std::ops::Range<usize>>,
    /// Old / new line number of `full_lines[0]`.
    pub full_start: (u32, u32),
}

fn parse_file_header_path(line: &str) -> Option<String> {
    // ^diff --git a/.+ b/(.+)$
    let rest = line.strip_prefix("diff --git a/")?;
    let idx = rest.find(" b/")?;
    Some(rest[idx + 3..].to_string())
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    // ^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@
    let rest = line.strip_prefix("@@ -")?;
    let end = rest.find(" @@")?;
    let spec = &rest[..end];
    let mut sides = spec.split(" +");
    let old = sides.next()?;
    let new = sides.next()?;
    let parse_pair = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (os, ol) = parse_pair(old)?;
    let (ns, nl) = parse_pair(new)?;
    Some((os, ol, ns, nl))
}

/// Parses a unified diff into per-file hunk structures. Returns `[]` for a
/// backend-truncated diff (a cut-off final hunk could corrupt the index).
pub fn parse_diff_hunks(diff: &str) -> Vec<FileDiff> {
    if diff.is_empty() || diff.contains("[diff truncated") || diff.contains("[diff too large]") {
        return Vec::new();
    }

    let all: Vec<&str> = diff.split('\n').collect();
    // Split into chunks at each "diff --git a/… b/…" line.
    let mut starts: Vec<usize> = Vec::new();
    for (i, l) in all.iter().enumerate() {
        if l.starts_with("diff --git a/") && l.contains(" b/") {
            starts.push(i);
        }
    }
    let mut files = Vec::new();
    for (si, &start) in starts.iter().enumerate() {
        let end = starts.get(si + 1).copied().unwrap_or(all.len());
        let mut lines: Vec<String> = all[start..end].iter().map(|s| s.to_string()).collect();
        // Drop the single trailing "" produced by the terminating "\n".
        if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        let Some(path) = lines.first().and_then(|l| parse_file_header_path(l)) else {
            continue;
        };
        let first_hunk = lines.iter().position(|l| parse_hunk_header(l).is_some());
        let header_lines: Vec<String> = match first_hunk {
            Some(idx) => lines[..idx].to_vec(),
            None => lines.clone(),
        };
        let is_new_file = header_lines.iter().any(|l| l.starts_with("new file mode"));
        let is_deleted_file = header_lines
            .iter()
            .any(|l| l.starts_with("deleted file mode"));

        let mut hunks = Vec::new();
        if let Some(mut i) = first_hunk {
            while i < lines.len() {
                let Some((os, ol, ns, nl)) = parse_hunk_header(&lines[i]) else {
                    break;
                };
                let header = lines[i].clone();
                let body_start = i + 1;
                let mut body_end = body_start;
                while body_end < lines.len() && parse_hunk_header(&lines[body_end]).is_none() {
                    body_end += 1;
                }
                hunks.push(DiffHunk {
                    header,
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    lines: lines[body_start..body_end].to_vec(),
                });
                i = body_end;
            }
        }
        files.push(FileDiff {
            path,
            header_lines,
            hunks,
            is_new_file,
            is_deleted_file,
            full_lines: None,
            hunk_spans: Vec::new(),
            full_start: (1, 1),
        });
    }
    files
}

/// Diff of two in-memory texts as a whole-file [`FileDiff`] (already
/// regrouped), for comparisons that have no Git revision behind them.
/// `None` when the texts are equal.
pub fn text_file_diff(title: &str, old: &str, new: &str) -> Option<FileDiff> {
    let diff = crate::diff::Diff::compute(old, new);
    if diff.hunks.is_empty() {
        return None;
    }
    let count = |keep: &dyn Fn(&crate::diff::DiffLine) -> bool| {
        diff.lines.iter().filter(|l| keep(l)).count()
    };
    let (old_len, new_len) = (
        count(&|l| l.tag != crate::diff::ChangeTag::Insert),
        count(&|l| l.tag != crate::diff::ChangeTag::Delete),
    );
    let mut patch = format!(
        "diff --git a/{title} b/{title}\n--- a/{title}\n+++ b/{title}\n@@ -{},{old_len} +{},{new_len} @@\n",
        usize::from(old_len > 0),
        usize::from(new_len > 0),
    );
    for line in &diff.lines {
        patch.push(match line.tag {
            crate::diff::ChangeTag::Equal => ' ',
            crate::diff::ChangeTag::Delete => '-',
            crate::diff::ChangeTag::Insert => '+',
        });
        patch.push_str(&line.text);
        patch.push('\n');
    }
    parse_diff_hunks(&patch)
        .into_iter()
        .next()
        .map(regroup_full_context)
}

/// Context lines Git itself puts around a hunk; regrouped hunks match it so a
/// standalone patch for `git apply` stays what Git would have produced.
pub const HUNK_CONTEXT: usize = 3;

/// Turns a diff requested with whole-file context (one giant hunk) into
/// ordinary Git-style hunks (3 lines of context, changes closer than twice
/// that merged) while keeping every line of the file in
/// [`FileDiff::full_lines`] for context expansion. Diffs that are not a
/// single modified-file hunk come back unchanged.
pub fn regroup_full_context(mut diff: FileDiff) -> FileDiff {
    if diff.hunks.len() != 1 || is_whole_file_single_hunk(&diff) {
        return diff;
    }
    let (old0, new0) = (diff.hunks[0].old_start, diff.hunks[0].new_start);
    let mut entries: Vec<String> = Vec::new();
    for line in &diff.hunks[0].lines {
        match entries.last_mut() {
            Some(last) if line.starts_with('\\') => {
                last.push('\n');
                last.push_str(line);
            }
            _ => entries.push(line.clone()),
        }
    }
    let is_change = |e: &String| e.starts_with('+') || e.starts_with('-');
    let mut groups: Vec<(usize, usize)> = Vec::new();
    for (i, _) in entries.iter().enumerate().filter(|(_, e)| is_change(e)) {
        match groups.last_mut() {
            Some((_, last)) if i - *last - 1 <= 2 * HUNK_CONTEXT => *last = i,
            _ => groups.push((i, i)),
        }
    }
    if groups.is_empty() {
        return diff;
    }
    let before = |upto: usize, skip: char| -> u32 {
        entries[..upto]
            .iter()
            .filter(|e| !e.starts_with(skip))
            .count() as u32
    };
    let mut hunks = Vec::new();
    let mut spans = Vec::new();
    for (first, last) in groups {
        let lo = first.saturating_sub(HUNK_CONTEXT);
        let hi = (last + 1 + HUNK_CONTEXT).min(entries.len());
        let old_lines = entries[lo..hi]
            .iter()
            .filter(|e| !e.starts_with('+'))
            .count() as u32;
        let new_lines = entries[lo..hi]
            .iter()
            .filter(|e| !e.starts_with('-'))
            .count() as u32;
        // Git numbers an empty side by the line *before* the hunk.
        let mut old_start = old0 + before(lo, '+');
        let mut new_start = new0 + before(lo, '-');
        if old_lines == 0 {
            old_start = old_start.saturating_sub(1);
        }
        if new_lines == 0 {
            new_start = new_start.saturating_sub(1);
        }
        hunks.push(DiffHunk {
            header: format!("@@ -{old_start},{old_lines} +{new_start},{new_lines} @@"),
            old_start,
            old_lines,
            new_start,
            new_lines,
            lines: entries[lo..hi]
                .iter()
                .flat_map(|e| e.split('\n').map(str::to_string))
                .collect(),
        });
        spans.push(lo..hi);
    }
    diff.hunks = hunks;
    diff.hunk_spans = spans;
    diff.full_lines = Some(entries);
    diff.full_start = (old0, new0);
    diff
}

/// Builds a standalone one-hunk unified-diff patch for `git apply --cached`.
pub fn build_hunk_patch(file: &FileDiff, hunk: &DiffHunk) -> String {
    let mut parts: Vec<&str> = file.header_lines.iter().map(|s| s.as_str()).collect();
    parts.push(hunk.header.as_str());
    for l in &hunk.lines {
        parts.push(l.as_str());
    }
    format!("{}\n", parts.join("\n"))
}

/// A brand-new / fully-deleted file collapses to one whole-file hunk — the
/// plain `git add` / `git restore --staged` path is far more robust than
/// applying a synthetic patch, so callers prefer it when this is true.
pub fn is_whole_file_single_hunk(file: &FileDiff) -> bool {
    (file.is_new_file || file.is_deleted_file) && file.hunks.len() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures are joined explicitly (not via `\` line-continuation, which
    // would strip the leading space that marks a diff context line).
    fn two_hunk_diff() -> String {
        [
            "diff --git a/tracked.txt b/tracked.txt",
            "index fe7fa38..e0b0b1c 100644",
            "--- a/tracked.txt",
            "+++ b/tracked.txt",
            "@@ -1,5 +1,5 @@",
            " a1",
            "-a2",
            "+a2_CHANGED",
            " a3",
            " a4",
            " a5",
            "@@ -11,5 +11,5 @@ a10",
            " a11",
            " a12",
            " a13",
            "-a14",
            "+a14_CHANGED",
            " a15",
            "",
        ]
        .join("\n")
    }

    fn new_file_diff() -> String {
        [
            "diff --git a/newfile.txt b/newfile.txt",
            "new file mode 100644",
            "index 0000000..71ac1b5",
            "--- /dev/null",
            "+++ b/newfile.txt",
            "@@ -0,0 +1,3 @@",
            "+a",
            "+b",
            "+c",
            "",
        ]
        .join("\n")
    }

    fn deleted_file_diff() -> String {
        [
            "diff --git a/newfile.txt b/newfile.txt",
            "deleted file mode 100644",
            "index 71ac1b5..0000000",
            "--- a/newfile.txt",
            "+++ /dev/null",
            "@@ -1,3 +0,0 @@",
            "-a",
            "-b",
            "-c",
            "",
        ]
        .join("\n")
    }

    fn crlf_diff() -> String {
        [
            "diff --git a/crlf.txt b/crlf.txt",
            "index 46b21fa..f146c25 100644",
            "--- a/crlf.txt",
            "+++ b/crlf.txt",
            "@@ -1,3 +1,3 @@",
            " x1\r",
            "-x2\r",
            "+X2_CHANGED\r",
            " x3\r",
            "",
        ]
        .join("\n")
    }

    /// Real `git diff -U1000000` of a 40-line file (edits on lines 3, 12, 13,
    /// and an insertion after 30); `git diff` itself produced the headers
    /// asserted below.
    #[test]
    fn regrouping_matches_the_hunks_git_produces() {
        let text = [
            "diff --git a/f.txt b/f.txt",
            "index 1c99002..53b354f 100644",
            "--- a/f.txt",
            "+++ b/f.txt",
            "@@ -1,40 +1,40 @@",
            " 1",
            " 2",
            "-3",
            "+three",
            " 4",
            " 5",
            " 6",
            " 7",
            " 8",
            " 9",
            " 10",
            " 11",
            "-12",
            "-13",
            "+twelve",
            " 14",
            " 15",
            " 16",
            " 17",
            " 18",
            " 19",
            " 20",
            " 21",
            " 22",
            " 23",
            " 24",
            " 25",
            " 26",
            " 27",
            " 28",
            " 29",
            " 30",
            "+inserted",
            " 31",
            " 32",
            " 33",
            " 34",
            " 35",
            " 36",
            " 37",
            " 38",
            " 39",
            " 40",
        ]
        .join("\n")
            + "\n";
        let diff = regroup_full_context(parse_diff_hunks(&text).remove(0));
        let headers: Vec<_> = diff.hunks.iter().map(|h| h.header.as_str()).collect();
        assert_eq!(
            headers,
            ["@@ -1,6 +1,6 @@", "@@ -9,8 +9,7 @@", "@@ -28,6 +27,7 @@"]
        );
    }

    #[test]
    fn text_diffs_become_regrouped_whole_file_diffs() {
        let old = (1..=20).map(|n| format!("l{n}\n")).collect::<String>();
        let new = old.replace("l10\n", "ten\n");
        let diff = text_file_diff("clip", &old, &new).expect("changed");
        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.full_lines.as_ref().map(Vec::len), Some(21));
        assert!(text_file_diff("clip", &old, &old).is_none());
    }

    #[test]
    fn full_context_diff_regroups_into_git_style_hunks() {
        let mut body = vec!["@@ -1,20 +1,20 @@".to_string()];
        for n in 1..=20 {
            match n {
                2 => {
                    body.push("-l2".into());
                    body.push("+L2".into());
                }
                15 => {
                    body.push("-l15".into());
                    body.push("+L15".into());
                }
                _ => body.push(format!(" l{n}")),
            }
        }
        let mut text = vec![
            "diff --git a/f.txt b/f.txt".to_string(),
            "--- a/f.txt".into(),
            "+++ b/f.txt".into(),
        ];
        text.extend(body);
        let parsed = parse_diff_hunks(&(text.join("\n") + "\n")).remove(0);
        let diff = regroup_full_context(parsed);
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].header, "@@ -1,5 +1,5 @@");
        assert_eq!(diff.hunks[1].header, "@@ -12,7 +12,7 @@");
        assert_eq!(diff.full_lines.as_ref().map(Vec::len), Some(22));
        assert_eq!(diff.hunk_spans, vec![0..6, 12..20]);
        // Each regrouped hunk is a valid standalone patch body.
        let patch = build_hunk_patch(&diff, &diff.hunks[1]);
        assert!(patch.contains("@@ -12,7 +12,7 @@\n l12"));
    }

    #[test]
    fn splits_two_hunks_with_line_numbers() {
        let files = parse_diff_hunks(&two_hunk_diff());
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.path, "tracked.txt");
        assert!(!f.is_new_file && !f.is_deleted_file);
        assert_eq!(f.hunks.len(), 2);
        assert_eq!(
            (
                f.hunks[0].old_start,
                f.hunks[0].old_lines,
                f.hunks[0].new_start,
                f.hunks[0].new_lines
            ),
            (1, 5, 1, 5)
        );
        assert_eq!(f.hunks[1].header, "@@ -11,5 +11,5 @@ a10");
    }

    #[test]
    fn detects_new_and_deleted_files() {
        let n = &parse_diff_hunks(&new_file_diff())[0];
        assert!(n.is_new_file && !n.is_deleted_file);
        assert!(is_whole_file_single_hunk(n));
        let d = &parse_diff_hunks(&deleted_file_diff())[0];
        assert!(d.is_deleted_file && !d.is_new_file);
        assert!(is_whole_file_single_hunk(d));
    }

    #[test]
    fn multi_hunk_is_not_whole_file() {
        assert!(!is_whole_file_single_hunk(
            &parse_diff_hunks(&two_hunk_diff())[0]
        ));
    }

    #[test]
    fn preserves_crlf_content_bytes() {
        let f = &parse_diff_hunks(&crlf_diff())[0];
        assert!(f.hunks[0].lines.contains(&" x1\r".to_string()));
        assert!(f.hunks[0].lines.contains(&"-x2\r".to_string()));
        assert!(f.hunks[0].lines.contains(&"+X2_CHANGED\r".to_string()));
    }

    #[test]
    fn truncated_and_empty_return_nothing() {
        let truncated = format!(
            "{}\n\n[diff truncated \u{2014} output exceeded 200 KB]",
            two_hunk_diff()
        );
        assert!(parse_diff_hunks(&truncated).is_empty());
        assert!(parse_diff_hunks("").is_empty());
    }

    #[test]
    fn parses_multiple_files_independently() {
        let combined = format!("{}\n{}", two_hunk_diff(), new_file_diff());
        let files = parse_diff_hunks(&combined);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "tracked.txt");
        assert_eq!(files[1].path, "newfile.txt");
        assert!(files[1].is_new_file);
    }

    #[test]
    fn builds_standalone_hunk_patch() {
        let f = &parse_diff_hunks(&two_hunk_diff())[0];
        let patch = build_hunk_patch(f, &f.hunks[0]);
        let expected = [
            "diff --git a/tracked.txt b/tracked.txt",
            "index fe7fa38..e0b0b1c 100644",
            "--- a/tracked.txt",
            "+++ b/tracked.txt",
            "@@ -1,5 +1,5 @@",
            " a1",
            "-a2",
            "+a2_CHANGED",
            " a3",
            " a4",
            " a5",
            "",
        ]
        .join("\n");
        assert_eq!(patch, expected);
    }

    #[test]
    fn hunk_patch_round_trips_crlf() {
        let f = &parse_diff_hunks(&crlf_diff())[0];
        let patch = build_hunk_patch(f, &f.hunks[0]);
        assert!(patch.contains(" x1\r\n"));
        assert!(patch.contains("-x2\r\n"));
        assert!(patch.contains("+X2_CHANGED\r\n"));
    }
}
