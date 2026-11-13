//! Shared tab-strip item for workspace and docked tab surfaces.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, Div, ElementId, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window,
};

use crate::animation::{state_fill, MotionExt};
use crate::{
    icon_button_builder, indicator, ButtonSize, ButtonVariant, IconButtonShape, IconName,
    IndicatorSize, Palette, DISABLED_OPACITY,
};

type TabAction = Rc<dyn Fn(&mut Window, &mut App)>;
type TabNavigation = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Orientation of a tab item within its strip.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabLayout {
    /// Compact item in a horizontal workspace strip.
    #[default]
    Horizontal,
    /// Full-width item in a vertical tab panel.
    Vertical,
}

/// Visual style of a tab item.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabStyle {
    /// The flat, contiguous strip (the current titlebar tabs).
    #[default]
    Classic,
    /// Rounded floating blocks, laid out with [`floating_tab_strip`].
    ///
    /// Horizontal blocks are 28px **pills** sitting in the strip's translucent
    /// track: the selected pill is `float.tab_active` with the `row` shadow,
    /// the others hover to `float.hover`. Vertical blocks (the Tabs panel)
    /// are 28px blocks with `radius.md`, a sunken fill and a `line_subtle`
    /// ring when selected. Both show the close button on hover only.
    Floating,
}

/// One caller-owned tab item. Workspace retains selection, ordering, drag/drop,
/// persistence, and close policy; this component supplies common tab chrome
/// and keyboard activation.
pub struct TabItemBuilder {
    id: SharedString,
    palette: Palette,
    label: SharedString,
    label_content: Option<AnyElement>,
    leading: Option<IconName>,
    layout: TabLayout,
    style: TabStyle,
    selected: bool,
    disabled: bool,
    dirty: bool,
    status_colors: Vec<Hsla>,
    busy: bool,
    peek: bool,
    pinned: bool,
    activate: Option<TabAction>,
    double_click: Option<TabAction>,
    navigate: Option<TabNavigation>,
    close: Option<TabAction>,
}

/// Start a shared tab item. The caller attaches owner-specific drag, context,
/// and lifecycle behavior to the returned `Stateful<Div>`.
pub fn tab_item(
    id: impl Into<SharedString>,
    palette: Palette,
    label: impl Into<SharedString>,
) -> TabItemBuilder {
    TabItemBuilder {
        id: id.into(),
        palette,
        label: label.into(),
        label_content: None,
        leading: None,
        layout: TabLayout::Horizontal,
        style: TabStyle::Classic,
        selected: false,
        disabled: false,
        dirty: false,
        status_colors: Vec::new(),
        busy: false,
        peek: false,
        pinned: false,
        activate: None,
        double_click: None,
        navigate: None,
        close: None,
    }
}

