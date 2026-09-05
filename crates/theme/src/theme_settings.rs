//! T20-007: the runtime **metric** layer that scales the active [`Theme`].
//!
//! Colour comes from the [`ThemeRegistry`](crate::ThemeRegistry) via
//! [`ThemeStore`](crate::ThemeStore); *metric* (font scales, UI density, corner
//! radius, reduce-motion) comes from the user's theme-settings. [`ActiveTheme`]
//! is the combined view the whole app reads — analogous to Zed's
//! `theme_settings` layer (`zed-refrence/zed/crates/theme/src/ui_density.rs`,
//! `theme_settings.rs`).
//!
//! The metric half is [`ThemeMetrics`]; the density scalar set is [`UiDensity`].
//! [`ThemeStore`] owns the live [`ThemeMetrics`] (`set_metrics`) and rebuilds
//! its cached [`ActiveTheme`] on every colour **or** metric change; the same
//! value is mirrored into the [`GlobalActiveTheme`] global by an observer
//! installed in [`crate::init_theme`], so `App`-level code can read it through
//! [`labonair_ui_kit::ActiveThemeExt`](../../labonair_ui_kit) without touching
//! the store entity.

use std::time::Duration;

use crate::{Animation, RadiusScale, Theme, Typography};

/// UI density — a spacing/size multiplier applied around the layout-contract
/// base metrics. Port of Zed's `UiDensity` (`crates/theme/src/ui_density.rs`).
///
/// The factors are deliberately conservative (`docs/architecture.md` §8.20):
/// `Compact` ≈ ×0.85, `Default` ×1.0, `Comfortable` ×1.15. They scale *around*
/// the layout-contract heights (40px titlebar / 32px statusbar stay the
/// reference), never replace them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiDensity {
    Compact,
    #[default]
    Default,
    Comfortable,
}

impl UiDensity {
    /// The spacing multiplier for this density.
    pub fn spacing_scale(self) -> f32 {
        match self {
            UiDensity::Compact => 0.85,
            UiDensity::Default => 1.0,
            UiDensity::Comfortable => 1.15,
        }
    }

    /// The stable settings token (`appearance.uiDensity`).
    pub fn as_str(self) -> &'static str {
        match self {
            UiDensity::Compact => "compact",
            UiDensity::Default => "default",
            UiDensity::Comfortable => "comfortable",
        }
    }

    /// Parse the settings token; unknown / empty falls back to [`UiDensity::Default`].
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "compact" => UiDensity::Compact,
            "comfortable" => UiDensity::Comfortable,
            _ => UiDensity::Default,
        }
    }

    pub const ALL: [UiDensity; 3] = [
        UiDensity::Compact,
        UiDensity::Default,
        UiDensity::Comfortable,
    ];
}

/// The resolved metric bundle — the "settings" half of [`ActiveTheme`].
///
/// Built from the user's theme-settings by the settings → `ThemeStore` bridge
/// (`labonair_settings_ui::apply_prefs_to_theme`). Defaults reproduce the
/// historical [`Typography`] defaults + `UiDensity::Default` + unit radius
/// scale + motion on, so a store that never receives a `set_metrics` call
/// renders exactly as it did before T20-007.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeMetrics {
    /// UI-chrome font family (empty = the theme's own `app_font_family`).
    pub ui_font_family: String,
    /// UI-chrome font size, px.
    pub ui_font_size: f32,
    /// UI-chrome line-height multiple.
    pub ui_line_height: f32,
    /// Editor/terminal text font family (empty = the theme's own mono family).
    pub buffer_font_family: String,
    /// Editor/terminal text font size, px.
    pub buffer_font_size: f32,
    /// Editor/terminal text line-height multiple.
    pub buffer_line_height: f32,
    /// Spacing/size density.
    pub density: UiDensity,
    /// Multiplier applied to the theme's [`RadiusScale`] (1.0 = unchanged).
    pub corner_radius_scale: f32,
    /// When `true`, [`ActiveTheme::animation`] reports zero durations.
    pub reduce_motion: bool,
}

impl Default for ThemeMetrics {
    fn default() -> Self {
        let ty = Typography::default();
        Self {
            ui_font_family: String::new(),
            ui_font_size: ty.app_font_size,
            ui_line_height: ty.app_line_height,
            buffer_font_family: String::new(),
            buffer_font_size: ty.buffer_font_size,
            buffer_line_height: ty.app_line_height,
            density: UiDensity::Default,
            corner_radius_scale: 1.0,
            reduce_motion: false,
        }
    }
}

impl ThemeMetrics {
    /// A spacing value (px) scaled by the active density.
    pub fn space(&self, px: f32) -> f32 {
        px * self.density.spacing_scale()
    }

    /// `base` after applying [`Self::corner_radius_scale`].
    pub fn scaled_radius(&self, base: RadiusScale) -> RadiusScale {
        base.scaled(self.corner_radius_scale)
    }
}

/// Colour (from the [`ThemeRegistry`](crate::ThemeRegistry)) **plus** metric
/// (from [`ThemeMetrics`]) — the single combined view the app renders from.
///
/// Cheap to `clone` (`colors` is a plain `Theme`, ~a few hundred bytes; kept
/// by value rather than `Arc` so `ActiveTheme` stays `Send + 'static` with no
/// atomic traffic). Rebuilt only on a real colour or metric change — never per
/// frame (see the T20-007 `## Warnungen`).
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTheme {
    colors: Theme,
    metrics: ThemeMetrics,
    /// `colors.radius` after `metrics.corner_radius_scale` — cached.
    radius: RadiusScale,
    /// `colors.animation`, durations zeroed when `metrics.reduce_motion` — cached.
    animation: Animation,
}

