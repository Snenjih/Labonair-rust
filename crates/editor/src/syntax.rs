//! Tree-sitter syntax highlighting (T06-002).
//!
//! [`SyntaxHighlighter`] parses a document with the Tree-sitter grammar for its
//! [`Language`] and turns the syntax captures into flat, non-overlapping
//! [`HighlightSpan`]s (byte offsets into the `\n`-joined document text).
//!
//! Grammars are **lazy-loaded**: the [`tree_sitter_highlight::HighlightConfiguration`]
//! for a language is built on first use and then cached for the process
//! lifetime. Languages without a bundled grammar simply produce no spans (the
//! editor renders them in the default foreground colour).
//!
//! Highlighting is **viewport-bound**: [`SyntaxHighlighter::update`] only keeps
//! spans for the visible byte range (plus a margin), and stops walking the
//! Tree-sitter event stream once it is past that window — so typing in a large
//! file never pays for tokenising the whole document.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Mutex, OnceLock};

use tree_sitter_highlight::{
    Error as HighlightError, Highlight, HighlightConfiguration, HighlightEvent, Highlighter,
};

use crate::buffer::{BufferSnapshot, Revision};
use crate::language::Language;

/// A coarse token class that an editor theme maps to a colour. Deliberately
/// smaller than the raw Tree-sitter capture set — the value is the mapping,
/// not the granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    Keyword,
    Function,
    Macro,
    Type,
    Constructor,
    Namespace,
    String,
    Escape,
    Number,
    Boolean,
    Comment,
    Constant,
    Property,
    Variable,
    Parameter,
    Operator,
    Punctuation,
    Tag,
    Attribute,
    Label,
}

/// The Tree-sitter capture names we recognise. Passed to
/// [`HighlightConfiguration::configure`]; anything a grammar's `highlights.scm`
/// captures that is not in this list is ignored.
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "comment.documentation",
    "constant",
    "constant.builtin",
    "constructor",
    "escape",
    "function",
    "function.builtin",
    "function.call",
    "function.macro",
    "function.method",
    "keyword",
    "keyword.control",
    "keyword.function",
    "keyword.operator",
    "keyword.return",
    "label",
    "module",
    "namespace",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "string.regexp",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.member",
    "variable.parameter",
];

/// Map a (possibly dotted) Tree-sitter capture name to a [`HighlightKind`].
fn capture_kind(name: &str) -> Option<HighlightKind> {
    use HighlightKind::*;
    let base = name.split('.').next().unwrap_or(name);
    let kind = match name {
        "escape" | "string.escape" => Escape,
        "function.macro" => Macro,
        "variable.parameter" => Parameter,
        _ => match base {
            "attribute" => Attribute,
            "boolean" => Boolean,
            "comment" => Comment,
            "constant" => Constant,
            "constructor" => Constructor,
            "function" => Function,
            "keyword" => Keyword,
            "label" => Label,
            "module" | "namespace" => Namespace,
            "number" => Number,
            "operator" => Operator,
            "property" => Property,
            "punctuation" => Punctuation,
            "string" => String,
            "tag" => Tag,
            "type" => Type,
            "variable" => Variable,
            _ => return None,
        },
    };
    Some(kind)
}

/// A highlighted run of source, as byte offsets into the document text.
/// Non-overlapping and ordered by `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

/// One rendered piece of a single line: its text plus an optional token class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledRun {
    pub text: String,
    pub kind: Option<HighlightKind>,
}

/// Documents larger than this are not highlighted (avoids UI stalls on huge
/// generated files / minified bundles).
const MAX_HIGHLIGHT_BYTES: usize = 2 * 1024 * 1024;
/// Extra bytes kept on each side of the viewport so small scrolls re-use the
/// cached spans instead of re-parsing.
const WINDOW_MARGIN: usize = 32 * 1024;

/// Per-document highlighter. Holds the cached spans for the last parsed
/// (revision, viewport) pair.
pub struct SyntaxHighlighter {
    language: Language,
    revision: Revision,
    covered: Range<usize>,
    spans: Vec<HighlightSpan>,
}

