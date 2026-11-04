//! Exit (closing) state for floating surfaces: popover, context menu, select
//! list and dialog.
//!
//! GPUI unmounts an element the frame its owner stops building it, so a plain
//! `Option<..>` open flag cannot fade out. [`Presence`] is the small caller-owned
//! state machine that keeps the surface mounted for the exit fade and then lets
//! it go:
//!
//! ```text
//!            open()                    request_close()        exit fade elapsed
//!  Closed ------------> Open ------------------------> Closing ----------------> Closed
//!                         ^                                |                 (tick = JustClosed)
//!                         +--------------- open() --------+
//! ```
//!
//! ## Wiring
//!
//! The owner keeps `presence: Presence` next to (or instead of) its `Option`
//! open state and builds the surface while [`Presence::is_mounted`]:
//!
//! ```ignore
//! // open
//! self.presence.open();
//! // dismiss (Esc, outside click, item click, ...) - safe to call repeatedly
//! if self.presence.request_close(Instant::now(), &c.motion) { self.restore_focus(w); }
//! // in render, once per frame:
//! if self.presence.tick(Instant::now(), &c.motion) == PresenceStep::JustClosed {
//!     self.restore_focus(window); // focus returns only after the unmount
//! }
//! if self.presence.is_mounted() {
//!     root = root.child(popover_with_exit(anchor, w, c, self.presence.is_closing(), dismiss, body));
//! }
//! ```
//!
//! The `*_with_exit` variants ([`crate::popover_with_exit`],
//! [`crate::context_menu_with_exit`], [`crate::popover_menu_with_exit`],
//! [`crate::select_popover_with_exit`], [`crate::modal_overlay_with_exit`]) take
//! the `closing` flag, play [`exit_fade`] while it is set and swallow pointer
//! input so a closing menu cannot fire a second action. The original
//! functions are unchanged and stay instant-unmount, so existing callers need no
//! migration.
//!
//! ## Guarantees
//!
//! * Esc spam is safe: [`Presence::request_close`] while closing keeps the
//!   original start time, so the exit never restarts or extends.
//! * Re-opening during the fade cancels it.
//! * [`MotionLevel::Off`] closes immediately (`request_close` returns `true`).
//! * The animation is time based (an [`Instant`] snapshot, no timer), and
//!   [`exit_fade`] runs a few frames longer than [`exit_duration`] so the owner
//!   is guaranteed one more render after the threshold to unmount.

use std::time::{Duration, Instant};

use gpui::{
    div, px, Animation, AnimationElement, AnimationExt, Div, ElementId, InteractiveElement,
    IntoElement, Pixels, Styled,
};

use crate::animation::{MotionLevel, MotionPrefs};
use crate::palette::Palette;

/// Extra animation time after [`exit_duration`], so the owner re-renders at
/// least once after the unmount threshold (a finished GPUI animation requests
/// no further frame).
const EXIT_PAD: Duration = Duration::from_millis(48);

/// What [`Presence::tick`] observed this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresenceStep {
    /// Not shown.
    Hidden,
    /// Shown normally.
    Open,
    /// Playing the exit fade.
    Closing,
    /// The exit fade just finished and the surface unmounted: restore focus
    /// now. Reported exactly once per close.
    JustClosed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Closed,
    Open,
    Closing { since: Instant },
}

/// Caller-owned open/closing/closed state of one floating surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presence {
    phase: Phase,
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}

/// How long the exit fade runs: `dur_fast` (120 ms), none when motion is off.
pub fn exit_duration(motion: &MotionPrefs) -> Duration {
    if motion.animates_exit() {
        motion.dur_fast
    } else {
        Duration::ZERO
    }
}

impl Presence {
    /// A closed surface.
    pub const fn new() -> Self {
        Self {
            phase: Phase::Closed,
        }
    }

    /// Show the surface; cancels a running exit fade.
    pub fn open(&mut self) {
        self.phase = Phase::Open;
    }

    /// Start closing. Returns `true` when the surface closed immediately
    /// (motion off), so the caller can return focus right away. Idempotent:
    /// calling it while closing or closed changes nothing.
    pub fn request_close(&mut self, now: Instant, motion: &MotionPrefs) -> bool {
        match self.phase {
            Phase::Open => {
                if exit_duration(motion).is_zero() {
                    self.phase = Phase::Closed;
                    true
                } else {
                    self.phase = Phase::Closing { since: now };
                    false
                }
            }
            Phase::Closing { .. } | Phase::Closed => false,
        }
    }

