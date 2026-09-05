//! The status bar (T17-003).
//!
//! Replaces the shell's former `render_bar_item` `match` cascade: this
//! component renders **only** from the
//! [`StatusItemRegistry`](labonair_panel::StatusItemRegistry) that
//! `labonair_shell::register_builtin_status_items` populates on the
//! [`Workspace`]. Each registered item is a self-describing
//! [`StatusItem`](labonair_panel::StatusItem) view: it names its `id`, its
//! `default_side` and its `order`, and renders its own content. Left = panel
//! controls, right = info dropdowns (`docs/architecture.md` §4).
//!
//! Personalization (T18-005): every item except the structural per-dock panel-
//! button groups (`not_moveable`, see [`StatusItem::hideable`] /
//! `crate::status_placements`) gets a right-click menu — "Move left" / "Move
//! right" / "Hide" — that calls [`Workspace::set_status_bar_placement`]. The
//! side/hidden overrides live on the registry
//! ([`StatusItemRegistry::resolve_side`] / [`StatusItemRegistry::is_hidden`]);
//! this component just reads them each render and re-reads them from disk
//! whenever [`crate::status_placements::StatusBarLayoutTick`] bumps (another
//! window persisted a change).

use gpui::{
    div, px, AnyElement, AnyView, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString,
    Styled, Window,
};
use labonair_panel::{AnyStatusItemHandle, StatusItemConstructor, StatusSide};
use labonair_ui_kit::{context_menu, MenuItem, Palette};

use crate::status_placements::StatusBarLayoutTick;
use crate::theme::ThemeStore;
use crate::Workspace;

/// Status-bar row height — matches the shell's former `STATUS_H`.
const STATUS_H: f32 = 32.0;

/// The per-dock panel-button groups are structural, not ordinary movable
/// status items. They own their placement (left group at the left edge, bottom
/// and right groups at the right edge) and carry their own right-click menu for
/// moving the panel between docks or hiding it, so this shared placement menu
/// skips them.
fn not_moveable(id: &str) -> bool {
    matches!(
        id,
        "dock-buttons-left" | "dock-buttons-right" | "dock-buttons-bottom"
    )
}

