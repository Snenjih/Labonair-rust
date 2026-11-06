//! Pane commands on `Workspace`: split, close, focus, zoom, move
//! (R10-002, ADR 0005). The tree maths lives in [`crate::tiling`] /
//! [`crate::pane_ops`]; this module applies it to the live pane store and
//! reports problems through the notification center (no toasts).

use gpui::{Context, Window};
use labonair_notifications::Notification;
use labonair_theme::PaneLayout;
use labonair_ui_kit::PaneLayoutStyle;

use crate::pane::{CloseOutcome, PaneId, SplitDirection};
use crate::pane_kind::PaneKind;
use crate::pane_ops::DropZone;
use crate::tiling::{self, PxRect, SplitRejection, TilingMetrics, COMPACT_GAP};
use crate::Workspace;

/// Fallback container when nothing was painted yet (very first frame).
const DEFAULT_AREA: (f32, f32) = (1200.0, 800.0);

impl Workspace {
    /// The pane layout style and the matching tiling metrics (gap in px).
    pub(crate) fn pane_style(&self, cx: &gpui::App) -> (PaneLayoutStyle, TilingMetrics) {
        let metrics = self.theme.read(cx).metrics();
        match metrics.pane_layout {
            PaneLayout::Comfortable => (
                PaneLayoutStyle::Comfortable,
                TilingMetrics {
                    gap: metrics.pane_gap(),
                    ..TilingMetrics::comfortable()
                },
            ),
            PaneLayout::Compact => (
                PaneLayoutStyle::Compact,
                TilingMetrics {
                    gap: COMPACT_GAP,
                    ..TilingMetrics::comfortable()
                },
            ),
        }
    }

    /// The painted pane area as a pixel rectangle in window coordinates.
    pub(crate) fn pane_container_px(&self) -> PxRect {
        match self.pane_area.get() {
            Some(b) => PxRect::new(
                f32::from(b.origin.x),
                f32::from(b.origin.y),
                f32::from(b.size.width),
                f32::from(b.size.height),
            ),
            None => PxRect::new(0.0, 0.0, DEFAULT_AREA.0, DEFAULT_AREA.1),
        }
    }

    pub(crate) fn notify_pane_problem(&self, title: &str, message: &str, cx: &mut Context<Self>) {
        labonair_notifications::notification_center(cx).update(cx, |center, cx| {
            center.push(
                Notification::warning(title.to_string(), message.to_string())
                    .source("workspace")
                    .dedupe_key(format!("pane:{title}")),
                cx,
            );
        });
    }

