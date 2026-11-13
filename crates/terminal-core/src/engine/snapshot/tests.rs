//! Round-trip verification for [`TerminalEmulator::snapshot`]: emulator A ->
//! snapshot -> fresh emulator B must give the same state, and keep behaving
//! the same (continuation probes) afterwards.

use super::*;
use crate::{EmulatorConfig, TerminalColors, TerminalEvent};
use alacritty_terminal::grid::Cursor;
use alacritty_terminal::index::{Column, Line};
use std::sync::mpsc::sync_channel;

fn emu_with(cols: usize, rows: usize, cfg: EmulatorConfig) -> TerminalEmulator {
    let (tx, rx) = sync_channel::<TerminalEvent>(256);
    // Semantic events apply backpressure; keep the queue drained.
    std::thread::spawn(move || for _ in rx {});
    TerminalEmulator::new_with(
        TerminalColors::xterm_default(),
        TermDimensions::new(cols, rows),
        tx,
        cfg,
    )
}

fn emu(cols: usize, rows: usize) -> TerminalEmulator {
    emu_with(cols, rows, EmulatorConfig::default())
}

/// Complete cell-level state of one screen.
#[derive(Debug, PartialEq)]
struct ScreenDump {
    rows: Vec<Vec<Cell>>,
    cursor: Cursor<Cell>,
    saved: Cursor<Cell>,
}

/// Cell-level dump with the engine artifacts that cannot (and need not) be
/// reproduced normalized away. All of them are states `alacritty_terminal`
/// leaves behind after reflow/resize or after erases cut through a wide char,
/// and that only matter for a few cells of a few rows:
/// * stale `WRAPLINE` flags (anywhere but the last column, on the bottom row,
///   or under a pending-wrap cursor, whose cell is rewritten),
/// * spacers: colors of (leading) wide-char spacers, orphaned spacers and wide
///   chars without a spacer,
/// * a pending-wrap flag on a cursor that is not in the last column,
/// * a tab character in the last column (approximated by a blank).
fn dump(grid: &Grid<Cell>, wrap_flags: bool) -> ScreenDump {
    let history = grid.history_size() as i32;
    let (columns, last_line) = (grid.columns(), grid.screen_lines() as i32 - 1);
    let mut rows: Vec<Vec<Cell>> = (-history..=last_line)
        .map(|line| {
            let mut cells: Vec<Cell> = (0..columns)
                .map(|column| grid[Line(line)][Column(column)].clone())
                .collect();
            let spacer = Cell {
                c: ' ',
                flags: Flags::WIDE_CHAR_SPACER,
                ..Cell::default()
            };
            for (column, cell) in cells.iter_mut().enumerate() {
                if !wrap_flags || column + 1 != columns || line == last_line {
                    cell.flags.remove(Flags::WRAPLINE);
                }
                if cell.c == '\t' && column + 1 == columns {
                    cell.c = ' ';
                }
                if cell.flags.contains(Flags::WIDE_CHAR) && column + 1 == columns {
                    cell.flags.remove(Flags::WIDE_CHAR);
                    cell.c = ' ';
                }
                if cell.flags.contains(Flags::LEADING_WIDE_CHAR_SPACER) {
                    if column + 1 == columns {
                        cell.fg = DEFAULT_FG;
                        cell.bg = DEFAULT_BG;
                        cell.flags &= Flags::LEADING_WIDE_CHAR_SPACER | Flags::WRAPLINE;
                        cell.extra = None;
                    } else {
                        cell.flags.remove(Flags::LEADING_WIDE_CHAR_SPACER);
                    }
                }
            }
            let mut follows_wide = false;
            for cell in &mut cells {
                let wide = cell.flags.contains(Flags::WIDE_CHAR);
                if follows_wide {
                    *cell = spacer.clone();
                } else if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    cell.flags.remove(Flags::WIDE_CHAR_SPACER);
                }
                follows_wide = wide;
            }
            cells
        })
        .collect();
    let normalize_cursor = |cursor: &Cursor<Cell>, rows: &mut Vec<Vec<Cell>>| {
        let mut cursor = cursor.clone();
        if cursor.point.column.0 + 1 != columns {
            cursor.input_needs_wrap = false;
        }
        if cursor.input_needs_wrap {
            let row = (cursor.point.line.0 + history) as usize;
            rows[row][columns - 1].flags.remove(Flags::WRAPLINE);
        }
        cursor
    };
    let cursor = normalize_cursor(&grid.cursor, &mut rows);
    let saved = normalize_cursor(&grid.saved_cursor, &mut rows);
    ScreenDump {
        rows,
        cursor,
        saved,
    }
}

fn dumps(emulator: &mut TerminalEmulator, wrap_flags: bool) -> (ScreenDump, Option<ScreenDump>) {
    emulator.with_grids(|primary, alt| {
        (
            dump(primary, wrap_flags),
            alt.map(|alt| dump(alt, wrap_flags)),
        )
    })
}

fn first_difference(a: &ScreenDump, b: &ScreenDump) -> String {
    if a.rows.len() != b.rows.len() {
        return format!("row count {} vs {}", a.rows.len(), b.rows.len());
    }
    for (line, (ra, rb)) in a.rows.iter().zip(&b.rows).enumerate() {
        for (column, (ca, cb)) in ra.iter().zip(rb).enumerate() {
            if ca != cb {
                return format!("row {line} col {column}: {ca:?} vs {cb:?}");
            }
        }
    }
    if a.cursor != b.cursor {
        return format!("cursor {:?} vs {:?}", a.cursor, b.cursor);
    }
    format!("saved cursor {:?} vs {:?}", a.saved, b.saved)
}

