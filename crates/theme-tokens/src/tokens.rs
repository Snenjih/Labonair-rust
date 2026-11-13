//! Typed design tokens for the Labonair UI.
//!
//! Base palette colors follow the frozen reference stylesheet. Labonair's
//! native surface opacity and radius choices are maintained here as semantic
//! tokens. Colors are stored as [`gpui::Hsla`], converted once from their
//! original Oklch notation by [`crate::color::oklch`].

use std::time::Duration;

use gpui::Hsla;

use crate::color::{parse_color, transparent};

/// The complete set of design tokens for one theme variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// `true` for the dark variant, `false` for light.
    pub is_dark: bool,
    pub core: CoreColors,
    pub sidebar: SidebarColors,
    pub surface: SurfaceColors,
    pub float: FloatColors,
    pub border: BorderVariants,
    pub status: StatusColors,
    pub diff: DiffColors,
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

/// The floating layer: a gradient **canvas** behind the window, **cards**
/// (panes, docks) that float on it, the pill **track** that holds the tabs and
/// the neutral **state** overlays every control uses for hover and press.
///
/// Every value is a plain colour a theme can replace. A theme that wants the
/// flat look sets the four canvas stops to one colour and the card fills to
/// `background`; nothing else in the UI depends on the gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatColors {
    /// The canvas gradient, top to bottom (`canvas_top` → `canvas_mid` over the
    /// first 38 %, → `canvas_low` over the next 32 %, → `canvas_bottom`).
    pub canvas_top: Hsla,
    pub canvas_mid: Hsla,
    pub canvas_low: Hsla,
    pub canvas_bottom: Hsla,
    /// A horizontal wash from 35 % of the width to the right edge.
    pub canvas_tint: Hsla,
    /// The fill of a floating card (pane, dock) at rest and while focused.
    pub pane: Hsla,
    pub pane_focused: Hsla,
    /// The 1 px rim of a card at rest and while focused.
    pub pane_border: Hsla,
    pub pane_border_focused: Hsla,
    /// The pill that holds the tabs, and its 0.5 px rim.
    pub track: Hsla,
    pub track_border: Hsla,
    /// The selected tab's pill.
    pub tab_active: Hsla,
    /// The 1 px divider between two inactive tabs.
    pub tab_separator: Hsla,
    /// Neutral hover / pressed overlays for borderless controls. Alpha black on
    /// light, alpha white on dark, so they work on any surface.
    pub hover: Hsla,
    pub pressed: Hsla,
}

/// How surface fills relate to what is painted behind them.
///
/// This is the single switch behind the Classic/Glassy interface style and the
/// background image: [`VibrancyLayer::resolve`] maps a material plus a semantic
/// layer to an alpha. `bool` converts for callers that only know the
/// `appearance.translucentSurfaces` setting (`true` = [`Self::Glassy`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceMaterial {
    /// Opaque, fill-contrast Ciel surfaces (Classic, no background image).
    #[default]
    Opaque,
    /// Surfaces over the native window blur (Glassy, no background image).
    Glassy(u8),
    /// Classic with a background image: no native blur behind, so the fills
    /// carry just enough transparency to let the image read through.
    ImageClassic,
    /// Glassy with a background image: the image sits over the native blur and
    /// the fills are the most transparent.
    ImageGlassy(u8),
}

/// Default glass transparency (`appearance.glassTransparency`, 0–100): the
/// strength at which the [`VibrancyLayer`] tables apply unscaled.
pub const GLASS_TRANSPARENCY_DEFAULT: u8 = 30;

impl SurfaceMaterial {
    /// Pick the material from the independent inputs; `glass` is the 0–100
    /// glass transparency and only matters when `glassy`.
    pub fn new(glassy: bool, background_image: bool, glass: u8) -> Self {
        match (glassy, background_image) {
            (false, false) => Self::Opaque,
            (true, false) => Self::Glassy(glass),
            (false, true) => Self::ImageClassic,
            (true, true) => Self::ImageGlassy(glass),
        }
    }

    /// Whether a background image is showing through the surfaces.
    pub fn has_image(self) -> bool {
        matches!(self, Self::ImageClassic | Self::ImageGlassy(_))
    }
}