impl SyntaxHighlighter {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            revision: Revision::new(u64::MAX),
            covered: 0..0,
            spans: Vec::new(),
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// Whether local syntax highlighting is available for the current language.
    pub fn has_grammar(&self) -> bool {
        self.language == Language::Markdown || config(self.language).is_some()
    }

    /// Switch language (e.g. after a file is loaded). Invalidates the cache.
    pub fn set_language(&mut self, language: Language) {
        if language != self.language {
            self.language = language;
            self.invalidate();
        }
    }

    /// Force a re-parse on the next [`Self::update`].
    pub fn invalidate(&mut self) {
        self.revision = Revision::new(u64::MAX);
        self.covered = 0..0;
        self.spans.clear();
    }

    /// Ensure the cached spans cover `visible` for a snapshot revision.
    ///
    /// The snapshot is the only source of document text. The temporary UTF-8
    /// view is created only when Tree-sitter needs contiguous source bytes;
    /// the highlighter never owns or mutates a second document buffer.
    pub fn update(&mut self, snapshot: &BufferSnapshot, visible: Range<usize>) {
        let revision = snapshot.revision();
        let text_len = snapshot.byte_len();
        let fresh = revision == self.revision;
        let covered =
            self.covered.start <= visible.start && self.covered.end >= visible.end.min(text_len);
        if fresh && covered && !self.spans.is_empty() {
            return;
        }
        if fresh && covered && self.covered.end >= text_len {
            // Whole document already covered and nothing changed.
            return;
        }

        self.revision = revision;
        self.spans.clear();
        self.covered = 0..0;

        if text_len == 0 || text_len > MAX_HIGHLIGHT_BYTES {
            return;
        }
        let text = snapshot.text();
        let win_start = visible.start.saturating_sub(WINDOW_MARGIN);
        let win_end = visible.end.saturating_add(WINDOW_MARGIN).min(text.len());

        if self.language == Language::Markdown {
            self.spans = markdown_highlights(&text, win_start..win_end);
            self.covered = win_start..win_end;
            return;
        }

        let Some(cfg) = config(self.language) else {
            return;
        };

        let mut highlighter = Highlighter::new();
        let events = match highlighter.highlight(cfg, text.as_bytes(), None, |_| None) {
            Ok(events) => events,
            Err(_) => return,
        };

        let mut stack: Vec<Option<HighlightKind>> = Vec::new();
        for event in events {
            let event: HighlightEvent = match event {
                Ok(event) => event,
                Err(HighlightError::Cancelled) => return,
                Err(_) => break,
            };
            match event {
                HighlightEvent::HighlightStart(Highlight(i)) => {
                    stack.push(HIGHLIGHT_NAMES.get(i).and_then(|n| capture_kind(n)));
                }
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    if start >= win_end {
                        break;
                    }
                    if end <= win_start {
                        continue;
                    }
                    if let Some(Some(kind)) = stack.last().copied() {
                        let s = start.max(win_start);
                        let e = end.min(win_end);
                        if s < e {
                            self.spans.push(HighlightSpan {
                                start: s,
                                end: e,
                                kind,
                            });
                        }
                    }
                }
            }
        }

        self.covered = win_start..win_end;
    }

    /// All cached spans (ordered, non-overlapping).
    pub fn spans(&self) -> &[HighlightSpan] {
        &self.spans
    }

    /// Split one line into styled runs.
    ///
    /// `line` is the raw line text; `line_start` is its start byte offset in the
    /// document text. Gaps between spans (and everything when no grammar is
    /// active) come back as runs with `kind: None`.
    pub fn line_runs(&self, snapshot: &BufferSnapshot, line: usize) -> Vec<StyledRun> {
        let content = snapshot.line(line);
        let line_start = snapshot.line_start_byte(line);
        let line_end = line_start + content.len();
        if self.spans.is_empty() {
            return vec![StyledRun {
                text: content,
                kind: None,
            }];
        }

        let mut runs = Vec::new();
        let mut at = line_start;
        for span in self
            .spans
            .iter()
            .filter(|s| s.end > line_start && s.start < line_end)
        {
            let s = span.start.max(line_start);
            let e = span.end.min(line_end);
            if s < at {
                // Overlap with a previous span — Tree-sitter output should not
                // produce these, but stay defensive.
                continue;
            }
            if s > at {
                runs.push(StyledRun {
                    text: content[at - line_start..s - line_start].to_string(),
                    kind: None,
                });
            }
            runs.push(StyledRun {
                text: content[s - line_start..e - line_start].to_string(),
                kind: Some(span.kind),
            });
            at = e;
        }
        if at < line_end {
            runs.push(StyledRun {
                text: content[at - line_start..].to_string(),
                kind: None,
            });
        }
        if runs.is_empty() {
            runs.push(StyledRun {
                text: content,
                kind: None,
            });
        }
        runs
    }
}

