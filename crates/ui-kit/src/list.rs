//! Shared `List` row primitives — `ListHeader`, `ListItem`, `ListSeparator`.
//!
//! Port of the "row with icon + label + trailing" and "section heading above
//! a row group" shapes hand-rolled across the app (the command palette's
//! result rows, the settings search-results list, the host list, …). This
//! module is the shared building block those views assemble from.
//!
//! Reference: `reference-src/src/components/ui/item.tsx` +
//! `command.tsx` (`CommandGroup` heading, `CommandItem` row,
//! `CommandSeparator`). Zed's counterpart is
//! `zed-refrence/zed/crates/ui/src/components/list.rs` (`ListItem`,
//! `ListHeader`, `ListSeparator`).
//!
//! ```ignore
//! div()
//!     .child(list_header("Recent", c.muted))
//!     .child(
//!         ListItem::new("row-1", c.fg, c.muted, c.accent)
//!             .selected(true)
//!             .on_click(cx.listener(|this, _, _w, cx| this.open(cx)))
//!             .child("Item"),
//!     )
//!     .child(list_separator(c.border))
//! ```

use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, Div, ElementId, Hsla,
    InteractiveElement, IntoElement, ParentElement, SharedString, Stateful,
    StatefulInteractiveElement, Styled, Window,
};

use crate::divider::{divider, Axis};
use crate::icon::IconName;

/// A small, muted section heading above a group of [`ListItem`]s.
pub fn list_header(label: impl Into<SharedString>, muted: Hsla) -> Div {
    div()
        .px(px(8.0))
        .py(px(4.0))
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(muted)
        .child(label.into())
}

/// A full-width 1px separator between list groups.
pub fn list_separator(border: Hsla) -> Div {
    divider(Axis::Horizontal, border).my(px(4.0))
}

/// One selectable/hoverable row: optional leading icon, label child(ren),
/// optional trailing element, `selected`/`disabled` state.
pub struct ListItem {
    id: ElementId,
    icon: Option<IconName>,
    trailing: Option<AnyElement>,
    selected: bool,
    disabled: bool,
    fg: Hsla,
    muted: Hsla,
    selected_fill: Hsla,
    children: Vec<AnyElement>,
    #[allow(clippy::type_complexity)]
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    #[allow(clippy::type_complexity)]
    extra: Option<Box<dyn FnOnce(Stateful<Div>) -> Stateful<Div>>>,
}

impl ListItem {
    pub fn new(id: impl Into<ElementId>, fg: Hsla, muted: Hsla, selected_fill: Hsla) -> Self {
        Self {
            id: id.into(),
            icon: None,
            trailing: None,
            selected: false,
            disabled: false,
            fg,
            muted,
            selected_fill,
            children: Vec::new(),
            on_click: None,
            extra: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn trailing(mut self, el: impl IntoElement) -> Self {
        self.trailing = Some(el.into_any_element());
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

    /// Row click handler. Never fires on a `disabled` row.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Escape hatch for behaviour the builder doesn't have a named method for
    /// (drag sources/targets, right-click menus, a one-off background
    /// override) — applied last, after every other style/handler, so it can
    /// override anything above. Keeps call sites like the explorer tree row
    /// (drag-and-drop, drop-target highlight) on the shared row chrome
    /// instead of hand-rolling their own `div()`.
    pub fn extra(mut self, f: impl FnOnce(Stateful<Div>) -> Stateful<Div> + 'static) -> Self {
        self.extra = Some(Box::new(f));
        self
    }
}

impl ParentElement for ListItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl IntoElement for ListItem {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let text_color = if self.disabled { self.muted } else { self.fg };
        let mut row = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .px(px(8.0))
            .py(px(6.0))
            .rounded_sm()
            .text_size(px(12.0))
            .text_color(text_color)
            .when(self.selected, |d| d.bg(self.selected_fill))
            .when_some(self.icon, |d, icon| {
                d.child(icon.svg(text_color).size(px(14.0)))
            })
            .children(self.children);
        if self.disabled {
            row = row.opacity(super::DISABLED_OPACITY);
        } else {
            row = row.cursor_pointer();
            if !self.selected {
                let hover_fill = self.selected_fill;
                row = row.hover(move |s| s.bg(hover_fill));
            }
            if let Some(h) = self.on_click {
                row = row.on_click(move |ev, w, cx| h(ev, w, cx));
            }
        }
        if let Some(trailing) = self.trailing {
            row = row.child(div().ml_auto().child(trailing));
        }
        if let Some(extra) = self.extra {
            row = extra(row);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::black;

    #[test]
    fn header_and_separator_build() {
        let _ = list_header("Section", black());
        let _ = list_separator(black());
    }

    #[test]
    fn extra_hook_runs_after_the_builder() {
        let _ = ListItem::new("row", black(), black(), black())
            .child("Label")
            .extra(|row| row.opacity(0.5))
            .into_element();
    }

    #[test]
    fn item_builds_in_every_state() {
        for selected in [true, false] {
            for disabled in [true, false] {
                let _ = ListItem::new("row", black(), black(), black())
                    .icon(IconName::File)
                    .trailing("⌘K")
                    .selected(selected)
                    .disabled(disabled)
                    .on_click(|_, _, _| {})
                    .child("Label")
                    .into_element();
            }
        }
    }
}
