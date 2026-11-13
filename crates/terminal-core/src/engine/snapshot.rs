//! Emulator state snapshot: serialize a [`TerminalEmulator`] as an escape-sequence
//! byte stream that, fed into an emulator of the same size, reproduces its state.
//!
//! The detached session host (`labonair-host`) keeps a *shadow* emulator per
//! session. On attach / resync it calls [`TerminalEmulator::snapshot`] and the
//! app feeds the bytes into its local (render-source) emulator, after which live
//! PTY bytes follow. The stream starts with `RIS` (`ESC c`), so it also works on
//! a dirty emulator, not just a fresh one.
//!
//! # What is reproduced
//!
//! * Primary grid with scrollback (newest [`SnapshotOptions::max_scrollback_lines`]
//!   rows): characters, wide chars, zero-width/combining marks, SGR (named /
//!   256 / truecolor fg+bg, bold, dim, italic, all underline styles + underline
//!   color, inverse, hidden, strikeout), OSC 8 hyperlinks, soft-wrap flags
//!   (`WRAPLINE`, incl. leading wide-char spacers) and tab characters stored in
//!   cells.
//! * Alternate screen: `ESC[?1049h` + its grid, when it is active.
//! * Cursor (position, pending-wrap state, SGR template incl. hyperlink,
//!   G0..G3 charset designations), saved cursor (DECSC), cursor style
//!   (DECSCUSR, only when it differs from the configured default).
//! * Scroll region (DECSTBM), custom tab stops, origin / autowrap / insert /
//!   LNM / application cursor + keypad / bracketed paste / focus reporting /
//!   mouse 1000, 1002, 1003 + 1005, 1006 encodings / alternate scroll /
//!   urgency hint / cursor visibility / kitty keyboard flags (per screen).
//! * [`SessionMetadata`]: title (OSC 2), cwd (OSC 7), last exit code, last
//!   command, prompt phase and `in_command` (OSC 133).
//!
//! # What is NOT reproduced (and why that is acceptable)
//!
//! * Active charset shift (SO/SI): the engine keeps it private. G0 is assumed
//!   active; a program that shifted to G1 is wrong until its next redraw.
//! * Title stack (`CSI 22/23 t`), kitty keyboard stack depth beyond the top
//!   entry, OSC 4/10/11/12 palette overrides (this engine renders from the
//!   theme palette and ignores them), selection, search state, display offset.
//! * `last_command` can only be set, not cleared, on a dirty emulator; the
//!   OSC 7 cwd can only be set, never cleared. A cwd that is not absolute is
//!   skipped.
//! * Text in `last_command` / title is stripped of control characters.
//! * Reproducing a cursor (or saved cursor) that waits to wrap rewrites the cell
//!   it sits on, which clears that cell's soft-wrap flag. A cursor with a pending
//!   wrap whose last-column cell is a tab character becomes a blank.
//! * Quirks the engine leaves after resizes or erases through wide chars (a pending
//!   wrap away from the last column, stale wrap flags, orphaned wide-char halves,
//!   stale BCE cells in rows whose width grew) are normalized, not reproduced.
//! * Shadow state is tracked by a small byte-stream sniffer (see
//!   [`StateTracker`]); malformed or C1 (8-bit) control sequences can desync
//!   it from the real parser.
//! * The daemon must resize the PTY *before* the snapshot and may "wiggle" the
//!   PTY size afterwards ([`resize_wiggle_sizes`]); all of the above is
//!   repaired by the TUI's redraw on `SIGWINCH`.
//!
//! Feeding a snapshot makes the receiving emulator emit the usual
//! [`TerminalEvent`]s (`Cwd`, `PromptStart`, ...), so callers should treat the
//! replay as history rather than as new activity.
//!
//! # Side effects of snapshotting
//!
//! `snapshot` takes `&mut self`: while the alternate screen is active the
//! primary grid is only reachable by swapping screens, so the alt grid is
//! cloned and restored afterwards. The selection is cleared in that case.

use std::mem;

use alacritty_terminal::grid::{Cursor, Dimensions, Grid};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags, Hyperlink};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{
    Color as AnsiColor, CursorShape, Handler, KeyboardModes, KeyboardModesApplyBehavior,
    NamedColor, StandardCharset,
};

use super::{PromptPhase, SessionMetadata, TermDimensions, TerminalEmulator};

/// Tunables for [`TerminalEmulator::snapshot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotOptions {
    /// How many of the newest scrollback rows to include (0 = none). The
    /// default mirrors the `detachedScrollbackLines` setting.
    pub max_scrollback_lines: usize,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            max_scrollback_lines: 5000,
        }
    }
}

