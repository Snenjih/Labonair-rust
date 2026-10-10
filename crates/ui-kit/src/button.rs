//! Shared button primitive.
//!
//! Buttons use one compact geometry scale, four corner radii, and explicit
//! surface treatments. Feature crates choose an appearance and size; the
//! shared primitive supplies hover, pressed, focused, and disabled
//! states.

use gpui::{
    div, prelude::FluentBuilder, px, transparent_black, Div, InteractiveElement, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled,
};

use crate::palette::Palette;

/// Opacity used by small disabled controls that do not have a button surface.
pub const DISABLED_OPACITY: f32 = 0.5;

/// Surface treatment for a button.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    /// Transparent at rest, with a neutral hover and pressed surface.
    #[default]
    Subtle,
    /// A raised surface for actions that need more emphasis.
    Filled,
    /// A visible border around the normal surface.
    Outlined,
    /// A visible border around a transparent surface.
    OutlinedGhost,
    /// Error-tinted surface for destructive actions.
    TintedError,
    /// Foreground-only action with an underline on hover.
    Link,
    /// Transparent foreground-only control.
    Transparent,
}

/// Shared button heights. Values use the app font scale and UI density.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonSize {
    /// 22 logical pixels at the reference UI font size.
    #[default]
    Default,
    /// 18 logical pixels at the reference UI font size.
    Xs,
    /// 28 logical pixels at the reference UI font size.
    Sm,
    /// 32 logical pixels at the reference UI font size.
    Lg,
    /// 16 logical pixels at the reference UI font size.
    None,
    Icon,
    IconXs,
    IconSm,
    IconLg,
    IconNone,
}

impl ButtonSize {
    fn height(self) -> f32 {
        match self {
            ButtonSize::Default | ButtonSize::Icon => 22.0,
            ButtonSize::Xs | ButtonSize::IconXs => 18.0,
            ButtonSize::Sm | ButtonSize::IconSm => 28.0,
            ButtonSize::Lg | ButtonSize::IconLg => 32.0,
            ButtonSize::None | ButtonSize::IconNone => 16.0,
        }
    }

    fn horizontal_padding(self) -> f32 {
        match self {
            ButtonSize::Default | ButtonSize::Xs => 4.0,
            ButtonSize::Sm | ButtonSize::Lg => 8.0,
            ButtonSize::None => 0.0,
            ButtonSize::Icon
            | ButtonSize::IconXs
            | ButtonSize::IconSm
            | ButtonSize::IconLg
            | ButtonSize::IconNone => 0.0,
        }
    }

    fn label_size(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs | ButtonSize::None | ButtonSize::IconNone => 11.0,
            ButtonSize::Default | ButtonSize::Sm | ButtonSize::Icon | ButtonSize::IconSm => 13.0,
            ButtonSize::Lg | ButtonSize::IconLg => 14.0,
        }
    }

    pub(crate) fn icon_size(self) -> f32 {
        match self {
            ButtonSize::Xs | ButtonSize::IconXs | ButtonSize::None | ButtonSize::IconNone => 12.0,
            ButtonSize::Default | ButtonSize::Sm | ButtonSize::Icon | ButtonSize::IconSm => 14.0,
            ButtonSize::Lg | ButtonSize::IconLg => 16.0,
        }
    }

    pub(crate) fn square(self) -> Self {
        match self {
            ButtonSize::Default | ButtonSize::Icon => ButtonSize::Icon,
            ButtonSize::Xs | ButtonSize::IconXs => ButtonSize::IconXs,
            ButtonSize::Sm | ButtonSize::IconSm => ButtonSize::IconSm,
            ButtonSize::Lg | ButtonSize::IconLg => ButtonSize::IconLg,
            ButtonSize::None | ButtonSize::IconNone => ButtonSize::IconNone,
        }
    }

    pub(crate) fn wide(self) -> Self {
        match self {
            ButtonSize::Default | ButtonSize::Icon => ButtonSize::Default,
            ButtonSize::Xs | ButtonSize::IconXs => ButtonSize::Xs,
            ButtonSize::Sm | ButtonSize::IconSm => ButtonSize::Sm,
            ButtonSize::Lg | ButtonSize::IconLg => ButtonSize::Lg,
            ButtonSize::None | ButtonSize::IconNone => ButtonSize::None,
        }
    }

    fn is_icon(self) -> bool {
        matches!(
            self,
            ButtonSize::Icon
                | ButtonSize::IconXs
                | ButtonSize::IconSm
                | ButtonSize::IconLg
                | ButtonSize::IconNone
        )
    }
}

/// Builds a button with the shared appearance and hover state. Callers add
/// children and an activation handler.
pub fn button(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    button_no_hover(id, c, variant, size).hover(variant_hover(variant, c))
}

/// Same shared geometry and focus/pressed behavior without a hover handler.
/// Use when a caller composes a custom hover state.
pub fn button_no_hover(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    button_surface(id, c, variant, size, false)
}

/// Builds an inert button with the enabled control's geometry and a disabled
/// palette treatment. Callers add only presentational children.
pub fn button_disabled(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    button_surface(id, c, variant, size, true)
}

