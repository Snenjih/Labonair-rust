//! Applying tab / pane drags to the live workspace (R10-002, ADR 0005).
//!
//! The geometry and tree surgery are pure and live in [`crate::pane_dnd`];
//! this module owns the hover state, the drop-zone overlay, the drop handlers
//! of the pane area, the tab blocks and the strip, Esc-cancel and strip
//! auto-scroll, and it applies a [`DropPlan`] to the pane stores: layouts,
//! tab metadata, SSH tab ids, zoom and focus.
//!
//! Priorities. Handlers are typed, so a `DraggedTab` never reaches a
//! `DraggedPane` handler and vice versa; the innermost hovered element with a
//! matching `on_drop` wins (a tab block before the strip). Pane areas react to
//! both payloads, tab blocks to `DraggedTab` (reorder) and `DraggedPane`
//! (join the group), the strip to `DraggedPane` (detach into a new tab). The
//! overlay and the handlers carry no occluding hitbox, so `ExternalPaths` and
//! `DraggedPaths` drops keep reaching terminals and editors.
//!
//! Zoom: while a tab is zoomed its pane area accepts no drops (the zoomed
//! pane covers the tree, so the zones would not match what the user sees);
//! moving a pane or merging a tab ends the zoom of both tabs involved.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, AnyElement, App, Bounds, Context, Div, DragMoveEvent, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Pixels, Point, Styled, Window,
};
use labonair_ui_kit::{Density, MotionExt, Palette};

use crate::pane::PaneId;
use crate::pane_dnd::{
    drop_allowed, edge_scroll, hit_test, plan_detach, plan_pane_drop, plan_tab_drop, preview_rect,
    spring_candidate, verdict_of, DropError, DropGuard, DropPlan, Payload, SourceAfter, Verdict,
};
use crate::pane_kind::PaneKind;
use crate::pane_ops::DropZone;
use crate::tabs::TabData;
use crate::tiling::{self, PxRect};
use crate::{DraggedPane, DraggedTab, Workspace};

/// How long a drag rests on a tab block before that tab opens (spring-loaded
/// tabs), so the user can drop into its panes.
const SPRING_DELAY: Duration = Duration::from_millis(500);

/// The drop target currently highlighted in a pane area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DropHover {
    pub tab: u64,
    pub pane: PaneId,
    /// Raw zone (centre = swap or smart split, decided by the payload).
    pub zone: DropZone,
    /// Highlight rectangle in window coordinates.
    pub preview: PxRect,
    /// Whether the drop would succeed; decided by a dry run of the plan.
    pub verdict: Verdict,
}

/// Transient drag-over state. Only meaningful while a drag is active; every
/// render checks `cx.has_active_drag()`, and it is reset on drop and Esc.
#[derive(Default)]
pub(crate) struct PaneDnd {
    /// Pane-area zone under the pointer.
    pub hover: Option<DropHover>,
    /// A pane dragged over this tab block (it would join the group).
    pub tab_join: Option<(u64, Verdict)>,
    /// The tab block under a dragged pane, whether or not it accepts the
    /// drop: the strip's new-tab marker only shows between blocks.
    pub over_block: Option<u64>,
    /// A pane dragged over empty strip space: the tab after which the new
    /// tab would appear (the last tab of the pane's Space).
    pub strip_end: Option<u64>,
    /// The tab block the drag rests on; it opens after [`SPRING_DELAY`].
    pub spring: Option<u64>,
    /// Bumped on every spring start / reset so stale timers do nothing. Kept
    /// across [`Workspace::clear_pane_dnd`] (a reset must not revive a timer).
    pub spring_gen: u64,
}

/// Fill and border of a drop highlight: primary when the drop fits, warning
/// when it would not.
fn verdict_colors(c: Palette, verdict: Verdict) -> (gpui::Hsla, gpui::Hsla) {
    match verdict {
        Verdict::Fits => (c.primary.opacity(0.18), c.primary.opacity(0.7)),
        Verdict::NoRoom => (c.warning.opacity(0.14), c.warning.opacity(0.8)),
    }
}