fn escape(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|&b| std::ascii::escape_default(b))
        .map(char::from)
        .collect()
}

/// Assert everything observable matches.
fn assert_same(a: &mut TerminalEmulator, b: &mut TerminalEmulator, what: &str) {
    assert_same_with(a, b, true, what);
}

/// `wrap_flags = false` ignores soft-wrap flags: after a probe moved the cursor,
/// a flag lost to the pending-wrap rewrite (see the module docs) may show up
/// as a difference on a cell the cursor no longer sits on.
fn assert_same_with(
    a: &mut TerminalEmulator,
    b: &mut TerminalEmulator,
    wrap_flags: bool,
    what: &str,
) {
    assert_eq!(a.dimensions(), b.dimensions(), "{what}: dimensions");
    assert_eq!(a.is_alt_screen(), b.is_alt_screen(), "{what}: alt screen");
    assert_eq!(a.history_len(), b.history_len(), "{what}: history_len");
    assert_eq!(a.mode_state(), b.mode_state(), "{what}: mode_state");
    assert_eq!(a.term.mode(), b.term.mode(), "{what}: TermMode");
    assert_eq!(
        a.term.cursor_style(),
        b.term.cursor_style(),
        "{what}: cursor style"
    );
    assert_eq!(a.metadata(), b.metadata(), "{what}: metadata");
    let (pa, aa) = dumps(a, wrap_flags);
    let (pb, ab) = dumps(b, wrap_flags);
    assert!(
        pa == pb,
        "{what}: primary grid differs at {}",
        first_difference(&pa, &pb)
    );
    match (&aa, &ab) {
        (Some(x), Some(y)) => assert!(
            x == y,
            "{what}: alt grid differs at {}",
            first_difference(x, y)
        ),
        (None, None) => {}
        _ => panic!("{what}: alt grid presence differs"),
    }
    let (ra, rb) = (a.render(), b.render());
    assert_eq!(ra.cursor, rb.cursor, "{what}: render cursor");
    assert_eq!(ra.cells, rb.cells, "{what}: render cells");
    assert_eq!(a.tracker.region, b.tracker.region, "{what}: scroll region");
    assert_eq!(
        a.tracker.custom_tabs(),
        b.tracker.custom_tabs(),
        "{what}: tab stops"
    );
}

/// Behavioral probes: they exercise state the engine keeps private (scroll
/// region, tab stops, charsets, wrap, saved cursor, the primary screen behind
/// an alt screen) so a silent difference shows up as a grid difference.
const PROBE_STAY: &[u8] = b"ab\tcd\tef\tg\x1b[999;999Hxyz\x1b[H\x1bM\x1bM\x1b8Z\x1b[3;3Hq\x1b[999;1H\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nend\x1b[1;1H\x1bM\x1bMtop";
const PROBE_LEAVE: &[u8] =
    b"\x1b[?1049l\r\n\r\nafter\tprobe\x1b8Z\x1b[999;1H\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n\nend";

fn feed_chunked(emulator: &mut TerminalEmulator, bytes: &[u8], seed: u64) {
    let mut rng = Lcg(seed ^ 0x9e37_79b9);
    let mut rest = bytes;
    while !rest.is_empty() {
        let n = (rng.below(40) as usize + 1).min(rest.len());
        emulator.feed(&rest[..n]);
        rest = &rest[n..];
    }
}

/// The core check. Returns the snapshot bytes.
fn roundtrip_with(
    input: &[u8],
    cols: usize,
    rows: usize,
    opts: &SnapshotOptions,
    what: &str,
) -> Vec<u8> {
    let mut a = emu(cols, rows);
    a.feed(input);
    roundtrip_emulator(&mut a, opts, what)
}

fn roundtrip_emulator(a: &mut TerminalEmulator, opts: &SnapshotOptions, what: &str) -> Vec<u8> {
    roundtrip_probed(a, opts, true, true, what)
}

/// `probe = false` skips the continuation probes and `exact = false` the
/// "snapshot of the restored emulator is byte-identical" check (see the fuzz
/// test for why).
fn roundtrip_probed(
    a: &mut TerminalEmulator,
    opts: &SnapshotOptions,
    probe: bool,
    exact: bool,
    what: &str,
) -> Vec<u8> {
    let dims = a.dimensions();
    // Snapshotting in the alt screen normalizes the hidden primary grid's saved
    // cursor to its cursor (the value it had at alt entry); warm up so the
    // "unchanged" check below compares like with like.
    a.with_grids(|_, _| ());
    let before = dumps(a, true);
    let snapshot = a.snapshot(opts);
    assert_eq!(
        before,
        dumps(a, true),
        "{what}: snapshot changed the source emulator"
    );

    let mut b = emu(dims.columns, dims.screen_lines);
    b.feed(&snapshot);
    assert_same(
        a,
        &mut b,
        &format!("{what} [snapshot {} B]", snapshot.len()),
    );
    // A snapshot of the restored emulator is byte-identical.
    if exact {
        assert_eq!(
            escape(&snapshot),
            escape(&b.snapshot(opts)),
            "{what}: re-snapshot differs"
        );
    }

    if probe {
        a.feed(PROBE_STAY);
        b.feed(PROBE_STAY);
        assert_same_with(a, &mut b, false, &format!("{what} [after stay probe]"));
        a.feed(PROBE_LEAVE);
        b.feed(PROBE_LEAVE);
        assert_same_with(a, &mut b, false, &format!("{what} [after leave probe]"));
    }
    snapshot
}

