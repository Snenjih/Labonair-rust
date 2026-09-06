//! Design tokens ported 1:1 from `reference-src/src/styles/globals.css`.
//!
//! Every value here is transcribed directly from that file — the `:root` block
//! for [`Theme::light`] and the `.dark` block for [`Theme::dark`]. Nothing is
//! invented or adjusted. Colors are stored as [`gpui::Hsla`], converted once
//! from their original Oklch notation by [`crate::color::oklch`].

use std::time::Duration;

use gpui::Hsla;

use crate::color::{oklch, oklch_a, transparent};

/// The complete set of design tokens for one theme variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// `true` for the dark variant, `false` for light.
    pub is_dark: bool,
    pub core: CoreColors,
    pub sidebar: SidebarColors,
    pub surface: SurfaceColors,
    pub border: BorderVariants,
    pub status: StatusColors,
    pub interaction: InteractionColors,
    pub terminal: TerminalPalette,
    pub radius: RadiusScale,
    pub shadows: Shadows,
    pub animation: Animation,
    pub typography: Typography,
}

/// Core shadcn-style surface/control colors and their foreground pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct CoreColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    /// `--chart-1` .. `--chart-5`.
    pub charts: [Hsla; 5],
}

/// Sidebar-specific colors (`--sidebar-*`).
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub border: Hsla,
    pub ring: Hsla,
}

/// Window-chrome surface tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceColors {
    pub toolbar: Hsla,
    pub title_bar: Hsla,
    pub status_bar: Hsla,
}

/// Border variant tokens (`--border-*`).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderVariants {
    pub variant: Hsla,
    pub focused: Hsla,
    pub selected: Hsla,
    pub transparent: Hsla,
    pub disabled: Hsla,
}

/// Semantic status colors.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusColors {
    pub modified: Hsla,
    pub error: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    pub hint: Hsla,
    pub success: Hsla,
}

/// UI interaction colors.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionColors {
    pub cursor: Hsla,
    pub selection: Hsla,
}

/// One ANSI color group of 8 (used for the normal, bright and dim rows).
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiColors {
    pub black: Hsla,
    pub red: Hsla,
    pub green: Hsla,
    pub yellow: Hsla,
    pub blue: Hsla,
    pub magenta: Hsla,
    pub cyan: Hsla,
    pub white: Hsla,
}

/// The full terminal color palette (`--terminal-*`).
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalPalette {
    pub background: Hsla,
    pub foreground: Hsla,
    pub bright_foreground: Hsla,
    pub dim_foreground: Hsla,
    pub normal: AnsiColors,
    pub bright: AnsiColors,
    pub dim: AnsiColors,
    /// `--cursor` — mirrored here so terminal rendering has a local reference.
    pub cursor: Hsla,
    /// `--selection` — same.
    pub selection: Hsla,
}

/// Corner-radius scale, in physical pixels (derived from `--radius: 0.3125rem`
/// at a 16px root font size, i.e. 5px, plus the fixed `--window-radius`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusScale {
    /// `--radius` itself (== `lg`).
    pub base: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xl2: f32,
    pub xl3: f32,
    pub xl4: f32,
    /// `--window-radius` — fixed 12px, not part of the scale.
    pub window: f32,
}

impl RadiusScale {
    /// Build the scale from the `--radius` base value in pixels, applying the
    /// `calc()` multipliers from the `@theme inline` block.
    fn from_base(base: f32) -> Self {
        Self {
            base,
            sm: base * 0.6,
            md: base * 0.8,
            lg: base,
            xl: base * 1.4,
            xl2: base * 1.8,
            xl3: base * 2.2,
            xl4: base * 2.6,
            window: 12.0,
        }
    }

