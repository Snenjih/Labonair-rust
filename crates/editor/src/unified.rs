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
        });
    }
    files
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
