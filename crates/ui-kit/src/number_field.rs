//! `NumberField` — a bounded numeric stepper (`−` / value / `+`) with an
//! optional filled track underneath.
//!
//! Required by T19-004's generated settings UI: every `FieldControl::Int` /
//! `FieldControl::Float` row is one of these. Before this primitive the
//! settings view hand-rolled it twice (`crates/settings-ui/src/panes/generic.rs`
//! plus the private `step_btn`/`slider_track` helpers in `view.rs`), each with
//! its own clamping.
//!
//! Reference: `reference-src/src/components/ui/input-group.tsx` +
//! `slider.tsx` (bounded numeric input with stepper affordances). Zed has no
//! direct equivalent — its settings use `NumericStepper`
//! (`zed-refrence/zed/crates/ui/src/components/numeric_stepper.rs`), which this
//! follows in shape (decrement / value / increment).
//!
//! ```ignore
//! number_field("font-size", c, cur, 8.0, 32.0, 1.0)
//!     .on_change(cx.listener(|this, v: &f64, _w, cx| this.set_font_size(*v, cx)))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, Div, ElementId,
    InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, Window,
};

use crate::palette::Palette;
use crate::DISABLED_OPACITY;

/// Apply `delta` to `value`, clamp into `min..=max` and round away the binary
/// float drift that repeated fractional steps accumulate (`0.05` steps must
/// still print as `0.05`, not `0.05000000000000001`).
///
/// Pure — the whole clamping contract of [`NumberField`] is testable through
/// this one function.
pub fn step_value(value: f64, delta: f64, min: f64, max: f64, decimals: usize) -> f64 {
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    let next = (value + delta).clamp(lo, hi);
    let factor = 10f64.powi(decimals as i32);
    (next * factor).round() / factor
}

/// A bounded numeric stepper. Build with [`number_field`].
pub struct NumberField {
    id: ElementId,
    c: Palette,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    track: bool,
    disabled: bool,
    #[allow(clippy::type_complexity)]
    on_change: Option<Rc<dyn Fn(&f64, &mut Window, &mut App)>>,
}

/// A [`NumberField`] over `min..=max`, stepping by `step`.
pub fn number_field(
    id: impl Into<ElementId>,
    c: Palette,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
) -> NumberField {
    NumberField {
        id: id.into(),
        c,
        value,
        min,
        max,
        step,
        decimals: 0,
        track: true,
        disabled: false,
        on_change: None,
    }
}

impl NumberField {
    /// How many decimals the value is displayed with (and rounded to). `0` by
    /// default — integer fields.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Hide the filled track under the stepper (shown by default).
    pub fn track(mut self, track: bool) -> Self {
        self.track = track;
        self
    }

    /// Dim + inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fires with the already-clamped, already-rounded new value.
    pub fn on_change(mut self, handler: impl Fn(&f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    /// The value one `+`/`−` press would produce — `direction` is `+1`/`-1`.
    /// Exposed so callers (and tests) can assert the clamping without
    /// rendering.
    pub fn stepped(&self, direction: i32) -> f64 {
        step_value(
            self.value,
            self.step * f64::from(direction),
            self.min,
            self.max,
            self.decimals,
        )
    }

    /// The `0.0..=1.0` fill fraction of the track.
    pub fn fraction(&self) -> f32 {
        if self.max <= self.min {
            return 0.0;
        }
        (((self.value - self.min) / (self.max - self.min)) as f32).clamp(0.0, 1.0)
    }

    fn step_button(&self, tag: &str, glyph: &'static str, direction: i32) -> AnyElement {
        let c = self.c;
        let next = self.stepped(direction);
        let at_bound = (next - self.value).abs() < f64::EPSILON;
        let inert = self.disabled || at_bound;
        let handler = self.on_change.clone();
        div()
            .id(SharedString::from(format!("{}-{tag}", self.id)))
            .size(px(20.0))
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .rounded(px(c.radius.sm))
            .border_1()
            .border_color(c.border)
            .text_color(c.fg)
            .when(inert, |d| d.opacity(DISABLED_OPACITY))
            .when(!inert, |d| {
                d.cursor_pointer().hover(move |s| s.bg(c.border))
            })
            .child(glyph)
            .when(!inert, move |d| match handler {
                Some(h) => d.on_click(move |_: &ClickEvent, w, cx| h(&next, w, cx)),
                None => d,
            })
            .into_any_element()
    }

    fn filled_track(&self) -> Div {
        let c = self.c;
        div()
            .mt(px(4.0))
            .w(px(120.0))
            .h(px(4.0))
            .rounded_full()
            .bg(c.border)
            .child(
                div()
                    .h_full()
                    .rounded_full()
                    .bg(c.accent)
                    .w(gpui::relative(self.fraction())),
            )
    }
}

impl IntoElement for NumberField {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let c = self.c;
        let label = SharedString::from(format!("{:.*}", self.decimals, self.value));
        div()
            .flex()
            .flex_col()
            .items_end()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    // `\u{2212}` is MINUS SIGN — the glyph the reference
                    // stepper uses (not a hyphen).
                    .child(self.step_button("dec", "\u{2212}", -1))
                    .child(
                        div()
                            .min_w(px(52.0))
                            .text_center()
                            .text_color(c.fg)
                            .child(label),
                    )
                    .child(self.step_button("inc", "+", 1)),
            )
            .when(self.track, |d| d.child(self.filled_track()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn clamps_at_both_bounds() {
        assert_eq!(step_value(10.0, -1.0, 8.0, 32.0, 0), 9.0);
        assert_eq!(step_value(8.0, -1.0, 8.0, 32.0, 0), 8.0);
        assert_eq!(step_value(32.0, 1.0, 8.0, 32.0, 0), 32.0);
        // A step larger than the remaining headroom still lands on the bound.
        assert_eq!(step_value(31.0, 5.0, 8.0, 32.0, 0), 32.0);
        // An out-of-range start value is pulled back in.
        assert_eq!(step_value(100.0, 0.0, 8.0, 32.0, 0), 32.0);
        // Reversed bounds are tolerated rather than panicking.
        assert_eq!(step_value(5.0, 0.0, 32.0, 8.0, 0), 8.0);
    }

    #[test]
    fn fractional_steps_do_not_drift() {
        let mut v = 0.0;
        for _ in 0..3 {
            v = step_value(v, 0.05, 0.0, 1.0, 2);
        }
        assert_eq!(v, 0.15);
    }

    #[test]
    fn stepped_and_fraction_follow_the_bounds() {
        let c = test_palette();
        let f = number_field("n", c, 12.0, 8.0, 32.0, 2.0);
        assert_eq!(f.stepped(1), 14.0);
        assert_eq!(f.stepped(-1), 10.0);
        assert!((f.fraction() - (4.0 / 24.0)).abs() < 1e-6);

        let at_min = number_field("n", c, 8.0, 8.0, 32.0, 2.0);
        assert_eq!(at_min.stepped(-1), 8.0);
        assert_eq!(at_min.fraction(), 0.0);

        // Degenerate range: no division by zero, no NaN.
        let flat = number_field("n", c, 5.0, 5.0, 5.0, 1.0);
        assert_eq!(flat.fraction(), 0.0);
    }

    #[test]
    fn builds_in_every_state() {
        let c = test_palette();
        for disabled in [true, false] {
            for track in [true, false] {
                let _ = number_field("n", c, 0.5, 0.0, 1.0, 0.05)
                    .decimals(2)
                    .track(track)
                    .disabled(disabled)
                    .on_change(|_, _, _| {})
                    .into_element();
            }
        }
    }
}