    /// Every rounded-corner value multiplied by `factor` (the T20-007
    /// `corner_radius_scale` theme-setting). `window` is left untouched — it
    /// is the fixed `--window-radius`, not part of the tunable scale.
    pub fn scaled(self, factor: f32) -> Self {
        let f = factor.max(0.0);
        Self {
            base: self.base * f,
            sm: self.sm * f,
            md: self.md * f,
            lg: self.lg * f,
            xl: self.xl * f,
            xl2: self.xl2 * f,
            xl3: self.xl3 * f,
            xl4: self.xl4 * f,
            window: self.window,
        }
    }
}

/// A single CSS box-shadow layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Hsla,
}

/// Elevation shadows — each tier can have multiple stacked layers.
#[derive(Debug, Clone, PartialEq)]
pub struct Shadows {
    pub row: Vec<ShadowLayer>,
    pub popover: Vec<ShadowLayer>,
    pub modal: Vec<ShadowLayer>,
}

/// A cubic-bezier easing curve (`cubic-bezier(x1, y1, x2, y2)`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl CubicBezier {
    /// Evaluate the curve's `y` for a linear time input `t` in `0..=1`.
    ///
    /// CSS `cubic-bezier` curves are parametric (control points `(0,0)`,
    /// `(x1,y1)`, `(x2,y2)`, `(1,1)`); this first solves the Bézier parameter
    /// `s` for which `x(s) == t` (Newton-Raphson, then bisection fallback) and
    /// returns `y(s)`. Used to drive GPUI [`Animation`](gpui) easing from the
    /// same tokens the reference CSS uses (`--ease-premium` / `--ease-soft`).
    pub fn eval(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let cx = 3.0 * self.x1;
        let bx = 3.0 * (self.x2 - self.x1) - cx;
        let ax = 1.0 - cx - bx;
        let cy = 3.0 * self.y1;
        let by = 3.0 * (self.y2 - self.y1) - cy;
        let ay = 1.0 - cy - by;
        let sample_x = |s: f32| ((ax * s + bx) * s + cx) * s;
        let sample_dx = |s: f32| (3.0 * ax * s + 2.0 * bx) * s + cx;

        // Newton-Raphson.
        let mut s = t;
        for _ in 0..8 {
            let x = sample_x(s) - t;
            if x.abs() < 1e-5 {
                return ((ay * s + by) * s + cy) * s;
            }
            let dx = sample_dx(s);
            if dx.abs() < 1e-6 {
                break;
            }
            s -= x / dx;
        }
        // Bisection fallback.
        let (mut lo, mut hi, mut s) = (0.0_f32, 1.0_f32, t);
        while lo < hi {
            let x = sample_x(s);
            if (x - t).abs() < 1e-5 {
                break;
            }
            if t > x {
                lo = s;
            } else {
                hi = s;
            }
            s = (hi - lo) * 0.5 + lo;
        }
        ((ay * s + by) * s + cy) * s
    }
}

/// The `from` scale of the reference `labonair-tab-in` keyframe
/// (`transform: scale(0.86) → scale(1)` over `--dur-base` `--ease-premium`,
/// `reference-src/src/styles/globals.css`). Deferred visual item **D4** from
/// the T15-001 catalog.
pub const TAB_IN_FROM_SCALE: f32 = 0.86;

/// Animation timing tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    pub dur_fast: Duration,
    pub dur_base: Duration,
    pub dur_slow: Duration,
    pub ease_premium: CubicBezier,
    pub ease_soft: CubicBezier,
}

/// Terminal/editor font weight preference (`preferencesStore` `terminalFontWeight`:
/// `"normal" | "medium" | "bold"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonoFontWeight {
    Normal,
    Medium,
    Bold,
}

