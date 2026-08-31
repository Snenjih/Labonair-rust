//! Keyboard & mouse → terminal byte-sequence mapping (T03-003).
//!
//! This module is render-free and framework-free: it takes plain, GPUI-agnostic
//! input descriptions ([`KeyInput`], [`MouseInput`], [`WheelInput`]) plus a
//! [`ModeState`] snapshot from the engine and returns the raw bytes to write to
//! the PTY. The GPUI → plain conversion lives in `labonair-ui`.
//!
//! The escape sequences follow xterm / DEC conventions (the same ones
//! `alacritty_terminal` and Zed emit):
//!
//! * cursor & edit keys respect DECCKM (application cursor mode);
//! * modifiers use the standard `CSI 1 ; <mod> <final>` / `CSI <n> ; <mod> ~`
//!   parameterisation (`mod = 1 + shift + 2*alt + 4*ctrl`);
//! * `Ctrl`+letter folds to control bytes `0x01..=0x1a`;
//! * `Alt`/`Option` prefixes an `ESC` (macOS "Option as Meta");
//! * paste is wrapped in bracketed-paste markers when the mode is set and the
//!   payload is sanitised so it can't smuggle its own terminator;
//! * mouse click / drag / wheel encode to SGR (`CSI < b ; x ; y M|m`) or the
//!   legacy `CSI M` form depending on the active mouse mode.
//!
//! Kitty keyboard protocol: we deliberately do **not** advertise or emit Kitty
//! sequences — the engine reports the flags (so `ModeState::kitty_keyboard` is
//! observable) but our DA responses never claim support, so shells fall back to
//! the legacy sequences produced here. See the task notes / warnings.

use crate::engine::ModeState;

/// A named (non-text-producing) key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Enter,
    Tab,
    Backspace,
    Escape,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    /// `F1`..=`F20`.
    Function(u8),
}

/// The key of a [`KeyInput`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A key that produces text (the resolved character after keyboard layout).
    Char(char),
    /// A named control / navigation key.
    Named(NamedKey),
}

/// Held modifier keys. `logo` is Cmd on macOS — it is never sent to the PTY
/// (it drives app shortcuts), it only suppresses text output here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub logo: bool,
}

impl Modifiers {
    /// The xterm modifier parameter: `1 + shift + 2*alt + 4*ctrl`. Returns
    /// `None` when no modifier that participates in CSI encoding is held.
    fn csi_param(&self) -> Option<u8> {
        let mut n = 0u8;
        if self.shift {
            n += 1;
        }
        if self.alt {
            n += 2;
        }
        if self.ctrl {
            n += 4;
        }
        (n != 0).then_some(n + 1)
    }
}

/// A single key press, already resolved through the keyboard layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    pub mods: Modifiers,
    /// The text the platform says this key produces, if any (used for dead keys,
    /// AltGr, IME single chars). Preferred over `Key::Char` when present.
    pub text: Option<String>,
}

impl KeyInput {
    /// Convenience constructor for a plain character key.
    pub fn char(c: char) -> Self {
        Self {
            key: Key::Char(c),
            mods: Modifiers::default(),
            text: Some(c.to_string()),
        }
    }

    /// Convenience constructor for a named key.
    pub fn named(key: NamedKey) -> Self {
        Self {
            key: Key::Named(key),
            mods: Modifiers::default(),
            text: None,
        }
    }

    /// With the given modifiers.
    pub fn with_mods(mut self, mods: Modifiers) -> Self {
        self.mods = mods;
        self
    }
}

/// Translate a key press to the bytes the PTY should receive. Returns `None`
/// when the key produces no terminal input (e.g. a bare modifier, or a Cmd
/// shortcut that belongs to the app).
pub fn key_to_bytes(input: &KeyInput, mode: &ModeState) -> Option<Vec<u8>> {
    let m = input.mods;

    // Cmd/logo combos never reach the shell (copy/paste/new-tab/… are app-level).
    if m.logo {
        return None;
    }

    match &input.key {
        Key::Named(named) => named_key_bytes(*named, m, mode),
        Key::Char(c) => char_key_bytes(*c, input.text.as_deref(), m),
    }
}