/// Lightweight Markdown highlighting without an additional grammar crate.
/// It covers common block and inline constructs while keeping byte ranges
/// compatible with the editor's existing syntax renderer.
fn markdown_highlights(text: &str, visible: Range<usize>) -> Vec<HighlightSpan> {
    use HighlightKind::{Comment, Keyword, Number, Punctuation, String, Type};

    let mut spans = Vec::new();
    let mut offset = 0usize;
    let mut fence: Option<(u8, usize)> = None;

    for source_line in text.split_inclusive('\n') {
        let line = source_line.strip_suffix('\n').unwrap_or(source_line);
        let line_end = offset + line.len();
        let leading = line.bytes().take_while(|byte| *byte == b' ').count();
        let content = &line[leading.min(3)..];

        if let Some((marker, width)) = fence {
            let closes = markdown_fence_width(content, marker).is_some_and(|count| count >= width)
                && content[markdown_fence_width(content, marker).unwrap_or(0)..]
                    .trim()
                    .is_empty();
            push_markdown_span(&mut spans, offset..line_end, String, &visible);
            if closes {
                fence = None;
            }
            offset += source_line.len();
            continue;
        }

        if leading <= 3 {
            if let Some((marker, width)) = markdown_fence(content) {
                let marker_end = offset + leading + width;
                push_markdown_span(
                    &mut spans,
                    offset + leading..marker_end,
                    Punctuation,
                    &visible,
                );
                if marker_end < line_end {
                    push_markdown_span(&mut spans, marker_end..line_end, Type, &visible);
                }
                fence = Some((marker, width));
                offset += source_line.len();
                continue;
            }
        }

        let trimmed = content.trim_start();
        let content_prefix = content.len() - trimmed.len();
        let base = offset + leading + content_prefix;
        let heading_width = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=6).contains(&heading_width)
            && trimmed
                .as_bytes()
                .get(heading_width)
                .is_none_or(|byte| byte.is_ascii_whitespace())
        {
            push_markdown_span(
                &mut spans,
                base..base + heading_width,
                Punctuation,
                &visible,
            );
            let body_start = base
                + heading_width
                + usize::from(trimmed.as_bytes().get(heading_width) == Some(&b' '));
            push_markdown_span(&mut spans, body_start..line_end, Keyword, &visible);
            offset += source_line.len();
            continue;
        }

        if trimmed.starts_with('>') {
            push_markdown_span(&mut spans, base..line_end, Comment, &visible);
            offset += source_line.len();
            continue;
        }

        if is_markdown_horizontal_rule(trimmed) {
            push_markdown_span(&mut spans, base..line_end, Punctuation, &visible);
            offset += source_line.len();
            continue;
        }

        let list_marker_end = markdown_list_marker_end(trimmed);
        if let Some(marker_end) = list_marker_end {
            push_markdown_span(
                &mut spans,
                base..base + marker_end,
                if trimmed.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    Number
                } else {
                    Punctuation
                },
                &visible,
            );
            markdown_inline_highlights(trimmed, marker_end, base, &visible, &mut spans);
        } else {
            markdown_inline_highlights(trimmed, 0, base, &visible, &mut spans);
        }
        offset += source_line.len();
    }

    spans.sort_by_key(|span| span.start);
    spans
}

fn push_markdown_span(
    spans: &mut Vec<HighlightSpan>,
    range: Range<usize>,
    kind: HighlightKind,
    visible: &Range<usize>,
) {
    let start = range.start.max(visible.start);
    let end = range.end.min(visible.end);
    if start < end {
        spans.push(HighlightSpan { start, end, kind });
    }
}

