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

use std::time::{Duration, Instant};

use gpui::{
    px, Animation, AnimationElement, AnimationExt, AnyElement, App, Element, ElementId,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Styled, Window,
};
use labonair_theme_tokens::{
    Animation as AnimationTokens, AnimationLevel, CubicBezier, GlobalActiveTheme,
};

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

    /// The motion tokens of the app-wide active theme, for elements that are
    /// built without a [`Palette`] (disclosure chevrons, tree rows). Falls back
    /// to [`MotionLevel::Off`] when no theme is installed (unit tests), so
    /// nothing animates by accident.
    pub fn from_app(cx: &App) -> Self {
        match cx.try_global::<GlobalActiveTheme>() {
            Some(GlobalActiveTheme(active)) => Self::new(
                level_for(active.metrics().animations),
                &active.theme().animation,
            ),
            None => Self::new(MotionLevel::Off, &AnimationTokens::default()),
        }
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
/// This is the **single place** that decides the level: it reads the resolved
/// `appearance.animations` choice ([`AnimationLevel`]) off the theme metrics.
fn motion_level(theme: &impl UiTheme) -> MotionLevel {
    level_for(theme.metrics().animations)
}

/// The kit [`MotionLevel`] for a resolved theme [`AnimationLevel`].
pub fn level_for(level: AnimationLevel) -> MotionLevel {
    match level {
        AnimationLevel::Full => MotionLevel::Full,
        AnimationLevel::Reduced => MotionLevel::Reduced,
        AnimationLevel::Off => MotionLevel::Off,
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

    /// A state layer (selection fill, check mark, disclosure glyph) whose
    /// opacity follows `on`: it fades in when `on` turns true and fades out
    /// when it turns false. Unlike an id-keyed [`AnimationElement`] it
    /// remembers the previous state in element state, so a layer that is
    /// *first mounted* already in its end state does not animate, and a flip
    /// mid-fade continues from the current opacity. Build the layer
    /// unconditionally (do not wrap it in `when(on, ..)`), otherwise there is
    /// nothing left to fade out.
    fn motion_state_fade(
        self,
        id: impl Into<ElementId>,
        c: Palette,
        on: bool,
    ) -> StateTransition<Self> {
        StateTransition::new(self, id.into(), on, false, apply_fade).with_motion(c.motion)
    }

    /// The general form of [`MotionExt::motion_state_fade`]: `apply` receives
    /// the element and the current amount (`0.0` off .. `1.0` on) and styles
    /// it, e.g. a switch thumb's `left` offset. Set `geometry` when `apply`
    /// moves or resizes the element: `Reduced` and `Off` then snap to the end
    /// state instead of animating.
    fn motion_state(
        self,
        id: impl Into<ElementId>,
        c: Palette,
        on: bool,
        geometry: bool,
        apply: impl Fn(Self, f32) -> Self + 'static,
    ) -> StateTransition<Self> {
        StateTransition::new(self, id.into(), on, geometry, apply).with_motion(c.motion)
    }
}

/// [`MotionExt::motion_state_fade`] for call sites without a [`Palette`]: the
/// motion tokens are read from the active theme at layout time.
pub fn state_fade_ambient<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<ElementId>,
    on: bool,
) -> StateTransition<E> {
    StateTransition::new(el, id.into(), on, false, apply_fade)
}

/// An absolutely positioned, non-interactive fill (`inset` px beyond the
/// parent's padding box on every side) that fades with `on`: the selected-row,
/// active-tab and checked-box backgrounds. Place it as the **first** child of a
/// positioned parent so it paints behind the content.
pub(crate) fn state_fill(
    id: impl Into<ElementId>,
    c: Palette,
    on: bool,
    color: gpui::Hsla,
    border: Option<gpui::Hsla>,
    radius: Pixels,
    inset: Pixels,
) -> StateTransition<gpui::Div> {
    use gpui::prelude::FluentBuilder;
    gpui::div()
        .absolute()
        .top(-inset)
        .right(-inset)
        .bottom(-inset)
        .left(-inset)
        .rounded(radius)
        .bg(color)
        .when_some(border, |d, border| d.border_1().border_color(border))
        .motion_state_fade(id, c, on)
}

/// Opacity of a state layer: untouched once fully on (no needless group
/// opacity layer), faded otherwise.
fn apply_fade<E: Styled>(el: E, amount: f32) -> E {
    if amount >= 1.0 {
        el
    } else {
        el.opacity(amount)
    }
}

/// The animated amount (`0.0` off .. `1.0` on) of one state layer: a tween
/// from where the layer was when it last flipped towards its current target.
/// Pure and clock-injected so the level rules are unit-testable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tween {
    from: f32,
    target: f32,
    start: Option<Instant>,
}

