//! Minimal theme accessor for the ui-kit primitives.
//!
//! The primitives need a handful of design tokens (the active [`Theme`], its
//! `radius` scale, and two derived colours). They must **not** depend on the
//! runtime theme store (`ThemeStore`), which lives in `crates/ui` and carries
//! preference state, font resolution and GPUI globals.
//!
//! [`UiTheme`] is the thin contract in between: `crates/ui` implements it for
//! its `ThemeStore`, ui-kit only ever sees this trait. This mirrors Zed's split
//! between the `ui` and `theme` crates (see `docs/architecture.md` §2.1, §5).

use gpui::{App, Hsla};
use labonair_theme::{ActiveTheme, GlobalActiveTheme, RadiusScale, Theme, ThemeMetrics};

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
    /// command-palette results). The reference `cmdk` command items use
    /// `data-selected:bg-muted`.
    fn selected_fill(&self) -> Hsla {
        self.muted()
    }
}

/// Lets the ui-kit primitives read tokens off the runtime [`labonair_theme::ThemeStore`]
/// without that store depending on ui-kit. The impl lives here (not in
/// `crates/theme`) because of the orphan rule: `labonair-theme` must not depend
/// on `labonair-ui-kit`, and `UiTheme` is defined in this crate. Only `theme()`
/// is provided — the rest fall through to the trait defaults, which match
/// `ThemeStore`'s inherent methods of the same name 1:1.
impl UiTheme for labonair_theme::ThemeStore {
    fn theme(&self) -> &Theme {
        labonair_theme::ThemeStore::theme(self)
    }

    /// The metric-scaled radius (T20-007) — not the theme's own `radius`.
    fn radius(&self) -> RadiusScale {
        self.active_theme().radius()
    }

    fn metrics(&self) -> ThemeMetrics {
        self.active_theme().metrics().clone()
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
