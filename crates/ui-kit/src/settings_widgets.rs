//! Apple-style grouped Settings widgets (R10-002): [`settings_group`],
//! [`settings_row`], [`slider_row`] and [`category_chip`].
//!
//! A group is one rounded card; rows inside it are separated by hairlines and
//! never draw their own border:
//!
//! ```ignore
//! settings_group("appearance-density", c)
//!     .title("Density")
//!     .row(settings_row("density", c, "UI density")
//!         .description("Spacing of rows and controls")
//!         .reset(reset_button)               // optional, left of the control
//!         .control(select_trigger(..)))
//!     .row(slider_row("radius", c, "Corner radius", &state, "12 px"))
//! ```
//!
//! Owners keep every value; these widgets only lay controls out. Geometry goes
//! through [`Palette::control_space`] so UI font size and density scale it.

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, Div, ElementId, Entity, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, Stateful, Styled,
};

use crate::divider::{divider, Axis};
use crate::icon::IconName;
use crate::palette::Palette;
use crate::slider::{slider, SliderState};

/// The minimum height of a settings row, before density scaling.
pub const SETTINGS_ROW_MIN_HEIGHT: f32 = 44.0;

/// One rounded card of settings rows, with an optional caption above it.
pub struct SettingsGroup {
    id: ElementId,
    c: Palette,
    title: Option<SharedString>,
    rows: Vec<AnyElement>,
}

/// Start a settings group card.
pub fn settings_group(id: impl Into<ElementId>, c: Palette) -> SettingsGroup {
    SettingsGroup {
        id: id.into(),
        c,
        title: None,
        rows: Vec::new(),
    }
}

impl SettingsGroup {
    /// A small caption above the card.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Append one row (usually a [`SettingsRow`]); hairlines are inserted
    /// between rows automatically.
    pub fn row(mut self, row: impl IntoElement) -> Self {
        self.rows.push(row.into_any_element());
        self
    }
}

impl ParentElement for SettingsGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.rows.extend(elements);
    }
}

impl IntoElement for SettingsGroup {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let Self { id, c, title, rows } = self;
        let mut card = div()
            .flex()
            .flex_col()
            .w_full()
            .overflow_hidden()
            .rounded(px(c.radius.lg))
            .bg(c.card)
            .border_1()
            .border_color(c.line_subtle);
        for (index, row) in rows.into_iter().enumerate() {
            if index > 0 {
                card = card.child(divider(Axis::Horizontal, c.line_subtle));
            }
            card = card.child(row);
        }
        div()
            .id(id)
            .flex()
            .flex_col()
            .w_full()
            .gap(c.space(8.0))
            .when_some(title, |group, title| {
                group.child(
                    div()
                        .px(c.control_space(4.0))
                        .text_size(c.control_space(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.muted)
                        .child(title),
                )
            })
            .child(card)
    }
}

/// One settings row: title and description on the left, control on the right.
pub struct SettingsRow {
    id: ElementId,
    c: Palette,
    title: SharedString,
    description: Option<SharedString>,
    extra: Option<AnyElement>,
    reset: Option<AnyElement>,
    control: Option<AnyElement>,
}

/// Start a settings row titled `title`.
pub fn settings_row(
    id: impl Into<ElementId>,
    c: Palette,
    title: impl Into<SharedString>,
) -> SettingsRow {
    SettingsRow {
        id: id.into(),
        c,
        title: title.into(),
        description: None,
        extra: None,
        reset: None,
        control: None,
    }
}

impl SettingsRow {
    /// A muted explanation under the title.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Extra left-column content under the description (a hint, a "modified
    /// in" note, an unavailable-in-scope explanation).
    pub fn extra(mut self, extra: impl IntoElement) -> Self {
        self.extra = Some(extra.into_any_element());
        self
    }

    /// The right-aligned control (switch, select, segmented control, ...).
    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }

    /// An optional "reset to default" affordance placed left of the control.
    pub fn reset(mut self, reset: impl IntoElement) -> Self {
        self.reset = Some(reset.into_any_element());
        self
    }
}

impl IntoElement for SettingsRow {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let Self {
            id,
            c,
            title,
            description,
            extra,
            reset,
            control,
        } = self;
        div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(c.space(16.0))
            .w_full()
            .min_h(c.control_space(SETTINGS_ROW_MIN_HEIGHT))
            .px(c.control_space(16.0))
            .py(c.control_space(10.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(c.space(2.0))
                    .child(
                        div()
                            .text_size(c.control_space(13.0))
                            .text_color(c.fg)
                            .child(title),
                    )
                    .when_some(description, |column, description| {
                        column.child(
                            div()
                                .text_size(c.control_space(12.0))
                                .text_color(c.muted)
                                .child(description),
                        )
                    })
                    .children(extra),
            )
            .when(reset.is_some() || control.is_some(), |row| {
                row.child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_shrink_0()
                        .items_center()
                        .gap(c.space(8.0))
                        .children(reset)
                        .children(control),
                )
            })
    }
}

/// Width of the slider track in a [`slider_row`], before density scaling.
const SLIDER_ROW_TRACK: f32 = 180.0;
/// Width reserved for the value read-out so the track does not jitter.
const SLIDER_ROW_VALUE: f32 = 48.0;

