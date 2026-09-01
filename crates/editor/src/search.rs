//! In-buffer find / replace (single-line matches, literal — not regex).

use crate::buffer::{Position, TextBuffer};

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    pub text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start: Position,
    pub end: Position,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// All matches for `query`, in document order.
pub fn find_all(buffer: &TextBuffer, query: &SearchQuery) -> Vec<Match> {
    if query.text.is_empty() {
        return Vec::new();
    }
    let needle: Vec<char> = if query.case_sensitive {
        query.text.chars().collect()
    } else {
        query.text.to_lowercase().chars().collect()
    };

    let mut out = Vec::new();
    for (line_idx, line) in (0..buffer.line_count()).map(|i| (i, buffer.line(i))) {
        let hay: Vec<char> = if query.case_sensitive {
            line.chars().collect()
        } else {
            line.to_lowercase().chars().collect()
        };
        if hay.len() < needle.len() {
            continue;
        }
        let mut col = 0;
        while col + needle.len() <= hay.len() {
            if hay[col..col + needle.len()] == needle[..] {
                let before_ok = col == 0 || !is_word_char(hay[col - 1]);
                let after_idx = col + needle.len();
                let after_ok = after_idx >= hay.len() || !is_word_char(hay[after_idx]);
                if !query.whole_word || (before_ok && after_ok) {
                    out.push(Match {
                        start: Position::new(line_idx, col),
                        end: Position::new(line_idx, col + needle.len()),
                    });
                    col += needle.len();
                    continue;
                }
            }
            col += 1;
        }
    }
    out
}

/// Index of the first match at or after `from` (wrapping), if any.
pub fn next_match(matches: &[Match], from: Position, forward: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    if forward {
        matches.iter().position(|m| m.start >= from).or(Some(0))
    } else {
        matches
            .iter()
            .rposition(|m| m.start < from)
            .or(Some(matches.len() - 1))
    }
}

/// Replace every match with `replacement`. Returns the number replaced.
pub fn replace_all(buffer: &mut TextBuffer, query: &SearchQuery, replacement: &str) -> usize {
    let matches = find_all(buffer, query);
    let n = matches.len();
    // Apply back-to-front so earlier positions stay valid.
    for m in matches.into_iter().rev() {
        buffer.delete(m.start, m.end);
        buffer.insert(m.start, replacement);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn case_insensitive_by_default() {
        let b = TextBuffer::from_text("Foo foo FOO");
        assert_eq!(find_all(&b, &q("foo")).len(), 3);
    }

    #[test]
    fn case_sensitive_option() {
        let b = TextBuffer::from_text("Foo foo FOO");
        let query = SearchQuery {
            text: "foo".into(),
            case_sensitive: true,
            whole_word: false,
        };
        assert_eq!(find_all(&b, &query).len(), 1);
    }

    #[test]
    fn whole_word_option() {
        let b = TextBuffer::from_text("bar barrier bar");
        let query = SearchQuery {
            text: "bar".into(),
            case_sensitive: false,
            whole_word: true,
        };
        let m = find_all(&b, &query);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].start, Position::new(0, 0));
    }

    #[test]
    fn replace_all_counts_and_rewrites() {
        let mut b = TextBuffer::from_text("a.a.a");
        let n = replace_all(&mut b, &q("a"), "X");
        assert_eq!(n, 3);
        assert_eq!(b.text(), "X.X.X");
    }

    #[test]
    fn next_match_wraps() {
        let b = TextBuffer::from_text("x y x y x");
        let m = find_all(&b, &q("x"));
        assert_eq!(next_match(&m, Position::new(0, 5), true), Some(2));
        assert_eq!(next_match(&m, Position::new(0, 9), true), Some(0));
        assert_eq!(next_match(&m, Position::new(0, 0), false), Some(2));
    }
}
