//! Labonair UI components and theme provider.
//!
//! Populated by later phases (T04+). T02-002 adds the runtime theme provider.

pub mod theme;

pub use labonair_theme::{ThemeFile, ThemeFileVariant};
pub use theme::{
    active_theme, init as init_theme, init_fonts, theme_store, GlobalTheme, ThemeMode,
    ThemePreference, ThemeStore,
};
