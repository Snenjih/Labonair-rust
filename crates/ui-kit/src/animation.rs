//! The ui-kit motion engine.
//!
//! Every animation in the app is a pure per-frame interpolation of a GPUI
//! [`gpui::Animation`] (`request_animation_frame`, no timers). GPUI 0.2.2 has
//! **no transforms and no hover transitions**, so the engine only animates what
//! can actually be interpolated: opacity and layout properties (offsets and
//! sizes). Nothing here scales or rotates.
//!
//! ## Tokens and levels
//!
//! [`MotionPrefs`] is the motion slice of the [`Palette`] snapshot: the three
//! theme durations (`dur_fast` 120 ms, `dur_base` 160 ms, `dur_slow` 200 ms),
//! the two easing curves, and a [`MotionLevel`]:
//!
//! | Level     | Behaviour                                                  |
//! |-----------|------------------------------------------------------------|
//! | `Full`    | fades, slides and size reveals                             |
//! | `Reduced` | fades only: offsets and size reveals snap to their end     |
//! | `Off`     | instant: every animation collapses to [`MOTION_FLOOR`]     |
//!
//! GPUI divides elapsed time by the animation duration every frame, so a hard
//! zero would divide by zero. Every duration therefore goes through
//! [`MotionPrefs::resolve`], which keeps a 10 microsecond floor far below one
//! frame instead of skipping the animation.
//!
//! ## Helpers
//!
//! [`fade_in`] builds the shared entrance fade (`dur_fast`, `ease_premium`) used
//! by popovers, menus and the select list. The [`MotionExt`] methods wrap any
//! styled element:
//!
//! ```ignore
//! use labonair_ui_kit::MotionExt;
//!
//! div().child(content).motion_fade_in("my-panel", c);
//! div().child(row).motion_slide_in_y(("row", ix), c, px(6.0));
//! div().child(dock).motion_reveal("dock", c, Axis::Horizontal, px(0.0), px(280.0));
//! div().child(fill).motion_state_fade(("sel", ix), c, selected);
//! ```
//!
//! Animation state lives in GPUI element state keyed by the element id, so the
//! id must be stable between frames and the animation restarts exactly when the
//! element is freshly mounted or its id changes. [`MotionExt::motion_state_fade`]
//! bakes the state into the id so it restarts on every flip.

use std::time::Duration;

use gpui::{px, Animation, AnimationElement, AnimationExt, ElementId, IntoElement, Pixels, Styled};
use labonair_theme_tokens::{Animation as AnimationTokens, CubicBezier};

use crate::divider::Axis;
use crate::palette::Palette;
use crate::theme::UiTheme;

/// GPUI divides elapsed time by the animation's duration every frame; a hard
/// zero (full reduce-motion) would divide by zero, so clamp to a floor far
/// below a single frame instead of skipping the animation outright. Mirrors
/// the existing tab-in animation's floor (`workspace.rs`).
pub const MOTION_FLOOR: Duration = Duration::from_micros(10);

/// How much motion the UI plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MotionLevel {
    /// Fades, slides and size reveals.
    #[default]
    Full,
    /// Fades only; offsets and size reveals snap to their end state.
    Reduced,
    /// Everything is instant (animations collapse to [`MOTION_FLOOR`]).
    Off,
}

/// The motion tokens of a [`Palette`] snapshot: the active [`MotionLevel`], the
/// theme durations and the theme easing curves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionPrefs {
    pub level: MotionLevel,
    /// `--dur-fast`: entrance fades, popovers, menus.
    pub dur_fast: Duration,
    /// `--dur-base`: selection fills, tab-in, small slides.
    pub dur_base: Duration,
    /// `--dur-slow`: size reveals (docks, sidebars, pane bars).
    pub dur_slow: Duration,
    /// `--ease-premium`: the default decelerating curve.
    pub ease_premium: CubicBezier,
    /// `--ease-soft`: symmetric curve for state crossfades.
    pub ease_soft: CubicBezier,
}

impl MotionPrefs {
    /// Build the preferences from the theme timing tokens and a level.
    pub fn new(level: MotionLevel, tokens: &AnimationTokens) -> Self {
        Self {
            level,
            dur_fast: tokens.dur_fast,
            dur_base: tokens.dur_base,
            dur_slow: tokens.dur_slow,
            ease_premium: tokens.ease_premium,
            ease_soft: tokens.ease_soft,
        }
    }

    /// Snapshot the motion tokens off any [`UiTheme`].
    pub fn from_theme(theme: &impl UiTheme) -> Self {
        Self::new(motion_level(theme), &theme.theme().animation)
    }

