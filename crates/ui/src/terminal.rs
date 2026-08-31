//! GPUI terminal renderer (T03-002).
//!
//! [`TerminalView`] is the GPUI entity that turns a [`TerminalSession`]'s
//! [`RenderableScreen`] snapshots into a visible, interactive terminal surface:
//!
//! * cells are drawn as batched style runs ([`labonair_terminal::batch_runs`]),
//!   one styled-text element per run rather than per cell;
//! * the block/beam/underline cursor and the selection highlight are painted as
//!   absolutely-positioned overlays using the theme's terminal palette;
//! * on every frame the view derives `(columns, rows)` from the available pixel
//!   area and the monospace cell metrics and forwards a resize to the engine +
//!   PTY when it changed;
//! * a lightweight poll task drains terminal events and only calls
//!   `cx.notify()` when the grid actually changed, so idle terminals don't
//!   re-render.
//!
//! Keyboard and mouse input (T03-003) is translated by
//! [`labonair_terminal::input`]: this view converts GPUI events to the plain
//! [`KeyInput`] / [`MouseInput`] / [`WheelInput`] descriptions, folds in the
//! live [`ModeState`] from the engine, and writes the resulting bytes to the
//! PTY. Mouse-wheel scrolling walks the scrollback unless a mouse-reporting or
//! alternate-scroll mode is active; drag selects text (copy-on-select), Cmd+C /
//! Cmd+V and right-click drive the clipboard with bracketed-paste support.

use std::time::Duration;

use gpui::{
    div, px, App, ClipboardItem, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Point, Render, ScrollDelta, ScrollWheelEvent,
    SharedString, Styled, Task, Window,
};
use labonair_terminal::{
    batch_runs, grid_size, key_to_bytes, mouse_report, paste_payload, wheel_action, CursorShape,
    Key, KeyInput, ModeState, Modifiers, MouseButton as TermMouseButton, MouseEventKind,
    MouseInput, NamedKey, RenderableScreen, Rgb, Scroll, SessionOptions, StyledRun, TermDimensions,
    TerminalColors, TerminalEvent, TerminalSession, WheelAction, WheelInput,
};

use crate::theme::ThemeStore;

/// How often the view polls the session for new terminal output.
const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// A running terminal, rendered with GPUI.
pub struct TerminalView {
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    /// `Ok` once the shell spawned; `Err` keeps the failure message for display.
    session: Result<TerminalSession, String>,
    /// Grid size last pushed to the engine, to avoid redundant resizes.
    grid: (usize, usize),
    /// Cell (width, height) in pixels from the last render — used to map mouse
    /// positions to grid cells.
    cell_size: (f32, f32),
    /// Anchor cell of an in-progress drag selection, if the user is selecting.
    drag_anchor: Option<(usize, usize)>,
    _poll: Task<()>,
}

impl TerminalView {
    /// Spawn a local shell and start rendering it.
    pub fn new(theme: Entity<ThemeStore>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let colors = TerminalColors::from_theme(theme.read(cx).theme());
        let dims = TermDimensions::new(80, 24);
        let session = TerminalSession::spawn(colors, dims, SessionOptions::default());

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        // Re-color the running shell when the theme changes.
        cx.observe(&theme, |this, theme, cx| {
            if let Ok(session) = &this.session {
                let colors = TerminalColors::from_theme(theme.read(cx).theme());
                let _ = session.set_colors(colors);
            }
            cx.notify();
        })
        .detach();

        let poll = cx.spawn(async move |view, cx| loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            let keep_going = view.update(cx, |this, cx| {
                let Ok(session) = &this.session else {
                    return false;
                };
                let events = session.drain_events();
                if events.is_empty() {
                    return true;
                }
                let exited = events
                    .iter()
                    .any(|e| matches!(e, TerminalEvent::Exit | TerminalEvent::ChildExit(_)));
                cx.notify();
                !exited
            });
            if !matches!(keep_going, Ok(true)) {
                break;
            }
        });

