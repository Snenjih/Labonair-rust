//! Shared search-field chrome.
//!
//! The caller owns the input entity, query behavior, and optional clear
//! action. This primitive keeps the search icon, field spacing, border, focus,
//! and surface consistent wherever a search input is composed.

use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, Stateful, StatefulInteractiveElement, Styled, Window,
};
use gpui_component::input::Input;

use crate::{
    button::{ButtonSize, ButtonVariant},
    icon::IconName,
    icon_button::{icon_button_builder, IconButtonShape},
    palette::Palette,
    text_field::InputState,
};

/// Builds the text editor used inside a shared search field. The input state
/// remains owned by the calling feature; this function standardizes its
/// chrome so search surfaces do not each tune the same input independently.
pub fn search_input(state: &Entity<InputState>, c: Palette) -> Input {
    crate::text_input(state, c)
}

/// The standard trailing action for a non-empty search field.
pub fn search_clear_button(
    id: impl Into<ElementId>,
    c: Palette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<gpui::Div> {
    icon_button_builder(id, c, IconName::X)
        .variant(ButtonVariant::Subtle)
        .size(ButtonSize::IconXs)
        .shape(IconButtonShape::Square)
        .tooltip("Clear search")
        .render()
        .on_click(on_click)
}

/// Builds the common single-line search-field container around a caller-owned
/// input and optional clear action.
pub fn search_field(
    id: impl Into<ElementId>,
    c: Palette,
    focused: bool,
    input: impl IntoElement,
    clear: Option<AnyElement>,
) -> Stateful<gpui::Div> {
    let mut field = div()
        .id(id)
        .h(c.space(28.0))
        .flex()
        .items_center()
        .gap(c.space(6.0))
        .pl(c.space(6.0))
        .pr(c.space(2.0))
        .rounded(px(c.radius.sm))
        .border_1()
        .border_color(if focused { c.ring } else { c.border })
        .bg(c.input)
        .child(IconName::Search.svg(c.muted).size(px(14.0)))
        .child(div().flex_1().min_w_0().child(input));

    if !focused {
        field = field.hover(move |style| style.border_color(c.accent));
    }
    if let Some(clear) = clear {
        field = field.child(clear);
    }

    field
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_focused_and_unfocused_fields_with_or_without_clear_action() {
        let c = test_palette();
        let _ = search_field("search", c, false, div().child("query"), None);
        let _ = search_field(
            "search-focused",
            c,
            true,
            div().child("query"),
            Some(div().child("clear").into_any_element()),
        );
        let _ = search_clear_button("search-clear", c, |_, _, _| {});
    }
}