fn roundtrip(input: &[u8], what: &str) {
    roundtrip_with(input, 40, 12, &SnapshotOptions::default(), what);
}

fn lines(n: usize) -> String {
    (0..n)
        .map(|i| format!("line {i:03} of output\r\n"))
        .collect()
}

#[test]
fn snapshot_plain_text_and_scrollback() {
    roundtrip(b"", "empty");
    roundtrip(b"hello", "hello");
    roundtrip(lines(60).as_bytes(), "60 lines");
    roundtrip(lines(5).as_bytes(), "5 lines");
}

#[test]
fn snapshot_sgr_colors_and_attributes() {
    let mut s = String::new();
    for (i, code) in [
        "1",
        "2",
        "3",
        "4",
        "4:2",
        "4:3",
        "4:4",
        "4:5",
        "7",
        "8",
        "9",
        "1;3;4",
        "31",
        "42",
        "91",
        "104",
        "38;5;202",
        "48;5;17",
        "38;2;10;200;30",
        "48;2;250;5;99",
        "4;58;2;255;0;0",
        "4:3;58;5;9",
        "1;38;2;1;2;3;48;5;100;7",
    ]
    .iter()
    .enumerate()
    {
        s += &format!("\x1b[{code}mstyle {i}\x1b[0m plain\r\n");
    }
    s += "\x1b[44mbackground to eol\x1b[K\x1b[0m\r\n\x1b[41m\x1b[2K\x1b[0m\r\n";
    roundtrip(s.as_bytes(), "sgr");
    // Trailing attributes that were never reset: the cursor template matters.
    roundtrip(b"abc\x1b[1;4;38;2;9;8;7;48;5;55m", "sgr template");
    roundtrip(b"\x1b[7mabc\x1b[0m\x1b[5;5H\x1b[42m", "template after cup");
}

#[test]
fn snapshot_wide_cjk_emoji_and_combining() {
    let mut s = String::new();
    for _ in 0..4 {
        s += "日本語のテキスト、全角文字 😀🎉🚀 e\u{301}a\u{308} ok\r\n";
    }
    // Odd width forces a wide char to wrap with a leading spacer.
    s += &"x".repeat(39);
    s += "日本語\r\n";
    s += &"y".repeat(78);
    s += "界界界";
    roundtrip(s.as_bytes(), "wide");
    roundtrip("日本語".as_bytes(), "wide cursor");
}

#[test]
fn snapshot_cursor_movement_save_restore_and_pending_wrap() {
    roundtrip(
        b"\x1b[3;5Hx\x1b[2Ay\x1b[3Bz\x1b[10Cw\x1b[4D\x1b[2G\x1b[7d!",
        "movement",
    );
    roundtrip(b"\x1b[2;3H\x1b7\x1b[1;1H\x1b[31mred\x1b[10;10H", "decsc");
    roundtrip(b"\x1b[2;3H\x1b[s\x1b[1;1Hred\x1b[u", "scosc");
    let full = "q".repeat(40);
    roundtrip(full.as_bytes(), "pending wrap at last column");
    roundtrip(format!("{full}日").as_bytes(), "pending wrap then wide");
    roundtrip(
        format!("\x1b[3;1H{}日\x1b[5;5H", "z".repeat(38)).as_bytes(),
        "wide at last cols",
    );
    roundtrip(
        format!("{}日", "z".repeat(38)).as_bytes(),
        "wide at the end, pending wrap",
    );
}

#[test]
fn snapshot_wrapped_lines_and_tabs() {
    let long = format!("{}\r\n", "0123456789".repeat(9));
    roundtrip(long.as_bytes(), "wrapped");
    roundtrip(b"a\tb\tc\r\n\td\r\n\x1b[4mx\ty\x1b[0m\r\n", "tabs in cells");
    roundtrip(
        b"\x1b[3g\x1b[1;5H\x1bH\x1b[1;13H\x1bH\x1b[1;1Ha\tb\tc\t",
        "custom tab stops",
    );
    roundtrip(
        b"\x1b[1;5H\x1bH\x1b[g\x1b[1;9H\x1b[g",
        "tab set then cleared",
    );
}

#[test]
fn snapshot_scroll_regions_and_origin_mode() {
    roundtrip(format!("\x1b[3;8r{}", lines(30)).as_bytes(), "region");
    roundtrip(b"\x1b[3;8r\x1b[?6h\x1b[2;2Hx\x1b[999;1Hy", "origin");
    roundtrip(
        b"\x1b[3;8r\x1b[?6h\x1b[2;2Hx\x1b7\x1b[1;1H\x1b[?6l",
        "origin off with region",
    );
    roundtrip(b"\x1b[3;8r\x1b[1;1H\x1b[r", "region reset");
    roundtrip(b"\x1b[5;3r\x1b[2;4r", "invalid then valid region");
    roundtrip(b"\x1b[3;8r\x1b[?3h", "deccolm clears region");
}

#[test]
fn snapshot_alt_screen_enter_and_leave() {
    let mut s = lines(20);
    s += "\x1b[3;8r\x1b[?25l\x1b[31mred prompt$ \x1b[0m";
    roundtrip(s.as_bytes(), "primary only");
    s += "\x1b[?1049h\x1b[H\x1b[2J";
    for i in 0..12 {
        s += &format!("\x1b[{};1H\x1b[7malt row {i}\x1b[0m", i + 1);
    }
    s += "\x1b7\x1b[5;5H\x1b[33malt cursor";
    roundtrip(s.as_bytes(), "in alt screen");
    let mut left = s.clone();
    left += "\x1b[?1049lback on primary";
    roundtrip(left.as_bytes(), "left alt screen");
    // Alt screen that never wrote: primary state behind it must survive.
    roundtrip(
        format!("{}\x1b[?1049h", lines(15)).as_bytes(),
        "empty alt screen",
    );
}

