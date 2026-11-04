//! Pane chrome: the generic [`pane_bar`] header and the [`pane_frame`] around a
//! pane (R10-002, ADR 0005).
//!
//! This module knows nothing about tabs, terminals or editors. The workspace
//! owns tiling, focus, zoom and drag-and-drop; it composes these two pieces per
//! pane and wires the callbacks.
//!
//! ```ignore
//! let bar = pane_bar("bar-3", c, IconName::Terminal, "zsh")
//!     .layout(style)
//!     .active(focused)
//!     .trailing(cwd_label)
//!     .split_horizontal(PaneAction::new("Split right").on_click(..))
//!     .split_vertical(PaneAction::new("Split down").on_click(..))
//!     .zoom(PaneAction::new("Zoom pane").on_click(..))
//!     .zoomed(is_zoomed)
//!     .close(PaneAction::new("Close pane").on_click(..))
//!     .on_double_click(|w, cx| toggle_zoom(w, cx))
//!     .render()                       // Stateful<Div>: keep chaining
//!     .on_drag(DraggedPane { .. }, |d, _, _, cx| cx.new(|_| ghost(d)));
//! let pane = pane_frame("pane-3", c, style, focused).child(bar).child(body);
//! ```
//!
//! The whole bar is the drag handle: `render` returns a `Stateful<Div>` so the
//! caller attaches `on_drag` itself (the payload type lives in the workspace).
//! Action buttons swallow mouse-down so pressing one never starts a drag.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, Div, ElementId, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window,
};

use crate::button::{ButtonSize, ButtonVariant};
use crate::density::Density;
use crate::icon::IconName;
use crate::icon_button::{icon_button_builder, IconButtonShape};
use crate::palette::Palette;

/// The gap between panes in the comfortable layout, in logical pixels before
/// density scaling.
pub const PANE_GAP: f32 = 6.0;

/// The `appearance.paneLayout` choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PaneLayoutStyle {
    /// Floating, rounded panes separated by [`PANE_GAP`].
    #[default]
    Comfortable,
    /// Flush panes without rounding or gap; dividers belong to the tiling
    /// container's resize handles.
    Compact,
}

impl PaneLayoutStyle {
    /// The gap between neighbouring panes: [`PANE_GAP`] scaled by density when
    /// comfortable, zero when compact.
    pub fn gap(self, c: &Palette) -> Pixels {
        match self {
            Self::Comfortable => c.space(PANE_GAP),
            Self::Compact => px(0.0),
        }
    }

    /// Whether panes draw their own rounded, ringed frame.
    pub fn is_framed(self) -> bool {
        self == Self::Comfortable
    }
}

/// The frame around one pane: comfortable panes get a `radius.lg` corner, a
/// 1px `line_subtle` ring (`line_strong` while `focused`) and the main fill;
/// compact panes only get the fill. The caller sizes it and adds the bar and
/// body as children.
pub fn pane_frame(
    id: impl Into<ElementId>,
    c: Palette,
    style: PaneLayoutStyle,
    focused: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .bg(c.bg)
        .when(style.is_framed(), |frame| {
            frame
                .rounded(px(c.radius.lg))
                .border_1()
                .border_color(if focused {
                    c.line_strong
                } else {
                    c.line_subtle
                })
        })
}

type PaneCallback = Rc<dyn Fn(&mut Window, &mut App)>;

/// One pane-bar action: tooltip, enabled flag and callback.
pub struct PaneAction {
    tooltip: SharedString,
    enabled: bool,
    on_click: Option<PaneCallback>,
}