/// Sizes for the SIGWINCH "wiggle" after a snapshot: apply the first to the
/// PTY, then the second (the real size). Resizing the PTY to an unchanged size
/// does not signal the foreground process, so the temporary size differs by one
/// row (or one column on a single-row terminal). Apply it to the **PTY only**,
/// never to the emulators: a shrink/grow cycle would reflow and lose content.
pub fn resize_wiggle_sizes(dims: TermDimensions) -> (TermDimensions, TermDimensions) {
    let mut wiggle = dims;
    if dims.screen_lines > 1 {
        wiggle.screen_lines -= 1;
    } else {
        wiggle.columns += 1;
    }
    (wiggle, dims)
}

// ── State the engine keeps private: scroll region + tab stops ────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum PState {
    Ground,
    Esc,
    EscInter,
    Csi,
    Str,
    StrEsc,
}

/// A tab-stop edit found in the byte stream; needs the cursor column, which
/// only the emulator knows, so [`StateTracker::step`] hands it back.
#[derive(Clone, Copy)]
pub(super) enum TabOp {
    Set,
    ClearCurrent,
}

/// Mirrors the two pieces of `Term` state that have no public accessor — the
/// DECSTBM scroll region and the tab stops — by sniffing the same byte stream
/// the parser sees. Deliberately tiny: it only recognizes `ESC H`, `ESC c`,
/// `CSI Ps g`, `CSI t;b r` and `CSI ? 3 h/l`.
// ponytail: no C1 (8-bit) controls, no DECSTR; add if a real program needs them.
pub(super) struct StateTracker {
    state: PState,
    params: [u32; 2],
    /// Number of `;` seen in the current CSI.
    separators: usize,
    /// First private marker / intermediate byte of the current CSI (0 = none).
    marker: u8,
    /// Colon sub-parameters or more than one marker/intermediate: not ours.
    foreign: bool,
    /// `(start, end)` like `Term::scroll_region` (end exclusive); `None` = full.
    pub(super) region: Option<(usize, usize)>,
    /// Custom tab stops; `None` = the default every-8-columns pattern.
    pub(super) tabs: Option<Vec<bool>>,
    columns: usize,
    lines: usize,
}

impl StateTracker {
    pub(super) fn new(columns: usize, lines: usize) -> Self {
        Self {
            state: PState::Ground,
            params: [0; 2],
            separators: 0,
            marker: 0,
            foreign: false,
            region: None,
            tabs: None,
            columns,
            lines,
        }
    }

    /// Mirror `Term::resize`: tab stops grow with the default pattern, the
    /// scroll region is reset (only when the size really changed).
    pub(super) fn resize(&mut self, columns: usize, lines: usize) {
        if columns == self.columns && lines == self.lines {
            return;
        }
        if let Some(tabs) = &mut self.tabs {
            let mut index = tabs.len();
            tabs.resize_with(columns, || {
                let stop = index % 8 == 0;
                index += 1;
                stop
            });
        }
        self.columns = columns;
        self.lines = lines;
        self.region = None;
    }

    fn reset(&mut self) {
        self.region = None;
        self.tabs = None;
    }

    fn default_tabs(&self) -> Vec<bool> {
        (0..self.columns).map(|i| i % 8 == 0).collect()
    }

    pub(super) fn tab_op(&mut self, op: TabOp, column: usize) {
        let default = self.default_tabs();
        let tabs = self.tabs.get_or_insert(default);
        if let Some(slot) = tabs.get_mut(column) {
            *slot = matches!(op, TabOp::Set);
        }
    }

    fn csi_start(&mut self) {
        self.params = [0; 2];
        self.separators = 0;
        self.marker = 0;
        self.foreign = false;
        self.state = PState::Csi;
    }

    /// Advance by one byte *before* the parser consumes it.
    #[inline]
    pub(super) fn step(&mut self, byte: u8) -> Option<TabOp> {
        if self.state == PState::Ground {
            if byte == 0x1b {
                self.state = PState::Esc;
            }
            return None;
        }
        self.step_slow(byte)
    }

