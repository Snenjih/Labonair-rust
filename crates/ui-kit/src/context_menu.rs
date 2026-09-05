//! Shared right-click context-menu primitive.
//!
//! A port of the reference `components/ui/context-menu.tsx` (radix) look &
//! behaviour: `min-w-[8rem]`, `rounded-md`, `bg-popover`, `border`, `p-1`,
//! `shadow-md`; items `rounded-sm px-2 py-1.5 text-sm`, `focus:bg-accent`;
//! destructive variant = `text-destructive`; separators; disabled items;
//! one level of sub-menus (revealed on hover).
//!
//! The port previously hand-rolled a bespoke `div` menu in every view
//! (`workspace.rs`, `explorer.rs`, `sftp.rs`, …). This module is the single
//! implementation they all now build from.
//!
//! Usage — the caller owns the "is this menu open?" state (an `Option<Point>`
//! plus whatever context it needs) and rebuilds the entries each render:
//!
//! ```ignore
//! let view = cx.entity();
//! let dismiss = { let v = view.clone(); move |_w: &mut Window, cx: &mut App|
//!     v.update(cx, |this, cx| { this.menu = None; cx.notify() }) };
//! context_menu(anchor, self.theme.read(cx), dismiss, vec![
//!     MenuItem::new("copy", "Copy").icon(IconName::Copy).on_click({
//!         let v = view.clone();
//!         move |_, w, cx| v.update(cx, |this, cx| { this.menu = None; this.copy(w, cx) })
//!     }),
//!     MenuItem::separator(),
//!     MenuItem::new("delete", "Delete").destructive().on_click(/* … */),
//! ])
//! ```

use std::rc::Rc;

use gpui::{
    anchored, deferred, div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::IconName;
use crate::kbd::kbd_row;
use crate::palette::Palette;

type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Boxed click handler — the shape `MenuItem::on_click` accepts once boxed.
/// Exported so call sites can name it in helper signatures (clippy
/// `type_complexity`).
pub type MenuClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One row of a [`context_menu`]. Build with [`MenuItem::new`] /
/// [`MenuItem::separator`] / [`MenuItem::label`] / [`MenuItem::submenu`].
pub struct MenuItem {
    kind: Kind,
}

enum Kind {
    Action {
        id: SharedString,
        label: SharedString,
        icon: Option<IconName>,
        destructive: bool,
        disabled: bool,
        checked: bool,
        /// Right-aligned keybinding hint (`["\u{2318}", "K"]`), rendered as
        /// [`crate::kbd`] chips — radix `ContextMenuShortcut`.
        keybind: Vec<SharedString>,
        handler: Option<Handler>,
    },
    Separator,
    Label(SharedString),
    Submenu {
        id: SharedString,
        label: SharedString,
        icon: Option<IconName>,
        items: Vec<MenuItem>,
    },
}

impl MenuItem {
    /// An actionable row. `id` must be unique within the menu (element id).
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Action {
                id: id.into(),
                label: label.into(),
                icon: None,
                destructive: false,
                disabled: false,
                checked: false,
                keybind: Vec::new(),
                handler: None,
            },
        }
    }

    /// A horizontal divider.
    pub fn separator() -> Self {
        Self {
            kind: Kind::Separator,
        }
    }

    /// A non-interactive section heading.
    pub fn label(text: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Label(text.into()),
        }
    }

    /// A row that reveals `items` in a nested panel on hover.
    pub fn submenu(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        items: Vec<MenuItem>,
    ) -> Self {
        Self {
            kind: Kind::Submenu {
                id: id.into(),
                label: label.into(),
                icon: None,
                items,
            },
        }
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        match &mut self.kind {
            Kind::Action { icon: i, .. } | Kind::Submenu { icon: i, .. } => *i = Some(icon),
            _ => {}
        }
        self
    }

    /// Render the label in the destructive colour (`text-destructive`).
    pub fn destructive(mut self) -> Self {
        if let Kind::Action { destructive, .. } = &mut self.kind {
            *destructive = true;
        }
        self
    }

    /// Dim + disable the row (no handler fires).
    pub fn disabled(mut self, disabled: bool) -> Self {
        if let Kind::Action { disabled: d, .. } = &mut self.kind {
            *d = disabled;
        }
        self
    }

    /// Right-aligned keybinding hint — one chip per key
    /// (`["\u{2318}", "\u{21E7}", "P"]`). Ignored on non-action items.
    pub fn keybind<S: Into<SharedString>>(mut self, keys: impl IntoIterator<Item = S>) -> Self {
        if let Kind::Action { keybind, .. } = &mut self.kind {
            *keybind = keys.into_iter().map(Into::into).collect();
        }
        self
    }

    /// Show a leading check mark (radio / checkbox item).
    pub fn checked(mut self, checked: bool) -> Self {
        if let Kind::Action { checked: c, .. } = &mut self.kind {
            *c = checked;
        }
        self
    }

    /// Click handler. Runs with `&mut App`, so the caller captures a
    /// `cx.entity()` clone and calls `.update` on it.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if let Kind::Action { handler: h, .. } = &mut self.kind {
            *h = Some(Rc::new(handler));
        }
        self
    }
}

