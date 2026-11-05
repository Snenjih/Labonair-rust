//! `switch` - an on/off track with a sliding thumb.
//!
//! The caller owns the boolean and flips it in `on_click`. The track fill fades
//! (`motion_state_fade`) and the thumb slides with a `left` offset lerp
//! (`motion_state`, geometry: it snaps under `Reduced`/`Off`).
//!
//! ```ignore
//! switch("wrap", c, self.wrap)
//!     .on_click(cx.listener(|this, on: &bool, _w, cx| this.set_wrap(*on, cx)))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, App, ClickEvent, Div, ElementId, InteractiveElement,
    IntoElement, ParentElement, Pixels, Stateful, StatefulInteractiveElement, Styled, Window,
};

use crate::animation::{state_fill, MotionExt};
use crate::palette::Palette;
use crate::DISABLED_OPACITY;

const TRACK_W: f32 = 32.0;
const TRACK_H: f32 = 18.0;
const THUMB: f32 = 14.0;
/// Gap between the thumb and the track edge.
const PAD: f32 = 2.0;

/// A switch. Build with [`switch`].
pub struct ToggleSwitch {
    id: ElementId,
    c: Palette,
    on: bool,
    disabled: bool,
    #[allow(clippy::type_complexity)]
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

/// A [`ToggleSwitch`] in the `on` state.
pub fn switch(id: impl Into<ElementId>, c: Palette, on: bool) -> ToggleSwitch {
    ToggleSwitch {
        id: id.into(),
        c,
        on,
        disabled: false,
        on_click: None,
    }
}

/// The thumb's `left` offset at `amount` (`0.0` off .. `1.0` on).
fn thumb_left(amount: f32) -> Pixels {
    px(PAD + (TRACK_W - THUMB - 2.0 * PAD) * amount)
}

impl ToggleSwitch {
    /// Dim + inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fires with the *new* state.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl IntoElement for ToggleSwitch {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let (c, on, disabled, handler) = (self.c, self.on, self.disabled, self.on_click);
        let next = !on;
        div()
            .id(self.id.clone())
            .relative()
            .flex_shrink_0()
            .w(px(TRACK_W))
            .h(px(TRACK_H))
            .rounded_full()
            .bg(c.input)
            .when(disabled, |d| d.opacity(DISABLED_OPACITY))
            .when(!disabled, |d| {
                d.cursor_pointer()
                    .tab_index(0)
                    .focus(|style| style.border_1().border_color(c.ring))
            })
            .child(state_fill(
                (self.id.clone(), "track"),
                c,
                on,
                c.primary,
                None,
                px(TRACK_H / 2.0),
                px(0.0),
            ))
            .child(
                div()
                    .absolute()
                    .top(px((TRACK_H - THUMB) / 2.0))
                    .w(px(THUMB))
                    .h(px(THUMB))
                    .rounded_full()
                    .bg(c.primary_fg)
                    .motion_state((self.id, "thumb"), c, on, true, |thumb, amount| {
                        thumb.left(thumb_left(amount))
                    }),
            )
            .when(!disabled, move |d| match handler {
                Some(h) => d
                    .on_click(move |_: &ClickEvent, w, cx| h(&next, w, cx))
                    .on_key_down(|event, _window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            cx.stop_propagation();
                        }
                    }),
                None => d,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_in_every_state() {
        for on in [true, false] {
            for disabled in [true, false] {
                let _ = switch("sw", test_palette(), on)
                    .disabled(disabled)
                    .on_click(|_, _, _| {})
                    .into_element();
            }
        }
    }

    #[test]
    fn thumb_travels_from_edge_to_edge() {
        assert_eq!(thumb_left(0.0), px(PAD));
        assert_eq!(thumb_left(1.0), px(TRACK_W - THUMB - PAD));
    }
}