    fn step_slow(&mut self, byte: u8) -> Option<TabOp> {
        match self.state {
            PState::Ground => {}
            PState::Esc => match byte {
                b'[' => self.csi_start(),
                b']' | b'P' | b'X' | b'^' | b'_' => self.state = PState::Str,
                0x1b => {}
                0x20..=0x2f => self.state = PState::EscInter,
                b'H' => {
                    self.state = PState::Ground;
                    return Some(TabOp::Set);
                }
                b'c' => {
                    self.state = PState::Ground;
                    self.reset();
                }
                0x00..=0x1f => {}
                _ => self.state = PState::Ground,
            },
            PState::EscInter => match byte {
                0x1b => self.state = PState::Esc,
                0x18 | 0x1a | 0x30..=0x7e => self.state = PState::Ground,
                _ => {}
            },
            PState::Csi => match byte {
                0x1b => self.state = PState::Esc,
                0x18 | 0x1a => self.state = PState::Ground,
                b'0'..=b'9' => {
                    if let Some(p) = self.params.get_mut(self.separators) {
                        *p = (*p * 10 + u32::from(byte - b'0')).min(65_535);
                    }
                }
                b';' => self.separators += 1,
                b':' => self.foreign = true,
                0x3c..=0x3f | 0x20..=0x2f => {
                    if self.marker == 0 {
                        self.marker = byte;
                    } else {
                        self.foreign = true;
                    }
                }
                0x40..=0x7e => {
                    self.state = PState::Ground;
                    return self.csi_dispatch(byte);
                }
                _ => {}
            },
            PState::Str => match byte {
                0x07 | 0x18 | 0x1a => self.state = PState::Ground,
                0x1b => self.state = PState::StrEsc,
                _ => {}
            },
            PState::StrEsc => match byte {
                b'\\' => self.state = PState::Ground,
                0x1b => {}
                _ => {
                    self.state = PState::Esc;
                    return self.step_slow(byte);
                }
            },
        }
        None
    }

    fn csi_dispatch(&mut self, action: u8) -> Option<TabOp> {
        if self.foreign {
            return None;
        }
        match (action, self.marker) {
            (b'r', 0) => {
                // Mirrors `Term::set_scrolling_region`.
                let top = if self.params[0] == 0 {
                    1
                } else {
                    self.params[0] as usize
                };
                let bottom = if self.separators >= 1 && self.params[1] != 0 {
                    self.params[1] as usize
                } else {
                    self.lines
                };
                if top < bottom {
                    let start = (top - 1).min(self.lines);
                    let end = bottom.min(self.lines);
                    self.region = (start != 0 || end != self.lines).then_some((start, end));
                }
            }
            (b'g', 0) => match self.params[0] {
                0 => return Some(TabOp::ClearCurrent),
                3 => {
                    self.tabs = Some(vec![false; self.columns]);
                }
                _ => {}
            },
            // DECCOLM clears the scroll region as a side effect.
            (b'h' | b'l', b'?') if self.params[0] == 3 => self.region = None,
            _ => {}
        }
        None
    }

    fn custom_tabs(&self) -> Option<&Vec<bool>> {
        self.tabs
            .as_ref()
            .filter(|tabs| **tabs != self.default_tabs())
    }
}

// ── Output helpers ───────────────────────────────────────────────────────────

fn push_num(out: &mut Vec<u8>, mut n: usize) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&buf[i..]);
}

/// `ESC [ <a> ; <b> <fin>` (or just one parameter when `b` is `None`).
fn csi(out: &mut Vec<u8>, a: usize, b: Option<usize>, fin: u8) {
    out.extend_from_slice(b"\x1b[");
    push_num(out, a);
    if let Some(b) = b {
        out.push(b';');
        push_num(out, b);
    }
    out.push(fin);
}

/// `ESC [ ? <mode> h|l`
fn private_mode(out: &mut Vec<u8>, mode: usize, on: bool) {
    out.extend_from_slice(b"\x1b[?");
    push_num(out, mode);
    out.push(if on { b'h' } else { b'l' });
}

fn cup(out: &mut Vec<u8>, line: usize, column: usize) {
    csi(out, line + 1, Some(column + 1), b'H');
}

fn cha(out: &mut Vec<u8>, column: usize) {
    csi(out, column + 1, None, b'G');
}

