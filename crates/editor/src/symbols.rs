//! Document-symbol extraction (palette "Go to Symbol" / outline).
//!
//! Parses the buffer with the raw Tree-sitter grammar and runs the grammar's
//! bundled `tags.scm` query (`TAGS_QUERY`) to pull out every definition
//! (function / method / class / type / module / constant) with its name and
//! line. Languages without a tags query return an empty list.

use tree_sitter::{Language as TsLanguage, Parser, Query, QueryCursor, StreamingIterator};

use crate::{BufferSnapshot, FoldRange, Language, TextRange};

/// A coarse LSP-ish symbol classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Type,
    Module,
    Interface,
    Constant,
    Other,
}

impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Type => "type",
            SymbolKind::Module => "module",
            SymbolKind::Interface => "interface",
            SymbolKind::Constant => "constant",
            SymbolKind::Other => "symbol",
        }
    }
}

/// One extracted symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: SymbolKind,
    /// 0-based line of the definition.
    pub line: usize,
    /// Byte range of the defining syntax node in the source snapshot.
    pub range: TextRange,
    /// Byte range of the symbol's name in the source snapshot.
    pub name_range: TextRange,
}

/// Cached structure extracted from one immutable document revision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentSyntax {
    pub symbols: Vec<DocumentSymbol>,
    pub folding_ranges: Vec<FoldRange>,
}

fn tree_sitter_language(lang: Language) -> Option<TsLanguage> {
    Some(match lang {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Json => tree_sitter_json::LANGUAGE.into(),
        Language::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
        Language::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Css => tree_sitter_css::LANGUAGE.into(),
        Language::Html => tree_sitter_html::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::Shell => tree_sitter_bash::LANGUAGE.into(),
        _ => return None,
    })
}

fn tags_query(lang: Language) -> Option<&'static str> {
    Some(match lang {
        Language::Rust => tree_sitter_rust::TAGS_QUERY,
        Language::Python => tree_sitter_python::TAGS_QUERY,
        Language::JavaScript => tree_sitter_javascript::TAGS_QUERY,
        Language::TypeScript => tree_sitter_typescript::TAGS_QUERY,
        Language::Go => tree_sitter_go::TAGS_QUERY,
        Language::C => tree_sitter_c::TAGS_QUERY,
        Language::Cpp => tree_sitter_cpp::TAGS_QUERY,
        Language::Java => tree_sitter_java::TAGS_QUERY,
        _ => return None,
    })
}

/// Return the next larger named syntax node for each selection range.
///
/// Tree-sitter exposes byte ranges, so this helper uses the same UTF-8 byte
/// offsets as editor anchors and selections. Languages without a bundled
/// grammar leave the caller's word/bracket/line fallback intact. One parse is
/// shared by every selection so multi-cursor expansion does not reparse once
/// per caret.
pub(crate) fn larger_syntax_ranges(
    lang: Language,
    snapshot: &BufferSnapshot,
    ranges: &[TextRange],
) -> Vec<Option<TextRange>> {
    let Some(tree) = syntax_tree(lang, snapshot) else {
        return vec![None; ranges.len()];
    };
    let root = tree.root_node();
    ranges
        .iter()
        .map(|range| larger_syntax_range(root, *range, snapshot.byte_len()))
        .collect()
}

fn larger_syntax_range(
    root: tree_sitter::Node<'_>,
    range: TextRange,
    byte_len: usize,
) -> Option<TextRange> {
    if range.end > byte_len {
        return None;
    }
    let mut node = root.descendant_for_byte_range(range.start, range.end)?;
    let mut candidate: Option<TextRange> = None;
    loop {
        let node_range = TextRange::new(node.start_byte(), node.end_byte());
        if node.is_named()
            && contains_range(node_range, range)
            && node_range.len() > range.len()
            && candidate.is_none_or(|current| node_range.len() < current.len())
        {
            candidate = Some(node_range);
        }
        let Some(parent) = node.parent() else {
            break;
        };
        node = parent;
    }
    candidate
}

/// Return the largest named syntax child fully contained in each selection.
///
/// As with [`larger_syntax_ranges`], parsing is shared across all selections
/// in the document.
pub(crate) fn smaller_syntax_ranges(
    lang: Language,
    snapshot: &BufferSnapshot,
    ranges: &[TextRange],
) -> Vec<Option<TextRange>> {
    let Some(tree) = syntax_tree(lang, snapshot) else {
        return vec![None; ranges.len()];
    };
    let root = tree.root_node();
    ranges
        .iter()
        .map(|range| smaller_syntax_range(root, *range, snapshot.byte_len()))
        .collect()
}

