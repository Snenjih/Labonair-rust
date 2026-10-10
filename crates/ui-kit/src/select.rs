//! `Select` / `EnumDropdown` — a trigger showing the current option plus the
//! anchored list of the alternatives.
//!
//! The trigger and list use Labonair theme tokens, and the popup shares the
//! same viewport-aware positioner as context menus and rich-content popovers.
//! It is designed from the pinned reference's observable control states while
//! keeping option ownership and selection with the calling feature.
//!
//! `gpui-component` ships a `select` module, but it styles itself from
//! *its own* `cx.theme()` global — which the app never syncs to
//! `labonair-theme` — so wrapping it would silently bypass our tokens
//! (Critical Rule 3). The trigger/list pair below is token-bound instead.
//!
//! The open/closed state stays with the caller (it already does — settings-ui
//! keeps a `dropdown: Option<SelectMenu>`), so this is deliberately two
//! functions rather than one stateful element:
//!
//! ```ignore
//! // in the row:
//! select_trigger("sel-font", c, current_label, is_open)
//!     .on_click(cx.listener(|this, ev: &ClickEvent, _w, cx| this.open_menu(ev.position(), cx)))
//! // once, at the view's top level (so it is not clipped by the scroll area):
//! select_popover(
//!     SelectPopoverAnchor::new("font", menu.at, list_state),
//!     c, &menu.options, &current, dismiss, on_select,
//! )
//! ```

use std::rc::Rc;

use gpui::{
    deferred, div, list, prelude::FluentBuilder, px, AnimationExt, AnyElement, App, ClickEvent,
    Div, ElementId, InteractiveElement, IntoElement, ListSizingBehavior, ListState, ParentElement,
    Pixels, Point, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};

use crate::animation::fade_in;
use crate::icon::IconName;
use crate::palette::Palette;
use crate::popover::anchored_popup;
use crate::DISABLED_OPACITY;

/// One `(token, label)` pair — the token is what gets stored, the label what is
/// shown.
pub type SelectOption = (SharedString, SharedString);

/// The label to show for `value`, or `None` when it matches no option.
///
/// Pure — lets call sites (and tests) resolve the trigger text without
/// rendering.
pub fn selected_label<'a>(options: &'a [SelectOption], value: &str) -> Option<&'a SharedString> {
    options
        .iter()
        .find(|(token, _)| token.as_ref() == value)
        .map(|(_, label)| label)
}

/// The closed control: current label + chevron, accent border while `open`.
pub fn select_trigger(
    id: impl Into<ElementId>,
    c: Palette,
    label: impl Into<SharedString>,
    open: bool,
) -> Stateful<Div> {
    select_trigger_with_state(id, c, label, open, false)
}

/// The inert visual state of a select trigger. Disabled triggers preserve the
/// normal control geometry while dropping pointer affordance and keyboard
/// focus.
pub fn select_trigger_disabled(
    id: impl Into<ElementId>,
    c: Palette,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    select_trigger_with_state(id, c, label, false, true)
}

/// Per-owner state required to render one anchored, virtualized select list.
#[derive(Clone)]
pub struct SelectPopoverAnchor {
    id: SharedString,
    position: Point<Pixels>,
    list_state: ListState,
}

impl SelectPopoverAnchor {
    pub fn new(
        id: impl Into<SharedString>,
        position: Point<Pixels>,
        list_state: ListState,
    ) -> Self {
        Self {
            id: id.into(),
            position,
            list_state,
        }
    }
}

fn select_trigger_with_state(
    id: impl Into<ElementId>,
    c: Palette,
    label: impl Into<SharedString>,
    open: bool,
    disabled: bool,
) -> Stateful<Div> {
    let mut trigger = div()
        .id(id)
        .min_w(c.space(160.0))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py(c.space(4.0))
        .rounded(px(c.radius.sm))
        .border_1()
        .border_color(if open { c.accent } else { c.border })
        .bg(if disabled { c.muted_bg } else { c.bg })
        .text_color(if disabled { c.muted } else { c.fg })
        .text_size(px(11.5))
        .child(label.into())
        .child(IconName::ChevronDown.svg(c.muted).size(px(12.0)));
    trigger = if disabled {
        trigger
            .tab_index(-1)
            .cursor_default()
            .opacity(DISABLED_OPACITY)
    } else {
        trigger
            .cursor_pointer()
            .tab_index(0)
            .focus(|s| s.border_color(c.ring))
    };
    trigger
}

