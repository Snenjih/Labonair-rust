//! Shared modal scrim and dialog surface.
//!
//! Owners keep dialog state, focus, dismissal, and decisions. This module
//! supplies the common centered overlay and theme-bound card treatment.

use gpui::{
    div, prelude::FluentBuilder, px, AnimationElement, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, Stateful, Styled,
};

use crate::presence::{closing_guard, enter_or_exit};
use crate::Palette;

/// A centered modal scrim. The scrim color is fixed across light and dark
/// themes, matching the application's other modal overlays.
pub fn modal_overlay(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black().opacity(0.30))
}

/// [`modal_overlay`] with enter/exit fade for the scrim and its `content`,
/// using the palette-aware [`Palette::scrim`]. While `closing` it plays the
/// exit fade and swallows pointer input. Drive `closing` from a
/// [`crate::Presence`] and return focus when it reports
/// [`crate::PresenceStep::JustClosed`]; see [`crate::presence`].
pub fn modal_overlay_with_exit(
    id: impl Into<ElementId>,
    c: Palette,
    closing: bool,
    content: impl IntoElement,
) -> AnimationElement<Stateful<Div>> {
    enter_or_exit(
        div()
            .id(id)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(c.scrim)
            .child(content)
            .when(closing, |overlay| overlay.child(closing_guard())),
        "dialog-fade",
        c,
        closing,
    )
}

/// The shared card frame used by owner-rendered dialogs.
pub fn dialog_surface(id: impl Into<ElementId>, palette: Palette) -> Stateful<Div> {
    div()
        .id(id)
        .min_w_0()
        .flex()
        .flex_col()
        .gap(palette.space(12.0))
        .p(palette.space(24.0))
        // Ciel dialogs carry a 1px `line-strong` border and no shadow.
        .rounded(px(palette.radius.xl4))
        .border_1()
        .border_color(palette.line_strong)
        .bg(palette.popover)
        .text_color(palette.fg)
}

#[cfg(test)]
mod tests {
    use gpui::ParentElement;

    use super::{dialog_surface, modal_overlay, modal_overlay_with_exit};
    use crate::test_support::test_palette;

    #[test]
    fn builds_centered_overlay_and_themed_dialog_surface() {
        let _ = modal_overlay("dialog-overlay");
        let _ = dialog_surface("dialog-card", test_palette()).child("Content");
        for closing in [false, true] {
            let _ = modal_overlay_with_exit("dialog-exit", test_palette(), closing, "Content");
        }
    }
}
