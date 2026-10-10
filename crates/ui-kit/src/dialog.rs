//! Shared modal scrim and dialog surface.
//!
//! Owners keep dialog state, focus, dismissal, and decisions. This module
//! supplies the common centered overlay and theme-bound card treatment.

use gpui::{div, px, Div, ElementId, InteractiveElement, Stateful, Styled};

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

/// The shared card frame used by owner-rendered dialogs.
pub fn dialog_surface(id: impl Into<ElementId>, palette: Palette) -> Stateful<Div> {
    div()
        .id(id)
        .min_w_0()
        .flex()
        .flex_col()
        .gap(palette.space(8.0))
        .p(palette.space(12.0))
        .rounded(px(palette.radius.md))
        .border_1()
        .border_color(palette.border)
        .bg(palette.card)
        .text_color(palette.fg)
        .shadow_lg()
}

#[cfg(test)]
mod tests {
    use gpui::ParentElement;

    use super::{dialog_surface, modal_overlay};
    use crate::test_support::test_palette;

    #[test]
    fn builds_centered_overlay_and_themed_dialog_surface() {
        let _ = modal_overlay("dialog-overlay");
        let _ = dialog_surface("dialog-card", test_palette()).child("Content");
    }
}