impl From<bool> for SurfaceMaterial {
    fn from(glassy: bool) -> Self {
        Self::new(glassy, false, GLASS_TRANSPARENCY_DEFAULT)
    }
}

/// Role-based opacity for surfaces layered over GPUI's native blurred window
/// background and/or the background image. Solid theme colors remain the source
/// hue; this scale controls how much of the layer behind each surface remains
/// visible through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrancyLayer {
    /// Main content surfaces: workspace area, editor.
    Canvas,
    Toolbar,
    Sidebar,
    Card,
    Popover,
    Control,
    Divider,
    /// The terminal surface fill; matches `Canvas` so terminals are as glassy as
    /// the rest of the app (on top of the terminal's own opacity setting).
    Terminal,
}

impl VibrancyLayer {
    /// The foreground material opacity for this semantic surface role over the
    /// native blur ([`SurfaceMaterial::Glassy`]).
    pub const fn opacity(self) -> f32 {
        match self {
            Self::Canvas => 0.92,
            Self::Toolbar => 0.82,
            Self::Sidebar => 0.86,
            Self::Card => 0.94,
            Self::Popover => 0.97,
            Self::Control => 0.94,
            Self::Divider => 0.56,
            Self::Terminal => 0.92,
        }
    }

    /// The material opacity for this role in `material`.
    ///
    /// With a background image the large surfaces (`Canvas`, `Toolbar`,
    /// `Sidebar`, `Terminal`) step down to ~0.78 in Classic and ~0.6 in Glassy
    /// so the image reads through them; floating layers (`Popover`) stay near
    /// opaque so menus remain readable on any image, and controls/cards keep
    /// enough fill to hold their text. Dividers do not change.
    pub fn opacity_in(self, material: SurfaceMaterial) -> f32 {
        match material {
            SurfaceMaterial::Opaque => 1.0,
            SurfaceMaterial::Glassy(glass) => self.glass_scaled(self.opacity(), glass),
            SurfaceMaterial::ImageClassic => match self {
                Self::Canvas | Self::Toolbar | Self::Sidebar => 0.78,
                Self::Terminal => 0.74,
                Self::Card => 0.86,
                Self::Popover => 0.97,
                Self::Control => 0.88,
                Self::Divider => 0.56,
            },
            SurfaceMaterial::ImageGlassy(glass) => self.glass_scaled(
                match self {
                    Self::Canvas | Self::Toolbar | Self::Sidebar => 0.6,
                    Self::Terminal => 0.55,
                    Self::Card => 0.74,
                    Self::Popover => 0.94,
                    Self::Control => 0.8,
                    Self::Divider => 0.5,
                },
                glass,
            ),
        }
    }

    /// Scale how far `base` sits below fully opaque by `glass` relative to
    /// [`GLASS_TRANSPARENCY_DEFAULT`] (0 = opaque, default = `base`). Dividers
    /// never change and nothing drops below 0.2 so text stays legible.
    fn glass_scaled(self, base: f32, glass: u8) -> f32 {
        if self == Self::Divider {
            return base;
        }
        let k = f32::from(glass.min(100)) / f32::from(GLASS_TRANSPARENCY_DEFAULT);
        (1.0 - (1.0 - base) * k).max(0.2)
    }