/// Strip control characters so text can never terminate its own OSC early.
fn push_clean(out: &mut Vec<u8>, text: &str) {
    let mut buf = [0u8; 4];
    for c in text.chars() {
        if !c.is_control() {
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
}

const ST: &[u8] = b"\x1b\\";
const ATTR_MASK: Flags = Flags::BOLD
    .union(Flags::DIM)
    .union(Flags::ITALIC)
    .union(Flags::ALL_UNDERLINES)
    .union(Flags::INVERSE)
    .union(Flags::HIDDEN)
    .union(Flags::STRIKEOUT);
const DEFAULT_FG: AnsiColor = AnsiColor::Named(NamedColor::Foreground);
const DEFAULT_BG: AnsiColor = AnsiColor::Named(NamedColor::Background);

/// Everything one SGR/OSC 8 state covers.
#[derive(Clone, PartialEq)]
struct Style {
    fg: AnsiColor,
    bg: AnsiColor,
    flags: Flags,
    underline: Option<AnsiColor>,
    link: Option<Hyperlink>,
}

impl Style {
    const DEFAULT: Style = Style {
        fg: DEFAULT_FG,
        bg: DEFAULT_BG,
        flags: Flags::empty(),
        underline: None,
        link: None,
    };

    fn of(cell: &Cell) -> Self {
        Self {
            fg: cell.fg,
            bg: cell.bg,
            flags: cell.flags & ATTR_MASK,
            underline: cell.underline_color(),
            link: cell.hyperlink(),
        }
    }

    /// Fast check for the common extra-less cell.
    #[inline]
    fn matches_plain(&self, cell: &Cell) -> bool {
        cell.extra.is_none()
            && self.underline.is_none()
            && self.link.is_none()
            && self.fg == cell.fg
            && self.bg == cell.bg
            && self.flags == cell.flags & ATTR_MASK
    }
}

fn push_color_sgr(out: &mut Vec<u8>, color: AnsiColor, base: usize) {
    // `base` = 30 (fg) / 40 (bg); extended form uses base + 8.
    match color {
        AnsiColor::Named(named) => {
            let i = named as usize;
            let code = match i {
                0..=7 => base + i,
                8..=15 => base + 52 + i,
                // Dim variants (never produced by SGR) fall back to the plain color.
                259..=266 => base + (i - 259),
                _ => return,
            };
            out.push(b';');
            push_num(out, code);
        }
        AnsiColor::Indexed(index) => {
            out.push(b';');
            push_num(out, base + 8);
            out.extend_from_slice(b";5;");
            push_num(out, usize::from(index));
        }
        AnsiColor::Spec(rgb) => {
            out.push(b';');
            push_num(out, base + 8);
            out.extend_from_slice(b";2;");
            push_num(out, usize::from(rgb.r));
            out.push(b';');
            push_num(out, usize::from(rgb.g));
            out.push(b';');
            push_num(out, usize::from(rgb.b));
        }
    }
}

/// Tracks the SGR / hyperlink state the receiving emulator is in and emits
/// only the difference.
struct Writer<'a> {
    out: &'a mut Vec<u8>,
    cur: Style,
    columns: usize,
}

impl Writer<'_> {
    fn style(&mut self, want: &Style) {
        if *want == self.cur {
            return;
        }
        let sgr_changed = want.fg != self.cur.fg
            || want.bg != self.cur.bg
            || want.flags != self.cur.flags
            || want.underline != self.cur.underline;
        if sgr_changed {
            let out = &mut *self.out;
            out.extend_from_slice(b"\x1b[0");
            let f = want.flags;
            if f.contains(Flags::BOLD) {
                out.extend_from_slice(b";1");
            }
            if f.contains(Flags::DIM) {
                out.extend_from_slice(b";2");
            }
            if f.contains(Flags::ITALIC) {
                out.extend_from_slice(b";3");
            }
            if f.contains(Flags::UNDERLINE) {
                out.extend_from_slice(b";4");
            }
            if f.contains(Flags::DOUBLE_UNDERLINE) {
                out.extend_from_slice(b";4:2");
            }
            if f.contains(Flags::UNDERCURL) {
                out.extend_from_slice(b";4:3");
            }
            if f.contains(Flags::DOTTED_UNDERLINE) {
                out.extend_from_slice(b";4:4");
            }
            if f.contains(Flags::DASHED_UNDERLINE) {
                out.extend_from_slice(b";4:5");
            }
            if f.contains(Flags::INVERSE) {
                out.extend_from_slice(b";7");
            }
            if f.contains(Flags::HIDDEN) {
                out.extend_from_slice(b";8");
            }
            if f.contains(Flags::STRIKEOUT) {
                out.extend_from_slice(b";9");
            }
            if want.fg != DEFAULT_FG {
                push_color_sgr(out, want.fg, 30);
            }
            if want.bg != DEFAULT_BG {
                push_color_sgr(out, want.bg, 40);
            }
            if let Some(color) = want.underline {
                match color {
                    AnsiColor::Named(named) if (named as usize) < 16 => {
                        out.extend_from_slice(b";58;5;");
                        push_num(out, named as usize);
                    }
                    AnsiColor::Named(_) => {}
                    AnsiColor::Indexed(index) => {
                        out.extend_from_slice(b";58;5;");
                        push_num(out, usize::from(index));
                    }
                    AnsiColor::Spec(rgb) => {
                        out.extend_from_slice(b";58;2;");
                        push_num(out, usize::from(rgb.r));
                        out.push(b';');
                        push_num(out, usize::from(rgb.g));
                        out.push(b';');
                        push_num(out, usize::from(rgb.b));
                    }
                }
            }
            out.push(b'm');
        }
        if want.link != self.cur.link {
            self.out.extend_from_slice(b"\x1b]8;");
            if let Some(link) = &want.link {
                self.out.extend_from_slice(b"id=");
                push_clean(self.out, link.id());
                self.out.push(b';');
                push_clean(self.out, link.uri());
            } else {
                self.out.push(b';');
            }
            self.out.extend_from_slice(ST);
        }
        self.cur = want.clone();
    }

    fn reset_style(&mut self) {
        if self.cur != Style::DEFAULT {
            self.style(&Style::DEFAULT);
        }
    }

    fn put_char(&mut self, c: char) {
        let c = if c.is_control() { ' ' } else { c };
        let mut buf = [0u8; 4];
        self.out
            .extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
    }

    /// Write one non-spacer cell at the cursor with its attributes.
    fn put_cell(&mut self, cell: &Cell, column: usize) {
        if !self.cur.matches_plain(cell) {
            self.style(&Style::of(cell));
        }
        // (A tab in the last column cannot be restored without losing the
        // pending wrap; it degrades to a blank.)
        if cell.c == '\t' && column + 1 < self.columns {
            // `put_tab` stores '\t' only into a blank cell, so write a blank
            // (to carry the attributes), step back, and tab over it.
            self.out.push(b' ');
            cha(self.out, column);
            self.out.push(b'\t');
            cha(self.out, column + 1);
            return;
        }
        self.put_char(if cell.c == '\t' { ' ' } else { cell.c });
        if let Some(marks) = cell.zerowidth() {
            for &mark in marks {
                self.put_char(mark);
            }
        }
    }
}