fn button_surface(
    id: impl Into<gpui::ElementId>,
    c: Palette,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
) -> Stateful<Div> {
    let height = c.control_space(size.height());
    let mut el = div()
        .id(id)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .gap(c.control_space(4.0))
        .h(height)
        .rounded(px(c.radius.sm))
        .text_size(px(size.label_size() * c.ui_font_scale))
        .when(size.is_icon(), |button| button.w(height))
        .when(!size.is_icon(), |button| {
            button.px(c.control_space(size.horizontal_padding()))
        });

    el = apply_variant(el, variant, c);

    if disabled {
        disabled_style(el, variant, c)
            .tab_index(-1)
            .cursor_default()
    } else {
        el.tab_index(0)
            .cursor_pointer()
            .focus(|style| focus_style(style, variant, c))
            .active(|style| active_style(style, variant, c))
    }
}

fn apply_variant(el: Stateful<Div>, variant: ButtonVariant, c: Palette) -> Stateful<Div> {
    match variant {
        ButtonVariant::Subtle => el.bg(transparent_black()).text_color(c.fg),
        ButtonVariant::Filled => el.bg(c.card).text_color(c.fg),
        ButtonVariant::Outlined => el
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .text_color(c.fg),
        ButtonVariant::OutlinedGhost => el
            .border_1()
            .border_color(c.border)
            .bg(transparent_black())
            .text_color(c.fg),
        ButtonVariant::TintedError => el.bg(c.error.opacity(0.12)).text_color(c.error),
        ButtonVariant::Link => el.bg(transparent_black()).text_color(c.primary),
        ButtonVariant::Transparent => el.bg(transparent_black()).text_color(c.muted),
    }
}

fn disabled_style(el: Stateful<Div>, variant: ButtonVariant, c: Palette) -> Stateful<Div> {
    match variant {
        ButtonVariant::Outlined => el.border_color(c.border).text_color(c.muted),
        ButtonVariant::OutlinedGhost => {
            el.border_color(c.border).bg(c.muted_bg).text_color(c.muted)
        }
        ButtonVariant::TintedError => el.bg(c.muted_bg).text_color(c.muted),
        ButtonVariant::Subtle
        | ButtonVariant::Filled
        | ButtonVariant::Link
        | ButtonVariant::Transparent => el.text_color(c.muted),
    }
}

fn variant_hover(
    variant: ButtonVariant,
    c: Palette,
) -> impl Fn(StyleRefinement) -> StyleRefinement {
    move |style| match variant {
        ButtonVariant::Subtle | ButtonVariant::OutlinedGhost => style.bg(c.accent),
        ButtonVariant::Filled | ButtonVariant::Outlined => style.bg(c.muted_bg),
        ButtonVariant::TintedError => style.bg(c.error.opacity(0.2)),
        ButtonVariant::Link => style.underline(),
        ButtonVariant::Transparent => style.text_color(c.fg),
    }
}

fn focus_style(style: StyleRefinement, variant: ButtonVariant, c: Palette) -> StyleRefinement {
    if matches!(
        variant,
        ButtonVariant::Outlined | ButtonVariant::OutlinedGhost
    ) {
        style.border_color(c.ring)
    } else {
        variant_hover(variant, c)(style)
    }
}

fn active_style(style: StyleRefinement, variant: ButtonVariant, c: Palette) -> StyleRefinement {
    match variant {
        ButtonVariant::Subtle | ButtonVariant::OutlinedGhost => style.bg(c.border),
        ButtonVariant::Filled | ButtonVariant::Outlined => style.bg(c.accent),
        ButtonVariant::TintedError => style.bg(c.error.opacity(0.24)),
        ButtonVariant::Link => style.underline(),
        ButtonVariant::Transparent => style.text_color(c.fg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn every_appearance_builds_at_every_shared_size() {
        let c = test_palette();
        for variant in [
            ButtonVariant::Subtle,
            ButtonVariant::Filled,
            ButtonVariant::Outlined,
            ButtonVariant::OutlinedGhost,
            ButtonVariant::TintedError,
            ButtonVariant::Link,
            ButtonVariant::Transparent,
        ] {
            for size in [
                ButtonSize::Default,
                ButtonSize::Xs,
                ButtonSize::Sm,
                ButtonSize::Lg,
                ButtonSize::None,
                ButtonSize::Icon,
                ButtonSize::IconXs,
                ButtonSize::IconSm,
                ButtonSize::IconLg,
                ButtonSize::IconNone,
            ] {
                let _ = button("enabled", c, variant, size);
                let _ = button_no_hover("no-hover", c, variant, size);
                let _ = button_disabled("disabled", c, variant, size);
            }
        }
    }

    #[test]
    fn shared_sizes_match_the_reference_control_scale() {
        assert_eq!(ButtonSize::Lg.height(), 32.0);
        assert_eq!(ButtonSize::Sm.height(), 28.0);
        assert_eq!(ButtonSize::Default.height(), 22.0);
        assert_eq!(ButtonSize::Xs.height(), 18.0);
        assert_eq!(ButtonSize::None.height(), 16.0);
    }
}