    /// Resolve the surface color for this role. With an opaque material (the
    /// Ciel default — fully opaque, fill-contrast surfaces) the theme color is
    /// returned untouched; otherwise this role's material opacity is applied
    /// without changing the theme color itself. Accepts a plain `bool`
    /// (`appearance.translucentSurfaces`) or a [`SurfaceMaterial`].
    pub fn resolve(self, color: Hsla, material: impl Into<SurfaceMaterial>) -> Hsla {
        match material.into() {
            SurfaceMaterial::Opaque => color,
            material => color.opacity(self.opacity_in(material)),
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

/// Optional per-theme overrides of the diff hunk colors (Zed's
/// `editor_diff_hunk_*` / `version_control_word_*` roles). `None` derives the
/// role from the status colors at use time, so a theme that only changes
/// `success` / `error` still gets matching diff tones.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiffColors {
    pub added_background: Option<Hsla>,
    pub added_hollow_background: Option<Hsla>,
    pub added_hollow_border: Option<Hsla>,
    pub added_word: Option<Hsla>,
    pub deleted_background: Option<Hsla>,
    pub deleted_hollow_background: Option<Hsla>,
    pub deleted_hollow_border: Option<Hsla>,
    pub deleted_word: Option<Hsla>,
}

/// The four colors of one side (added / deleted) of a diff hunk once the
/// theme's overrides and the derived defaults are merged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffSideColors {
    pub background: Hsla,
    pub hollow_background: Hsla,
    pub hollow_border: Hsla,
    pub word: Hsla,
}

impl DiffColors {
    /// `(added, deleted)` colors: an override wins, otherwise the role is the
    /// status color at a fixed alpha (stronger fills on light themes).
    pub fn resolve(&self, status: &StatusColors, dark: bool) -> (DiffSideColors, DiffSideColors) {
        let (fill, hollow_fill, outline, word) = if dark {
            (0.14, 0.05, 0.45, 0.35)
        } else {
            (0.18, 0.07, 0.55, 0.30)
        };
        let side = |base: Hsla, over: [Option<Hsla>; 4]| DiffSideColors {
            background: over[0].unwrap_or_else(|| base.opacity(fill)),
            hollow_background: over[1].unwrap_or_else(|| base.opacity(hollow_fill)),
            hollow_border: over[2].unwrap_or_else(|| base.opacity(outline)),
            word: over[3].unwrap_or_else(|| base.opacity(word)),
        };
        (
            side(
                status.success,
                [
                    self.added_background,
                    self.added_hollow_background,
                    self.added_hollow_border,
                    self.added_word,
                ],
            ),
            side(
                status.error,
                [
                    self.deleted_background,
                    self.deleted_hollow_background,
                    self.deleted_hollow_border,
                    self.deleted_word,
                ],
            ),
        )
    }
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
    /// A raised control at rest: the active tab pill, a tile.
    pub row: Vec<ShadowLayer>,
    /// A floating card (pane, dock) at rest.
    pub pane: Vec<ShadowLayer>,
    /// The focused card: same geometry, stronger alpha.
    pub pane_focused: Vec<ShadowLayer>,
    /// Menus, dropdowns, tooltips, toasts.
    pub popover: Vec<ShadowLayer>,
    /// Dialogs and the command palette.
    pub modal: Vec<ShadowLayer>,
}

/// A [`Shadows`] tier as a `Copy` value (at most [`ShadowStack::MAX`] layers),
/// so it can ride along in the `Copy` ui-kit `Palette`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowStack {
    layers: [Option<ShadowLayer>; Self::MAX],
}

impl ShadowStack {
    pub const MAX: usize = 3;

    /// The first [`Self::MAX`] layers of `layers`.
    pub fn from_layers(layers: &[ShadowLayer]) -> Self {
        let mut out = [None; Self::MAX];
        for (slot, layer) in out.iter_mut().zip(layers) {
            *slot = Some(*layer);
        }
        Self { layers: out }
    }