    /// Advance the state machine. Call once per render before building the
    /// surface; [`PresenceStep::JustClosed`] is the moment to return focus.
    pub fn tick(&mut self, now: Instant, motion: &MotionPrefs) -> PresenceStep {
        match self.phase {
            Phase::Closed => PresenceStep::Hidden,
            Phase::Open => PresenceStep::Open,
            Phase::Closing { since } => {
                if now.saturating_duration_since(since) >= exit_duration(motion) {
                    self.phase = Phase::Closed;
                    PresenceStep::JustClosed
                } else {
                    PresenceStep::Closing
                }
            }
        }
    }

    /// Whether the surface should be built this frame (open or closing).
    pub fn is_mounted(&self) -> bool {
        self.phase != Phase::Closed
    }

    /// Whether the surface is playing its exit fade.
    pub fn is_closing(&self) -> bool {
        matches!(self.phase, Phase::Closing { .. })
    }
}

/// The exit [`Animation`]: `dur_fast` of fade progress (`0 -> 1`, drive opacity
/// with `1 - delta`), padded so the owner gets a frame after the unmount
/// threshold. The easing saturates at `1` once [`exit_duration`] has elapsed.
pub fn exit_fade(c: Palette) -> Animation {
    let motion = c.motion;
    let fade = motion.resolve(motion.dur_fast);
    let total = fade + EXIT_PAD;
    let ease = motion.ease_premium;
    let ratio = total.as_secs_f32() / fade.as_secs_f32();
    Animation::new(total).with_easing(move |t| ease.eval((t * ratio).min(1.0)))
}

/// How far a popup card (popover, menu, select list) rises into place on
/// enter. `Full` motion only; `Reduced` and `Off` fade (or snap) in place.
pub const POPUP_SLIDE: Pixels = px(4.0);

/// Apply the shared enter/exit fade: [`crate::fade_in`] under `id` while open,
/// [`exit_fade`] under a distinct id (so the animation restarts) while
/// `closing`.
pub(crate) fn enter_or_exit<E: Styled + IntoElement + 'static>(
    el: E,
    id: &'static str,
    c: Palette,
    closing: bool,
) -> AnimationElement<E> {
    enter_or_exit_sliding(el, id, c, closing, px(0.0))
}

/// [`enter_or_exit`] for popup cards: the enter fade is joined by a
/// [`POPUP_SLIDE`] rise (a `top` offset lerp, `Full` motion only). The exit is
/// a plain fade.
pub(crate) fn enter_or_popup<E: Styled + IntoElement + 'static>(
    el: E,
    id: &'static str,
    c: Palette,
    closing: bool,
) -> AnimationElement<E> {
    enter_or_exit_sliding(el, id, c, closing, POPUP_SLIDE)
}

fn enter_or_exit_sliding<E: Styled + IntoElement + 'static>(
    el: E,
    id: &'static str,
    c: Palette,
    closing: bool,
    slide: Pixels,
) -> AnimationElement<E> {
    if closing && c.motion.level != MotionLevel::Off {
        el.with_animation(
            ElementId::from((ElementId::from(id), "exit")),
            exit_fade(c),
            |el, delta| el.opacity(1.0 - delta),
        )
    } else {
        let motion = c.motion;
        el.with_animation(id, crate::animation::fade_in(c), move |el, delta| {
            let el = el.opacity(delta);
            if slide == px(0.0) {
                el
            } else {
                el.top(crate::animation::slide_offset(&motion, slide, delta))
            }
        })
    }
}

