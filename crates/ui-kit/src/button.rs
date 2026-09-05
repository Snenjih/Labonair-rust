//! Shared `Button` primitive.
//!
//! 1:1 port of `reference-src/src/components/ui/button.tsx` (`buttonVariants`
//! cva): six variants, eight sizes, pill radius (`rounded-4xl` ==
//! `radius.xl4`), transparent border, `disabled:opacity-50`. Replaces the
//! ad-hoc `btn` / `tool_btn` / `step_btn` helpers scattered across the views.

use gpui::{div, px, Div, InteractiveElement, Stateful, StyleRefinement, Styled};

use crate::palette::Palette;

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
/// `btn`-helper call sites. Comes with the per-variant hover style baked in; use
/// [`button_no_hover`] when the call site sets its own `.hover(..)`.
pub fn button(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    button_no_hover(id, c, variant, size).hover(variant_hover(variant, c))
}

/// Same geometry and variant paint as [`button`] but without the baked-in hover
/// style, for the toolbar call sites that apply their own `.hover(..)` (calling
/// `.hover()` twice panics with "hover style already set" in debug builds).
pub fn button_no_hover(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    let radius = c.radius.xl4;
    let h = size.height();

    let mut el = div()
        .id(id)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(c.space(6.0))
        .h(c.space(h))
        .rounded(px(radius))
        .border_1()
        .border_color(gpui::transparent_black())
        .text_size(px(size.text()))
        .cursor_pointer();

    el = match size.px() {
        Some(p) => el.px(c.space(p)),
        None => el.w(c.space(h)),
    };

    apply_variant(el, variant, c)
}

fn apply_variant(el: Stateful<Div>, variant: ButtonVariant, c: Palette) -> Stateful<Div> {
    match variant {
        ButtonVariant::Default => el.bg(c.primary).text_color(c.primary_fg),
        ButtonVariant::Outline => el.border_color(c.border).bg(c.bg).text_color(c.fg),
        ButtonVariant::Secondary => el.bg(c.secondary).text_color(c.secondary_fg),
        ButtonVariant::Ghost => el.text_color(c.fg),
        ButtonVariant::Destructive => el.bg(c.destructive.opacity(0.1)).text_color(c.destructive),
        ButtonVariant::Link => el.text_color(c.primary),
    }
}

/// The per-variant hover style baked into [`button`] — see `button.tsx` cva.
fn variant_hover(
    variant: ButtonVariant,
    c: Palette,
) -> impl Fn(StyleRefinement) -> StyleRefinement {
    move |s| match variant {
        ButtonVariant::Default => s.bg(c.primary.opacity(0.8)),
        ButtonVariant::Outline | ButtonVariant::Ghost => s.bg(c.muted_bg),
        ButtonVariant::Secondary => s.bg(c.secondary.opacity(0.8)),
        ButtonVariant::Destructive => s.bg(c.destructive.opacity(0.2)),
        ButtonVariant::Link => s.underline(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_every_variant_and_size() {
        let c = test_palette();
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
                let _ = button("btn", c, v, s);
            }
        }
    }

    #[test]
    fn pill_radius_matches_reference_radius_4xl() {
        assert!((test_palette().radius.xl4 - 13.0).abs() < 1e-6);
    }
}
