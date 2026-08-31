//! GPUI terminal renderer (T03-002).
//!
//! [`TerminalView`] is the GPUI entity that turns a [`TerminalSession`]'s
//! [`RenderableScreen`] snapshots into a visible, interactive terminal surface:
//!
//! * cells are drawn as batched style runs ([`labonair_terminal::batch_runs`]),
//!   one styled-text element per run rather than per cell;
//! * the block/beam/underline cursor and the selection highlight are painted as
//!   absolutely-positioned overlays using the theme's terminal palette;
//! * mouse-wheel scrolling walks the scrollback and any keystroke snaps back to
//!   the prompt;
//! * on every frame the view derives `(columns, rows)` from the available pixel
//!   area and the monospace cell metrics and forwards a resize to the engine +
//!   PTY when it changed;
//! * a lightweight poll task drains terminal events and only calls
//!   `cx.notify()` when the grid actually changed, so idle terminals don't
//!   re-render.
//!
//! Full keyboard/mouse mapping (modifiers, drag-select, bracketed paste) is
//! T03-003 — this view wires the minimal input path.

use std::time::Duration;

use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla, InteractiveElement,
    IntoElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent, ParentElement, Render,
    ScrollDelta, ScrollWheelEvent, SharedString, Styled, Task, Window,
};
use labonair_terminal::{
    batch_runs, grid_size, CursorShape, RenderableScreen, Rgb, Scroll, SessionOptions, StyledRun,
    TermDimensions, TerminalColors, TerminalEvent, TerminalSession,
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
            _poll: poll,
        }
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
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle);
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
                    if step != 0 {
                        this.scroll_lines(step);
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                if let Some(bytes) = keystroke_to_bytes(&ev.keystroke) {
                    this.send_input(&bytes);
                    this.snap_to_bottom();
                    cx.notify();
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

/// Minimal keystroke → PTY byte mapping. The full mapping (all modifiers, key
/// sequences, bracketed paste) is T03-003.
fn keystroke_to_bytes(ks: &Keystroke) -> Option<Vec<u8>> {
    let m = &ks.modifiers;
    let key = ks.key.as_str();

    if m.control {
        let mut chars = key.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            let lower = c.to_ascii_lowercase();
            if lower.is_ascii_alphabetic() {
                return Some(vec![lower as u8 - b'a' + 1]);
            }
            match lower {
                '[' => return Some(vec![0x1b]),
                '\\' => return Some(vec![0x1c]),
                ']' => return Some(vec![0x1d]),
                ' ' => return Some(vec![0x00]),
                _ => {}
            }
        }
    }

    let mut bytes = match key {
        "enter" => vec![b'\r'],
        "tab" => vec![b'\t'],
        "backspace" => vec![0x7f],
        "escape" => vec![0x1b],
        "space" => vec![b' '],
        "left" => vec![0x1b, b'[', b'D'],
        "right" => vec![0x1b, b'[', b'C'],
        "up" => vec![0x1b, b'[', b'A'],
        "down" => vec![0x1b, b'[', b'B'],
        "home" => vec![0x1b, b'[', b'H'],
        "end" => vec![0x1b, b'[', b'F'],
        "delete" => vec![0x1b, b'[', b'3', b'~'],
        _ => {
            if let Some(text) = &ks.key_char {
                text.clone().into_bytes()
            } else if !m.control && !m.platform && key.chars().count() == 1 {
                key.as_bytes().to_vec()
            } else {
                return None;
            }
        }
    };

    if m.alt && !bytes.is_empty() && bytes[0] != 0x1b {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Modifiers;

    fn ks(key: &str, mods: Modifiers) -> Keystroke {
        Keystroke {
            modifiers: mods,
            key: key.into(),
            key_char: None,
        }
    }

    #[test]
    fn control_letter_maps_to_control_byte() {
        let m = Modifiers {
            control: true,
            ..Default::default()
        };
        assert_eq!(keystroke_to_bytes(&ks("c", m)), Some(vec![0x03]));
    }

    #[test]
    fn named_keys_map_to_expected_sequences() {
        let m = Modifiers::default();
        assert_eq!(keystroke_to_bytes(&ks("enter", m)), Some(vec![b'\r']));
        assert_eq!(keystroke_to_bytes(&ks("backspace", m)), Some(vec![0x7f]));
        assert_eq!(
            keystroke_to_bytes(&ks("up", m)),
            Some(vec![0x1b, b'[', b'A'])
        );
    }

    #[test]
    fn printable_char_passes_through_and_alt_prefixes_escape() {
        let m = Modifiers::default();
        let mut k = ks("a", m);
        k.key_char = Some("a".into());
        assert_eq!(keystroke_to_bytes(&k), Some(vec![b'a']));

        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };
        let mut k = ks("b", alt);
        k.key_char = Some("b".into());
        assert_eq!(keystroke_to_bytes(&k), Some(vec![0x1b, b'b']));
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