/// Highlight for a tab block that a dragged pane would join. Absolutely
/// positioned: add it as a child of the (relative) tab block.
pub(crate) fn tab_join_highlight(c: Palette, verdict: Verdict) -> Div {
    let (bg, border) = verdict_colors(c, verdict);
    div()
        .absolute()
        .inset_0()
        .rounded(px(c.radius.md))
        .bg(bg)
        .border_1()
        .border_color(border)
}

fn to_f32(p: Pixels) -> f32 {
    f32::from(p)
}

impl Workspace {
    // ── hover state ────────────────────────────────────────────────────

    pub(crate) fn clear_pane_dnd(&mut self, cx: &mut Context<Self>) {
        self.pane_dnd = PaneDnd {
            spring_gen: self.pane_dnd.spring_gen.wrapping_add(1),
            ..PaneDnd::default()
        };
        self.tab_strip.drop_target = None;
        cx.notify();
    }

    /// Esc during a drag cancels it (GPUI itself only cancels on mouse-up).
    /// Registered in the capture phase so a focused terminal does not see it.
    pub(crate) fn cancel_drag_on_escape(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ev.keystroke.key == "escape" && cx.has_active_drag() {
            cx.stop_active_drag(window);
            self.clear_pane_dnd(cx);
            cx.stop_propagation();
        }
    }

    fn same_space(&self, a: u64, b: u64, cx: &App) -> bool {
        let tabs = self.tabs.read(cx);
        match (tabs.get(a), tabs.get(b)) {
            (Some(a), Some(b)) => a.space_id == b.space_id,
            _ => false,
        }
    }

    fn payload_allowed(&self, payload: Payload, dst_tab: u64, cx: &App) -> bool {
        let src = match payload {
            Payload::Pane { tab, .. } | Payload::Tab { tab } => tab,
        };
        let guard = DropGuard {
            same_space: self.same_space(src, dst_tab, cx),
            src_pinned: self.tabs.read(cx).get(src).is_some_and(|t| t.pinned),
            dst_pane_count: self.tab_pane_ids(dst_tab).len(),
        };
        drop_allowed(payload, dst_tab, guard)
    }

    // ── pane area ──────────────────────────────────────────────────────

    /// Dry-run the drop of `payload` onto `target` of tab `dst`. `None`: the
    /// drop would do nothing. Layouts that do not exist yet (a tab never
    /// shown) cannot be judged and count as fitting; the real drop creates
    /// them and decides.
    fn dry_run(
        &self,
        payload: Payload,
        dst: u64,
        target: PaneId,
        zone: DropZone,
        cx: &App,
    ) -> Option<Verdict> {
        let (Some(dst_layout), (_, metrics)) = (self.layouts.get(&dst), self.pane_style(cx)) else {
            return Some(Verdict::Fits);
        };
        let container = self.pane_container_px();
        // Throwaway ids: a hover must not consume the real pane id counter.
        let mut next = u64::MAX / 2;
        let mut alloc = move || {
            next += 1;
            next
        };
        let plan = match payload {
            Payload::Pane { pane, tab: src } => {
                let Some(src_layout) = self.layouts.get(&src) else {
                    return Some(Verdict::Fits);
                };
                let dst_layout = (src != dst).then_some(dst_layout);
                plan_pane_drop(
                    src_layout, dst_layout, pane, target, zone, container, &metrics, &mut alloc,
                )
            }
            Payload::Tab { tab: src } => {
                let Some(src_layout) = self.layouts.get(&src) else {
                    return Some(Verdict::Fits);
                };
                plan_tab_drop(
                    src_layout, dst_layout, target, zone, container, &metrics, &mut alloc,
                )
            }
        };
        verdict_of(&plan)
    }