        Self {
            theme,
            focus_handle,
            session: session.map_err(|e| format!("failed to start shell: {e}")),
            grid: (80, 24),
            cell_size: (8.0, 16.0),
            drag_anchor: None,
            _poll: poll,
        }
    }

    /// The current terminal mode snapshot (falls back to defaults on error).
    fn mode(&self) -> ModeState {
        self.session
            .as_ref()
            .ok()
            .and_then(|s| s.mode_state().ok())
            .unwrap_or_default()
    }

    /// Map a window-relative pixel position to a viewport cell `(col, row)`,
    /// clamped to the grid.
    fn cell_at(&self, pos: Point<gpui::Pixels>) -> (usize, usize) {
        let (cw, ch) = self.cell_size;
        let (cols, rows) = self.grid;
        let col = (f32::from(pos.x).max(0.0) / cw) as usize;
        let row = (f32::from(pos.y).max(0.0) / ch) as usize;
        (
            col.min(cols.saturating_sub(1)),
            row.min(rows.saturating_sub(1)),
        )
    }

    fn copy_selection(&self, cx: &mut Context<Self>) {
        if let Ok(session) = &self.session {
            if let Ok(Some(text)) = session.selection_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }
    }

    fn paste_from_clipboard(&self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) else {
            return;
        };
        if text.is_empty() {
            return;
        }
        let bracketed = self.mode().bracketed_paste;
        self.send_input(&paste_payload(&text, bracketed));
        self.snap_to_bottom();
    }

    fn scroll_lines(&self, delta: i32) {
        if let Ok(session) = &self.session {
            let _ = session.scroll(Scroll::Delta(delta));
        }
    }

    fn snap_to_bottom(&self) {
        if let Ok(session) = &self.session {
            let _ = session.scroll(Scroll::Bottom);
        }
    }

    fn send_input(&self, bytes: &[u8]) {
        if let Ok(session) = &self.session {
            let _ = session.write(bytes);
        }
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.theme.read(cx);
        let colors = TerminalColors::from_theme(store.theme());
        let base_font = store.terminal_font();
        let font_px = store.terminal_font_size();
        let line_height = store.terminal_line_height();

        let font_id = cx.text_system().resolve_font(&base_font);
        let cell_w = cx
            .text_system()
            .ch_advance(font_id, px(font_px))
            .map(f32::from)
            .unwrap_or(font_px * 0.6)
            .max(1.0);
        let cell_h = (font_px * line_height).ceil().max(1.0);

        let bg = to_hsla(colors.background, 1.0);
        let fg = to_hsla(colors.foreground, 1.0);

        if let Err(message) = &self.session {
            return div()
                .size_full()
                .bg(bg)
                .text_color(fg)
                .font(base_font)
                .p_4()
                .child(SharedString::from(message.clone()))
                .into_any_element();
        }

        self.cell_size = (cell_w, cell_h);

        // Fit the grid to the current viewport and inform the engine/PTY.
        let viewport = window.viewport_size();
        let (cols, rows) = grid_size(
            f32::from(viewport.width),
            f32::from(viewport.height),
            cell_w,
            cell_h,
        );
        if (cols, rows) != self.grid {
            self.grid = (cols, rows);
            if let Ok(session) = &mut self.session {
                let _ = session.resize(TermDimensions {
                    columns: cols,
                    screen_lines: rows,
                    cell_width: cell_w.round().max(1.0) as u16,
                    cell_height: cell_h as u16,
                });
            }
        }

        let screen: RenderableScreen = self
            .session
            .as_ref()
            .expect("session present")
            .render()
            .unwrap_or_else(|_| RenderableScreen {
                columns: cols,
                screen_lines: rows,
                display_offset: 0,
                cursor: labonair_terminal::RenderableCursor {
                    line: 0,
                    column: 0,
                    shape: CursorShape::Block,
                },
                cells: Vec::new(),
                selection: Vec::new(),
            });

        let runs = batch_runs(&screen);
        let bold_font = {
            let mut f = base_font.clone();
            f.weight = FontWeight::BOLD;
            f
        };

        let cell = |x: usize, y: usize| (px(x as f32 * cell_w), px(y as f32 * cell_h));

        let run_elements = runs.into_iter().map(|run: StyledRun| {
            let (left, top) = cell(run.start_col, run.line);
            let text_color = if run.style.hidden {
                to_hsla(run.style.bg, 1.0)
            } else {
                to_hsla(run.style.fg, 1.0)
            };
            let mut el = div()
                .absolute()
                .left(left)
                .top(top)
                .w(px(run.width() as f32 * cell_w))
                .h(px(cell_h))
                .bg(to_hsla(run.style.bg, 1.0))
                .text_color(text_color)
                .text_size(px(font_px))
                .line_height(px(cell_h))
                .whitespace_nowrap()
                .font(if run.style.bold {
                    bold_font.clone()
                } else {
                    base_font.clone()
                });
            if run.style.italic {
                el = el.italic();
            }
            if run.style.underline {
                el = el.underline();
            }
            if run.style.strikeout {
                el = el.line_through();
            }
            el.child(SharedString::from(run.text))
        });

        let selection_color = to_hsla(colors.selection, colors.selection_alpha.clamp(0.0, 1.0));
        let selection_elements = screen.selection.clone().into_iter().map(move |span| {
            let (left, top) = cell(span.start_col, span.line);
            div()
                .absolute()
                .left(left)
                .top(top)
                .w(px((span.end_col - span.start_col) as f32 * cell_w))
                .h(px(cell_h))
                .bg(selection_color)
        });

        let cursor_element = cursor_overlay(&screen, cell_w, cell_h, to_hsla(colors.cursor, 1.0));

        div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(bg)
            .text_color(fg)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle);
                    let cell = this.cell_at(ev.position);
                    let mode = this.mode();
                    let mods = to_term_mods(&ev.modifiers);
                    if let Some(bytes) = mouse_report(
                        &MouseInput {
                            button: TermMouseButton::Left,
                            kind: MouseEventKind::Press,
                            col: cell.0,
                            row: cell.1,
                            mods,
                        },
                        &mode,
                    ) {
                        this.send_input(&bytes);
                    } else {
                        // Native selection: anchor here, clear any old selection.
                        this.drag_anchor = Some(cell);
                        if let Ok(session) = &this.session {
                            let _ = session.clear_selection();
                        }
                    }
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _window, cx| {
                let Some(anchor) = this.drag_anchor else {
                    return;
                };
                if ev.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let head = this.cell_at(ev.position);
                if let Ok(session) = &this.session {
                    let _ = session.update_selection(anchor, head);
                }
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseUpEvent, _window, cx| {
                    let cell = this.cell_at(ev.position);
                    let mode = this.mode();
                    if let Some(bytes) = mouse_report(
                        &MouseInput {
                            button: TermMouseButton::Left,
                            kind: MouseEventKind::Release,
                            col: cell.0,
                            row: cell.1,
                            mods: to_term_mods(&ev.modifiers),
                        },
                        &mode,
                    ) {
                        this.send_input(&bytes);
                    } else if this.drag_anchor.is_some() {
                        // Copy-on-select parity with the reference terminal.
                        this.copy_selection(cx);
                    }
                    this.drag_anchor = None;
                    cx.notify();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _: &MouseDownEvent, _window, cx| {
                    // Right-click pastes (reference `terminalRightClickPastes`).
                    this.paste_from_clipboard(cx);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(
                cx.listener(move |this, ev: &ScrollWheelEvent, _window, cx| {
                    let lines = match ev.delta {
                        ScrollDelta::Lines(p) => p.y,
                        ScrollDelta::Pixels(p) => f32::from(p.y) / cell_h,
                    };
                    let step = lines.round() as i32;
                    if step == 0 {
                        return;
                    }
                    let cell = this.cell_at(ev.position);
                    match wheel_action(
                        &WheelInput {
                            lines: step,
                            col: cell.0,
                            row: cell.1,
                            mods: to_term_mods(&ev.modifiers),
                        },
                        &this.mode(),
                    ) {
                        WheelAction::Bytes(bytes) => this.send_input(&bytes),
                        WheelAction::Scrollback(n) if n != 0 => this.scroll_lines(n),
                        WheelAction::Scrollback(_) => {}
                    }
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                let ks = &ev.keystroke;
                // App-level clipboard shortcuts never reach the shell.
                if ks.modifiers.platform && !ks.modifiers.control && !ks.modifiers.alt {
                    match ks.key.as_str() {
                        "c" => {
                            this.copy_selection(cx);
                            return;
                        }
                        "v" => {
                            this.paste_from_clipboard(cx);
                            cx.notify();
                            return;
                        }
                        _ => {}
                    }
                }
                if let Some(input) = keystroke_to_input(ks) {
                    if let Some(bytes) = key_to_bytes(&input, &this.mode()) {
                        this.send_input(&bytes);
                        this.snap_to_bottom();
                        if let Ok(session) = &this.session {
                            let _ = session.clear_selection();
                        }
                        cx.notify();
                    }
                }
            }))
            .children(run_elements)
            .children(selection_elements)
            .children(cursor_element)
            .into_any_element()
    }
}

/// The cursor overlay div, or `None` when the cursor is hidden or scrolled out
/// of view.
fn cursor_overlay(
    screen: &RenderableScreen,
    cell_w: f32,
    cell_h: f32,
    color: Hsla,
) -> Option<gpui::Div> {
    if screen.display_offset != 0 {
        return None;
    }
    let cur = screen.cursor;
    if cur.line < 0 || cur.line as usize >= screen.screen_lines {
        return None;
    }
    let x = px(cur.column as f32 * cell_w);
    let y = px(cur.line as f32 * cell_h);
    let base = div().absolute();
    Some(match cur.shape {
        CursorShape::Hidden => return None,
        CursorShape::Beam => base.left(x).top(y).w(px(2.0)).h(px(cell_h)).bg(color),
        CursorShape::Underline => base
            .left(x)
            .top(px(cur.line as f32 * cell_h + cell_h - 2.0))
            .w(px(cell_w))
            .h(px(2.0))
            .bg(color),
        // Block / HollowBlock: a translucent fill so the glyph shows through.
        _ => base
            .left(x)
            .top(y)
            .w(px(cell_w))
            .h(px(cell_h))
            .bg(with_alpha(color, 0.55)),
    })
}

fn to_hsla(c: Rgb, alpha: f32) -> Hsla {
    gpui::Rgba {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        a: alpha,
    }
    .into()
}

fn with_alpha(mut color: Hsla, alpha: f32) -> Hsla {
    color.a = alpha;
    color
}

/// Translate GPUI [`Modifiers`](gpui::Modifiers) to the engine's [`Modifiers`].
fn to_term_mods(m: &gpui::Modifiers) -> Modifiers {
    Modifiers {
        shift: m.shift,
        alt: m.alt,
        ctrl: m.control,
        logo: m.platform,
    }
}

/// Map a GPUI keystroke to a framework-agnostic [`KeyInput`] for
/// [`key_to_bytes`]. Returns `None` for keys the terminal never consumes (bare
/// modifiers, unknown named keys).
fn keystroke_to_input(ks: &Keystroke) -> Option<KeyInput> {
    let mods = to_term_mods(&ks.modifiers);
    let key = ks.key.as_str();

    let named = match key {
        "enter" => Some(NamedKey::Enter),
        "tab" => Some(NamedKey::Tab),
        "backspace" => Some(NamedKey::Backspace),
        "escape" => Some(NamedKey::Escape),
        "space" => Some(NamedKey::Space),
        "up" => Some(NamedKey::Up),
        "down" => Some(NamedKey::Down),
        "left" => Some(NamedKey::Left),
        "right" => Some(NamedKey::Right),
        "home" => Some(NamedKey::Home),
        "end" => Some(NamedKey::End),
        "pageup" => Some(NamedKey::PageUp),
        "pagedown" => Some(NamedKey::PageDown),
        "insert" => Some(NamedKey::Insert),
        "delete" => Some(NamedKey::Delete),
        _ => key
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().ok())
            .filter(|n| (1..=20).contains(n))
            .map(NamedKey::Function),
    };

    if let Some(named) = named {
        return Some(KeyInput {
            key: Key::Named(named),
            mods,
            text: None,
        });
    }

    // Text-producing key. Prefer the platform-resolved char; for Ctrl combos
    // GPUI often reports no `key_char`, so fall back to the key name.
    let ch = ks
        .key_char
        .as_deref()
        .filter(|t| t.chars().count() == 1)
        .and_then(|t| t.chars().next())
        .or_else(|| {
            let mut it = key.chars();
            match (it.next(), it.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        })?;

    Some(KeyInput {
        key: Key::Char(ch),
        mods,
        text: ks.key_char.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ks(key: &str, mods: gpui::Modifiers) -> Keystroke {
        Keystroke {
            modifiers: mods,
            key: key.into(),
            key_char: None,
        }
    }

    fn bytes(ks: &Keystroke) -> Option<Vec<u8>> {
        key_to_bytes(&keystroke_to_input(ks)?, &ModeState::default())
    }

    #[test]
    fn control_letter_maps_to_control_byte() {
        let m = gpui::Modifiers {
            control: true,
            ..Default::default()
        };
        assert_eq!(bytes(&ks("c", m)), Some(vec![0x03]));
    }

    #[test]
    fn named_keys_map_to_expected_sequences() {
        let m = gpui::Modifiers::default();
        assert_eq!(bytes(&ks("enter", m)), Some(vec![b'\r']));
        assert_eq!(bytes(&ks("backspace", m)), Some(vec![0x7f]));
        assert_eq!(bytes(&ks("up", m)), Some(vec![0x1b, b'[', b'A']));
        assert_eq!(bytes(&ks("f5", m)), Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn app_cursor_mode_changes_arrow_keys() {
        let mode = ModeState {
            app_cursor: true,
            ..ModeState::default()
        };
        let input = keystroke_to_input(&ks("left", gpui::Modifiers::default())).unwrap();
        assert_eq!(key_to_bytes(&input, &mode), Some(vec![0x1b, b'O', b'D']));
    }

    #[test]
    fn printable_char_passes_through_and_alt_prefixes_escape() {
        let m = gpui::Modifiers::default();
        let mut k = ks("a", m);
        k.key_char = Some("a".into());
        assert_eq!(bytes(&k), Some(vec![b'a']));

        let alt = gpui::Modifiers {
            alt: true,
            ..Default::default()
        };
        let mut k = ks("b", alt);
        k.key_char = Some("b".into());
        assert_eq!(bytes(&k), Some(vec![0x1b, b'b']));
    }

    #[test]
    fn platform_shortcuts_produce_no_pty_bytes() {
        let m = gpui::Modifiers {
            platform: true,
            ..Default::default()
        };
        let mut k = ks("c", m);
        k.key_char = Some("c".into());
        assert_eq!(bytes(&k), None);
    }

    #[test]
    fn to_hsla_maps_white_and_black() {
        let white = to_hsla(
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            1.0,
        );
        assert!(white.l > 0.99);
        let black = to_hsla(Rgb { r: 0, g: 0, b: 0 }, 1.0);
        assert!(black.l < 0.01);
    }
}
