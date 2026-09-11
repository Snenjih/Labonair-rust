//! Shared entrance-fade for popovers, dropdowns, context menus and the
//! command palette.
//!
//! Every floating panel in the app used to appear on its very first painted
//! frame at full opacity — a "PLOP". [`fade_in`] builds a GPUI
//! [`gpui::Animation`] that eases a panel's opacity from `0` to `1` over
//! [`Palette::dur_fast`] using the [`Palette::ease_premium`] curve, the same
//! tokens the existing tab-in fade (`crates/workspace/src/workspace.rs`)
//! already reads off the theme. It is a pure per-frame interpolation
//! (`request_animation_frame`, no timer/async delay), so the click that opens
//! the panel still reacts immediately — only the panel's own appearance eases
//! in.
//!
//! Callers apply it as the last builder call on the panel element, right
//! before it is handed to `deferred`/`anchored`:
//!
//! ```ignore
//! use gpui::AnimationExt;
//!
//! div()
//!     .child(content)
//!     .with_animation("my-popover-fade", fade_in(c), |el, delta| el.opacity(delta))
//! ```
//!
//! Every call site here builds the panel only while it is open (`Option<..>`
//! state), so the element is freshly mounted each time it opens and the fade
//! restarts from `0` on every open — no extra state needed.

use std::time::Duration;

use gpui::Animation;

use crate::palette::Palette;

/// GPUI divides elapsed time by the animation's duration every frame; a hard
/// zero (full reduce-motion) would divide by zero, so clamp to a floor far
/// below a single frame instead of skipping the animation outright. Mirrors
/// the existing tab-in animation's floor (`workspace.rs`).
const REDUCE_MOTION_FLOOR: Duration = Duration::from_micros(10);

/// The fade-in [`Animation`] for a panel styled from `c` — `dur_fast`,
/// `ease_premium`-eased, reduce-motion aware.
pub fn fade_in(c: Palette) -> Animation {
    let duration = if c.reduce_motion || c.dur_fast.is_zero() {
        REDUCE_MOTION_FLOOR
    } else {
        c.dur_fast
    };
    let ease = c.ease_premium;
    Animation::new(duration).with_easing(move |t| ease.eval(t))
}
