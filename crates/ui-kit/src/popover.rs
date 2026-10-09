//! Shared anchored dropdown/popover primitive for statusbar-style info items
//! (T18-004).
//!
//! [`context_menu`](crate::context_menu) covers flat `MenuItem` lists
//! anchored at a click point; the statusbar info items (Notifications,
//! Agent-Access, …) need the same anchor/dismiss mechanics around arbitrary
//! rich content (badges, list rows, "Clear all"). This module is that: a
//! `deferred` + `anchored` + `snap_to_window_with_margin` layer (same pattern
//! as `settings-ui`'s `render_dropdown`), so the popover never clips against
//! the statusbar's own `overflow_hidden` and stays inset from the window edge,
//! with card-level outside-click dismissal. This is the shared statusbar
//! dropdown primitive for rich content.

use gpui::{
    anchored, deferred, div, AnimationExt, AnyElement, App, InteractiveElement, IntoElement,
    ParentElement, Pixels, Point, Styled, Window,
};

use crate::animation::fade_in;
use crate::palette::Palette;

/// Build a dropdown card anchored at `anchor` (window coordinates — pass the
/// trigger's bottom-left point so the card opens below the item). Clicks
/// outside the card dismiss it; the caller wires `Esc` itself
/// (`on_key_down` + `track_focus`).
pub fn popover(
    anchor: Point<Pixels>,
    width: Pixels,
    c: Palette,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    content: AnyElement,
) -> AnyElement {
    let (card_bg, border) = (c.popover, c.border);

    let card = anchored()
        .position(anchor)
        .snap_to_window_with_margin(gpui::px(8.0))
        .child(
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
                .child(content)
                .on_mouse_down_out(move |_, window, cx| dismiss(window, cx))
                .with_animation("popover-fade", fade_in(c), |el, delta| el.opacity(delta)),
        );

    deferred(card).with_priority(200).into_any_element()
}
