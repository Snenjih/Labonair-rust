//! Merge-conflict markers in a worktree file: parsing and resolution.
//!
//! Pure text in, text out. The DiffView lists every `<<<<<<<` … `>>>>>>>`
//! block of a conflicted file and rewrites the file when the user picks a side.

/// Which side(s) of a conflict block to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The current branch's lines (between `<<<<<<<` and `=======`).
    Ours,
    /// The incoming lines (between `=======` and `>>>>>>>`).
    Theirs,
    /// Ours followed by theirs.
    Both,
}

/// One conflict block. Line indices are zero-based into the file's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictBlock {
    /// Line of the `<<<<<<<` marker.
    pub start: usize,
    /// Line of the `=======` separator.
    pub middle: usize,
    /// Line of the `>>>>>>>` marker.
    pub end: usize,
    /// Text after `<<<<<<<` (usually `HEAD`).
    pub ours_label: String,
    /// Text after `>>>>>>>` (usually the merged branch).
    pub theirs_label: String,
    pub ours: Vec<String>,
    pub theirs: Vec<String>,
}

/// Every well-formed conflict block of `text`, in order. A block missing its
/// separator or end marker is ignored (the file is left untouched there).
/// A `|||||||` (diff3 base) section is skipped: ours and theirs stay exact.
pub fn parse_conflicts(text: &str) -> Vec<ConflictBlock> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(label) = lines[i].strip_prefix("<<<<<<<") {
            let start = i;
            let mut middle = None;
            let mut base = None;
            let mut j = i + 1;
            while j < lines.len() {
                let line = lines[j];
                if line.starts_with("<<<<<<<") {
                    break;
                }
                if line.starts_with("|||||||") && middle.is_none() {
                    base = Some(j);
                } else if line.starts_with("=======") && middle.is_none() {
                    middle = Some(j);
                } else if let (Some(mid), Some(theirs_label)) =
                    (middle, line.strip_prefix(">>>>>>>"))
                {
                    let ours_end = base.unwrap_or(mid);
                    blocks.push(ConflictBlock {
                        start,
                        middle: mid,
                        end: j,
                        ours_label: label.trim().to_string(),
                        theirs_label: theirs_label.trim().to_string(),
                        ours: lines[start + 1..ours_end]
                            .iter()
                            .map(|l| l.to_string())
                            .collect(),
                        theirs: lines[mid + 1..j].iter().map(|l| l.to_string()).collect(),
                    });
                    i = j;
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }
    blocks
}

/// `text` with conflict block `index` replaced by the chosen side(s). `None`
/// when the block does not exist (the file changed since it was parsed).
pub fn resolve(text: &str, index: usize, resolution: Resolution) -> Option<String> {
    let block = parse_conflicts(text).into_iter().nth(index)?;
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<&str> = lines[..block.start].to_vec();
    match resolution {
        Resolution::Ours => out.extend(block.ours.iter().map(String::as_str)),
        Resolution::Theirs => out.extend(block.theirs.iter().map(String::as_str)),
        Resolution::Both => {
            out.extend(block.ours.iter().map(String::as_str));
            out.extend(block.theirs.iter().map(String::as_str));
        }
    }
    out.extend(&lines[block.end + 1..]);
    Some(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "a\n<<<<<<< HEAD\nour1\nour2\n=======\ntheir1\n>>>>>>> feature\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> feature\nc\n";

    #[test]
    fn parses_every_block_with_labels_and_sides() {
        let blocks = parse_conflicts(TEXT);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].ours_label, "HEAD");
        assert_eq!(blocks[0].theirs_label, "feature");
        assert_eq!(blocks[0].ours, ["our1", "our2"]);
        assert_eq!(blocks[0].theirs, ["their1"]);
        assert_eq!(
            (blocks[0].start, blocks[0].middle, blocks[0].end),
            (1, 4, 6)
        );
    }

    #[test]
    fn resolves_one_block_and_keeps_the_rest_of_the_file() {
        assert_eq!(
            resolve(TEXT, 0, Resolution::Ours).unwrap(),
            "a\nour1\nour2\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> feature\nc\n"
        );
        assert_eq!(
            resolve(TEXT, 1, Resolution::Theirs).unwrap(),
            "a\n<<<<<<< HEAD\nour1\nour2\n=======\ntheir1\n>>>>>>> feature\nb\ny\nc\n"
        );
        let both = resolve(TEXT, 0, Resolution::Both).unwrap();
        assert!(both.starts_with("a\nour1\nour2\ntheir1\nb\n"));
        assert!(resolve(TEXT, 2, Resolution::Ours).is_none());
    }

    #[test]
    fn diff3_base_section_is_not_part_of_ours() {
        let text = "<<<<<<< HEAD\nours\n||||||| base\nold\n=======\ntheirs\n>>>>>>> topic\n";
        let block = &parse_conflicts(text)[0];
        assert_eq!(block.ours, ["ours"]);
        assert_eq!(block.theirs, ["theirs"]);
    }

    #[test]
    fn unterminated_blocks_are_ignored() {
        assert!(parse_conflicts("<<<<<<< HEAD\nx\n=======\ny\n").is_empty());
        assert!(parse_conflicts("plain text\n").is_empty());
    }
}