fn markdown_fence(line: &str) -> Option<(u8, usize)> {
    let marker = *line.as_bytes().first()?;
    markdown_fence_width(line, marker)
        .filter(|width| *width >= 3)
        .map(|width| (marker, width))
}

fn markdown_fence_width(line: &str, marker: u8) -> Option<usize> {
    let width = line.bytes().take_while(|byte| *byte == marker).count();
    (width > 0).then_some(width)
}

fn is_markdown_horizontal_rule(line: &str) -> bool {
    let compact = line
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    compact.len() >= 3
        && b"-*_"
            .iter()
            .copied()
            .any(|marker| compact.iter().all(|byte| *byte == marker))
}

fn markdown_list_marker_end(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    if bytes.len() >= 2 && matches!(bytes[0], b'-' | b'+' | b'*') && bytes[1].is_ascii_whitespace()
    {
        return Some(2);
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    (digits > 0
        && bytes
            .get(digits..digits + 2)
            .is_some_and(|suffix| suffix[0] == b'.' && suffix[1].is_ascii_whitespace()))
    .then_some(digits + 2)
}

fn markdown_inline_highlights(
    line: &str,
    mut cursor: usize,
    absolute_start: usize,
    visible: &Range<usize>,
    spans: &mut Vec<HighlightSpan>,
) {
    use HighlightKind::{Function, Operator, Punctuation, String, Type};
    let bytes = line.as_bytes();

    while cursor < bytes.len() {
        if bytes[cursor] == b'`' {
            let width = bytes[cursor..]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            let marker = vec![b'`'; width];
            if let Some(relative_end) = line.as_bytes()[cursor + width..]
                .windows(width)
                .position(|window| window == marker)
            {
                let end = cursor + width + relative_end + width;
                push_markdown_span(
                    spans,
                    absolute_start + cursor..absolute_start + end,
                    String,
                    visible,
                );
                cursor = end;
                continue;
            }
        }

        if bytes[cursor] == b'[' || (bytes[cursor] == b'!' && bytes.get(cursor + 1) == Some(&b'['))
        {
            let label_start = cursor + usize::from(bytes[cursor] == b'!') + 1;
            if let Some(close_label) = line[label_start..].find("](").map(|i| label_start + i) {
                if let Some(close_url) = line[close_label + 2..]
                    .find(')')
                    .map(|i| close_label + 2 + i)
                {
                    push_markdown_span(
                        spans,
                        absolute_start + cursor..absolute_start + label_start,
                        Punctuation,
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + label_start..absolute_start + close_label,
                        Function,
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + close_label..absolute_start + close_label + 2,
                        Punctuation,
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + close_label + 2..absolute_start + close_url,
                        String,
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + close_url..absolute_start + close_url + 1,
                        Punctuation,
                        visible,
                    );
                    cursor = close_url + 1;
                    continue;
                }
            }
        }

        let marker = if bytes[cursor..].starts_with(b"**")
            || bytes[cursor..].starts_with(b"__")
            || bytes[cursor..].starts_with(b"~~")
        {
            Some(&line[cursor..cursor + 2])
        } else if matches!(bytes[cursor], b'*' | b'_') {
            Some(&line[cursor..cursor + 1])
        } else {
            None
        };
        if let Some(marker) = marker {
            if let Some(relative_end) = line[cursor + marker.len()..].find(marker) {
                let content_start = cursor + marker.len();
                let content_end = content_start + relative_end;
                if content_end > content_start {
                    push_markdown_span(
                        spans,
                        absolute_start + cursor..absolute_start + content_start,
                        Punctuation,
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + content_start..absolute_start + content_end,
                        if marker == "~~" { Operator } else { Type },
                        visible,
                    );
                    push_markdown_span(
                        spans,
                        absolute_start + content_end..absolute_start + content_end + marker.len(),
                        Punctuation,
                        visible,
                    );
                    cursor = content_end + marker.len();
                    continue;
                }
            }
        }

        if line[cursor..].starts_with("https://") || line[cursor..].starts_with("http://") {
            let end = line[cursor..]
                .find(char::is_whitespace)
                .map_or(line.len(), |len| cursor + len);
            push_markdown_span(
                spans,
                absolute_start + cursor..absolute_start + end,
                Function,
                visible,
            );
            cursor = end;
            continue;
        }

        cursor += line[cursor..].chars().next().map_or(1, char::len_utf8);
    }
}

/// Lazily built, process-lifetime [`HighlightConfiguration`] for a language.
fn config(language: Language) -> Option<&'static HighlightConfiguration> {
    static REGISTRY: OnceLock<Mutex<HashMap<Language, Option<&'static HighlightConfiguration>>>> =
        OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = registry.lock().unwrap();
    *map.entry(language)
        .or_insert_with(|| build_config(language).map(|cfg| &*Box::leak(Box::new(cfg))))
}