    /// The stack as GPUI box shadows, ready for `.shadow(..)`.
    pub fn to_box_shadows(&self) -> Vec<gpui::BoxShadow> {
        self.layers
            .iter()
            .flatten()
            .map(|l| gpui::BoxShadow {
                color: l.color,
                offset: gpui::point(gpui::px(l.x), gpui::px(l.y)),
                blur_radius: gpui::px(l.blur),
                spread_radius: gpui::px(l.spread),
            })
            .collect()
    }
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

/// One shadow layer in the colour `rgb` (0–255 per channel) at `alpha`.
fn shadow(x: f32, y: f32, blur: f32, spread: f32, rgb: [u8; 3], alpha: f32) -> ShadowLayer {
    ShadowLayer {
        x,
        y,
        blur,
        spread,
        color: rgba8(rgb, alpha),
    }
}

/// `rgb` (0–255 per channel) at `alpha` as an [`Hsla`].
fn rgba8(rgb: [u8; 3], alpha: f32) -> Hsla {
    let c = |v: u8| f32::from(v) / 255.0;
    let mut out: Hsla = gpui::Rgba {
        r: c(rgb[0]),
        g: c(rgb[1]),
        b: c(rgb[2]),
        a: alpha,
    }
    .into();
    out.a = alpha;
    out
}

/// Light shadows are tinted toward the canvas (a warm plum), never pure black:
/// pure black on a pastel canvas reads as dirt.
const LIGHT_SHADOW: [u8; 3] = [60, 40, 50];
const DARK_SHADOW: [u8; 3] = [0, 0, 0];

/// The shadow recipe of the floating layer. Every tier is a tight **contact**
/// layer plus a wide, soft **ambient** layer; the ambient layer's negative
/// spread keeps it from leaking past the card's edges. `rest` and `lift` are
/// the alphas of an unfocused and a focused/raised element.
fn shadow_tiers(rgb: [u8; 3], rest: f32, lift: f32) -> Shadows {
    Shadows {
        row: vec![
            shadow(0.0, 1.0, 2.0, 0.0, rgb, rest),
            shadow(0.0, 2.0, 8.0, 0.0, rgb, rest),
        ],
        pane: vec![
            shadow(0.0, 1.0, 2.0, 0.0, rgb, rest),
            shadow(0.0, 6.0, 22.0, -2.0, rgb, rest),
        ],
        pane_focused: vec![
            shadow(0.0, 1.0, 2.0, 0.0, rgb, lift),
            shadow(0.0, 6.0, 22.0, -2.0, rgb, lift),
        ],
        popover: vec![
            shadow(0.0, 1.0, 2.0, 0.0, rgb, lift),
            shadow(0.0, 8.0, 24.0, 0.0, rgb, lift),
        ],
        modal: vec![
            shadow(0.0, 2.0, 6.0, 0.0, rgb, lift),
            shadow(0.0, 18.0, 48.0, -4.0, rgb, lift),
        ],
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
                foreground: hex("#1d1d1f"),
                card: hex("#f4f4f6"),
                card_foreground: hex("#1d1d1f"),
                popover: hex("#ffffff"),
                popover_foreground: hex("#1d1d1f"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                secondary: hex("#f4f4f6"),
                secondary_foreground: hex("#48484a"),
                muted: hex("#f4f4f6"),
                muted_foreground: hex("#636366"),
                accent: hex("#efeff2"),
                accent_foreground: hex("#1d1d1f"),
                destructive: hex("#e8222d"),
                destructive_foreground: hex("#ffffff"),
                border: hex("#00000014"),
                input: hex("#d9d9de"),
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
                background: hex("#f6f6f8"),
                foreground: hex("#1d1d1f"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                accent: hex("#efeff2"),
                accent_foreground: hex("#1d1d1f"),
                border: hex("#00000014"),
                ring: hex("#6b6b70"),
            },
            // `--surface-chrome` for every window bar and dock.
            surface: SurfaceColors {
                toolbar: hex("#f6f6f8"),
                title_bar: hex("#f6f6f8"),
                status_bar: hex("#f6f6f8"),
                hover_strong: hex("#dcdce0"),
                sunken: hex("#e4e4e8"),
                action: hex("#1c1c1e"),
                action_foreground: hex("#ffffff"),
                action_hover: hex("#343436"),
                action_active: hex("#080809"),
            },
            // The floating layer: the Gorex-style pastel canvas, paper cards,
            // a translucent tab track and neutral alpha hover/pressed overlays.
            float: FloatColors {
                canvas_top: hex("#f6e7f6"),
                canvas_mid: hex("#efe1e6"),
                canvas_low: hex("#eee0d3"),
                canvas_bottom: hex("#ece1bd"),
                canvas_tint: rgba8([242, 214, 222], 0.35),
                pane: hex("#ffffff"),
                pane_focused: hex("#ffffff"),
                pane_border: hex("#ffffffb8"),
                pane_border_focused: hex("#ffffff"),
                track: hex("#ffffff57"),
                track_border: hex("#ffffff73"),
                tab_active: hex("#fffffff7"),
                tab_separator: hex("#00000021"),
                hover: hex("#0000000e"),
                pressed: hex("#0000001a"),
            },
            border: BorderVariants {
                variant: hex("#00000014"),  // --line
                focused: hex("#6b6b70"),    // --ring
                selected: hex("#0000001f"), // --line-strong
                transparent: transparent(),
                disabled: hex("#0000000d"), // --line-subtle
                strong: hex("#0000001f"),
                subtle: hex("#0000000d"),
            },
            status: StatusColors {
                modified: hex("#3a6ea5"),
                error: hex("#c0392b"),
                warning: hex("#b07a2a"),
                info: hex("#3a6ea5"),
                hint: hex("#6f6b5c"),
                success: hex("#5a7d3c"),
            },
            diff: DiffColors::default(),
            interaction: InteractionColors {
                cursor: hex("#3a3d42"),
                selection: hex("#007cf740"),
            },
            terminal: TerminalPalette {
                background: hex("#fbfbfb"),
                foreground: hex("#2a2d31"),
                bright_foreground: hex("#1d1d1f"),
                dim_foreground: hex("#6e7178"),
                normal: AnsiColors {
                    black: hex("#2a2d31"),
                    red: hex("#c2465a"),
                    green: hex("#4e8e5f"),
                    yellow: hex("#b07a1e"),
                    blue: hex("#2e6fd0"),
                    magenta: hex("#9050c8"),
                    cyan: hex("#1e8c9c"),
                    white: hex("#8a8d93"),
                },
                bright: AnsiColors {
                    black: hex("#6e7178"),
                    red: hex("#d9566b"),
                    green: hex("#5ba671"),
                    yellow: hex("#c89026"),
                    blue: hex("#4a8fe0"),
                    magenta: hex("#a56bd8"),
                    cyan: hex("#2ba2b3"),
                    white: hex("#b5b8be"),
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
            shadows: shadow_tiers(LIGHT_SHADOW, 0.07, 0.12),
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
                background: hex("#1e1e22"),
                foreground: hex("#f2f2f7"),
                card: hex("#26262b"),
                card_foreground: hex("#f2f2f7"),
                popover: hex("#26262b"),
                popover_foreground: hex("#ffffff"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                secondary: hex("#26262b"),
                secondary_foreground: hex("#d1d1d6"),
                muted: hex("#26262b"),
                muted_foreground: hex("#aeaeb2"),
                accent: hex("#303036"),
                accent_foreground: hex("#ffffff"),
                destructive: hex("#fd2f3b"),
                destructive_foreground: hex("#ffffff"),
                border: hex("#ffffff0f"),
                input: hex("#ffffff1a"),
                ring: hex("#98989d"),
                charts: [
                    hex("#3987e5"),
                    hex("#d95926"),
                    hex("#199e70"),
                    hex("#c98500"),
                    hex("#d55181"),
                ],
            },
            sidebar: SidebarColors {
                background: hex("#19191d"),
                foreground: hex("#f2f2f7"),
                primary: hex("#007cf7"),
                primary_foreground: hex("#ffffff"),
                accent: hex("#303036"),
                accent_foreground: hex("#f2f2f7"),
                border: hex("#ffffff0f"),
                ring: hex("#98989d"),
            },
            surface: SurfaceColors {
                toolbar: hex("#19191d"),
                title_bar: hex("#19191d"),
                status_bar: hex("#19191d"),
                hover_strong: hex("#46464c"),
                sunken: hex("#3a3a40"),
                action: hex("#f0f0f0"),
                action_foreground: hex("#141414"),
                action_hover: hex("#cdcdcd"),
                action_active: hex("#c8c8c8"),
            },
            float: FloatColors {
                canvas_top: hex("#2c2131"),
                canvas_mid: hex("#231e27"),
                canvas_low: hex("#221f22"),
                canvas_bottom: hex("#2a2619"),
                canvas_tint: rgba8([80, 40, 60], 0.25),
                pane: hex("#1e1e22"),
                pane_focused: hex("#1e1e22"),
                pane_border: hex("#ffffff0d"),
                pane_border_focused: hex("#ffffff24"),
                track: hex("#ffffff0f"),
                track_border: hex("#ffffff0f"),
                tab_active: hex("#ffffff24"),
                tab_separator: hex("#ffffff24"),
                hover: hex("#ffffff14"),
                pressed: hex("#ffffff24"),
            },
            border: BorderVariants {
                variant: hex("#ffffff0f"),
                focused: hex("#cdcdcd"), // --field-ring
                selected: hex("#ffffff1f"),
                transparent: transparent(),
                disabled: hex("#ffffff0a"),
                strong: hex("#ffffff1f"),
                subtle: hex("#ffffff0a"),
            },
            status: StatusColors {
                modified: hex("#6bc7d9"),
                error: hex("#fd2f3b"),
                warning: hex("#ff9412"),
                info: hex("#6bc7d9"),
                hint: hex("#6a6a6a"),
                success: hex("#31cf76"),
            },
            diff: DiffColors::default(),
            interaction: InteractionColors {
                cursor: hex("#ffffff"),
                selection: hex("#007cf740"),
            },
            terminal: TerminalPalette {
                background: hex("#1e1e22"),
                foreground: hex("#e6e6ea"),
                bright_foreground: hex("#ffffff"),
                dim_foreground: hex("#98989d"),
                normal: AnsiColors {
                    black: hex("#3a3a40"),
                    red: hex("#ff6b7f"),
                    green: hex("#7bd88f"),
                    yellow: hex("#e5c07b"),
                    blue: hex("#6cb2ff"),
                    magenta: hex("#c792ea"),
                    cyan: hex("#56d4dd"),
                    white: hex("#d0d0d6"),
                },
                bright: AnsiColors {
                    black: hex("#6e6e78"),
                    red: hex("#ff8c9c"),
                    green: hex("#9be8a8"),
                    yellow: hex("#f2d28a"),
                    blue: hex("#8ec5ff"),
                    magenta: hex("#d7a8f2"),
                    cyan: hex("#7fe3ea"),
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
            shadows: shadow_tiers(DARK_SHADOW, 0.25, 0.40),
            animation: Animation::default(),
            typography: Typography::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{oklch_a, to_rgb8};

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
    fn glass_transparency_scales_surfaces() {
        let at = |g| VibrancyLayer::Toolbar.opacity_in(SurfaceMaterial::Glassy(g));
        assert_eq!(at(0), 1.0);
        assert!((at(GLASS_TRANSPARENCY_DEFAULT) - VibrancyLayer::Toolbar.opacity()).abs() < 1e-6);
        assert!(at(100) < at(GLASS_TRANSPARENCY_DEFAULT) && at(100) >= 0.2);
        assert_eq!(
            VibrancyLayer::Divider.opacity_in(SurfaceMaterial::Glassy(100)),
            VibrancyLayer::Divider.opacity()
        );
    }

    #[test]
    fn background_image_makes_large_surfaces_more_transparent() {
        use SurfaceMaterial::*;
        for layer in [
            VibrancyLayer::Canvas,
            VibrancyLayer::Toolbar,
            VibrancyLayer::Sidebar,
            VibrancyLayer::Terminal,
        ] {
            assert!(
                layer.opacity_in(ImageClassic)
                    < layer.opacity_in(Glassy(GLASS_TRANSPARENCY_DEFAULT))
            );
            assert!(
                layer.opacity_in(ImageGlassy(GLASS_TRANSPARENCY_DEFAULT))
                    < layer.opacity_in(ImageClassic)
            );
            assert!(layer.opacity_in(ImageClassic) < 1.0);
        }
        assert_eq!(VibrancyLayer::Canvas.opacity_in(ImageClassic), 0.78);
        // Floating layers stay readable on any image.
        assert!(VibrancyLayer::Popover.opacity_in(ImageGlassy(GLASS_TRANSPARENCY_DEFAULT)) >= 0.9);
        // Opaque never changes a color; `bool` keeps the legacy meaning.
        let base = oklch_a(55.0, 0.03, 250.0, 1.0);
        assert_eq!(VibrancyLayer::Canvas.resolve(base, Opaque), base);
        assert_eq!(VibrancyLayer::Canvas.resolve(base, false), base);
        assert_eq!(
            VibrancyLayer::Canvas.resolve(base, true).a,
            VibrancyLayer::Canvas.opacity()
        );
        assert!(SurfaceMaterial::new(false, true, 30).has_image());
        assert!(!SurfaceMaterial::new(true, false, 30).has_image());
    }

    #[test]
    fn ciel_theme_defaults_use_the_ciel_radius_scale_and_soft_dark_shadows() {
        for theme in [Theme::light(), Theme::dark()] {
            assert_eq!(theme.radius, RadiusScale::ciel());
            // Every tier is a contact layer plus an ambient layer.
            for tier in [
                &theme.shadows.row,
                &theme.shadows.pane,
                &theme.shadows.pane_focused,
                &theme.shadows.popover,
                &theme.shadows.modal,
            ] {
                assert_eq!(tier.len(), 2);
            }
            // A focused card casts a stronger shadow than a resting one.
            assert!(theme.shadows.pane_focused[1].color.a > theme.shadows.pane[1].color.a);
        }

        let dark = Theme::dark();
        assert!(dark.shadows.row[0].color.a < 0.3);
        assert!(dark.shadows.popover[0].color.a <= 0.4);
    }

    #[test]
    fn float_layer_defaults_are_a_gradient_canvas_with_translucent_chrome() {
        for theme in [Theme::light(), Theme::dark()] {
            let f = theme.float;
            assert_ne!(f.canvas_top, f.canvas_bottom, "the canvas is a gradient");
            assert!(f.track.a < 1.0 && f.hover.a < 1.0 && f.pressed.a > f.hover.a);
            assert!(f.pane.a == 1.0, "cards are opaque so editor bodies match");
        }
    }

    #[test]
    fn shadow_stack_keeps_at_most_three_layers() {
        let s = Theme::light().shadows;
        let four = vec![s.pane[0]; 4];
        assert_eq!(ShadowStack::from_layers(&four).to_box_shadows().len(), 3);
        assert_eq!(ShadowStack::from_layers(&s.pane).to_box_shadows().len(), 2);
        assert!(ShadowStack::from_layers(&[]).to_box_shadows().is_empty());
    }

    #[test]
    fn both_variants_build() {
        let _ = Theme::light();
        let _ = Theme::dark();
    }

    #[test]
    fn dark_spot_checks_against_ciel_tokens() {
        let d = Theme::dark();
        // --accent #007cf7, --surface #1e1e22, --surface-chrome #19191d,
        // --surface-sunken #3a3a40, --destructive #fd2f3b
        assert!(close(to_rgb8(d.core.primary), [0x00, 0x7C, 0xF7], 1));
        assert!(close(to_rgb8(d.core.background), [0x1E, 0x1E, 0x22], 1));
        assert!(close(to_rgb8(d.surface.toolbar), [0x19, 0x19, 0x1D], 1));
        assert!(close(to_rgb8(d.surface.sunken), [0x3A, 0x3A, 0x40], 1));
        assert!(close(to_rgb8(d.core.destructive), [0xFD, 0x2F, 0x3B], 1));
    }

    #[test]
    fn light_spot_checks_against_ciel_tokens() {
        let l = Theme::light();
        // --surface #fff, --surface-chrome #f6f6f8, --surface-sunken #e4e4e8,
        // --action-surface #1c1c1e
        assert!(close(to_rgb8(l.core.background), [0xFF, 0xFF, 0xFF], 1));
        assert!(close(to_rgb8(l.surface.toolbar), [0xF6, 0xF6, 0xF8], 1));
        assert!(close(to_rgb8(l.surface.sunken), [0xE4, 0xE4, 0xE8], 1));
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
        assert_eq!(s.row.len(), 2);
        assert_eq!(s.popover.len(), 2);
        assert!((s.popover[0].color.a - 0.40).abs() < 1e-6);
        assert!((s.pane[1].color.a - 0.25).abs() < 1e-6);
        assert_eq!(s.pane[1].spread, -2.0);
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
