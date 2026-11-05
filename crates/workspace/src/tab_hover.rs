//! Hover list of a pane-group tab (R10-002 "Hover-Liste der Panes").
//!
//! Hovering a tab with two or more panes opens, after a short delay, a small
//! popover that lists its panes (icon, title, active marker); a click focuses
//! one. Leaving the tab or the list closes it after a grace period so the
//! pointer can travel from the tab into the list. It is suppressed while a
//! context menu, another popover, a rename or a drag is active
//! ([`crate::pane_group_naming::hover_list_allowed`]).
//!
//! The timing decisions live in [`HoverIntent`], a pure generation counter:
//! every enter/leave bumps it, and a delayed open/close only acts when its
//! generation is still current. That is what prevents flicker when the
//! pointer sweeps across several tabs.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Pixels,
    Point, StatefulInteractiveElement, Styled, Window,
};
use labonair_ui_kit::{popover_with_exit, IconName, ListItem, Palette, PresenceSlot};

use crate::pane::PaneId;
use crate::pane_group_naming::{hover_list_allowed, pane_list, HoverGate};
use crate::Workspace;

/// How long the pointer must rest on a tab before the list opens.
pub(crate) const HOVER_DELAY: Duration = Duration::from_millis(350);
/// Delay when another tab's list is already open (switching feels immediate).
pub(crate) const SWITCH_DELAY: Duration = Duration::from_millis(80);
/// How long the list survives after the pointer left both the tab and itself.
pub(crate) const LEAVE_GRACE: Duration = Duration::from_millis(220);

const LIST_WIDTH: Pixels = px(240.0);

/// Pure hover-intent tracking (see the module docs).
#[derive(Debug, Default)]
pub(crate) struct HoverIntent {
    generation: u64,
    over_tab: Option<u64>,
    over_list: bool,
}

impl HoverIntent {
    /// The pointer entered `tab`; returns the generation a delayed open carries.
    pub(crate) fn enter_tab(&mut self, tab: u64) -> u64 {
        self.over_tab = Some(tab);
        self.bump()
    }

    /// The pointer left the tab; returns the generation a delayed close carries.
    pub(crate) fn leave_tab(&mut self) -> u64 {
        self.over_tab = None;
        self.bump()
    }

    pub(crate) fn enter_list(&mut self) -> u64 {
        self.over_list = true;
        self.bump()
    }

    pub(crate) fn leave_list(&mut self) -> u64 {
        self.over_list = false;
        self.bump()
    }

    /// Forget everything (the list was dismissed or a menu took over).
    pub(crate) fn reset(&mut self) {
        self.over_tab = None;
        self.over_list = false;
        self.bump();
    }

    /// Whether a delayed open for `tab` scheduled at `generation` still applies.
    pub(crate) fn open_due(&self, generation: u64, tab: u64) -> bool {
        self.generation == generation && self.over_tab == Some(tab)
    }

    /// Whether a delayed close scheduled at `generation` still applies.
    pub(crate) fn close_due(&self, generation: u64) -> bool {
        self.generation == generation && self.over_tab.is_none() && !self.over_list
    }

    fn bump(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }
}

/// What the open list is about.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HoverTarget {
    pub tab: u64,
    pub anchor: Point<Pixels>,
}

/// The one shared hover-list surface of the workspace.
#[derive(Default)]
pub(crate) struct TabHover {
    intent: HoverIntent,
    slot: PresenceSlot<HoverTarget>,
}

impl Workspace {
    /// Suppression inputs for the hover list.
    fn tab_hover_gate(&self, cx: &App) -> HoverGate {
        HoverGate {
            context_menu_open: self.context_menu.mounted().is_some()
                || self.new_tab_menu.mounted().is_some()
                || self.space_actions_menu.mounted().is_some(),
            dragging: cx.has_active_drag(),
            popover_open: self.spaces_menu.mounted().is_some()
                || self.new_space_menu.mounted().is_some()
                || self.rename_tab.is_some()
                || self.confirm_close.is_some()
                || self.ssh_prompt.is_some(),
        }
    }

    fn tab_hover_allowed(&self, tab: u64, cx: &App) -> bool {
        hover_list_allowed(self.tab_hover_gate(cx), self.tab_pane_ids(tab).len())
    }