    /// The zone of tab `tab`'s pane area under `pos`, if `payload` may drop
    /// there. `prev` is the current hover: the dry run only repeats when the
    /// pane or zone changed.
    fn drop_hover(
        &self,
        tab: u64,
        payload: Payload,
        pos: Point<Pixels>,
        prev: Option<DropHover>,
        cx: &App,
    ) -> Option<DropHover> {
        let layout = self.layouts.get(&tab)?;
        if self.zoom.zoomed(tab).is_some() || !self.payload_allowed(payload, tab, cx) {
            return None;
        }
        let (_, metrics) = self.pane_style(cx);
        let c = Palette::from_theme(self.theme.read(cx));
        let solved = tiling::solve(&layout.group, self.pane_container_px(), metrics.gap);
        let bar_h = if layout.len() > 1 {
            to_f32(Density::from_palette(&c).pane_bar_height())
        } else {
            0.0
        };
        let hit = hit_test(&solved.panes, to_f32(pos.x), to_f32(pos.y), bar_h)?;
        if matches!(payload, Payload::Pane { pane, tab: t } if t == tab && pane == hit.pane) {
            return None;
        }
        let same = |p: &DropHover| p.tab == tab && p.pane == hit.pane && p.zone == hit.zone;
        let verdict = match prev.filter(same) {
            Some(p) => p.verdict,
            None => self.dry_run(payload, tab, hit.pane, hit.zone, cx)?,
        };
        Some(DropHover {
            tab,
            pane: hit.pane,
            zone: hit.zone,
            preview: preview_rect(payload, tab, hit),
            verdict,
        })
    }

    fn pane_area_drag_over(
        &mut self,
        tab: u64,
        payload: Payload,
        bounds: Bounds<Pixels>,
        pos: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let next = bounds
            .contains(&pos)
            .then(|| self.drop_hover(tab, payload, pos, self.pane_dnd.hover, cx))
            .flatten();
        if self.pane_dnd.hover != next {
            self.pane_dnd.hover = next;
            cx.notify();
        }
    }

    /// Drag handlers and the highlight overlay for a tab's pane area. `area`
    /// is the relative container whose bounds the pane tree fills.
    pub(crate) fn with_pane_dnd(&self, area: Div, tab_id: u64, cx: &mut Context<Self>) -> Div {
        let overlay = self.render_drop_overlay(tab_id, cx);
        area.on_drag_move(
            cx.listener(move |this, ev: &DragMoveEvent<DraggedPane>, _window, cx| {
                let d = ev.drag(cx);
                let payload = Payload::Pane {
                    pane: d.pane_id,
                    tab: d.tab_id,
                };
                this.pane_area_drag_over(tab_id, payload, ev.bounds, ev.event.position, cx);
            }),
        )
        .on_drag_move(
            cx.listener(move |this, ev: &DragMoveEvent<DraggedTab>, _window, cx| {
                let payload = Payload::Tab {
                    tab: ev.drag(cx).id,
                };
                this.pane_area_drag_over(tab_id, payload, ev.bounds, ev.event.position, cx);
            }),
        )
        .on_drop(cx.listener(move |this, d: &DraggedPane, window, cx| {
            let payload = Payload::Pane {
                pane: d.pane_id,
                tab: d.tab_id,
            };
            this.drop_on_pane_area(tab_id, payload, window, cx);
        }))
        .on_drop(cx.listener(move |this, d: &DraggedTab, window, cx| {
            this.drop_on_pane_area(tab_id, Payload::Tab { tab: d.id }, window, cx);
        }))
        .children(overlay)
    }

