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

use gpui::Hsla;
use labonair_theme::{RadiusScale, Theme};

/// Token accessor the ui-kit primitives build against.
///
/// Only `theme()` must be provided; the rest are 1:1 derivations kept as
/// defaulted methods so implementors (and call sites) stay unchanged.
pub trait UiTheme {
    /// The currently active theme tokens.
    fn theme(&self) -> &Theme;

    /// The active radius scale (`--radius` family).
    fn radius(&self) -> RadiusScale {
        self.theme().radius
    }

    /// `--muted-foreground`.
    fn muted_foreground(&self) -> Hsla {
        self.theme().core.muted_foreground
    }

    /// `--border`.
    fn border(&self) -> Hsla {
        self.theme().core.border
    }
}