    /// The duration an animation of nominal length `d` actually runs: `d`, or
    /// [`MOTION_FLOOR`] when motion is off or `d` is zero.
    pub fn resolve(&self, d: Duration) -> Duration {
        if self.level == MotionLevel::Off || d.is_zero() {
            MOTION_FLOOR
        } else {
            d
        }
    }

    /// A GPUI [`Animation`] of nominal length `d` eased with `ease`.
    pub fn animation(&self, d: Duration, ease: CubicBezier) -> Animation {
        Animation::new(self.resolve(d)).with_easing(move |t| ease.eval(t))
    }

    /// Whether offsets and size reveals animate (`Full` only). `Reduced` and
    /// `Off` keep fades but snap geometry.
    pub fn animates_geometry(&self) -> bool {
        self.level == MotionLevel::Full
    }

    /// Whether the exit (closing) state plays at all.
    pub fn animates_exit(&self) -> bool {
        self.level != MotionLevel::Off
    }
}

/// Map the theme to a [`MotionLevel`].
///
/// This is the **single place** that decides the level. Today the only input is
/// the legacy `reduce_motion` metric (`true` maps to `Off`, matching the
/// settings migration); once the `appearance.animations` setting
/// (`system | full | reduced | off`) reaches `ThemeMetrics`, switch it here and
/// nowhere else.
fn motion_level(theme: &impl UiTheme) -> MotionLevel {
    if theme.metrics().reduce_motion {
        MotionLevel::Off
    } else {
        MotionLevel::Full
    }
}

/// The fade-in [`Animation`] for a panel styled from `c`: `dur_fast`,
/// `ease_premium`-eased, floored when motion is off.
pub fn fade_in(c: Palette) -> Animation {
    c.motion.animation(c.motion.dur_fast, c.motion.ease_premium)
}

/// The value an offset lerps to at `delta` (`0..=1`): from `distance` to `0`
/// while geometry animates, otherwise `0` (the end state) immediately.
pub fn slide_offset(motion: &MotionPrefs, distance: Pixels, delta: f32) -> Pixels {
    if motion.animates_geometry() {
        distance * (1.0 - delta)
    } else {
        px(0.0)
    }
}

/// The extent a size lerps to at `delta` (`0..=1`) between `from` and `to`;
/// snaps to `to` when geometry does not animate.
pub fn reveal_extent(motion: &MotionPrefs, from: Pixels, to: Pixels, delta: f32) -> Pixels {
    if motion.animates_geometry() {
        from + (to - from) * delta
    } else {
        to
    }
}

/// The opacity of a state layer at `delta`: fades `0 -> 1` while `on`, stays
/// hidden while off.
pub fn state_opacity(on: bool, delta: f32) -> f32 {
    if on {
        delta
    } else {
        0.0
    }
}

/// Motion wrappers for any styled element. Each takes a stable animation id
/// and the per-render [`Palette`]; see the module docs for the level rules.
pub trait MotionExt: Styled + IntoElement + Sized + 'static {
    /// Fade the element in over `dur_fast` (restarts when freshly mounted).
    fn motion_fade_in(self, id: impl Into<ElementId>, c: Palette) -> AnimationElement<Self> {
        self.with_animation(id, fade_in(c), |el, delta| el.opacity(delta))
    }

    /// Fade in while sliding `distance` up into place (`top` offset lerp, no
    /// transforms; the element keeps its layout slot). `Reduced`/`Off` fade
    /// only.
    fn motion_slide_in_y(
        self,
        id: impl Into<ElementId>,
        c: Palette,
        distance: Pixels,
    ) -> AnimationElement<Self> {
        let motion = c.motion;
        self.with_animation(
            id,
            c.motion.animation(motion.dur_base, motion.ease_premium),
            move |el, delta| {
                el.opacity(delta)
                    .top(slide_offset(&motion, distance, delta))
            },
        )
    }

    /// Grow the element's width or height from `from` to `to` (clipped), for
    /// docks, sidebars and pane bars. `Reduced`/`Off` snap to `to` and keep
    /// the fade.
    fn motion_reveal(
        self,
        id: impl Into<ElementId>,
        c: Palette,
        axis: Axis,
        from: Pixels,
        to: Pixels,
    ) -> AnimationElement<Self> {
        let motion = c.motion;
        self.with_animation(
            id,
            c.motion.animation(motion.dur_slow, motion.ease_premium),
            move |el, delta| {
                let extent = reveal_extent(&motion, from, to, delta);
                let el = el.overflow_hidden().opacity(delta);
                match axis {
                    Axis::Horizontal => el.w(extent),
                    Axis::Vertical => el.h(extent),
                }
            },
        )
    }

    /// A state layer (selection fill, switch thumb, disclosure glyph) that fades
    /// in whenever `on` is true. The state is part of the animation id, so the
    /// fade restarts on every flip; while `on` is false the layer is hidden.
    /// Crossfade two glyphs by giving one `on` and the other `!on`.
    fn motion_state_fade(
        self,
        id: impl Into<ElementId>,
        c: Palette,
        on: bool,
    ) -> AnimationElement<Self> {
        self.with_animation(
            (id.into(), if on { "on" } else { "off" }),
            c.motion.animation(c.motion.dur_base, c.motion.ease_soft),
            move |el, delta| el.opacity(state_opacity(on, delta)),
        )
    }
}

