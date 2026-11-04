//! Typed design tokens for the Labonair UI.
//!
//! Base palette colors follow the frozen reference stylesheet. Labonair's
//! native surface opacity and radius choices are maintained here as semantic
//! tokens. Colors are stored as [`gpui::Hsla`], converted once from their
//! original Oklch notation by [`crate::color::oklch`].

use std::time::Duration;

use gpui::Hsla;

use crate::color::{oklch_a, parse_color, transparent};

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
    /// Hover on an already-hovered fill (e.g. the × in a hovered tab).
    pub hover_strong: Hsla,
    /// The selected/active fill — the one Ciel active state (§8): rows, tabs,
    /// nav items and toggles read as "current" through this fill plus a
    /// semibold label, never an accent tint.
    pub sunken: Hsla,
    /// The neutral high-contrast primary-action fill (`--action-surface`).
    pub action: Hsla,
    pub action_foreground: Hsla,
    pub action_hover: Hsla,
    pub action_active: Hsla,
}

/// Role-based opacity for surfaces layered over GPUI's native blurred window
/// background. Solid theme colors remain the source hue; this scale controls
/// how much of the platform material remains visible through each surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrancyLayer {
    Canvas,
    Toolbar,
    Sidebar,
    Card,
    Popover,
    Control,
    Divider,
}

impl VibrancyLayer {
    /// The foreground material opacity for this semantic surface role.
    pub const fn opacity(self) -> f32 {
        match self {
            Self::Canvas => 0.92,
            Self::Toolbar => 0.82,
            Self::Sidebar => 0.86,
            Self::Card => 0.94,
            Self::Popover => 0.97,
            Self::Control => 0.94,
            Self::Divider => 0.56,
        }
    }

    /// Resolve the surface color for this role. With `translucent` off (the
    /// Ciel default — fully opaque, fill-contrast surfaces) the theme color is
    /// returned untouched; with it on, this role's material opacity is applied
    /// without changing the theme color itself.
    pub fn resolve(self, color: Hsla, translucent: bool) -> Hsla {
        if translucent {
            color.opacity(self.opacity())
        } else {
            color
        }
    }
}

