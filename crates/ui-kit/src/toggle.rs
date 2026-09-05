//! `ToggleButton` / `IconToggleButton` — a button with a sticky pressed state.
//!
//! Port of `reference-src/src/components/ui/toggle.tsx` (`toggleVariants`):
//! `rounded-3xl`, `hover:bg-muted hover:text-foreground`, `aria-pressed:bg-muted`,
//! `disabled:opacity-50`; the `outline` variant adds `border border-input`.
//! Zed's counterpart is
//! `zed-refrence/zed/crates/ui/src/components/toggle.rs`.
//!
//! Replaces the statusbar panel-toggle cluster
//! (`crates/shell/src/status_items.rs::PanelTogglesStatusItem`) and the
//! AI/Shell composer toggle (`crates/panel-ai/src/panel_ai.rs`), which each
//! rebuilt the pressed/hover pair by hand.
//!
//! Unlike [`crate::button`] this is *stateful in appearance only* — the caller
//! still owns the `pressed` flag and flips it in `on_click`.
//!
//! Two entry points: [`icon_toggle_button`] for the square icon-only shape, and
//! [`toggle_base`] for everything else (labelled toggles, non-default
//! variant/size, the disabled state) — chain `.child(..)`/`.on_click(..)` on
//! either, exactly like [`crate::button`].
//!
//! ```ignore
//! icon_toggle_button("bar-toggle-explorer", c, IconName::FolderTree, is_open)
//!     .on_click(cx.listener(|this, _, _w, cx| this.toggle_panel(cx)))
//! ```

use gpui::{
    div, prelude::FluentBuilder, px, Div, ElementId, InteractiveElement, ParentElement, Stateful,
    Styled,
};

use crate::icon::IconName;
use crate::palette::Palette;
use crate::DISABLED_OPACITY;

/// `toggleVariants.variant`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToggleVariant {
    /// `bg-transparent` — the statusbar / toolbar look.
    #[default]
    Default,
    /// `border border-input bg-transparent`.
    Outline,
}

/// `toggleVariants.size`, plus the `Xs` statusbar scale the port needs (the
/// reference's smallest, `sm`, is 32px — twice the height of the 20px status
/// bar controls).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToggleSize {
    /// 20px — statusbar controls.
    #[default]
    Xs,
    /// 32px (`h-8 min-w-8`).
    Sm,
    /// 36px (`h-9 min-w-9`).
    Md,
}

impl ToggleSize {
    fn height(self) -> f32 {
        match self {
            ToggleSize::Xs => 20.0,
            ToggleSize::Sm => 32.0,
            ToggleSize::Md => 36.0,
        }
    }

    fn icon(self) -> f32 {
        match self {
            ToggleSize::Xs => 16.0,
            ToggleSize::Sm | ToggleSize::Md => 18.0,
        }
    }

    fn text(self) -> f32 {
        match self {
            ToggleSize::Xs => 11.0,
            ToggleSize::Sm | ToggleSize::Md => 14.0,
        }
    }
}

/// The pre-styled, square icon toggle. Chain `.child(..)` / `.on_click(..)` on
/// the returned element exactly like [`crate::button`].
pub fn icon_toggle_button(
    id: impl Into<ElementId>,
    c: Palette,
    icon: IconName,
    pressed: bool,
) -> Stateful<Div> {
    let size = ToggleSize::default();
    let color = if pressed { c.fg } else { c.muted };
    base(id, c, ToggleVariant::default(), size, pressed, false)
        .w(c.space(size.height()))
        .justify_center()
        .child(icon.svg(color).size(px(size.icon())))
}

/// The full builder, for the labelled toggles and any call site that needs a
/// non-default variant/size or the disabled state. Chain `.child(..)` for the
/// icon/label and `.on_click(..)` for the handler, exactly like
/// [`crate::button`].
pub fn toggle_base(
    id: impl Into<ElementId>,
    c: Palette,
    variant: ToggleVariant,
    size: ToggleSize,
    pressed: bool,
    disabled: bool,
) -> Stateful<Div> {
    base(id, c, variant, size, pressed, disabled)
}

fn base(
    id: impl Into<ElementId>,
    c: Palette,
    variant: ToggleVariant,
    size: ToggleSize,
    pressed: bool,
    disabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .gap(c.space(4.0))
        .h(c.space(size.height()))
        .min_w(c.space(size.height()))
        .rounded(px(c.radius.md))
        .text_size(px(size.text()))
        .text_color(if pressed { c.fg } else { c.muted })
        .when(variant == ToggleVariant::Outline, |d| {
            d.border_1().border_color(c.input)
        })
        // `aria-pressed:bg-muted` from `toggleVariants`.
        .when(pressed, |d| d.bg(c.muted_bg))
        .when(disabled, |d| d.opacity(DISABLED_OPACITY))
        .when(!disabled, |d| {
            d.cursor_pointer()
                .hover(move |s| s.bg(c.muted_bg).text_color(c.fg))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_in_every_variant_size_and_state() {
        let c = test_palette();
        for v in [ToggleVariant::Default, ToggleVariant::Outline] {
            for s in [ToggleSize::Xs, ToggleSize::Sm, ToggleSize::Md] {
                for pressed in [true, false] {
                    for disabled in [true, false] {
                        let _ = toggle_base("t", c, v, s, pressed, disabled);
                    }
                }
            }
        }
    }

    #[test]
    fn icon_helper_builds_in_both_states() {
        let c = test_palette();
        for pressed in [true, false] {
            let _ = icon_toggle_button("t", c, IconName::PanelLeft, pressed);
        }
    }
}
