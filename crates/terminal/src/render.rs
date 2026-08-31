//! Render-preparation helpers for the GPUI terminal renderer (T03-002).
//!
//! Pure data transforms with no GPUI dependency so they stay unit-testable:
//!
//! * [`batch_runs`] collapses a [`RenderableScreen`]'s per-cell grid into
//!   horizontal runs of identical style — the single most important terminal
//!   rendering optimisation (one styled-text draw per run instead of per cell).
//! * [`grid_size`] turns an available pixel area + cell metrics into the largest
//!   whole `(columns, rows)` grid that fits, which the view feeds back to the
//!   engine / PTY on resize.

use alacritty_terminal::vte::ansi::Rgb;

use crate::engine::RenderableScreen;

/// The visual style shared by every cell in a [`StyledRun`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunStyle {
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    /// `HIDDEN` attribute — glyph present but painted in the background color.
    pub hidden: bool,
}

/// A maximal horizontal stretch of cells on one row that share [`RunStyle`] and
/// sit on contiguous columns.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledRun {
    /// Visible row, 0 = top.
    pub line: usize,
    /// Column of the run's first cell.
    pub start_col: usize,
    /// The run's text, one `char` per cell.
    pub text: String,
    pub style: RunStyle,
}

impl StyledRun {
    /// Number of cells (columns) the run occupies.
    pub fn width(&self) -> usize {
        self.text.chars().count()
    }
}

/// Collapse the grid in `screen` into per-row style runs, ordered by row then
/// column. A gap in the columns (a missing cell) always breaks a run so callers
/// can position each run by `start_col` alone.
pub fn batch_runs(screen: &RenderableScreen) -> Vec<StyledRun> {
    let mut per_line: Vec<Vec<&crate::engine::RenderableCell>> =
        vec![Vec::new(); screen.screen_lines];
    for cell in &screen.cells {
        if cell.line < screen.screen_lines {
            per_line[cell.line].push(cell);
        }
    }

    let mut runs = Vec::new();
    for (line, mut cells) in per_line.into_iter().enumerate() {
        cells.sort_by_key(|c| c.column);
        let mut current: Option<StyledRun> = None;
        let mut next_col = 0usize;
        for cell in cells {
            let style = RunStyle {
                fg: cell.fg,
                bg: cell.bg,
                bold: cell.bold,
                italic: cell.italic,
                underline: cell.underline,
                strikeout: cell.strikeout,
                hidden: cell.hidden,
            };
            let contiguous = cell.column == next_col;
            match current.as_mut() {
                Some(run) if contiguous && run.style == style => run.text.push(cell.c),
                _ => {
                    if let Some(run) = current.take() {
                        runs.push(run);
                    }
                    current = Some(StyledRun {
                        line,
                        start_col: cell.column,
                        text: cell.c.to_string(),
                        style,
                    });
                }
            }
            next_col = cell.column + 1;
        }
        if let Some(run) = current.take() {
            runs.push(run);
        }
    }
    runs
}

/// Largest whole `(columns, rows)` grid that fits `width_px` × `height_px` given
/// a cell of `cell_width` × `cell_height` pixels. Always at least `1 × 1`.
pub fn grid_size(
    width_px: f32,
    height_px: f32,
    cell_width: f32,
    cell_height: f32,
) -> (usize, usize) {
    let cols = if cell_width > 0.0 {
        (width_px / cell_width).floor() as usize
    } else {
        1
    };
    let rows = if cell_height > 0.0 {
        (height_px / cell_height).floor() as usize
    } else {
        1
    };
    (cols.max(1), rows.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{TermDimensions, TerminalEmulator};
    use crate::TerminalColors;
    use std::sync::mpsc::channel;

    fn emulator(cols: usize, rows: usize) -> TerminalEmulator {
        let (tx, _rx) = channel();
        let colors = TerminalColors::from_theme(&labonair_theme::Theme::dark());
        TerminalEmulator::new(colors, TermDimensions::new(cols, rows), tx)
    }

    #[test]
    fn identical_style_cells_merge_into_one_run() {
        let mut term = emulator(20, 3);
        term.feed(b"hello world");
        let runs = batch_runs(&term.render());
        // "hello world" plus the trailing blank cells share the default style,
        // so the whole row is a single run.
        let row0: Vec<_> = runs.iter().filter(|r| r.line == 0).collect();
        assert_eq!(row0.len(), 1);
        assert_eq!(row0[0].start_col, 0);
        assert!(row0[0].text.starts_with("hello world"));
        assert_eq!(row0[0].width(), 20);
    }

    #[test]
    fn a_style_change_starts_a_new_run_with_palette_colors() {
        let mut term = emulator(20, 3);
        term.feed(b"a\x1b[31mbb\x1b[0mc");
        let runs: Vec<_> = batch_runs(&term.render())
            .into_iter()
            .filter(|r| r.line == 0)
            .collect();
        // "a" | "bb" (red) | "c" + trailing blanks
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "a");
        assert_eq!(runs[1].text, "bb");
        assert!(runs[2].text.starts_with('c'));
        let palette = TerminalColors::from_theme(&labonair_theme::Theme::dark());
        assert_eq!(runs[1].style.fg, palette.normal[1]);
        assert_eq!(runs[0].style.fg, palette.foreground);
    }

    #[test]
    fn bold_attribute_breaks_a_run() {
        let mut term = emulator(20, 3);
        term.feed(b"n\x1b[1mB\x1b[0m");
        let runs: Vec<_> = batch_runs(&term.render())
            .into_iter()
            .filter(|r| r.line == 0)
            .collect();
        // "n" | "B" (bold) | trailing blanks (not bold)
        assert_eq!(runs.len(), 3);
        assert!(!runs[0].style.bold);
        assert!(runs[1].style.bold);
        assert_eq!(runs[1].text, "B");
        assert!(!runs[2].style.bold);
    }

    #[test]
    fn grid_size_is_floor_of_area_over_cell() {
        assert_eq!(grid_size(800.0, 600.0, 8.0, 16.0), (100, 37));
        // Never returns zero.
        assert_eq!(grid_size(1.0, 1.0, 8.0, 16.0), (1, 1));
        assert_eq!(grid_size(100.0, 100.0, 0.0, 0.0), (1, 1));
    }
}