impl ActiveTheme {
    /// Combine resolved colours with resolved metrics.
    pub fn new(colors: Theme, metrics: ThemeMetrics) -> Self {
        let radius = metrics.scaled_radius(colors.radius);
        let mut animation = colors.animation;
        if metrics.reduce_motion {
            animation.dur_fast = Duration::ZERO;
            animation.dur_base = Duration::ZERO;
            animation.dur_slow = Duration::ZERO;
        }
        Self {
            colors,
            metrics,
            radius,
            animation,
        }
    }

    /// The resolved colour tokens.
    pub fn theme(&self) -> &Theme {
        &self.colors
    }

    /// The resolved metrics.
    pub fn metrics(&self) -> &ThemeMetrics {
        &self.metrics
    }

    /// The density-and-corner-radius-scaled [`RadiusScale`] (use this, not
    /// `theme().radius`, for any rounded corner).
    pub fn radius(&self) -> RadiusScale {
        self.radius
    }

    /// Animation timings — zero durations when reduce-motion is on.
    pub fn animation(&self) -> &Animation {
        &self.animation
    }

    /// The active density's spacing multiplier.
    pub fn density_scale(&self) -> f32 {
        self.metrics.density.spacing_scale()
    }

    /// A spacing value (px) scaled by the active density.
    pub fn space(&self, px: f32) -> f32 {
        self.metrics.space(px)
    }

    pub fn ui_font_size(&self) -> f32 {
        self.metrics.ui_font_size
    }

    pub fn ui_line_height(&self) -> f32 {
        self.metrics.ui_line_height
    }

    pub fn buffer_font_size(&self) -> f32 {
        self.metrics.buffer_font_size
    }

    pub fn buffer_line_height(&self) -> f32 {
        self.metrics.buffer_line_height
    }

    pub fn reduce_motion(&self) -> bool {
        self.metrics.reduce_motion
    }
}

/// App-wide handle to the current [`ActiveTheme`]. Kept in sync with the
/// [`ThemeStore`](crate::ThemeStore)'s cached value by an observer installed in
/// [`crate::init_theme`].
pub struct GlobalActiveTheme(pub ActiveTheme);

impl gpui::Global for GlobalActiveTheme {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_factors_are_conservative_and_ordered() {
        assert!(UiDensity::Compact.spacing_scale() < 1.0);
        assert_eq!(UiDensity::Default.spacing_scale(), 1.0);
        assert!(UiDensity::Comfortable.spacing_scale() > 1.0);
        // "Conservative" — within ±15% of the base.
        for d in UiDensity::ALL {
            assert!((d.spacing_scale() - 1.0).abs() <= 0.15 + 1e-6);
        }
        assert_eq!(
            UiDensity::from_str_or_default("compact"),
            UiDensity::Compact
        );
        assert_eq!(UiDensity::from_str_or_default("bogus"), UiDensity::Default);
    }

    #[test]
    fn density_scales_spacing() {
        let compact = ThemeMetrics {
            density: UiDensity::Compact,
            ..ThemeMetrics::default()
        };
        assert!((compact.space(40.0) - 34.0).abs() < 1e-4);
        let comfy = ThemeMetrics {
            density: UiDensity::Comfortable,
            ..ThemeMetrics::default()
        };
        assert!((comfy.space(40.0) - 46.0).abs() < 1e-4);
    }

    #[test]
    fn corner_radius_scale_multiplies_the_radius_scale() {
        let m = ThemeMetrics {
            corner_radius_scale: 2.0,
            ..ThemeMetrics::default()
        };
        let base = Theme::dark().radius;
        let scaled = m.scaled_radius(base);
        assert!((scaled.md - base.md * 2.0).abs() < 1e-4);
        // `window` stays fixed.
        assert_eq!(scaled.window, base.window);
    }

    #[test]
    fn reduce_motion_zeroes_animation_durations() {
        let colors = Theme::dark();
        let normal = ActiveTheme::new(colors.clone(), ThemeMetrics::default());
        assert!(normal.animation().dur_base > Duration::ZERO);

        let reduced = ActiveTheme::new(
            colors,
            ThemeMetrics {
                reduce_motion: true,
                ..ThemeMetrics::default()
            },
        );
        assert_eq!(reduced.animation().dur_fast, Duration::ZERO);
        assert_eq!(reduced.animation().dur_base, Duration::ZERO);
        assert_eq!(reduced.animation().dur_slow, Duration::ZERO);
    }

    #[test]
    fn default_metrics_match_the_historical_typography_defaults() {
        let m = ThemeMetrics::default();
        let ty = Typography::default();
        assert_eq!(m.ui_font_size, ty.app_font_size);
        assert_eq!(m.ui_line_height, ty.app_line_height);
        assert_eq!(m.buffer_font_size, ty.buffer_font_size);
        assert_eq!(m.corner_radius_scale, 1.0);
        assert_eq!(m.density, UiDensity::Default);
        assert!(!m.reduce_motion);
    }
}