/// Bytes for a text-producing key.
fn char_key_bytes(c: char, text: Option<&str>, m: Modifiers) -> Option<Vec<u8>> {
    // Ctrl+<key> → C0 control byte.
    if m.ctrl {
        if let Some(b) = control_byte(c, m.shift) {
            return Some(alt_prefix(vec![b], m));
        }
        // Unhandled Ctrl combo (e.g. Ctrl+1): fall through to text if any.
    }

    let base: Vec<u8> = match text {
        Some(t) if !t.is_empty() && !m.ctrl => t.as_bytes().to_vec(),
        _ => {
            // No layout text (e.g. Ctrl held). Only emit the raw char if it is
            // printable ASCII/Unicode and no Ctrl is involved.
            if m.ctrl {
                return None;
            }
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).as_bytes().to_vec()
        }
    };
    if base.is_empty() {
        return None;
    }
    Some(alt_prefix(base, m))
}

/// Map a character to its C0 control byte (`Ctrl+A` → 0x01, …).
fn control_byte(c: char, shift: bool) -> Option<u8> {
    let lower = c.to_ascii_lowercase();
    match lower {
        'a'..='z' => Some(lower as u8 - b'a' + 1),
        // Ctrl+Space / Ctrl+@ → NUL.
        ' ' | '2' | '@' => Some(0x00),
        '[' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' | '/' => Some(0x1f),
        '3' => Some(0x1b),
        '8' => Some(0x7f),
        '?' => Some(0x7f),
        _ if shift => None,
        _ => None,
    }
}

/// Prefix an `ESC` when Alt/Option is held (macOS "Option as Meta").
fn alt_prefix(mut bytes: Vec<u8>, m: Modifiers) -> Vec<u8> {
    if m.alt && bytes.first() != Some(&0x1b) {
        bytes.insert(0, 0x1b);
    }
    bytes
}

/// Bytes for a named control / navigation key.
fn named_key_bytes(key: NamedKey, m: Modifiers, mode: &ModeState) -> Option<Vec<u8>> {
    use NamedKey::*;

    let out = match key {
        Enter => {
            // Shift+Enter → ESC CR so CLIs (Claude Code, …) can tell it apart
            // from a plain submit (mirrors the reference key handler).
            if m.shift && !m.ctrl && !m.alt {
                return Some(vec![0x1b, b'\r']);
            }
            return Some(alt_prefix(vec![b'\r'], m));
        }
        Tab => {
            if m.shift {
                return Some(vec![0x1b, b'[', b'Z']); // CBT / back-tab
            }
            return Some(alt_prefix(vec![b'\t'], m));
        }
        Backspace => {
            let b = if m.ctrl { 0x08 } else { 0x7f };
            return Some(alt_prefix(vec![b], m));
        }
        Escape => return Some(alt_prefix(vec![0x1b], m)),
        Space => {
            if m.ctrl {
                return Some(alt_prefix(vec![0x00], m));
            }
            return Some(alt_prefix(vec![b' '], m));
        }

        Up => cursor_key(b'A', m, mode),
        Down => cursor_key(b'B', m, mode),
        Right => cursor_key(b'C', m, mode),
        Left => cursor_key(b'D', m, mode),
        Home => cursor_key(b'H', m, mode),
        End => cursor_key(b'F', m, mode),

        Insert => tilde_key(2, m),
        Delete => tilde_key(3, m),
        PageUp => tilde_key(5, m),
        PageDown => tilde_key(6, m),

        Function(n) => return function_key(n, m),
    };
    Some(out)
}

