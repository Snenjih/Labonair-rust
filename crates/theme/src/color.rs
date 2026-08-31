//! Color handling for the theme system.
//!
//! Labonair authors every color in `reference-src/src/styles/globals.css` in the
//! Oklch color space. GPUI has no Oklch type, so every token is converted once,
//! here, to [`gpui::Hsla`] via sRGB. The Oklch -> sRGB math is delegated to the
//! `palette` crate rather than reimplemented.

use gpui::{Hsla, Rgba};
use palette::{FromColor, Oklch, Srgb};

/// Convert an Oklch color to `gpui::Hsla`.
///
/// * `l` — lightness as written in CSS in percent (e.g. `79.68` for `79.68%`).
/// * `c` — chroma, absolute (e.g. `0.1298`).
/// * `h` — hue in degrees (e.g. `82.18`).
///
/// Out-of-gamut results are clamped per channel into `[0, 1]`.
pub fn oklch(l: f32, c: f32, h: f32) -> Hsla {
    oklch_a(l, c, h, 1.0)
}

/// Like [`oklch`], with an explicit alpha in `[0, 1]`.
pub fn oklch_a(l: f32, c: f32, h: f32, alpha: f32) -> Hsla {
    let srgb: Srgb = Srgb::from_color(Oklch::new(l / 100.0, c, h));
    let rgba = Rgba {
        r: srgb.red.clamp(0.0, 1.0),
        g: srgb.green.clamp(0.0, 1.0),
        b: srgb.blue.clamp(0.0, 1.0),
        a: alpha.clamp(0.0, 1.0),
    };
    let mut hsla: Hsla = rgba.into();
    hsla.a = rgba.a;
    hsla
}

/// Fully transparent color (`transparent` keyword in CSS).
pub fn transparent() -> Hsla {
    Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.0,
    }
}

/// Parse a single color value from the text formats that appear in `globals.css`
/// and in user-imported themes: `oklch(...)`, `#rgb`/`#rgba`/`#rrggbb`/`#rrggbbaa`,
/// `rgb(...)`/`rgba(...)`, and the `transparent` keyword.
///
/// This is a convenience helper for [`T02-003`] theme import — the built-in
/// light/dark themes are constructed directly from [`oklch`] calls.
pub fn parse_color(input: &str) -> Result<Hsla, String> {
    let s = input.trim();
    if s.eq_ignore_ascii_case("transparent") {
        return Ok(transparent());
    }
    if let Some(rest) = s.strip_prefix('#') {
        return parse_hex(rest);
    }
    if let Some(inner) = strip_call(s, "oklch") {
        return parse_oklch(inner);
    }
    if let Some(inner) = strip_call(s, "rgba").or_else(|| strip_call(s, "rgb")) {
        return parse_rgb(inner);
    }
    Err(format!("unrecognized color value: {input:?}"))
}

fn strip_call<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let s = s.strip_prefix(name)?.trim_start();
    let s = s.strip_prefix('(')?;
    s.strip_suffix(')')
}

