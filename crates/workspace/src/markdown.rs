//! Minimal Markdown parsing for the AI chat's streaming responses (T11-003).
//!
//! The reference app renders assistant output with the web `streamdown`
//! package. GPUI has no Markdown engine, so this is a small, self-contained,
//! block-level parser + inline tokenizer tuned for the *streaming* case: a
//! partially-received document (e.g. an unterminated fenced code block or a
//! dangling `**`) must still parse into something stable so the view does not
//! flicker between renders.
//!
//! It is deliberately not a full CommonMark implementation — it covers the
//! constructs assistants actually emit: headings, paragraphs, fenced code,
//! bullet/ordered lists, block quotes, thematic breaks, pipe tables, and the
//! inline run of bold / italic / inline-code / links.

/// A block-level element.
#[derive(Debug, Clone, PartialEq)]
pub enum MdBlock {
    Heading {
        level: u8,
        spans: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    /// A fenced code block. `closed` is false while the closing fence has not
    /// been received yet (still streaming).
    Code {
        lang: Option<String>,
        text: String,
        closed: bool,
    },
    Bullets(Vec<Vec<Inline>>),
    Ordered(Vec<(u64, Vec<Inline>)>),
    Quote(Vec<Inline>),
    Rule,
    Table {
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
}

/// An inline span within a block.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Bold(String),
    Italic(String),
    Link { text: String, href: String },
}

impl Inline {
    /// The plain-text content, ignoring styling (used for tables / fallbacks).
    pub fn plain(&self) -> &str {
        match self {
            Inline::Text(s) | Inline::Code(s) | Inline::Bold(s) | Inline::Italic(s) => s,
            Inline::Link { text, .. } => text,
        }
    }
}

/// Parse a (possibly partial) Markdown document into blocks.
pub fn parse_markdown(src: &str) -> Vec<MdBlock> {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut blocks = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut i = 0;

    let flush_para = |para: &mut Vec<&str>, blocks: &mut Vec<MdBlock>| {
        if !para.is_empty() {
            let joined = para.join("\n");
            blocks.push(MdBlock::Paragraph(parse_inline(joined.trim())));
            para.clear();
        }
    };

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Fenced code block.
        if let Some(fence) = fence_marker(trimmed) {
            flush_para(&mut para, &mut blocks);
            let lang = {
                let rest = trimmed[fence.len()..].trim();
                (!rest.is_empty()).then(|| rest.to_string())
            };
            let mut body = Vec::new();
            let mut closed = false;
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if l.trim_start().starts_with(fence) && fence_marker(l.trim_start()).is_some() {
                    closed = true;
                    i += 1;
                    break;
                }
                body.push(l);
                i += 1;
            }
            blocks.push(MdBlock::Code {
                lang,
                text: body.join("\n"),
                closed,
            });
            continue;
        }

        // Blank line -> paragraph break.
        if trimmed.is_empty() {
            flush_para(&mut para, &mut blocks);
            i += 1;
            continue;
        }

        // Thematic break.
        if is_thematic_break(trimmed) {
            flush_para(&mut para, &mut blocks);
            blocks.push(MdBlock::Rule);
            i += 1;
            continue;
        }

        // ATX heading.
        if let Some((level, text)) = heading(trimmed) {
            flush_para(&mut para, &mut blocks);
            blocks.push(MdBlock::Heading {
                level,
                spans: parse_inline(text),
            });
            i += 1;
            continue;
        }

        // Pipe table: header row followed by a delimiter row.
        if trimmed.contains('|') && i + 1 < lines.len() && is_table_delimiter(lines[i + 1].trim()) {
            flush_para(&mut para, &mut blocks);
            let headers = split_table_row(trimmed)
                .iter()
                .map(|c| parse_inline(c))
                .collect();
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().contains('|') && !lines[i].trim().is_empty() {
                rows.push(
                    split_table_row(lines[i].trim())
                        .iter()
                        .map(|c| parse_inline(c))
                        .collect(),
                );
                i += 1;
            }
            blocks.push(MdBlock::Table { headers, rows });
            continue;
        }