impl<E: Styled + IntoElement + 'static> MotionExt for E {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_palette, TestTheme};
    use gpui::div;
    use labonair_theme_tokens::Theme;

    fn prefs(level: MotionLevel) -> MotionPrefs {
        MotionPrefs::new(level, &Theme::dark().animation)
    }

    #[test]
    fn duration_floor_never_reaches_zero() {
        for level in [MotionLevel::Full, MotionLevel::Reduced, MotionLevel::Off] {
            assert!(!prefs(level).resolve(Duration::ZERO).is_zero());
        }
        assert_eq!(
            prefs(MotionLevel::Full).resolve(Duration::ZERO),
            MOTION_FLOOR
        );
    }

    #[test]
    fn off_collapses_every_duration_to_the_floor() {
        let m = prefs(MotionLevel::Off);
        assert_eq!(m.resolve(m.dur_fast), MOTION_FLOOR);
        assert_eq!(m.resolve(m.dur_slow), MOTION_FLOOR);
        assert_eq!(m.animation(m.dur_base, m.ease_soft).duration, MOTION_FLOOR);
    }

    #[test]
    fn full_and_reduced_keep_the_theme_durations() {
        for level in [MotionLevel::Full, MotionLevel::Reduced] {
            let m = prefs(level);
            assert_eq!(m.resolve(m.dur_fast), Duration::from_millis(120));
            assert_eq!(m.resolve(m.dur_base), Duration::from_millis(160));
            assert_eq!(m.resolve(m.dur_slow), Duration::from_millis(200));
        }
    }

    #[test]
    fn reduced_fades_only_and_off_is_instant() {
        let (full, reduced, off) = (
            prefs(MotionLevel::Full),
            prefs(MotionLevel::Reduced),
            prefs(MotionLevel::Off),
        );
        assert!(full.animates_geometry() && full.animates_exit());
        assert!(!reduced.animates_geometry() && reduced.animates_exit());
        assert!(!off.animates_geometry() && !off.animates_exit());

        // Slide lerps only at Full; Reduced/Off sit at the end state at once.
        assert_eq!(slide_offset(&full, px(8.0), 0.0), px(8.0));
        assert_eq!(slide_offset(&full, px(8.0), 1.0), px(0.0));
        assert_eq!(slide_offset(&reduced, px(8.0), 0.0), px(0.0));
        assert_eq!(slide_offset(&off, px(8.0), 0.0), px(0.0));

        // Reveal lerps only at Full.
        assert_eq!(reveal_extent(&full, px(0.0), px(200.0), 0.5), px(100.0));
        assert_eq!(reveal_extent(&reduced, px(0.0), px(200.0), 0.0), px(200.0));
        assert_eq!(reveal_extent(&off, px(0.0), px(200.0), 0.0), px(200.0));
    }

    #[test]
    fn state_layer_is_hidden_when_off_and_fades_when_on() {
        assert_eq!(state_opacity(false, 0.7), 0.0);
        assert_eq!(state_opacity(true, 0.0), 0.0);
        assert_eq!(state_opacity(true, 1.0), 1.0);
    }

    #[test]
    fn legacy_reduce_motion_metric_maps_to_off() {
        struct Reduced(Theme);
        impl UiTheme for Reduced {
            fn theme(&self) -> &Theme {
                &self.0
            }
            fn metrics(&self) -> labonair_theme_tokens::ThemeMetrics {
                labonair_theme_tokens::ThemeMetrics {
                    reduce_motion: true,
                    ..Default::default()
                }
            }
        }
        assert_eq!(
            MotionPrefs::from_theme(&Reduced(Theme::dark())).level,
            MotionLevel::Off
        );
        assert_eq!(
            MotionPrefs::from_theme(&TestTheme(Theme::dark())).level,
            MotionLevel::Full
        );
    }

    #[test]
    fn wrappers_build_for_every_level() {
        for level in [MotionLevel::Full, MotionLevel::Reduced, MotionLevel::Off] {
            let mut c = test_palette();
            c.motion.level = level;
            let _ = div().motion_fade_in("fade", c);
            let _ = div().motion_slide_in_y("slide", c, px(6.0));
            let _ = div().motion_reveal("reveal", c, Axis::Vertical, px(0.0), px(40.0));
            let _ = div().motion_state_fade("state", c, true);
        }
    }
}
