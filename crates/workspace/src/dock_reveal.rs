//! Open/close reveal of the edge docks (R10-002).
//!
//! A dock is only built while it is open, so a reveal needs two pieces of
//! per-dock state: the open flag seen on the previous frame (a *flip*, never
//! the startup state, starts an animation) and, for a close, a short window in
//! which the dock stays mounted while it shrinks. [`DockReveal::step`] turns the
//! open flag of one frame into "build it or not" plus the animation to apply.
//! The animation itself is `MotionExt::motion_reveal` (open) or a mirrored
//! shrink (close) in [`Workspace::render_dock_slot`].

use std::time::{Duration, Instant};

use gpui::{px, AnimationExt, AnyElement, Context, IntoElement, SharedString, Styled, Window};
use labonair_panel::DockPosition;
use labonair_ui_kit::{reveal_extent, Axis, MotionExt, MotionLevel, MotionPrefs, Palette};

use crate::Workspace;

/// Extra time after the reveal so the owner renders once more with the plain
/// dock (a finished GPUI animation requests no further frame).
const REVEAL_PAD: Duration = Duration::from_millis(48);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RevealPhase {
    Opening,
    Closing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DockStep {
    /// Whether the dock must be built this frame (open, or still shrinking).
    pub mounted: bool,
    /// The running reveal and a generation number that changes on every flip
    /// (it keys the animation so a flip restarts it).
    pub reveal: Option<(RevealPhase, u64)>,
}

/// Per-dock reveal state.
#[derive(Default)]
pub(crate) struct DockReveal {
    prev_open: Option<bool>,
    generation: u64,
    running: Option<(RevealPhase, Instant)>,
}

impl DockReveal {
    /// Observe one frame. The first observation is the startup state and never
    /// animates; with motion off a flip is instant.
    pub(crate) fn step(&mut self, open: bool, now: Instant, motion: &MotionPrefs) -> DockStep {
        let flipped = self.prev_open.is_some_and(|prev| prev != open);
        self.prev_open = Some(open);
        if flipped {
            self.generation += 1;
            self.running = (motion.level != MotionLevel::Off).then_some((
                if open {
                    RevealPhase::Opening
                } else {
                    RevealPhase::Closing
                },
                now,
            ));
        }
        if let Some((_, since)) = self.running {
            if now.saturating_duration_since(since) >= motion.dur_slow + REVEAL_PAD {
                self.running = None;
            }
        }
        // A reveal that no longer matches the flag (reopened mid-close) is moot.
        let reveal = self
            .running
            .filter(|(phase, _)| (*phase == RevealPhase::Opening) == open)
            .map(|(phase, _)| (phase, self.generation));
        DockStep {
            mounted: open || reveal.is_some(),
            reveal,
        }
    }
}

fn slot(pos: DockPosition) -> usize {
    match pos {
        DockPosition::Left => 0,
        DockPosition::Right => 1,
        DockPosition::Bottom => 2,
    }
}

impl Workspace {
    /// The dock at `pos` as a layout child, wrapped in its open/close reveal
    /// when one is running; `None` while it is closed and fully collapsed.
    pub(crate) fn render_dock_slot(
        &mut self,
        pos: DockPosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let c = Palette::from_theme(self.theme.read(cx));
        let open = self.dock(pos).is_open();
        let (size, zoomed) = (f32::from(self.dock(pos).size()), self.dock(pos).is_zoomed());
        let step = self.dock_reveal[slot(pos)].step(open, Instant::now(), &c.motion);
        if !step.mounted {
            return None;
        }
        let dock = self.render_dock(pos, cx);
        let Some((phase, generation)) = step.reveal.filter(|_| !zoomed) else {
            return Some(dock.into_any_element());
        };
        // Keep frames coming until the plain dock replaces the wrapper.
        window.request_animation_frame();
        let axis = if pos == DockPosition::Bottom {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let id = SharedString::from(format!(
            "dock-reveal-{}-{generation}",
            crate::dock::position_slug(pos)
        ));
        Some(match phase {
            RevealPhase::Opening => dock
                .motion_reveal(id, c, axis, px(0.0), px(size))
                .into_any_element(),
            RevealPhase::Closing => {
                let motion = c.motion;
                dock.with_animation(
                    id,
                    motion.animation(motion.dur_slow, motion.ease_premium),
                    move |el, delta| {
                        let extent = reveal_extent(&motion, px(size), px(0.0), delta);
                        let el = el.overflow_hidden().opacity(1.0 - delta);
                        match axis {
                            Axis::Horizontal => el.w(extent),
                            Axis::Vertical => el.h(extent),
                        }
                    },
                )
                .into_any_element()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motion(level: MotionLevel) -> MotionPrefs {
        MotionPrefs::new(level, &Default::default())
    }

    #[test]
    fn startup_state_never_animates() {
        let m = motion(MotionLevel::Full);
        let mut r = DockReveal::default();
        let t = Instant::now();
        assert_eq!(
            r.step(true, t, &m),
            DockStep {
                mounted: true,
                reveal: None
            }
        );
    }

    #[test]
    fn open_flip_reveals_then_settles() {
        let m = motion(MotionLevel::Full);
        let mut r = DockReveal::default();
        let t = Instant::now();
        assert!(!r.step(false, t, &m).mounted);
        let s = r.step(true, t, &m);
        assert!(s.mounted);
        assert_eq!(s.reveal.map(|r| r.0), Some(RevealPhase::Opening));
        let later = t + m.dur_slow + REVEAL_PAD;
        assert_eq!(r.step(true, later, &m).reveal, None);
    }

    #[test]
    fn close_flip_keeps_the_dock_mounted_until_the_shrink_ends() {
        let m = motion(MotionLevel::Full);
        let mut r = DockReveal::default();
        let t = Instant::now();
        r.step(true, t, &m);
        let s = r.step(false, t, &m);
        assert!(s.mounted);
        assert_eq!(s.reveal.map(|r| r.0), Some(RevealPhase::Closing));
        assert!(!r.step(false, t + m.dur_slow + REVEAL_PAD, &m).mounted);
    }

    #[test]
    fn reopening_mid_close_swaps_to_an_open_reveal() {
        let m = motion(MotionLevel::Full);
        let mut r = DockReveal::default();
        let t = Instant::now();
        r.step(true, t, &m);
        r.step(false, t, &m);
        let s = r.step(true, t, &m);
        assert_eq!(s.reveal.map(|r| r.0), Some(RevealPhase::Opening));
    }

    #[test]
    fn motion_off_flips_instantly() {
        let m = motion(MotionLevel::Off);
        let mut r = DockReveal::default();
        let t = Instant::now();
        r.step(true, t, &m);
        assert_eq!(
            r.step(false, t, &m),
            DockStep {
                mounted: false,
                reveal: None
            }
        );
        assert_eq!(r.step(true, t, &m).reveal, None);
    }
}