/// Each mode on its own, set and then unset, plus all together.
#[test]
fn snapshot_each_mode_flag() {
    let modes: &[(&str, &str)] = &[
        ("\x1b[?1h", "\x1b[?1l"),
        ("\x1b=", "\x1b>"),
        ("\x1b[?1000h", "\x1b[?1000l"),
        ("\x1b[?1002h", "\x1b[?1002l"),
        ("\x1b[?1003h", "\x1b[?1003l"),
        ("\x1b[?1005h", "\x1b[?1005l"),
        ("\x1b[?1006h", "\x1b[?1006l"),
        ("\x1b[?1000h\x1b[?1006h", "\x1b[?1000l\x1b[?1006l"),
        ("\x1b[?1004h", "\x1b[?1004l"),
        ("\x1b[?2004h", "\x1b[?2004l"),
        ("\x1b[?7l", "\x1b[?7h"),
        ("\x1b[?1007l", "\x1b[?1007h"),
        ("\x1b[?1042l", "\x1b[?1042h"),
        ("\x1b[?25l", "\x1b[?25h"),
        ("\x1b[4h", "\x1b[4l"),
        ("\x1b[20h", "\x1b[20l"),
        ("\x1b[?6h", "\x1b[?6l"),
    ];
    let mut all_on = String::new();
    for (on, off) in modes {
        roundtrip(
            format!("text{on}more").as_bytes(),
            &format!("set {}", escape(on.as_bytes())),
        );
        roundtrip(
            format!("text{on}x{off}y").as_bytes(),
            &format!("unset {}", escape(off.as_bytes())),
        );
        // Same inside the alt screen.
        roundtrip(
            format!("primary\x1b[?1049h{on}alt").as_bytes(),
            &format!("alt set {}", escape(on.as_bytes())),
        );
        all_on += on;
    }
    roundtrip(all_on.as_bytes(), "all modes on");
    roundtrip(
        format!("{all_on}\x1b[?1049h").as_bytes(),
        "all modes on in alt",
    );
    // Cursor styles (DECSCUSR) and hidden/shown cursor.
    for n in 0..=6 {
        roundtrip(format!("ab\x1b[{n} q").as_bytes(), &format!("decscusr {n}"));
    }
    roundtrip(b"ab\x1b[?12h", "blinking via mode 12");
}

#[test]
fn snapshot_metadata_title_cwd_and_prompt_phase() {
    let cases: &[&str] = &[
        "\x1b]0;my title\x07",
        "\x1b]2;title with ; semicolons\x1b\\",
        "\x1b]7;file://host/home/me/My%20Projects/x\x07",
        "\x1b]133;A\x07$ ",
        "\x1b]133;A\x07\x1b]133;B\x07ls -la",
        "\x1b]133;A\x07\x1b]133;B\x07\x1b]133;C;ls -la\x07",
        "\x1b]133;C;make test\x07output\r\n\x1b]133;D;2\x07",
        "\x1b]133;C;make\x07\x1b]133;D;0\x07\x1b]133;A\x07$ ",
        "\x1b]133;C;make\x07\x1b]133;D\x07\x1b]133;A\x07\x1b]133;B\x07",
        "\x1b]7;file:///a\x07\x1b]133;C\x07\x1b]7;file:///ignored\x07",
        "\x1b]133;C;cmd\x07\x1b]133;B\x07",
        "\x1b]133;D;130\x07",
    ];
    for (i, case) in cases.iter().enumerate() {
        roundtrip(case.as_bytes(), &format!("metadata case {i}"));
    }
    // The seeded spawn cwd (not from an OSC) is carried too.
    let mut a = emu(40, 12);
    a.set_initial_cwd("/tmp/some dir");
    a.feed(b"hi");
    roundtrip_emulator(&mut a, &SnapshotOptions::default(), "seeded cwd");
}

#[test]
fn snapshot_hyperlinks() {
    let s = "plain \x1b]8;id=a;https://example.com/1\x1b\\link one\x1b]8;;\x1b\\ then \
             \x1b]8;;https://example.com/auto-id\x1b\\auto\x1b[31m red\x1b]8;;\x1b\\ \x1b[0m\r\n\
             \x1b]8;id=b;file:///x\x1b\\spanning\r\nrows\x1b]8;;\x1b\\";
    roundtrip(s.as_bytes(), "hyperlinks");
    roundtrip(
        b"\x1b]8;id=z;https://e.org\x1b\\open link at end of input",
        "open link template",
    );
}

#[test]
fn snapshot_resize_in_the_middle() {
    let mut a = emu(40, 12);
    a.feed(lines(30).as_bytes());
    a.feed(b"\x1b[3;8r\x1b[1;1H\x1b[4mhello\x1b[0m world, a long line that will rewrap when narrowed\r\n");
    a.resize(TermDimensions::new(25, 9));
    a.feed(b"after resize\x1b[2;2Hxx");
    roundtrip_emulator(&mut a, &SnapshotOptions::default(), "narrowed");
    a.resize(TermDimensions::new(60, 15));
    a.feed(b"\x1b[?1049hin alt after grow");
    roundtrip_emulator(&mut a, &SnapshotOptions::default(), "grown, in alt");
    a.resize(TermDimensions::new(30, 10));
    roundtrip_emulator(&mut a, &SnapshotOptions::default(), "resized in alt");
}

