//! Bridge from the design-token [`TerminalPalette`] (T02-001) to the color model
//! the terminal engine consumes (T02-004).
//!
//! `alacritty_terminal` (integrated in Phase 02) works with 8-bit-per-channel
//! RGB and an indexed color list (`alacritty_terminal::term::color::Colors`,
//! 269 slots). This module converts the theme's Oklch-derived [`gpui::Hsla`]
//! colors into that world:
//!
//! * [`TerminalColors::from_theme`] resolves every palette color to [`Rgb`].
//! * [`TerminalColors::ansi256`] implements the standard xterm 0–255 lookup
//!   (0–7 normal, 8–15 bright, 16–231 6×6×6 cube, 232–255 grayscale ramp).
//! * [`TerminalColors::to_alacritty_colors`] fills the engine's indexed list,
//!   including the Labonair-specific dim group and bright/dim foreground slots.
//!
//! The terminal color model has **no alpha channel** — the selection overlay
//! opacity from `--selection` is carried separately as
//! [`TerminalColors::selection_alpha`] and must be applied by the renderer.

use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{NamedColor, Rgb};
use gpui::Hsla;
use labonair_theme::{to_rgb8, AnsiColors, TerminalPalette, Theme};

/// Convert a theme color to an opaque 8-bit RGB triple (alpha is dropped).
fn to_rgb(c: Hsla) -> Rgb {
    let [r, g, b] = to_rgb8(c);
    Rgb { r, g, b }
}

/// The 8 colors of one ANSI row, in index order (black, red, green, yellow,
/// blue, magenta, cyan, white).
fn row(a: &AnsiColors) -> [Rgb; 8] {
    [
        a.black, a.red, a.green, a.yellow, a.blue, a.magenta, a.cyan, a.white,
    ]
    .map(to_rgb)
}

/// One 6×6×6 color-cube axis value (`0, 95, 135, 175, 215, 255`).
fn cube_axis(n: u8) -> u8 {
    if n == 0 {
        0
    } else {
        55 + n * 40
    }
}

/// Every theme-derived terminal color, resolved to 8-bit RGB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalColors {
    pub background: Rgb,
    pub foreground: Rgb,
    /// `--terminal-bright-foreground`.
    pub bright_foreground: Rgb,
    /// `--terminal-dim-foreground`.
    pub dim_foreground: Rgb,
    /// `--cursor`.
    pub cursor: Rgb,
    /// Text drawn beneath a block cursor (xterm `cursorAccent`) — the terminal
    /// background, so the cell content stays legible.
    pub cursor_text: Rgb,
    /// `--selection` with the alpha stripped (see [`Self::selection_alpha`]).
    pub selection: Rgb,
    /// Opacity of the selection overlay, from the `--selection` alpha.
    pub selection_alpha: f32,
    /// ANSI 0–7.
    pub normal: [Rgb; 8],
    /// ANSI 8–15.
    pub bright: [Rgb; 8],
    /// Labonair's dim group — used when the SGR "dim" (2) attribute is set.
    pub dim: [Rgb; 8],
}

impl TerminalColors {
    /// Resolve the active theme's terminal palette.
    pub fn from_theme(theme: &Theme) -> Self {
        Self::from_palette(&theme.terminal)
    }

    /// Resolve a [`TerminalPalette`] directly.
    pub fn from_palette(p: &TerminalPalette) -> Self {
        Self {
            background: to_rgb(p.background),
            foreground: to_rgb(p.foreground),
            bright_foreground: to_rgb(p.bright_foreground),
            dim_foreground: to_rgb(p.dim_foreground),
            cursor: to_rgb(p.cursor),
            cursor_text: to_rgb(p.background),
            selection: to_rgb(p.selection),
            selection_alpha: p.selection.a,
            normal: row(&p.normal),
            bright: row(&p.bright),
            dim: row(&p.dim),
        }
    }