impl TabItemBuilder {
    pub fn layout(mut self, layout: TabLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Switch between the classic strip and the floating block style.
    pub fn style(mut self, style: TabStyle) -> Self {
        self.style = style;
        self
    }

    pub fn leading(mut self, icon: IconName) -> Self {
        self.leading = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Append a small owner-colored state marker after the tab title.
    pub fn status_color(mut self, color: Option<Hsla>) -> Self {
        if let Some(color) = color {
            self.status_colors.push(color);
        }
        self
    }

    /// Append another small owner-colored state marker after existing markers.
    pub fn status_marker(mut self, color: Option<Hsla>) -> Self {
        if let Some(color) = color {
            self.status_colors.push(color);
        }
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn peek(mut self, peek: bool) -> Self {
        self.peek = peek;
        self
    }

    /// Show a pin in place of the close control for a pinned tab.
    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// Replace the standard truncated title with an owner-rendered title, such
    /// as the workspace's inline rename editor.
    pub fn label_content(mut self, content: impl IntoElement) -> Self {
        self.label_content = Some(content.into_any_element());
        self
    }

    /// Activate this tab by pointer click, Enter, or Space.
    pub fn on_activate(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.activate = Some(Rc::new(handler));
        self
    }

    /// Handle a pointer double-click after the tab is activated.
    pub fn on_double_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.double_click = Some(Rc::new(handler));
        self
    }

    /// Move to the adjacent tab. `true` means next/right/down; `false` means
    /// previous/left/up, with the key pair selected from `layout`.
    pub fn on_navigate(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.navigate = Some(Rc::new(handler));
        self
    }

    /// Add the owner's close decision to the shared close control.
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.close = Some(Rc::new(handler));
        self
    }

    pub fn render(self) -> Stateful<Div> {
        let Self {
            id,
            palette,
            label,
            label_content,
            leading,
            layout,
            style,
            selected,
            disabled,
            dirty,
            status_colors,
            busy,
            peek,
            pinned,
            activate,
            double_click,
            navigate,
            close,
        } = self;

        let horizontal = layout == TabLayout::Horizontal;
        let floating = style == TabStyle::Floating;
        // Horizontal floating tabs are pills on the strip's track.
        let pill = floating && horizontal;
        let mut row = div()
            .id(id.clone())
            .group("tab_item")
            .flex()
            .flex_row()
            .items_center()
            .gap(palette.space(if horizontal && !floating { 8.0 } else { 6.0 }))
            .h(palette.space(if horizontal && !floating { 40.0 } else { 28.0 }))
            .px(palette.space(if horizontal && !floating {
                12.0
            } else if floating && horizontal {
                10.0
            } else {
                8.0
            }))
            .rounded(if horizontal && !floating {
                px(0.0)
            } else if pill {
                px(FLOATING_PILL_HEIGHT / 2.0)
            } else {
                px(palette.radius.md)
            })
            .text_size(palette.control_space(if horizontal && !floating { 13.0 } else { 12.0 }))
            .text_color(if selected { palette.fg } else { palette.muted })
            .when(layout == TabLayout::Vertical, |row| {
                row.w_full().flex_shrink_0()
            })
            // Ciel `EditorTabs variant="flat"`: borderless; the active tab is a
            // fill, never an underline or accent tint.
            .when(horizontal, |row| {
                row.relative().flex_shrink_0().max_w(palette.space(240.0))
            })
            // Floating blocks keep a constant 1px border (transparent) so
            // selecting never shifts layout; the selected ring is part of the
            // fading fill layer below.
            .when(floating, |row| {
                row.border_1().border_color(gpui::transparent_black())
            })
            .relative()
            .when(selected, |row| row.font_weight(FontWeight::SEMIBOLD))
            // Hover stays under the selected layer so a click on a hovered tab
            // blends into the selection instead of dipping to the bar colour.
            .when(!disabled, |row| {
                row.hover(|style| style.bg(if pill { palette.hover } else { palette.accent }))
            })
            .when(!disabled, |row| {
                row.cursor_pointer()
                    .tab_index(0)
                    .focus(|style| style.border_1().border_color(palette.ring))
            })
            .when(disabled, |row| row.opacity(DISABLED_OPACITY));

        // Selected fill (classic: editor surface; floating: sunken block with a
        // 1px ring). An always-built layer, so selecting and deselecting
        // crossfade; paints behind everything that follows.
        row = if pill {
            // The selected pill: `float.tab_active` plus the `row` shadow tier.
            row.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(FLOATING_PILL_HEIGHT / 2.0))
                    .bg(palette.tab_active)
                    .shadow(palette.row_shadow())
                    .motion_state_fade((ElementId::from(id.clone()), "fill"), palette, selected),
            )
        } else {
            row.child(state_fill(
                (ElementId::from(id.clone()), "fill"),
                palette,
                selected,
                if horizontal && !floating {
                    palette.bg
                } else {
                    palette.selected_fill
                },
                floating.then_some(palette.line_subtle),
                if horizontal && !floating {
                    px(0.0)
                } else {
                    px(palette.radius.md)
                },
                px(if floating { 1.0 } else { 0.0 }),
            ))
        };

        if let Some(icon) = leading {
            row = row.child(
                div().flex_shrink_0().child(
                    icon.svg(if selected { palette.fg } else { palette.muted })
                        .size(palette.control_space(14.0)),
                ),
            );
        }

        let title = label_content.unwrap_or_else(|| {
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .when(layout == TabLayout::Horizontal, |title| {
                    title.max_w(palette.space(180.0))
                })
                .when(peek, |title| title.italic())
                .child(label)
                .into_any_element()
        });
        row = row.child(title);

        if dirty {
            row = row.child(indicator(IndicatorSize::Xs, palette.fg.opacity(0.7)));
        }
        for color in status_colors {
            row = row.child(indicator(IndicatorSize::Xs, color));
        }
        if busy {
            row = row.child(indicator(IndicatorSize::Xs, palette.info));
        }

