//! `SegmentedControl` — a row of mutually exclusive options where exactly one
//! is selected.
//!
//! Port of `reference-src/src/components/ui/toggle-group.tsx` +
//! `tabs.tsx`, which share the two looks this primitive exposes:
//! * [`SegmentVariant::Outline`] — `toggleVariants.outline`:
//!   `border border-input bg-transparent hover:bg-muted`, the pressed segment
//!   keeps the accent border. This is the shape the port already uses.
//! * [`SegmentVariant::Solid`] — `tabsListVariants.default`: a `bg-muted`
//!   container whose active segment lifts to `bg-background`.
//!
//! Replaces two hand-rolled versions: the Installed/Community tab pair
//! (`crates/settings-ui/src/panes/themes.rs::render_theme_tabs`) and the theme
//! variant picker (same file, `render_variant_picker`). The ModelPicker's
//! All/Favorites/Recent strip (`crates/panel-ai/src/panel_ai.rs`) is the same
//! shape and moves over in T20-002.
//!
//! ```ignore
//! segmented_control("theme-tabs", c, selected_key)
//!     .segment("installed", "Installed")
//!     .segment("community", "Community")
//!     .on_select(cx.listener(|this, key: &SharedString, _w, cx| this.pick(key, cx)))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, App, ClickEvent, Div, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window,
};

use crate::palette::Palette;
use crate::DISABLED_OPACITY;

/// Which of the two reference looks the control uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SegmentVariant {
    /// Bordered pills, accent border on the active one.
    #[default]
    Outline,
    /// A muted track whose active segment lifts to the background colour.
    Solid,
}

/// Segment height/typography. Matches [`crate::ButtonSize`]'s naming.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SegmentSize {
    /// 20px — statusbar / dense panel strips.
    Xs,
    /// 24px — the settings and picker strips.
    #[default]
    Sm,
    /// 32px — standalone page-level tabs.
    Md,
}

impl SegmentSize {
    fn height(self) -> f32 {
        match self {
            SegmentSize::Xs => 20.0,
            SegmentSize::Sm => 24.0,
            SegmentSize::Md => 32.0,
        }
    }

    fn text(self) -> f32 {
        match self {
            SegmentSize::Xs => 10.0,
            SegmentSize::Sm => 11.5,
            SegmentSize::Md => 13.0,
        }
    }
}

/// A segmented control. Build with [`segmented_control`].
pub struct SegmentedControl {
    id: SharedString,
    c: Palette,
    selected: SharedString,
    variant: SegmentVariant,
    size: SegmentSize,
    disabled: bool,
    segments: Vec<(SharedString, SharedString)>,
    #[allow(clippy::type_complexity)]
    on_select: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App)>>,
}

/// A [`SegmentedControl`] whose active segment is the one whose key equals
/// `selected`.
pub fn segmented_control(
    id: impl Into<SharedString>,
    c: Palette,
    selected: impl Into<SharedString>,
) -> SegmentedControl {
    SegmentedControl {
        id: id.into(),
        c,
        selected: selected.into(),
        variant: SegmentVariant::default(),
        size: SegmentSize::default(),
        disabled: false,
        segments: Vec::new(),
        on_select: None,
    }
}

impl SegmentedControl {
    /// Append one segment (`key` identifies it, `label` is shown).
    pub fn segment(mut self, key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.segments.push((key.into(), label.into()));
        self
    }

    /// Append many segments at once.
    pub fn segments<K: Into<SharedString>, L: Into<SharedString>>(
        mut self,
        segments: impl IntoIterator<Item = (K, L)>,
    ) -> Self {
        self.segments
            .extend(segments.into_iter().map(|(k, l)| (k.into(), l.into())));
        self
    }

    pub fn variant(mut self, variant: SegmentVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: SegmentSize) -> Self {
        self.size = size;
        self
    }

    /// Dim + inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Fires with the key of the clicked segment.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// The key of the currently active segment, or `None` when `selected`
    /// matches nothing. Lets call sites (and tests) assert the reported
    /// selection without rendering.
    pub fn selection(&self) -> Option<&SharedString> {
        self.segments
            .iter()
            .find(|(k, _)| *k == self.selected)
            .map(|(k, _)| k)
    }
}

impl IntoElement for SegmentedControl {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        let (c, variant, size, disabled) = (self.c, self.variant, self.size, self.disabled);
        let selected = self.selected.clone();
        let handler = self.on_select.clone();
        let group_id = self.id.clone();

        let track = div()
            .flex()
            .flex_row()
            .items_center()
            .when(variant == SegmentVariant::Outline, |d| d.gap(px(6.0)))
            .when(variant == SegmentVariant::Solid, |d| {
                d.gap(px(2.0))
                    .p(px(2.0))
                    .rounded(px(c.radius.lg))
                    .bg(c.muted_bg)
            })
            .when(disabled, |d| d.opacity(DISABLED_OPACITY));

        track.children(self.segments.into_iter().map(move |(key, label)| {
            let on = key == selected;
            let handler = handler.clone();
            let clicked = key.clone();
            div()
                .id(SharedString::from(format!("{group_id}-{key}")))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .h(px(size.height()))
                .px(px(10.0))
                .rounded(px(c.radius.md))
                .text_size(px(size.text()))
                .text_color(if on { c.fg } else { c.muted })
                .when(variant == SegmentVariant::Outline, |d| {
                    d.border_1()
                        .border_color(if on { c.accent } else { c.border })
                })
                .when(variant == SegmentVariant::Solid && on, |d| d.bg(c.bg))
                .when(!disabled, |d| {
                    d.cursor_pointer().hover(move |s| s.bg(c.muted_bg))
                })
                .child(label)
                .when(!disabled, move |d| match handler {
                    Some(h) => d.on_click(move |_: &ClickEvent, w, cx| h(&clicked, w, cx)),
                    None => d,
                })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn reports_the_active_segment() {
        let c = test_palette();
        let sc = segmented_control("tabs", c, "community")
            .segment("installed", "Installed")
            .segment("community", "Community");
        assert_eq!(
            sc.selection().map(SharedString::to_string).as_deref(),
            Some("community")
        );

        // A selection that matches no segment reports nothing rather than
        // silently highlighting the first one.
        let none = segmented_control("tabs", c, "nope").segment("installed", "Installed");
        assert!(none.selection().is_none());
    }

    #[test]
    fn builds_in_every_variant_size_and_state() {
        let c = test_palette();
        for v in [SegmentVariant::Outline, SegmentVariant::Solid] {
            for s in [SegmentSize::Xs, SegmentSize::Sm, SegmentSize::Md] {
                for disabled in [true, false] {
                    let _ = segmented_control("g", c, "a")
                        .segments([("a", "A"), ("b", "B")])
                        .variant(v)
                        .size(s)
                        .disabled(disabled)
                        .on_select(|_, _, _| {})
                        .into_element();
                }
            }
        }
    }
}