#[test]
fn snapshot_honours_the_scrollback_limit() {
    let input = lines(500);
    let mut a = emu(40, 12);
    a.feed(input.as_bytes());
    assert!(a.history_len() > 300);
    let opts = SnapshotOptions {
        max_scrollback_lines: 100,
    };
    let snapshot = a.snapshot(&opts);
    let mut b = emu(40, 12);
    b.feed(&snapshot);
    assert_eq!(b.history_len(), 100);
    assert_eq!(a.render().to_text(), b.render().to_text());
    // The kept rows are the newest ones.
    assert!(b
        .serialize_scrollback(None)
        .starts_with(&format!("line {:03}", 500 - 100 - 11)));
    let none = a.snapshot(&SnapshotOptions {
        max_scrollback_lines: 0,
    });
    let mut c = emu(40, 12);
    c.feed(&none);
    assert_eq!(c.history_len(), 0);
    assert_eq!(a.render().to_text(), c.render().to_text());
}

/// A snapshot starts with RIS so it also repairs an emulator in any state.
#[test]
fn snapshot_resyncs_a_dirty_emulator() {
    let mut a = emu(40, 12);
    a.feed(lines(20).as_bytes());
    a.feed(b"\x1b[3;8r\x1b[?1000h\x1b[?2004h\x1b[31mred\x1b]2;a title\x07\x1b[2;2H");
    let snapshot = a.snapshot(&SnapshotOptions::default());

    let mut dirty = emu(40, 12);
    dirty.feed(
        b"junk\x1b[?1049hmore junk\x1b[5;9r\x1b[?6h\x1b[?25l\x1b[3g\x1b[1;7H\x1bH\x1b7\x1b[44m\
          \x1b]133;C\x07\x1b[?1006h\x1b(0\x1b[3 q\x1b]2;old title\x07\x1b]8;id=q;http://x\x1b\\",
    );
    dirty.feed(&snapshot);
    assert_same(&mut a, &mut dirty, "dirty resync");
    a.feed(PROBE_STAY);
    dirty.feed(PROBE_STAY);
    assert_same(&mut a, &mut dirty, "dirty resync after probe");
}

// ── Recorded-style corpora ──────────────────────────────────────────────────

/// vim-like: alt screen, mouse + bracketed paste + focus on, app cursor/keypad,
/// scroll region, tilde rows, inverse status line, cursor shape.
fn vim_like() -> Vec<u8> {
    let mut s = String::new();
    s += "\x1b]0;main.rs (~/proj) - VIM\x07\x1b[?1049h\x1b[22;0;0t\x1b[>4;2m\x1b[?1h\x1b=\x1b[?2004h\
          \x1b[?1004h\x1b[?1000h\x1b[?1002h\x1b[?1006h\x1b[1;24r\x1b[H\x1b[2J";
    let code = [
        "\x1b[38;5;208mfn\x1b[39m \x1b[38;5;75mmain\x1b[39m() {",
        "    \x1b[38;5;208mlet\x1b[39m x = \x1b[38;5;141m42\x1b[39m;",
        "    \x1b[38;5;75mprintln!\x1b[39m(\x1b[38;5;114m\"hello 日本語\"\x1b[39m);",
        "\x1b[38;5;240m    // TODO: refactor\x1b[39m",
        "}",
    ];
    for (i, line) in code.iter().enumerate() {
        s += &format!("\x1b[{};1H{line}", i + 1);
    }
    for row in code.len() + 1..=22 {
        s += &format!("\x1b[{row};1H\x1b[94m~\x1b[39;49m");
    }
    s += "\x1b[23;1H\x1b[7m main.rs [+]                                     5,1   All \x1b[0m\x1b[24;1H\x1b[?2004h-- INSERT --";
    s += "\x1b[2 q\x1b[?25h\x1b[3;12H";
    // A later partial redraw with scrolling inside the region.
    s += "\x1b[1;22r\x1b[22;1H\x1b[2K\x1b[Sline pushed\x1b[1;24r\x1b[3;13H\x1b[?25h";
    s.into_bytes()
}

/// htop-like: full-screen redraws with BCE backgrounds, cursor hidden,
/// autowrap off, function-key bar.
fn htop_like() -> Vec<u8> {
    let mut s = String::new();
    s += "\x1b[?1049h\x1b[1;24r\x1b(B\x1b[m\x1b[4l\x1b[?7l\x1b[?25l\x1b[H\x1b[2J\x1b[?1000h\x1b[?1006h";
    for frame in 0..3 {
        s += "\x1b[H";
        s += &format!(
            "\x1b[36m  0[\x1b[32m|||||||\x1b[37m        {:>4}%\x1b[36m]\x1b[39;49m   Tasks: \x1b[1m{}\x1b(B\x1b[m, \x1b[1m\x1b[32m12 thr\x1b(B\x1b[m\r\n",
            frame * 7 + 3,
            120 + frame
        );
        s += "\x1b[36m  1[\x1b[31m|||\x1b[33m||\x1b[37m                  \x1b[36m]\x1b(B\x1b[m   Load average: 0.42 0.51 0.55\r\n";
        s += "\x1b[30m\x1b[46m  PID USER      PRI  NI  VIRT   RES   SHR S CPU% MEM%   TIME+  Command\x1b[K\x1b(B\x1b[m\r\n";
        for row in 0..18 {
            let pid = 1000 + row * 7 + frame;
            if row == frame {
                s += "\x1b[30m\x1b[42m";
            }
            s += &format!(
                "{pid:>5} root       20   0 1234M  567M  89M S {}.{} 1.2  0:0{}.{:02} /usr/bin/process-{row}\x1b[K\x1b(B\x1b[m\r\n",
                row % 9,
                row % 7,
                frame,
                row
            );
        }
        s += "\x1b[24;1HF1\x1b[30m\x1b[46mHelp  \x1b(B\x1b[mF2\x1b[30m\x1b[46mSetup \x1b(B\x1b[mF10\x1b[30m\x1b[46mQuit\x1b[K\x1b(B\x1b[m";
    }
    s.into_bytes()
}