        if pinned {
            row = row.child(
                div().flex_shrink_0().child(
                    IconName::Pin
                        .svg(if selected { palette.fg } else { palette.muted })
                        .size(palette.control_space(14.0)),
                ),
            );
        } else if let Some(close) = close.filter(|_| !disabled) {
            let click_close = close.clone();
            let close_button = icon_button_builder(
                SharedString::from(format!("{id}-close")),
                palette,
                IconName::X,
            )
            .variant(ButtonVariant::Subtle)
            .size(ButtonSize::IconXs)
            .shape(IconButtonShape::Square)
            .tooltip("Close tab")
            .render()
            // The tab hovers to `float.hover` and the X's own hover overlay
            // stacks on it, so it reads as its own target without a second
            // `.hover` (GPUI asserts a style is set once).
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                click_close(window, cx);
            })
            .on_key_down(move |event, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    close(window, cx);
                }
            })
            .when(horizontal || floating, |button| {
                button
                    .invisible()
                    .group_hover("tab_item", |style| style.visible())
            });
            row = row.child(close_button);
        }

        if !disabled {
            if let Some(activate) = activate.as_ref() {
                let click_activate = activate.clone();
                row = row.on_click(move |event, window, cx| {
                    click_activate(window, cx);
                    if event.click_count() > 1 {
                        if let Some(double_click) = &double_click {
                            double_click(window, cx);
                        }
                    }
                });
            }
        }
        if !disabled && (activate.is_some() || navigate.is_some()) {
            row = row.on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                if matches!(key, "enter" | "space") {
                    if let Some(activate) = &activate {
                        cx.stop_propagation();
                        activate(window, cx);
                    }
                    return;
                }
                let Some(direction) = navigation_direction(layout, key) else {
                    return;
                };
                if let Some(navigate) = &navigate {
                    cx.stop_propagation();
                    navigate(direction, window, cx);
                }
            });
        }

        if horizontal && !floating {
            row = row.child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(1.0))
                    .bg(palette.border),
            );
        }

        row
    }
}

/// Height of a horizontal floating pill (the tab block).
const FLOATING_PILL_HEIGHT: f32 = 28.0;
/// Padding between the track's edge and its pills (horizontal strip).
const FLOATING_TRACK_PADDING: f32 = 2.0;
/// Padding around the blocks of a vertical floating strip.
const FLOATING_BAR_PADDING: f32 = 6.0;
/// Gap between floating blocks: horizontal and vertical strips.
const FLOATING_GAP_H: f32 = 2.0;
const FLOATING_GAP_V: f32 = 2.0;
/// Extra space that replaces the pinned/unpinned divider of the classic strip.
const FLOATING_GROUP_GAP: f32 = 8.0;
/// Thickness of the drop-insertion marker.
const DROP_MARKER_THICKNESS: f32 = 2.0;

/// The container for [`TabStyle::Floating`] blocks: a translucent pill-shaped
/// **track** (`float.track` with a `float.track_border` rim) as tall as its
/// pills plus 2px padding and the rim on every side and 2px gaps (horizontal), or a column
/// with 6px padding and 2px gaps (vertical).
/// The caller adds the tab blocks, and [`tab_group_gap`] between groups.
pub fn floating_tab_strip(
    id: impl Into<ElementId>,
    c: Palette,
    layout: TabLayout,
) -> Stateful<Div> {
    let strip = div().id(id).flex().flex_shrink_0();
    match layout {
        TabLayout::Horizontal => {
            // Pills + padding + the 1px rim on both sides (border-box sizing).
            let height = c.space(FLOATING_PILL_HEIGHT + 2.0 * FLOATING_TRACK_PADDING) + px(2.0);
            strip
                .flex_row()
                .items_center()
                .h(height)
                .p(c.space(FLOATING_TRACK_PADDING))
                .gap(c.space(FLOATING_GAP_H))
                .rounded(height / 2.0)
                .bg(c.track)
                .border_1()
                .border_color(c.track_line)
        }
        TabLayout::Vertical => strip
            .flex_col()
            .w_full()
            .p(c.space(FLOATING_BAR_PADDING))
            .gap(c.space(FLOATING_GAP_V)),
    }
}

/// A spacer that separates tab groups (for example pinned from unpinned) in a
/// floating strip, in place of the classic 1px divider.
pub fn tab_group_gap(c: Palette, layout: TabLayout) -> Div {
    let gap = div().flex_shrink_0();
    match layout {
        TabLayout::Horizontal => gap.w(c.space(FLOATING_GROUP_GAP)).h_full(),
        TabLayout::Vertical => gap.h(c.space(FLOATING_GROUP_GAP)).w_full(),
    }
}