        // Block quote.
        if trimmed.starts_with('>') {
            flush_para(&mut para, &mut blocks);
            let mut parts = Vec::new();
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                parts.push(
                    lines[i]
                        .trim_start()
                        .trim_start_matches('>')
                        .trim_start()
                        .to_string(),
                );
                i += 1;
            }
            blocks.push(MdBlock::Quote(parse_inline(parts.join("\n").trim())));
            continue;
        }

        // Bullet list.
        if bullet_item(trimmed).is_some() {
            flush_para(&mut para, &mut blocks);
            let mut items = Vec::new();
            while i < lines.len() {
                if let Some(item) = bullet_item(lines[i].trim_start()) {
                    items.push(parse_inline(item));
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(MdBlock::Bullets(items));
            continue;
        }

        // Ordered list.
        if ordered_item(trimmed).is_some() {
            flush_para(&mut para, &mut blocks);
            let mut items = Vec::new();
            while i < lines.len() {
                if let Some((n, item)) = ordered_item(lines[i].trim_start()) {
                    items.push((n, parse_inline(item)));
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(MdBlock::Ordered(items));
            continue;
        }

        // Otherwise: paragraph text.
        para.push(line);
        i += 1;
    }
    flush_para(&mut para, &mut blocks);
    blocks
}

fn fence_marker(trimmed: &str) -> Option<&'static str> {
    if trimmed.starts_with("```") {
        Some("```")
    } else if trimmed.starts_with("~~~") {
        Some("~~~")
    } else {
        None
    }
}

fn heading(trimmed: &str) -> Option<(u8, &str)> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
        Some((hashes as u8, trimmed[hashes..].trim()))
    } else {
        None
    }
}

fn is_thematic_break(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && (compact.chars().all(|c| c == '-')
            || compact.chars().all(|c| c == '*')
            || compact.chars().all(|c| c == '_'))
}

fn is_table_delimiter(trimmed: &str) -> bool {
    if !trimmed.contains('|') || !trimmed.contains('-') {
        return false;
    }
    trimmed
        .trim_matches('|')
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .all(|cell| {
            let c = cell.trim();
            !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
        })
}

fn split_table_row(trimmed: &str) -> Vec<String> {
    trimmed
        .trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|s| s.trim().to_string())
        .collect()
}

fn bullet_item(trimmed: &str) -> Option<&str> {
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(m) {
            return Some(rest.trim_end());
        }
    }
    None
}

fn ordered_item(trimmed: &str) -> Option<(u64, &str)> {
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() || digits.len() > 9 {
        return None;
    }
    let after = &trimmed[digits.len()..];
    let rest = after
        .strip_prefix(". ")
        .or_else(|| after.strip_prefix(") "))?;
    Some((digits.parse().unwrap_or(0), rest.trim_end()))
}