fn smaller_syntax_range(
    root: tree_sitter::Node<'_>,
    range: TextRange,
    byte_len: usize,
) -> Option<TextRange> {
    if range.is_empty() || range.end > byte_len {
        return None;
    }
    let mut node = root.descendant_for_byte_range(range.start, range.end)?;
    while !contains_range(TextRange::new(node.start_byte(), node.end_byte()), range) {
        node = node.parent()?;
    }

    let mut candidates = Vec::new();
    collect_contained_named_nodes(node, range, &mut candidates);
    candidates.sort_by(|left, right| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left.start.cmp(&right.start))
    });
    candidates.into_iter().next()
}

fn syntax_tree(lang: Language, snapshot: &BufferSnapshot) -> Option<tree_sitter::Tree> {
    if snapshot.byte_len() == 0 {
        return None;
    }
    let language = tree_sitter_language(lang)?;
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    parser.parse(snapshot.text(), None)
}

fn contains_range(outer: TextRange, inner: TextRange) -> bool {
    outer.start <= inner.start && outer.end >= inner.end
}

fn collect_contained_named_nodes(
    node: tree_sitter::Node<'_>,
    range: TextRange,
    candidates: &mut Vec<TextRange>,
) {
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        let child_range = TextRange::new(child.start_byte(), child.end_byte());
        if child_range.start >= range.start && child_range.end <= range.end {
            if child_range.len() < range.len() {
                candidates.push(child_range);
            }
            collect_contained_named_nodes(child, range, candidates);
        }
    }
}

fn classify(def_capture: &str) -> SymbolKind {
    match def_capture {
        "function" => SymbolKind::Function,
        "method" => SymbolKind::Method,
        "class" => SymbolKind::Class,
        "type" | "struct" | "enum" | "union" => SymbolKind::Type,
        "module" | "namespace" | "package" => SymbolKind::Module,
        "interface" | "trait" => SymbolKind::Interface,
        "constant" => SymbolKind::Constant,
        _ => SymbolKind::Other,
    }
}