    /// Standard xterm 256-color lookup.
    ///
    /// * `0..=7`   → [`Self::normal`]
    /// * `8..=15`  → [`Self::bright`]
    /// * `16..=231`→ 6×6×6 color cube (derived, not theme-defined)
    /// * `232..=255` → 24-step grayscale ramp (derived)
    pub fn ansi256(&self, index: u8) -> Rgb {
        match index {
            0..=7 => self.normal[index as usize],
            8..=15 => self.bright[(index - 8) as usize],
            16..=231 => {
                let i = index - 16;
                Rgb {
                    r: cube_axis(i / 36),
                    g: cube_axis((i % 36) / 6),
                    b: cube_axis(i % 6),
                }
            }
            232..=255 => {
                let v = 8 + 10 * (index - 232);
                Rgb { r: v, g: v, b: v }
            }
        }
    }

    /// Populate an `alacritty_terminal` indexed color list from this palette.
    ///
    /// Fills the 256 ANSI slots, the foreground/background/cursor specials, the
    /// bright/dim foreground slots and the dim color group. The renderer still
    /// owns the background opacity and the [`Self::selection_alpha`] overlay.
    pub fn to_alacritty_colors(&self) -> Colors {
        let mut colors = Colors::default();
        for i in 0u16..=255 {
            colors[i as usize] = Some(self.ansi256(i as u8));
        }
        colors[NamedColor::Foreground] = Some(self.foreground);
        colors[NamedColor::Background] = Some(self.background);
        colors[NamedColor::Cursor] = Some(self.cursor);
        colors[NamedColor::BrightForeground] = Some(self.bright_foreground);
        colors[NamedColor::DimForeground] = Some(self.dim_foreground);
        for (i, named) in [
            NamedColor::DimBlack,
            NamedColor::DimRed,
            NamedColor::DimGreen,
            NamedColor::DimYellow,
            NamedColor::DimBlue,
            NamedColor::DimMagenta,
            NamedColor::DimCyan,
            NamedColor::DimWhite,
        ]
        .into_iter()
        .enumerate()
        {
            colors[named] = Some(self.dim[i]);
        }
        colors
    }
}