/// Claude-Code-like inline TUI (no alt screen): earlier shell output with OSC
/// 133 / OSC 7, then a boxed prompt, a spinner redrawn with cursor-up, truecolor
/// and emoji, bracketed paste on, cursor hidden.
fn claude_like() -> Vec<u8> {
    let mut s = String::new();
    s += &lines(25);
    s += "\x1b]7;file://mac/Users/me/Developer/proj\x07\x1b]133;A\x07$ \x1b]133;B\x07claude\r\n\x1b]133;C;claude\x07";
    s += "\x1b]0;\u{2733} Claude Code\x07\x1b[?25l\x1b[?2004h\x1b[?1004h";
    s += "\x1b[38;2;215;119;87m\u{256d}\u{2500}\u{2500} Claude Code \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256e}\x1b[39m\r\n";
    s += "\x1b[38;2;215;119;87m\u{2502}\x1b[39m \x1b[1mWelcome back!\x1b[22m  \u{1f680}                  \x1b[38;2;215;119;87m\u{2502}\x1b[39m\r\n";
    s += "\x1b[38;2;215;119;87m\u{2570}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{256f}\x1b[39m\r\n";
    for (i, spinner) in ["\u{2736}", "\u{273b}", "\u{273d}", "\u{2733}"]
        .iter()
        .enumerate()
    {
        // Redraw the 3-line status block in place.
        s += "\x1b[3A\x1b[G";
        s += &format!(
            "\x1b[2K\x1b[38;5;174m{spinner} Thinking\u{2026} ({i}s \u{b7} \u{2191} {} tokens \u{b7} esc to interrupt)\x1b[39m\r\n",
            100 * i
        );
        s += "\x1b[2K\x1b[2m  \u{23bf}  Tip: use /help\x1b[22m\r\n\x1b[2K";
        s += "\x1b[48;2;55;55;55m > \x1b[39;49m\x1b[7m \x1b[27m";
    }
    s += "\x1b[?25h\x1b[3;4H";
    s.into_bytes()
}

#[test]
fn snapshot_recorded_corpora() {
    for (name, input, cols, rows) in [
        ("vim", vim_like(), 80, 24),
        ("htop", htop_like(), 80, 24),
        ("claude", claude_like(), 80, 30),
    ] {
        let snapshot = roundtrip_with(&input, cols, rows, &SnapshotOptions::default(), name);
        // Feeding the stream in odd chunks (PTY read boundaries) must not matter.
        let mut chunked = emu(cols, rows);
        feed_chunked(&mut chunked, &input, 7);
        let mut whole = emu(cols, rows);
        whole.feed(&input);
        assert_same(&mut whole, &mut chunked, &format!("{name}: chunked feed"));
        assert_eq!(
            escape(&snapshot),
            escape(&chunked.snapshot(&SnapshotOptions::default())),
            "{name}: chunked snapshot"
        );
    }
}

// ── Seeded fuzz over a menu of sequences ────────────────────────────────────

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }

    fn pick<'a>(&mut self, items: &'a [&'a str]) -> &'a str {
        items[self.below(items.len() as u32) as usize]
    }
}

const WORDS: &[&str] = &[
    "alpha", "beta", "gamma", "delta ", "x", "yy", "zzz ", "foo/bar", "42",
];
const WIDE: &[&str] = &["日本語", "😀", "e\u{301}", "ＡＢ", "🎉x", "界"];
const MODES: &[&str] = &[
    "\x1b[?1h",
    "\x1b[?1l",
    "\x1b=",
    "\x1b>",
    "\x1b[?1000h",
    "\x1b[?1000l",
    "\x1b[?1002h",
    "\x1b[?1002l",
    "\x1b[?1003h",
    "\x1b[?1003l",
    "\x1b[?1005h",
    "\x1b[?1006h",
    "\x1b[?1006l",
    "\x1b[?1004h",
    "\x1b[?1004l",
    "\x1b[?2004h",
    "\x1b[?2004l",
    "\x1b[?7l",
    "\x1b[?7h",
    "\x1b[?1007l",
    "\x1b[?1007h",
    "\x1b[?1042l",
    "\x1b[?25l",
    "\x1b[?25h",
    "\x1b[4h",
    "\x1b[4l",
    "\x1b[20h",
    "\x1b[20l",
    "\x1b[?6h",
    "\x1b[?6l",
    "\x1b[?12h",
    "\x1b[?12l",
];
const OSC: &[&str] = &[
    "\x1b]0;a title\x07",
    "\x1b]2;other\x1b\\",
    "\x1b]2;\x07",
    "\x1b]7;file://h/tmp/a%20b\x07",
    "\x1b]7;file:///usr/local\x1b\\",
    "\x1b]133;A\x07",
    "\x1b]133;B\x07",
    "\x1b]133;C;cargo build\x07",
    "\x1b]133;C\x07",
    "\x1b]133;D;1\x07",
    "\x1b]133;D\x07",
    "\x1b]8;id=k;https://a.example/x\x1b\\",
    "\x1b]8;;https://b.example/auto\x1b\\",
    "\x1b]8;;\x1b\\",
];
const SIMPLE: &[&str] = &[
    "\r\n", "\n", "\r", "\x08", "\t", "\x1bM", "\x1bD", "\x1bE", "\x1b7", "\x1b8", "\x1b[s",
    "\x1b[u", "\x1bH", "\x1b[g", "\x1b[3g", "\x1b(0", "\x1b(B", "\x1b)0", "\x1b[0 q", "\x1b[3 q",
    "\x1b[6 q", "\x1b[1 q",
];