fn render_item(item: MenuItem, c: Palette, depth: usize) -> AnyElement {
    match item.kind {
        Kind::Separator => div().my(px(4.0)).h(px(1.0)).bg(c.border).into_any_element(),
        Kind::Label(text) => div()
            .px(px(8.0))
            .py(px(4.0))
            .text_size(px(11.0))
            .text_color(c.muted)
            .child(text)
            .into_any_element(),
        Kind::Action {
            id,
            label,
            icon,
            destructive,
            disabled,
            checked,
            keybind,
            handler,
        } => {
            let text_color = if disabled {
                c.muted
            } else if destructive {
                c.destructive
            } else {
                c.popover_fg
            };
            let mut row = div()
                .id(id)
                .flex()
                .items_center()
                .gap_2()
                .w_full()
                .px(px(8.0))
                .py(px(6.0))
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(text_color)
                .when(checked, |d| {
                    d.child(IconName::CircleCheck.svg(text_color).size(px(13.0)))
                })
                .when(!checked && icon.is_some(), |d| {
                    d.child(icon.unwrap().svg(text_color).size(px(14.0)))
                })
                .child(label)
                .when(!keybind.is_empty(), |d| {
                    d.child(div().ml_auto().child(kbd_row(keybind.clone(), c)))
                });
            if disabled {
                row = row.opacity(super::DISABLED_OPACITY);
            } else {
                let hover_fg = if destructive {
                    c.destructive
                } else {
                    c.accent_fg
                };
                row = row.hover(move |s| s.bg(c.accent).text_color(hover_fg));
                if let Some(h) = handler {
                    row = row.on_click(move |ev, w, cx| h(ev, w, cx));
                }
            }
            let _ = depth;
            row.into_any_element()
        }
        Kind::Submenu {
            id,
            label,
            icon,
            items,
        } => {
            let group = SharedString::from(format!("ctxsub-{id}-{depth}"));
            let panel = div()
                .absolute()
                .left_full()
                .top_0()
                .ml(px(2.0))
                .invisible()
                .group_hover(group.clone(), |s| s.visible())
                .flex()
                .flex_col()
                .min_w(px(160.0))
                .p(px(4.0))
                .rounded_md()
                .bg(c.popover)
                .border_1()
                .border_color(c.border)
                .shadow_lg()
                .children(items.into_iter().map(|it| render_item(it, c, depth + 1)));
            div()
                .id(id)
                .group(group)
                .relative()
                .flex()
                .items_center()
                .gap_2()
                .w_full()
                .px(px(8.0))
                .py(px(6.0))
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(c.popover_fg)
                .hover(|s| s.bg(c.accent).text_color(c.accent_fg))
                .when_some(icon, |d, ic| d.child(ic.svg(c.popover_fg).size(px(14.0))))
                .child(label)
                .child(
                    div()
                        .ml_auto()
                        .child(IconName::ChevronRight.svg(c.muted).size(px(13.0))),
                )
                .child(panel)
                .into_any_element()
        }
    }
}

/// The menu card itself — the `p-1 rounded-md bg-popover border shadow-md`
/// panel shared by [`context_menu`] and [`popover_menu`].
fn menu_card(c: Palette, items: Vec<MenuItem>) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w(px(160.0))
        .max_w(px(320.0))
        .p(px(4.0))
        .rounded_md()
        .bg(c.popover)
        .border_1()
        .border_color(c.border)
        .shadow_lg()
        .occlude()
        .children(items.into_iter().map(move |it| render_item(it, c, 0)))
}

