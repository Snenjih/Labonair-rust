//! Terminal emulation core (T03-001).
//!
//! Wraps [`alacritty_terminal::Term`] + its VTE parser into a small, render-free
//! unit. Bytes go in via [`TerminalEmulator::feed`]; a structured snapshot of the
//! visible grid comes out via [`TerminalEmulator::render`]. No PTY, no threads,
//! no GPUI here — that lives in [`crate::session`] and the renderer task
//! (T03-002).
//!
//! Cell colors are resolved through the theme palette ([`TerminalColors`],
//! T02-004): every [`alacritty_terminal::vte::ansi::Color`] on a cell becomes a
//! concrete [`Rgb`] taken from the active theme, never an Alacritty default.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event as AlacEvent, EventListener, WindowSize};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, CursorShape, NamedColor, Processor, Rgb};

use crate::TerminalColors;

/// Default scrollback depth (lines kept above the viewport). Mirrors the
/// reference app's xterm `scrollback` default.
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

/// Terminal grid dimensions in cells, plus the pixel size of the text area
/// (needed by some programs via `TIOCGWINSZ` / `CSI 14 t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermDimensions {
    pub columns: usize,
    pub screen_lines: usize,
    pub cell_width: u16,
    pub cell_height: u16,
}

impl TermDimensions {
    /// Build from a cell grid size, using a nominal cell pixel size.
    pub fn new(columns: usize, screen_lines: usize) -> Self {
        Self {
            columns: columns.max(1),
            screen_lines: screen_lines.max(1),
            cell_width: 8,
            cell_height: 16,
        }
    }

    /// The `alacritty_terminal` window-size view of these dimensions.
    pub fn window_size(&self) -> WindowSize {
        WindowSize {
            num_lines: self.screen_lines as u16,
            num_cols: self.columns as u16,
            cell_width: self.cell_width.max(1),
            cell_height: self.cell_height.max(1),
        }
    }
}

impl Default for TermDimensions {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// A change the emulation layer surfaces to the UI. Delivered over a channel so
/// the PTY I/O thread never touches UI state directly (see [`crate::session`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    /// Grid content changed — the renderer should repaint.
    Wakeup,
    /// Window title set via OSC 0/2.
    Title(String),
    /// Title reset to the default.
    ResetTitle,
    /// Working directory reported via OSC 7 (`file://host/path`).
    Cwd(String),
    /// Terminal bell.
    Bell,
    /// The emulator requested a shutdown.
    Exit,
    /// Child process exited with this status code.
    ChildExit(i32),
    /// The mouse cursor shape may need updating.
    MouseCursorDirty,
}

/// [`EventListener`] that forwards [`alacritty_terminal`] events onto an
/// [`mpsc`](std::sync::mpsc) channel as [`TerminalEvent`]s. Cloneable and
/// `Send` so it can live inside a `Term` that moves between threads.
#[derive(Clone)]
pub struct EventProxy {
    tx: Sender<TerminalEvent>,
    /// Bytes the emulator wants written back to the PTY (DA/DSR replies). The
    /// session's I/O thread drains this via [`TerminalEmulator::take_pty_output`].
    pty_out: Arc<Mutex<Vec<u8>>>,
}

impl EventProxy {
    fn new(tx: Sender<TerminalEvent>, pty_out: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { tx, pty_out }
    }
}

impl EventListener for EventProxy {
    fn send_event(&self, event: AlacEvent) {
        let mapped = match event {
            AlacEvent::Wakeup => TerminalEvent::Wakeup,
            AlacEvent::Title(t) => TerminalEvent::Title(t),
            AlacEvent::ResetTitle => TerminalEvent::ResetTitle,
            AlacEvent::Bell => TerminalEvent::Bell,
            AlacEvent::Exit => TerminalEvent::Exit,
            AlacEvent::ChildExit(code) => TerminalEvent::ChildExit(code),
            AlacEvent::PtyWrite(text) => {
                if let Ok(mut buf) = self.pty_out.lock() {
                    buf.extend_from_slice(text.as_bytes());
                }
                let _ = self.tx.send(TerminalEvent::Wakeup);
                return;
            }
            AlacEvent::MouseCursorDirty => TerminalEvent::MouseCursorDirty,
            // Not needed by the current renderer / backend logic.
            AlacEvent::CursorBlinkingChange
            | AlacEvent::ClipboardStore(..)
            | AlacEvent::ClipboardLoad(..)
            | AlacEvent::ColorRequest(..)
            | AlacEvent::TextAreaSizeRequest(_) => return,
        };
        let _ = self.tx.send(mapped);
    }
}