impl PaneAction {
    /// An enabled action with `tooltip`. Without a callback it renders disabled.
    pub fn new(tooltip: impl Into<SharedString>) -> Self {
        Self {
            tooltip: tooltip.into(),
            enabled: true,
            on_click: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn is_active(&self) -> bool {
        self.enabled && self.on_click.is_some()
    }
}

/// Tooltip shown on the split actions of a pane that cannot be split.
const CANNOT_SPLIT: &str = "This pane cannot be split";

/// The pane header. Build with [`pane_bar`], finish with [`PaneBar::render`].
pub struct PaneBar {
    id: SharedString,
    c: Palette,
    icon: IconName,
    title: SharedString,
    leading: Option<AnyElement>,
    trailing: Option<AnyElement>,
    active: bool,
    layout: PaneLayoutStyle,
    can_split: bool,
    split_horizontal: Option<PaneAction>,
    split_vertical: Option<PaneAction>,
    zoom: Option<PaneAction>,
    zoomed: bool,
    close: Option<PaneAction>,
    double_click: Option<PaneCallback>,
}

/// Start a pane bar with the pane's `icon` and `title`.
pub fn pane_bar(
    id: impl Into<SharedString>,
    c: Palette,
    icon: IconName,
    title: impl Into<SharedString>,
) -> PaneBar {
    PaneBar {
        id: id.into(),
        c,
        icon,
        title: title.into(),
        leading: None,
        trailing: None,
        active: false,
        layout: PaneLayoutStyle::Comfortable,
        can_split: true,
        split_horizontal: None,
        split_vertical: None,
        zoom: None,
        zoomed: false,
        close: None,
        double_click: None,
    }
}

impl PaneBar {
    /// A slot between the icon and the title (status dot, dirty marker).
    pub fn leading(mut self, content: impl IntoElement) -> Self {
        self.leading = Some(content.into_any_element());
        self
    }

    /// A slot after the title that takes the remaining width (path, cwd, host).
    pub fn trailing(mut self, content: impl IntoElement) -> Self {
        self.trailing = Some(content.into_any_element());
        self
    }

    /// Whether this is the focused pane: sunken fill, foreground title.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Round the top corners to sit inside a comfortable [`pane_frame`].
    pub fn layout(mut self, layout: PaneLayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    /// `false` disables both split actions and swaps their tooltip for an
    /// explanation (singleton pane kinds).
    pub fn can_split(mut self, can_split: bool) -> Self {
        self.can_split = can_split;
        self
    }

    /// Split so the new pane sits to the right (side by side).
    pub fn split_horizontal(mut self, action: PaneAction) -> Self {
        self.split_horizontal = Some(action);
        self
    }

    /// Split so the new pane sits below (stacked).
    pub fn split_vertical(mut self, action: PaneAction) -> Self {
        self.split_vertical = Some(action);
        self
    }

    /// The zoom toggle; pair with [`PaneBar::zoomed`] for its pressed state.
    pub fn zoom(mut self, action: PaneAction) -> Self {
        self.zoom = Some(action);
        self
    }

    /// Whether the pane is currently zoomed (selected state, restore glyph).
    pub fn zoomed(mut self, zoomed: bool) -> Self {
        self.zoomed = zoomed;
        self
    }

    pub fn close(mut self, action: PaneAction) -> Self {
        self.close = Some(action);
        self
    }

    /// Double-clicking the bar (not its buttons) calls this, usually zoom.
    pub fn on_double_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.double_click = Some(Rc::new(handler));
        self
    }

    /// The bar element. Keep chaining on it, e.g. `.on_drag(..)`.
    pub fn render(self) -> Stateful<Div> {
        let Self {
            id,
            c,
            icon,
            title,
            leading,
            trailing,
            active,
            layout,
            can_split,
            split_horizontal,
            split_vertical,
            zoom,
            zoomed,
            close,
            double_click,
        } = self;

        let density = Density::from_palette(&c);
        let actions = [
            (
                "split-h",
                IconName::Split,
                split_horizontal.map(|a| gate_split(a, can_split)),
                false,
            ),
            (
                "split-v",
                IconName::SplitAlt,
                split_vertical.map(|a| gate_split(a, can_split)),
                false,
            ),
            ("zoom", IconName::Maximize, zoom, zoomed),
            ("close", IconName::Close, close, false),
        ];
        let buttons: Vec<AnyElement> = actions
            .into_iter()
            .filter_map(|(key, glyph, action, selected)| {
                action.map(|a| action_button(&id, key, glyph, a, selected, c))
            })
            .collect();

        let mut bar = div()
            .id(ElementId::from(id.clone()))
            .group("pane_bar")
            .flex()
            .flex_row()
            .items_center()
            .flex_shrink_0()
            .gap(c.space(6.0))
            .h(density.pane_bar_height())
            .px(c.space(8.0))
            .border_b_1()
            .border_color(c.line_subtle)
            .text_size(c.control_space(12.0))
            .when(layout.is_framed(), |bar| {
                bar.rounded_t(px((c.radius.lg - 1.0).max(0.0)))
            })
            .when(active, |bar| bar.bg(c.sunken).text_color(c.fg))
            .when(!active, |bar| {
                bar.text_color(c.muted).hover(|style| style.bg(c.accent))
            })
            .child(
                div().flex_shrink_0().child(
                    icon.svg(if active { c.fg } else { c.muted })
                        .size(c.control_space(14.0)),
                ),
            );
        if let Some(leading) = leading {
            bar = bar.child(div().flex_shrink_0().child(leading));
        }
        bar = bar.child(
            div()
                .flex_shrink_0()
                .max_w(c.space(180.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .when(active, |title| title.font_weight(FontWeight::SEMIBOLD))
                .child(title),
        );
        bar = bar.child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .children(trailing),
        );
        if !buttons.is_empty() {
            bar = bar.child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(c.space(2.0))
                    .when(!active, |actions| {
                        actions
                            .invisible()
                            .group_hover("pane_bar", |style| style.visible())
                    })
                    .children(buttons),
            );
        }
        if let Some(double_click) = double_click {
            bar = bar.on_click(move |event, window, cx| {
                if event.click_count() > 1 {
                    double_click(window, cx);
                }
            });
        }
        bar
    }
}

/// Disable a split action and explain why when the pane cannot be split.
fn gate_split(action: PaneAction, can_split: bool) -> PaneAction {
    if can_split {
        action
    } else {
        PaneAction {
            tooltip: CANNOT_SPLIT.into(),
            enabled: false,
            on_click: action.on_click,
        }
    }
}

fn action_button(
    bar_id: &SharedString,
    key: &str,
    glyph: IconName,
    action: PaneAction,
    selected: bool,
    c: Palette,
) -> AnyElement {
    let enabled = action.is_active();
    let mut button = icon_button_builder(SharedString::from(format!("{bar_id}-{key}")), c, glyph)
        .variant(ButtonVariant::Subtle)
        .size(ButtonSize::IconXs)
        .shape(IconButtonShape::Square)
        .selected(selected)
        .disabled(!enabled)
        .tooltip(action.tooltip);
    if selected && glyph == IconName::Maximize {
        button = button.selected_icon(IconName::Minimize);
    }
    let button = button.render();
    match action.on_click.filter(|_| enabled) {
        Some(on_click) => button
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                on_click(window, cx);
            })
            .into_any_element(),
        None => button.into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    fn full_bar(active: bool, layout: PaneLayoutStyle, can_split: bool, zoomed: bool) {
        let c = test_palette();
        let _ = pane_bar("bar", c, IconName::Terminal, "zsh")
            .leading(div().child("*"))
            .trailing(div().child("~/code"))
            .active(active)
            .layout(layout)
            .can_split(can_split)
            .zoomed(zoomed)
            .split_horizontal(PaneAction::new("Split right").on_click(|_, _| {}))
            .split_vertical(PaneAction::new("Split down").on_click(|_, _| {}))
            .zoom(PaneAction::new("Zoom").on_click(|_, _| {}))
            .close(PaneAction::new("Close").enabled(false))
            .on_double_click(|_, _| {})
            .render();
    }

    #[test]
    fn bar_builds_in_every_state() {
        for active in [true, false] {
            for layout in [PaneLayoutStyle::Comfortable, PaneLayoutStyle::Compact] {
                for can_split in [true, false] {
                    for zoomed in [true, false] {
                        full_bar(active, layout, can_split, zoomed);
                    }
                }
            }
        }
        // A bar with no actions or slots is valid too.
        let _ = pane_bar("bare", test_palette(), IconName::File, "a.rs").render();
    }

    #[test]
    fn frame_builds_in_both_layouts_and_focus_states() {
        for style in [PaneLayoutStyle::Comfortable, PaneLayoutStyle::Compact] {
            for focused in [true, false] {
                let _ = pane_frame("pane", test_palette(), style, focused).child("body");
            }
        }
    }

    #[test]
    fn pane_gap_is_six_comfortable_and_zero_compact() {
        let mut c = test_palette();
        c.density = 1.0;
        assert_eq!(PaneLayoutStyle::Comfortable.gap(&c), px(6.0));
        assert_eq!(PaneLayoutStyle::Compact.gap(&c), px(0.0));
        c.density = 1.15;
        assert!(PaneLayoutStyle::Comfortable.gap(&c) > px(6.0));
        assert_eq!(PaneLayoutStyle::Compact.gap(&c), px(0.0));
    }

    #[test]
    fn bar_height_follows_density() {
        let mut c = test_palette();
        c.density = 1.0;
        assert_eq!(Density::from_palette(&c).pane_bar_height(), px(28.0));
        c.density = 0.85;
        assert!(Density::from_palette(&c).pane_bar_height() < px(28.0));
    }

    #[test]
    fn disabled_split_swaps_tooltip_and_keeps_callback_inert() {
        let action = PaneAction::new("Split right").on_click(|_, _| {});
        assert!(action.is_active());
        let gated = gate_split(PaneAction::new("Split right").on_click(|_, _| {}), false);
        assert!(!gated.is_active());
        assert_eq!(gated.tooltip.as_ref(), CANNOT_SPLIT);
        let open = gate_split(PaneAction::new("Split right").on_click(|_, _| {}), true);
        assert!(open.is_active());
        assert_eq!(open.tooltip.as_ref(), "Split right");
        // No callback renders disabled even when "enabled".
        assert!(!PaneAction::new("Close").is_active());
    }
}
