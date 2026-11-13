//! Bridge from the design-token [`TerminalPalette`] (T02-001) to the engine's
//! [`TerminalColors`] (T02-004).
//!
//! The color model itself is GPUI-free and lives in `labonair-terminal-core`;
//! this module converts the theme's Oklch-derived [`gpui::Hsla`] colors into it.
//! It is an extension trait (the struct is defined in another crate), so callers
//! bring [`TerminalColorsExt`] into scope to use `TerminalColors::from_theme`.

use alacritty_terminal::vte::ansi::Rgb;
use gpui::Hsla;
use labonair_theme::{to_rgb8, AnsiColors, TerminalPalette, Theme};

pub use labonair_terminal_core::palette::{ansi_self_test, TerminalColors};

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

/// Theme -> [`TerminalColors`] constructors.
pub trait TerminalColorsExt: Sized {
    /// Resolve the active theme's terminal palette.
    fn from_theme(theme: &Theme) -> Self;
    /// Resolve a [`TerminalPalette`] directly.
    fn from_palette(p: &TerminalPalette) -> Self;
}

impl TerminalColorsExt for TerminalColors {
    fn from_theme(theme: &Theme) -> Self {
        Self::from_palette(&theme.terminal)
    }

    fn from_palette(p: &TerminalPalette) -> Self {
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
    fn dark_values_match_the_default_theme_tokens() {
        let c = TerminalColors::from_theme(&Theme::dark());
        // dark --terminal-ansi-yellow #e5c07b (the softened floating-layer palette)
        assert!(close(c.normal[3], [0xE5, 0xC0, 0x7B], 2));
        // dark --terminal-ansi-red #ff6b7f
        assert!(close(c.normal[1], [0xFF, 0x6B, 0x7F], 2));
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
        assert!((c.selection_alpha - 0.25).abs() < 0.01);
    }
}
