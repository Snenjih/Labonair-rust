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

use gpui::{point, px, BoxShadow, Hsla, Pixels};
use labonair_theme_tokens::{RadiusScale, VibrancyLayer};

use crate::animation::MotionPrefs;
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
    /// `--toolbar-background` (the titlebar and inactive tab-strip surface).
    pub toolbar_bg: Hsla,
    /// `--foreground`.
    pub fg: Hsla,
    /// `--muted-foreground` (the "secondary text" colour).
    pub muted: Hsla,
    /// `--border`.
    pub border: Hsla,
    /// `--muted` (the muted *surface*, not the muted text colour).
    pub muted_bg: Hsla,
    /// `--input` (the border of form controls; see [`Palette::field_bg`] for
    /// their fill).
    pub input: Hsla,
    /// `--field-bg`: the resting fill of form controls (the surface in light
    /// themes, the muted surface in dark ones).
    pub field_bg: Hsla,
    /// `--field-ring`: the focus border of form controls.
    pub field_ring: Hsla,
    /// Whether the active theme is dark (picks shadow strength).
    pub is_dark: bool,
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
    /// `--accent` (neutral hover fill).
    pub accent: Hsla,
    /// `--accent-foreground`.
    pub accent_fg: Hsla,
    /// `--surface-hover-strong`: hover on an already-hovered fill.
    pub hover_strong: Hsla,
    /// `--surface-sunken`: the one selected/active fill (Ciel §8), always paired
    /// with a semibold label — never an accent tint.
    pub sunken: Hsla,
    /// `--line-strong`: popover/dialog hairlines, active segment rings.
    pub line_strong: Hsla,
    /// `--line-subtle`: row separators inside a filled group.
    pub line_subtle: Hsla,
    /// `--action-surface`: the neutral high-contrast primary-action fill.
    pub action: Hsla,
    /// `--action-foreground`.
    pub action_fg: Hsla,
    /// `--action-hover`.
    pub action_hover: Hsla,
    /// `--action-active`.
    pub action_active: Hsla,
    /// `--scrim`: dialog/overlay backdrop.
    pub scrim: Hsla,
    /// Canonical selected/active fill — alias of [`Palette::sunken`].
    pub selected_fill: Hsla,
    /// The accent (`--accent` blue) marker for genuine "active/changed" state
    /// (toggled icons, match highlights) — never a row selection.
    pub selected_accent: Hsla,
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
    /// UI-font rem scale used by controls whose dimensions track text size.
    pub ui_font_scale: f32,
    /// Motion tokens: the active [`crate::animation::MotionLevel`], the theme durations and
    /// easing curves (see [`crate::animation`]).
    pub motion: MotionPrefs,
}

impl Palette {
    /// Snapshot the tokens off any [`UiTheme`] (the runtime `ThemeStore`, or a
    /// bare `Theme` in tests).
    pub fn from_theme(theme: &impl UiTheme) -> Self {
        let t = theme.theme();
        let core = &t.core;
        let status = &t.status;
        let translucent = theme.surface_material();
        Self {
            bg: VibrancyLayer::Canvas.resolve(core.background, translucent),
            toolbar_bg: VibrancyLayer::Toolbar.resolve(t.surface.toolbar, translucent),
            fg: core.foreground,
            muted: core.muted_foreground,
            border: VibrancyLayer::Divider.resolve(core.border, translucent),
            muted_bg: core.muted,
            input: core.input,
            field_bg: VibrancyLayer::Control.resolve(
                if t.is_dark {
                    core.muted
                } else {
                    core.background
                },
                translucent,
            ),
            field_ring: t.border.focused,
            is_dark: t.is_dark,
            ring: core.ring,
            card: VibrancyLayer::Card.resolve(core.card, translucent),
            card_fg: core.card_foreground,
            popover: VibrancyLayer::Popover.resolve(core.popover, translucent),
            popover_fg: core.popover_foreground,
            sidebar: VibrancyLayer::Sidebar.resolve(t.sidebar.background, translucent),
            sidebar_fg: t.sidebar.foreground,
            sidebar_border: VibrancyLayer::Divider.resolve(t.sidebar.border, translucent),
            accent: core.accent,
            accent_fg: core.accent_foreground,
            hover_strong: t.surface.hover_strong,
            sunken: t.surface.sunken,
            line_strong: t.border.strong,
            line_subtle: t.border.subtle,
            action: t.surface.action,
            action_fg: t.surface.action_foreground,
            action_hover: t.surface.action_hover,
            action_active: t.surface.action_active,
            scrim: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: if t.is_dark { 0.55 } else { 0.30 },
            },
            selected_fill: t.surface.sunken,
            selected_accent: core.primary,
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
            ui_font_scale: theme.metrics().ui_font_size / 16.0,
            motion: MotionPrefs::from_theme(theme),
        }
    }

    /// The floating-layer shadow (`--shadow-popover`): menus, dropdowns,
    /// tooltips and toasts. Dialogs use a `line_strong` border instead.
    pub fn popover_shadow(&self) -> Vec<BoxShadow> {
        let (a, b) = if self.is_dark {
            (0.26, 0.18)
        } else {
            (0.12, 0.06)
        };
        let black = |alpha: f32| Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: alpha,
        };
        vec![
            BoxShadow {
                color: black(a),
                offset: point(px(0.0), px(12.0)),
                blur_radius: px(32.0),
                spread_radius: px(0.0),
            },
            BoxShadow {
                color: black(b),
                offset: point(px(0.0), px(1.0)),
                blur_radius: px(4.0),
                spread_radius: px(0.0),
            },
        ]
    }

    /// A spacing/size literal (px) scaled by the active UI density (T20-007).
    /// Primitives use this for every padding / gap / height / fixed layout
    /// dimension instead of a bare `px(..)`. Hairlines (1px borders/dividers),
    /// icon glyph sizes and `text_size` are *typographic*, not spacing, and
    /// stay as literals (`docs/architecture.md` §8.20).
    pub fn space(&self, value: f32) -> Pixels {
        px(value * self.density)
    }

    /// A control dimension scaled by both the UI density and the active UI
    /// font size, matching rem-sized native controls.
    pub fn control_space(&self, value: f32) -> Pixels {
        px(value * self.density * self.ui_font_scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_theme_tokens::Theme;

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
        // Opaque by default: the theme colors pass through untouched.
        assert_eq!(c.toolbar_bg, theme.0.surface.toolbar);
        assert_eq!(c.border, theme.0.core.border);
        assert_eq!(c.sunken, theme.0.surface.sunken);
        assert_eq!(c.selected_fill, c.sunken);
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
    fn control_space_scales_with_density_and_ui_font_size() {
        use gpui::px;
        let mut c = Palette::from_theme(&TestTheme(Theme::dark()));
        c.density = 0.85;
        c.ui_font_scale = 1.25;
        assert_eq!(c.control_space(32.0), px(34.0));
    }

    #[test]
    fn light_and_dark_differ() {
        assert_ne!(
            Palette::from_theme(&TestTheme(Theme::dark())),
            Palette::from_theme(&TestTheme(Theme::light()))
        );
    }
}