fn is_blank(cell: &Cell) -> bool {
    cell.c == ' '
        && cell.fg == DEFAULT_FG
        && cell.bg == DEFAULT_BG
        && (cell.flags & !Flags::WRAPLINE).is_empty()
        && cell.extra.is_none()
}

/// Write rows `first..screen_lines` of `grid` (negative = scrollback),
/// separated by `CRLF` unless the row soft-wraps into the next one.
fn dump_rows(w: &mut Writer, grid: &Grid<Cell>, first: i32, columns: usize, screen_lines: usize) {
    let last_line = screen_lines as i32 - 1;
    let mut prev_wrapped = false;
    for line in first..=last_line {
        let row = &grid[Line(line)];
        let last_cell = &row[Column(columns - 1)];
        let wrapped = line != last_line && last_cell.flags.contains(Flags::WRAPLINE);
        let mut end = columns;
        if wrapped {
            // A wide char that did not fit starts the next row; the cursor must
            // sit on the spacer column so the engine wraps by itself.
            if last_cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) {
                end -= 1;
            }
        } else {
            while end > 0 && is_blank(&row[Column(end - 1)]) {
                end -= 1;
            }
        }
        if end == 0 && prev_wrapped {
            // The wrap flag is set by the first character of the next row and an
            // empty row would otherwise merge into the wrapped one: write a
            // blank, which equals the default cell.
            w.reset_style();
            w.out.push(b' ');
        }
        let started_by_wrap = prev_wrapped;
        prev_wrapped = wrapped;
        let mut skip_spacer = false;
        for column in 0..end {
            if mem::take(&mut skip_spacer) {
                continue;
            }
            let cell = &row[Column(column)];
            if cell.flags.contains(Flags::WIDE_CHAR) && column + 1 < columns {
                // The engine writes the spacer itself.
                w.put_cell(cell, column);
                skip_spacer = true;
            } else if cell.flags.intersects(
                Flags::WIDE_CHAR | Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER,
            ) {
                // Orphaned half of a wide char (left by erases): keep the
                // columns aligned with a blank.
                w.style(&Style::of(cell));
                w.out.push(b' ');
            } else {
                w.put_cell(cell, column);
            }
        }
        if started_by_wrap && end < columns {
            // An automatic wrap at the bottom creates the new row with the
            // current background (BCE); the cells past the text are default.
            w.reset_style();
            w.out.extend_from_slice(b"\x1b[K");
        }
        if line != last_line && !wrapped {
            // New rows inherit the current background (BCE): leave SGR first.
            w.reset_style();
            w.out.extend_from_slice(b"\r\n");
        }
    }
}

