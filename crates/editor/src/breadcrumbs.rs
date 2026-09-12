//! UI-free data and state helpers for the Editor breadcrumb row.

use std::path::Path;

use crate::lifecycle::FileState;
use crate::{DocumentSymbol, SymbolKind};

/// A path label with a bounded display form and its unabridged tooltip value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreadcrumbPath {
    pub label: String,
    pub full_path: String,
}

/// The symbol currently containing, or immediately preceding, the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BreadcrumbSymbol {
    pub name: String,
    pub kind: SymbolKind,
    /// Zero-based definition line used by Editor navigation.
    pub line: usize,
}

/// The minimal truthful state the breadcrumb surface can show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BreadcrumbState {
    Empty,
    Loading,
    Missing(Option<BreadcrumbPath>),
    Error(Option<BreadcrumbPath>),
    Ready {
        path: Option<BreadcrumbPath>,
        symbol: Option<BreadcrumbSymbol>,
    },
}

/// Build the breadcrumb state without depending on a view or renderer.
///
/// Non-ready file states intentionally do not expose symbols: the buffer is
/// either not loaded or its contents are not trustworthy for navigation.
pub fn build_breadcrumb_state(
    file_state: &FileState,
    path: Option<&Path>,
    symbols: &[DocumentSymbol],
    cursor_line: usize,
    max_path_chars: usize,
) -> BreadcrumbState {
    let path = breadcrumb_path(path, max_path_chars);
    match file_state {
        FileState::New => BreadcrumbState::Empty,
        FileState::Loading => BreadcrumbState::Loading,
        FileState::Missing => BreadcrumbState::Missing(path),
        FileState::Error(_) => BreadcrumbState::Error(path),
        FileState::Clean
        | FileState::Dirty
        | FileState::Conflict
        | FileState::ReloadNeeded
        | FileState::ReadOnly => BreadcrumbState::Ready {
            path,
            symbol: current_symbol(symbols, cursor_line),
        },
        FileState::Binary | FileState::TooLarge => BreadcrumbState::Ready { path, symbol: None },
    }
}

fn breadcrumb_path(path: Option<&Path>, max_path_chars: usize) -> Option<BreadcrumbPath> {
    let full_path = path?.to_string_lossy().into_owned();
    if full_path.is_empty() {
        return None;
    }
    Some(BreadcrumbPath {
        label: truncate_middle(&full_path, max_path_chars),
        full_path,
    })
}

fn current_symbol(symbols: &[DocumentSymbol], cursor_line: usize) -> Option<BreadcrumbSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.line <= cursor_line)
        .max_by_key(|symbol| (symbol.line, symbol.range.start))
        .map(|symbol| BreadcrumbSymbol {
            name: symbol.name.clone(),
            kind: symbol.kind,
            line: symbol.line,
        })
}

/// Keep both ends of a path visible so the filename remains identifiable.
pub fn truncate_middle(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }

    let prefix_len = (max_chars - 1) / 2;
    let suffix_len = max_chars - 1 - prefix_len;
    let prefix: String = value.chars().take(prefix_len).collect();
    let suffix: String = value
        .chars()
        .skip(count.saturating_sub(suffix_len))
        .collect();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditorBuffer, Language, TextRange};

    fn symbol(name: &str, line: usize, start: usize) -> DocumentSymbol {
        DocumentSymbol {
            name: name.to_string(),
            kind: SymbolKind::Function,
            line,
            range: TextRange::new(start, start + 4),
            name_range: TextRange::new(start, start + name.len()),
        }
    }

    #[test]
    fn ready_state_keeps_path_and_nearest_symbol() {
        let symbols = [symbol("outer", 2, 10), symbol("inner", 8, 40)];
        let state = build_breadcrumb_state(
            &FileState::Clean,
            Some(Path::new("src/editor.rs")),
            &symbols,
            9,
            40,
        );
        assert_eq!(
            state,
            BreadcrumbState::Ready {
                path: Some(BreadcrumbPath {
                    label: "src/editor.rs".into(),
                    full_path: "src/editor.rs".into(),
                }),
                symbol: Some(BreadcrumbSymbol {
                    name: "inner".into(),
                    kind: SymbolKind::Function,
                    line: 8,
                }),
            }
        );
    }

    #[test]
    fn empty_and_loading_states_do_not_invent_navigation_data() {
        let symbols = [symbol("main", 0, 0)];
        assert_eq!(
            build_breadcrumb_state(&FileState::New, None, &symbols, 0, 40),
            BreadcrumbState::Empty
        );
        assert_eq!(
            build_breadcrumb_state(
                &FileState::Loading,
                Some(Path::new("src/main.rs")),
                &symbols,
                0,
                40,
            ),
            BreadcrumbState::Loading
        );
    }

    #[test]
    fn missing_and_error_states_retain_only_truthful_path() {
        let path = Some(Path::new("missing.rs"));
        let symbols = [symbol("stale", 0, 0)];
        assert!(matches!(
            build_breadcrumb_state(&FileState::Missing, path, &symbols, 0, 40),
            BreadcrumbState::Missing(Some(_))
        ));
        assert!(matches!(
            build_breadcrumb_state(
                &FileState::Error("read failed".into()),
                path,
                &symbols,
                0,
                40,
            ),
            BreadcrumbState::Error(Some(_))
        ));
    }

    #[test]
    fn long_paths_are_middle_truncated_but_full_path_is_preserved() {
        let state = build_breadcrumb_state(
            &FileState::Clean,
            Some(Path::new("workspace/very/long/source/filename.rs")),
            &[],
            0,
            18,
        );
        let BreadcrumbState::Ready {
            path: Some(path), ..
        } = state
        else {
            panic!("expected ready path");
        };
        assert_eq!(path.label.chars().count(), 18);
        assert!(path.label.contains('…'));
        assert_eq!(path.full_path, "workspace/very/long/source/filename.rs");
    }

    #[test]
    fn unsupported_language_symbol_input_can_be_empty() {
        let buffer = EditorBuffer::from_text("plain text");
        let symbols = crate::document_symbols(Language::PlainText, &buffer.snapshot());
        assert!(symbols.is_empty());
        assert!(matches!(
            build_breadcrumb_state(
                &FileState::Clean,
                Some(Path::new("notes.txt")),
                &symbols,
                0,
                40,
            ),
            BreadcrumbState::Ready {
                path: Some(_),
                symbol: None,
            }
        ));
    }

    #[test]
    fn binary_and_too_large_states_hide_symbol_context() {
        let symbols = [symbol("stale", 0, 0)];
        for state in [FileState::Binary, FileState::TooLarge] {
            assert!(matches!(
                build_breadcrumb_state(&state, Some(Path::new("data.bin")), &symbols, 0, 40,),
                BreadcrumbState::Ready {
                    path: Some(_),
                    symbol: None,
                }
            ));
        }
    }
}