    /// Split the focused pane of the active tab. `Right`/`Down` are explicit
    /// requests that fall back to the other axis when the requested one has
    /// no room; `Left`/`Up` never fall back.
    pub fn split_focused(
        &mut self,
        dir: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, pane)) = self.active_pane(cx) else {
            return;
        };
        let fallback = matches!(dir, SplitDirection::Right | SplitDirection::Down);
        self.split_pane_dir(pane, dir, fallback, window, cx);
    }

    /// Split `source` with a new pane of the same kind on the `dir` edge.
    pub fn split_pane_dir(
        &mut self,
        source: PaneId,
        dir: SplitDirection,
        fallback: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab) = self.tab_of_pane(source) else {
            return;
        };
        let kind = self.pane_kind(source, cx);
        if !kind.can_split() {
            self.notify_pane_problem(
                "Pane cannot be split",
                "This pane can be moved but not split.",
                cx,
            );
            return;
        }
        let Some(mut candidate) = self.layouts.get(&tab).cloned() else {
            return;
        };
        let (_, metrics) = self.pane_style(cx);
        let container = self.pane_container_px();
        let new_id = crate::pane::next_id();
        let mut alloc = crate::pane::next_id;
        let mut result = tiling::try_split(
            &mut candidate,
            source,
            new_id,
            dir,
            &mut alloc,
            container,
            &metrics,
        );
        if fallback && result == Err(SplitRejection::NoRoom) {
            let other = match dir {
                SplitDirection::Right | SplitDirection::Left => SplitDirection::Down,
                _ => SplitDirection::Right,
            };
            result = tiling::try_split(
                &mut candidate,
                source,
                new_id,
                other,
                &mut alloc,
                container,
                &metrics,
            );
        }
        match result {
            Ok(()) => {}
            Err(SplitRejection::NoRoom) => {
                self.notify_pane_problem(
                    "No room to split",
                    "A pane needs at least 240 x 120 px. Close a pane or enlarge the window.",
                    cx,
                );
                return;
            }
            Err(_) => return,
        }
        if let Err(error) = self.spawn_pane_clone(source, new_id, window, cx) {
            let message = match error {
                crate::pane_items::SplitCloneError::Unsupported(m) => m.to_string(),
                crate::pane_items::SplitCloneError::Failed(m) => m,
            };
            self.notify_pane_problem("Pane could not be split", &message, cx);
            return;
        }
        self.layouts.insert(tab, candidate);
        self.zoom.clear(tab);
        self.fresh_pane = Some(new_id);
        self.sync_tab_kind(tab, cx);
        self.focus_pane(new_id, window, cx);
        cx.notify();
    }

    /// Keep the tab's kind equal to the kind of its first pane (the tab strip
    /// and tab-level lookups read it).
    pub(crate) fn sync_tab_kind(&mut self, tab: u64, cx: &mut Context<Self>) {
        let Some(first) = self
            .layouts
            .get(&tab)
            .and_then(|l| l.leaves().first().copied())
        else {
            return;
        };
        let kind = self.pane_kind(first, cx).tab_kind();
        self.tabs
            .update(cx, |tabs, cx| tabs.set_kind(tab, kind, cx));
    }

    /// Make `pane` the focused pane of its tab and select that tab.
    pub fn activate_pane(&mut self, pane: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_of_pane(pane) else {
            return;
        };
        if let Some(layout) = self.layouts.get_mut(&tab) {
            layout.set_active(pane);
        }
        // Focusing a pane other than the zoomed one (focus-move keys) ends the
        // zoom, so the active pane is always the visible one.
        if self.zoom.zoomed(tab).is_some_and(|z| z != pane) {
            self.zoom.clear(tab);
        }
        if self.tabs.read(cx).active_id() != tab {
            self.tabs.update(cx, |s, cx| s.set_active(tab, cx));
        }
        self.focus_pane(pane, window, cx);
        cx.notify();
    }

    /// `Close Pane`: the last pane of a tab closes the tab.
    pub fn close_pane_id(&mut self, pane: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_of_pane(pane) else {
            return;
        };
        let count = self.layouts.get(&tab).map_or(1, |l| l.len());
        if count <= 1 {
            self.request_close(tab, window, cx);
            return;
        }
        if self.pane_close_needs_confirmation(pane, cx) && self.confirm_close != Some(pane) {
            self.confirm_close = Some(pane);
            window.focus(&self.focus_handle);
            cx.notify();
            return;
        }
        self.do_close_pane(pane, window, cx);
    }

    /// An editor pane asks first only when it is the last view of a dirty
    /// document: closing one of several panes never loses edits.
    pub(crate) fn pane_close_needs_confirmation(&self, pane: PaneId, cx: &gpui::App) -> bool {
        if self.pane_kind(pane, cx) != PaneKind::Editor {
            return false;
        }
        let Some(slot) = self.editor_slot_of_pane(pane) else {
            return false;
        };
        let dirty = self
            .editors
            .get(&slot)
            .is_some_and(|e| e.read(cx).is_dirty());
        dirty && !self.document_shown_elsewhere(slot, pane)
    }

    /// Whether another pane (in any tab) shows the document of `slot`.
    pub(crate) fn document_shown_elsewhere(&self, slot: u64, except: PaneId) -> bool {
        self.layouts
            .values()
            .flat_map(|l| l.leaves())
            .any(|p| p != except && self.editor_slot_of_pane(p) == Some(slot))
    }

    pub(crate) fn do_close_pane(
        &mut self,
        pane: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.confirm_close = None;
        let Some(tab) = self.tab_of_pane(pane) else {
            return;
        };
        // The spatial neighbour (left, above, right, below) receives focus.
        let preferred = {
            let (_, metrics) = self.pane_style(cx);
            let solved = self
                .layouts
                .get(&tab)
                .map(|l| tiling::solve(&l.group, self.pane_container_px(), metrics.gap));
            solved.and_then(|s| {
                [
                    SplitDirection::Left,
                    SplitDirection::Up,
                    SplitDirection::Right,
                    SplitDirection::Down,
                ]
                .into_iter()
                .find_map(|d| tiling::neighbor(&s.panes, pane, d, false))
            })
        };
        let outcome = self
            .layouts
            .get_mut(&tab)
            .map(|l| l.close_pane(pane))
            .unwrap_or(CloseOutcome::NotFound);
        match outcome {
            CloseOutcome::LastPane => self.request_close(tab, window, cx),
            CloseOutcome::Closed { new_active } => {
                self.zoom.clear(tab);
                self.retire_any_pane(pane, cx);
                if let Some(layout) = self.layouts.get_mut(&tab) {
                    layout.set_active(preferred.unwrap_or(new_active));
                }
                self.sync_tab_kind(tab, cx);
                self.sync_tab_dirty(tab, cx);
                let focus = self.focused_pane(tab);
                self.focus_pane(focus, window, cx);
                cx.notify();
            }
            CloseOutcome::NotFound => {}
        }
    }

    /// Close the focused pane of the active tab (`pane::Close`).
    pub fn close_focused_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, pane)) = self.active_pane(cx) {
            self.close_pane_id(pane, window, cx);
        }
    }

    /// Move focus to the spatial neighbour of the focused pane.
    pub fn focus_pane_dir(
        &mut self,
        dir: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((tab, pane)) = self.active_pane(cx) else {
            return;
        };
        let (_, metrics) = self.pane_style(cx);
        let Some(layout) = self.layouts.get(&tab) else {
            return;
        };
        let solved = tiling::solve(&layout.group, self.pane_container_px(), metrics.gap);
        if let Some(target) = tiling::neighbor(&solved.panes, pane, dir, false) {
            self.activate_pane(target, window, cx);
        }
    }

    /// Cycle focus through the panes of the active tab.
    pub fn focus_pane_cycle(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((tab, pane)) = self.active_pane(cx) else {
            return;
        };
        let order = self.tab_pane_ids(tab);
        if let Some(target) = tiling::cycle_focus(&order, pane, forward) {
            self.activate_pane(target, window, cx);
        }
    }

    /// Toggle the zoom overlay of the focused pane (needs two or more panes).
    pub fn toggle_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((tab, pane)) = self.active_pane(cx) else {
            return;
        };
        self.toggle_zoom_of(tab, pane, window, cx);
    }

    pub(crate) fn toggle_zoom_of(
        &mut self,
        tab: u64,
        pane: PaneId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.layouts.get(&tab).map_or(0, |l| l.len()) < 2 {
            return;
        }
        if let Some(layout) = self.layouts.get_mut(&tab) {
            layout.set_active(pane);
        }
        self.zoom.toggle(tab, pane);
        self.focus_pane(pane, window, cx);
        cx.notify();
    }

    /// Swap the focused pane with its neighbour in `dir` (`pane::MovePane*`).
    pub fn move_pane_dir(
        &mut self,
        dir: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((tab, pane)) = self.active_pane(cx) else {
            return;
        };
        let (_, metrics) = self.pane_style(cx);
        let Some(layout) = self.layouts.get(&tab) else {
            return;
        };
        let solved = tiling::solve(&layout.group, self.pane_container_px(), metrics.gap);
        let Some(target) = tiling::neighbor(&solved.panes, pane, dir, false) else {
            return;
        };
        let mut alloc = crate::pane::next_id;
        if let Some(layout) = self.layouts.get_mut(&tab) {
            if layout.move_pane(pane, target, DropZone::Centre, &mut alloc) {
                self.zoom.clear(tab);
                self.sync_tab_kind(tab, cx);
                self.focus_pane(pane, window, cx);
                cx.notify();
            }
        }
    }

    /// Set the tab's dirty marker from every editor document in its tree.
    pub(crate) fn sync_tab_dirty(&mut self, tab: u64, cx: &mut Context<Self>) {
        let dirty = self.tab_pane_ids(tab).into_iter().any(|p| {
            self.editor_slot_of_pane(p)
                .and_then(|slot| self.editors.get(&slot))
                .is_some_and(|e| e.read(cx).is_dirty())
        });
        self.tabs
            .update(cx, |tabs, cx| tabs.set_dirty(tab, dirty, cx));
    }

    /// The tab holding the single instance of a singleton kind, if open. The
    /// pane store is authoritative (a singleton may live inside a group);
    /// the tab-kind scan covers tabs that were never rendered yet. When the
    /// pane sits in a group it becomes that tab's focused pane.
    pub(crate) fn find_singleton_tab(&mut self, kind: PaneKind, cx: &gpui::App) -> Option<u64> {
        if let Some(pane) = self.pane_meta.find_singleton(kind) {
            if let Some(tab) = self.tab_of_pane(pane) {
                if let Some(layout) = self.layouts.get_mut(&tab) {
                    layout.set_active(pane);
                }
                return Some(tab);
            }
        }
        self.tabs
            .read(cx)
            .tabs()
            .iter()
            .find(|t| PaneKind::from(t.kind) == kind)
            .map(|t| t.id)
    }
}
