//! Editor-owned display mapping.
//!
//! A display snapshot maps one immutable buffer revision to visual rows. It
//! stores coordinates and source ranges, never a second copy of the document.
//! The workspace renderer can therefore paint only the visible rows while the
//! editor keeps wrapping, folding, gutters, and scroll invariants together.

use crate::language_services::{Diagnostic, SemanticToken};
use crate::{BufferSnapshot, Position, Revision, TextRange};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayDiagnostic {
    pub visual_row: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub severity: crate::language_services::DiagnosticSeverity,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplaySemanticToken {
    pub visual_row: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub token_type: String,
    pub modifiers: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhitespaceMode {
    None,
    Boundary,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayConfig {
    /// `None` keeps one visual row per logical line. `Some(0)` is normalized
    /// to `None`; positive values are Unicode-scalar columns.
    pub wrap_columns: Option<usize>,
    pub show_line_numbers: bool,
    pub relative_line_numbers: bool,
    pub indentation_guides: bool,
    pub whitespace: WhitespaceMode,
    pub scroll_beyond_last_line: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            wrap_columns: None,
            show_line_numbers: true,
            relative_line_numbers: false,
            indentation_guides: true,
            whitespace: WhitespaceMode::None,
            scroll_beyond_last_line: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub top_row: usize,
    pub left_column: usize,
    pub width_columns: usize,
    pub height_rows: usize,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            top_row: 0,
            left_column: 0,
            width_columns: 80,
            height_rows: 40,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoldRange {
    pub start_line: usize,
    pub end_line: usize,
}

impl FoldRange {
    pub fn new(start_line: usize, end_line: usize) -> Option<Self> {
        (start_line < end_line).then_some(Self {
            start_line,
            end_line,
        })
    }

    pub fn contains_line(self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayRowKind {
    Text {
        logical_line: usize,
        start_column: usize,
        end_column: usize,
    },
    FoldPlaceholder {
        start_line: usize,
        end_line: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayRow {
    pub visual_row: usize,
    pub kind: DisplayRowKind,
    pub line_number: Option<usize>,
    pub relative_line_number: Option<usize>,
    pub indent_guides: Vec<usize>,
}

impl DisplayRow {
    pub fn logical_line(&self) -> usize {
        match self.kind {
            DisplayRowKind::Text { logical_line, .. }
            | DisplayRowKind::FoldPlaceholder {
                start_line: logical_line,
                ..
            } => logical_line,
        }
    }

    pub fn source_range(&self, snapshot: &BufferSnapshot) -> Option<TextRange> {
        match self.kind {
            DisplayRowKind::Text {
                logical_line,
                start_column,
                end_column,
            } => Some(TextRange::new(
                snapshot.position_to_byte(Position::new(logical_line, start_column)),
                snapshot.position_to_byte(Position::new(logical_line, end_column)),
            )),
            DisplayRowKind::FoldPlaceholder { .. } => None,
        }
    }

    pub fn text(&self, snapshot: &BufferSnapshot) -> String {
        match self.kind {
            DisplayRowKind::Text {
                logical_line,
                start_column,
                end_column,
            } => snapshot
                .line(logical_line)
                .chars()
                .skip(start_column)
                .take(end_column.saturating_sub(start_column))
                .collect(),
            DisplayRowKind::FoldPlaceholder {
                start_line,
                end_line,
            } => {
                format!("… {} lines folded", end_line - start_line)
            }
        }
    }

    pub fn position_at_column(&self, column: usize) -> Position {
        match self.kind {
            DisplayRowKind::Text {
                logical_line,
                start_column,
                end_column,
            } => Position::new(logical_line, (start_column + column).min(end_column)),
            DisplayRowKind::FoldPlaceholder { start_line, .. } => Position::new(start_line, 0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WhitespaceToken {
    pub range: TextRange,
    pub marker: char,
}

#[derive(Clone, Debug)]
pub struct DisplaySnapshot {
    pub revision: Revision,
    pub viewport: Viewport,
    pub rows: Vec<DisplayRow>,
    pub total_rows: usize,
    pub max_line_width: usize,
    line_starts: Vec<usize>,
    config: DisplayConfig,
}

impl DisplaySnapshot {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn max_scroll_row(&self) -> usize {
        self.total_rows
            .saturating_sub(self.viewport.height_rows.max(1))
    }

    pub fn logical_line_to_visual_row(&self, line: usize) -> usize {
        self.line_starts
            .get(line.min(self.line_starts.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0)
    }

    pub fn position_to_display(&self, position: Position) -> Option<(usize, usize)> {
        let first_row = *self.line_starts.get(position.line)?;
        let display = self.row_for_visual(first_row)?;
        let (row, column) = match display.kind {
            DisplayRowKind::Text {
                start_column: _,
                end_column: _,
                ..
            } => {
                let width = self.config.wrap_columns.filter(|width| *width > 0);
                let segment = width.map_or(0, |width| position.column / width);
                let row = first_row + segment;
                let display = self.row_for_visual(row)?;
                let (start, end) = match display.kind {
                    DisplayRowKind::Text {
                        start_column,
                        end_column,
                        ..
                    } => (start_column, end_column),
                    DisplayRowKind::FoldPlaceholder { .. } => (0, 0),
                };
                (
                    row,
                    position.column.clamp(start, end) - start - self.viewport.left_column,
                )
            }
            DisplayRowKind::FoldPlaceholder { .. } => (first_row, 0),
        };
        Some((row, column))
    }

    pub fn display_to_position(&self, visual_row: usize, column: usize) -> Option<Position> {
        self.row_for_visual(visual_row)
            .map(|row| row.position_at_column(column + self.viewport.left_column))
    }

    /// Project editor-owned diagnostics onto the current display map. Results
    /// from an older revision must be discarded by the language-service
    /// state before reaching this method.
    pub fn diagnostic_decorations(
        &self,
        snapshot: &BufferSnapshot,
        diagnostics: &[Diagnostic],
    ) -> Vec<DisplayDiagnostic> {
        diagnostics
            .iter()
            .filter_map(|diagnostic| {
                let range = diagnostic.range.to_text_range(snapshot);
                let start = self.position_to_display(snapshot.byte_to_position(range.start))?;
                let end = self.position_to_display(snapshot.byte_to_position(range.end))?;
                (start.0 == end.0 && end.1 >= start.1).then_some(DisplayDiagnostic {
                    visual_row: start.0,
                    start_column: start.1,
                    end_column: end.1.max(start.1 + 1),
                    severity: diagnostic.severity,
                    message: diagnostic.message.clone(),
                })
            })
            .collect()
    }

    /// Project semantic token ranges into visual rows for rendering. Tokens
    /// spanning a wrapped row are clipped to the row rather than duplicated.
    pub fn semantic_token_decorations(
        &self,
        snapshot: &BufferSnapshot,
        tokens: &[SemanticToken],
    ) -> Vec<DisplaySemanticToken> {
        tokens
            .iter()
            .filter_map(|token| {
                let range = token.range.to_text_range(snapshot);
                let start = self.position_to_display(snapshot.byte_to_position(range.start))?;
                let end = self.position_to_display(snapshot.byte_to_position(range.end))?;
                (start.0 == end.0 && end.1 > start.1).then_some(DisplaySemanticToken {
                    visual_row: start.0,
                    start_column: start.1,
                    end_column: end.1,
                    token_type: token.token_type.clone(),
                    modifiers: token.modifiers.clone(),
                })
            })
            .collect()
    }

    pub fn whitespace_tokens(
        &self,
        snapshot: &BufferSnapshot,
        row: &DisplayRow,
    ) -> Vec<WhitespaceToken> {
        if self.config.whitespace == WhitespaceMode::None {
            return Vec::new();
        }
        let DisplayRowKind::Text {
            logical_line,
            start_column,
            end_column,
        } = row.kind
        else {
            return Vec::new();
        };
        let line = snapshot.line(logical_line);
        let chars: Vec<char> = line.chars().collect();
        let mut tokens = Vec::new();
        for index in start_column..end_column.min(chars.len()) {
            let ch = chars[index];
            let boundary = index == 0
                || index + 1 == chars.len()
                || !chars[index.saturating_sub(1)].is_whitespace()
                || !chars[index + 1.min(chars.len().saturating_sub(1))].is_whitespace();
            if !ch.is_whitespace()
                || (self.config.whitespace == WhitespaceMode::Boundary && !boundary)
            {
                continue;
            }
            let byte = snapshot.position_to_byte(Position::new(logical_line, index));
            let end = snapshot.position_to_byte(Position::new(logical_line, index + 1));
            tokens.push(WhitespaceToken {
                range: TextRange::new(byte, end),
                marker: if ch == '\t' { '→' } else { '·' },
            });
        }
        tokens
    }

    fn row_for_visual(&self, visual_row: usize) -> Option<&DisplayRow> {
        self.rows.iter().find(|row| row.visual_row == visual_row)
    }
}

pub struct DisplayMap;

impl DisplayMap {
    pub fn build(
        snapshot: &BufferSnapshot,
        viewport: Viewport,
        config: DisplayConfig,
        folds: &[FoldRange],
    ) -> DisplaySnapshot {
        let wrap = config.wrap_columns.filter(|columns| *columns > 0);
        let mut all_rows = Vec::new();
        let mut line_starts = vec![0; snapshot.line_count()];
        let mut max_line_width = 0;
        let mut line = 0;
        while line < snapshot.line_count() {
            line_starts[line] = all_rows.len();
            let fold = folds
                .iter()
                .filter(|fold| fold.start_line == line && fold.end_line < snapshot.line_count())
                .max_by_key(|fold| fold.end_line);
            if let Some(fold) = fold {
                all_rows.push(DisplayRow {
                    visual_row: all_rows.len(),
                    kind: DisplayRowKind::FoldPlaceholder {
                        start_line: line,
                        end_line: fold.end_line,
                    },
                    line_number: line_number(config, line, line),
                    relative_line_number: None,
                    indent_guides: Vec::new(),
                });
                for start in line_starts
                    .iter_mut()
                    .take(fold.end_line + 1)
                    .skip(line + 1)
                {
                    *start = all_rows.len() - 1;
                }
                line = fold.end_line + 1;
                continue;
            }
            let length = snapshot.line_len(line);
            max_line_width = max_line_width.max(length);
            let rows = wrap.map_or(1, |columns| length.max(1).div_ceil(columns));
            for segment in 0..rows {
                let start = wrap.map_or(0, |columns| segment * columns);
                let end = wrap.map_or(length, |columns| (start + columns).min(length));
                all_rows.push(DisplayRow {
                    visual_row: all_rows.len(),
                    kind: DisplayRowKind::Text {
                        logical_line: line,
                        start_column: start,
                        end_column: end,
                    },
                    line_number: line_number(config, line, line),
                    relative_line_number: None,
                    indent_guides: if config.indentation_guides && segment == 0 {
                        indentation_guides(snapshot, line)
                    } else {
                        Vec::new()
                    },
                });
            }
            line += 1;
        }

        let current_line = viewport
            .top_row
            .min(snapshot.line_count().saturating_sub(1));
        for row in &mut all_rows {
            if config.relative_line_numbers {
                row.relative_line_number = Some(row.logical_line().abs_diff(current_line));
            }
        }
        let total_rows = all_rows.len().max(1);
        let start = viewport.top_row.min(total_rows.saturating_sub(1));
        let end = start.saturating_add(viewport.height_rows.max(1));
        let rows = all_rows
            .into_iter()
            .filter(|row| row.visual_row >= start && row.visual_row < end)
            .collect();
        DisplaySnapshot {
            revision: snapshot.revision(),
            viewport,
            rows,
            total_rows,
            max_line_width,
            line_starts,
            config,
        }
    }
}

fn line_number(config: DisplayConfig, line: usize, current_line: usize) -> Option<usize> {
    config
        .show_line_numbers
        .then_some(if config.relative_line_numbers {
            line.abs_diff(current_line)
        } else {
            line + 1
        })
}

fn indentation_guides(snapshot: &BufferSnapshot, line: usize) -> Vec<usize> {
    let text = snapshot.line(line);
    let leading = text.chars().take_while(|ch| ch.is_whitespace()).count();
    (1..=leading).filter(|column| column % 2 == 0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_services::LocalLanguageService;

    fn snapshot(text: &str) -> BufferSnapshot {
        crate::EditorBuffer::from_text(text).snapshot()
    }

    #[test]
    fn maps_wrapped_unicode_rows_without_copying_document_text() {
        let source = snapshot("äbcdef\nsecond");
        let display = DisplayMap::build(
            &source,
            Viewport {
                height_rows: 8,
                ..Default::default()
            },
            DisplayConfig {
                wrap_columns: Some(3),
                ..Default::default()
            },
            &[],
        );
        assert_eq!(display.total_rows, 4);
        assert_eq!(display.display_to_position(1, 0), Some(Position::new(0, 3)));
        assert_eq!(
            display.position_to_display(Position::new(0, 4)),
            Some((1, 1))
        );
        assert_eq!(display.rows[0].text(&source), "äbc");
    }

    #[test]
    fn folds_hide_inner_lines_and_map_to_placeholder() {
        let source = snapshot("one\ntwo\nthree\nfour");
        let fold = FoldRange::new(0, 2).unwrap();
        let display = DisplayMap::build(
            &source,
            Viewport::default(),
            DisplayConfig::default(),
            &[fold],
        );
        assert_eq!(display.total_rows, 2);
        assert!(matches!(
            display.rows[0].kind,
            DisplayRowKind::FoldPlaceholder { .. }
        ));
        assert_eq!(
            display.position_to_display(Position::new(1, 0)),
            Some((0, 0))
        );
        assert_eq!(display.display_to_position(0, 0), Some(Position::new(0, 0)));
    }

    #[test]
    fn scroll_is_clamped_and_line_numbers_are_relative() {
        let source = snapshot("a\nb\nc");
        let display = DisplayMap::build(
            &source,
            Viewport {
                top_row: 99,
                height_rows: 2,
                ..Default::default()
            },
            DisplayConfig {
                relative_line_numbers: true,
                ..Default::default()
            },
            &[],
        );
        assert_eq!(display.rows.len(), 1);
        assert_eq!(display.rows[0].relative_line_number, Some(0));
    }

    #[test]
    fn whitespace_tokens_use_snapshot_byte_ranges() {
        let source = snapshot("  λ\t");
        let display = DisplayMap::build(
            &source,
            Viewport::default(),
            DisplayConfig {
                whitespace: WhitespaceMode::All,
                ..Default::default()
            },
            &[],
        );
        let tokens = display.whitespace_tokens(&source, &display.rows[0]);
        assert_eq!(tokens.len(), 3);
        assert_eq!(source.text_range(tokens[0].range), " ");
        assert_eq!(tokens[2].marker, '→');
    }

    #[test]
    fn language_service_ranges_are_consumed_by_display_mapping() {
        let source = snapshot("fn main() {}");
        let display =
            DisplayMap::build(&source, Viewport::default(), DisplayConfig::default(), &[]);
        let service = crate::SyntaxLanguageService::for_language(crate::Language::Rust);
        let result = service
            .request(
                &source,
                crate::DocumentVersion::new(1),
                crate::LanguageServiceRequest::Diagnostics,
                &crate::CancellationToken::default(),
            )
            .unwrap();
        let crate::LanguageServiceResponse::Diagnostics(diagnostics) = result else {
            panic!("wrong response")
        };
        assert!(display
            .diagnostic_decorations(&source, &diagnostics)
            .is_empty());

        let tokens = service
            .request(
                &source,
                crate::DocumentVersion::new(1),
                crate::LanguageServiceRequest::SemanticTokens,
                &crate::CancellationToken::default(),
            )
            .unwrap();
        let crate::LanguageServiceResponse::SemanticTokens(tokens) = tokens else {
            panic!("wrong response")
        };
        assert!(!display
            .semantic_token_decorations(&source, &tokens)
            .is_empty());
    }
}