/// Cursor / Home / End keys, honouring DECCKM and modifier parameters.
fn cursor_key(final_byte: u8, m: Modifiers, mode: &ModeState) -> Vec<u8> {
    if let Some(param) = m.csi_param() {
        // Modified cursor keys are always `CSI 1 ; <mod> <final>`.
        return format!("\x1b[1;{param}{}", final_byte as char).into_bytes();
    }
    if mode.app_cursor {
        vec![0x1b, b'O', final_byte]
    } else {
        vec![0x1b, b'[', final_byte]
    }
}

/// `CSI <n> ~` style keys (Insert/Delete/PageUp/PageDown, F5+).
fn tilde_key(n: u8, m: Modifiers) -> Vec<u8> {
    match m.csi_param() {
        Some(param) => format!("\x1b[{n};{param}~").into_bytes(),
        None => format!("\x1b[{n}~").into_bytes(),
    }
}

/// Function keys F1–F20. F1–F4 use the SS3 form (`ESC O P..S`) unmodified,
/// everything else uses the `CSI <n> ~` form.
fn function_key(n: u8, m: Modifiers) -> Option<Vec<u8>> {
    let param = m.csi_param();
    let bytes = match n {
        1..=4 if param.is_none() => {
            let final_byte = b'P' + (n - 1);
            vec![0x1b, b'O', final_byte]
        }
        1..=4 => {
            let final_byte = (b'P' + (n - 1)) as char;
            format!("\x1b[1;{}{final_byte}", param.unwrap()).into_bytes()
        }
        _ => {
            // xterm code table for F5..F20.
            let code = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                13 => 25,
                14 => 26,
                15 => 28,
                16 => 29,
                17 => 31,
                18 => 32,
                19 => 33,
                20 => 34,
                _ => return None,
            };
            match param {
                Some(p) => format!("\x1b[{code};{p}~").into_bytes(),
                None => format!("\x1b[{code}~").into_bytes(),
            }
        }
    };
    Some(bytes)
}

/// Wrap pasted text for the PTY. When `bracketed` is set (mode 2004) the payload
/// is sanitised — any embedded `ESC[201~` is neutralised — and wrapped in the
/// start/end markers. Bare `\r\n` is normalised to `\r` either way (terminals
/// expect CR for line breaks on input).
pub fn paste_payload(text: &str, bracketed: bool) -> Vec<u8> {
    let normalised = text.replace("\r\n", "\r").replace('\n', "\r");
    if !bracketed {
        return normalised.into_bytes();
    }
    // Strip any terminator the payload tries to smuggle in.
    let safe = normalised.replace("\x1b[201~", "");
    let mut out = Vec::with_capacity(safe.len() + 12);
    out.extend_from_slice(b"\x1b[200~");
    out.extend_from_slice(safe.as_bytes());
    out.extend_from_slice(b"\x1b[201~");
    out
}

/// A mouse button for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// What happened with the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Press,
    Release,
    /// Motion while a button is held (mode 1002).
    Drag,
    /// Motion with no button (mode 1003).
    Motion,
}

/// A mouse event in terminal cell coordinates (0-based, viewport-relative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseInput {
    pub button: MouseButton,
    pub kind: MouseEventKind,
    pub col: usize,
    pub row: usize,
    pub mods: Modifiers,
}

