//! Labonair theme system and design tokens.
//!
//! The single source of truth for the app's visual design is
//! `reference-src/src/styles/globals.css`. This crate transcribes every token
//! from that file into typed Rust data ([`Theme`]) with all Oklch colors
//! converted to [`gpui::Hsla`].
//!
//! Populated by Phase 01 (T02-*). T02-001 covers token extraction; later tasks
//! add the runtime theme provider/store and user import/export.

mod color;
pub mod contrast;
pub mod fonts;
pub mod icon_theme;
mod import;
mod prefs;
pub mod registry;
pub mod store;
pub mod theme_settings;
mod tokens;

pub use color::{oklch, oklch_a, parse_color, to_hex, to_rgb8, transparent};
pub use contrast::{composite_over, contrast_ratio, relative_luminance};
pub use fonts::{
    embedded_fonts, MONO_FONT_FALLBACKS, MONO_FONT_FAMILY, UI_FONT_FALLBACKS, UI_FONT_FAMILY,
};
pub use icon_theme::{
    ChevronIcons, DirectoryIcons, IconDefinition, IconThemeContent, IconThemeMeta,
    IconThemeNotFoundError, IconThemeRegistry, BUILTIN_ICON_THEME_ID, BUILTIN_ICON_THEME_NAME,
};
pub use import::{ThemeFile, ThemeFileVariant, COLOR_TOKENS};
pub use prefs::{EditorThemeId, ThemePreference};
pub use registry::{
    Appearance, ThemeFamilyContent, ThemeMeta, ThemeNotFoundError, ThemeRegistry,
    ThemeVariantContent, BUILTIN_FAMILY,
};
pub use store::{
    active_theme, init as init_theme, init_fonts, menu_metrics, modal_scrim, theme_store,
    FontOverrides, GlobalTheme, ThemeMode, ThemeStore, SCROLLBAR_SIZE,
};
pub use theme_settings::{ActiveTheme, GlobalActiveTheme, ThemeMetrics, UiDensity};
pub use tokens::{
    Animation, AnsiColors, BorderVariants, CoreColors, CubicBezier, InteractionColors,
    MonoFontWeight, RadiusScale, ShadowLayer, Shadows, SidebarColors, StatusColors, SurfaceColors,
    TerminalPalette, Theme, Typography, TAB_IN_FROM_SCALE,
};