/// One resolved cell ready for the GPUI renderer (T03-002).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderableCell {
    /// Viewport line (0 = top visible row).
    pub line: usize,
    pub column: usize,
    pub c: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub inverse: bool,
    pub dim: bool,
    pub hidden: bool,
}

/// Cursor state for the renderable snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderableCursor {
    pub line: i32,
    pub column: usize,
    pub shape: CursorShape,
}

/// A horizontal stretch of selected cells on one visible row (end exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionSpan {
    pub line: usize,
    pub start_col: usize,
    pub end_col: usize,
}

/// Snapshot of the terminal modes that change how keyboard and mouse input is
/// encoded (see [`crate::input`]). Pulled from the `alacritty_terminal` engine
/// via [`TerminalEmulator::mode_state`] and passed into the input mappers so the
/// generated escape sequences match what the running program expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModeState {
    /// DECCKM — cursor keys send `ESC O x` instead of `ESC [ x`.
    pub app_cursor: bool,
    /// DECKPAM — application keypad mode.
    pub app_keypad: bool,
    /// Bracketed paste (DEC 2004) — pasted text is wrapped in `ESC[200~`/`ESC[201~`.
    pub bracketed_paste: bool,
    /// Insert mode (IRM).
    pub insert: bool,
    /// Alternate screen buffer is active (vim, less, htop, …).
    pub alt_screen: bool,
    /// DEC 1007 — wheel events become arrow keys on the alternate screen.
    pub alternate_scroll: bool,
    /// DEC 1000 — report button press/release.
    pub mouse_report_click: bool,
    /// DEC 1002 — also report motion while a button is held.
    pub mouse_drag: bool,
    /// DEC 1003 — report all motion.
    pub mouse_motion: bool,
    /// DEC 1006 — SGR extended mouse encoding.
    pub sgr_mouse: bool,
    /// DEC 1005 — UTF-8 extended mouse encoding.
    pub utf8_mouse: bool,
    /// Any Kitty keyboard protocol flag is enabled.
    pub kitty_keyboard: bool,
    /// Kitty "report all keys as escape codes".
    pub report_all_keys_as_esc: bool,
}

impl ModeState {
    /// `true` when any mouse-reporting mode is active, i.e. wheel/click events
    /// must be sent to the program rather than driving scrollback/selection.
    pub fn mouse_reporting(&self) -> bool {
        self.mouse_report_click || self.mouse_drag || self.mouse_motion
    }
}

/// An immutable snapshot of the visible terminal, produced by
/// [`TerminalEmulator::render`] and consumed by the renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderableScreen {
    pub columns: usize,
    pub screen_lines: usize,
    /// How many lines the viewport is scrolled up into history (0 = bottom).
    pub display_offset: usize,
    pub cursor: RenderableCursor,
    pub cells: Vec<RenderableCell>,
    /// Highlighted selection, split per visible row (empty when nothing is
    /// selected or the selection lies entirely in hidden scrollback).
    pub selection: Vec<SelectionSpan>,
}