/// A settings row whose control is a slider with a value read-out. The owner
/// keeps the [`SliderState`] and formats `value_text` (for example `"12 px"`).
pub fn slider_row(
    id: impl Into<ElementId>,
    c: Palette,
    label: impl Into<SharedString>,
    state: &Entity<SliderState>,
    value_text: impl Into<SharedString>,
) -> SettingsRow {
    settings_row(id, c, label).control(slider_control(state, c, value_text, false))
}

/// The control half of a [`slider_row`]: slider track plus a fixed-width value
/// read-out. `disabled` makes the track inert (the owner also dims the row).
pub fn slider_control(
    state: &Entity<SliderState>,
    c: Palette,
    value_text: impl Into<SharedString>,
    disabled: bool,
) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(c.space(12.0))
        .child(
            slider(state, c)
                .disabled(disabled)
                .w(c.space(SLIDER_ROW_TRACK)),
        )
        .child(
            div()
                .w(c.control_space(SLIDER_ROW_VALUE))
                .flex_shrink_0()
                .flex()
                .justify_end()
                .text_size(c.control_space(12.0))
                .text_color(c.muted)
                .child(value_text.into()),
        )
}

/// The hue of a [`category_chip`], in degrees on the colour wheel. The chip
/// derives its fill from the hue and the light/dark mode, so one token reads
/// well on both.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChipHue(pub f32);

impl ChipHue {
    pub const RED: Self = Self(2.0);
    pub const ORANGE: Self = Self(26.0);
    pub const AMBER: Self = Self(42.0);
    pub const GREEN: Self = Self(140.0);
    pub const TEAL: Self = Self(175.0);
    pub const BLUE: Self = Self(215.0);
    pub const INDIGO: Self = Self(250.0);
    pub const PURPLE: Self = Self(280.0);
    pub const PINK: Self = Self(330.0);
    /// Neutral grey chip (no hue).
    pub const GRAY: Self = Self(f32::NAN);

    /// The chip fill for `hue`. Lightness is held low enough that the white
    /// glyph keeps at least 3:1 contrast on every named hue in both modes.
    pub fn fill(self, is_dark: bool) -> Hsla {
        let lightness = if is_dark { 0.38 } else { 0.36 };
        if self.0.is_nan() {
            return Hsla {
                h: 0.0,
                s: 0.0,
                l: lightness,
                a: 1.0,
            };
        }
        Hsla {
            h: self.0.rem_euclid(360.0) / 360.0,
            s: 0.62,
            l: lightness,
            a: 1.0,
        }
    }
}

/// Edge length of a [`category_chip`], before density scaling.
const CHIP_SIZE: f32 = 22.0;

/// The 22px rounded colour badge in front of a Settings category, with a white
/// glyph (white on the saturated fill is deliberate: the fill is chosen for it).
pub fn category_chip(c: Palette, hue: ChipHue, glyph: IconName) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(c.control_space(CHIP_SIZE))
        // About 6px at the default radius scale; follows the corner-radius metric.
        .rounded(px(c.radius.md * 0.75))
        .bg(hue.fill(c.is_dark))
        .child(glyph.svg(gpui::white()).size(c.control_space(14.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;
    use gpui::{AppContext, TestAppContext};
    use labonair_theme_tokens::contrast_ratio;

    const NAMED: [ChipHue; 10] = [
        ChipHue::RED,
        ChipHue::ORANGE,
        ChipHue::AMBER,
        ChipHue::GREEN,
        ChipHue::TEAL,
        ChipHue::BLUE,
        ChipHue::INDIGO,
        ChipHue::PURPLE,
        ChipHue::PINK,
        ChipHue::GRAY,
    ];

    #[test]
    fn chip_glyph_stays_readable_in_light_and_dark() {
        for hue in NAMED {
            for is_dark in [false, true] {
                let ratio = contrast_ratio(gpui::white(), hue.fill(is_dark));
                assert!(
                    ratio >= 3.0,
                    "white on {hue:?} (dark={is_dark}) has contrast {ratio:.2}"
                );
            }
        }
    }

    #[test]
    fn group_builds_with_and_without_title_and_rows() {
        let c = test_palette();
        let _ = settings_group("empty", c).into_element();
        let _ = settings_group("g", c)
            .title("Density")
            .row(settings_row("a", c, "A").description("About A"))
            .row(
                settings_row("b", c, "B")
                    .extra(div().child("hint"))
                    .reset(div().child("reset"))
                    .control(div().child("ctl")),
            )
            .row(settings_row("c", c, "C"))
            .into_element();
    }

    #[test]
    fn chip_builds_for_each_mode() {
        for is_dark in [false, true] {
            let mut c = test_palette();
            c.is_dark = is_dark;
            for hue in NAMED {
                let _ = category_chip(c, hue, IconName::Settings);
            }
        }
    }

    #[gpui::test]
    fn slider_row_builds(cx: &mut TestAppContext) {
        let state = cx.new(|_| SliderState::new().min(0.0).max(100.0).default_value(10.0));
        let _ = slider_row("s", test_palette(), "Opacity", &state, "10 %").into_element();
    }
}
