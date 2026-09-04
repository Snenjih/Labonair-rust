//! Shared anchored dropdown/popover primitive for statusbar-style info items
//! (T18-004).
//!
//! [`context_menu`](crate::context_menu) covers flat `MenuItem` lists
//! anchored at a click point; the statusbar info items (Notifications,
//! Agent-Access, …) need the same anchor/dismiss mechanics around arbitrary
//! rich content (badges, list rows, "Clear all"). This module is that: a
//! `deferred` + `anchored` + `snap_to_window` layer (same pattern as
//! `settings-ui`'s `render_dropdown`), so the popover never clips against the
//! statusbar's own `overflow_hidden` and never runs off the top/right of the
//! window, plus a transparent backdrop that dismisses on outside click. Until
//! T20-001 replaces this with a dedicated `gpui-component` popover, every
//! statusbar dropdown builds on this one function.

use gpui::{
    anchored, deferred, div, AnyElement, App, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, Styled, Window,
};

use crate::theme::UiTheme;

/// Build a dropdown card anchored at `anchor` (window coordinates — pass the
/// trigger's bottom-left point so the card opens below the item), with a
/// transparent full-window backdrop that dismisses on outside click. The
/// caller wires `Esc` itself (`on_key_down` + `track_focus`), matching the
/// existing `BookmarksView`/`SearchOverlay` convention.
pub fn popover(
    anchor: Point<Pixels>,
    width: Pixels,
    theme: &impl UiTheme,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    content: AnyElement,
) -> AnyElement {
    let core = &theme.theme().core;
    let (card_bg, border) = (core.popover, theme.border());

    let card = anchored().position(anchor).snap_to_window().child(
        div()
            .occlude()
            .w(width)
            .flex()
            .flex_col()
            .rounded_md()
            .bg(card_bg)
            .border_1()
            .border_color(border)
            .shadow_lg()
            .child(content),
    );

    deferred(
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, w: &mut Window, cx: &mut App| dismiss(w, cx),
            )
            .child(card),
    )
    .with_priority(200)
    .into_any_element()
}
