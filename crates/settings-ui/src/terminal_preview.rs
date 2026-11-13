//! Live terminal preview shown above the Terminal settings.
//!
//! Renders a fixed sample screen with the same inputs the real terminal view
//! uses — `ThemeStore` font / size / line height / ANSI palette and the layered
//! `TerminalSettings` (cursor shape, blink, opacity) — so every edit on the page
//! is visible 1:1 without opening a terminal. The cell grid, bold font and
//! cursor geometry mirror `labonair-workspace`'s `TerminalView::render`; keep
//! the two in step.

use std::time::Duration;

use gpui::{
    div, px, Context, Entity, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, Task, Window,
};
use labonair_settings::content::terminal::CursorStyle;
use labonair_settings::{Settings as _, TerminalSettings};
use labonair_theme::{AnsiColors, ThemeStore};

/// Which ANSI slot (or the plain foreground) a sample run is painted with.
#[derive(Clone, Copy)]
enum Ink {
    Fg,
    Dim,
    Blue,
    Magenta,
    Green,
    Yellow,
    Red,
}

#[derive(Clone, Copy)]
struct Run {
    text: &'static str,
    ink: Ink,
    bold: bool,
    italic: bool,
}

const fn run(text: &'static str, ink: Ink) -> Run {
    Run {
        text,
        ink,
        bold: false,
        italic: false,
    }
}

const fn bold(text: &'static str, ink: Ink) -> Run {
    Run {
        text,
        ink,
        bold: true,
        italic: false,
    }
}

const fn italic(text: &'static str, ink: Ink) -> Run {
    Run {
        text,
        ink,
        bold: false,
        italic: true,
    }
}

/// The sample screen. The last line is the live prompt; the cursor sits at its
/// end.
const LINES: &[&[Run]] = &[
    &[
        run("~/labonair", Ink::Blue),
        run(" main", Ink::Magenta),
        run(" $ cargo build && cargo test", Ink::Fg),
    ],
    &[
        bold("   Compiling", Ink::Green),
        run(" labonair v1.0.0", Ink::Fg),
    ],
    &[
        bold("warning:", Ink::Yellow),
        run(" `run() -> Bool` is deprecated", Ink::Fg),
    ],
    &[
        bold("error[E0425]:", Ink::Red),
        run(" 0O 1lI != =>  >=", Ink::Fg),
    ],
    &[
        bold("test result: ok.", Ink::Green),
        run(" 42 passed ", Ink::Fg),
        italic("(1.24s)", Ink::Dim),
    ],
    &[
        run("~/labonair", Ink::Blue),
        run(" main", Ink::Magenta),
        run(" $ ", Ink::Fg),
    ],
];

pub(crate) struct TerminalPreview {
    theme: Entity<ThemeStore>,
    blink_on: bool,
    _blink: Task<()>,
}

impl TerminalPreview {
    pub(crate) fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        // Same cadence rule as the real terminal: the interval is re-read every
        // tick so a `terminalCursorBlinkInterval` edit applies on the next flip.
        let blink = cx.spawn(async move |this, cx| loop {
            let ms = this
                .read_with(cx, |_, cx| {
                    TerminalSettings::try_get(cx)
                        .map(|s| s.cursor_blink_interval_ms())
                        .unwrap_or(1_000)
                })
                .unwrap_or(1_000);
            cx.background_executor()
                .timer(Duration::from_millis(ms))
                .await;
            if this
                .update(cx, |this, cx| {
                    this.blink_on = !this.blink_on;
                    cx.notify();
                })
                .is_err()
            {
                break;
            }
        });
        Self {
            theme,
            blink_on: true,
            _blink: blink,
        }
    }
}

fn opaque(mut color: Hsla) -> Hsla {
    color.a = 1.0;
    color
}