impl Tween {
    /// A layer at rest in the `on` state (first mount: nothing animates).
    pub(crate) fn settled(on: bool) -> Self {
        let target = if on { 1.0 } else { 0.0 };
        Self {
            from: target,
            target,
            start: None,
        }
    }

    fn value(&self, now: Instant, dur: Duration, ease: &CubicBezier) -> f32 {
        match self.start {
            None => self.target,
            Some(start) => {
                let t = (now.saturating_duration_since(start).as_secs_f32() / dur.as_secs_f32())
                    .clamp(0.0, 1.0);
                self.from + (self.target - self.from) * ease.eval(t)
            }
        }
    }

    /// Advance to `now` with the layer wanting state `on`; returns the amount
    /// to draw and whether another frame is needed. `geometry` layers snap
    /// unless the level is `Full`; every layer snaps when the level is `Off`.
    pub(crate) fn advance(
        &mut self,
        on: bool,
        now: Instant,
        motion: &MotionPrefs,
        geometry: bool,
    ) -> (f32, bool) {
        let target = if on { 1.0 } else { 0.0 };
        if motion.level == MotionLevel::Off || (geometry && !motion.animates_geometry()) {
            *self = Self::settled(on);
            return (target, false);
        }
        let dur = motion.resolve(motion.dur_base);
        let ease = motion.ease_soft;
        if target != self.target {
            self.from = self.value(now, dur, &ease);
            self.target = target;
            self.start = Some(now);
        }
        match self.start {
            Some(start) if now.saturating_duration_since(start) < dur => {
                (self.value(now, dur, &ease), true)
            }
            _ => {
                *self = Self::settled(on);
                (target, false)
            }
        }
    }
}

/// An element whose style follows a boolean state with a tween. Built through
/// [`MotionExt::motion_state_fade`], [`MotionExt::motion_state`] or
/// [`state_fade_ambient`]; it adds no layout node of its own.
pub struct StateTransition<E> {
    id: ElementId,
    element: Option<E>,
    on: bool,
    geometry: bool,
    motion: Option<MotionPrefs>,
    apply: Box<dyn Fn(E, f32) -> E + 'static>,
}

impl<E> StateTransition<E> {
    fn new(
        element: E,
        id: ElementId,
        on: bool,
        geometry: bool,
        apply: impl Fn(E, f32) -> E + 'static,
    ) -> Self {
        Self {
            id,
            element: Some(element),
            on,
            geometry,
            motion: None,
            apply: Box::new(apply),
        }
    }

    fn with_motion(mut self, motion: MotionPrefs) -> Self {
        self.motion = Some(motion);
        self
    }
}