fn designate_charsets(out: &mut Vec<u8>, cursor: &Cursor<Cell>) {
    use alacritty_terminal::vte::ansi::CharsetIndex::{G0, G1, G2, G3};
    for (index, intro) in [(G0, b'('), (G1, b')'), (G2, b'*'), (G3, b'+')] {
        let set = match cursor.charsets[index] {
            StandardCharset::Ascii => b'B',
            StandardCharset::SpecialCharacterAndLineDrawing => b'0',
        };
        out.extend_from_slice(&[0x1b, intro, set]);
    }
}

/// Move to `cursor`, reproduce a pending wrap, and leave the SGR / hyperlink
/// state equal to the cursor template.
fn place_cursor(w: &mut Writer, grid: &Grid<Cell>, cursor: &Cursor<Cell>, columns: usize) {
    let line = cursor.point.line.0.max(0) as usize;
    let mut column = cursor.point.column.0;
    cup(w.out, line, column);
    if cursor.input_needs_wrap && column + 1 == columns {
        // Rewrite the last cell: that is what puts the engine in wrap-pending.
        // A previously applied designation must not remap its character.
        designate_charsets(w.out, &Cursor::<Cell>::default());
        let row = &grid[cursor.point.line];
        if row[Column(column)].flags.contains(Flags::WIDE_CHAR_SPACER) && column > 0 {
            column -= 1;
            cup(w.out, line, column);
        }
        let cell = &row[Column(column)];
        w.put_cell(cell, column);
    }
    w.style(&Style::of(&cursor.template));
    designate_charsets(w.out, cursor);
}

fn kitty_bits(mode: TermMode) -> u8 {
    let mut bits = KeyboardModes::empty();
    bits.set(
        KeyboardModes::DISAMBIGUATE_ESC_CODES,
        mode.contains(TermMode::DISAMBIGUATE_ESC_CODES),
    );
    bits.set(
        KeyboardModes::REPORT_EVENT_TYPES,
        mode.contains(TermMode::REPORT_EVENT_TYPES),
    );
    bits.set(
        KeyboardModes::REPORT_ALTERNATE_KEYS,
        mode.contains(TermMode::REPORT_ALTERNATE_KEYS),
    );
    bits.set(
        KeyboardModes::REPORT_ALL_KEYS_AS_ESC,
        mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
    );
    bits.set(
        KeyboardModes::REPORT_ASSOCIATED_TEXT,
        mode.contains(TermMode::REPORT_ASSOCIATED_TEXT),
    );
    bits.bits()
}

fn kitty(out: &mut Vec<u8>, mode: TermMode) {
    out.extend_from_slice(b"\x1b[=");
    push_num(out, usize::from(kitty_bits(mode)));
    out.extend_from_slice(b";1u");
}

/// Every mode except the per-screen kitty keyboard flags.
fn emit_modes(out: &mut Vec<u8>, mode: TermMode) {
    let has = |flag: TermMode| mode.contains(flag);
    private_mode(out, 25, has(TermMode::SHOW_CURSOR));
    private_mode(out, 1, has(TermMode::APP_CURSOR));
    out.extend_from_slice(if has(TermMode::APP_KEYPAD) {
        b"\x1b="
    } else {
        b"\x1b>"
    });
    for number in [1000, 1002, 1003, 1005, 1006] {
        private_mode(out, number, false);
    }
    if has(TermMode::MOUSE_REPORT_CLICK) {
        private_mode(out, 1000, true);
    }
    if has(TermMode::MOUSE_DRAG) {
        private_mode(out, 1002, true);
    }
    if has(TermMode::MOUSE_MOTION) {
        private_mode(out, 1003, true);
    }
    if has(TermMode::UTF8_MOUSE) {
        private_mode(out, 1005, true);
    }
    if has(TermMode::SGR_MOUSE) {
        private_mode(out, 1006, true);
    }
    private_mode(out, 2004, has(TermMode::BRACKETED_PASTE));
    private_mode(out, 1004, has(TermMode::FOCUS_IN_OUT));
    private_mode(out, 7, has(TermMode::LINE_WRAP));
    // Origin mode only changes how later CUPs are read; it does not move the
    // cursor, so it can come after the final (absolute) cursor placement.
    private_mode(out, 6, has(TermMode::ORIGIN));
    private_mode(out, 1007, has(TermMode::ALTERNATE_SCROLL));
    private_mode(out, 1042, has(TermMode::URGENCY_HINTS));
    out.extend_from_slice(if has(TermMode::INSERT) {
        b"\x1b[4h"
    } else {
        b"\x1b[4l"
    });
    out.extend_from_slice(if has(TermMode::LINE_FEED_NEW_LINE) {
        b"\x1b[20h"
    } else {
        b"\x1b[20l"
    });
}

