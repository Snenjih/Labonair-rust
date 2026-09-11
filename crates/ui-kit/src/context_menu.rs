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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    anchored, canvas, deferred, div, prelude::FluentBuilder, px, AnimationExt, AnyElement, App,
    Bounds, ClickEvent, Corner, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, Point, SharedString, StatefulInteractiveElement, Styled, Window,
};

use super::IconName;
use crate::animation::fade_in;
use crate::kbd::kbd_row;
use crate::palette::Palette;

type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Which part of a controlled submenu a hover transition came from. The caller
/// needs the distinction: leaving the *trigger* must not close the submenu
/// (the pointer may be travelling into the flyout), while leaving the *flyout*
/// should.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmenuHoverSource {
    /// The `Label ▸` row in the parent menu.
    Trigger,
    /// The open flyout panel.
    Flyout,
}

/// Hover callback for a *controlled* submenu — `(source, hovered)`: `hovered`
/// is `true` on pointer-enter, `false` on pointer-leave.
type SubmenuHover = Rc<dyn Fn(SubmenuHoverSource, bool, &mut Window, &mut App)>;

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
        /// Right-aligned muted secondary text (e.g. a host address next to a
        /// host name). Mutually exclusive with `keybind` in practice.
        detail: Option<SharedString>,
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
        /// `None` → the flyout is revealed by pure CSS `:hover` (fine for a
        /// lone submenu). `Some` → the caller owns "which submenu is open" as
        /// explicit state: the flyout is only built when `open`, and hover
        /// transitions are reported back through the callback. Two adjacent
        /// controlled submenus can't both latch open, since the caller holds a
        /// single open-id.
        control: Option<SubmenuControl>,
    },
}

struct SubmenuControl {
    open: bool,
    on_hover: SubmenuHover,
}

