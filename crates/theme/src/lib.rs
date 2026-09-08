//! Labonair theme system.
//!
//! The design *tokens* (`Theme`, `RadiusScale`, `ThemeMetrics`, `ActiveTheme`,
//! colour structs, `IconThemeContent`, the `UiTheme` contrast/contract) live
//! in the foundation-level [`labonair_theme_tokens`] crate and are re-exported
//! here for compatibility. This crate owns the *runtime*: the `ThemeStore`
//! entity, the built-in colour/icon registries, preview/activation, fonts, and
//! user import/export.

pub mod command_provider;
pub mod fonts;
pub mod icon_theme;
mod import;
mod prefs;
pub mod registry;
pub mod store;
mod ui_theme_impl;

pub use labonair_theme_tokens::{
    composite_over, contrast_ratio, oklch, oklch_a, parse_color, relative_luminance, to_hex,
    to_rgb8, transparent, ActiveTheme, ActiveThemeExt, Animation, AnsiColors, BorderVariants,
    ChevronIcons, CoreColors, CubicBezier, DirectoryIcons, GlobalActiveTheme, IconDefinition,
    IconThemeContent, InteractionColors, MonoFontWeight, RadiusScale, ShadowLayer, Shadows,
    SidebarColors, StatusColors, SurfaceColors, TerminalPalette, Theme, ThemeMetrics, Typography,
    UiDensity, UiTheme, BUILTIN_ICON_THEME_ID, BUILTIN_ICON_THEME_NAME, MONO_FONT_FALLBACKS,
    MONO_FONT_FAMILY, TAB_IN_FROM_SCALE, UI_FONT_FALLBACKS, UI_FONT_FAMILY,
};

/// The `contrast` module path is kept for callers that used
/// `labonair_theme::contrast::…`.
pub mod contrast {
    pub use labonair_theme_tokens::contrast::*;
}

pub use fonts::{embedded_fonts, list_system_fonts};
pub use icon_theme::{IconThemeMeta, IconThemeNotFoundError, IconThemeRegistry};
pub use import::{ThemeFile, ThemeFileConversion, ThemeFileVariant, COLOR_TOKENS};
pub use prefs::{EditorThemeId, ThemePreference};
pub use registry::{
    Appearance, ThemeFamilyContent, ThemeMeta, ThemeNotFoundError, ThemeRegistry,
    ThemeVariantContent, BUILTIN_FAMILY,
};
pub use store::{
    active_theme, init as init_theme, init_fonts, menu_metrics, modal_scrim, theme_store,
    FontOverrides, GlobalTheme, ThemeMode, ThemeStore, SCROLLBAR_SIZE,
};