/// Typography tokens. `app_*` values are runtime-mutable in the original
/// (`useTypographyEngine.ts`); here they hold the CSS / `preferencesStore`
/// defaults. Font families name the bundled assets from [`crate::fonts`];
/// the `*_fallback` chains list system fonts to fall back to when an asset is
/// missing (mirrors the reference app's CSS font stacks). T13-003 (settings)
/// later overrides these at runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    pub sans_family: String,
    pub heading_family: String,
    /// UI font — `preferencesStore.appFontFamily` default.
    pub app_font_family: String,
    pub app_font_size: f32,
    pub app_line_height: f32,
    pub ui_font_fallback: Vec<String>,
    /// Code editor — `preferencesStore.editorFontFamily` / `editorFontSize`.
    pub buffer_font_family: String,
    pub buffer_font_size: f32,
    /// Terminal emulator — `preferencesStore.terminal*` defaults.
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_line_height: f32,
    pub terminal_letter_spacing: f32,
    pub terminal_font_weight: MonoFontWeight,
    pub mono_font_fallback: Vec<String>,
    /// Whether programming ligatures (`calt`) are enabled for mono text. The
    /// reference app always loads xterm's `LigaturesAddon`, so this is on.
    pub font_ligatures: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            dur_fast: Duration::from_millis(160),
            dur_base: Duration::from_millis(240),
            dur_slow: Duration::from_millis(320),
            ease_premium: CubicBezier {
                x1: 0.16,
                y1: 1.0,
                x2: 0.3,
                y2: 1.0,
            },
            ease_soft: CubicBezier {
                x1: 0.4,
                y1: 0.0,
                x2: 0.2,
                y2: 1.0,
            },
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        let ui_fallback: Vec<String> = crate::fonts::UI_FONT_FALLBACKS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mono_fallback: Vec<String> = crate::fonts::MONO_FONT_FALLBACKS
            .iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            sans_family: crate::fonts::UI_FONT_FAMILY.into(),
            heading_family: crate::fonts::UI_FONT_FAMILY.into(),
            app_font_family: crate::fonts::UI_FONT_FAMILY.into(),
            app_font_size: 16.0,
            app_line_height: 1.5,
            ui_font_fallback: ui_fallback,
            buffer_font_family: crate::fonts::MONO_FONT_FAMILY.into(),
            buffer_font_size: 15.0,
            terminal_font_family: crate::fonts::MONO_FONT_FAMILY.into(),
            terminal_font_size: 15.0,
            terminal_line_height: 1.05,
            terminal_letter_spacing: 0.0,
            terminal_font_weight: MonoFontWeight::Normal,
            mono_font_fallback: mono_fallback,
            font_ligatures: true,
        }
    }
}

fn shadow(x: f32, y: f32, blur: f32, spread: f32, alpha: f32) -> ShadowLayer {
    // Every shadow in globals.css uses `oklch(0% 0 0 / a)` — pure black.
    ShadowLayer {
        x,
        y,
        blur,
        spread,
        color: oklch_a(0.0, 0.0, 0.0, alpha),
    }
}