/// The bare menu card on its own — no full-screen backdrop, no anchoring, no
/// dismiss wiring. Used by the component gallery (T20-004) to show a context
/// menu as a permanently-open example; not for normal call sites, which want
/// [`context_menu`] / [`popover_menu`].
#[cfg(any(debug_assertions, feature = "gallery"))]
pub fn menu_card_preview(c: Palette, items: Vec<MenuItem>) -> gpui::Div {
    menu_card(c, items)
}

/// Build a full-screen context-menu overlay anchored at `anchor` (window
/// coordinates). `dismiss` fires on a click anywhere outside the menu card.
pub fn context_menu(
    anchor: Point<Pixels>,
    c: Palette,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    items: Vec<MenuItem>,
) -> AnyElement {
    let dismiss = Rc::new(dismiss);
    let d2 = dismiss.clone();

    let card = menu_card(c, items).absolute().left(anchor.x).top(anchor.y);

    div()
        .absolute()
        .inset_0()
        .on_mouse_down(
            MouseButton::Left,
            move |_: &MouseDownEvent, w: &mut Window, cx: &mut App| dismiss(w, cx),
        )
        .on_mouse_down(
            MouseButton::Right,
            move |_: &MouseDownEvent, w: &mut Window, cx: &mut App| d2(w, cx),
        )
        .child(card)
        .into_any_element()
}

/// The same menu, opened from a *trigger* instead of a right-click: the card is
/// `anchored().snap_to_window()` + `deferred(..)` so it cannot be clipped by an
/// ancestor's `overflow_hidden` and flips itself back into the window near an
/// edge. This is radix' `DropdownMenu`/`PopoverMenu` (reference
/// `components/ui/dropdown-menu.tsx`) as opposed to `ContextMenu`; Zed splits
/// the same two roles across `popover_menu.rs` and `context_menu.rs`.
///
/// Pass the trigger's bottom-left window point as `anchor` so the card opens
/// below it.
///
/// ```ignore
/// popover_menu(bounds.bottom_left(), c, dismiss, vec![
///     MenuItem::new("settings", "Settings\u{2026}").on_click(..),
///     MenuItem::separator(),
///     MenuItem::new("profile", "Profile").on_click(..),
/// ])
/// ```
pub fn popover_menu(
    anchor: Point<Pixels>,
    c: Palette,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    items: Vec<MenuItem>,
) -> AnyElement {
    let card = anchored()
        .position(anchor)
        .snap_to_window()
        .child(menu_card(c, items));

    deferred(
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                move |_: &MouseDownEvent, w: &mut Window, cx: &mut App| dismiss(w, cx),
            )
            .child(card),
    )
    .with_priority(200)
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::type_complexity)]
    fn action(item: &MenuItem) -> (&SharedString, bool, bool, bool, bool, &[SharedString]) {
        match &item.kind {
            Kind::Action {
                label,
                destructive,
                disabled,
                checked,
                keybind,
                handler,
                ..
            } => (
                label,
                *destructive,
                *disabled,
                *checked,
                handler.is_some(),
                keybind,
            ),
            _ => panic!("not an action item"),
        }
    }

    #[test]
    fn builder_sets_item_flags() {
        let plain = MenuItem::new("id", "Plain");
        let (label, destr, dis, chk, has_handler, keys) = action(&plain);
        assert_eq!(label.as_ref(), "Plain");
        assert!(!destr && !dis && !chk && !has_handler);
        assert!(keys.is_empty());

        let styled = MenuItem::new("id", "Danger")
            .icon(IconName::Trash)
            .destructive()
            .disabled(true)
            .checked(true)
            .keybind(["\u{2318}", "\u{232B}"])
            .on_click(|_, _, _| {});
        let (_, destr, dis, chk, has_handler, keys) = action(&styled);
        assert!(destr && dis && chk && has_handler);
        assert_eq!(keys.len(), 2);

        assert!(matches!(MenuItem::separator().kind, Kind::Separator));
        assert!(matches!(MenuItem::label("Sec").kind, Kind::Label(_)));
        assert!(matches!(
            MenuItem::submenu("s", "Sub", vec![MenuItem::new("a", "A")]).kind,
            Kind::Submenu { .. }
        ));
    }
}
