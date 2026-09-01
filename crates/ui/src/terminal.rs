//! GPUI terminal renderer (T03-002).
//!
//! [`TerminalView`] is the GPUI entity that turns a registry session's
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

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, App, ClipboardItem, Context, Entity, FocusHandle, Focusable, FontWeight, Hsla,
    InteractiveElement, IntoElement, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollDelta,
    ScrollWheelEvent, SharedString, Size, Styled, Task, Window,
};
use labonair_terminal::{
    batch_runs, grid_size, key_to_bytes, mouse_report, paste_payload, wheel_action, CursorShape,
    Key, KeyInput, ModeState, Modifiers, MouseButton as TermMouseButton, MouseEventKind,
    MouseInput, NamedKey, RenderableScreen, Rgb, Scroll, SessionHandle, SessionStatus, StyledRun,
    TermDimensions, TerminalColors, TerminalEvent, WheelAction, WheelInput,
};

use crate::background::{BackgroundStore, LayerScope};
use crate::explorer::{quote_paths, DraggedPaths};
use crate::theme::ThemeStore;

/// How often the view polls the session for new terminal output.
const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// A running terminal, rendered with GPUI.
///
/// The terminal session itself lives in the shared [`TerminalRegistry`]
/// (T03-005) and is reached through a cheap [`SessionHandle`] — the view only
/// renders it and forwards input. Switching which tab is visible never touches
/// the session, so background terminals keep running (T04-001).
///
/// [`TerminalRegistry`]: labonair_terminal::TerminalRegistry
pub struct TerminalView {
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    focus_handle: FocusHandle,
    /// Handle to this tab's session in the registry.
    handle: SessionHandle,
    /// Grid size last pushed to the engine, to avoid redundant resizes.
    grid: (usize, usize),
    /// Cell (width, height) in pixels from the last render — used to map mouse
    /// positions to grid cells.
    cell_size: (f32, f32),
    /// Anchor cell of an in-progress drag selection, if the user is selecting.
    drag_anchor: Option<(usize, usize)>,
    /// Pixel size of this view's content area, captured each paint. Preferred
    /// over the whole window's viewport so a terminal hosted in a split pane
    /// sizes its grid to the pane, not the window (T04-002).
    measured: Option<Size<Pixels>>,
    _poll: Task<()>,
}