impl Theme {
    /// The default light theme — values from the `:root` block of `globals.css`.
    pub fn light() -> Self {
        Self {
            is_dark: false,
            core: CoreColors {
                background: oklch(97.0, 0.0, 0.0),
                foreground: oklch(20.9, 0.0, 0.0),
                card: oklch(100.0, 0.0, 0.0),
                card_foreground: oklch(20.9, 0.0, 0.0),
                popover: oklch(100.0, 0.0, 0.0),
                popover_foreground: oklch(20.9, 0.0, 0.0),
                primary: oklch(79.68, 0.1298, 82.18),
                primary_foreground: oklch(20.9, 0.0, 0.0),
                secondary: oklch(93.0, 0.0, 0.0),
                secondary_foreground: oklch(25.2, 0.0, 0.0),
                muted: oklch(93.0, 0.0, 0.0),
                muted_foreground: oklch(55.41, 0.022, 262.94),
                accent: oklch(93.0, 0.0, 0.0),
                accent_foreground: oklch(25.2, 0.0, 0.0),
                destructive: oklch(55.0, 0.1637, 17.27),
                destructive_foreground: oklch(100.0, 0.0, 0.0),
                border: oklch(88.0, 0.0, 0.0),
                input: oklch(88.0, 0.0, 0.0),
                ring: oklch(79.68, 0.1298, 82.18),
                charts: [
                    oklch(79.68, 0.1298, 82.18),
                    oklch(53.0, 0.1, 225.69),
                    oklch(52.0, 0.14, 125.86),
                    oklch(55.0, 0.1637, 17.27),
                    oklch(55.0, 0.1308, 306.3),
                ],
            },
            sidebar: SidebarColors {
                background: oklch(97.0, 0.0, 0.0),
                foreground: oklch(20.9, 0.0, 0.0),
                primary: oklch(79.68, 0.1298, 82.18),
                primary_foreground: oklch(20.9, 0.0, 0.0),
                accent: oklch(93.0, 0.0, 0.0),
                accent_foreground: oklch(25.2, 0.0, 0.0),
                border: oklch(88.0, 0.0, 0.0),
                ring: oklch(79.68, 0.1298, 82.18),
            },
            // `--toolbar/title-bar/status-bar-background: var(--card)`
            surface: SurfaceColors {
                toolbar: oklch(100.0, 0.0, 0.0),
                title_bar: oklch(100.0, 0.0, 0.0),
                status_bar: oklch(100.0, 0.0, 0.0),
            },
            border: BorderVariants {
                variant: oklch(88.0, 0.0, 0.0),        // var(--border)
                focused: oklch(79.68, 0.1298, 82.18),  // var(--ring)
                selected: oklch(79.68, 0.1298, 82.18), // var(--ring)
                transparent: transparent(),
                disabled: oklch(88.0, 0.0, 0.0),
            },
            status: StatusColors {
                modified: oklch(53.0, 0.1, 225.69),
                error: oklch(55.0, 0.1637, 17.27),
                warning: oklch(54.0, 0.14, 55.0),
                info: oklch(53.0, 0.1, 225.69),
                hint: oklch(55.41, 0.022, 262.94),
                success: oklch(52.0, 0.14, 125.86),
            },
            interaction: InteractionColors {
                cursor: oklch(20.9, 0.0, 0.0), // var(--foreground)
                selection: oklch_a(79.68, 0.1298, 82.18, 0.13),
            },
            terminal: TerminalPalette {
                background: oklch(100.0, 0.0, 0.0),
                foreground: oklch(20.9, 0.0, 0.0),
                bright_foreground: oklch(100.0, 0.0001, 259.98),
                dim_foreground: oklch(60.0, 0.0, 0.0),
                normal: AnsiColors {
                    black: oklch(20.9, 0.0, 259.98),
                    red: oklch(59.32, 0.1422, 16.44),
                    green: oklch(69.52, 0.1447, 125.28),
                    yellow: oklch(67.56, 0.1087, 82.29),
                    blue: oklch(62.72, 0.1048, 225.37),
                    magenta: oklch(67.62, 0.1088, 306.27),
                    cyan: oklch(73.24, 0.0743, 170.89),
                    white: oklch(79.8, 0.0099, 93.74),
                },
                bright: AnsiColors {
                    black: oklch(55.41, 0.022, 262.94),
                    red: oklch(76.32, 0.1408, 20.86),
                    green: oklch(93.51, 0.1593, 130.47),
                    yellow: oklch(89.59, 0.1392, 91.09),
                    blue: oklch(72.0, 0.1524, 249.03),
                    magenta: oklch(84.18, 0.1159, 315.2),
                    cyan: oklch(90.08, 0.0588, 232.06),
                    white: oklch(100.0, 0.0001, 259.98),
                },
                dim: AnsiColors {
                    black: oklch(20.9, 0.0, 259.98),
                    red: oklch(50.0, 0.11, 17.0),
                    green: oklch(64.0, 0.13, 125.0),
                    yellow: oklch(60.0, 0.09, 82.0),
                    blue: oklch(54.0, 0.09, 225.0),
                    magenta: oklch(60.0, 0.09, 306.0),
                    cyan: oklch(68.0, 0.07, 171.0),
                    white: oklch(60.0, 0.005, 93.0),
                },
                cursor: oklch(20.9, 0.0, 0.0),
                selection: oklch_a(79.68, 0.1298, 82.18, 0.13),
            },
            radius: RadiusScale::from_base(5.0),
            shadows: Shadows {
                row: vec![shadow(0.0, 1.0, 2.0, 0.0, 0.06)],
                popover: vec![
                    shadow(0.0, 4.0, 12.0, -2.0, 0.12),
                    shadow(0.0, 2.0, 4.0, -2.0, 0.08),
                ],
                modal: vec![
                    shadow(0.0, 16.0, 40.0, -8.0, 0.18),
                    shadow(0.0, 4.0, 12.0, -4.0, 0.1),
                ],
            },
            animation: Animation::default(),
            typography: Typography::default(),
        }
    }