/// Extract every top-level and nested definition from a read-only snapshot.
///
/// The returned byte ranges are valid for `snapshot.revision()` and remain
/// independent of later edits because the snapshot owns that immutable
/// revision. Tree-sitter receives one temporary contiguous view of the
/// snapshot; this module does not retain a second text source.
pub fn document_syntax(lang: Language, snapshot: &BufferSnapshot) -> DocumentSyntax {
    let Some(ts_lang) = tree_sitter_language(lang) else {
        return DocumentSyntax::default();
    };
    if snapshot.byte_len() == 0 {
        return DocumentSyntax::default();
    }
    let text = snapshot.text();
    let mut parser = Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return DocumentSyntax::default();
    }
    let Some(tree) = parser.parse(&text, None) else {
        return DocumentSyntax::default();
    };
    let mut folding_ranges = Vec::new();
    collect_syntax_folding_ranges(tree.root_node(), &mut folding_ranges);
    folding_ranges.sort_by_key(|range| (range.start_line, range.end_line));
    folding_ranges.dedup();

    let Some(tags) = tags_query(lang) else {
        return DocumentSyntax {
            symbols: Vec::new(),
            folding_ranges,
        };
    };
    let Ok(query) = Query::new(&ts_lang, tags) else {
        return DocumentSyntax {
            symbols: Vec::new(),
            folding_ranges,
        };
    };
    let names = query.capture_names();
    let bytes = text.as_bytes();
    let mut cursor = QueryCursor::new();
    let mut out: Vec<DocumentSymbol> = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        let mut kind: Option<SymbolKind> = None;
        let mut name: Option<String> = None;
        let mut line = 0usize;
        let mut range = None;
        let mut name_range = None;
        for cap in m.captures {
            let cname = names[cap.index as usize];
            if let Some(rest) = cname.strip_prefix("definition.") {
                kind = Some(classify(rest));
                line = cap.node.start_position().row;
                range = Some(TextRange::new(cap.node.start_byte(), cap.node.end_byte()));
            } else if cname == "name" {
                name = cap.node.utf8_text(bytes).ok().map(str::to_string);
                name_range = Some(TextRange::new(cap.node.start_byte(), cap.node.end_byte()));
            }
        }
        if let (Some(kind), Some(name), Some(range), Some(name_range)) =
            (kind, name, range, name_range)
        {
            if !name.is_empty() {
                out.push(DocumentSymbol {
                    name,
                    kind,
                    line,
                    range,
                    name_range,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then_with(|| a.name.cmp(&b.name))
    });
    out.dedup_by(|a, b| a.range == b.range && a.name == b.name);
    DocumentSyntax {
        symbols: out,
        folding_ranges,
    }
}

/// Extract document symbols using the bundled Tree-sitter grammar.
pub fn document_symbols(lang: Language, snapshot: &BufferSnapshot) -> Vec<DocumentSymbol> {
    document_syntax(lang, snapshot).symbols
}

fn collect_syntax_folding_ranges(node: tree_sitter::Node<'_>, ranges: &mut Vec<FoldRange>) {
    for index in 0..node.named_child_count() {
        let Some(child) = node.named_child(index) else {
            continue;
        };
        let start_line = child.start_position().row;
        let end_position = child.end_position();
        let mut end_line = end_position.row;
        if end_position.column == 0 && end_line > start_line {
            end_line -= 1;
        }
        if let Some(range) = FoldRange::new(start_line, end_line) {
            ranges.push(range);
        }
        collect_syntax_folding_ranges(child, ranges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_symbols() {
        let src =
            "fn alpha() {}\n\nstruct Beta { x: u32 }\n\nimpl Beta {\n    fn gamma(&self) {}\n}\n";
        let buffer = crate::EditorBuffer::from_text(src);
        let syms = document_symbols(Language::Rust, &buffer.snapshot());
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"Beta"));
        assert!(names.contains(&"gamma"));
        let alpha = syms.iter().find(|s| s.name == "alpha").unwrap();
        assert_eq!(alpha.line, 0);
        assert_eq!(alpha.kind, SymbolKind::Function);
        assert_eq!(alpha.range, TextRange::new(0, 13));
        assert_eq!(alpha.name_range, TextRange::new(3, 8));
    }

    #[test]
    fn unsupported_language_is_empty() {
        let plain = crate::EditorBuffer::from_text("hello");
        let toml = crate::EditorBuffer::from_text("[x]\na = 1");
        assert!(document_symbols(Language::PlainText, &plain.snapshot()).is_empty());
        assert!(document_symbols(Language::Toml, &toml.snapshot()).is_empty());
    }

    #[test]
    fn extracts_python_symbols() {
        let src = "def top():\n    pass\n\nclass Widget:\n    def method(self):\n        pass\n";
        let buffer = crate::EditorBuffer::from_text(src);
        let syms = document_symbols(Language::Python, &buffer.snapshot());
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"top"));
        assert!(names.contains(&"Widget"));
        assert!(names.contains(&"method"));
    }

    #[test]
    fn empty_snapshot_has_no_symbols() {
        let buffer = crate::EditorBuffer::default();
        assert!(document_symbols(Language::Rust, &buffer.snapshot()).is_empty());
    }

    #[test]
    fn rust_syntax_selection_grows_from_expression_to_parent_and_shrinks() {
        let source = "fn main() { let answer = add(1 + 2, 3); }";
        let buffer = crate::EditorBuffer::from_text(source);
        let snapshot = buffer.snapshot();
        let one_start = source.find("1 + 2").expect("expression in source");
        let one = TextRange::new(one_start, one_start + 1);
        let binary = TextRange::new(one_start, one_start + "1 + 2".len());

        assert_eq!(
            larger_syntax_ranges(Language::Rust, &snapshot, &[one]),
            vec![Some(binary)]
        );
        assert_eq!(
            smaller_syntax_ranges(Language::Rust, &snapshot, &[binary]),
            vec![Some(one)]
        );
    }

    #[test]
    fn syntax_selection_supports_highlight_grammars_without_symbol_queries() {
        let source = "{\"answer\": 42}";
        let buffer = crate::EditorBuffer::from_text(source);
        let snapshot = buffer.snapshot();
        let number = TextRange::new(
            source.find("42").expect("number in source"),
            source.len() - 1,
        );

        assert_eq!(
            larger_syntax_ranges(Language::Json, &snapshot, &[number]),
            vec![Some(TextRange::new(1, source.len() - 1))]
        );
        assert!(document_symbols(Language::Json, &snapshot).is_empty());
    }

    #[test]
    fn rust_syntax_folding_includes_nested_multiline_blocks() {
        let source = "fn outer() {\n    if ready {\n        work();\n    }\n}\n";
        let buffer = crate::EditorBuffer::from_text(source);
        let syntax = document_syntax(Language::Rust, &buffer.snapshot());

        assert!(syntax.folding_ranges.contains(&FoldRange {
            start_line: 0,
            end_line: 4,
        }));
        assert!(syntax.folding_ranges.contains(&FoldRange {
            start_line: 1,
            end_line: 3,
        }));
    }

    #[test]
    fn json_syntax_folding_works_without_a_symbol_query() {
        let source = "{\n  \"nested\": {\n    \"value\": 1\n  }\n}\n";
        let buffer = crate::EditorBuffer::from_text(source);
        let syntax = document_syntax(Language::Json, &buffer.snapshot());

        assert!(syntax.symbols.is_empty());
        assert!(syntax.folding_ranges.contains(&FoldRange {
            start_line: 0,
            end_line: 4,
        }));
        assert!(syntax.folding_ranges.contains(&FoldRange {
            start_line: 1,
            end_line: 3,
        }));
    }
}