/// An ANSI escape-sequence dump that exercises the whole palette — write it to a
/// PTY (or `print!` it) to eyeball the colors against the reference app.
pub fn ansi_self_test() -> String {
    let mut s = String::from("Labonair terminal palette self-test\n\nSystem colors (0-15):\n");
    for i in 0..16 {
        s.push_str(&format!("\x1b[48;5;{i}m  \x1b[0m"));
        if i == 7 {
            s.push('\n');
        }
    }
    s.push_str("\n\n216-color cube (16-231):\n");
    for i in 16..232 {
        s.push_str(&format!("\x1b[48;5;{i}m  \x1b[0m"));
        if (i - 16) % 36 == 35 {
            s.push('\n');
        }
    }
    s.push_str("\nGrayscale ramp (232-255):\n");
    for i in 232..256 {
        s.push_str(&format!("\x1b[48;5;{i}m  \x1b[0m"));
    }
    s.push_str("\n\nAttributes: \x1b[1mbold\x1b[0m \x1b[2mdim\x1b[0m \x1b[3mitalic\x1b[0m ");
    s.push_str("\x1b[4munderline\x1b[0m \x1b[7mreverse\x1b[0m\n");
    s.push_str("Foreground: \x1b[31mred \x1b[32mgreen \x1b[33myellow \x1b[34mblue ");
    s.push_str("\x1b[35mmagenta \x1b[36mcyan\x1b[0m\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Rgb, b: [u8; 3], tol: i32) -> bool {
        (a.r as i32 - b[0] as i32).abs() <= tol
            && (a.g as i32 - b[1] as i32).abs() <= tol
            && (a.b as i32 - b[2] as i32).abs() <= tol
    }

    #[test]
    fn palette_has_all_three_rows_plus_specials() {
        let c = TerminalColors::from_theme(&Theme::dark());
        assert_eq!(c.normal.len(), 8);
        assert_eq!(c.bright.len(), 8);
        assert_eq!(c.dim.len(), 8);
        assert_ne!(c.background, c.foreground);
        assert_ne!(c.normal[1], c.bright[1]);
        assert_ne!(c.normal[1], c.dim[1]);
    }

    #[test]
    fn ansi256_named_range_maps_to_rows() {
        let c = TerminalColors::from_theme(&Theme::dark());
        assert_eq!(c.ansi256(0), c.normal[0]);
        assert_eq!(c.ansi256(7), c.normal[7]);
        assert_eq!(c.ansi256(8), c.bright[0]);
        assert_eq!(c.ansi256(15), c.bright[7]);
    }

    #[test]
    fn ansi256_cube_and_grayscale_follow_xterm_scheme() {
        let c = TerminalColors::from_theme(&Theme::light());
        assert_eq!(c.ansi256(16), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            c.ansi256(231),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(c.ansi256(196), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(c.ansi256(46), Rgb { r: 0, g: 255, b: 0 });
        assert_eq!(c.ansi256(21), Rgb { r: 0, g: 0, b: 255 });
        assert_eq!(c.ansi256(232), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(
            c.ansi256(255),
            Rgb {
                r: 238,
                g: 238,
                b: 238
            }
        );
    }

    #[test]
    fn conversion_is_exact_for_every_palette_color() {
        // `< 1/255` deviation: `to_rgb` is the same rounding path as `to_rgb8`,
        // so the resolved bytes must match the theme's own conversion exactly.
        let t = Theme::dark();
        let c = TerminalColors::from_palette(&t.terminal);
        let src = [
            t.terminal.normal.black,
            t.terminal.normal.red,
            t.terminal.normal.green,
            t.terminal.normal.yellow,
            t.terminal.normal.blue,
            t.terminal.normal.magenta,
            t.terminal.normal.cyan,
            t.terminal.normal.white,
        ];
        for (i, hsla) in src.into_iter().enumerate() {
            let [r, g, b] = to_rgb8(hsla);
            assert_eq!(c.normal[i], Rgb { r, g, b });
        }
    }

    #[test]
    fn dark_values_match_globals_css() {
        let c = TerminalColors::from_theme(&Theme::dark());
        // .dark --terminal-yellow == --primary #E6B450
        assert!(close(c.normal[3], [0xE6, 0xB4, 0x50], 3));
        // .dark --terminal-red == --destructive #F26D78
        assert!(close(c.normal[1], [0xF2, 0x6D, 0x78], 3));
        // --terminal-bright-white / --terminal-bright-foreground == white
        assert!(close(c.bright[7], [0xFF, 0xFF, 0xFF], 2));
        assert!(close(c.bright_foreground, [0xFF, 0xFF, 0xFF], 2));
    }

    #[test]
    fn light_and_dark_palettes_differ() {
        let l = TerminalColors::from_theme(&Theme::light());
        let d = TerminalColors::from_theme(&Theme::dark());
        assert_ne!(l.background, d.background);
        assert_ne!(l.normal[1], d.normal[1]);
    }

    #[test]
    fn selection_alpha_is_carried_separately() {
        let c = TerminalColors::from_theme(&Theme::dark());
        assert!((c.selection_alpha - 0.13).abs() < 1e-6);
    }

    #[test]
    fn alacritty_color_list_is_filled() {
        let c = TerminalColors::from_theme(&Theme::dark());
        let list = c.to_alacritty_colors();
        for i in 0..=255usize {
            assert!(list[i].is_some(), "ansi slot {i} unset");
        }
        assert_eq!(list[NamedColor::Foreground], Some(c.foreground));
        assert_eq!(list[NamedColor::Background], Some(c.background));
        assert_eq!(list[NamedColor::Cursor], Some(c.cursor));
        assert_eq!(
            list[NamedColor::BrightForeground],
            Some(c.bright_foreground)
        );
        assert_eq!(list[NamedColor::DimForeground], Some(c.dim_foreground));
        assert_eq!(list[NamedColor::DimRed], Some(c.dim[1]));
        assert_eq!(list[NamedColor::DimWhite], Some(c.dim[7]));
    }

    #[test]
    fn self_test_covers_all_256_indices() {
        let s = ansi_self_test();
        for i in 0..256 {
            assert!(s.contains(&format!("48;5;{i}m")), "missing index {i}");
        }
        assert!(s.contains("\x1b[2mdim\x1b[0m"));
    }
}