impl TerminalView {
    /// Render the registry session behind `handle`.
    pub fn new(
        handle: SessionHandle,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        // Re-color the running shell when the theme changes.
        cx.observe(&theme, |this, theme, cx| {
            let colors = TerminalColors::from_theme(theme.read(cx).theme());
            let _ = this.handle.set_colors(colors);
            cx.notify();
        })
        .detach();

        // Repaint when the background image / settings change.
        cx.observe(&background, |_, _, cx| cx.notify()).detach();

        let poll = cx.spawn(async move |view, cx| loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            let keep_going = view.update(cx, |this, cx| {
                let events = this.handle.drain_events();
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
            background,
            focus_handle,
            handle,
            grid: (80, 24),
            cell_size: (8.0, 16.0),
            drag_anchor: None,
            measured: None,
            _poll: poll,
        }
    }

    /// The session this view renders.
    pub fn handle(&self) -> &SessionHandle {
        &self.handle
    }

    /// Focus the terminal surface (called by the workspace on tab switch).
    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    /// The shell's current working directory (OSC 7 shell integration), if
    /// known. Surfaced to the status bar / breadcrumb and used as the starting
    /// directory for new tabs (Phase 03).
    pub fn cwd(&self) -> Option<String> {
        self.handle.with(|s| s.cwd())
    }

    /// The process/window title set via OSC 0/2, if any.
    pub fn shell_title(&self) -> Option<String> {
        self.handle
            .with(|s| s.metadata().ok().and_then(|m| m.title))
    }

    /// Find `query` in the currently visible screen and select the first
    /// match (top-to-bottom, left-to-right). Returns `true` if a match was
    /// found. An empty query just clears any active selection.
    ///
    /// This is the target of the header's inline search (T04-003). A full
    /// scrollback-spanning find widget with next/previous navigation is a
    /// later search-module concern.
    pub fn search(&self, query: &str, cx: &mut Context<Self>) -> bool {
        if query.is_empty() {
            let _ = self.handle.with(|s| s.clear_selection());
            cx.notify();
            return false;
        }
        let Ok(screen) = self.handle.with(|s| s.render()) else {
            return false;
        };
        let text = screen.to_text();
        for (row, line) in text.lines().enumerate() {
            if let Some(byte_idx) = line.find(query) {
                let col = line[..byte_idx].chars().count();
                let len = query.chars().count();
                let _ = self
                    .handle
                    .with(|s| s.update_selection((col, row), (col + len, row)));
                cx.notify();
                return true;
            }
        }
        false
    }

    /// The current terminal mode snapshot (falls back to defaults on error).
    fn mode(&self) -> ModeState {
        self.handle
            .with(|s| s.mode_state().ok())
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
        if let Ok(Some(text)) = self.handle.with(|s| s.selection_text()) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
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
        let _ = self.handle.with(|s| s.scroll(Scroll::Delta(delta)));
    }

    fn snap_to_bottom(&self) {
        let _ = self.handle.with(|s| s.scroll(Scroll::Bottom));
    }

    fn send_input(&self, bytes: &[u8]) {
        let _ = self.handle.write(bytes);
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

        let exited = match self.handle.status() {
            SessionStatus::Running => None,
            SessionStatus::Exited(code) => Some(code),
        };

        self.cell_size = (cell_w, cell_h);

        // Fit the grid to this view's content area (falling back to the whole
        // window before the first paint has measured it) and inform the
        // engine/PTY.
        let viewport = self.measured.unwrap_or_else(|| window.viewport_size());
        let (cols, rows) = grid_size(
            f32::from(viewport.width),
            f32::from(viewport.height),
            cell_w,
            cell_h,
        );
        if (cols, rows) != self.grid {
            self.grid = (cols, rows);
            let _ = self.handle.resize(TermDimensions {
                columns: cols,
                screen_lines: rows,
                cell_width: cell_w.round().max(1.0) as u16,
                cell_height: cell_h as u16,
            });
        }

        let screen: RenderableScreen =
            self.handle
                .with(|s| s.render())
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

        // Background image overlay (only when the target is Terminal-only;
        // App/Both is painted window-wide by the app root instead).
        let background_layer = self.background.read(cx).layer(LayerScope::Terminal);

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

        let view = cx.weak_entity();
        let size_probe = canvas(
            move |bounds, _window, cx| {
                let _ = view.update(cx, |this, cx| {
                    if this.measured != Some(bounds.size) {
                        this.measured = Some(bounds.size);
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        div()
            .id("terminal")
            .track_focus(&self.focus_handle)
            .key_context("Terminal")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(bg)
            .text_color(fg)
            .child(size_probe)
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
                        let _ = this.handle.with(|s| s.clear_selection());
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
                let _ = this.handle.with(|s| s.update_selection(anchor, head));
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
                        let _ = this.handle.with(|s| s.clear_selection());
                        cx.notify();
                    }
                }
            }))
            .on_drop(cx.listener(|this, d: &DraggedPaths, _window, _cx| {
                // Explorer file dragged onto the terminal → insert its quoted
                // path(s), like the reference "drag file into shell" workflow.
                if d.paths.is_empty() {
                    return;
                }
                let text = format!("{} ", quote_paths(&d.paths));
                this.send_input(text.as_bytes());
            }))
            .children(run_elements)
            .children(selection_elements)
            .children(cursor_element)
            .children(background_layer)
            .when_some(exited, |el, code| {
                el.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .px_3()
                        .py_1()
                        .bg(with_alpha(fg, 0.08))
                        .text_color(fg)
                        .text_size(px(font_px))
                        .child(SharedString::from(format!(
                            "Shell exited ({code}) — press \u{2318}W to close this tab"
                        ))),
                )
            })
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