/// A transparent full-size layer that swallows pointer input while a surface
/// is closing, so a fading menu cannot fire a second action.
pub(crate) fn closing_guard() -> Div {
    div().absolute().inset_0().occlude()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;
    use gpui::div;

    fn motion(level: MotionLevel) -> MotionPrefs {
        let mut m = test_palette().motion;
        m.level = level;
        m
    }

    #[test]
    fn closes_after_the_exit_fade_and_reports_it_once() {
        let m = motion(MotionLevel::Full);
        let t0 = Instant::now();
        let mut p = Presence::new();
        assert_eq!(p.tick(t0, &m), PresenceStep::Hidden);
        assert!(!p.is_mounted());

        p.open();
        assert_eq!(p.tick(t0, &m), PresenceStep::Open);
        assert!(p.is_mounted() && !p.is_closing());

        assert!(!p.request_close(t0, &m));
        assert!(p.is_mounted() && p.is_closing());
        assert_eq!(
            p.tick(t0 + Duration::from_millis(119), &m),
            PresenceStep::Closing
        );
        assert_eq!(
            p.tick(t0 + Duration::from_millis(120), &m),
            PresenceStep::JustClosed
        );
        assert!(!p.is_mounted());
        // The unmount is reported exactly once.
        assert_eq!(
            p.tick(t0 + Duration::from_millis(200), &m),
            PresenceStep::Hidden
        );
    }

    #[test]
    fn repeated_close_requests_do_not_extend_the_fade() {
        let m = motion(MotionLevel::Full);
        let t0 = Instant::now();
        let mut p = Presence::new();
        p.open();
        p.request_close(t0, &m);
        // Esc spam while closing keeps the original start.
        for ms in [10, 40, 90, 110] {
            assert!(!p.request_close(t0 + Duration::from_millis(ms), &m));
        }
        assert_eq!(
            p.tick(t0 + Duration::from_millis(120), &m),
            PresenceStep::JustClosed
        );
        // Closing a closed surface is a no-op.
        assert!(!p.request_close(t0 + Duration::from_millis(300), &m));
        assert!(!p.is_mounted());
    }

    #[test]
    fn reopening_cancels_a_running_exit() {
        let m = motion(MotionLevel::Reduced);
        let t0 = Instant::now();
        let mut p = Presence::new();
        p.open();
        p.request_close(t0, &m);
        p.open();
        assert_eq!(
            p.tick(t0 + Duration::from_millis(500), &m),
            PresenceStep::Open
        );
    }

    #[test]
    fn motion_off_closes_immediately() {
        let m = motion(MotionLevel::Off);
        let t0 = Instant::now();
        let mut p = Presence::new();
        p.open();
        assert!(p.request_close(t0, &m), "Off must unmount at once");
        assert!(!p.is_mounted() && !p.is_closing());
        assert_eq!(exit_duration(&m), Duration::ZERO);
    }

    #[test]
    fn exit_fade_outlasts_the_unmount_threshold() {
        let c = test_palette();
        let anim = exit_fade(c);
        assert!(anim.duration > exit_duration(&c.motion));
        // Fully faded by the time the threshold is reached, never out of range.
        let at_threshold = exit_duration(&c.motion).as_secs_f32() / anim.duration.as_secs_f32();
        assert!((anim.easing)(at_threshold) > 0.999);
        assert!((anim.easing)(1.0) <= 1.0);
        assert!((anim.easing)(0.0).abs() < 1e-3);
    }

    #[test]
    fn popup_rise_only_plays_at_full_motion() {
        use crate::animation::slide_offset;
        for (level, start) in [
            (MotionLevel::Full, POPUP_SLIDE),
            (MotionLevel::Reduced, px(0.0)),
            (MotionLevel::Off, px(0.0)),
        ] {
            assert_eq!(slide_offset(&motion(level), POPUP_SLIDE, 0.0), start);
            assert_eq!(slide_offset(&motion(level), POPUP_SLIDE, 1.0), px(0.0));
        }
    }

    #[test]
    fn enter_or_exit_builds_in_both_states_and_levels() {
        for level in [MotionLevel::Full, MotionLevel::Reduced, MotionLevel::Off] {
            let mut c = test_palette();
            c.motion.level = level;
            for closing in [false, true] {
                let _ = enter_or_exit(div(), "surface", c, closing);
                let _ = enter_or_popup(div(), "popup", c, closing);
            }
        }
        let _ = closing_guard();
    }

    #[test]
    fn exit_variants_build_open_and_closing() {
        use gpui::{px, ListAlignment, ListState, Point};

        use crate::{
            context_menu_with_exit, popover_menu_with_exit, popover_with_exit,
            select_popover_with_exit, MenuItem, SelectPopoverAnchor,
        };
        let c = test_palette();
        for closing in [false, true] {
            let items = || vec![MenuItem::new("a", "A").on_click(|_, _, _| {})];
            let _ = context_menu_with_exit(Point::default(), c, closing, |_, _| {}, items());
            let _ = popover_menu_with_exit(Point::default(), c, closing, |_, _| {}, items());
            let _ = popover_with_exit(
                Point::default(),
                px(200.0),
                c,
                closing,
                |_, _| {},
                div().into_any_element(),
            );
            let _ = select_popover_with_exit(
                SelectPopoverAnchor::new(
                    "sel",
                    Point::default(),
                    ListState::new(1, ListAlignment::Top, px(100.0)),
                ),
                c,
                &[("a".into(), "A".into())],
                "a",
                closing,
                |_, _| {},
                |_, _, _| {},
            );
        }
    }
}
