//! The color model the terminal engine consumes (T02-004).
//!
//! `alacritty_terminal` works with 8-bit-per-channel RGB and an indexed color
//! list (`alacritty_terminal::term::color::Colors`, 269 slots). This module is
//! GPUI-free; the theme -> [`TerminalColors`] bridge (`from_theme`) lives in
//! `labonair-terminal`'s `TerminalColorsExt`.
//!
//! * [`TerminalColors::xterm_default`] is a theme-independent palette (daemon,
//!   tests).
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
    /// A theme-independent palette (classic xterm colors, dim = normal).
    pub fn xterm_default() -> Self {
        let rgb = |r, g, b| Rgb { r, g, b };
        let normal = [
            rgb(0, 0, 0),
            rgb(205, 0, 0),
            rgb(0, 205, 0),
            rgb(205, 205, 0),
            rgb(0, 0, 238),
            rgb(205, 0, 205),
            rgb(0, 205, 205),
            rgb(229, 229, 229),
        ];
        let bright = [
            rgb(127, 127, 127),
            rgb(255, 0, 0),
            rgb(0, 255, 0),
            rgb(255, 255, 0),
            rgb(92, 92, 255),
            rgb(255, 0, 255),
            rgb(0, 255, 255),
            rgb(255, 255, 255),
        ];
        Self {
            background: rgb(0, 0, 0),
            foreground: rgb(229, 229, 229),
            bright_foreground: rgb(255, 255, 255),
            dim_foreground: rgb(127, 127, 127),
            cursor: rgb(229, 229, 229),
            cursor_text: rgb(0, 0, 0),
            selection: rgb(68, 68, 68),
            selection_alpha: 0.25,
            normal,
            bright,
            dim: normal,
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

    #[test]
    fn ansi256_named_range_maps_to_rows() {
        let c = TerminalColors::xterm_default();
        assert_eq!(c.ansi256(0), c.normal[0]);
        assert_eq!(c.ansi256(7), c.normal[7]);
        assert_eq!(c.ansi256(8), c.bright[0]);
        assert_eq!(c.ansi256(15), c.bright[7]);
    }

    #[test]
    fn ansi256_cube_and_grayscale_follow_xterm_scheme() {
        let c = TerminalColors::xterm_default();
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
    fn alacritty_color_list_is_filled() {
        let c = TerminalColors::xterm_default();
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
