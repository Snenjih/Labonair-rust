//! Token-bound styling for the native GPUI Component slider.
//!
//! Slider state remains owned by the feature. This adapter only applies the
//! shared Labonair color and sizing tokens to the interaction control.

use gpui::{Entity, Styled};
use gpui_component::slider::Slider;

use crate::palette::Palette;

pub use gpui_component::slider::{SliderEvent, SliderState, SliderValue};

/// A horizontal single-value slider styled from the shared palette.
pub fn slider(state: &Entity<SliderState>, c: Palette) -> Slider {
    Slider::new(state)
        .horizontal()
        .h(c.space(28.0))
        .bg(c.primary)
        .text_color(c.primary)
}
