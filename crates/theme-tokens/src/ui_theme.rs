//! The token-accessor contract the `labonair-ui-kit` primitives build against.
//!
//! Moved out of `labonair-ui-kit::theme` in R08-008. `ui-kit` only ever sees
//! [`UiTheme`]; `labonair-theme` implements it for its runtime `ThemeStore`
//! (that `impl` lives in `labonair-theme` because of the orphan rule). This
//! mirrors Zed's split between the `ui` and `theme` crates.

use gpui::{App, Hsla};

use crate::metrics::{ActiveTheme, GlobalActiveTheme, ThemeMetrics};
use crate::tokens::{RadiusScale, Theme};

/// Token accessor the ui-kit primitives build against.
///
/// Only `theme()` must be provided; the rest are 1:1 derivations kept as
/// defaulted methods so implementors (and call sites) stay unchanged.
pub trait UiTheme {
    /// The currently active theme tokens.
    fn theme(&self) -> &Theme;

    /// The active radius scale (`--radius` family), **after** the T20-007
    /// `corner_radius_scale` metric. The runtime `ThemeStore` overrides this to
    /// return the scaled value; a bare `Theme` (tests) has no metric layer, so
    /// the default is the theme's own unscaled scale.
    fn radius(&self) -> RadiusScale {
        self.theme().radius
    }

    /// The active metric layer (T20-007: font scales, UI density, corner-radius
    /// scale, reduce-motion). Defaults to [`ThemeMetrics::default`] for a bare
    /// `Theme`; the runtime `ThemeStore` overrides it with the live metrics.
    fn metrics(&self) -> ThemeMetrics {
        ThemeMetrics::default()
    }

    /// `--muted-foreground`.
    fn muted_foreground(&self) -> Hsla {
        self.theme().core.muted_foreground
    }

    /// `--border`.
    fn border(&self) -> Hsla {
        self.theme().core.border
    }

    /// `--foreground`.
    fn foreground(&self) -> Hsla {
        self.theme().core.foreground
    }

    /// `--card`.
    fn card(&self) -> Hsla {
        self.theme().core.card
    }

    /// `--muted`.
    fn muted(&self) -> Hsla {
        self.theme().core.muted
    }

    /// `--primary`.
    fn primary(&self) -> Hsla {
        self.theme().core.primary
    }

    /// `--primary-foreground`.
    fn primary_foreground(&self) -> Hsla {
        self.theme().core.primary_foreground
    }

    /// `--background`.
    fn background(&self) -> Hsla {
        self.theme().core.background
    }

    /// `--accent`.
    fn accent(&self) -> Hsla {
        self.theme().core.accent
    }

    /// `--accent-foreground`.
    fn accent_foreground(&self) -> Hsla {
        self.theme().core.accent_foreground
    }

    /// `--destructive`.
    fn destructive(&self) -> Hsla {
        self.theme().core.destructive
    }

    /// `--success` status color (severity: success).
    fn status_success(&self) -> Hsla {
        self.theme().status.success
    }

    /// `--destructive` / status `error` color (severity: error).
    fn status_error(&self) -> Hsla {
        self.theme().status.error
    }

    /// `--warning` status color (severity: warning).
    fn status_warning(&self) -> Hsla {
        self.theme().status.warning
    }

    /// `--info` status color (severity: info).
    fn status_info(&self) -> Hsla {
        self.theme().status.info
    }

    /// Canonical selected/active fill for list selection (Explorer rows,
    /// command-palette results, active tab, active nav item) — the primary
    /// colour at a low alpha; hover stays neutral.
    fn selected_fill(&self) -> Hsla {
        self.primary().opacity(0.16)
    }

    /// The solid primary marker that accompanies [`Self::selected_fill`].
    fn selected_accent(&self) -> Hsla {
        self.primary()
    }
}

/// `cx.active_theme()` — the colour+metric [`ActiveTheme`] from the
/// [`GlobalActiveTheme`] global (installed by `labonair_theme::init_theme`).
/// This is the T20-007 read path for `App`-level code that isn't holding the
/// `ThemeStore` entity. Panics if the global was never installed (same
/// contract as `labonair_theme::active_theme`).
pub trait ActiveThemeExt {
    fn active_theme(&self) -> &ActiveTheme;
}

impl ActiveThemeExt for App {
    fn active_theme(&self) -> &ActiveTheme {
        &self.global::<GlobalActiveTheme>().0
    }
}