fn emit_metadata(out: &mut Vec<u8>, meta: &SessionMetadata) {
    // OSC 7 is ignored while `in_command`, so leave command state first.
    out.extend_from_slice(b"\x1b]133;D");
    out.extend_from_slice(ST);
    if let Some(cwd) = meta.cwd.as_deref().filter(|cwd| cwd.starts_with('/')) {
        out.extend_from_slice(b"\x1b]7;file://");
        for &byte in cwd.as_bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                out.push(byte);
            } else {
                out.extend_from_slice(format!("%{byte:02X}").as_bytes());
            }
        }
        out.extend_from_slice(ST);
    }
    // An empty title resets it to `None`.
    out.extend_from_slice(b"\x1b]2;");
    if let Some(title) = &meta.title {
        push_clean(out, title);
    }
    out.extend_from_slice(ST);

    let code = |out: &mut Vec<u8>| {
        out.extend_from_slice(b"\x1b]133;D");
        if let Some(code) = meta.last_exit_code {
            out.push(b';');
            out.extend_from_slice(code.to_string().as_bytes());
        }
        out.extend_from_slice(ST);
    };
    let command = |out: &mut Vec<u8>| {
        out.extend_from_slice(b"\x1b]133;C");
        if let Some(text) = &meta.last_command {
            out.push(b';');
            push_clean(out, text);
        }
        out.extend_from_slice(ST);
    };
    if meta.in_command {
        code(out);
        command(out);
        if meta.prompt_phase == PromptPhase::Prompt {
            out.extend_from_slice(b"\x1b]133;B");
            out.extend_from_slice(ST);
        }
    } else {
        if meta.last_command.is_some() {
            command(out);
        }
        code(out);
        if matches!(
            meta.prompt_phase,
            PromptPhase::PromptStart | PromptPhase::Prompt
        ) {
            out.extend_from_slice(b"\x1b]133;A");
            out.extend_from_slice(ST);
        }
        if meta.prompt_phase == PromptPhase::Prompt {
            out.extend_from_slice(b"\x1b]133;B");
            out.extend_from_slice(ST);
        }
    }
}

/// Per-screen tail: region, saved cursor, origin mode, cursor.
fn emit_tail(w: &mut Writer, grid: &Grid<Cell>, region: Option<(usize, usize)>, columns: usize) {
    if let Some((start, end)) = region {
        // The engine rejects `top >= bottom`, yet clamps a too-large bottom, so
        // one-line / empty regions at the screen bottom need a bottom past the top.
        csi(
            w.out,
            start + 1,
            Some(if end <= start + 1 { start + 2 } else { end }),
            b'r',
        );
    }
    if grid.saved_cursor != Cursor::default() {
        place_cursor(w, grid, &grid.saved_cursor, columns);
        w.out.extend_from_slice(b"\x1b7");
    }
    place_cursor(w, grid, &grid.cursor, columns);
}

struct Capture<'a> {
    mode: TermMode,
    primary_kitty_mode: TermMode,
    region: Option<(usize, usize)>,
    tabs: Option<&'a Vec<bool>>,
    cursor_style: Option<alacritty_terminal::vte::ansi::CursorStyle>,
    metadata: &'a SessionMetadata,
    history: usize,
}