    /// Pointer entered (`hovered`) or left tab `tab`; `anchor` is where the
    /// list would open.
    pub(crate) fn tab_hover_changed(
        &mut self,
        tab: u64,
        hovered: bool,
        anchor: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            let generation = self.tab_hover.intent.enter_tab(tab);
            if !self.tab_hover_allowed(tab, cx) {
                return;
            }
            let delay = if self.tab_hover.slot.is_open() {
                SWITCH_DELAY
            } else {
                HOVER_DELAY
            };
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(delay).await;
                let _ = this.update(cx, |this, cx| {
                    if !this.tab_hover.intent.open_due(generation, tab)
                        || !this.tab_hover_allowed(tab, cx)
                    {
                        return;
                    }
                    this.tab_hover.slot.open(HoverTarget { tab, anchor });
                    cx.notify();
                });
            })
            .detach();
        } else {
            let generation = self.tab_hover.intent.leave_tab();
            self.schedule_tab_hover_close(generation, cx);
        }
    }

    fn schedule_tab_hover_close(&mut self, generation: u64, cx: &mut Context<Self>) {
        if !self.tab_hover.slot.is_open() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LEAVE_GRACE).await;
            let _ = this.update(cx, |this, cx| {
                if this.tab_hover.intent.close_due(generation) {
                    this.close_tab_hover(cx);
                }
            });
        })
        .detach();
    }

    /// Hide the hover list (fade out) and forget the hover intent.
    pub(crate) fn close_tab_hover(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.tab_hover.intent.reset();
        self.tab_hover.slot.close(&motion);
        cx.notify();
    }

    /// Build the hover list while it is mounted. Also ticks its exit fade and
    /// closes it when a suppression condition appeared (menu, drag, rename) or
    /// the tab no longer has several panes. Call once per render.
    pub(crate) fn render_tab_hover(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let motion = self.motion(cx);
        self.tab_hover.slot.tick(&motion);
        let (target, closing) = {
            let (target, closing) = self.tab_hover.slot.mounted()?;
            (*target, closing)
        };
        if !closing && !self.tab_hover_allowed(target.tab, cx) {
            self.close_tab_hover(cx);
        }
        let closing = closing || !self.tab_hover.slot.is_open();
        let active = self.layouts.get(&target.tab).and_then(|l| l.active);
        let entries = pane_list(&self.tab_pane_ids(target.tab), &self.pane_meta, active);
        let c = Palette::from_theme(self.theme.read(cx));
        let rows: Vec<AnyElement> = entries
            .into_iter()
            .map(|entry| {
                let pane: PaneId = entry.id;
                ListItem::new(("tab-hover-pane", pane), c)
                    .icon(entry.icon)
                    .selected(entry.active)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(entry.title),
                    )
                    .when(entry.active, |row| {
                        row.trailing(IconName::Check.svg(c.muted).size(px(12.0)))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_tab_hover(cx);
                        this.activate_pane(pane, window, cx);
                    }))
                    .into_any_element()
            })
            .collect();
        if rows.len() < 2 {
            self.close_tab_hover(cx);
            return None;
        }
        let content = div()
            .id("tab-hover-list")
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(4.0))
            .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                let generation = if *hovered {
                    this.tab_hover.intent.enter_list()
                } else {
                    this.tab_hover.intent.leave_list()
                };
                if !*hovered {
                    this.schedule_tab_hover_close(generation, cx);
                }
            }))
            .children(rows)
            .into_any_element();
        let view = cx.entity();
        Some(popover_with_exit(
            target.anchor,
            LIST_WIDTH,
            c,
            closing,
            move |_window: &mut Window, app: &mut App| {
                view.update(app, |this, cx| this.close_tab_hover(cx));
            },
            content,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_needs_the_pointer_to_stay_on_the_same_tab() {
        let mut i = HoverIntent::default();
        let g = i.enter_tab(1);
        assert!(i.open_due(g, 1));
        // Sweeping to another tab invalidates the first open.
        let g2 = i.enter_tab(2);
        assert!(!i.open_due(g, 1));
        assert!(i.open_due(g2, 2));
        assert!(!i.open_due(g2, 1));
    }

    #[test]
    fn leaving_before_the_delay_cancels_the_open() {
        let mut i = HoverIntent::default();
        let g = i.enter_tab(1);
        i.leave_tab();
        assert!(!i.open_due(g, 1));
    }

    #[test]
    fn close_waits_for_the_pointer_to_leave_tab_and_list() {
        let mut i = HoverIntent::default();
        i.enter_tab(1);
        let leave = i.leave_tab();
        // Pointer reaches the list within the grace period: no close.
        i.enter_list();
        assert!(!i.close_due(leave));
        let leave_list = i.leave_list();
        assert!(i.close_due(leave_list));
        // Re-entering the tab cancels the pending close.
        i.enter_tab(1);
        assert!(!i.close_due(leave_list));
    }

    #[test]
    fn reset_invalidates_pending_work() {
        let mut i = HoverIntent::default();
        let g = i.enter_tab(1);
        i.reset();
        assert!(!i.open_due(g, 1));
    }

    #[test]
    fn gate_suppresses_the_list() {
        let open = HoverGate::default();
        assert!(hover_list_allowed(open, 2));
        assert!(!hover_list_allowed(open, 1));
        for gate in [
            HoverGate {
                context_menu_open: true,
                ..open
            },
            HoverGate {
                dragging: true,
                ..open
            },
            HoverGate {
                popover_open: true,
                ..open
            },
        ] {
            assert!(!hover_list_allowed(gate, 3));
        }
    }
}
