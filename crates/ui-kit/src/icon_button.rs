//! Shared icon-only button with caller-owned selected and disabled states.

use gpui::{
    div, Div, ElementId, Hsla, InteractiveElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled,
};

use crate::{
    button, button_disabled, button_no_hover, ButtonSize, ButtonVariant, IconName, IndicatorSize,
    Palette, Tooltip,
};

/// Width treatment for an icon button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonShape {
    /// Uses the size's natural horizontal padding.
    #[default]
    Wide,
    /// Uses equal width and height.
    Square,
}

/// Builder for icon buttons used by toolbars, panel headers, and forms.
///
/// The owner supplies selected and disabled state on each render. The builder
/// only renders those states and never retains feature state.
pub struct IconButtonBuilder {
    id: ElementId,
    palette: Palette,
    icon: IconName,
    selected_icon: Option<IconName>,
    variant: ButtonVariant,
    selected_variant: Option<ButtonVariant>,
    size: ButtonSize,
    shape: IconButtonShape,
    icon_color: Option<Hsla>,
    selected_icon_color: Option<Hsla>,
    icon_size: Option<f32>,
    alpha: f32,
    selected: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    indicator: Option<(IndicatorSize, Hsla)>,
    indicator_border_color: Option<Hsla>,
}

impl IconButtonBuilder {
    pub fn new(id: impl Into<ElementId>, palette: Palette, icon: IconName) -> Self {
        Self {
            id: id.into(),
            palette,
            icon,
            selected_icon: None,
            variant: ButtonVariant::Subtle,
            selected_variant: None,
            size: ButtonSize::Default,
            shape: IconButtonShape::Wide,
            icon_color: None,
            selected_icon_color: None,
            icon_size: None,
            alpha: 1.0,
            selected: false,
            disabled: false,
            tooltip: None,
            indicator: None,
            indicator_border_color: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn selected_variant(mut self, variant: ButtonVariant) -> Self {
        self.selected_variant = Some(variant);
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn shape(mut self, shape: IconButtonShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn selected_icon(mut self, icon: IconName) -> Self {
        self.selected_icon = Some(icon);
        self
    }

    pub fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = Some(color);
        self
    }

    pub fn selected_icon_color(mut self, color: Hsla) -> Self {
        self.selected_icon_color = Some(color);
        self
    }

    /// Override the glyph size in logical pixels; the active UI font scale
    /// and density still apply when the button is rendered.
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, label: impl Into<SharedString>) -> Self {
        self.tooltip = Some(label.into());
        self
    }

    pub fn indicator(mut self, size: IndicatorSize, color: Hsla) -> Self {
        self.indicator = Some((size, color));
        self
    }

    pub fn indicator_border_color(mut self, color: Hsla) -> Self {
        self.indicator_border_color = Some(color);
        self
    }

    pub fn render(self) -> Stateful<Div> {
        let Self {
            id,
            palette,
            icon,
            selected_icon,
            variant,
            selected_variant,
            size,
            shape,
            icon_color,
            selected_icon_color,
            icon_size,
            alpha,
            selected,
            disabled,
            tooltip,
            indicator,
            indicator_border_color,
        } = self;

        let button_size = match shape {
            IconButtonShape::Wide => size.wide(),
            IconButtonShape::Square => size.square(),
        };
        let button_variant = if selected {
            selected_variant.unwrap_or(variant)
        } else {
            variant
        };

        let mut button = if disabled {
            button_disabled(id, palette, button_variant, button_size)
        } else if selected && selected_variant.is_none() {
            button_no_hover(id, palette, button_variant, button_size)
                .bg(palette.selected_fill)
                .text_color(palette.selected_accent)
                .hover(move |style| style.bg(palette.accent).text_color(palette.fg))
        } else {
            button(id, palette, button_variant, button_size)
        };

        let icon = if selected {
            selected_icon.unwrap_or(icon)
        } else {
            icon
        };
        let color = if disabled {
            palette.muted
        } else if selected {
            selected_icon_color.unwrap_or(palette.selected_accent)
        } else {
            icon_color.unwrap_or_else(|| default_icon_color(palette, variant))
        }
        .opacity(alpha);
        let glyph = icon
            .svg(color)
            .size(palette.control_space(icon_size.unwrap_or_else(|| size.icon_size())));
        let mut icon_content = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .child(glyph);

        if let Some((size, color)) = indicator {
            let mark = crate::indicator(size, color);
            let mark = if let Some(border) = indicator_border_color {
                mark.border_1().border_color(border)
            } else {
                mark
            };
            icon_content = icon_content.child(div().absolute().top_0().right_0().child(mark));
        }

        button = button.child(icon_content);
        if let Some(label) = tooltip {
            button =
                button.tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx));
        }
        button
    }
}

/// Creates the common icon-button builder. New call sites should specify
/// shape when their surrounding layout requires a square control.
pub fn icon_button_builder(
    id: impl Into<ElementId>,
    c: Palette,
    icon: IconName,
) -> IconButtonBuilder {
    IconButtonBuilder::new(id, c, icon)
}

/// Compatibility helper for compact square icon actions.
pub fn icon_button(
    id: impl Into<ElementId>,
    c: Palette,
    icon: IconName,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    icon_button_builder(id, c, icon)
        .variant(variant)
        .size(size)
        .shape(IconButtonShape::Square)
        .render()
}

/// Inert compact icon action with the enabled control's geometry.
pub fn icon_button_disabled(
    id: impl Into<ElementId>,
    c: Palette,
    icon: IconName,
    variant: ButtonVariant,
    size: ButtonSize,
) -> Stateful<Div> {
    icon_button_builder(id, c, icon)
        .variant(variant)
        .size(size)
        .shape(IconButtonShape::Square)
        .disabled(true)
        .render()
}

fn default_icon_color(c: Palette, variant: ButtonVariant) -> Hsla {
    match variant {
        ButtonVariant::Subtle
        | ButtonVariant::Filled
        | ButtonVariant::Outlined
        | ButtonVariant::OutlinedGhost => c.fg,
        ButtonVariant::TintedError => c.error,
        ButtonVariant::Link => c.primary,
        ButtonVariant::Transparent => c.muted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_every_shape_variant_and_size() {
        let c = test_palette();
        for shape in [IconButtonShape::Square, IconButtonShape::Wide] {
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
                    for disabled in [false, true] {
                        let _ = icon_button_builder("icon", c, IconName::Refresh)
                            .variant(variant)
                            .size(size)
                            .shape(shape)
                            .disabled(disabled)
                            .selected(true)
                            .selected_icon(IconName::Check)
                            .selected_variant(ButtonVariant::Filled)
                            .selected_icon_color(c.success)
                            .icon_size(15.0)
                            .indicator(IndicatorSize::Xs, c.success)
                            .indicator_border_color(c.bg)
                            .tooltip("Refresh")
                            .alpha(0.8)
                            .render();
                    }
                }
            }
        }
    }
}