impl<E: IntoElement + 'static> IntoElement for StateTransition<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<E: IntoElement + 'static> Element for StateTransition<E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let motion = self.motion.unwrap_or_else(|| MotionPrefs::from_app(cx));
        let (on, geometry) = (self.on, self.geometry);
        let (amount, running) = match global_id {
            Some(global_id) => window.with_element_state(global_id, |state, _window| {
                let mut tween = state.unwrap_or_else(|| Tween::settled(on));
                let step = tween.advance(on, Instant::now(), &motion, geometry);
                (step, tween)
            }),
            None => (
                Tween::settled(on)
                    .advance(on, Instant::now(), &motion, geometry)
                    .0,
                false,
            ),
        };
        if running {
            window.request_animation_frame();
        }
        let element = self.element.take().expect("should only be called once");
        let mut element = (self.apply)(element, amount).into_any_element();
        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: gpui::Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
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
    fn animations_metric_maps_to_the_motion_level() {
        struct Metered(Theme, AnimationLevel);
        impl UiTheme for Metered {
            fn theme(&self) -> &Theme {
                &self.0
            }
            fn metrics(&self) -> labonair_theme_tokens::ThemeMetrics {
                labonair_theme_tokens::ThemeMetrics {
                    animations: self.1,
                    reduce_motion: self.1 != AnimationLevel::Full,
                    ..Default::default()
                }
            }
        }
        for (animations, level) in [
            (AnimationLevel::Full, MotionLevel::Full),
            (AnimationLevel::Reduced, MotionLevel::Reduced),
            (AnimationLevel::Off, MotionLevel::Off),
        ] {
            assert_eq!(
                MotionPrefs::from_theme(&Metered(Theme::dark(), animations)).level,
                level
            );
        }
        assert_eq!(
            MotionPrefs::from_theme(&TestTheme(Theme::dark())).level,
            MotionLevel::Full
        );
    }

    fn motion(level: MotionLevel) -> MotionPrefs {
        prefs(level)
    }

    #[test]
    fn first_mount_and_steady_state_never_animate() {
        let now = Instant::now();
        for on in [true, false] {
            let mut tween = Tween::settled(on);
            let (amount, running) = tween.advance(on, now, &motion(MotionLevel::Full), false);
            assert_eq!(amount, if on { 1.0 } else { 0.0 });
            assert!(!running);
        }
    }

    #[test]
    fn a_flip_tweens_and_then_settles() {
        let m = motion(MotionLevel::Full);
        let t0 = Instant::now();
        let mut tween = Tween::settled(false);
        // Flip on: starts from 0, runs, settles at 1 after dur_base.
        let (a0, run0) = tween.advance(true, t0, &m, false);
        assert!(a0 < 0.05 && run0);
        let (mid, run_mid) = tween.advance(true, t0 + m.dur_base / 2, &m, false);
        assert!(mid > 0.0 && mid < 1.0 && run_mid);
        let (end, run_end) = tween.advance(true, t0 + m.dur_base, &m, false);
        assert_eq!((end, run_end), (1.0, false));
    }

    #[test]
    fn a_flip_back_continues_from_the_current_amount() {
        let m = motion(MotionLevel::Full);
        let t0 = Instant::now();
        let mut tween = Tween::settled(false);
        tween.advance(true, t0, &m, false);
        let (up, _) = tween.advance(true, t0 + m.dur_base / 2, &m, false);
        let (back, running) = tween.advance(false, t0 + m.dur_base / 2, &m, false);
        assert!(running);
        assert!(
            (back - up).abs() < 1e-3,
            "no jump on reversal: {up} vs {back}"
        );
    }

    #[test]
    fn off_snaps_everything_and_reduced_snaps_only_geometry() {
        let t0 = Instant::now();
        for geometry in [false, true] {
            let mut tween = Tween::settled(false);
            let step = tween.advance(true, t0, &motion(MotionLevel::Off), geometry);
            assert_eq!(step, (1.0, false), "Off is instant (geometry={geometry})");
        }
        // Reduced: fades still run, geometry snaps (never creates a tween).
        let mut fade = Tween::settled(false);
        assert!(
            fade.advance(true, t0, &motion(MotionLevel::Reduced), false)
                .1
        );
        let mut geo = Tween::settled(false);
        assert_eq!(
            geo.advance(true, t0, &motion(MotionLevel::Reduced), true),
            (1.0, false)
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
