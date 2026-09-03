//! Shared `Button` primitive.
//!
//! 1:1 port of `reference-src/src/components/ui/button.tsx` (`buttonVariants`
//! cva): six variants, eight sizes, pill radius (`rounded-4xl` ==
//! `radius.xl4`), transparent border, `disabled:opacity-50`. Replaces the
//! ad-hoc `btn` / `tool_btn` / `step_btn` helpers scattered across the views.

use gpui::{div, px, Div, InteractiveElement, Stateful, Styled};
use labonair_theme::CoreColors;

use crate::theme::UiTheme;

/// `disabled:opacity-50` from the cva base.
pub const DISABLED_OPACITY: f32 = 0.5;

/// cva `variant` — see `button.tsx:12-24`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

/// cva `size` — see `button.tsx:25-34`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonSize {
    #[default]
    Default,
    Xs,
    Sm,
    Lg,
    Icon,
    IconXs,
    IconSm,
    IconLg,
}

impl ButtonSize {
    /// Height in px (`h-9` / `h-6` / `h-8` / `h-10` / `size-*`).
    fn height(self) -> f32 {
        match self {
            ButtonSize::Default | ButtonSize::Icon => 36.0,
            ButtonSize::Xs | ButtonSize::IconXs => 24.0,
            ButtonSize::Sm | ButtonSize::IconSm => 32.0,
            ButtonSize::Lg | ButtonSize::IconLg => 40.0,
        }
    }

    /// Horizontal padding in px (`px-3` / `px-2.5` / `px-4`); `None` for the
    /// square icon sizes (`size-*`, width == height).
    fn px(self) -> Option<f32> {
        match self {
            ButtonSize::Default | ButtonSize::Sm => Some(12.0),
            ButtonSize::Xs => Some(10.0),
            ButtonSize::Lg => Some(16.0),
            ButtonSize::Icon | ButtonSize::IconXs | ButtonSize::IconSm | ButtonSize::IconLg => None,
        }
    }

    /// Font size in px (`text-sm` default, `text-xs` for `xs`).
    fn text(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs => 12.0,
            _ => 14.0,
        }
    }
}

/// Builds the base, pre-styled button element. Callers add `.child(..)` for the
/// label/icon and `.on_click(..)` for the handler, mirroring the existing
/// `btn`-helper call sites.
pub fn button(
    id: impl Into<gpui::ElementId>,
    theme: &impl UiTheme,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    let core = theme.theme().core.clone();
    let radius = theme.radius().xl4;
    let h = size.height();

    let mut el = div()
        .id(id)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .h(px(h))
        .rounded(px(radius))
        .border_1()
        .border_color(gpui::transparent_black())
        .text_size(px(size.text()))
        .cursor_pointer();

    el = match size.px() {
        Some(p) => el.px(px(p)),
        None => el.w(px(h)),
    };

    apply_variant(el, variant, core)
}

fn apply_variant(el: Stateful<Div>, variant: ButtonVariant, c: CoreColors) -> Stateful<Div> {
    match variant {
        ButtonVariant::Default => el
            .bg(c.primary)
            .text_color(c.primary_foreground)
            .hover(move |s| s.bg(c.primary.opacity(0.8))),
        ButtonVariant::Outline => el
            .border_color(c.border)
            .bg(c.background)
            .text_color(c.foreground)
            .hover(move |s| s.bg(c.muted)),
        ButtonVariant::Secondary => el
            .bg(c.secondary)
            .text_color(c.secondary_foreground)
            .hover(move |s| s.bg(c.secondary.opacity(0.8))),
        ButtonVariant::Ghost => el.text_color(c.foreground).hover(move |s| s.bg(c.muted)),
        ButtonVariant::Destructive => el
            .bg(c.destructive.opacity(0.1))
            .text_color(c.destructive)
            .hover(move |s| s.bg(c.destructive.opacity(0.2))),
        ButtonVariant::Link => el.text_color(c.primary).hover(|s| s.underline()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_theme::Theme;

    /// Bare [`UiTheme`] impl over a fixed [`Theme`] — the primitives only read
    /// tokens, so no runtime store is needed in these unit tests.
    struct TestTheme(Theme);
    impl UiTheme for TestTheme {
        fn theme(&self) -> &Theme {
            &self.0
        }
    }

    #[test]
    fn builds_every_variant_and_size() {
        let theme = TestTheme(Theme::dark());
        for v in [
            ButtonVariant::Default,
            ButtonVariant::Outline,
            ButtonVariant::Secondary,
            ButtonVariant::Ghost,
            ButtonVariant::Destructive,
            ButtonVariant::Link,
        ] {
            for s in [
                ButtonSize::Default,
                ButtonSize::Xs,
                ButtonSize::Sm,
                ButtonSize::Lg,
                ButtonSize::Icon,
                ButtonSize::IconXs,
                ButtonSize::IconSm,
                ButtonSize::IconLg,
            ] {
                // Smoke test: the builder must not panic for any combination.
                let _ = button("btn", &theme, v, s);
            }
        }
    }

    #[test]
    fn pill_radius_matches_reference_radius_4xl() {
        let theme = TestTheme(Theme::dark());
        assert!((theme.radius().xl4 - 13.0).abs() < 1e-6);
    }
}