impl RenderableScreen {
    /// The visible grid rendered as plain text, one row per line (trailing
    /// blanks trimmed). Used by tests and the headless dump.
    pub fn to_text(&self) -> String {
        let mut rows = vec![String::new(); self.screen_lines];
        for cell in &self.cells {
            if let Some(row) = rows.get_mut(cell.line) {
                while row.chars().count() < cell.column {
                    row.push(' ');
                }
                if cell.column == row.chars().count() {
                    row.push(cell.c);
                }
            }
        }
        rows.iter()
            .map(|r| r.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Resolve an ANSI cell color against the theme palette.
fn resolve_color(color: AnsiColor, colors: &TerminalColors, dim: bool) -> Rgb {
    match color {
        AnsiColor::Spec(rgb) => rgb,
        AnsiColor::Indexed(i) => {
            if dim && i < 8 {
                colors.dim[i as usize]
            } else {
                colors.ansi256(i)
            }
        }
        AnsiColor::Named(named) => match named {
            NamedColor::Foreground => {
                if dim {
                    colors.dim_foreground
                } else {
                    colors.foreground
                }
            }
            NamedColor::Background => colors.background,
            NamedColor::Cursor => colors.cursor,
            NamedColor::BrightForeground => colors.bright_foreground,
            NamedColor::DimForeground => colors.dim_foreground,
            NamedColor::Black
            | NamedColor::Red
            | NamedColor::Green
            | NamedColor::Yellow
            | NamedColor::Blue
            | NamedColor::Magenta
            | NamedColor::Cyan
            | NamedColor::White => {
                let idx = named as usize;
                if dim {
                    colors.dim[idx]
                } else {
                    colors.normal[idx]
                }
            }
            NamedColor::BrightBlack
            | NamedColor::BrightRed
            | NamedColor::BrightGreen
            | NamedColor::BrightYellow
            | NamedColor::BrightBlue
            | NamedColor::BrightMagenta
            | NamedColor::BrightCyan
            | NamedColor::BrightWhite => {
                colors.bright[named as usize - NamedColor::BrightBlack as usize]
            }
            NamedColor::DimBlack
            | NamedColor::DimRed
            | NamedColor::DimGreen
            | NamedColor::DimYellow
            | NamedColor::DimBlue
            | NamedColor::DimMagenta
            | NamedColor::DimCyan
            | NamedColor::DimWhite => colors.dim[named as usize - NamedColor::DimBlack as usize],
        },
    }
}

/// OSC 7 (working directory) sniffer that runs on the raw byte stream before it
/// reaches the VTE parser — `alacritty_terminal` 0.24 ignores OSC 7, so the CWD
/// event has to be recovered here. Full shell-integration (OSC 133) is T03-004.
#[derive(Default)]
struct OscSniffer {
    /// Buffered bytes once we've seen `ESC ]`, until the terminator.
    buf: Vec<u8>,
    in_osc: bool,
    saw_esc: bool,
}

impl OscSniffer {
    fn feed(&mut self, bytes: &[u8], out: &mut Vec<TerminalEvent>) {
        for &b in bytes {
            if self.in_osc {
                // Terminator: BEL (0x07) or ST (ESC \).
                if b == 0x07 {
                    self.finish(out);
                } else if self.saw_esc && b == b'\\' {
                    self.buf.pop(); // drop the trailing ESC
                    self.finish(out);
                } else {
                    self.saw_esc = b == 0x1b;
                    self.buf.push(b);
                    if self.buf.len() > 4096 {
                        self.reset();
                    }
                }
                continue;
            }
            if self.saw_esc && b == b']' {
                self.in_osc = true;
                self.saw_esc = false;
                self.buf.clear();
            } else {
                self.saw_esc = b == 0x1b;
            }
        }
    }

    fn finish(&mut self, out: &mut Vec<TerminalEvent>) {
        let payload = std::mem::take(&mut self.buf);
        self.reset();
        // OSC 7 ; file://host/path
        if let Some(rest) = payload.strip_prefix(b"7;") {
            if let Ok(s) = std::str::from_utf8(rest) {
                let path = s
                    .strip_prefix("file://")
                    .and_then(|u| u.split_once('/').map(|(_, p)| format!("/{p}")))
                    .unwrap_or_else(|| s.to_string());
                if !path.is_empty() {
                    out.push(TerminalEvent::Cwd(path));
                }
            }
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.in_osc = false;
        self.saw_esc = false;
    }
}

/// Render-free terminal emulator: VTE parser + grid + scrollback.
pub struct TerminalEmulator {
    term: Term<EventProxy>,
    parser: Processor,
    colors: TerminalColors,
    dimensions: TermDimensions,
    sniffer: OscSniffer,
    pty_out: Arc<Mutex<Vec<u8>>>,
}

impl TerminalEmulator {
    /// Create an emulator with the given palette and initial size. Events go to
    /// `event_tx`.
    pub fn new(
        colors: TerminalColors,
        dimensions: TermDimensions,
        event_tx: Sender<TerminalEvent>,
    ) -> Self {
        let config = Config {
            scrolling_history: DEFAULT_SCROLLBACK_LINES,
            ..Config::default()
        };
        let pty_out = Arc::new(Mutex::new(Vec::new()));
        let proxy = EventProxy::new(event_tx, Arc::clone(&pty_out));
        let term = Term::new(config, &dimensions, proxy);
        Self {
            term,
            parser: Processor::new(),
            colors,
            dimensions,
            sniffer: OscSniffer::default(),
            pty_out,
        }
    }

    /// Take any bytes the emulator queued to be written back to the PTY.
    pub fn take_pty_output(&mut self) -> Vec<u8> {
        match self.pty_out.lock() {
            Ok(mut buf) => std::mem::take(&mut *buf),
            Err(_) => Vec::new(),
        }
    }

    /// Feed raw PTY output through the parser. Returns events that must reach the
    /// UI: a trailing [`TerminalEvent::Wakeup`] whenever bytes were processed
    /// (`alacritty_terminal`'s `Term` never emits `Wakeup` itself — that was the
    /// job of Alacritty's event loop), plus anything the raw-stream sniffer
    /// recovered (OSC 7). Other `Term` events arrive on the channel from
    /// [`Self::new`].
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TerminalEvent> {
        if bytes.is_empty() {
            return Vec::new();
        }
        let mut extra = Vec::new();
        self.sniffer.feed(bytes, &mut extra);
        for &byte in bytes {
            self.parser.advance(&mut self.term, byte);
        }
        extra.push(TerminalEvent::Wakeup);
        extra
    }

    /// Swap in a new theme palette (e.g. on light/dark switch).
    pub fn set_colors(&mut self, colors: TerminalColors) {
        self.colors = colors;
    }

    /// Current grid size in cells.
    pub fn dimensions(&self) -> TermDimensions {
        self.dimensions
    }

    /// Resize the grid. The caller is responsible for resizing the PTY too
    /// ([`crate::session::TerminalSession::resize`] does both).
    pub fn resize(&mut self, dimensions: TermDimensions) {
        self.dimensions = dimensions;
        self.term.resize(dimensions);
    }

    /// Scroll the viewport within the scrollback buffer.
    pub fn scroll(&mut self, scroll: Scroll) {
        self.term.scroll_display(scroll);
    }

    /// Set a simple (linewise-free) text selection between two grid points,
    /// given as `(line, column)` where `line` is relative to the visible top
    /// (negative = scrollback). Used by the mouse mapping (T03-003); exposed
    /// here so the renderer can already draw selections.
    pub fn set_selection(&mut self, start: (i32, usize), end: (i32, usize)) {
        use alacritty_terminal::index::{Column, Line, Point as GridIndex, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};

        let anchor = GridIndex::new(Line(start.0), Column(start.1));
        let head = GridIndex::new(Line(end.0), Column(end.1));
        let mut selection = Selection::new(SelectionType::Simple, anchor, Side::Left);
        selection.update(head, Side::Right);
        self.term.selection = Some(selection);
    }

    /// Clear any active selection.
    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    /// Snapshot of the input-relevant terminal modes.
    pub fn mode_state(&self) -> ModeState {
        let m = *self.term.mode();
        ModeState {
            app_cursor: m.contains(TermMode::APP_CURSOR),
            app_keypad: m.contains(TermMode::APP_KEYPAD),
            bracketed_paste: m.contains(TermMode::BRACKETED_PASTE),
            insert: m.contains(TermMode::INSERT),
            alt_screen: m.contains(TermMode::ALT_SCREEN),
            alternate_scroll: m.contains(TermMode::ALTERNATE_SCROLL),
            mouse_report_click: m.contains(TermMode::MOUSE_REPORT_CLICK),
            mouse_drag: m.contains(TermMode::MOUSE_DRAG),
            mouse_motion: m.contains(TermMode::MOUSE_MOTION),
            sgr_mouse: m.contains(TermMode::SGR_MOUSE),
            utf8_mouse: m.contains(TermMode::UTF8_MOUSE),
            kitty_keyboard: m.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
            report_all_keys_as_esc: m.contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
        }
    }

    /// Begin or extend a simple text selection using **viewport** cell
    /// coordinates `(column, row)` where row 0 is the top visible line. The
    /// current scrollback offset is folded in so the selection stays anchored to
    /// the buffer content while the user scrolls. `anchor` is the fixed corner
    /// (mouse-down cell), `head` the moving corner (current mouse cell).
    pub fn update_selection_viewport(&mut self, anchor: (usize, usize), head: (usize, usize)) {
        use alacritty_terminal::index::{Column, Line, Point as GridIndex, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};

        let offset = self.term.grid().display_offset() as i32;
        let cols = self.dimensions.columns.max(1);
        let to_point = |(col, row): (usize, usize)| {
            let line = Line(row as i32 - offset);
            let col = col.min(cols - 1);
            (GridIndex::new(line, Column(col)), col)
        };
        let (a_point, a_col) = to_point(anchor);
        let (h_point, h_col) = to_point(head);
        // Anchor side stays on the outer edge of the cell so a click that never
        // moves selects nothing, matching how xterm/alacritty behave.
        let anchor_side = if h_col >= a_col {
            Side::Left
        } else {
            Side::Right
        };
        let head_side = if h_col >= a_col {
            Side::Right
        } else {
            Side::Left
        };
        let mut selection = Selection::new(SelectionType::Simple, a_point, anchor_side);
        selection.update(h_point, head_side);
        self.term.selection = Some(selection);
    }

    /// The currently selected text, if any (empty selection → `None`).
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }

    /// `true` while a full-screen application (vim, less, …) holds the alternate
    /// screen.
    pub fn is_alt_screen(&self) -> bool {
        self.term
            .mode()
            .contains(alacritty_terminal::term::TermMode::ALT_SCREEN)
    }

    /// Number of lines currently held in scrollback history.
    pub fn history_len(&self) -> usize {
        self.term.grid().history_size()
    }

    /// Build an immutable snapshot of the visible grid with theme-resolved
    /// colors.
    pub fn render(&self) -> RenderableScreen {
        let content = self.term.renderable_content();
        let display_offset = content.display_offset;
        let selection_range = content.selection;
        let cursor = RenderableCursor {
            line: content.cursor.point.line.0,
            column: content.cursor.point.column.0,
            shape: content.cursor.shape,
        };

        let mut cells = Vec::with_capacity(self.dimensions.columns * self.dimensions.screen_lines);
        for item in content.display_iter {
            let cell = item.cell;
            let flags = cell.flags;
            if flags.contains(Flags::WIDE_CHAR_SPACER)
                || flags.contains(Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            let point: Point = item.point;
            let line = point.line.0;
            if line < 0 {
                continue;
            }
            let dim = flags.contains(Flags::DIM);
            let inverse = flags.contains(Flags::INVERSE);
            let mut fg = resolve_color(cell.fg, &self.colors, dim);
            let mut bg = resolve_color(cell.bg, &self.colors, false);
            if inverse {
                std::mem::swap(&mut fg, &mut bg);
            }
            cells.push(RenderableCell {
                line: line as usize,
                column: point.column.0,
                c: cell.c,
                fg,
                bg,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.intersects(Flags::ALL_UNDERLINES),
                strikeout: flags.contains(Flags::STRIKEOUT),
                inverse,
                dim,
                hidden: flags.contains(Flags::HIDDEN),
            });
        }

        let mut selection = Vec::new();
        if let Some(range) = selection_range {
            let columns = self.dimensions.columns;
            let (start_line, end_line) = (range.start.line.0, range.end.line.0);
            for line in 0..self.dimensions.screen_lines {
                let l = line as i32;
                if l < start_line || l > end_line {
                    continue;
                }
                let (mut start_col, mut end_col) = if range.is_block {
                    (range.start.column.0, range.end.column.0 + 1)
                } else {
                    let s = if l == start_line {
                        range.start.column.0
                    } else {
                        0
                    };
                    let e = if l == end_line {
                        range.end.column.0 + 1
                    } else {
                        columns
                    };
                    (s, e)
                };
                start_col = start_col.min(columns);
                end_col = end_col.min(columns);
                if end_col > start_col {
                    selection.push(SelectionSpan {
                        line,
                        start_col,
                        end_col,
                    });
                }
            }
        }

        RenderableScreen {
            columns: self.dimensions.columns,
            screen_lines: self.dimensions.screen_lines,
            display_offset,
            cursor,
            cells,
            selection,
        }
    }
}

/// A grid position, re-exported for renderer/tests without pulling the whole
/// `alacritty_terminal::index` module.
pub type GridPoint = Point<Line, Column>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};

    fn emulator(cols: usize, rows: usize) -> (TerminalEmulator, Receiver<TerminalEvent>) {
        let (tx, rx) = channel();
        let colors = TerminalColors::from_theme(&labonair_theme::Theme::dark());
        (
            TerminalEmulator::new(colors, TermDimensions::new(cols, rows), tx),
            rx,
        )
    }

    #[test]
    fn plain_text_lands_in_the_grid() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"hello");
        assert!(term.render().to_text().starts_with("hello"));
    }

