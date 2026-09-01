//! Labonair code editor core (T06-*).
//!
//! Framework-free editing model: a line [`buffer::TextBuffer`], a
//! [`document::Document`] wrapping it with caret / selection / undo history /
//! dirty-baseline tracking, literal [`search`] find-replace, and lightweight
//! [`language`] identification. The GPUI view that renders and drives this
//! lives in `labonair-ui` (`editor.rs`). Syntax highlighting (T06-002), vim
//! mode (T06-003) and diff view (T06-004) build on top.

pub mod buffer;
pub mod document;
pub mod history;
pub mod language;
pub mod search;
pub mod syntax;
pub mod vim;

pub use buffer::{Position, TextBuffer};
pub use document::{Document, Motion};
pub use language::Language;
pub use search::{find_all, next_match, replace_all, Match, SearchQuery};
pub use syntax::{HighlightKind, HighlightSpan, StyledRun, SyntaxHighlighter};
pub use vim::{Vim, VimKey, VimMode, VimOptions, VimResponse};