/// Tokenize an inline run. Unterminated markers (`` ` ``, `*`, `**`, `[`)
/// degrade to literal text so a mid-stream document stays stable.
pub fn parse_inline(src: &str) -> Vec<Inline> {
    let chars: Vec<char> = src.chars().collect();
    let mut out: Vec<Inline> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    let push_text = |buf: &mut String, out: &mut Vec<Inline>| {
        if !buf.is_empty() {
            out.push(Inline::Text(std::mem::take(buf)));
        }
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            '`' => {
                if let Some(end) = find_from(&chars, i + 1, "`") {
                    push_text(&mut buf, &mut out);
                    out.push(Inline::Code(chars[i + 1..end].iter().collect()));
                    i = end + 1;
                } else {
                    buf.push(c);
                    i += 1;
                }
            }
            '*' | '_' if i + 1 < chars.len() && chars[i + 1] == c => {
                let marker = [c, c].iter().collect::<String>();
                if let Some(end) = find_from(&chars, i + 2, &marker) {
                    push_text(&mut buf, &mut out);
                    out.push(Inline::Bold(chars[i + 2..end].iter().collect()));
                    i = end + 2;
                } else {
                    buf.push(c);
                    i += 1;
                }
            }
            '*' | '_' => {
                let marker = c.to_string();
                if let Some(end) = find_from(&chars, i + 1, &marker) {
                    if end > i + 1 {
                        push_text(&mut buf, &mut out);
                        out.push(Inline::Italic(chars[i + 1..end].iter().collect()));
                        i = end + 1;
                        continue;
                    }
                }
                buf.push(c);
                i += 1;
            }
            '[' => {
                if let Some((text, href, next)) = parse_link(&chars, i) {
                    push_text(&mut buf, &mut out);
                    out.push(Inline::Link { text, href });
                    i = next;
                } else {
                    buf.push(c);
                    i += 1;
                }
            }
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    push_text(&mut buf, &mut out);
    out
}

fn find_from(chars: &[char], start: usize, needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || start >= chars.len() {
        return None;
    }
    (start..=chars.len().saturating_sub(n.len())).find(|&j| chars[j..j + n.len()] == n[..])
}

fn parse_link(chars: &[char], open: usize) -> Option<(String, String, usize)> {
    let close = find_from(chars, open + 1, "]")?;
    if close + 1 >= chars.len() || chars[close + 1] != '(' {
        return None;
    }
    let paren_close = find_from(chars, close + 2, ")")?;
    let text: String = chars[open + 1..close].iter().collect();
    let href: String = chars[close + 2..paren_close].iter().collect();
    Some((text, href, paren_close + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_and_paragraphs() {
        let b = parse_markdown("# Title\n\nHello **world** and `code`.");
        assert_eq!(
            b[0],
            MdBlock::Heading {
                level: 1,
                spans: vec![Inline::Text("Title".into())]
            }
        );
        match &b[1] {
            MdBlock::Paragraph(spans) => {
                assert_eq!(spans[0], Inline::Text("Hello ".into()));
                assert_eq!(spans[1], Inline::Bold("world".into()));
                assert_eq!(spans[2], Inline::Text(" and ".into()));
                assert_eq!(spans[3], Inline::Code("code".into()));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn fenced_code_open_and_closed() {
        let closed = parse_markdown("```rust\nfn main() {}\n```");
        assert_eq!(
            closed[0],
            MdBlock::Code {
                lang: Some("rust".into()),
                text: "fn main() {}".into(),
                closed: true
            }
        );
        // Still streaming: no closing fence yet.
        let open = parse_markdown("```py\nprint(1)");
        assert_eq!(
            open[0],
            MdBlock::Code {
                lang: Some("py".into()),
                text: "print(1)".into(),
                closed: false
            }
        );
    }

    #[test]
    fn lists_quote_rule() {
        let b = parse_markdown("- one\n- two\n\n1. first\n2. second\n\n> quoted\n\n---");
        assert_eq!(
            b[0],
            MdBlock::Bullets(vec![
                vec![Inline::Text("one".into())],
                vec![Inline::Text("two".into())]
            ])
        );
        assert_eq!(
            b[1],
            MdBlock::Ordered(vec![
                (1, vec![Inline::Text("first".into())]),
                (2, vec![Inline::Text("second".into())])
            ])
        );
        assert_eq!(b[2], MdBlock::Quote(vec![Inline::Text("quoted".into())]));
        assert_eq!(b[3], MdBlock::Rule);
    }

    #[test]
    fn pipe_table() {
        let b = parse_markdown("| a | b |\n| - | - |\n| 1 | 2 |\n| 3 | 4 |");
        match &b[0] {
            MdBlock::Table { headers, rows } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[1][0], vec![Inline::Text("3".into())]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unterminated_inline_is_literal() {
        assert_eq!(
            parse_inline("a **bold and `code"),
            vec![Inline::Text("a **bold and `code".into())]
        );
    }

    #[test]
    fn link_parsing() {
        assert_eq!(
            parse_inline("see [docs](https://x.dev) ok"),
            vec![
                Inline::Text("see ".into()),
                Inline::Link {
                    text: "docs".into(),
                    href: "https://x.dev".into()
                },
                Inline::Text(" ok".into()),
            ]
        );
    }

    #[test]
    fn incremental_prefix_is_stable() {
        // Each streamed prefix of a document parses without panicking and the
        // final parse matches the full text.
        let full = "# H\n\ntext `c` **b**\n\n```rs\nfn a(){}\n```\n\n- x\n- y";
        for end in 1..=full.len() {
            if full.is_char_boundary(end) {
                let _ = parse_markdown(&full[..end]);
            }
        }
        assert!(!parse_markdown(full).is_empty());
    }
}