    /// The default dark theme — values from the `.dark` block of `globals.css`.
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            core: CoreColors {
                background: oklch(20.9, 0.0, 259.98),
                foreground: oklch(94.52, 0.0001, 259.98),
                card: oklch(23.93, 0.0, 259.98),
                card_foreground: oklch(94.52, 0.0001, 259.98),
                popover: oklch(23.93, 0.0, 259.98),
                popover_foreground: oklch(94.52, 0.0001, 259.98),
                primary: oklch(79.68, 0.1298, 82.18),
                primary_foreground: oklch(20.9, 0.0, 259.98),
                secondary: oklch(35.62, 0.0, 259.98),
                secondary_foreground: oklch(94.52, 0.0001, 259.98),
                muted: oklch(31.32, 0.0, 259.98),
                muted_foreground: oklch(69.6, 0.0001, 259.98),
                accent: oklch(35.62, 0.0, 259.98),
                accent_foreground: oklch(94.52, 0.0001, 259.98),
                destructive: oklch(69.82, 0.1637, 17.27),
                destructive_foreground: oklch(100.0, 0.0, 0.0),
                border: oklch(28.91, 0.0, 259.98),
                input: oklch(31.32, 0.0, 259.98),
                ring: oklch(79.68, 0.1298, 82.18),
                charts: [
                    oklch(79.68, 0.1298, 82.18),
                    oklch(73.86, 0.1252, 225.69),
                    oklch(82.51, 0.1745, 125.86),
                    oklch(69.82, 0.1637, 17.27),
                    oklch(79.7, 0.1308, 306.3),
                ],
            },
            sidebar: SidebarColors {
                background: oklch(20.9, 0.0, 259.98),
                foreground: oklch(94.52, 0.0001, 259.98),
                primary: oklch(79.68, 0.1298, 82.18),
                primary_foreground: oklch(20.9, 0.0, 259.98),
                accent: oklch(35.62, 0.0, 259.98),
                accent_foreground: oklch(94.52, 0.0001, 259.98),
                border: oklch(28.91, 0.0, 259.98),
                ring: oklch(79.68, 0.1298, 82.18),
            },
            surface: SurfaceColors {
                toolbar: oklch(23.93, 0.0, 259.98),
                title_bar: oklch(23.93, 0.0, 259.98),
                status_bar: oklch(23.93, 0.0, 259.98),
            },
            border: BorderVariants {
                variant: oklch(28.91, 0.0, 259.98),
                focused: oklch(79.68, 0.1298, 82.18),
                selected: oklch(79.68, 0.1298, 82.18),
                transparent: transparent(),
                disabled: oklch(28.0, 0.0, 259.98),
            },
            status: StatusColors {
                modified: oklch(73.86, 0.1252, 225.69),
                error: oklch(69.82, 0.1637, 17.27),
                warning: oklch(80.0, 0.13, 55.0),
                info: oklch(73.86, 0.1252, 225.69),
                hint: oklch(50.0, 0.012, 262.94),
                success: oklch(82.51, 0.1745, 125.86),
            },
            interaction: InteractionColors {
                cursor: oklch(79.68, 0.1298, 82.18),
                selection: oklch_a(79.68, 0.1298, 82.18, 0.13),
            },
            terminal: TerminalPalette {
                background: oklch(20.9, 0.0, 259.98),
                foreground: oklch(94.52, 0.0001, 259.98),
                bright_foreground: oklch(100.0, 0.0001, 259.98),
                dim_foreground: oklch(61.5, 0.0, 259.98),
                normal: AnsiColors {
                    black: oklch(20.9, 0.0, 259.98),
                    red: oklch(69.82, 0.1637, 17.27),
                    green: oklch(82.51, 0.1745, 125.86),
                    yellow: oklch(79.68, 0.1298, 82.18),
                    blue: oklch(73.86, 0.1252, 225.69),
                    magenta: oklch(79.7, 0.1308, 306.3),
                    cyan: oklch(86.51, 0.0884, 171.17),
                    white: oklch(79.8, 0.0099, 93.74),
                },
                bright: AnsiColors {
                    black: oklch(55.41, 0.022, 262.94),
                    red: oklch(76.32, 0.1408, 20.86),
                    green: oklch(93.51, 0.1593, 130.47),
                    yellow: oklch(89.59, 0.1392, 91.09),
                    blue: oklch(72.0, 0.1524, 249.03),
                    magenta: oklch(84.18, 0.1159, 315.2),
                    cyan: oklch(90.08, 0.0588, 232.06),
                    white: oklch(100.0, 0.0001, 259.98),
                },
                dim: AnsiColors {
                    black: oklch(20.9, 0.0, 259.98),
                    red: oklch(50.2, 0.117, 17.27),
                    green: oklch(63.8, 0.135, 125.86),
                    yellow: oklch(60.2, 0.098, 82.18),
                    blue: oklch(54.6, 0.096, 225.69),
                    magenta: oklch(59.5, 0.098, 306.3),
                    cyan: oklch(68.5, 0.07, 171.17),
                    white: oklch(60.2, 0.005, 93.74),
                },
                cursor: oklch(79.68, 0.1298, 82.18),
                selection: oklch_a(79.68, 0.1298, 82.18, 0.13),
            },
            radius: RadiusScale::from_base(5.0),
            shadows: Shadows {
                row: vec![shadow(0.0, 1.0, 2.0, 0.0, 0.35)],
                popover: vec![
                    shadow(0.0, 4.0, 16.0, -2.0, 0.55),
                    shadow(0.0, 2.0, 6.0, -2.0, 0.35),
                ],
                modal: vec![
                    shadow(0.0, 20.0, 48.0, -8.0, 0.65),
                    shadow(0.0, 6.0, 16.0, -4.0, 0.4),
                ],
            },
            animation: Animation::default(),
            typography: Typography::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::to_rgb8;