/// Renders the registered [`StatusItem`](labonair_panel::StatusItem)s, sorted
/// by `order` within each side.
pub struct StatusBar {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    /// Built once from the registry: `(default_side, order, group, id, handle)`.
    items: Vec<(StatusSide, i32, u32, &'static str, AnyStatusItemHandle)>,
    built: bool,
    /// The open right-click placement menu, if any: `(item id, anchor)`.
    menu: Option<(&'static str, Point<Pixels>)>,
    /// Focus for the toolbar/tab-group keyboard contract (Zed-parity redesign
    /// Phase 5.1 — the piece Phase 1 explicitly deferred). Once focus is inside
    /// the bar, Left/Right Arrow walk the tab stops of the panel-button groups
    /// and the movable status items.
    focus: FocusHandle,
}

impl StatusBar {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        // Another window persisted a placement change — reload the blob and
        // re-render (T18-005 point 8, "two windows").
        cx.observe_global::<StatusBarLayoutTick>(|this, cx| {
            this.workspace
                .update(cx, |w, _| w.reload_status_bar_placements());
            cx.notify();
        })
        .detach();
        Self {
            workspace,
            theme,
            items: Vec::new(),
            built: false,
            menu: None,
            focus: cx.focus_handle(),
        }
    }

    /// Left/Right Arrow move focus across the bar's tab stops while focus is
    /// inside it; Escape hands focus back to the workspace. This is the
    /// clean-room equivalent of Zed's status-bar toolbar/tab-group arrow loop
    /// (§6.4). GPUI 0.2.2 has no ARIA-role API, so the semantics are carried by
    /// `key_context` + tab stops + per-button tooltips-with-shortcut.
    // TODO(a11y): replace with a real toolbar/button role + accessible-name API
    // once GPUI exposes one.
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "right" => {
                window.focus_next();
                cx.stop_propagation();
            }
            "left" => {
                window.focus_prev();
                cx.stop_propagation();
            }
            "escape" => {
                window.blur();
                cx.stop_propagation();
            }
            _ => {}
        }
    }

    /// Instantiate every registered item once (the constructors hand back a
    /// clone of a pre-built entity, so this is cheap refcount work — mirrors
    /// `Workspace::init_docks`). Registration happens at startup before the
    /// first render, so a single lazy build is enough.
    fn ensure_built(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.built {
            return;
        }
        let regs: Vec<(StatusSide, i32, u32, &'static str, StatusItemConstructor)> = self
            .workspace
            .read(cx)
            .status_item_registry()
            .iter()
            .map(|r| (r.default_side, r.order, r.group, r.id, r.build.clone()))
            .collect();
        if regs.is_empty() {
            return;
        }
        for (side, order, group, id, build) in regs {
            let handle = build(window, cx);
            self.items.push((side, order, group, id, handle));
        }
        self.built = true;
    }

    fn open_menu(&mut self, id: &'static str, pos: Point<Pixels>, cx: &mut Context<Self>) {
        self.menu = Some((id, pos));
        cx.notify();
    }

    /// The right-click "move left/right/hide" menu for `id` (T18-005 point 3).
    fn render_menu(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (id, pos) = self.menu?;
        let registry_snapshot = {
            let ws = self.workspace.read(cx);
            let registry = ws.status_item_registry();
            (registry.resolve_side(id), registry.is_hidden(id))
        };
        let (side, hidden) = registry_snapshot;
        let hideable = self
            .items
            .iter()
            .find(|(_, _, _, item_id, _)| *item_id == id)
            .map(|(_, _, _, _, h)| h.hideable(cx))
            .unwrap_or(false);

        let view = cx.entity();
        let close = {
            let v = view.clone();
            move |cx: &mut gpui::App| {
                v.update(cx, |this, cx| {
                    this.menu = None;
                    cx.notify();
                })
            }
        };

        let mut items: Vec<MenuItem> = Vec::new();
        {
            let ws = self.workspace.clone();
            let close = close.clone();
            items.push(
                MenuItem::new("status-move-left", "Move left")
                    .disabled(side == StatusSide::Left)
                    .on_click(move |_, _w, cx| {
                        ws.update(cx, |w, cx| {
                            w.set_status_bar_placement(id, Some(StatusSide::Left), None, cx);
                        });
                        close(cx);
                    }),
            );
        }
        {
            let ws = self.workspace.clone();
            let close = close.clone();
            items.push(
                MenuItem::new("status-move-right", "Move right")
                    .disabled(side == StatusSide::Right)
                    .on_click(move |_, _w, cx| {
                        ws.update(cx, |w, cx| {
                            w.set_status_bar_placement(id, Some(StatusSide::Right), None, cx);
                        });
                        close(cx);
                    }),
            );
        }
        if hideable {
            items.push(MenuItem::separator());
            let ws = self.workspace.clone();
            let close = close.clone();
            items.push(
                MenuItem::new("status-hide", "Hide")
                    .disabled(hidden)
                    .on_click(move |_, _w, cx| {
                        ws.update(cx, |w, cx| {
                            w.set_status_bar_placement(id, None, Some(true), cx);
                        });
                        close(cx);
                    }),
            );
        }

        let dismiss = move |_w: &mut Window, cx: &mut gpui::App| close(cx);
        Some(context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        ))
    }

    /// Views for `side` (resolved through the registry's overrides — T18-005),
    /// sorted by `order`, with a divider inserted between two consecutive
    /// items whose `group` differs (T18-004 point 8 — dividers only between
    /// logical groups, never between every item). Hidden items are omitted.
    fn cluster(&mut self, side: StatusSide, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let registry_state: Vec<(&'static str, StatusSide, bool)> = {
            let ws = self.workspace.read(cx);
            let registry = ws.status_item_registry();
            self.items
                .iter()
                .map(|(_, _, _, id, _)| (*id, registry.resolve_side(id), registry.is_hidden(id)))
                .collect()
        };

        let mut v: Vec<(i32, u32, &'static str, AnyView)> = self
            .items
            .iter()
            .zip(registry_state.iter())
            .filter(|(_, (_, resolved_side, hidden))| *resolved_side == side && !hidden)
            .map(|((_, order, group, id, h), _)| (*order, *group, *id, h.to_any()))
            .collect();
        v.sort_by_key(|(order, _, _, _)| *order);

        let border = self.theme.read(cx).border();
        let menu_open_id = self.menu.map(|(id, _)| id);
        let mut out = Vec::with_capacity(v.len() * 2);
        let mut prev_group: Option<u32> = None;
        for (_, group, id, view) in v {
            if let Some(pg) = prev_group {
                if pg != group {
                    out.push(
                        div()
                            .flex_shrink_0()
                            .w(px(1.0))
                            .h(px(14.0))
                            .bg(border)
                            .into_any_element(),
                    );
                }
            }
            prev_group = Some(group);

            if not_moveable(id) {
                out.push(view.into_any_element());
                continue;
            }

            let menu = (menu_open_id == Some(id))
                .then(|| self.render_menu(cx))
                .flatten();
            out.push(
                div()
                    .id(SharedString::from(format!("status-item-{id}")))
                    .relative()
                    .flex()
                    .items_center()
                    .child(view.into_any_element())
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            this.open_menu(id, ev.position, cx);
                        }),
                    )
                    .children(menu)
                    .into_any_element(),
            );
        }
        out
    }
}

impl Render for StatusBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span =
            tracing::trace_span!(target: "labonair::perf", "render", view = "status_bar").entered();
        self.ensure_built(window, cx);

        let (status_bar_bg, muted, border) = {
            let t = self.theme.read(cx);
            (t.status_bar(), t.muted_foreground(), t.border())
        };
        let left = self.cluster(StatusSide::Left, cx);
        let right = self.cluster(StatusSide::Right, cx);

        div()
            .track_focus(&self.focus)
            .key_context("StatusBar")
            .on_key_down(cx.listener(Self::on_key_down))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(STATUS_H))
            .w_full()
            .flex_shrink_0()
            .px_3()
            .bg(status_bar_bg)
            .border_t_1()
            .border_color(border)
            .text_size(px(11.0))
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .children(left),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_1()
                    .children(right),
            )
    }
}