/// Border variant tokens (`--border-*`).
#[derive(Debug, Clone, PartialEq)]
pub struct BorderVariants {
    pub variant: Hsla,
    pub focused: Hsla,
    pub selected: Hsla,
    pub transparent: Hsla,
    pub disabled: Hsla,
    /// `--line-strong`: popover/dialog hairlines and active segment rings.
    pub strong: Hsla,
    /// `--line-subtle`: row separators inside a filled group.
    pub subtle: Hsla,
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

/// Corner-radius scale in logical pixels (Ciel §6): `sm` 4 · `md` 8 · `lg` 12 ·
/// `xl` 14 · `xl2` 20 · `xl3` 22 (message bubble) · `xl4` 30 (modal). Controls
/// use `md`; panels, popovers and tiles `lg`–`xl`; the fixed window radius is
/// not part of the scale.
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
    /// The Ciel radius scale.
    pub const fn ciel() -> Self {
        Self {
            base: 12.0,
            sm: 4.0,
            md: 8.0,
            lg: 12.0,
            xl: 14.0,
            xl2: 20.0,
            xl3: 22.0,
            xl4: 30.0,
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
            dur_fast: Duration::from_millis(120),
            dur_base: Duration::from_millis(160),
            dur_slow: Duration::from_millis(200),
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
        let ui_fallback: Vec<String> = crate::font_families::UI_FONT_FALLBACKS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mono_fallback: Vec<String> = crate::font_families::MONO_FONT_FALLBACKS
            .iter()
            .map(|s| s.to_string())
            .collect();
        Self {
            sans_family: crate::font_families::UI_FONT_FAMILY.into(),
            heading_family: crate::font_families::UI_FONT_FAMILY.into(),
            app_font_family: crate::font_families::UI_FONT_FAMILY.into(),
            app_font_size: 13.0,
            app_line_height: 1.5,
            ui_font_fallback: ui_fallback,
            buffer_font_family: crate::font_families::MONO_FONT_FAMILY.into(),
            buffer_font_size: 13.0,
            terminal_font_family: crate::font_families::MONO_FONT_FAMILY.into(),
            terminal_font_size: 14.0,
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

/// Parse a static theme literal (`#rrggbb` / `#rrggbbaa`). The literals below
/// are compile-time constants covered by `both_variants_build`, so a typo
/// fails the first test run rather than a user session.
fn hex(value: &str) -> Hsla {
    parse_color(value).expect("static theme literal is a valid color")
}

impl Theme {
    /// The default light theme: Ciel's warm-neutral palette (`tokens.css`
    /// `:root`). Surfaces separate by fill contrast — `background` is Ciel's
    /// `--surface`, `card`/`muted`/`secondary` its `--surface-muted`,
    /// `accent` its hover fill, `surface.sunken` its selected fill, and the
    /// chrome (`toolbar`/`sidebar`) its `--surface-chrome`.
    pub fn light() -> Self {
        Self {
            is_dark: false,
            core: CoreColors {
                background: hex("#ffffff"),
                foreground: hex("#2a2822"),
                card: hex("#f0eee6"),
                card_foreground: hex("#2a2822"),
                popover: hex("#ffffff"),
                popover_foreground: hex("#2a2822"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                secondary: hex("#f0eee6"),
                secondary_foreground: hex("#4a473c"),
                muted: hex("#f0eee6"),
                muted_foreground: hex("#5c5849"),
                accent: hex("#f0ede4"),
                accent_foreground: hex("#2a2822"),
                destructive: hex("#e8222d"),
                destructive_foreground: hex("#ffffff"),
                border: hex("#e3e2dd"),
                input: hex("#dcdad5"),
                ring: hex("#6b6b70"),
                charts: [
                    hex("#2a78d6"),
                    hex("#eb6834"),
                    hex("#1baf7a"),
                    hex("#eda100"),
                    hex("#e87ba4"),
                ],
            },
            sidebar: SidebarColors {
                background: hex("#f5f4ee"),
                foreground: hex("#2a2822"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                accent: hex("#f0ede4"),
                accent_foreground: hex("#2a2822"),
                border: hex("#e3e2dd"),
                ring: hex("#6b6b70"),
            },
            // `--surface-chrome` for every window bar and dock.
            surface: SurfaceColors {
                toolbar: hex("#f5f4ee"),
                title_bar: hex("#f5f4ee"),
                status_bar: hex("#f5f4ee"),
                hover_strong: hex("#e2dccb"),
                sunken: hex("#d8d0ba"),
                action: hex("#1c1c1e"),
                action_foreground: hex("#ffffff"),
                action_hover: hex("#343436"),
                action_active: hex("#080809"),
            },
            border: BorderVariants {
                variant: hex("#e3e2dd"),  // --line
                focused: hex("#6b6b70"),  // --ring
                selected: hex("#dcdad5"), // --line-strong
                transparent: transparent(),
                disabled: hex("#edece7"), // --line-subtle
                strong: hex("#dcdad5"),
                subtle: hex("#edece7"),
            },
            status: StatusColors {
                modified: hex("#3a6ea5"),
                error: hex("#c0392b"),
                warning: hex("#b07a2a"),
                info: hex("#3a6ea5"),
                hint: hex("#6f6b5c"),
                success: hex("#5a7d3c"),
            },
            interaction: InteractionColors {
                cursor: hex("#2a2822"),
                selection: hex("#007cf740"),
            },
            terminal: TerminalPalette {
                background: hex("#faf9f5"),
                foreground: hex("#3d3a2f"),
                bright_foreground: hex("#1a1814"),
                dim_foreground: hex("#73705f"),
                normal: AnsiColors {
                    black: hex("#3d3a2f"),
                    red: hex("#c0141d"),
                    green: hex("#187a41"),
                    yellow: hex("#9c5200"),
                    blue: hex("#0064c8"),
                    magenta: hex("#a05278"),
                    cyan: hex("#127589"),
                    white: hex("#cbc8bd"),
                },
                bright: AnsiColors {
                    black: hex("#73705f"),
                    red: hex("#e8222d"),
                    green: hex("#1fa85a"),
                    yellow: hex("#f08a00"),
                    blue: hex("#007cf7"),
                    magenta: hex("#b0608a"),
                    cyan: hex("#16849b"),
                    white: hex("#3d3a2f"),
                },
                dim: AnsiColors {
                    black: hex("#3d3a2f"),
                    red: hex("#a3262c"),
                    green: hex("#3f7a56"),
                    yellow: hex("#8a6224"),
                    blue: hex("#2a63a8"),
                    magenta: hex("#8a5a73"),
                    cyan: hex("#2f7c8a"),
                    white: hex("#a8a597"),
                },
                cursor: hex("#007cf7"),
                selection: hex("#007cf740"),
            },
            radius: RadiusScale::ciel(),
            shadows: Shadows {
                row: vec![shadow(0.0, 1.0, 2.0, 0.0, 0.04)],
                popover: vec![
                    shadow(0.0, 12.0, 32.0, 0.0, 0.12),
                    shadow(0.0, 1.0, 4.0, 0.0, 0.06),
                ],
                // Ciel dialogs carry a 1px `line-strong` border, not a shadow.
                modal: Vec::new(),
            },
            animation: Animation::default(),
            typography: Typography::default(),
        }
    }

    /// The default dark theme: Ciel's neutral dark palette (`tokens.css`
    /// `.dark`) with the blue accent.
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            core: CoreColors {
                background: hex("#1a1a1a"),
                foreground: hex("#ffffff"),
                card: hex("#212121"),
                card_foreground: hex("#ffffff"),
                popover: hex("#1a1a1a"),
                popover_foreground: hex("#ffffff"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                secondary: hex("#212121"),
                secondary_foreground: hex("#dcdcdc"),
                muted: hex("#212121"),
                muted_foreground: hex("#979797"),
                accent: hex("#2e2e2e"),
                accent_foreground: hex("#ffffff"),
                destructive: hex("#fd2f3b"),
                destructive_foreground: hex("#ffffff"),
                border: hex("#ffffff0b"),
                input: hex("#ffffff14"),
                ring: hex("#979797"),
                charts: [
                    hex("#3987e5"),
                    hex("#d95926"),
                    hex("#199e70"),
                    hex("#c98500"),
                    hex("#d55181"),
                ],
            },
            sidebar: SidebarColors {
                background: hex("#0d0d0d"),
                foreground: hex("#ffffff"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                accent: hex("#2e2e2e"),
                accent_foreground: hex("#ffffff"),
                border: hex("#ffffff0b"),
                ring: hex("#979797"),
            },
            surface: SurfaceColors {
                toolbar: hex("#0d0d0d"),
                title_bar: hex("#0d0d0d"),
                status_bar: hex("#0d0d0d"),
                hover_strong: hex("#4d4d4d"),
                sunken: hex("#4a4a4a"),
                action: hex("#f0f0f0"),
                action_foreground: hex("#141414"),
                action_hover: hex("#cdcdcd"),
                action_active: hex("#c8c8c8"),
            },
            border: BorderVariants {
                variant: hex("#ffffff0b"),
                focused: hex("#cdcdcd"), // --field-ring
                selected: hex("#ffffff14"),
                transparent: transparent(),
                disabled: hex("#ffffff09"),
                strong: hex("#ffffff14"),
                subtle: hex("#ffffff09"),
            },
            status: StatusColors {
                modified: hex("#6bc7d9"),
                error: hex("#fd2f3b"),
                warning: hex("#ff9412"),
                info: hex("#6bc7d9"),
                hint: hex("#6a6a6a"),
                success: hex("#31cf76"),
            },
            interaction: InteractionColors {
                cursor: hex("#ffffff"),
                selection: hex("#007cf740"),
            },
            terminal: TerminalPalette {
                background: hex("#1a1a1a"),
                foreground: hex("#dcdcdc"),
                bright_foreground: hex("#ffffff"),
                dim_foreground: hex("#979797"),
                normal: AnsiColors {
                    black: hex("#141414"),
                    red: hex("#fd2f3b"),
                    green: hex("#31cf76"),
                    yellow: hex("#ff9412"),
                    blue: hex("#42a0ff"),
                    magenta: hex("#d6adf2"),
                    cyan: hex("#6bc7d9"),
                    white: hex("#dcdcdc"),
                },
                bright: AnsiColors {
                    black: hex("#6a6a6a"),
                    red: hex("#ff6069"),
                    green: hex("#48e18a"),
                    yellow: hex("#ffab43"),
                    blue: hex("#74b9ff"),
                    magenta: hex("#e4c1f7"),
                    cyan: hex("#8bdee8"),
                    white: hex("#ffffff"),
                },
                dim: AnsiColors {
                    black: hex("#141414"),
                    red: hex("#b8353d"),
                    green: hex("#2a9a5a"),
                    yellow: hex("#b87020"),
                    blue: hex("#3278bb"),
                    magenta: hex("#a585bb"),
                    cyan: hex("#4f93a0"),
                    white: hex("#9a9a9a"),
                },
                cursor: hex("#007cf7"),
                selection: hex("#007cf740"),
            },
            radius: RadiusScale::ciel(),
            shadows: Shadows {
                row: vec![shadow(0.0, 1.0, 2.0, 0.0, 0.24)],
                popover: vec![
                    shadow(0.0, 12.0, 32.0, 0.0, 0.26),
                    shadow(0.0, 1.0, 4.0, 0.0, 0.18),
                ],
                modal: Vec::new(),
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
    fn vibrancy_layers_keep_a_clear_material_hierarchy() {
        assert!(VibrancyLayer::Toolbar.opacity() < VibrancyLayer::Canvas.opacity());
        assert!(VibrancyLayer::Sidebar.opacity() < VibrancyLayer::Card.opacity());
        assert!(VibrancyLayer::Card.opacity() < VibrancyLayer::Popover.opacity());
        assert_eq!(VibrancyLayer::Divider.opacity(), 0.56);

        let base = oklch_a(55.0, 0.03, 250.0, 0.5);
        let material = VibrancyLayer::Card.resolve(base, true);
        assert_eq!(VibrancyLayer::Card.resolve(base, false), base);
        assert_eq!(
            (material.h, material.s, material.l),
            (base.h, base.s, base.l)
        );
        assert!((material.a - base.a * VibrancyLayer::Card.opacity()).abs() < 1e-6);
    }

    #[test]
    fn ciel_theme_defaults_use_the_ciel_radius_scale_and_soft_dark_shadows() {
        for theme in [Theme::light(), Theme::dark()] {
            assert_eq!(theme.radius, RadiusScale::ciel());
            // Ciel dialogs are bordered, not shadowed.
            assert!(theme.shadows.modal.is_empty());
        }

        let dark = Theme::dark();
        assert!(dark.shadows.row[0].color.a < 0.3);
        assert!(dark.shadows.popover[0].color.a < 0.4);
    }

    #[test]
    fn both_variants_build() {
        let _ = Theme::light();
        let _ = Theme::dark();
    }

    #[test]
    fn dark_spot_checks_against_ciel_tokens() {
        let d = Theme::dark();
        // --accent #007cf7, --surface #1a1a1a, --surface-chrome #0d0d0d,
        // --surface-sunken #4a4a4a, --destructive #fd2f3b
        assert!(close(to_rgb8(d.core.primary), [0x00, 0x7C, 0xF7], 1));
        assert!(close(to_rgb8(d.core.background), [0x1A, 0x1A, 0x1A], 1));
        assert!(close(to_rgb8(d.surface.toolbar), [0x0D, 0x0D, 0x0D], 1));
        assert!(close(to_rgb8(d.surface.sunken), [0x4A, 0x4A, 0x4A], 1));
        assert!(close(to_rgb8(d.core.destructive), [0xFD, 0x2F, 0x3B], 1));
    }

    #[test]
    fn light_spot_checks_against_ciel_tokens() {
        let l = Theme::light();
        // --surface #fff, --surface-chrome #f5f4ee, --surface-sunken #d8d0ba,
        // --action-surface #1c1c1e
        assert!(close(to_rgb8(l.core.background), [0xFF, 0xFF, 0xFF], 1));
        assert!(close(to_rgb8(l.surface.toolbar), [0xF5, 0xF4, 0xEE], 1));
        assert!(close(to_rgb8(l.surface.sunken), [0xD8, 0xD0, 0xBA], 1));
        assert!(close(to_rgb8(l.surface.action), [0x1C, 0x1C, 0x1E], 1));
    }

    #[test]
    fn selection_carries_alpha() {
        assert!((Theme::dark().interaction.selection.a - 0.25).abs() < 0.01);
        assert!((Theme::light().interaction.selection.a - 0.25).abs() < 0.01);
    }

    #[test]
    fn radius_scale_is_the_ciel_scale() {
        let r = Theme::light().radius;
        assert_eq!((r.sm, r.md, r.lg, r.xl), (4.0, 8.0, 12.0, 14.0));
        assert_eq!((r.xl2, r.xl3, r.xl4), (20.0, 22.0, 30.0));
        assert_eq!(r.window, 12.0);
        assert_eq!(r.scaled(2.0).md, 16.0);
        assert_eq!(r.scaled(2.0).window, 12.0);
    }

    #[test]
    fn shadow_layer_counts() {
        let s = Theme::dark().shadows;
        assert_eq!(s.row.len(), 1);
        assert_eq!(s.popover.len(), 2);
        assert!((s.popover[0].color.a - 0.26).abs() < 1e-6);
    }

    #[test]
    fn animation_durations() {
        let a = Theme::light().animation;
        assert_eq!(a.dur_fast, Duration::from_millis(120));
        assert_eq!(a.dur_base, Duration::from_millis(160));
        assert_eq!(a.dur_slow, Duration::from_millis(200));
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