    #[test]
    fn sgr_color_is_resolved_from_the_theme_palette() {
        let (mut term, _rx) = emulator(20, 3);
        term.feed(b"\x1b[31mX\x1b[0m");
        let cell = term
            .render()
            .cells
            .into_iter()
            .find(|c| c.c == 'X')
            .unwrap();
        let expected = TerminalColors::from_theme(&labonair_theme::Theme::dark()).normal[1];
        assert_eq!(cell.fg, expected);
    }

    #[test]
    fn cursor_movement_sequences_are_honored() {
        let (mut term, _rx) = emulator(20, 5);
        // Move to row 3, col 5 (1-based) then write.
        term.feed(b"\x1b[3;5HZ");
        let cell = term
            .render()
            .cells
            .into_iter()
            .find(|c| c.c == 'Z')
            .unwrap();
        assert_eq!((cell.line, cell.column), (2, 4));
    }

    #[test]
    fn wakeup_event_is_emitted_on_output() {
        let (mut term, _rx) = emulator(20, 5);
        assert!(term.feed(b"x").contains(&TerminalEvent::Wakeup));
        assert!(term.feed(b"").is_empty());
    }

    #[test]
    fn title_sequence_emits_event() {
        let (mut term, rx) = emulator(20, 5);
        term.feed(b"\x1b]0;my-title\x07");
        assert!(rx
            .try_iter()
            .any(|e| e == TerminalEvent::Title("my-title".into())));
    }