    fn close(a: [u8; 3], b: [u8; 3], tol: i32) -> bool {
        a.iter()
            .zip(b.iter())
            .all(|(x, y)| (*x as i32 - *y as i32).abs() <= tol)
    }

    #[test]
    fn both_variants_build() {
        let _ = Theme::light();
        let _ = Theme::dark();
    }

    #[test]
    fn dark_spot_checks_against_globals_css() {
        let d = Theme::dark();
        // --primary #E6B450, --destructive #F26D78, --accent #3C3C3C, --border #2B2B2B
        assert!(close(to_rgb8(d.core.primary), [0xE6, 0xB4, 0x50], 3));
        assert!(close(to_rgb8(d.core.destructive), [0xF2, 0x6D, 0x78], 3));
        assert!(close(to_rgb8(d.core.accent), [0x3C, 0x3C, 0x3C], 3));
        assert!(close(to_rgb8(d.core.border), [0x2B, 0x2B, 0x2B], 3));
    }

    #[test]
    fn selection_carries_alpha() {
        assert!((Theme::dark().interaction.selection.a - 0.13).abs() < 1e-6);
        assert!((Theme::light().interaction.selection.a - 0.13).abs() < 1e-6);
    }

    #[test]
    fn radius_scale_multipliers() {
        let r = Theme::light().radius;
        assert_eq!(r.base, 5.0);
        assert_eq!(r.lg, 5.0);
        assert!((r.sm - 3.0).abs() < 1e-6);
        assert!((r.xl4 - 13.0).abs() < 1e-6);
        assert_eq!(r.window, 12.0);
    }

