//! Plain text buffer: a line vector with char-indexed positions.
//!
//! This is deliberately simple (a `Vec<String>`, one entry per line, `\n`
//! separated on serialisation). It is fast enough for the file sizes an editor
//! tab realistically opens and keeps the editing math obvious. A rope can slot
//! in behind the same API later if profiling demands it.

/// A caret / selection endpoint. `column` is a **character** index within the
/// line (not a byte offset), `line` is 0-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Line-oriented text storage. Always holds at least one (possibly empty) line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBuffer {
    lines: Vec<String>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
        }
    }
}

impl TextBuffer {
    /// Parse `text` into lines. A trailing newline yields a final empty line,
    /// matching editor conventions (and round-tripping through [`Self::text`]).
    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        Self {
            lines: if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            },
        }
    }

    /// Serialise back to a `\n`-joined string.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, idx: usize) -> &str {
        self.lines.get(idx).map(String::as_str).unwrap_or("")
    }

    /// Character length of a line.
    pub fn line_len(&self, idx: usize) -> usize {
        self.lines.get(idx).map(|l| l.chars().count()).unwrap_or(0)
    }

    /// Total character count (including the `\n` separators).
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.chars().count()).sum::<usize>()
            + self.lines.len().saturating_sub(1)
    }

    /// Clamp a position into the valid range for the current content.
    pub fn clamp(&self, pos: Position) -> Position {
        let line = pos.line.min(self.lines.len().saturating_sub(1));
        let column = pos.column.min(self.line_len(line));
        Position { line, column }
    }

    /// The last valid position (end of the last line).
    pub fn end(&self) -> Position {
        let line = self.lines.len().saturating_sub(1);
        Position {
            line,
            column: self.line_len(line),
        }
    }

    fn byte_offset(line: &str, char_col: usize) -> usize {
        line.char_indices()
            .nth(char_col)
            .map(|(b, _)| b)
            .unwrap_or(line.len())
    }

    /// Insert `text` at `pos`, returning the position just past the insertion.
    pub fn insert(&mut self, pos: Position, text: &str) -> Position {
        let pos = self.clamp(pos);
        let current = self.lines[pos.line].clone();
        let split = Self::byte_offset(&current, pos.column);
        let (before, after) = current.split_at(split);

        let mut pieces = text.split('\n');
        let first = pieces.next().unwrap_or("");
        let rest: Vec<&str> = pieces.collect();

        if rest.is_empty() {
            let mut merged = String::with_capacity(before.len() + first.len() + after.len());
            merged.push_str(before);
            merged.push_str(first);
            merged.push_str(after);
            self.lines[pos.line] = merged;
            return Position {
                line: pos.line,
                column: pos.column + first.chars().count(),
            };
        }

        let mut new_lines: Vec<String> = Vec::with_capacity(rest.len() + 1);
        new_lines.push(format!("{before}{first}"));
        for mid in &rest[..rest.len() - 1] {
            new_lines.push((*mid).to_string());
        }
        let last = rest[rest.len() - 1];
        let end_col = last.chars().count();
        new_lines.push(format!("{last}{after}"));

        let end_line = pos.line + rest.len();
        self.lines.splice(pos.line..=pos.line, new_lines);
        Position {
            line: end_line,
            column: end_col,
        }
    }

    /// Delete `[start, end)` and return the removed text. Order-insensitive.
    pub fn delete(&mut self, a: Position, b: Position) -> String {
        let (start, end) = order(self.clamp(a), self.clamp(b));
        if start == end {
            return String::new();
        }

        if start.line == end.line {
            let line = &self.lines[start.line];
            let s = Self::byte_offset(line, start.column);
            let e = Self::byte_offset(line, end.column);
            let removed = line[s..e].to_string();
            self.lines[start.line].replace_range(s..e, "");
            return removed;
        }

        let first = self.lines[start.line].clone();
        let last = self.lines[end.line].clone();
        let s = Self::byte_offset(&first, start.column);
        let e = Self::byte_offset(&last, end.column);

        let mut removed = String::new();
        removed.push_str(&first[s..]);
        for mid in &self.lines[start.line + 1..end.line] {
            removed.push('\n');
            removed.push_str(mid);
        }
        removed.push('\n');
        removed.push_str(&last[..e]);

        let merged = format!("{}{}", &first[..s], &last[e..]);
        self.lines.splice(start.line..=end.line, [merged]);
        removed
    }
}

fn order(a: Position, b: Position) -> (Position, Position) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserves_trailing_newline() {
        for src in ["", "abc", "a\nb", "a\nb\n", "\n\n"] {
            assert_eq!(TextBuffer::from_text(src).text(), src);
        }
    }

    #[test]
    fn insert_single_line() {
        let mut b = TextBuffer::from_text("hello");
        let end = b.insert(Position::new(0, 5), " world");
        assert_eq!(b.text(), "hello world");
        assert_eq!(end, Position::new(0, 11));
    }

    #[test]
    fn insert_with_newlines() {
        let mut b = TextBuffer::from_text("ac");
        let end = b.insert(Position::new(0, 1), "b\nX");
        assert_eq!(b.text(), "ab\nXc");
        assert_eq!(end, Position::new(1, 1));
    }

    #[test]
    fn delete_within_line() {
        let mut b = TextBuffer::from_text("hello world");
        let removed = b.delete(Position::new(0, 5), Position::new(0, 11));
        assert_eq!(removed, " world");
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn delete_across_lines() {
        let mut b = TextBuffer::from_text("one\ntwo\nthree");
        let removed = b.delete(Position::new(0, 1), Position::new(2, 2));
        assert_eq!(removed, "ne\ntwo\nth");
        assert_eq!(b.text(), "oree");
    }

    #[test]
    fn delete_is_order_insensitive() {
        let mut b = TextBuffer::from_text("abcdef");
        b.delete(Position::new(0, 4), Position::new(0, 2));
        assert_eq!(b.text(), "abef");
    }

    #[test]
    fn unicode_columns_are_chars_not_bytes() {
        let mut b = TextBuffer::from_text("é•é");
        assert_eq!(b.line_len(0), 3);
        b.insert(Position::new(0, 1), "X");
        assert_eq!(b.text(), "éX•é");
    }
}