/// Split the inside of an `oklch(...)` / `rgb(...)` call into the space/comma
/// separated component list and the optional `/ alpha` part.
fn split_components(inner: &str) -> (Vec<&str>, Option<&str>) {
    let (main, alpha) = match inner.split_once('/') {
        Some((m, a)) => (m, Some(a.trim())),
        None => (inner, None),
    };
    let parts = main
        .split([',', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    (parts, alpha)
}

/// Parse a number that may carry a trailing `%`, returning the fraction in
/// `[0, 1]` when `percent_base` is set, or the raw value otherwise.
fn parse_number(tok: &str) -> Result<(f32, bool), String> {
    if let Some(v) = tok.strip_suffix('%') {
        v.trim()
            .parse::<f32>()
            .map(|n| (n, true))
            .map_err(|e| format!("bad number {tok:?}: {e}"))
    } else {
        tok.parse::<f32>()
            .map(|n| (n, false))
            .map_err(|e| format!("bad number {tok:?}: {e}"))
    }
}

fn parse_alpha(alpha: Option<&str>) -> Result<f32, String> {
    match alpha {
        None => Ok(1.0),
        Some(tok) => {
            let (v, is_percent) = parse_number(tok)?;
            Ok(if is_percent { v / 100.0 } else { v }.clamp(0.0, 1.0))
        }
    }
}

fn parse_oklch(inner: &str) -> Result<Hsla, String> {
    let (parts, alpha) = split_components(inner);
    if parts.len() != 3 {
        return Err(format!("oklch expects 3 components, got {}", parts.len()));
    }
    let (l_raw, l_percent) = parse_number(parts[0])?;
    // CSS allows `oklch(0.5 ...)` or `oklch(50% ...)`; oklch() below wants percent.
    let l = if l_percent { l_raw } else { l_raw * 100.0 };
    let (c, _) = parse_number(parts[1])?;
    let (h, _) = parse_number(parts[2])?;
    Ok(oklch_a(l, c, h, parse_alpha(alpha)?))
}

fn parse_rgb(inner: &str) -> Result<Hsla, String> {
    let (parts, alpha) = split_components(inner);
    if parts.len() != 3 {
        return Err(format!("rgb expects 3 components, got {}", parts.len()));
    }
    let mut ch = [0.0f32; 3];
    for (i, p) in parts.iter().enumerate() {
        let (v, is_percent) = parse_number(p)?;
        ch[i] = if is_percent { v / 100.0 } else { v / 255.0 };
    }
    let rgba = Rgba {
        r: ch[0].clamp(0.0, 1.0),
        g: ch[1].clamp(0.0, 1.0),
        b: ch[2].clamp(0.0, 1.0),
        a: parse_alpha(alpha)?,
    };
    let mut hsla: Hsla = rgba.into();
    hsla.a = rgba.a;
    Ok(hsla)
}

fn parse_hex(hex: &str) -> Result<Hsla, String> {
    let expand = |c: char| -> String { format!("{c}{c}") };
    let full = match hex.len() {
        3 => hex.chars().map(expand).collect::<String>() + "ff",
        4 => hex.chars().map(expand).collect::<String>(),
        6 => format!("{hex}ff"),
        8 => hex.to_string(),
        n => return Err(format!("hex color has {n} digits, expected 3/4/6/8")),
    };
    let byte = |i: usize| -> Result<f32, String> {
        u8::from_str_radix(&full[i..i + 2], 16)
            .map(|b| b as f32 / 255.0)
            .map_err(|e| format!("bad hex {hex:?}: {e}"))
    };
    let rgba = Rgba {
        r: byte(0)?,
        g: byte(2)?,
        b: byte(4)?,
        a: byte(6)?,
    };
    let mut hsla: Hsla = rgba.into();
    hsla.a = rgba.a;
    Ok(hsla)
}

/// Convert an `Hsla` back to 8-bit sRGB — used by tests and by theme export.
pub fn to_rgb8(color: Hsla) -> [u8; 3] {
    let rgba: Rgba = color.into();
    [
        (rgba.r * 255.0).round() as u8,
        (rgba.g * 255.0).round() as u8,
        (rgba.b * 255.0).round() as u8,
    ]
}

/// Serialize an `Hsla` to a `#rrggbb` (opaque) or `#rrggbbaa` hex string — the
/// format Labonair theme files use. Used by theme export (T02-003).
pub fn to_hex(color: Hsla) -> String {
    let [r, g, b] = to_rgb8(color);
    let rgba: Rgba = color.into();
    let a = (rgba.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb8_of(l: f32, c: f32, h: f32) -> [u8; 3] {
        to_rgb8(oklch(l, c, h))
    }

    fn close(a: [u8; 3], b: [u8; 3], tol: i32) -> bool {
        a.iter()
            .zip(b.iter())
            .all(|(x, y)| (*x as i32 - *y as i32).abs() <= tol)
    }

    #[test]
    fn oklch_extremes() {
        assert_eq!(rgb8_of(0.0, 0.0, 0.0), [0, 0, 0]);
        assert_eq!(rgb8_of(100.0, 0.0, 0.0), [255, 255, 255]);
    }

    #[test]
    fn oklch_grayscale_is_neutral() {
        let [r, g, b] = rgb8_of(20.9, 0.0, 259.98);
        assert!((r as i32 - g as i32).abs() <= 1 && (g as i32 - b as i32).abs() <= 1);
    }

    #[test]
    fn oklch_matches_design_hex_comments() {
        // Hex values are the design-intent comments from globals.css (.dark).
        assert!(
            close(rgb8_of(79.68, 0.1298, 82.18), [0xE6, 0xB4, 0x50], 3),
            "primary ~ #E6B450, got {:?}",
            rgb8_of(79.68, 0.1298, 82.18)
        );
        assert!(
            close(rgb8_of(69.82, 0.1637, 17.27), [0xF2, 0x6D, 0x78], 3),
            "destructive ~ #F26D78, got {:?}",
            rgb8_of(69.82, 0.1637, 17.27)
        );
        assert!(
            close(rgb8_of(35.62, 0.0, 259.98), [0x3C, 0x3C, 0x3C], 3),
            "accent ~ #3C3C3C, got {:?}",
            rgb8_of(35.62, 0.0, 259.98)
        );
        assert!(
            close(rgb8_of(28.91, 0.0, 259.98), [0x2B, 0x2B, 0x2B], 3),
            "border ~ #2B2B2B, got {:?}",
            rgb8_of(28.91, 0.0, 259.98)
        );
    }

    #[test]
    fn oklch_conversion_is_stable_within_tolerance() {
        // Re-running the conversion must be bit-identical (no accumulated error).
        for (l, c, h) in [
            (20.9, 0.0, 0.0),
            (79.68, 0.1298, 82.18),
            (55.0, 0.1637, 17.27),
        ] {
            assert_eq!(rgb8_of(l, c, h), rgb8_of(l, c, h));
        }
    }

    #[test]
    fn parse_hex_forms() {
        assert_eq!(to_rgb8(parse_color("#fff").unwrap()), [255, 255, 255]);
        assert_eq!(to_rgb8(parse_color("#000000").unwrap()), [0, 0, 0]);
        assert_eq!(to_rgb8(parse_color("#ff8800").unwrap()), [255, 136, 0]);
        assert_eq!(parse_color("#80808080").unwrap().a, 128.0 / 255.0);
    }

    #[test]
    fn parse_rgb_and_transparent() {
        assert_eq!(
            to_rgb8(parse_color("rgb(255, 136, 0)").unwrap()),
            [255, 136, 0]
        );
        assert_eq!(parse_color("rgba(0 0 0 / 0.5)").unwrap().a, 0.5);
        assert_eq!(parse_color("transparent").unwrap().a, 0.0);
    }

    #[test]
    fn parse_oklch_forms() {
        let a = parse_color("oklch(79.68% 0.1298 82.18)").unwrap();
        let b = oklch(79.68, 0.1298, 82.18);
        assert_eq!(to_rgb8(a), to_rgb8(b));
        // fractional lightness form
        let c = parse_color("oklch(0.7968 0.1298 82.18)").unwrap();
        assert!(
            (to_rgb8(c)[0] as i32 - to_rgb8(b)[0] as i32).abs() <= 1,
            "fractional and percent lightness agree"
        );
        // alpha as percent
        assert_eq!(
            parse_color("oklch(79.68% 0.1298 82.18 / 13%)").unwrap().a,
            0.13
        );
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_color("chartreuse").is_err());
        assert!(parse_color("oklch(1 2)").is_err());
        assert!(parse_color("#12345").is_err());
    }
}