fn push_sgr(rng: &mut Lcg, out: &mut String) {
    const POOL: &[&str] = &[
        "0", "1", "2", "3", "4", "4:3", "4:2", "4:4", "4:5", "7", "8", "9", "22", "23", "24", "27",
        "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "39", "40", "41", "42", "43",
        "44", "45", "46", "47", "49", "90", "91", "97", "100", "104", "107", "59",
    ];
    out.push_str("\x1b[");
    for i in 0..=rng.below(4) {
        if i > 0 {
            out.push(';');
        }
        match rng.below(10) {
            0 => out.push_str(&format!("38;5;{}", rng.below(256))),
            1 => out.push_str(&format!("48;5;{}", rng.below(256))),
            2 => out.push_str(&format!(
                "38;2;{};{};{}",
                rng.below(256),
                rng.below(256),
                rng.below(256)
            )),
            3 => out.push_str(&format!(
                "48;2;{};{};{}",
                rng.below(256),
                rng.below(256),
                rng.below(256)
            )),
            4 => out.push_str(&format!(
                "58;2;{};{};{}",
                rng.below(256),
                rng.below(256),
                rng.below(256)
            )),
            _ => out.push_str(rng.pick(POOL)),
        }
    }
    out.push('m');
}

/// `wide` swaps the menu: wide chars appear, but erase / insert / delete
/// sequences do not (they split wide chars, leaving halves the engine itself
/// never writes; see `dump`), nor do scroll regions (a wide char wrapping on a
/// bottom row outside the region leaves an unwrapped leading spacer) or insert
/// mode (shifts spacers away from their wide chars).
fn fragment(rng: &mut Lcg, cols: u32, rows: u32, wide: bool, out: &mut String) {
    let mut pick = rng.below(30);
    if wide && (12..=15).contains(&pick) {
        pick = 0;
    }
    match pick {
        0..=3 => {
            for _ in 0..=rng.below(4) {
                out.push_str(rng.pick(WORDS));
            }
        }
        4 => {
            let n = (cols + rng.below(cols)) as usize;
            out.push_str(&"abcdefghij0123456789".repeat(n / 20 + 1)[..n]);
        }
        5 if wide => out.push_str(rng.pick(WIDE)),
        6 | 7 => out.push_str(rng.pick(SIMPLE)),
        8 | 9 => push_sgr(rng, out),
        10 => out.push_str(&format!(
            "\x1b[{};{}H",
            rng.below(rows + 2),
            rng.below(cols + 3)
        )),
        11 => out.push_str(&format!(
            "\x1b[{}{}",
            rng.below(6),
            rng.pick(&["A", "B", "C", "D", "G", "d"])
        )),
        12 => out.push_str(&format!("\x1b[{}J", rng.below(4))),
        13 => out.push_str(&format!("\x1b[{}K", rng.below(3))),
        14 => out.push_str(&format!(
            "\x1b[{}{}",
            rng.below(5),
            rng.pick(&["X", "P", "@", "L", "M", "S", "T"])
        )),
        15 => {
            let top = 1 + rng.below(rows);
            let bottom = top + rng.below(rows);
            if rng.below(4) == 0 {
                out.push_str("\x1b[r");
            } else {
                out.push_str(&format!("\x1b[{top};{bottom}r"));
            }
        }
        16..=18 => out.push_str(rng.pick(MODES)),
        19 | 20 => out.push_str(if rng.below(2) == 0 {
            "\x1b[?1049h"
        } else {
            "\x1b[?1049l"
        }),
        21 | 22 => out.push_str(rng.pick(OSC)),
        23 => out.push_str("\r\n\r\n\r\n"),
        24 => out.push_str(&"long wrapping line 0123456789 ".repeat(1 + rng.below(4) as usize)),
        25 => {
            if rng.below(12) == 0 {
                out.push_str("\x1bc");
            } else {
                out.push_str(&format!("{}\r\n", rng.pick(WORDS)));
            }
        }
        _ => {
            push_sgr(rng, out);
            out.push_str(rng.pick(WORDS));
        }
    }
}

/// One step of a fuzz case: bytes to feed, or a resize.
#[derive(Clone)]
enum Step {
    Feed(String),
    Resize(usize, usize),
}

fn fuzz_case(seed: u64, wide: bool) -> (usize, usize, Vec<Step>) {
    let mut rng = Lcg(seed.wrapping_mul(0x2545_f491_4f6c_dd1d) | 1);
    let (cols, rows) = (30 + rng.below(20), 8 + rng.below(8));
    let mut steps = Vec::new();
    for _ in 0..(40 + rng.below(80)) {
        let mut piece = String::new();
        fragment(&mut rng, cols, rows, wide, &mut piece);
        if wide {
            piece = piece.replace("\x1b[4h", "");
        }
        steps.push(Step::Feed(piece));
        // Resizing wide-char text truncates / reflows through wide chars.
        if !wide && rng.below(40) == 0 {
            steps.push(Step::Resize(
                20 + rng.below(30) as usize,
                6 + rng.below(10) as usize,
            ));
        }
    }
    (cols as usize, rows as usize, steps)
}

