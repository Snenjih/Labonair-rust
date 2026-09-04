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
use alacritty_terminal::index::{Column, Direction, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::search::{RegexIter, RegexSearch};
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
    /// Working directory reported via OSC 7, percent-decoded to a plain path.
    Cwd(String),
    /// OSC 133;A — the shell is about to draw a prompt.
    PromptStart,
    /// OSC 133;B — end of prompt; command-line input begins.
    PromptEnd,
    /// OSC 133;C — a foreground command started executing. Carries the literal
    /// command text when the shell runs in block mode, otherwise `None`.
    CommandStart(Option<String>),
    /// OSC 133;D — the running command finished. Carries the exit code when the
    /// shell reported one.
    CommandFinished(Option<i32>),
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

/// Prompt/command lifecycle phase derived from the OSC 133 markers. The basis
/// for block detection ("select just the last command") and for handing the AI
/// live-context reader a clean prompt/command/output split.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PromptPhase {
    /// No marker seen yet, or a command just finished.
    #[default]
    Unknown,
    /// OSC 133;A — the prompt is being drawn.
    PromptStart,
    /// OSC 133;B — accepting command-line input.
    Prompt,
    /// OSC 133;C — a command is executing.
    Executing,
}

/// Shell-integration metadata for a session, updated as OSC 7 / OSC 133 /
/// OSC 0/2 sequences arrive in the output stream (T03-004). Read via
/// [`TerminalEmulator::metadata`] and surfaced to the UI (status bar / breadcrumb,
/// inherited cwd for new tabs) and the AI live-context reader.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionMetadata {
    /// Current working directory (OSC 7), percent-decoded. Seeded with the
    /// spawn directory so it is populated before the first prompt.
    pub cwd: Option<String>,
    /// Process/window title (OSC 0/2). `None` after the shell resets it at the
    /// next prompt.
    pub title: Option<String>,
    /// `true` between an OSC 133;C and the matching A/D — a foreground command
    /// owns the tty. Gates untrusted OSC 7 emitted by command output.
    pub in_command: bool,
    /// Prompt/command lifecycle phase from the OSC 133 markers.
    pub prompt_phase: PromptPhase,
    /// Exit code of the last finished command (OSC 133;D), when reported.
    pub last_exit_code: Option<i32>,
    /// Literal text of the last executed command (OSC 133;C block mode), when
    /// reported.
    pub last_command: Option<String>,
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
    /// Inactive search matches (the active one is in `selection`), split per
    /// visible row. Drawn with a distinct "find" highlight (T18-002).
    pub search: Vec<SelectionSpan>,
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

/// A shell-integration signal recovered from the raw output stream by
/// [`OscSniffer`], before [`TerminalEmulator::feed`] folds it into
/// [`SessionMetadata`] and (for most kinds) a [`TerminalEvent`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum OscUpdate {
    /// OSC 7 — working directory (percent-decoded).
    Cwd(String),
    /// OSC 0/1/2 — process/window title (`None` = reset to default).
    Title(Option<String>),
    /// OSC 133;A.
    PromptStart,
    /// OSC 133;B.
    PromptEnd,
    /// OSC 133;C — optional literal command text (block mode).
    CommandStart(Option<String>),
    /// OSC 133;D — optional exit code.
    CommandFinished(Option<i32>),
}

/// Sniffs OSC 7 (cwd), OSC 133 A/B/C/D (shell integration) and OSC 0/1/2
/// (title) out of the raw byte stream *before* it reaches the VTE parser —
/// `alacritty_terminal` 0.24 ignores OSC 7 and 133 entirely, and we want the
/// title in the metadata too. The parser still consumes every OSC sequence, so
/// none of this ever appears in the visible grid; this sniffer is purely a
/// read-only tap (T03-001 for OSC 7, T03-004 for OSC 133 + title).
#[derive(Default)]
struct OscSniffer {
    /// Buffered bytes once we've seen `ESC ]`, until the terminator.
    buf: Vec<u8>,
    in_osc: bool,
    saw_esc: bool,
    /// `true` while a foreground command owns the tty (between OSC 133;C and
    /// the next A/D) — untrusted OSC 7 from command output is then ignored,
    /// matching the reference `registerCwdHandler`.
    in_command: bool,
}

