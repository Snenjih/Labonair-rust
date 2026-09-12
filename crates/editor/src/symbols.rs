//! Document-symbol extraction (palette "Go to Symbol" / outline).
//!
//! Parses the buffer with the raw Tree-sitter grammar and runs the grammar's
//! bundled `tags.scm` query (`TAGS_QUERY`) to pull out every definition
//! (function / method / class / type / module / constant) with its name and
//! line. Languages without a tags query return an empty list.

use tree_sitter::{Language as TsLanguage, Parser, Query, QueryCursor, StreamingIterator};

use crate::{BufferSnapshot, Language, TextRange};

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

fn grammar(lang: Language) -> Option<(TsLanguage, &'static str)> {
    Some(match lang {
        Language::Rust => (
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::TAGS_QUERY,
        ),
        Language::Python => (
            tree_sitter_python::LANGUAGE.into(),
            tree_sitter_python::TAGS_QUERY,
        ),
        Language::JavaScript => (
            tree_sitter_javascript::LANGUAGE.into(),
            tree_sitter_javascript::TAGS_QUERY,
        ),
        Language::TypeScript => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            tree_sitter_typescript::TAGS_QUERY,
        ),
        Language::Go => (tree_sitter_go::LANGUAGE.into(), tree_sitter_go::TAGS_QUERY),
        Language::C => (tree_sitter_c::LANGUAGE.into(), tree_sitter_c::TAGS_QUERY),
        Language::Cpp => (
            tree_sitter_cpp::LANGUAGE.into(),
            tree_sitter_cpp::TAGS_QUERY,
        ),
        Language::Java => (
            tree_sitter_java::LANGUAGE.into(),
            tree_sitter_java::TAGS_QUERY,
        ),
        _ => return None,
    })
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
pub fn document_symbols(lang: Language, snapshot: &BufferSnapshot) -> Vec<DocumentSymbol> {
    let Some((ts_lang, tags)) = grammar(lang) else {
        return Vec::new();
    };
    if snapshot.byte_len() == 0 {
        return Vec::new();
    }
    let text = snapshot.text();
    let mut parser = Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&text, None) else {
        return Vec::new();
    };
    let Ok(query) = Query::new(&ts_lang, tags) else {
        return Vec::new();
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
    out
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
}
