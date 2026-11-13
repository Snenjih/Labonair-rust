//! `option_card` — a selectable card with a title and a description, for
//! choosing one of a few mutually exclusive options (onboarding steps).
//!
//! The caller owns the selection and chains `.on_click(..)`; the card owns
//! the resting, hover and selected look.
//!
//! ```ignore
//! option_card("mode-app", c, "Tied to the app", "Ends with the app.", on)
//!     .on_click(cx.listener(|this, _, _, cx| this.select(cx)))
//! ```

use gpui::{
    div, prelude::FluentBuilder, px, ElementId, FontWeight, InteractiveElement, ParentElement,
    SharedString, Stateful, Styled,
};

use crate::palette::Palette;

/// One option card; fills its flex row (`flex_1`).
pub fn option_card(
    id: impl Into<ElementId>,
    c: Palette,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    selected: bool,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_1()
        .flex()
        .flex_col()
        .gap(c.space(4.0))
        .p(c.space(12.0))
        .rounded(px(c.radius.lg))
        .border_2()
        .border_color(if selected { c.primary } else { c.line_subtle })
        .bg(if selected { c.selected_fill } else { c.card })
        .cursor_pointer()
        .hover(|card| card.bg(c.hover))
        .child(
            div()
                .text_color(c.fg)
                .when(selected, |t| t.font_weight(FontWeight::BOLD))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(c.control_space(12.0))
                .text_color(c.muted)
                .child(description.into()),
        )
}
