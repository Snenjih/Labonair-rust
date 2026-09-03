//! Re-export shim for the runtime theme provider.
//!
//! The `ThemeStore` / `ThemeMode` / `FontOverrides` / `GlobalTheme` runtime
//! provider, its `init` / `init_fonts` / `theme_store` / `active_theme`
//! helpers, `modal_scrim`, `SCROLLBAR_SIZE` and `menu_metrics` moved to
//! `labonair_theme::store` in T16-006 (so `labonair-workspace` can name
//! `ThemeStore` concretely without depending on `crates/ui`). Every existing
//! `crate::theme::…` path in `crates/ui` keeps resolving through this glob.
//! `EditorThemeId` / `ThemePreference` continue to live in `labonair_theme`
//! (T16-004) and are re-exported by `store` too.

pub use labonair_theme::store::*;