/// Encode a mouse event for the program, or `None` when the active mouse mode
/// does not want this event (e.g. plain motion without mode 1003).
pub fn mouse_report(input: &MouseInput, mode: &ModeState) -> Option<Vec<u8>> {
    if !mode.mouse_reporting() {
        return None;
    }
    match input.kind {
        MouseEventKind::Press | MouseEventKind::Release => {}
        MouseEventKind::Drag => {
            if !(mode.mouse_drag || mode.mouse_motion) {
                return None;
            }
        }
        MouseEventKind::Motion => {
            if !mode.mouse_motion {
                return None;
            }
        }
    }

    let mut cb = match input.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };
    if matches!(input.kind, MouseEventKind::Drag | MouseEventKind::Motion) {
        cb += 32;
    }
    if input.mods.shift {
        cb += 4;
    }
    if input.mods.alt {
        cb += 8;
    }
    if input.mods.ctrl {
        cb += 16;
    }

    let (col, row) = (input.col + 1, input.row + 1);

    if mode.sgr_mouse {
        let final_byte = if matches!(input.kind, MouseEventKind::Release) {
            'm'
        } else {
            'M'
        };
        return Some(format!("\x1b[<{cb};{col};{row}{final_byte}").into_bytes());
    }

    // Legacy `CSI M Cb Cx Cy` with +32 bias. Release reports button 3.
    let btn = if matches!(input.kind, MouseEventKind::Release) {
        3 + (cb & 0b1110_0000)
    } else {
        cb
    };
    let enc = |v: usize| -> u8 {
        let v = (v + 32).min(255);
        v as u8
    };
    Some(vec![
        0x1b,
        b'[',
        b'M',
        enc(btn as usize),
        enc(col),
        enc(row),
    ])
}

/// A wheel scroll. When a mouse mode is active this becomes wheel button
/// reports; on the alternate screen with `alternate_scroll` it becomes arrow
/// keys; otherwise the caller should drive scrollback (returns `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelInput {
    /// Positive = scroll up (toward history), negative = scroll down.
    pub lines: i32,
    pub col: usize,
    pub row: usize,
    pub mods: Modifiers,
}

/// The intent derived from a wheel event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelAction {
    /// Write these bytes to the PTY.
    Bytes(Vec<u8>),
    /// Scroll the scrollback view by this many lines (positive = up).
    Scrollback(i32),
}

