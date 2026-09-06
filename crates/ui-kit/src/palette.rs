//! The token bundle the ui-kit primitives are styled from.
//!
//! Rendering a primitive from inside a view's `render` usually cannot hold a
//! `&impl UiTheme` borrow: `theme.read(cx)` borrows `cx` immutably, while
//! `cx.listener(..)` for the primitive's `on_click` needs `&mut Context`. Every
//! view therefore already snapshots the handful of tokens it needs into a local
//! struct before building elements (`settings-ui`'s `Palette`,
//! `panel-explorer`'s `Colors`, …).
//!
//! [`Palette`] is the shared version of that snapshot: `Copy`, built once per
//! render via [`Palette::from_theme`], and the single parameter the primitives
//! with more than ~3 colour needs take. This is what keeps every primitive
//! token-bound (Critical Rule 3) without forcing the borrow gymnastics — a call
//! site literally cannot pass a hardcoded colour without writing one itself.

use gpui::{px, Hsla, Pixels};
use labonair_theme::RadiusScale;

use crate::theme::UiTheme;

/// A per-render snapshot of the design tokens the primitives read.
///
/// ```ignore
/// let c = Palette::from_theme(self.theme.read(cx));
/// div().child(checkbox("cb", c, checked).on_click(cx.listener(..)))
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    /// `--background`.
    pub bg: Hsla,
    /// `--foreground`.
    pub fg: Hsla,
    /// `--muted-foreground` (the "secondary text" colour).
    pub muted: Hsla,
    /// `--border`.
    pub border: Hsla,
    /// `--muted` (the muted *surface*, not the muted text colour).
    pub muted_bg: Hsla,
    /// `--input` (the resting fill of form controls).
    pub input: Hsla,
    /// `--ring` (the focus ring).
    pub ring: Hsla,
    /// `--card` (raised surfaces: cards, panels).
    pub card: Hsla,
    /// `--card-foreground`.
    pub card_fg: Hsla,
    /// `--popover` (menus, dropdowns, floating cards).
    pub popover: Hsla,
    /// `--popover-foreground`.
    pub popover_fg: Hsla,
    /// `--sidebar` (navigation rails: settings sidebar, dock panels).
    pub sidebar: Hsla,
    /// `--sidebar-foreground`.
    pub sidebar_fg: Hsla,
    /// `--sidebar-border`.
    pub sidebar_border: Hsla,
    /// `--accent` (hover/selected fill).
    pub accent: Hsla,
    /// `--accent-foreground`.
    pub accent_fg: Hsla,
    /// `--primary`.
    pub primary: Hsla,
    /// `--primary-foreground`.
    pub primary_fg: Hsla,
    /// `--secondary`.
    pub secondary: Hsla,
    /// `--secondary-foreground`.
    pub secondary_fg: Hsla,
    /// `--destructive`.
    pub destructive: Hsla,
    /// Status `error`.
    pub error: Hsla,
    /// Status `warning`.
    pub warning: Hsla,
    /// Status `info`.
    pub info: Hsla,
    /// Status `success`.
    pub success: Hsla,
    /// The active `--radius` family — already scaled by the T20-007
    /// `corner_radius_scale` metric (via [`UiTheme::radius`]).
    pub radius: RadiusScale,
    /// The T20-007 UI-density spacing multiplier (`0.85` / `1.0` / `1.15`).
    /// Feed spacing/size literals through [`Palette::space`] rather than
    /// multiplying by hand.
    pub density: f32,
}

impl Palette {
    /// Snapshot the tokens off any [`UiTheme`] (the runtime `ThemeStore`, or a
    /// bare `Theme` in tests).
    pub fn from_theme(theme: &impl UiTheme) -> Self {
        let core = &theme.theme().core;
        let status = &theme.theme().status;
        Self {
            bg: core.background,
            fg: core.foreground,
            muted: core.muted_foreground,
            border: core.border,
            muted_bg: core.muted,
            input: core.input,
            ring: core.ring,
            card: core.card,
            card_fg: core.card_foreground,
            popover: core.popover,
            popover_fg: core.popover_foreground,
            sidebar: theme.theme().sidebar.background,
            sidebar_fg: theme.theme().sidebar.foreground,
            sidebar_border: theme.theme().sidebar.border,
            accent: core.accent,
            accent_fg: core.accent_foreground,
            primary: core.primary,
            primary_fg: core.primary_foreground,
            secondary: core.secondary,
            secondary_fg: core.secondary_foreground,
            destructive: core.destructive,
            error: status.error,
            warning: status.warning,
            info: status.info,
            success: status.success,
            radius: theme.radius(),
            density: theme.metrics().density.spacing_scale(),
        }
    }

    /// A spacing/size literal (px) scaled by the active UI density (T20-007).
    /// Primitives use this for every padding / gap / height / fixed layout
    /// dimension instead of a bare `px(..)`. Hairlines (1px borders/dividers),
    /// icon glyph sizes and `text_size` are *typographic*, not spacing, and
    /// stay as literals (`docs/architecture.md` §8.20).
    pub fn space(&self, value: f32) -> Pixels {
        px(value * self.density)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_theme::Theme;

    struct TestTheme(Theme);
    impl UiTheme for TestTheme {
        fn theme(&self) -> &Theme {
            &self.0
        }
    }

    #[test]
    fn snapshot_matches_the_theme_tokens() {
        let theme = TestTheme(Theme::dark());
        let c = Palette::from_theme(&theme);
        assert_eq!(c.fg, theme.0.core.foreground);
        assert_eq!(c.border, theme.0.core.border);
        assert_eq!(c.warning, theme.0.status.warning);
        assert_eq!(c.radius.sm, theme.0.radius.sm);
    }

    #[test]
    fn space_scales_with_density() {
        use gpui::px;
        let mut c = Palette::from_theme(&TestTheme(Theme::dark()));
        assert_eq!(c.density, 1.0);
        assert_eq!(c.space(40.0), px(40.0));
        c.density = 0.85;
        assert_eq!(c.space(40.0), px(34.0));
        c.density = 1.15;
        assert_eq!(c.space(40.0), px(46.0));
    }

    #[test]
    fn light_and_dark_differ() {
        assert_ne!(
            Palette::from_theme(&TestTheme(Theme::dark())),
            Palette::from_theme(&TestTheme(Theme::light()))
        );
    }
}