    /// The zone highlight: a fading rectangle over the half the dropped pane
    /// would occupy. Absolute and input-transparent, so it never shifts the
    /// layout or steals events from the panes below.
    fn render_drop_overlay(&self, tab_id: u64, cx: &App) -> Option<AnyElement> {
        let hover = self
            .pane_dnd
            .hover
            .filter(|h| h.tab == tab_id && cx.has_active_drag())?;
        let c = Palette::from_theme(self.theme.read(cx));
        let container = self.pane_container_px();
        let r = hover.preview;
        let zone_ix = match hover.zone {
            DropZone::Left => 0,
            DropZone::Right => 1,
            DropZone::Top => 2,
            DropZone::Bottom => 3,
            DropZone::Centre => 4,
        };
        let (bg, border) = verdict_colors(c, hover.verdict);
        Some(
            div()
                .absolute()
                .left(px(r.x - container.x))
                .top(px(r.y - container.y))
                .w(px(r.w))
                .h(px(r.h))
                .rounded(px(c.radius.md))
                .bg(bg)
                .border_1()
                .border_color(border)
                .when(hover.verdict == Verdict::NoRoom, |zone| {
                    zone.flex()
                        .items_center()
                        .justify_center()
                        .p_2()
                        .text_xs()
                        .text_center()
                        .text_color(c.warning)
                        .child("No room: panes need at least 240 x 120 px")
                })
                .motion_fade_in(("pane-drop-zone", hover.pane * 8 + zone_ix), c)
                .into_any_element(),
        )
    }

    fn drop_on_pane_area(
        &mut self,
        tab: u64,
        payload: Payload,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let hit = self.drop_hover(tab, payload, window.mouse_position(), None, cx);
        self.clear_pane_dnd(cx);
        let Some(hit) = hit else {
            return;
        };
        match payload {
            Payload::Pane { pane, tab: src } => {
                self.move_pane_to(pane, src, tab, hit.pane, hit.zone, window, cx)
            }
            Payload::Tab { tab: src } => {
                self.merge_tab_into(src, tab, hit.pane, hit.zone, window, cx)
            }
        }
    }

    // ── tab blocks ─────────────────────────────────────────────────────

    /// A dragged pane over (or leaving) tab block `tab`: it would join that
    /// tab's group.
    pub(crate) fn tab_pane_drag_over(
        &mut self,
        tab: u64,
        hovered: bool,
        drag: &DraggedPane,
        cx: &mut Context<Self>,
    ) {
        let payload = Payload::Pane {
            pane: drag.pane_id,
            tab: drag.tab_id,
        };
        let (join, over) = if hovered {
            let join = (drag.tab_id != tab && self.payload_allowed(payload, tab, cx))
                .then(|| {
                    let target = self.focused_pane(tab);
                    self.dry_run(payload, tab, target, DropZone::Centre, cx)
                })
                .flatten()
                .map(|verdict| (tab, verdict));
            (join, Some(tab))
        } else {
            (
                self.pane_dnd.tab_join.filter(|(t, _)| *t != tab),
                self.pane_dnd.over_block.filter(|t| *t != tab),
            )
        };
        if join != self.pane_dnd.tab_join || over != self.pane_dnd.over_block {
            self.pane_dnd.tab_join = join;
            self.pane_dnd.over_block = over;
            cx.notify();
        }
    }

    /// Drop a pane on tab block `tab`: it moves into that tab's group (smart
    /// split of the tab's focused pane).
    pub(crate) fn drop_pane_on_tab(
        &mut self,
        drag: &DraggedPane,
        tab: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_pane_dnd(cx);
        let payload = Payload::Pane {
            pane: drag.pane_id,
            tab: drag.tab_id,
        };
        if drag.tab_id == tab || !self.payload_allowed(payload, tab, cx) {
            return;
        }
        self.ensure_layout(tab, cx);
        let target = self.focused_pane(tab);
        self.move_pane_to(
            drag.pane_id,
            drag.tab_id,
            tab,
            target,
            DropZone::Centre,
            window,
            cx,
        );
    }

