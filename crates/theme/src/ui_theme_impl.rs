//! `impl UiTheme for ThemeStore`.
//!
//! The [`UiTheme`](labonair_theme_tokens::UiTheme) contract is defined in
//! `labonair-theme-tokens` (below `ui-kit`). This impl lives here because of
//! the orphan rule: `ThemeStore` is local to this crate. Only `theme()` is
//! provided — the rest fall through to the trait defaults, which match
//! `ThemeStore`'s inherent methods 1:1.

use labonair_theme_tokens::{RadiusScale, Theme, ThemeMetrics, UiTheme};

use crate::store::ThemeStore;

impl UiTheme for ThemeStore {
    fn theme(&self) -> &Theme {
        ThemeStore::theme(self)
    }

    /// The metric-scaled radius (T20-007) — not the theme's own `radius`.
    fn radius(&self) -> RadiusScale {
        self.active_theme().radius()
    }

    fn metrics(&self) -> ThemeMetrics {
        self.active_theme().metrics().clone()
    }
}