fn ink_color(ink: Ink, fg: Hsla, normal: &AnsiColors, dim_fg: Hsla) -> Hsla {
    opaque(match ink {
        Ink::Fg => fg,
        Ink::Dim => dim_fg,
        Ink::Blue => normal.blue,
        Ink::Magenta => normal.magenta,
        Ink::Green => normal.green,
        Ink::Yellow => normal.yellow,
        Ink::Red => normal.red,
    })
}

impl Render for TerminalPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let store = self.theme.read(cx);
        let palette = store.theme().terminal.clone();
        let base_font = store.terminal_font();
        let font_px = store.terminal_font_size();
        let line_height = store.terminal_line_height();
        let surface_alpha = store.terminal_background().a;
        let border = store.theme().core.border;
        let radius = store.theme().radius.lg;

        let settings = TerminalSettings::try_get(cx);
        let opacity_pct = settings.map_or(100, |s| s.terminal_opacity());
        let cursor_style = settings.map_or(CursorStyle::Bar, |s| s.cursor_style());
        let cursor_blink = settings.is_none_or(|s| s.cursor_blink());

        let font_id = cx.text_system().resolve_font(&base_font);
        let cell_w = cx
            .text_system()
            .ch_advance(font_id, px(font_px))
            .map(f32::from)
            .unwrap_or(font_px * 0.6)
            .max(1.0);
        let cell_h = (font_px * line_height).ceil().max(1.0);

        let opacity = ((opacity_pct as f32 / 100.0) * surface_alpha).clamp(0.05, 1.0);
        let mut bg = palette.background;
        bg.a = opacity;
        let fg = opaque(palette.foreground);
        let mut bold_font = base_font.clone();
        bold_font.weight = FontWeight::BOLD;

        let mut runs = Vec::new();
        let mut cursor_at = (0usize, 0usize);
        for (line, parts) in LINES.iter().enumerate() {
            let mut col = 0usize;
            for part in *parts {
                let width = part.text.chars().count();
                let mut el = div()
                    .absolute()
                    .left(px(col as f32 * cell_w))
                    .top(px(line as f32 * cell_h))
                    .w(px(width as f32 * cell_w))
                    .h(px(cell_h))
                    .text_color(ink_color(
                        part.ink,
                        palette.foreground,
                        &palette.normal,
                        palette.dim_foreground,
                    ))
                    .text_size(px(font_px))
                    .line_height(px(cell_h))
                    .whitespace_nowrap()
                    .font(if part.bold {
                        bold_font.clone()
                    } else {
                        base_font.clone()
                    });
                if part.italic {
                    el = el.italic();
                }
                runs.push(el.child(SharedString::from(part.text)));
                col += width;
            }
            cursor_at = (col, line);
        }

        let cursor_visible = !cursor_blink || self.blink_on;
        let cursor = cursor_visible.then(|| {
            let color = opaque(palette.cursor);
            let x = px(cursor_at.0 as f32 * cell_w);
            let y = px(cursor_at.1 as f32 * cell_h);
            let base = div().absolute();
            match cursor_style {
                CursorStyle::Bar => base.left(x).top(y).w(px(1.0)).h(px(cell_h)).bg(color),
                CursorStyle::Underline => base
                    .left(x)
                    .top(px(cursor_at.1 as f32 * cell_h + cell_h - 1.0))
                    .w(px(cell_w))
                    .h(px(1.0))
                    .bg(color),
                CursorStyle::Block => {
                    let mut fill = color;
                    fill.a = 0.55;
                    base.left(x).top(y).w(px(cell_w)).h(px(cell_h)).bg(fill)
                }
            }
        });

        div()
            .id("terminal-settings-preview")
            .relative()
            .w_full()
            .h(px(LINES.len() as f32 * cell_h + 24.0))
            .overflow_hidden()
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .child(
                // Inset like a real pane so the grid doesn't touch the border.
                div()
                    .absolute()
                    .top(px(12.0))
                    .left(px(12.0))
                    .right(px(12.0))
                    .bottom(px(12.0))
                    .children(runs)
                    .children(cursor),
            )
    }
}