    /// A drag moved over (`hovered`) or away from tab block `tab`. Resting on
    /// another tab for [`SPRING_DELAY`] opens it, so a tab or pane can be
    /// dropped into panes of a tab that is not active yet.
    pub(crate) fn tab_drag_spring(
        &mut self,
        tab: u64,
        hovered: bool,
        payload: Payload,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !hovered {
            if self.pane_dnd.spring == Some(tab) {
                self.pane_dnd.spring = None;
                self.pane_dnd.spring_gen = self.pane_dnd.spring_gen.wrapping_add(1);
            }
            return;
        }
        let src = match payload {
            Payload::Pane { tab, .. } | Payload::Tab { tab } => tab,
        };
        let active = self.tabs.read(cx).active_id();
        if self.pane_dnd.spring == Some(tab)
            || !spring_candidate(payload, tab, active, self.same_space(src, tab, cx))
        {
            return;
        }
        self.pane_dnd.spring = Some(tab);
        self.pane_dnd.spring_gen = self.pane_dnd.spring_gen.wrapping_add(1);
        let generation = self.pane_dnd.spring_gen;
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(SPRING_DELAY).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let due = this.pane_dnd.spring_gen == generation
                    && this.pane_dnd.spring == Some(tab)
                    && cx.has_active_drag();
                if due {
                    this.pane_dnd.spring = None;
                    this.pane_dnd.hover = None;
                    this.select_tab(tab, window, cx);
                }
            });
        })
        .detach();
    }

    // ── tab strip / sidebar ────────────────────────────────────────────

    /// A dragged pane over (or leaving) the strip: dropping on empty space
    /// detaches it into a new tab placed at the end of its Space.
    pub(crate) fn strip_pane_drag_over(
        &mut self,
        hovered: bool,
        drag: &DraggedPane,
        cx: &mut Context<Self>,
    ) {
        let next = hovered
            .then(|| {
                let space = self.tabs.read(cx).get(drag.tab_id)?.space_id;
                if self.tab_pane_ids(drag.tab_id).len() < 2 {
                    return None;
                }
                self.tabs.read(cx).tabs_in_space(space).last().map(|t| t.id)
            })
            .flatten();
        if next != self.pane_dnd.strip_end {
            self.pane_dnd.strip_end = next;
            cx.notify();
        }
    }

    /// Drop a pane on empty strip space.
    pub(crate) fn drop_pane_on_strip(
        &mut self,
        drag: &DraggedPane,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_pane_dnd(cx);
        self.detach_pane_to_new_tab(drag.pane_id, window, cx);
    }

    /// Scroll a tab strip while a drag rests near one of its ends. GPUI only
    /// delivers drag-move events when the pointer moves, so the strip scrolls
    /// with the pointer rather than on a timer.
    pub(crate) fn auto_scroll_strip(
        &self,
        vertical: bool,
        bounds: Bounds<Pixels>,
        pos: Point<Pixels>,
    ) {
        if !bounds.contains(&pos) {
            return;
        }
        let (handle, at, start, end) = if vertical {
            (
                &self.tab_strip.scroll_v,
                pos.y,
                bounds.origin.y,
                bounds.origin.y + bounds.size.height,
            )
        } else {
            (
                &self.tab_strip.scroll_h,
                pos.x,
                bounds.origin.x,
                bounds.origin.x + bounds.size.width,
            )
        };
        let delta = edge_scroll(to_f32(at), to_f32(start), to_f32(end));
        if delta == 0.0 {
            return;
        }
        let (offset, max) = (handle.offset(), handle.max_offset());
        let next = if vertical {
            point(
                offset.x,
                px((to_f32(offset.y) + delta).clamp(-to_f32(max.height), 0.0)),
            )
        } else {
            point(
                px((to_f32(offset.x) + delta).clamp(-to_f32(max.width), 0.0)),
                offset.y,
            )
        };
        if next != offset {
            handle.set_offset(next);
        }
    }

    // ── applying drops ─────────────────────────────────────────────────

    fn notify_no_room(&self, cx: &mut Context<Self>) {
        self.notify_pane_problem(
            "No room to move",
            "Every pane needs at least 240 x 120 px. Close a pane or enlarge the window.",
            cx,
        );
    }

    /// Move `pane` (of tab `src`) onto `target` (of tab `dst`) at `zone`.
    #[allow(clippy::too_many_arguments)]
    fn move_pane_to(
        &mut self,
        pane: PaneId,
        src: u64,
        dst: u64,
        target: PaneId,
        zone: DropZone,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(src_layout) = self.layouts.get(&src).cloned() else {
            return;
        };
        let dst_layout = if src == dst {
            None
        } else {
            match self.layouts.get(&dst).cloned() {
                Some(layout) => Some(layout),
                None => return,
            }
        };
        let (_, metrics) = self.pane_style(cx);
        let container = self.pane_container_px();
        let mut alloc = crate::pane::next_id;
        match plan_pane_drop(
            &src_layout,
            dst_layout.as_ref(),
            pane,
            target,
            zone,
            container,
            &metrics,
            &mut alloc,
        ) {
            Ok(plan) => self.apply_drop_plan(plan, vec![pane], src, dst, window, cx),
            Err(DropError::NoRoom) => self.notify_no_room(cx),
            Err(DropError::Invalid) => {}
        }
    }

    /// Merge tab `src` with its whole pane group into tab `dst`.
    fn merge_tab_into(
        &mut self,
        src: u64,
        dst: u64,
        target: PaneId,
        zone: DropZone,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ensure_layout(src, cx);
        self.ensure_layout(dst, cx);
        let (Some(src_layout), Some(dst_layout)) = (
            self.layouts.get(&src).cloned(),
            self.layouts.get(&dst).cloned(),
        ) else {
            return;
        };
        let moved = src_layout.leaves();
        let (_, metrics) = self.pane_style(cx);
        let container = self.pane_container_px();
        let mut alloc = crate::pane::next_id;
        match plan_tab_drop(
            &src_layout,
            &dst_layout,
            target,
            zone,
            container,
            &metrics,
            &mut alloc,
        ) {
            Ok(plan) => self.apply_drop_plan(plan, moved, src, dst, window, cx),
            Err(DropError::NoRoom) => self.notify_no_room(cx),
            Err(DropError::Invalid) => {}
        }
    }

    /// Commit a plan: layouts, SSH tab ids of the moved panes, zoom, tab
    /// metadata of both tabs, selection and focus. Resources keyed by pane id
    /// (views, sessions, editor bindings) need no change; the only
    /// tab-keyed field a pane carries is [`crate::SshTab::tab_id`].
    fn apply_drop_plan(
        &mut self,
        plan: DropPlan,
        moved: Vec<PaneId>,
        src: u64,
        dst: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DropPlan {
            dst: dst_layout,
            src: src_after,
            focus,
        } = plan;
        self.layouts.insert(dst, dst_layout);
        if src != dst {
            match src_after {
                SourceAfter::Layout(rest) => {
                    self.layouts.insert(src, rest);
                }
                SourceAfter::Removed => self.remove_emptied_tab(src, cx),
                SourceAfter::Same => {}
            }
            for ssh in self.ssh_tabs.values_mut() {
                if moved.contains(&ssh.pane_id) {
                    ssh.tab_id = dst;
                }
            }
        }
        self.zoom.clear(src);
        self.zoom.clear(dst);
        self.resync_tab(dst, cx);
        if src != dst && self.layouts.contains_key(&src) {
            self.resync_tab(src, cx);
        }
        if self.tabs.read(cx).active_id() != dst {
            self.tabs.update(cx, |tabs, cx| tabs.set_active(dst, cx));
        }
        self.focus_pane(focus, window, cx);
        cx.notify();
    }

    /// Drop the tab whose last pane moved away, without retiring any pane
    /// (they live on in the target tab).
    fn remove_emptied_tab(&mut self, tab: u64, cx: &mut Context<Self>) {
        self.revoke_tab_grant(tab, cx);
        self.layouts.remove(&tab);
        self.zoom.clear(tab);
        if self.confirm_close == Some(tab) {
            self.confirm_close = None;
        }
        if self.rename_tab.as_ref().is_some_and(|(id, _)| *id == tab) {
            self.rename_tab = None;
        }
        self.tabs.update(cx, |tabs, cx| {
            tabs.close(tab, cx);
        });
    }

    /// Re-derive what a tab shows from its panes after a drop: kind, dirty
    /// marker and, for a lone non-terminal pane, the title and file path
    /// (terminal titles follow the shell).
    fn resync_tab(&mut self, tab: u64, cx: &mut Context<Self>) {
        self.sync_tab_kind(tab, cx);
        self.sync_tab_dirty(tab, cx);
        let panes = self.tab_pane_ids(tab);
        let [pane] = panes[..] else {
            return;
        };
        let spec = self.pane_spec(pane, cx);
        if spec.kind == PaneKind::Terminal {
            return;
        }
        let path = (spec.kind == PaneKind::Editor)
            .then(|| spec.trailing.as_ref().map(|p| p.to_string()))
            .flatten();
        self.tabs.update(cx, |tabs, cx| {
            tabs.set_title(tab, spec.title.to_string(), cx);
            if path.is_some() {
                tabs.set_path(tab, path, cx);
            }
        });
    }

    /// Move `pane` out of its group into a tab of its own, appended at the end
    /// of the group's Space. A lone pane already is a tab: nothing happens.
    pub(crate) fn detach_pane_to_new_tab(
        &mut self,
        pane: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(src) = self.tab_of_pane(pane) else {
            return;
        };
        let Some((rest, single)) = self
            .layouts
            .get(&src)
            .and_then(|layout| plan_detach(layout, pane))
        else {
            return;
        };
        let Some(space) = self.tabs.read(cx).get(src).map(|t| t.space_id) else {
            return;
        };
        let kind = self.pane_kind(pane, cx).tab_kind();
        let new_tab = self.tabs.update(cx, |tabs, cx| {
            let previous = tabs.current_space();
            tabs.set_current_space(space);
            let id = tabs.open(kind, TabData::default(), cx);
            tabs.set_current_space(previous);
            id
        });
        self.layouts.insert(src, rest);
        self.layouts.insert(new_tab, single);
        for ssh in self.ssh_tabs.values_mut() {
            if ssh.pane_id == pane {
                ssh.tab_id = new_tab;
            }
        }
        self.zoom.clear(src);
        self.resync_tab(src, cx);
        self.resync_tab(new_tab, cx);
        self.focus_pane(pane, window, cx);
        cx.notify();
    }

    /// `pane::MoveToNewTab`: detach the focused pane of the active tab.
    pub fn move_focused_pane_to_new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, pane)) = self.active_pane(cx) {
            self.detach_pane_to_new_tab(pane, window, cx);
        }
    }

    /// `pane::CloseOthers` for the focused pane of the active tab.
    pub fn close_other_panes_of_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, pane)) = self.active_pane(cx) {
            self.close_other_panes(pane, window, cx);
        }
    }

    /// Close every pane of `keep`'s tab except `keep`. An editor pane that is
    /// the last view of a dirty document is kept (closing it would lose
    /// edits; close it on its own to get the usual confirmation) and the
    /// user is told how many stayed.
    pub(crate) fn close_other_panes(
        &mut self,
        keep: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab_of_pane(keep) else {
            return;
        };
        let others: Vec<PaneId> = self
            .tab_pane_ids(tab)
            .into_iter()
            .filter(|p| *p != keep)
            .collect();
        let mut kept = 0;
        for pane in others {
            if self.pane_close_needs_confirmation(pane, cx) {
                kept += 1;
            } else {
                self.do_close_pane(pane, window, cx);
            }
        }
        if kept > 0 {
            self.notify_pane_problem(
                "Some panes were kept",
                &format!("{kept} pane(s) with unsaved changes were not closed."),
                cx,
            );
        }
        self.activate_pane(keep, window, cx);
    }

    /// The tab marker state for block `id` while a drag is active:
    /// `(join highlight with its verdict, end-of-strip marker)`.
    pub(crate) fn tab_drop_hints(&self, id: u64, cx: &App) -> (Option<Verdict>, bool) {
        let dragging = cx.has_active_drag();
        let join = self
            .pane_dnd
            .tab_join
            .filter(|(t, _)| dragging && *t == id)
            .map(|(_, verdict)| verdict);
        let end =
            dragging && self.pane_dnd.over_block.is_none() && self.pane_dnd.strip_end == Some(id);
        (join, end)
    }
}