fn build(
    out: &mut Vec<u8>,
    cap: &Capture,
    primary: &Grid<Cell>,
    alt: Option<&Grid<Cell>>,
    columns: usize,
    screen_lines: usize,
) {
    let mut w = Writer {
        out,
        cur: Style::DEFAULT,
        columns,
    };
    w.out.extend_from_slice(b"\x1bc");
    if let Some(tabs) = cap.tabs {
        w.out.extend_from_slice(b"\x1b[3g");
        for (column, _) in tabs.iter().enumerate().filter(|(_, stop)| **stop) {
            cup(w.out, 0, column);
            w.out.extend_from_slice(b"\x1bH");
        }
    }
    w.out.extend_from_slice(b"\x1b[H");
    dump_rows(
        &mut w,
        primary,
        -(cap.history as i32),
        columns,
        screen_lines,
    );
    w.reset_style();
    match alt {
        None => {
            emit_tail(&mut w, primary, cap.region, columns);
            kitty(w.out, cap.mode);
        }
        Some(alt) => {
            place_cursor(&mut w, primary, &primary.cursor, columns);
            kitty(w.out, cap.primary_kitty_mode);
            private_mode(w.out, 1049, true);
            // The alt cursor starts as a copy of the primary one, and entering
            // clears the alt grid with its background: use defaults instead.
            designate_charsets(w.out, &Cursor::<Cell>::default());
            w.reset_style();
            w.out.extend_from_slice(b"\x1b[2J\x1b[H");
            dump_rows(&mut w, alt, 0, columns, screen_lines);
            w.reset_style();
            emit_tail(&mut w, alt, cap.region, columns);
            kitty(w.out, cap.mode);
        }
    }
    emit_modes(w.out, cap.mode);
    // DECSCUSR; 0 drops a program-set style so the configured default applies.
    w.out.extend_from_slice(b"\x1b[");
    match cap.cursor_style {
        Some(style) => {
            let base = match style.shape {
                CursorShape::Underline => 3,
                CursorShape::Beam => 5,
                _ => 1,
            };
            push_num(w.out, base + usize::from(!style.blinking));
        }
        None => w.out.push(b'0'),
    }
    w.out.extend_from_slice(b" q");
    emit_metadata(w.out, cap.metadata);
}

/// The alt grid and live kitty flags set aside while the primary grid is
/// swapped in.
struct AltStash {
    grid: Grid<Cell>,
    mode: TermMode,
}

impl TerminalEmulator {
    /// While the alt screen is active, swap the primary grid in (the alt grid is
    /// cloned aside). Pair with [`Self::leave_primary`].
    fn enter_primary(&mut self) -> Option<AltStash> {
        let mode = *self.term.mode();
        mode.contains(TermMode::ALT_SCREEN).then(|| {
            let grid = self.term.grid().clone();
            self.term.swap_alt();
            AltStash { grid, mode }
        })
    }

    fn leave_primary(&mut self, stash: Option<AltStash>) {
        if let Some(stash) = stash {
            // Swapping back clears the alt grid and re-derives the kitty flags
            // from the keyboard-mode stack: restore both.
            self.term.swap_alt();
            *self.term.grid_mut() = stash.grid;
            Handler::set_keyboard_mode(
                &mut self.term,
                KeyboardModes::from_bits_truncate(kitty_bits(stash.mode)),
                KeyboardModesApplyBehavior::Replace,
            );
        }
    }

    /// Serialize the complete emulator state as escape sequences; see the
    /// [module docs](self) for what is and is not reproduced. Feeding the
    /// result into an emulator of the same size reproduces this one.
    ///
    /// Takes `&mut self` because the primary grid is only reachable by
    /// temporarily swapping screens while the alt screen is active (the alt grid
    /// is restored; the selection is cleared in that case).
    pub fn snapshot(&mut self, opts: &SnapshotOptions) -> Vec<u8> {
        let columns = self.term.columns();
        let screen_lines = self.term.screen_lines();
        let mode = *self.term.mode();
        let stash = self.enter_primary();
        let history = self
            .term
            .grid()
            .history_size()
            .min(opts.max_scrollback_lines);
        let cursor_style = {
            let style = self.term.cursor_style();
            (style != self.default_cursor_style).then_some(style)
        };
        let cap = Capture {
            mode: stash.as_ref().map_or(mode, |stash| stash.mode),
            primary_kitty_mode: *self.term.mode(),
            region: self.tracker.region,
            tabs: self.tracker.custom_tabs(),
            cursor_style,
            metadata: &self.metadata,
            history,
        };
        let mut out = Vec::with_capacity((history + screen_lines) * (columns + 16) + 2048);
        build(
            &mut out,
            &cap,
            self.term.grid(),
            stash.as_ref().map(|stash| &stash.grid),
            columns,
            screen_lines,
        );
        self.leave_primary(stash);
        out
    }

    /// Run `f` with the primary grid and, while the alt screen is active, the
    /// alt grid. Test helper for comparing complete grid state.
    #[cfg(test)]
    fn with_grids<R>(&mut self, f: impl FnOnce(&Grid<Cell>, Option<&Grid<Cell>>) -> R) -> R {
        let stash = self.enter_primary();
        let result = f(self.term.grid(), stash.as_ref().map(|stash| &stash.grid));
        self.leave_primary(stash);
        result
    }
}

#[cfg(test)]
mod tests;