fn run_fuzz_case(cols: usize, rows: usize, steps: &[Step], what: &str) {
    let mut a = emu(cols, rows);
    for step in steps {
        match step {
            Step::Feed(bytes) => {
                a.feed(bytes.as_bytes());
            }
            Step::Resize(c, r) => a.resize(TermDimensions::new(*c, *r)),
        }
    }
    // Continuation probes are skipped after a resize: alacritty's row-reset
    // optimization (`Row::occ`) leaves stale BCE-colored cells in a source
    // emulator whose width grew, and its later erases "forget" them. The visible
    // state is still compared exactly before any probe.
    let resized = steps.iter().any(|step| matches!(step, Step::Resize(..)));
    // The byte-identity check is skipped too: erases that cut through wide chars
    // leave orphaned halves that the engine itself would never write again, so
    // the restored emulator (normalized, see `dump`) snapshots slightly
    // differently.
    roundtrip_probed(&mut a, &SnapshotOptions::default(), !resized, false, what);
}

fn describe(steps: &[Step]) -> String {
    steps
        .iter()
        .map(|step| match step {
            Step::Feed(bytes) => escape(bytes.as_bytes()),
            Step::Resize(c, r) => format!("<resize {c}x{r}>"),
        })
        .collect()
}

/// Greedy delta-debugging: drop steps while the case still fails.
fn shrink(cols: usize, rows: usize, mut steps: Vec<Step>) -> Vec<Step> {
    let fails = |steps: &[Step]| {
        std::panic::catch_unwind(|| run_fuzz_case(cols, rows, steps, "shrink")).is_err()
    };
    let mut chunk = steps.len() / 2;
    while chunk > 0 {
        let mut i = 0;
        while i < steps.len() {
            let mut candidate = steps.clone();
            candidate.drain(i..(i + chunk).min(steps.len()));
            if fails(&candidate) {
                steps = candidate;
            } else {
                i += chunk;
            }
        }
        chunk /= 2;
    }
    steps
}

fn fuzz(wide: bool, seeds: u64) {
    let mut failures = Vec::new();
    for seed in 0..seeds {
        let (cols, rows, steps) = fuzz_case(seed, wide);
        let failed =
            std::panic::catch_unwind(|| run_fuzz_case(cols, rows, &steps, "fuzz")).is_err();
        if failed {
            let small = shrink(cols, rows, steps);
            let message = std::panic::catch_unwind(|| run_fuzz_case(cols, rows, &small, "shrunk"))
                .err()
                .map(|payload| {
                    payload
                        .downcast_ref::<String>()
                        .cloned()
                        .unwrap_or_default()
                        .chars()
                        .take(260)
                        .collect::<String>()
                })
                .unwrap_or_default();
            failures.push(format!(
                "seed {seed} ({cols}x{rows}): {}\n      -> {}",
                describe(&small),
                message.replace('\n', " ")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failing seeds, shrunk:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn snapshot_fuzz_round_trip() {
    fuzz(false, 2500);
}

#[test]
fn snapshot_fuzz_round_trip_wide_chars() {
    fuzz(true, 1500);
}

#[test]
fn snapshot_wiggle_sizes_differ_and_end_at_the_real_size() {
    let dims = TermDimensions::new(100, 30);
    let (temporary, last) = resize_wiggle_sizes(dims);
    assert_ne!(temporary, dims);
    assert_eq!(last, dims);
    let (temporary, _) = resize_wiggle_sizes(TermDimensions::new(10, 1));
    assert_eq!((temporary.columns, temporary.screen_lines), (11, 1));
}

/// Not run by default: `cargo test -p labonair-terminal-core snapshot_bench --
/// --ignored --nocapture`.
#[test]
#[ignore = "micro-benchmark, prints timings"]
fn snapshot_bench_10k_lines_at_200_columns() {
    let mut a = emu_with(
        200,
        50,
        EmulatorConfig {
            scrollback: 20_000,
            ..EmulatorConfig::default()
        },
    );
    let mut input = String::new();
    for i in 0..10_000 {
        input += &format!(
            "\x1b[32m{i:05}\x1b[0m \x1b[1;34mINFO\x1b[0m request handled path=/api/v1/items/{i} status=200 \
             \x1b[38;2;200;100;50mlatency\x1b[0m={}ms {}\r\n",
            i % 97,
            "x".repeat(60 + i % 70)
        );
    }
    a.feed(input.as_bytes());
    assert!(a.history_len() >= 9_900, "history {}", a.history_len());

    let opts = SnapshotOptions {
        max_scrollback_lines: 20_000,
    };
    let mut best = std::time::Duration::MAX;
    let mut size = 0;
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let snapshot = a.snapshot(&opts);
        best = best.min(start.elapsed());
        size = snapshot.len();
    }
    let mut b = emu_with(
        200,
        50,
        EmulatorConfig {
            scrollback: 20_000,
            ..EmulatorConfig::default()
        },
    );
    let snapshot = a.snapshot(&opts);
    let start = std::time::Instant::now();
    b.feed(&snapshot);
    let replay = start.elapsed();
    assert_same(&mut a, &mut b, "bench");
    println!(
        "snapshot: 10k lines x 200 cols -> {size} bytes, best of 5 = {best:?}; replay into fresh emulator = {replay:?}"
    );
}
