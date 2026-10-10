//! Shared tab-strip item for workspace and docked tab surfaces.

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, Div, InteractiveElement, IntoElement,
    ParentElement, SharedString, Stateful, StatefulInteractiveElement, Styled, Window,
};

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
    selected: bool,
    disabled: bool,
    dirty: bool,
    busy: bool,
    peek: bool,
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
        selected: false,
        disabled: false,
        dirty: false,
        busy: false,
        peek: false,
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

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn peek(mut self, peek: bool) -> Self {
        self.peek = peek;
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
            selected,
            disabled,
            dirty,
            busy,
            peek,
            activate,
            double_click,
            navigate,
            close,
        } = self;

        let horizontal = layout == TabLayout::Horizontal;
        let mut row = div()
            .id(id.clone())
            .group("tab_item")
            .flex()
            .flex_row()
            .items_center()
            .gap(palette.space(if horizontal { 8.0 } else { 6.0 }))
            .h(palette.space(if horizontal { 32.0 } else { 28.0 }))
            .px(palette.space(if horizontal { 12.0 } else { 8.0 }))
            .rounded(if horizontal {
                px(0.0)
            } else {
                px(palette.radius.md)
            })
            .text_size(palette.control_space(if horizontal { 13.0 } else { 12.0 }))
            .text_color(if selected { palette.fg } else { palette.muted })
            .when(layout == TabLayout::Vertical, |row| {
                row.w_full().flex_shrink_0()
            })
            .when(horizontal, |row| {
                row.relative()
                    .flex_shrink_0()
                    .max_w(palette.space(240.0))
                    .border_b_1()
                    .border_r_1()
                    .border_color(palette.border)
            })
            .when(selected, |row| {
                if horizontal {
                    row.bg(palette.bg).border_color(palette.bg)
                } else {
                    row.bg(palette.selected_fill)
                }
            })
            .when(!selected && !disabled, |row| {
                if horizontal {
                    row
                } else {
                    row.hover(|style| style.bg(palette.border))
                }
            })
            .when(!disabled, |row| {
                row.cursor_pointer()
                    .tab_index(0)
                    .focus(|style| style.border_1().border_color(palette.ring))
            })
            .when(disabled, |row| row.opacity(DISABLED_OPACITY));

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
        if busy {
            row = row.child(indicator(IndicatorSize::Xs, palette.info));
        }

        if let Some(close) = close.filter(|_| !disabled) {
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
            .when(horizontal, |button| {
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

        if horizontal {
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
