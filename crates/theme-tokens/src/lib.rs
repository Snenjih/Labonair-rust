//! Design tokens — the foundation-level visual contract (R08-008).
//!
//! The reference stylesheet supplies the base light/dark palette, transcribed
//! into typed Rust data ([`Theme`]) with Oklch colors converted to
//! [`gpui::Hsla`]. Labonair's native window materials and semantic opacity
//! layers are defined alongside those palette tokens.
//!
//! This crate was split out of `labonair-theme` so `labonair-ui-kit` can build
//! its primitives against the token contract ([`UiTheme`], [`Theme`],
//! [`RadiusScale`], [`ThemeMetrics`], [`IconThemeContent`]) without depending
//! on the Themes *feature* crate (the runtime `ThemeStore`, registries,
//! preview, and import stay in `labonair-theme`, which re-exports every type
//! here so downstream code is unaffected).

pub mod color;
pub mod contrast;
pub mod font_families;
pub mod icons;
pub mod metrics;
pub mod tokens;
pub mod ui_theme;

pub use color::{oklch, oklch_a, parse_color, to_hex, to_rgb8, transparent};
pub use contrast::{composite_over, contrast_ratio, relative_luminance};
pub use font_families::{MONO_FONT_FALLBACKS, MONO_FONT_FAMILY, UI_FONT_FALLBACKS, UI_FONT_FAMILY};
pub use icons::{
    ChevronIcons, DirectoryIcons, IconDefinition, IconThemeContent, BUILTIN_ICON_THEME_ID,
    BUILTIN_ICON_THEME_NAME, DEFAULT_FILE_ICONS, DEFAULT_FILE_STEMS, DEFAULT_FILE_SUFFIXES,
};
pub use metrics::{
    ActiveTheme, AnimationLevel, GlobalActiveTheme, PaneLayout, ThemeMetrics, UiDensity,
    PANE_GAP_COMFORTABLE,
};
pub use tokens::{
    Animation, AnsiColors, BorderVariants, CoreColors, CubicBezier, DiffColors, DiffSideColors,
    InteractionColors, MonoFontWeight, RadiusScale, ShadowLayer, Shadows, SidebarColors,
    StatusColors, SurfaceColors, SurfaceMaterial, TerminalPalette, Theme, Typography,
    VibrancyLayer, GLASS_TRANSPARENCY_DEFAULT, TAB_IN_FROM_SCALE,
};
pub use ui_theme::{ActiveThemeExt, UiTheme};