impl OscSniffer {
    fn feed(&mut self, bytes: &[u8], out: &mut Vec<OscUpdate>) {
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
                    if self.buf.len() > 8192 {
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

    fn finish(&mut self, out: &mut Vec<OscUpdate>) {
        let payload = std::mem::take(&mut self.buf);
        self.reset();

        let Some(sep) = payload.iter().position(|&b| b == b';') else {
            return;
        };
        let (id, rest) = payload.split_at(sep);
        let rest = &rest[1..]; // drop the ';'

        match id {
            b"0" | b"1" | b"2" => {
                let title = std::str::from_utf8(rest).unwrap_or("").trim();
                out.push(OscUpdate::Title(
                    (!title.is_empty()).then(|| title.to_string()),
                ));
            }
            b"7" => {
                if self.in_command {
                    return; // untrusted OSC 7 from command output
                }
                if let Ok(s) = std::str::from_utf8(rest) {
                    if let Some(path) = parse_osc7(s) {
                        if !path.is_empty() {
                            out.push(OscUpdate::Cwd(path));
                        }
                    }
                }
            }
            b"133" => {
                let (kind, arg) = match rest.iter().position(|&b| b == b';') {
                    Some(p) => (&rest[..p], Some(&rest[p + 1..])),
                    None => (rest, None),
                };
                match kind {
                    b"A" => {
                        self.in_command = false;
                        out.push(OscUpdate::PromptStart);
                    }
                    b"B" => out.push(OscUpdate::PromptEnd),
                    b"C" => {
                        self.in_command = true;
                        let text = arg
                            .and_then(|a| std::str::from_utf8(a).ok())
                            .filter(|s| !s.is_empty())
                            .map(str::to_string);
                        out.push(OscUpdate::CommandStart(text));
                    }
                    b"D" => {
                        self.in_command = false;
                        let code = arg
                            .and_then(|a| std::str::from_utf8(a).ok())
                            .and_then(|s| s.trim().parse::<i32>().ok());
                        out.push(OscUpdate::CommandFinished(code));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.in_osc = false;
        self.saw_esc = false;
    }
}

/// Parse an OSC 7 payload (`file://host/percent%20encoded/path`) into a plain,
/// percent-decoded filesystem path. Mirrors the reference `parseOsc7`
/// (`^file://[^/]*(/.*)$` + `decodeURIComponent`).
fn parse_osc7(data: &str) -> Option<String> {
    let rest = data.strip_prefix("file://")?;
    let slash = rest.find('/')?;
    Some(percent_decode(&rest[slash..]))
}

/// Percent-decode a string (`%20` → space). Undecodable byte pairs are left
/// verbatim; the result is UTF-8 lossily reconstructed.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 3 <= bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
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
    metadata: SessionMetadata,
    search: SearchState,
}

/// Literal (non-regex) scrollback search state (T18-002). `matches` hold
/// absolute grid points; the active match is mirrored into `term.selection` so
/// the renderer highlights it like a normal selection, while the rest are
/// exposed as [`RenderableScreen::search`] spans.
#[derive(Default)]
struct SearchState {
    query: String,
    case_sensitive: bool,
    matches: Vec<std::ops::RangeInclusive<Point>>,
    active: Option<usize>,
}

/// Escape `q` into a regex-automata pattern that matches it literally.
///
/// [`RegexSearch::new`] derives its own case-insensitivity from whether the
/// *pattern* contains an uppercase character (alacritty's smart-case), which
/// would silently override an explicit case-sensitive request whenever the
/// query itself happens to be all-lowercase. An explicit `(?i)` / `(?-i)`
/// prefix pins the flag regardless of that heuristic.
fn to_regex_literal(q: &str, case_sensitive: bool) -> String {
    let mut out = String::new();
    out.push_str(if case_sensitive { "(?-i)" } else { "(?i)" });
    for c in q.chars() {
        if "\\.+*?()|[]{}^$#&~-".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Tunable emulator parameters sourced from the app preferences (T13-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmulatorConfig {
    /// Lines of scrollback history kept above the viewport.
    pub scrollback: usize,
    /// Default cursor shape (until a program overrides it via DECSCUSR).
    pub cursor_shape: CursorShape,
    /// Whether the default cursor blinks.
    pub cursor_blink: bool,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            scrollback: DEFAULT_SCROLLBACK_LINES,
            cursor_shape: CursorShape::Block,
            cursor_blink: false,
        }
    }
}

impl TerminalEmulator {
    /// Create an emulator with the given palette and initial size. Events go to
    /// `event_tx`.
    pub fn new(
        colors: TerminalColors,
        dimensions: TermDimensions,
        event_tx: Sender<TerminalEvent>,
    ) -> Self {
        Self::new_with(colors, dimensions, event_tx, EmulatorConfig::default())
    }

    /// Like [`Self::new`] but with explicit [`EmulatorConfig`] tunables.
    pub fn new_with(
        colors: TerminalColors,
        dimensions: TermDimensions,
        event_tx: Sender<TerminalEvent>,
        cfg: EmulatorConfig,
    ) -> Self {
        let config = Config {
            scrolling_history: cfg.scrollback.max(1),
            default_cursor_style: alacritty_terminal::vte::ansi::CursorStyle {
                shape: cfg.cursor_shape,
                blinking: cfg.cursor_blink,
            },
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
            metadata: SessionMetadata::default(),
            search: SearchState::default(),
        }
    }

    /// Seed the working directory before the first prompt (the shell's spawn
    /// cwd). Later OSC 7 reports overwrite it.
    pub fn set_initial_cwd(&mut self, cwd: impl Into<String>) {
        let cwd = cwd.into();
        if !cwd.is_empty() {
            self.metadata.cwd = Some(cwd);
        }
    }

    /// Shell-integration metadata (cwd, title, prompt/command state), updated
    /// on every [`Self::feed`] from the OSC 7 / 133 / 0-2 sequences.
    pub fn metadata(&self) -> &SessionMetadata {
        &self.metadata
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
        let mut updates = Vec::new();
        self.sniffer.feed(bytes, &mut updates);
        let mut extra = Vec::with_capacity(updates.len() + 1);
        for update in updates {
            match update {
                OscUpdate::Cwd(path) => {
                    self.metadata.cwd = Some(path.clone());
                    extra.push(TerminalEvent::Cwd(path));
                }
                OscUpdate::Title(title) => {
                    self.metadata.title = title;
                }
                OscUpdate::PromptStart => {
                    self.metadata.in_command = false;
                    self.metadata.prompt_phase = PromptPhase::PromptStart;
                    extra.push(TerminalEvent::PromptStart);
                }
                OscUpdate::PromptEnd => {
                    self.metadata.prompt_phase = PromptPhase::Prompt;
                    extra.push(TerminalEvent::PromptEnd);
                }
                OscUpdate::CommandStart(text) => {
                    self.metadata.in_command = true;
                    self.metadata.prompt_phase = PromptPhase::Executing;
                    if text.is_some() {
                        self.metadata.last_command = text.clone();
                    }
                    extra.push(TerminalEvent::CommandStart(text));
                }
                OscUpdate::CommandFinished(code) => {
                    self.metadata.in_command = false;
                    self.metadata.prompt_phase = PromptPhase::Unknown;
                    self.metadata.last_exit_code = code;
                    extra.push(TerminalEvent::CommandFinished(code));
                }
            }
        }
        for &byte in bytes {
            self.parser.advance(&mut self.term, byte);
        }
        if !self.search.query.is_empty() {
            self.refresh_search_matches();
        }
        extra.push(TerminalEvent::Wakeup);
        extra
    }

    // ── Literal scrollback search (T18-002) ───────────────────────────────

    /// Start / update a literal search over the whole buffer (scrollback +
    /// screen). Selects the first match at or after the current viewport top
    /// and scrolls it into view. Returns `(current_1_based, total)` — `(0, n)`
    /// when there are no matches, `(0, 0)` on an empty query.
    pub fn search_set(&mut self, query: &str, case_sensitive: bool) -> (usize, usize) {
        self.search.query = query.to_string();
        self.search.case_sensitive = case_sensitive;
        self.search.matches.clear();
        self.search.active = None;
        if query.is_empty() {
            self.term.selection = None;
            return (0, 0);
        }
        self.compute_search_matches();
        let offset = self.term.grid().display_offset() as i32;
        let top = Point::new(Line(-offset), Column(0));
        self.search.active = self
            .search
            .matches
            .iter()
            .position(|m| *m.start() >= top)
            .or_else(|| (!self.search.matches.is_empty()).then_some(0));
        self.focus_active_match();
        self.search_count()
    }

    /// Move to the next / previous match (wrapping). Returns `(current, total)`.
    pub fn search_step(&mut self, forward: bool) -> (usize, usize) {
        let n = self.search.matches.len();
        if n == 0 {
            return (0, 0);
        }
        let cur = self.search.active.unwrap_or(0);
        let next = if forward {
            (cur + 1) % n
        } else {
            (cur + n - 1) % n
        };
        self.search.active = Some(next);
        self.focus_active_match();
        (next + 1, n)
    }

    /// Drop all search state and clear the match selection.
    pub fn search_clear(&mut self) {
        self.search = SearchState::default();
        self.term.selection = None;
    }

    /// `(current_1_based, total)` for the active search (`0` current = none).
    pub fn search_count(&self) -> (usize, usize) {
        (
            self.search.active.map(|i| i + 1).unwrap_or(0),
            self.search.matches.len(),
        )
    }

    /// Re-run the matcher after buffer mutation, keeping the active index in
    /// range.
    fn refresh_search_matches(&mut self) {
        self.compute_search_matches();
        match self.search.active {
            Some(i) if i >= self.search.matches.len() => {
                self.search.active =
                    (!self.search.matches.is_empty()).then_some(self.search.matches.len() - 1);
            }
            None if !self.search.matches.is_empty() => self.search.active = Some(0),
            _ => {}
        }
    }

    fn compute_search_matches(&mut self) {
        self.search.matches.clear();
        let pattern = to_regex_literal(&self.search.query, self.search.case_sensitive);
        let Ok(mut regex) = RegexSearch::new(&pattern) else {
            return;
        };
        let grid = self.term.grid();
        let start = Point::new(grid.topmost_line(), Column(0));
        let end = Point::new(grid.bottommost_line(), grid.last_column());
        let iter = RegexIter::new(start, end, Direction::Right, &self.term, &mut regex);
        for m in iter {
            self.search.matches.push(m);
            if self.search.matches.len() >= 10_000 {
                break;
            }
        }
    }

    fn focus_active_match(&mut self) {
        let Some(m) = self
            .search
            .active
            .and_then(|i| self.search.matches.get(i).cloned())
        else {
            self.term.selection = None;
            return;
        };
        let mut selection = Selection::new(SelectionType::Simple, *m.start(), Side::Left);
        selection.update(*m.end(), Side::Right);
        self.term.selection = Some(selection);
        self.term.scroll_to_point(*m.start());
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

        let mut search = Vec::new();
        let offset = display_offset as i32;
        let columns = self.dimensions.columns;
        let screen_lines = self.dimensions.screen_lines as i32;
        for (idx, m) in self.search.matches.iter().enumerate() {
            if self.search.active == Some(idx) {
                continue;
            }
            let (s, e) = (*m.start(), *m.end());
            let s_row = s.line.0 + offset;
            let e_row = e.line.0 + offset;
            for row in s_row.max(0)..=e_row.min(screen_lines - 1) {
                let start_col = if row == s_row { s.column.0 } else { 0 };
                let end_col = if row == e_row {
                    e.column.0 + 1
                } else {
                    columns
                };
                let start_col = start_col.min(columns);
                let end_col = end_col.min(columns);
                if end_col > start_col {
                    search.push(SelectionSpan {
                        line: row as usize,
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
            search,
        }
    }

    /// Serialize the scrollback history plus the visible screen as plain text,
    /// one grid row per line, `\r\n`-joined, with trailing blanks trimmed and
    /// leading blank lines dropped. Feeds the session scrollback-persistence
    /// layer (T14-002).
    ///
    /// `max_lines` caps how many of the most recent rows are kept (`None` or
    /// `Some(0)` = all). Returns an empty string while a full-screen app holds
    /// the alternate screen — a TUI's transient buffer is not history worth
    /// persisting.
    pub fn serialize_scrollback(&self, max_lines: Option<usize>) -> String {
        if self.is_alt_screen() {
            return String::new();
        }
        let grid = self.term.grid();
        let history = grid.history_size() as i32;
        let screen_lines = self.dimensions.screen_lines as i32;
        let cols = self.dimensions.columns;
        let total = history + screen_lines;
        let keep = match max_lines {
            Some(0) | None => total,
            Some(m) => (m as i32).min(total),
        }
        .max(1);
        let first = screen_lines - keep;
        let mut lines: Vec<String> = Vec::with_capacity(keep.max(0) as usize);
        for l in first..screen_lines {
            let row = &grid[Line(l)];
            let mut s = String::new();
            for c in 0..cols {
                s.push(row[Column(c)].c);
            }
            lines.push(s.trim_end().to_string());
        }
        let start = lines
            .iter()
            .position(|l| !l.is_empty())
            .unwrap_or(lines.len());
        lines[start..].join("\r\n")
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
    fn emulator_config_caps_scrollback_history() {
        let (tx, _rx) = channel();
        let colors = TerminalColors::from_theme(&labonair_theme::Theme::dark());
        let mut term = TerminalEmulator::new_with(
            colors,
            TermDimensions::new(20, 5),
            tx,
            EmulatorConfig {
                scrollback: 5,
                ..EmulatorConfig::default()
            },
        );
        for _ in 0..100 {
            term.feed(b"line\r\n");
        }
        assert!(
            term.history_len() <= 5,
            "history {} exceeded configured cap",
            term.history_len()
        );
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
    fn osc7_percent_decodes_and_updates_metadata() {
        let (mut term, _rx) = emulator(20, 5);
        let events = term.feed(b"\x1b]7;file://host/Users/me/my%20dev%2Fdir\x07");
        assert!(events.contains(&TerminalEvent::Cwd("/Users/me/my dev/dir".into())));
        assert_eq!(term.metadata().cwd.as_deref(), Some("/Users/me/my dev/dir"));
    }

    #[test]
    fn osc133_markers_model_the_prompt_lifecycle() {
        let (mut term, _rx) = emulator(20, 5);

        let e = term.feed(b"\x1b]133;A\x1b\\");
        assert!(e.contains(&TerminalEvent::PromptStart));
        assert_eq!(term.metadata().prompt_phase, PromptPhase::PromptStart);

        term.feed(b"\x1b]133;B\x1b\\");
        assert_eq!(term.metadata().prompt_phase, PromptPhase::Prompt);

        let e = term.feed(b"\x1b]133;C;git status\x1b\\");
        assert!(e.contains(&TerminalEvent::CommandStart(Some("git status".into()))));
        assert!(term.metadata().in_command);
        assert_eq!(term.metadata().last_command.as_deref(), Some("git status"));

        let e = term.feed(b"\x1b]133;D;3\x1b\\");
        assert!(e.contains(&TerminalEvent::CommandFinished(Some(3))));
        assert!(!term.metadata().in_command);
        assert_eq!(term.metadata().last_exit_code, Some(3));
        assert_eq!(term.metadata().prompt_phase, PromptPhase::Unknown);
    }

    #[test]
    fn bare_osc133_c_and_d_carry_no_payload() {
        let (mut term, _rx) = emulator(20, 5);
        let e = term.feed(b"\x1b]133;C\x1b\\");
        assert!(e.contains(&TerminalEvent::CommandStart(None)));
        let e = term.feed(b"\x1b]133;D\x1b\\");
        assert!(e.contains(&TerminalEvent::CommandFinished(None)));
    }

    #[test]
    fn osc7_from_command_output_is_ignored_while_a_command_runs() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"\x1b]7;file://host/home\x07");
        term.feed(b"\x1b]133;C\x1b\\");
        // Untrusted OSC 7 emitted by the running command.
        let e = term.feed(b"\x1b]7;file://host/evil\x07");
        assert!(!e.iter().any(|e| matches!(e, TerminalEvent::Cwd(_))));
        assert_eq!(term.metadata().cwd.as_deref(), Some("/home"));
        // Once the prompt returns, OSC 7 is trusted again.
        term.feed(b"\x1b]133;A\x1b\\");
        term.feed(b"\x1b]7;file://host/work\x07");
        assert_eq!(term.metadata().cwd.as_deref(), Some("/work"));
    }

    #[test]
    fn osc0_title_updates_metadata_and_resets_to_none() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"\x1b]2;my session\x07");
        assert_eq!(term.metadata().title.as_deref(), Some("my session"));
        term.feed(b"\x1b]0;\x07");
        assert_eq!(term.metadata().title, None);
    }

    #[test]
    fn shell_integration_sequences_do_not_appear_in_the_grid() {
        let (mut term, _rx) = emulator(40, 5);
        term.feed(b"\x1b]133;A\x1b\\$ \x1b]133;B\x1b\\\x1b]133;C;ls\x1b\\");
        term.feed(b"\r\nfile.txt\r\n\x1b]133;D;0\x1b\\");
        term.feed(b"\x1b]7;file://host/tmp\x07");
        let text = term.render().to_text();
        assert!(!text.contains("133"));
        assert!(!text.contains("file://"));
        assert!(text.contains("file.txt"));
    }

    #[test]
    fn initial_cwd_is_used_before_the_first_prompt() {
        let (mut term, _rx) = emulator(20, 5);
        term.set_initial_cwd("/Users/me/project");
        assert_eq!(term.metadata().cwd.as_deref(), Some("/Users/me/project"));
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
    fn serialize_scrollback_captures_history_and_visible_rows() {
        let (mut term, _rx) = emulator(12, 3);
        for i in 0..20 {
            term.feed(format!("line{i}\r\n").as_bytes());
        }
        let all = term.serialize_scrollback(None);
        assert!(all.contains("line0"), "oldest line missing:\n{all}");
        assert!(all.contains("line19"), "newest line missing:\n{all}");
        assert!(all.contains("\r\n"), "rows must be CRLF-joined");
        // No trailing spaces on padded rows.
        assert!(all.lines().all(|l| l == l.trim_end()));

        // max_lines keeps only the most recent rows.
        let tail = term.serialize_scrollback(Some(3));
        assert!(!tail.contains("line0"));
        assert!(tail.contains("line19"));
        assert!(tail.lines().count() <= 3);
    }

    #[test]
    fn serialize_scrollback_is_empty_on_the_alternate_screen() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"visible history\r\n");
        term.feed(b"\x1b[?1049h");
        assert_eq!(term.serialize_scrollback(None), "");
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

    #[test]
    fn search_set_counts_matches_and_selects_one() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"foo bar foo baz foo\r\n");
        let (current, total) = term.search_set("foo", false);
        assert_eq!(total, 3);
        assert_eq!(current, 1);
        assert!(term.render().selection.iter().any(|s| s.start_col == 0));
    }

    #[test]
    fn search_step_wraps_and_updates_the_selection() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"foo bar foo baz foo\r\n");
        term.search_set("foo", false);
        let (second, total) = term.search_step(true);
        assert_eq!((second, total), (2, 3));
        let (third, _) = term.search_step(true);
        assert_eq!(third, 3);
        // Wraps back to the first match.
        let (wrapped, _) = term.search_step(true);
        assert_eq!(wrapped, 1);
        let (prev, _) = term.search_step(false);
        assert_eq!(prev, 3);
    }

    #[test]
    fn search_is_case_sensitive_when_requested() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"Foo foo FOO\r\n");
        assert_eq!(term.search_set("foo", false).1, 3);
        assert_eq!(term.search_set("foo", true).1, 1);
    }

    #[test]
    fn search_finds_matches_in_scrollback_history() {
        let (mut term, _rx) = emulator(20, 3);
        term.feed(b"NEEDLE\r\n");
        for _ in 0..50 {
            term.feed(b"filler\r\n");
        }
        assert!(term.history_len() > 0);
        let (current, total) = term.search_set("NEEDLE", false);
        assert_eq!(total, 1);
        assert_eq!(current, 1);
    }

    #[test]
    fn search_clear_drops_matches_and_selection() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"foo foo\r\n");
        term.search_set("foo", false);
        assert_eq!(term.search_count(), (1, 2));
        term.search_clear();
        assert_eq!(term.search_count(), (0, 0));
        assert!(term.render().selection.is_empty());
    }

    #[test]
    fn empty_query_clears_search() {
        let (mut term, _rx) = emulator(20, 5);
        term.feed(b"foo foo\r\n");
        term.search_set("foo", false);
        assert_eq!(term.search_set("", false), (0, 0));
        assert!(term.render().selection.is_empty());
    }
}
