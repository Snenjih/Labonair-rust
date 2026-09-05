//! `Select` / `EnumDropdown` — a trigger showing the current option plus the
//! anchored list of the alternatives.
//!
//! Port of `reference-src/src/components/ui/select.tsx`: the trigger is
//! `border-input bg-background rounded-md px-3` with a trailing chevron and an
//! accent border while open; the content is a `bg-popover rounded-md border
//! shadow-md` panel whose items are `rounded-sm px-2 focus:bg-accent`. Zed's
//! counterpart is
//! `zed-refrence/zed/crates/ui/src/components/dropdown_menu.rs`.
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
//! select_popover(menu.at, c, &menu.options, &current, dismiss, on_select)
//! ```

use std::rc::Rc;

use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, Div,
    ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels,
    Point, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};

use crate::icon::IconName;
use crate::palette::Palette;

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
    div()
        .id(id)
        .min_w(px(160.0))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py(px(4.0))
        .rounded(px(c.radius.sm))
        .border_1()
        .border_color(if open { c.accent } else { c.border })
        .bg(c.bg)
        .text_color(c.fg)
        .text_size(px(11.5))
        .cursor_pointer()
        .child(label.into())
        .child(IconName::ChevronDown.svg(c.muted).size(px(12.0)))
}

/// The open options list, anchored at `anchor` (window coordinates — pass the
/// click position or the trigger's bottom-left) with a dismissing backdrop.
///
/// `deferred` + `anchored().snap_to_window()` so the list is neither clipped by
/// an ancestor's `overflow_hidden` nor pushed off-screen near a window edge.
pub fn select_popover(
    id: impl Into<SharedString>,
    anchor: Point<Pixels>,
    c: Palette,
    options: &[SelectOption],
    selected: &str,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = id.into();
    let dismiss = Rc::new(dismiss);
    let on_select = Rc::new(on_select);

    let rows: Vec<_> = options
        .iter()
        .enumerate()
        .map(|(i, (token, label))| {
            let on = token.as_ref() == selected;
            let token = token.clone();
            let (dismiss, on_select) = (dismiss.clone(), on_select.clone());
            div()
                .id(SharedString::from(format!("{id}-opt-{i}")))
                .w_full()
                .px_2()
                .py(px(4.0))
                .rounded(px(c.radius.sm))
                .text_size(px(11.5))
                .text_color(if on { c.fg } else { c.muted })
                .cursor_pointer()
                .when(on, |d| d.bg(c.accent))
                .when(!on, |d| d.hover(|s| s.bg(c.border)))
                .child(label.clone())
                .on_click(move |_: &ClickEvent, w, cx| {
                    on_select(&token, w, cx);
                    dismiss(w, cx);
                })
        })
        .collect();

    let list = anchored().position(anchor).snap_to_window().child(
        div()
            .id(SharedString::from(format!("{id}-list")))
            .occlude()
            .min_w(px(180.0))
            .max_h(px(320.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .p_1()
            .rounded(px(c.radius.md))
            .bg(c.popover)
            .border_1()
            .border_color(c.border)
            .shadow_lg()
            .children(rows),
    );

    let backdrop_dismiss = dismiss.clone();
    deferred(
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, w: &mut Window, cx: &mut App| backdrop_dismiss(w, cx),
            )
            .child(list),
    )
    .with_priority(200)
    .into_any_element()
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
        let _ = select_popover(
            "sel",
            Point::default(),
            c,
            &opts(),
            "bar",
            |_, _| {},
            |_, _, _| {},
        );
        // An empty option list must not panic either.
        let _ = select_popover("sel", Point::default(), c, &[], "", |_, _| {}, |_, _, _| {});
    }
}