/// The open options list, anchored at `anchor` (window coordinates — pass the
/// click position or the trigger's bottom-left) with outside-press dismissal.
///
/// `deferred` + shared window-anchored placement keeps the list out of ancestor
/// clipping and preserves a consistent margin near each viewport edge. The
/// caller owns `list_state`; rows are rendered virtually and the viewport is
/// capped at 320 logical pixels.
pub fn select_popover(
    anchor: SelectPopoverAnchor,
    c: Palette,
    options: &[SelectOption],
    selected: &str,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let SelectPopoverAnchor {
        id,
        position,
        list_state,
    } = anchor;
    let dismiss = Rc::new(dismiss);
    let on_select = Rc::new(on_select);

    let selected = SharedString::from(selected.to_owned());
    let row_dismiss = dismiss.clone();
    let row_on_select = on_select.clone();
    let row_id = id.clone();
    let rows = options.to_vec();
    let list = list(list_state, move |i, _, _| {
        let (token, label) = rows[i].clone();
        let on = token == selected;
        let (dismiss, on_select) = (row_dismiss.clone(), row_on_select.clone());
        div()
            .id(SharedString::from(format!("{row_id}-opt-{i}")))
            .w_full()
            .px_2()
            .py(c.space(4.0))
            .rounded(px(c.radius.sm))
            .text_size(px(11.5))
            .text_color(if on { c.fg } else { c.muted })
            .cursor_pointer()
            .when(on, |d| d.bg(c.accent))
            .when(!on, |d| d.hover(|s| s.bg(c.border)))
            .child(label)
            .on_click(move |_: &ClickEvent, w, cx| {
                on_select(&token, w, cx);
                dismiss(w, cx);
                cx.stop_propagation();
            })
            .into_any_element()
    })
    .with_sizing_behavior(ListSizingBehavior::Infer)
    .max_h(c.space(320.0))
    .w_full();

    let outside_dismiss = dismiss.clone();
    let list = anchored_popup(position, c).child(
        div()
            .id(SharedString::from(format!("{id}-list")))
            .occlude()
            .min_w(c.space(180.0))
            .flex()
            .flex_col()
            .p_1()
            .rounded(px(c.radius.md))
            .bg(c.popover)
            .border_1()
            .border_color(c.border)
            .shadow_lg()
            .child(list)
            .on_mouse_down_out(move |_, window, cx| outside_dismiss(window, cx))
            .with_animation("select-fade", fade_in(c), |el, delta| el.opacity(delta)),
    );

    deferred(list).with_priority(200).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    fn opts() -> Vec<SelectOption> {
        vec![
            ("block".into(), "Block".into()),
            ("bar".into(), "Bar".into()),
        ]
    }

    #[test]
    fn resolves_the_selected_label() {
        let o = opts();
        assert_eq!(
            selected_label(&o, "bar").map(SharedString::as_ref),
            Some("Bar")
        );
        assert!(selected_label(&o, "underline").is_none());
        assert!(selected_label(&[], "bar").is_none());
    }

    #[test]
    fn trigger_and_popover_build_in_both_states() {
        let c = test_palette();
        for open in [true, false] {
            let _ = select_trigger("sel", c, "Block", open);
        }
        let _ = select_trigger_disabled("sel-disabled", c, "Block");
        let _ = select_popover(
            SelectPopoverAnchor::new(
                "sel",
                Point::default(),
                ListState::new(opts().len(), gpui::ListAlignment::Top, px(320.0)),
            ),
            c,
            &opts(),
            "bar",
            |_, _| {},
            |_, _, _| {},
        );
        // An empty option list must not panic either.
        let _ = select_popover(
            SelectPopoverAnchor::new(
                "sel",
                Point::default(),
                ListState::new(0, gpui::ListAlignment::Top, px(320.0)),
            ),
            c,
            &[],
            "",
            |_, _| {},
            |_, _, _| {},
        );
    }
}