impl MenuItem {
    /// An actionable row. `id` must be unique within the menu (element id).
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            kind: Kind::Action {
                id: id.into(),
                label: label.into(),
                icon: None,
                detail: None,
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
                control: None,
            },
        }
    }

    /// Put this submenu under caller-owned open/close state instead of pure
    /// `:hover`. `open` builds (and shows) the flyout; `on_hover` fires `true`
    /// when the pointer enters the trigger row or the open flyout and `false`
    /// when it leaves. The caller keeps one "open submenu" id, so sibling
    /// submenus can't both open and their flyouts can't overlap. No-op on
    /// non-submenu items.
    pub fn submenu_control(
        mut self,
        open: bool,
        on_hover: impl Fn(SubmenuHoverSource, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        if let Kind::Submenu { control, .. } = &mut self.kind {
            *control = Some(SubmenuControl {
                open,
                on_hover: Rc::new(on_hover),
            });
        }
        self
    }

    /// Leading icon.
    pub fn icon(mut self, icon: IconName) -> Self {
        match &mut self.kind {
            Kind::Action { icon: i, .. } | Kind::Submenu { icon: i, .. } => *i = Some(icon),
            _ => {}
        }
        self
    }

    /// Right-aligned muted secondary text (radix `ContextMenuShortcut` slot,
    /// but plain text — used for a host address beside a host name). Ignored on
    /// non-action items.
    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        if let Kind::Action { detail, .. } = &mut self.kind {
            *detail = Some(text.into());
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

/// Per-render list of open submenu-flyout rectangles in window space. A
/// controlled submenu flyout is painted *outside* the parent menu card's own
/// bounds (`absolute().left_full()`), so a click that lands in the flyout would
/// otherwise trip the card's `on_mouse_down_out` (capture phase) and dismiss the
/// whole menu before the flyout row's `on_click` (bubble, on mouse-up) can run —
/// i.e. clicking a host in the `+ ▸ SSH ▸` / `SFTP ▸` submenus did nothing.
///
/// Each open flyout records its bounds here during prepaint; the card's
/// outside-press dismiss and the full-screen backdrop consult it and skip the
/// dismiss when the press is inside a flyout. The `Rc` is rebuilt on every
/// `context_menu` / `popover_menu` call, so it is always fresh for the frame.
type FlyoutBounds = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A layout-neutral probe that records its own painted rectangle into `sink`
/// during prepaint. Dropped into each open submenu flyout so the menu's dismiss
/// paths can tell "inside a flyout" from "outside the menu".
fn flyout_probe(sink: FlyoutBounds) -> impl IntoElement {
    canvas(
        move |bounds, _window, _cx| sink.borrow_mut().push(bounds),
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
}

/// True when `pos` falls inside any recorded open-flyout rectangle.
fn in_flyout(flyouts: &FlyoutBounds, pos: Point<Pixels>) -> bool {
    flyouts.borrow().iter().any(|b| b.contains(&pos))
}

/// Assumed flyout width used only to decide whether a submenu has room to
/// open to the right of its trigger row — the panel's real width isn't known
/// until its own layout runs (cards in this file are `min_w` 160px but grow
/// with content), so this is a conservative overestimate. Mirrors the same
/// kind of margin Zed's `ContextMenu::open_submenu` uses for this decision.
const ASSUMED_FLYOUT_WIDTH: Pixels = px(200.0);

thread_local! {
    /// Most recently observed window-space bounds of each submenu trigger
    /// row, keyed by the flyout's element id (`ctxsubfly-{id}-{depth}`).
    /// Recorded every frame by [`trigger_probe`] so [`record_submenu_side`]
    /// always has a fresh measurement to work from as soon as the trigger is
    /// hovered.
    static TRIGGER_BOUNDS: RefCell<HashMap<SharedString, Bounds<Pixels>>> =
        RefCell::new(HashMap::new());

    /// Whether each submenu flyout should open to the left of its trigger row
    /// instead of the right, keyed the same way as `TRIGGER_BOUNDS`. Decided
    /// in [`record_submenu_side`] when the trigger is hovered and read back
    /// by `render_item` on the next render — menus in this file are rebuilt
    /// fresh from caller state every render, so this cache is the only place
    /// the decision can live between "hover fires" and "the flyout is built".
    static SUBMENU_FLIP_LEFT: RefCell<HashMap<SharedString, bool>> = RefCell::new(HashMap::new());
}

/// A layout-neutral probe that continuously records the trigger row's own
/// bounds into [`TRIGGER_BOUNDS`], mirroring [`flyout_probe`] but for the row
/// rather than the flyout.
fn trigger_probe(key: SharedString) -> impl IntoElement {
    canvas(
        move |bounds, _window, _cx| {
            TRIGGER_BOUNDS.with(|b| b.borrow_mut().insert(key.clone(), bounds));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
}

/// Recomputes whether the submenu keyed by `key` has room to open to the
/// right of its trigger row, using the row's last-probed bounds and the
/// window's current viewport, and caches the answer in [`SUBMENU_FLIP_LEFT`]
/// for `render_item` to pick up on the next render. Called from the trigger
/// row's `on_hover` (for both controlled and CSS-hover submenus) so the
/// flyout already opens on the correct side by the time it becomes visible,
/// rather than only correcting itself a frame after rendering off-screen.
fn record_submenu_side(key: &SharedString, window: &Window) {
    let Some(bounds) = TRIGGER_BOUNDS.with(|b| b.borrow().get(key).cloned()) else {
        return;
    };
    let flip_left = bounds.right() + ASSUMED_FLYOUT_WIDTH > window.viewport_size().width;
    SUBMENU_FLIP_LEFT.with(|f| f.borrow_mut().insert(key.clone(), flip_left));
}

/// A fixed 16px centred box holding a 14px glyph — keeps every row's icon the
/// same size and every label aligned to the same left edge regardless of the
/// individual SVG's internal padding.
fn icon_slot(icon: IconName, color: gpui::Hsla) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .size(px(16.0))
        .child(icon.svg(color).size(px(14.0)))
}

fn render_item(item: MenuItem, c: Palette, depth: usize, flyouts: &FlyoutBounds) -> AnyElement {
    match item.kind {
        Kind::Separator => div()
            .my(c.space(4.0))
            .h(px(1.0))
            .bg(c.border)
            .into_any_element(),
        Kind::Label(text) => div()
            .px(c.space(8.0))
            .py(c.space(4.0))
            .text_size(px(11.0))
            .text_color(c.muted)
            .child(text)
            .into_any_element(),
        Kind::Action {
            id,
            label,
            icon,
            detail,
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
                .px(c.space(8.0))
                .py(c.space(6.0))
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(text_color)
                .when(checked || icon.is_some(), |d| {
                    let glyph = if checked {
                        IconName::CircleCheck
                    } else {
                        icon.unwrap()
                    };
                    d.child(icon_slot(glyph, text_color))
                })
                .child(label)
                .when_some(detail, |d, text| {
                    d.child(
                        div()
                            .ml_auto()
                            .pl(c.space(8.0))
                            .text_size(px(12.0))
                            .text_color(c.muted)
                            .child(text),
                    )
                })
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
            control,
        } => {
            let group = SharedString::from(format!("ctxsub-{id}-{depth}"));
            let flyout_id = SharedString::from(format!("ctxsubfly-{id}-{depth}"));

            // In controlled mode the flyout only exists while the caller says
            // it's open; otherwise it's always in the tree and revealed by CSS.
            let show_panel = control.as_ref().map(|c| c.open).unwrap_or(true);
            let flip_left = SUBMENU_FLIP_LEFT
                .with(|f| f.borrow().get(&flyout_id).copied())
                .unwrap_or(false);
            let panel = show_panel.then(|| {
                let card = div()
                    .id(flyout_id.clone())
                    .flex()
                    .flex_col()
                    .min_w(c.space(160.0))
                    .p(c.space(4.0))
                    .rounded_md()
                    .bg(c.popover)
                    .border_1()
                    .border_color(c.border)
                    .shadow_lg()
                    .children(
                        items
                            .into_iter()
                            .map(|it| render_item(it, c, depth + 1, flyouts)),
                    );
                let card = match &control {
                    Some(ctrl) => {
                        let on_hover = ctrl.on_hover.clone();
                        card.occlude()
                            .on_hover(move |h, w, cx| {
                                on_hover(SubmenuHoverSource::Flyout, *h, w, cx)
                            })
                            // Report this flyout's rect so a click inside it
                            // isn't treated as an outside-press on the card.
                            .child(flyout_probe(flyouts.clone()))
                    }
                    // CSS-hover mode: flush edge so the pointer crosses from
                    // the trigger into the panel without passing over dead
                    // space, which would drop `group_hover`.
                    None => card
                        .invisible()
                        .group_hover(group.clone(), |s| s.visible())
                        .hover(|s| s.visible()),
                };

                // The host pins itself to the trigger row's top-right corner
                // (open right) or top-left corner (`flip_left` — open left)
                // via CSS-style percentage insets, which Taffy resolves
                // against the row's real measured width. `anchored()` (no
                // explicit `.position()`) then reads that resolved point as
                // its own origin, and `snap_to_window_with_margin` slides the
                // flyout back inside the window if it still doesn't fit on
                // either axis — the same idea as the top-level menu card's
                // `snap_to_window()`, just anchored to the row instead of a
                // click point.
                let host = div()
                    .absolute()
                    .top(c.space(4.0))
                    .when(flip_left, |d| d.right_full())
                    .when(!flip_left, |d| d.left_full())
                    // Controlled open/close doesn't depend on an unbroken
                    // hover chain, so the flyout can sit clear of the parent
                    // card instead of flush against it.
                    .when(control.is_some() && !flip_left, |d| d.ml(c.space(6.0)))
                    .when(control.is_some() && flip_left, |d| d.mr(c.space(6.0)));

                host.child(
                    anchored()
                        .anchor(if flip_left {
                            Corner::TopRight
                        } else {
                            Corner::TopLeft
                        })
                        .snap_to_window_with_margin(px(8.0))
                        .child(card),
                )
            });

            let mut row = div()
                .id(id)
                .relative()
                .flex()
                .items_center()
                .gap_2()
                .w_full()
                .px(c.space(8.0))
                .py(c.space(6.0))
                .rounded_sm()
                .text_size(px(13.0))
                .text_color(c.popover_fg)
                .hover(|s| s.bg(c.accent).text_color(c.accent_fg))
                .when_some(icon, |d, ic| d.child(icon_slot(ic, c.popover_fg)))
                .child(label)
                .child(
                    div()
                        .ml_auto()
                        .child(IconName::ChevronRight.svg(c.muted).size(px(14.0))),
                );
            row = match control {
                Some(ctrl) => {
                    let on_hover = ctrl.on_hover.clone();
                    let key = flyout_id.clone();
                    row.on_hover(move |h, w, cx| {
                        if *h {
                            record_submenu_side(&key, w);
                        }
                        on_hover(SubmenuHoverSource::Trigger, *h, w, cx)
                    })
                }
                None => {
                    let key = flyout_id.clone();
                    row.group(group).on_hover(move |h, w, _cx| {
                        if *h {
                            record_submenu_side(&key, w);
                        }
                    })
                }
            };
            row.child(trigger_probe(flyout_id))
                .children(panel)
                .into_any_element()
        }
    }
}

/// The menu card itself — the `p-1 rounded-md bg-popover border shadow-md`
/// panel shared by [`context_menu`] and [`popover_menu`].
fn menu_card(c: Palette, items: Vec<MenuItem>, flyouts: FlyoutBounds) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .min_w(c.space(160.0))
        .max_w(c.space(320.0))
        .p(c.space(4.0))
        .rounded_md()
        .bg(c.popover)
        .border_1()
        .border_color(c.border)
        .shadow_lg()
        .occlude()
        .children(
            items
                .into_iter()
                .map(move |it| render_item(it, c, 0, &flyouts)),
        )
}

/// The bare menu card on its own — no full-screen backdrop, no anchoring, no
/// dismiss wiring. Used by the component gallery (T20-004) to show a context
/// menu as a permanently-open example; not for normal call sites, which want
/// [`context_menu`] / [`popover_menu`].
#[cfg(any(debug_assertions, feature = "gallery"))]
pub fn menu_card_preview(c: Palette, items: Vec<MenuItem>) -> gpui::Div {
    menu_card(c, items, Rc::new(RefCell::new(Vec::new())))
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
    let d3 = dismiss.clone();

    // Open submenu flyouts record their rects here each frame so the dismiss
    // paths below can skip a press that landed inside a flyout (which paints
    // outside the card's own bounds). See [`FlyoutBounds`].
    let flyouts: FlyoutBounds = Rc::new(RefCell::new(Vec::new()));
    let fb_out = flyouts.clone();
    let fb_left = flyouts.clone();
    let fb_right = flyouts.clone();

    // `anchored().snap_to_window()` positions the card in *window* coordinates
    // (the right-click `MouseDownEvent::position` is already window-space) and
    // flips it back inside the viewport near an edge; `deferred(..)` lifts the
    // whole overlay above every sibling so an ancestor's `overflow_hidden`
    // cannot clip it and it always paints on top. Previously this was a plain
    // `absolute().inset_0()` child of whatever view opened it, which made the
    // menu mispositioned (anchor is window-space, parent is not the window),
    // clipped by scrollable panels, and sometimes hidden behind later siblings.
    // `on_mouse_down_out` on the card itself is the reliable dismiss path: the
    // full-screen backdrop below is laid out relative to whichever (often tiny,
    // e.g. a 20px status-bar item) element opened the menu, so `inset_0` does
    // not actually cover the window and a click next to the menu would leave it
    // stuck open. The card, by contrast, always knows its own bounds.
    let card = anchored().position(anchor).snap_to_window().child(
        menu_card(c, items, flyouts)
            .on_mouse_down_out(move |ev, w, cx| {
                if !in_flyout(&fb_out, ev.position) {
                    d3(w, cx)
                }
            })
            .with_animation("context-menu-fade", fade_in(c), |el, delta| {
                el.opacity(delta)
            }),
    );

    deferred(
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                move |ev: &MouseDownEvent, w: &mut Window, cx: &mut App| {
                    if !in_flyout(&fb_left, ev.position) {
                        dismiss(w, cx)
                    }
                },
            )
            .on_mouse_down(
                MouseButton::Right,
                move |ev: &MouseDownEvent, w: &mut Window, cx: &mut App| {
                    if !in_flyout(&fb_right, ev.position) {
                        d2(w, cx)
                    }
                },
            )
            .child(card),
    )
    .with_priority(200)
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
    let dismiss = Rc::new(dismiss);
    let d_out = dismiss.clone();

    let flyouts: FlyoutBounds = Rc::new(RefCell::new(Vec::new()));
    let fb_out = flyouts.clone();
    let fb_bd = flyouts.clone();

    let card = anchored().position(anchor).snap_to_window().child(
        menu_card(c, items, flyouts)
            .on_mouse_down_out(move |ev, w, cx| {
                if !in_flyout(&fb_out, ev.position) {
                    d_out(w, cx)
                }
            })
            .with_animation("popover-menu-fade", fade_in(c), |el, delta| {
                el.opacity(delta)
            }),
    );

    deferred(
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                move |ev: &MouseDownEvent, w: &mut Window, cx: &mut App| {
                    if !in_flyout(&fb_bd, ev.position) {
                        dismiss(w, cx)
                    }
                },
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

    #[test]
    fn in_flyout_matches_recorded_rects() {
        use gpui::{point, px, size};
        let flyouts: FlyoutBounds = Rc::new(RefCell::new(Vec::new()));
        // No flyout open → every press is "outside", menu dismisses.
        assert!(!in_flyout(&flyouts, point(px(150.0), px(80.0))));

        flyouts.borrow_mut().push(Bounds {
            origin: point(px(120.0), px(40.0)),
            size: size(px(160.0), px(120.0)),
        });
        // A press inside the flyout rect must not count as an outside-press…
        assert!(in_flyout(&flyouts, point(px(150.0), px(80.0))));
        // …but one clearly outside it still does.
        assert!(!in_flyout(&flyouts, point(px(400.0), px(400.0))));
    }
}