/// Resolve a wheel event against the current mode.
pub fn wheel_action(input: &WheelInput, mode: &ModeState) -> WheelAction {
    let steps = input.lines.unsigned_abs() as usize;
    if steps == 0 {
        return WheelAction::Scrollback(0);
    }

    if mode.mouse_reporting() {
        let button = if input.lines > 0 {
            MouseButton::WheelUp
        } else {
            MouseButton::WheelDown
        };
        let mut bytes = Vec::new();
        for _ in 0..steps {
            if let Some(seq) = mouse_report(
                &MouseInput {
                    button,
                    kind: MouseEventKind::Press,
                    col: input.col,
                    row: input.row,
                    mods: input.mods,
                },
                mode,
            ) {
                bytes.extend_from_slice(&seq);
            }
        }
        return WheelAction::Bytes(bytes);
    }

    if mode.alt_screen && mode.alternate_scroll {
        // Emit arrow keys (application cursor form if set).
        let final_byte = if input.lines > 0 { b'A' } else { b'B' };
        let seq: Vec<u8> = if mode.app_cursor {
            vec![0x1b, b'O', final_byte]
        } else {
            vec![0x1b, b'[', final_byte]
        };
        let mut bytes = Vec::with_capacity(seq.len() * steps);
        for _ in 0..steps {
            bytes.extend_from_slice(&seq);
        }
        return WheelAction::Bytes(bytes);
    }

    WheelAction::Scrollback(input.lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode() -> ModeState {
        ModeState::default()
    }

    fn app_cursor() -> ModeState {
        ModeState {
            app_cursor: true,
            ..ModeState::default()
        }
    }

    fn mods(shift: bool, alt: bool, ctrl: bool) -> Modifiers {
        Modifiers {
            shift,
            alt,
            ctrl,
            logo: false,
        }
    }

    fn bytes(input: KeyInput, m: &ModeState) -> Vec<u8> {
        key_to_bytes(&input, m).unwrap()
    }

    #[test]
    fn plain_char_passes_through() {
        assert_eq!(bytes(KeyInput::char('a'), &mode()), b"a");
        assert_eq!(bytes(KeyInput::char('€'), &mode()), "€".as_bytes());
    }

    #[test]
    fn ctrl_letters_fold_to_control_bytes() {
        for (c, b) in [('a', 1u8), ('c', 3), ('z', 26)] {
            let k = KeyInput {
                key: Key::Char(c),
                mods: mods(false, false, true),
                text: None,
            };
            assert_eq!(key_to_bytes(&k, &mode()), Some(vec![b]));
        }
    }

    #[test]
    fn ctrl_symbols_fold() {
        let k = |c| KeyInput {
            key: Key::Char(c),
            mods: mods(false, false, true),
            text: None,
        };
        assert_eq!(key_to_bytes(&k('['), &mode()), Some(vec![0x1b]));
        assert_eq!(key_to_bytes(&k(' '), &mode()), Some(vec![0x00]));
        assert_eq!(key_to_bytes(&k('\\'), &mode()), Some(vec![0x1c]));
        assert_eq!(key_to_bytes(&k(']'), &mode()), Some(vec![0x1d]));
    }

    #[test]
    fn alt_char_gets_escape_prefix() {
        let k = KeyInput {
            key: Key::Char('b'),
            mods: mods(false, true, false),
            text: Some("b".into()),
        };
        assert_eq!(key_to_bytes(&k, &mode()), Some(vec![0x1b, b'b']));
    }

    #[test]
    fn logo_combo_is_swallowed() {
        let k = KeyInput {
            key: Key::Char('c'),
            mods: Modifiers {
                logo: true,
                ..Default::default()
            },
            text: Some("c".into()),
        };
        assert_eq!(key_to_bytes(&k, &mode()), None);
    }

    #[test]
    fn cursor_keys_respect_app_mode() {
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Up), &mode()),
            vec![0x1b, b'[', b'A']
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Up), &app_cursor()),
            vec![0x1b, b'O', b'A']
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::End), &app_cursor()),
            vec![0x1b, b'O', b'F']
        );
    }

    #[test]
    fn modified_cursor_keys_use_csi_param() {
        // Ctrl+Right → CSI 1 ; 5 C
        let k = KeyInput::named(NamedKey::Right).with_mods(mods(false, false, true));
        assert_eq!(key_to_bytes(&k, &app_cursor()), Some(b"\x1b[1;5C".to_vec()));
        // Shift+Alt+Left → mod = 1 + 1 + 2 = 4
        let k = KeyInput::named(NamedKey::Left).with_mods(mods(true, true, false));
        assert_eq!(key_to_bytes(&k, &mode()), Some(b"\x1b[1;4D".to_vec()));
    }

    #[test]
    fn navigation_tilde_keys() {
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Delete), &mode()),
            b"\x1b[3~"
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::PageUp), &mode()),
            b"\x1b[5~"
        );
        let k = KeyInput::named(NamedKey::Delete).with_mods(mods(false, false, true));
        assert_eq!(key_to_bytes(&k, &mode()), Some(b"\x1b[3;5~".to_vec()));
    }

    #[test]
    fn function_keys_both_forms() {
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Function(1)), &mode()),
            b"\x1bOP"
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Function(4)), &mode()),
            b"\x1bOS"
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Function(5)), &mode()),
            b"\x1b[15~"
        );
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Function(12)), &mode()),
            b"\x1b[24~"
        );
        let k = KeyInput::named(NamedKey::Function(1)).with_mods(mods(true, false, false));
        assert_eq!(key_to_bytes(&k, &mode()), Some(b"\x1b[1;2P".to_vec()));
    }

    #[test]
    fn enter_tab_backspace_specials() {
        assert_eq!(bytes(KeyInput::named(NamedKey::Enter), &mode()), b"\r");
        let shift_enter = KeyInput::named(NamedKey::Enter).with_mods(mods(true, false, false));
        assert_eq!(key_to_bytes(&shift_enter, &mode()), Some(vec![0x1b, b'\r']));
        assert_eq!(
            bytes(KeyInput::named(NamedKey::Backspace), &mode()),
            vec![0x7f]
        );
        let ctrl_bs = KeyInput::named(NamedKey::Backspace).with_mods(mods(false, false, true));
        assert_eq!(key_to_bytes(&ctrl_bs, &mode()), Some(vec![0x08]));
        let shift_tab = KeyInput::named(NamedKey::Tab).with_mods(mods(true, false, false));
        assert_eq!(
            key_to_bytes(&shift_tab, &mode()),
            Some(vec![0x1b, b'[', b'Z'])
        );
    }

    #[test]
    fn bracketed_paste_wraps_and_sanitises() {
        assert_eq!(paste_payload("hi\nthere", false), b"hi\rthere");
        let out = paste_payload("a\x1b[201~b", true);
        assert_eq!(out, b"\x1b[200~ab\x1b[201~");
    }

    #[test]
    fn sgr_mouse_click_encoding() {
        let m = ModeState {
            mouse_report_click: true,
            sgr_mouse: true,
            ..ModeState::default()
        };
        let press = MouseInput {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 3,
            row: 4,
            mods: Modifiers::default(),
        };
        assert_eq!(mouse_report(&press, &m), Some(b"\x1b[<0;4;5M".to_vec()));
        let release = MouseInput {
            kind: MouseEventKind::Release,
            ..press
        };
        assert_eq!(mouse_report(&release, &m), Some(b"\x1b[<0;4;5m".to_vec()));
    }

    #[test]
    fn legacy_mouse_click_encoding() {
        let m = ModeState {
            mouse_report_click: true,
            ..ModeState::default()
        };
        let press = MouseInput {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 0,
            row: 0,
            mods: Modifiers::default(),
        };
        assert_eq!(
            mouse_report(&press, &m),
            Some(vec![0x1b, b'[', b'M', 32, 33, 33])
        );
    }

    #[test]
    fn mouse_report_none_without_mode() {
        let press = MouseInput {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 0,
            row: 0,
            mods: Modifiers::default(),
        };
        assert_eq!(mouse_report(&press, &ModeState::default()), None);
    }

    #[test]
    fn drag_needs_drag_mode() {
        let click_only = ModeState {
            mouse_report_click: true,
            sgr_mouse: true,
            ..ModeState::default()
        };
        let drag = MouseInput {
            button: MouseButton::Left,
            kind: MouseEventKind::Drag,
            col: 1,
            row: 1,
            mods: Modifiers::default(),
        };
        assert_eq!(mouse_report(&drag, &click_only), None);
        let with_drag = ModeState {
            mouse_drag: true,
            ..click_only
        };
        assert_eq!(
            mouse_report(&drag, &with_drag),
            Some(b"\x1b[<32;2;2M".to_vec())
        );
    }

    #[test]
    fn wheel_scrollback_by_default() {
        let w = WheelInput {
            lines: 3,
            col: 0,
            row: 0,
            mods: Modifiers::default(),
        };
        assert_eq!(
            wheel_action(&w, &ModeState::default()),
            WheelAction::Scrollback(3)
        );
    }

    #[test]
    fn wheel_alt_screen_becomes_arrows() {
        let m = ModeState {
            alt_screen: true,
            alternate_scroll: true,
            ..ModeState::default()
        };
        let w = WheelInput {
            lines: -2,
            col: 0,
            row: 0,
            mods: Modifiers::default(),
        };
        assert_eq!(
            wheel_action(&w, &m),
            WheelAction::Bytes(vec![0x1b, b'[', b'B', 0x1b, b'[', b'B'])
        );
    }

    #[test]
    fn wheel_reports_buttons_in_mouse_mode() {
        let m = ModeState {
            mouse_report_click: true,
            sgr_mouse: true,
            ..ModeState::default()
        };
        let w = WheelInput {
            lines: 1,
            col: 2,
            row: 2,
            mods: Modifiers::default(),
        };
        assert_eq!(
            wheel_action(&w, &m),
            WheelAction::Bytes(b"\x1b[<64;3;3M".to_vec())
        );
    }
}
