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
    revision: u64,
    covered: Range<usize>,
    spans: Vec<HighlightSpan>,
}

impl SyntaxHighlighter {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            revision: u64::MAX,
            covered: 0..0,
            spans: Vec::new(),
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    /// Whether a Tree-sitter grammar is bundled for the current language.
    pub fn has_grammar(&self) -> bool {
        config(self.language).is_some()
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
        self.revision = u64::MAX;
        self.covered = 0..0;
        self.spans.clear();
    }

    /// Ensure the cached spans cover `visible` for document `text` at `revision`.
    ///
    /// `text` must be the exact `\n`-joined document contents; `visible` is a
    /// byte range into it. Cheap no-op when nothing relevant changed.
    pub fn update(&mut self, text: &str, revision: u64, visible: Range<usize>) {
        let fresh = revision == self.revision;
        let covered =
            self.covered.start <= visible.start && self.covered.end >= visible.end.min(text.len());
        if fresh && covered && !self.spans.is_empty() {
            return;
        }
        if fresh && covered && self.covered.end >= text.len() {
            // Whole document already covered and nothing changed.
            return;
        }

        self.revision = revision;
        self.spans.clear();
        self.covered = 0..0;

        if text.is_empty() || text.len() > MAX_HIGHLIGHT_BYTES {
            return;
        }
        let Some(cfg) = config(self.language) else {
            return;
        };

        let win_start = visible.start.saturating_sub(WINDOW_MARGIN);
        let win_end = visible.end.saturating_add(WINDOW_MARGIN).min(text.len());

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
    pub fn line_runs(&self, line: &str, line_start: usize) -> Vec<StyledRun> {
        let line_end = line_start + line.len();
        if self.spans.is_empty() {
            return vec![StyledRun {
                text: line.to_string(),
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
                    text: line[at - line_start..s - line_start].to_string(),
                    kind: None,
                });
            }
            runs.push(StyledRun {
                text: line[s - line_start..e - line_start].to_string(),
                kind: Some(span.kind),
            });
            at = e;
        }
        if at < line_end {
            runs.push(StyledRun {
                text: line[at - line_start..].to_string(),
                kind: None,
            });
        }
        if runs.is_empty() {
            runs.push(StyledRun {
                text: line.to_string(),
                kind: None,
            });
        }
        runs
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
        hl.update(src, 1, 0..src.len());
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
    fn plain_text_and_unsupported_produce_no_spans() {
        let mut hl = SyntaxHighlighter::new(Language::PlainText);
        hl.update("just words\n", 1, 0..11);
        assert!(hl.spans().is_empty());
        assert!(!hl.has_grammar());
        assert!(SyntaxHighlighter::new(Language::Rust).has_grammar());
    }

    #[test]
    fn line_runs_partition_the_line_exactly() {
        let src = "let x = 1;\n";
        let mut hl = SyntaxHighlighter::new(Language::Rust);
        hl.update(src, 1, 0..src.len());
        let runs = hl.line_runs("let x = 1;", 0);
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
        // Only look at the first 200 bytes.
        hl.update(&src, 1, 0..200);
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
        hl.update(src, 7, 0..src.len());
        let snapshot = hl.spans().to_vec();
        hl.update(src, 7, 0..src.len());
        assert_eq!(hl.spans(), snapshot.as_slice());
    }
}
