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
    div, prelude::FluentBuilder, px, AnyElement, App, ClickEvent, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, SharedString,
    StatefulInteractiveElement, Styled, Window,
};

use super::IconName;
use crate::theme::ThemeStore;

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

#[derive(Clone, Copy)]
struct Colors {
    card: Hsla,
    fg: Hsla,
    muted: Hsla,
    border: Hsla,
    accent: Hsla,
    accent_fg: Hsla,
    destructive: Hsla,
}

fn colors(theme: &ThemeStore) -> Colors {
    let core = &theme.theme().core;
    Colors {
        card: core.popover,
        fg: core.popover_foreground,
        muted: theme.muted_foreground(),
        border: theme.border(),
        accent: core.accent,
        accent_fg: core.accent_foreground,
        destructive: core.destructive,
    }
}

fn render_item(item: MenuItem, c: Colors, depth: usize) -> AnyElement {
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
            handler,
        } => {
            let text_color = if disabled {
                c.muted
            } else if destructive {
                c.destructive
            } else {
                c.fg
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
                .child(label);
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
                .bg(c.card)
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
                .text_color(c.fg)
                .hover(|s| s.bg(c.accent).text_color(c.accent_fg))
                .when_some(icon, |d, ic| d.child(ic.svg(c.fg).size(px(14.0))))
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

/// Build a full-screen context-menu overlay anchored at `anchor` (window
/// coordinates). `dismiss` fires on a click anywhere outside the menu card.
pub fn context_menu(
    anchor: Point<Pixels>,
    theme: &ThemeStore,
    dismiss: impl Fn(&mut Window, &mut App) + 'static,
    items: Vec<MenuItem>,
) -> AnyElement {
    let c = colors(theme);
    let dismiss = Rc::new(dismiss);
    let d2 = dismiss.clone();

    let card = div()
        .absolute()
        .left(anchor.x)
        .top(anchor.y)
        .flex()
        .flex_col()
        .min_w(px(160.0))
        .max_w(px(320.0))
        .p(px(4.0))
        .rounded_md()
        .bg(c.card)
        .border_1()
        .border_color(c.border)
        .shadow_lg()
        .occlude()
        .children(items.into_iter().map(move |it| render_item(it, c, 0)));

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

#[cfg(test)]
mod tests {
    use super::*;

    fn action(item: &MenuItem) -> (&SharedString, bool, bool, bool, bool) {
        match &item.kind {
            Kind::Action {
                label,
                destructive,
                disabled,
                checked,
                handler,
                ..
            } => (label, *destructive, *disabled, *checked, handler.is_some()),
            _ => panic!("not an action item"),
        }
    }

    #[test]
    fn builder_sets_item_flags() {
        let plain = MenuItem::new("id", "Plain");
        let (label, destr, dis, chk, has_handler) = action(&plain);
        assert_eq!(label.as_ref(), "Plain");
        assert!(!destr && !dis && !chk && !has_handler);

        let styled = MenuItem::new("id", "Danger")
            .icon(IconName::Trash)
            .destructive()
            .disabled(true)
            .checked(true)
            .on_click(|_, _, _| {});
        let (_, destr, dis, chk, has_handler) = action(&styled);
        assert!(destr && dis && chk && has_handler);

        assert!(matches!(MenuItem::separator().kind, Kind::Separator));
        assert!(matches!(MenuItem::label("Sec").kind, Kind::Label(_)));
        assert!(matches!(
            MenuItem::submenu("s", "Sub", vec![MenuItem::new("a", "A")]).kind,
            Kind::Submenu { .. }
        ));
    }
}