/// Which edge of a tab block a drop would insert next to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabDropSide {
    /// Before the block (left in a horizontal strip, above in a vertical one).
    Before,
    /// After the block.
    After,
}

/// The drop-insertion marker: a 2px primary bar centred in the gap next to a
/// tab block. It is absolutely positioned, so showing it never shifts layout:
/// add it as a child of the (relative) tab block while a drag hovers it.
pub fn tab_drop_marker(c: Palette, layout: TabLayout, side: TabDropSide) -> Div {
    let thickness = px(DROP_MARKER_THICKNESS);
    let bar = div().absolute().bg(c.primary).rounded_full();
    // Centre the bar in the gap: half the gap plus half the bar, outward.
    let (gap, inset) = match layout {
        TabLayout::Horizontal => (FLOATING_GAP_H, c.space(4.0)),
        TabLayout::Vertical => (FLOATING_GAP_V, c.space(4.0)),
    };
    let outward = -(c.space(gap) + thickness) / 2.0;
    match (layout, side) {
        (TabLayout::Horizontal, TabDropSide::Before) => {
            bar.left(outward).top(inset).bottom(inset).w(thickness)
        }
        (TabLayout::Horizontal, TabDropSide::After) => {
            bar.right(outward).top(inset).bottom(inset).w(thickness)
        }
        (TabLayout::Vertical, TabDropSide::Before) => {
            bar.top(outward).left(inset).right(inset).h(thickness)
        }
        (TabLayout::Vertical, TabDropSide::After) => {
            bar.bottom(outward).left(inset).right(inset).h(thickness)
        }
    }
}

fn navigation_direction(layout: TabLayout, key: &str) -> Option<bool> {
    match (layout, key) {
        (TabLayout::Horizontal, "right") | (TabLayout::Vertical, "down") => Some(true),
        (TabLayout::Horizontal, "left") | (TabLayout::Vertical, "up") => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_palette;

    #[test]
    fn builds_each_layout_and_visible_state() {
        for layout in [TabLayout::Horizontal, TabLayout::Vertical] {
            for selected in [true, false] {
                for disabled in [true, false] {
                    let _ = tab_item("tab", test_palette(), "Editor")
                        .layout(layout)
                        .leading(IconName::SquarePen)
                        .selected(selected)
                        .disabled(disabled)
                        .dirty(true)
                        .busy(true)
                        .peek(true)
                        .on_activate(|_, _| {})
                        .on_double_click(|_, _| {})
                        .on_navigate(|_, _, _| {})
                        .on_close(|_, _| {})
                        .render();
                }
            }
        }
    }

    #[test]
    fn floating_style_builds_each_layout_and_state() {
        let c = test_palette();
        for layout in [TabLayout::Horizontal, TabLayout::Vertical] {
            for selected in [true, false] {
                for disabled in [true, false] {
                    let _ = tab_item("tab", c, "Editor")
                        .style(TabStyle::Floating)
                        .layout(layout)
                        .leading(IconName::SquarePen)
                        .selected(selected)
                        .disabled(disabled)
                        .dirty(true)
                        .pinned(!selected)
                        .on_activate(|_, _| {})
                        .on_close(|_, _| {})
                        .render()
                        .child(tab_drop_marker(c, layout, TabDropSide::Before))
                        .child(tab_drop_marker(c, layout, TabDropSide::After));
                }
            }
            let _ = floating_tab_strip("strip", c, layout).child(tab_group_gap(c, layout));
        }
    }

    #[test]
    fn classic_stays_the_default_style() {
        assert_eq!(TabStyle::default(), TabStyle::Classic);
    }

    #[test]
    fn custom_rename_content_builds_without_replacing_tab_chrome() {
        let _ = tab_item("tab", test_palette(), "Editor")
            .label_content(div().child("renamed"))
            .render();
    }

    #[test]
    fn keyboard_navigation_matches_tab_orientation() {
        assert_eq!(
            navigation_direction(TabLayout::Horizontal, "right"),
            Some(true)
        );
        assert_eq!(
            navigation_direction(TabLayout::Horizontal, "left"),
            Some(false)
        );
        assert_eq!(navigation_direction(TabLayout::Horizontal, "down"), None);
        assert_eq!(
            navigation_direction(TabLayout::Vertical, "down"),
            Some(true)
        );
        assert_eq!(navigation_direction(TabLayout::Vertical, "up"), Some(false));
        assert_eq!(navigation_direction(TabLayout::Vertical, "right"), None);
    }
}
