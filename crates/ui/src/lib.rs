//! Labonair UI components and theme provider.
//!
//! Populated by later phases (T04+). T02-002 adds the runtime theme provider.

pub mod background;
pub mod tabs;
pub mod terminal;
pub mod theme;
pub mod workspace;

pub use background::{
    background_store, init as init_background, BackgroundFit, BackgroundStore, BackgroundTarget,
    GlobalBackground, LayerScope,
};
pub use labonair_theme::{ThemeFile, ThemeFileVariant};
pub use tabs::{Tab, TabData, TabKind, TabStore};
pub use terminal::TerminalView;
pub use theme::{
    active_theme, init as init_theme, init_fonts, theme_store, GlobalTheme, ThemeMode,
    ThemePreference, ThemeStore,
};
pub use workspace::Workspace;