fn build_config(language: Language) -> Option<HighlightConfiguration> {
    use tree_sitter::Language as TsLanguage;

    let (lang, name, highlights, injections, locals): (TsLanguage, &str, String, String, String) =
        match language {
            Language::Rust => (
                tree_sitter_rust::LANGUAGE.into(),
                "rust",
                tree_sitter_rust::HIGHLIGHTS_QUERY.to_string(),
                tree_sitter_rust::INJECTIONS_QUERY.to_string(),
                String::new(),
            ),
            Language::Json => (
                tree_sitter_json::LANGUAGE.into(),
                "json",
                tree_sitter_json::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Toml => (
                tree_sitter_toml_ng::LANGUAGE.into(),
                "toml",
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Yaml => (
                tree_sitter_yaml::LANGUAGE.into(),
                "yaml",
                tree_sitter_yaml::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Python => (
                tree_sitter_python::LANGUAGE.into(),
                "python",
                tree_sitter_python::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::JavaScript => (
                tree_sitter_javascript::LANGUAGE.into(),
                "javascript",
                tree_sitter_javascript::HIGHLIGHT_QUERY.to_string(),
                tree_sitter_javascript::INJECTIONS_QUERY.to_string(),
                tree_sitter_javascript::LOCALS_QUERY.to_string(),
            ),
            Language::TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "typescript",
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
                tree_sitter_javascript::INJECTIONS_QUERY.to_string(),
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::LOCALS_QUERY,
                    tree_sitter_typescript::LOCALS_QUERY
                ),
            ),
            Language::Go => (
                tree_sitter_go::LANGUAGE.into(),
                "go",
                tree_sitter_go::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::C => (
                tree_sitter_c::LANGUAGE.into(),
                "c",
                tree_sitter_c::HIGHLIGHT_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Cpp => (
                tree_sitter_cpp::LANGUAGE.into(),
                "cpp",
                format!(
                    "{}\n{}",
                    tree_sitter_c::HIGHLIGHT_QUERY,
                    tree_sitter_cpp::HIGHLIGHT_QUERY
                ),
                String::new(),
                String::new(),
            ),
            Language::Css => (
                tree_sitter_css::LANGUAGE.into(),
                "css",
                tree_sitter_css::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Html => (
                tree_sitter_html::LANGUAGE.into(),
                "html",
                tree_sitter_html::HIGHLIGHTS_QUERY.to_string(),
                tree_sitter_html::INJECTIONS_QUERY.to_string(),
                String::new(),
            ),
            Language::Java => (
                tree_sitter_java::LANGUAGE.into(),
                "java",
                tree_sitter_java::HIGHLIGHTS_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::Shell => (
                tree_sitter_bash::LANGUAGE.into(),
                "bash",
                tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
                String::new(),
                String::new(),
            ),
            Language::PlainText
            | Language::Markdown
            | Language::Sql
            | Language::Php
            | Language::Xml
            | Language::Ruby
            | Language::Swift
            | Language::Kotlin => return None,
        };

    let mut cfg =
        HighlightConfiguration::new(lang, name, &highlights, &injections, &locals).ok()?;
    cfg.configure(HIGHLIGHT_NAMES);
    Some(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lang: Language, src: &str) -> Vec<(HighlightKind, String)> {
        let mut hl = SyntaxHighlighter::new(lang);
        let buffer = crate::EditorBuffer::from_text(src);
        let snapshot = buffer.snapshot();
        hl.update(&snapshot, 0..src.len());
        hl.spans()
            .iter()
            .map(|s| (s.kind, src[s.start..s.end].to_string()))
            .collect()
    }

    #[test]
    fn rust_snippet_highlights_keyword_and_string() {
        let got = kinds(Language::Rust, "fn main() {\n    let s = \"hi\";\n}\n");
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::Keyword && t == "fn"));
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::Keyword && t == "let"));
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::String && t.contains("hi")));
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::Function && t == "main"));
    }

    #[test]
    fn python_snippet_highlights_comment_and_number() {
        let got = kinds(Language::Python, "# note\nx = 42\n");
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::Comment && t.contains("note")));
        assert!(got
            .iter()
            .any(|(k, t)| *k == HighlightKind::Number && t == "42"));
    }

    #[test]
    fn json_snippet_highlights_string_and_number() {
        let got = kinds(Language::Json, "{\"a\": 1, \"b\": \"c\"}");
        assert!(got.iter().any(|(k, _)| *k == HighlightKind::Number));
        assert!(got.iter().any(|(k, _)| *k == HighlightKind::String));
    }

    #[test]
    fn markdown_highlights_headings_fences_inline_code_links_and_lists() {
        let source = "# Overview\n\n- item with `code` and **bold** [link](https://example.test)\n\n```rust\nlet answer = 42;\n```\n";
        let got = kinds(Language::Markdown, source);
        assert!(got
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::Keyword && text == "Overview"));
        assert!(got
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::String && text == "`code`"));
        assert!(got
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::Function && text == "link"));
        assert!(got
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::String && text == "https://example.test"));
        assert!(got
            .iter()
            .any(|(kind, text)| *kind == HighlightKind::String && text == "let answer = 42;"));
    }

    #[test]
    fn plain_text_and_unsupported_produce_no_spans() {
        let mut hl = SyntaxHighlighter::new(Language::PlainText);
        let buffer = crate::EditorBuffer::from_text("just words\n");
        let snapshot = buffer.snapshot();
        hl.update(&snapshot, 0..11);
        assert!(hl.spans().is_empty());
        assert!(!hl.has_grammar());
        assert!(SyntaxHighlighter::new(Language::Rust).has_grammar());
        assert!(SyntaxHighlighter::new(Language::Markdown).has_grammar());
    }

    #[test]
    fn line_runs_partition_the_line_exactly() {
        let src = "let x = 1;\n";
        let mut hl = SyntaxHighlighter::new(Language::Rust);
        let buffer = crate::EditorBuffer::from_text(src);
        let snapshot = buffer.snapshot();
        hl.update(&snapshot, 0..src.len());
        let runs = hl.line_runs(&snapshot, 0);
        let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(joined, "let x = 1;");
        assert!(runs.iter().any(|r| r.kind == Some(HighlightKind::Keyword)));
    }

    #[test]
    fn viewport_bound_skips_offscreen_tokens() {
        let mut src = String::from("fn a() {}\n");
        src.push_str(&"// filler line\n".repeat(4000));
        src.push_str("fn zzz() {}\n");
        let mut hl = SyntaxHighlighter::new(Language::Rust);
        let buffer = crate::EditorBuffer::from_text(&src);
        let snapshot = buffer.snapshot();
        // Only look at the first 200 bytes.
        hl.update(&snapshot, 0..200);
        let last = src.rfind("zzz").unwrap();
        assert!(
            hl.spans().iter().all(|s| s.start < last),
            "no spans should be produced for the far-offscreen tail"
        );
        assert!(hl.spans().iter().any(|s| s.kind == HighlightKind::Function));
    }

    #[test]
    fn revision_cache_is_reused() {
        let src = "fn a() {}\n";
        let mut hl = SyntaxHighlighter::new(Language::Rust);
        let buffer = crate::EditorBuffer::from_text(src);
        let snapshot = buffer.snapshot();
        hl.update(&snapshot, 0..src.len());
        let snapshot = hl.spans().to_vec();
        let buffer_snapshot = buffer.snapshot();
        hl.update(&buffer_snapshot, 0..src.len());
        assert_eq!(hl.spans(), snapshot.as_slice());
    }
}