    #[test]
    fn shadow_layer_counts() {
        let s = Theme::dark().shadows;
        assert_eq!(s.row.len(), 1);
        assert_eq!(s.popover.len(), 2);
        assert_eq!(s.modal.len(), 2);
        assert!((s.modal[0].color.a - 0.65).abs() < 1e-6);
    }

    #[test]
    fn animation_durations() {
        let a = Theme::light().animation;
        assert_eq!(a.dur_fast, Duration::from_millis(160));
        assert_eq!(a.dur_base, Duration::from_millis(240));
        assert_eq!(a.dur_slow, Duration::from_millis(320));
    }

    #[test]
    fn cubic_bezier_eval_endpoints_and_monotonic() {
        let e = Theme::dark().animation.ease_premium;
        assert!(e.eval(0.0).abs() < 1e-4, "starts at 0");
        assert!((e.eval(1.0) - 1.0).abs() < 1e-4, "ends at 1");
        // `--ease-premium` (0.16, 1, 0.3, 1) is an "ease-out expo" curve: it
        // races ahead of linear early on.
        assert!(e.eval(0.25) > 0.25);
        assert!(e.eval(0.5) > 0.5);
        // Non-decreasing over the domain.
        let mut prev = -1.0;
        for i in 0..=20 {
            let y = e.eval(i as f32 / 20.0);
            assert!(y + 1e-4 >= prev, "monotonic at t={}", i);
            prev = y;
        }
        // Linear identity curve round-trips.
        let lin = CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        };
        assert!((lin.eval(0.4) - 0.4).abs() < 1e-3);
    }

    #[test]
    fn tab_in_from_scale_matches_reference_keyframe() {
        // `@keyframes labonair-tab-in { from { transform: scale(0.86) } }`
        assert!((TAB_IN_FROM_SCALE - 0.86).abs() < 1e-6);
    }

    use crate::contrast::contrast_ratio as contrast;

    #[test]
    fn body_text_meets_wcag_aa_contrast() {
        // Primary UI + terminal text on their own backgrounds must clear the
        // 4.5:1 AA threshold in both variants — a regression guard so a token
        // edit can't silently make text unreadable.
        for t in [Theme::light(), Theme::dark()] {
            assert!(
                contrast(t.core.foreground, t.core.background) >= 4.5,
                "fg/bg contrast too low ({}): {:.2}",
                if t.is_dark { "dark" } else { "light" },
                contrast(t.core.foreground, t.core.background)
            );
            assert!(
                contrast(t.terminal.foreground, t.terminal.background) >= 4.5,
                "terminal fg/bg contrast too low ({}): {:.2}",
                if t.is_dark { "dark" } else { "light" },
                contrast(t.terminal.foreground, t.terminal.background)
            );
            // Muted/secondary text is allowed to be dimmer but must stay legible
            // (WCAG AA large-text / UI-component threshold of 3:1).
            assert!(
                contrast(t.core.muted_foreground, t.core.background) >= 3.0,
                "muted-fg contrast too low ({}): {:.2}",
                if t.is_dark { "dark" } else { "light" },
                contrast(t.core.muted_foreground, t.core.background)
            );
        }
    }

    #[test]
    fn terminal_palette_is_fully_populated() {
        // Compile-time guarantee of 8+8+8; here assert the groups differ so we
        // know no row was left as a copy/paste of another.
        let t = Theme::dark().terminal;
        assert_ne!(t.normal.red, t.bright.red);
        assert_ne!(t.normal.red, t.dim.red);
        assert_ne!(t.background, t.foreground);
    }
}