    #[test]
    fn osc7_reports_working_directory() {
        let (mut term, _rx) = emulator(20, 5);
        let events = term.feed(b"\x1b]7;file://host/Users/me/dev\x07");
        assert!(events.contains(&TerminalEvent::Cwd("/Users/me/dev".into())));
    }

    #[test]
    fn resize_changes_grid_dimensions() {
        let (mut term, _rx) = emulator(80, 24);
        term.resize(TermDimensions::new(100, 40));
        let screen = term.render();
        assert_eq!((screen.columns, screen.screen_lines), (100, 40));
    }

    #[test]
    fn scrollback_accumulates_and_scrolls() {
        let (mut term, _rx) = emulator(10, 3);
        for i in 0..50 {
            term.feed(format!("line{i}\r\n").as_bytes());
        }
        assert!(term.history_len() >= 40, "history {}", term.history_len());
        assert_eq!(term.render().display_offset, 0);
        term.scroll(Scroll::Top);
        assert!(term.render().display_offset > 0);
        term.scroll(Scroll::Bottom);
        assert_eq!(term.render().display_offset, 0);
    }

    #[test]
    fn alternate_screen_toggles() {
        let (mut term, _rx) = emulator(20, 5);
        assert!(!term.is_alt_screen());
        term.feed(b"\x1b[?1049h");
        assert!(term.is_alt_screen());
        term.feed(b"\x1b[?1049l");
        assert!(!term.is_alt_screen());
    }

    #[test]
    fn selection_is_split_into_per_row_spans() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"line one\r\nline two\r\nline three");
        // Select from row 0 col 2 to row 2 col 4.
        term.set_selection((0, 2), (2, 4));
        let screen = term.render();
        let rows: Vec<_> = screen.selection.iter().map(|s| s.line).collect();
        assert_eq!(rows, vec![0, 1, 2]);
        assert_eq!(screen.selection[0].start_col, 2);
        assert_eq!(
            screen.selection[1],
            SelectionSpan {
                line: 1,
                start_col: 0,
                end_col: 20
            }
        );
        assert_eq!(screen.selection[2].end_col, 5);

        term.clear_selection();
        assert!(term.render().selection.is_empty());
    }

    #[test]
    fn inverse_attribute_swaps_fg_and_bg() {
        let (mut term, _rx) = emulator(20, 3);
        term.feed(b"\x1b[7mI\x1b[0m");
        let palette = TerminalColors::from_theme(&labonair_theme::Theme::dark());
        let cell = term
            .render()
            .cells
            .into_iter()
            .find(|c| c.c == 'I')
            .unwrap();
        assert_eq!(cell.fg, palette.background);
        assert!(cell.inverse);
    }
}
