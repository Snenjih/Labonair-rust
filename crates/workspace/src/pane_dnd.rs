//! Drag and drop between tabs and panes: the pure half (R10-002, ADR 0005).
//!
//! Everything here is geometry and tree surgery on [`WorkspaceLayout`]s; no
//! GPUI types. [`crate::pane_drop`] applies the results to the live pane
//! stores.
//!
//! Drop semantics (payload on a pane zone):
//!
//! | payload | zone | result |
//! |---|---|---|
//! | pane, same tab | edge | the pane moves next to the target ([`plan_pane_drop`]) |
//! | pane, same tab | centre / pane bar | the two panes swap |
//! | pane, other tab | edge | detached from its tab (which closes when emptied), split into the target |
//! | pane, other tab | centre / pane bar | same, along the target's longer axis |
//! | whole tab | edge | the tab's pane group merges as a nested axis ([`plan_tab_drop`]); source tab removed |
//! | whole tab | centre / pane bar | same, along the target's longer axis |
//!
//! Every plan is computed on clones: a rejected drop (no room for the 240x120
//! px minimum, invalid target) leaves all layouts untouched.

use crate::pane::{CloseOutcome, PaneId, SplitDirection, WorkspaceLayout};
use crate::pane_ops::DropZone;
use crate::tiling::{self, PxRect, SplitRejection, TilingMetrics};

/// What is dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Payload {
    /// One pane, remembering the tab it comes from.
    Pane { pane: PaneId, tab: u64 },
    /// A whole tab including its pane group.
    Tab { tab: u64 },
}

/// Why a drop was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropError {
    /// Some pane would fall below the pixel minimum.
    NoRoom,
    /// Not a meaningful drop (same pane, unknown target, ...): do nothing.
    Invalid,
}

fn reject(r: SplitRejection) -> DropError {
    match r {
        SplitRejection::NoRoom => DropError::NoRoom,
        _ => DropError::Invalid,
    }
}

/// What happens to the tab a pane or group was taken from.
#[derive(Debug, PartialEq)]
pub(crate) enum SourceAfter {
    /// The drop stayed inside one tab.
    Same,
    /// The source tab keeps this (smaller) layout.
    Layout(WorkspaceLayout),
    /// Nothing is left: the source tab closes.
    Removed,
}

/// The outcome of a planned drop.
#[derive(Debug, PartialEq)]
pub(crate) struct DropPlan {
    /// The new layout of the target tab.
    pub dst: WorkspaceLayout,
    pub src: SourceAfter,
    /// The pane that receives focus afterwards.
    pub focus: PaneId,
}

/// Facts about the source tab and the target tab that gate a drop.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DropGuard {
    pub same_space: bool,
    /// The dragged tab is pinned (only matters for whole-tab drags).
    pub src_pinned: bool,
    pub dst_pane_count: usize,
}

/// Whether a drag may react to tab `dst_tab` at all: same Space only, a tab
/// never drops onto itself (a pane may, when it has siblings to swap with or
/// move next to), and a pinned tab is never merged away (that would close it,
/// which pinned tabs are protected from).
pub(crate) fn drop_allowed(payload: Payload, dst_tab: u64, guard: DropGuard) -> bool {
    guard.same_space
        && match payload {
            Payload::Pane { tab, .. } => tab != dst_tab || guard.dst_pane_count >= 2,
            Payload::Tab { tab } => tab != dst_tab && !guard.src_pinned,
        }
}

/// What a hover tells the user about the drop it would trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// The drop would succeed.
    Fits,
    /// The drop is meaningful but a pane would fall below the pixel minimum.
    NoRoom,
}

/// Verdict of a dry-run plan; `None` for a drop that does nothing (no
/// highlight at all).
pub(crate) fn verdict_of(plan: &Result<DropPlan, DropError>) -> Option<Verdict> {
    match plan {
        Ok(_) => Some(Verdict::Fits),
        Err(DropError::NoRoom) => Some(Verdict::NoRoom),
        Err(DropError::Invalid) => None,
    }
}

/// Whether resting a drag on tab block `hovered` should switch to that tab
/// (spring-loaded tabs), so the user can drop into its panes. Never the
/// active tab, the drag's own tab, or a tab of another Space.
pub(crate) fn spring_candidate(
    payload: Payload,
    hovered: u64,
    active: u64,
    same_space: bool,
) -> bool {
    let src = match payload {
        Payload::Pane { tab, .. } | Payload::Tab { tab } => tab,
    };
    same_space && hovered != active && hovered != src
}

// ───────────────────────────── hit testing ───────────────────────────────

/// The pane under the pointer, its raw zone and its rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ZoneHit {
    pub pane: PaneId,
    pub zone: DropZone,
    pub rect: PxRect,
}

/// Zone under `(x, y)` among the solved pane rectangles. Gaps between panes
/// hit nothing. The pane bar (`bar_h` px at the top of a pane of a group;
/// `0.0` for a single pane) always counts as the centre zone, so dropping on a
/// bar swaps / merges instead of splitting the pane's top edge.
pub(crate) fn hit_test(panes: &[(PaneId, PxRect)], x: f32, y: f32, bar_h: f32) -> Option<ZoneHit> {
    let (pane, zone) = tiling::pane_drop_target(panes, x, y)?;
    let rect = panes.iter().find(|(p, _)| *p == pane)?.1;
    let zone = if bar_h > 0.0 && y < rect.y + bar_h {
        DropZone::Centre
    } else {
        zone
    };
    Some(ZoneHit { pane, zone, rect })
}

fn zone_of(dir: SplitDirection) -> DropZone {
    match dir {
        SplitDirection::Left => DropZone::Left,
        SplitDirection::Right => DropZone::Right,
        SplitDirection::Up => DropZone::Top,
        SplitDirection::Down => DropZone::Bottom,
    }
}

/// The highlight rectangle for a hit: the half the dropped pane will occupy.
/// A centre zone that swaps (pane inside its own tab) highlights the whole
/// target; every other centre drop splits along the target's longer axis and
/// previews that second half.
pub(crate) fn preview_rect(payload: Payload, dst_tab: u64, hit: ZoneHit) -> PxRect {
    let swap = matches!(payload, Payload::Pane { tab, .. } if tab == dst_tab);
    let zone = if hit.zone == DropZone::Centre && !swap {
        zone_of(tiling::smart_direction(hit.rect))
    } else {
        hit.zone
    };
    tiling::drop_preview(hit.rect, zone)
}

// ───────────────────────────── planning ──────────────────────────────────

/// Move `pane` onto `target` at `zone`.
///
/// `dst` is `None` when the pane stays in its own tab (`src`): an edge zone
/// re-inserts it next to `target`, centre swaps. With `Some(dst)` the pane
/// leaves `src` (closing it when it was the last pane) and is split into
/// `dst` at the edge, or along `target`'s longer axis for the centre zone.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_pane_drop(
    src: &WorkspaceLayout,
    dst: Option<&WorkspaceLayout>,
    pane: PaneId,
    target: PaneId,
    zone: DropZone,
    container: PxRect,
    m: &TilingMetrics,
    alloc: &mut dyn FnMut() -> PaneId,
) -> Result<DropPlan, DropError> {
    if !src.group.find_pane(pane) {
        return Err(DropError::Invalid);
    }
    match dst {
        None => {
            if pane == target || !src.group.find_pane(target) || src.len() < 2 {
                return Err(DropError::Invalid);
            }
            let mut sim = src.clone();
            let Some(dir) = zone.direction() else {
                if !sim.swap_panes(pane, target) {
                    return Err(DropError::Invalid);
                }
                sim.set_active(pane);
                return Ok(DropPlan {
                    dst: sim,
                    src: SourceAfter::Same,
                    focus: pane,
                });
            };
            if !matches!(sim.close_pane(pane), CloseOutcome::Closed { .. }) {
                return Err(DropError::Invalid);
            }
            tiling::try_split(&mut sim, target, pane, dir, alloc, container, m).map_err(reject)?;
            Ok(DropPlan {
                dst: sim,
                src: SourceAfter::Same,
                focus: pane,
            })
        }
        Some(dst) => {
            if dst.group.find_pane(pane) || !dst.group.find_pane(target) {
                return Err(DropError::Invalid);
            }
            let mut rest = src.clone();
            let after = match rest.close_pane(pane) {
                CloseOutcome::Closed { .. } => SourceAfter::Layout(rest),
                CloseOutcome::LastPane => SourceAfter::Removed,
                CloseOutcome::NotFound => return Err(DropError::Invalid),
            };
            let mut merged = dst.clone();
            match zone.direction() {
                Some(dir) => {
                    tiling::try_split(&mut merged, target, pane, dir, alloc, container, m)
                        .map_err(reject)?;
                }
                None => {
                    tiling::smart_split(&mut merged, target, pane, alloc, container, m)
                        .map_err(reject)?;
                }
            }
            Ok(DropPlan {
                dst: merged,
                src: after,
                focus: pane,
            })
        }
    }
}

/// Merge the whole layout `src` (a dragged tab) into `dst` next to `target`.
/// An edge zone picks that side; the centre zone splits `target` along its
/// longer axis, falling back to the other axis when only that has room.
pub(crate) fn plan_tab_drop(
    src: &WorkspaceLayout,
    dst: &WorkspaceLayout,
    target: PaneId,
    zone: DropZone,
    container: PxRect,
    m: &TilingMetrics,
    alloc: &mut dyn FnMut() -> PaneId,
) -> Result<DropPlan, DropError> {
    if src.group.is_empty()
        || !dst.group.find_pane(target)
        || src.leaves().iter().any(|p| dst.group.find_pane(*p))
    {
        return Err(DropError::Invalid);
    }
    let before = tiling::solve(&dst.group, container, m.gap);
    let mut attempt = |dir: SplitDirection| -> Result<WorkspaceLayout, DropError> {
        let mut merged = dst.clone();
        if !merged.insert_subtree(Some(target), src.clone(), dir, alloc) {
            return Err(DropError::Invalid);
        }
        let after = tiling::solve(&merged.group, container, m.gap);
        if !tiling::respects_minimum(&before, &after, m) {
            return Err(DropError::NoRoom);
        }
        Ok(merged)
    };
    let merged = match zone.direction() {
        Some(dir) => attempt(dir)?,
        None => {
            let rect = before.pane_rect(target).unwrap_or(container);
            let first = tiling::smart_direction(rect);
            let second = match first {
                SplitDirection::Right => SplitDirection::Down,
                _ => SplitDirection::Right,
            };
            match attempt(first) {
                Err(DropError::NoRoom) => attempt(second)?,
                other => other?,
            }
        }
    };
    let focus = merged.active.unwrap_or(target);
    Ok(DropPlan {
        dst: merged,
        src: SourceAfter::Removed,
        focus,
    })
}

/// Take `pane` out of `src` into a layout of its own: `(rest, detached)`.
/// `None` for the only pane of a tab (it already is a tab of its own).
pub(crate) fn plan_detach(
    src: &WorkspaceLayout,
    pane: PaneId,
) -> Option<(WorkspaceLayout, WorkspaceLayout)> {
    let mut rest = src.clone();
    let single = rest.detach(pane)?;
    Some((rest, single))
}

// ───────────────────────────── strip auto-scroll ─────────────────────────

/// Width of the band at either end of a tab strip that scrolls it.
pub(crate) const EDGE_SCROLL_ZONE: f32 = 32.0;
/// Largest scroll step per pointer move event.
pub(crate) const EDGE_SCROLL_STEP: f32 = 14.0;

/// Scroll-offset delta for a drag at `pos` over a strip spanning
/// `start..end` along its scroll axis. Scroll offsets are `<= 0`: a positive
/// delta reveals content before the start, a negative one content after the
/// end. The step grows toward the edge; `0.0` away from both edges.
pub(crate) fn edge_scroll(pos: f32, start: f32, end: f32) -> f32 {
    let step = |dist: f32| {
        let t = ((EDGE_SCROLL_ZONE - dist) / EDGE_SCROLL_ZONE).clamp(0.0, 1.0);
        EDGE_SCROLL_STEP * (0.25 + 0.75 * t)
    };
    if end - start < 2.0 * EDGE_SCROLL_ZONE {
        return 0.0;
    }
    if pos < start + EDGE_SCROLL_ZONE {
        step((pos - start).max(0.0))
    } else if pos > end - EDGE_SCROLL_ZONE {
        -step((end - pos).max(0.0))
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_ops::test_support::Rng;

    const CONTAINER: PxRect = PxRect {
        x: 0.0,
        y: 0.0,
        w: 1200.0,
        h: 800.0,
    };

    fn metrics() -> TilingMetrics {
        TilingMetrics::comfortable()
    }

    fn alloc_from(start: PaneId) -> impl FnMut() -> PaneId {
        let mut n = start;
        move || {
            n += 1;
            n
        }
    }

    /// A layout with `ids` side by side.
    fn row(ids: &[PaneId]) -> WorkspaceLayout {
        // Axis ids are process-unique in the app; keep them unique here too.
        static AXES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(10_000);
        let mut layout = WorkspaceLayout::new(ids[0]);
        let mut alloc = || AXES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        for pair in ids.windows(2) {
            assert!(layout.split_pane(pair[0], pair[1], SplitDirection::Right, &mut alloc));
        }
        layout
    }

    fn single(id: PaneId) -> WorkspaceLayout {
        WorkspaceLayout::new(id)
    }

    // ── hit testing ────────────────────────────────────────────────────

    #[test]
    fn gaps_between_panes_hit_nothing() {
        let layout = row(&[1, 2]);
        let solved = tiling::solve(&layout.group, CONTAINER, 6.0);
        let (a, b) = (solved.pane_rect(1).unwrap(), solved.pane_rect(2).unwrap());
        let gap_x = (a.right() + b.x) / 2.0;
        assert!(b.x - a.right() > 1.0);
        assert_eq!(hit_test(&solved.panes, gap_x, 400.0, 0.0), None);
        // Just inside either neighbour does hit.
        assert_eq!(
            hit_test(&solved.panes, a.right() - 1.0, 400.0, 0.0)
                .unwrap()
                .pane,
            1
        );
        assert_eq!(
            hit_test(&solved.panes, b.x + 1.0, 400.0, 0.0).unwrap().pane,
            2
        );
    }

    #[test]
    fn compact_layout_has_no_dead_gap_to_speak_of() {
        let layout = row(&[1, 2]);
        let solved = tiling::solve(&layout.group, CONTAINER, 1.0);
        let a = solved.pane_rect(1).unwrap();
        assert!(hit_test(&solved.panes, a.right() - 0.5, 400.0, 0.0).is_some());
    }

    #[test]
    fn edges_pick_split_zones_and_the_middle_the_centre() {
        let solved = tiling::solve(&single(1).group, CONTAINER, 6.0);
        let z = |x: f32, y: f32| hit_test(&solved.panes, x, y, 0.0).unwrap().zone;
        assert_eq!(z(10.0, 400.0), DropZone::Left);
        assert_eq!(z(1190.0, 400.0), DropZone::Right);
        assert_eq!(z(600.0, 790.0), DropZone::Bottom);
        assert_eq!(z(600.0, 400.0), DropZone::Centre);
    }

    #[test]
    fn the_pane_bar_is_always_the_centre_zone() {
        let solved = tiling::solve(&row(&[1, 2]).group, CONTAINER, 6.0);
        let a = solved.pane_rect(1).unwrap();
        // Top edge band of a pane would be `Top`, but the bar overrides it.
        let raw = hit_test(&solved.panes, a.x + a.w / 2.0, a.y + 4.0, 0.0).unwrap();
        assert_eq!(raw.zone, DropZone::Top);
        let on_bar = hit_test(&solved.panes, a.x + a.w / 2.0, a.y + 4.0, 28.0).unwrap();
        assert_eq!(on_bar.zone, DropZone::Centre);
        // Below the bar the normal zones apply again.
        let below = hit_test(&solved.panes, a.x + 5.0, a.y + 200.0, 28.0).unwrap();
        assert_eq!(below.zone, DropZone::Left);
    }

    #[test]
    fn preview_shows_the_half_the_pane_lands_in() {
        let rect = PxRect::new(0.0, 0.0, 400.0, 200.0);
        let hit = |zone| ZoneHit {
            pane: 1,
            zone,
            rect,
        };
        let pane_in_tab = Payload::Pane { pane: 2, tab: 7 };
        // Swap inside the tab highlights everything.
        assert_eq!(preview_rect(pane_in_tab, 7, hit(DropZone::Centre)), rect);
        // Cross-tab centre previews the second half along the longer (wide) axis.
        assert_eq!(
            preview_rect(pane_in_tab, 8, hit(DropZone::Centre)),
            PxRect::new(200.0, 0.0, 200.0, 200.0)
        );
        assert_eq!(
            preview_rect(Payload::Tab { tab: 3 }, 8, hit(DropZone::Left)),
            PxRect::new(0.0, 0.0, 200.0, 200.0)
        );
    }

    // ── guards ─────────────────────────────────────────────────────────

    fn guard(same_space: bool, src_pinned: bool, dst_pane_count: usize) -> DropGuard {
        DropGuard {
            same_space,
            src_pinned,
            dst_pane_count,
        }
    }

    #[test]
    fn cross_space_drops_are_rejected() {
        let pane = Payload::Pane { pane: 5, tab: 1 };
        let tab = Payload::Tab { tab: 1 };
        assert!(!drop_allowed(pane, 2, guard(false, false, 3)));
        assert!(!drop_allowed(tab, 2, guard(false, false, 3)));
        assert!(drop_allowed(pane, 2, guard(true, false, 1)));
        assert!(drop_allowed(tab, 2, guard(true, false, 1)));
    }

    #[test]
    fn dragging_onto_the_own_tab() {
        let pane = Payload::Pane { pane: 5, tab: 1 };
        // The only pane of a single-pane tab onto its own tab: no-op.
        assert!(!drop_allowed(pane, 1, guard(true, false, 1)));
        // A pane of a group may swap / move inside its own tab.
        assert!(drop_allowed(pane, 1, guard(true, false, 2)));
        // A group tab onto itself: no-op.
        assert!(!drop_allowed(
            Payload::Tab { tab: 1 },
            1,
            guard(true, false, 4)
        ));
    }

    #[test]
    fn a_pinned_tab_is_never_merged_away() {
        let tab = Payload::Tab { tab: 1 };
        assert!(!drop_allowed(tab, 2, guard(true, true, 1)));
        // Panes may leave a pinned group: the tab keeps at least one pane.
        let pane = Payload::Pane { pane: 5, tab: 1 };
        assert!(drop_allowed(pane, 2, guard(true, true, 1)));
    }

    #[test]
    fn spring_skips_active_own_and_foreign_space_tabs() {
        let pane = Payload::Pane { pane: 5, tab: 1 };
        let tab = Payload::Tab { tab: 1 };
        for p in [pane, tab] {
            assert!(spring_candidate(p, 2, 3, true));
            assert!(!spring_candidate(p, 3, 3, true), "already active");
            assert!(!spring_candidate(p, 1, 3, true), "the drag's own tab");
            assert!(!spring_candidate(p, 2, 3, false), "other Space");
        }
    }

    #[test]
    fn verdict_separates_no_room_from_a_no_op() {
        let mut alloc = alloc_from(100);
        let fits = plan_tab_drop(
            &single(1),
            &single(10),
            10,
            DropZone::Centre,
            CONTAINER,
            &metrics(),
            &mut alloc,
        );
        assert_eq!(verdict_of(&fits), Some(Verdict::Fits));
        let tiny = PxRect::new(0.0, 0.0, 300.0, 200.0);
        let no_room = plan_tab_drop(
            &single(1),
            &single(10),
            10,
            DropZone::Centre,
            tiny,
            &metrics(),
            &mut alloc,
        );
        assert_eq!(verdict_of(&no_room), Some(Verdict::NoRoom));
        let invalid = plan_tab_drop(
            &single(1),
            &single(10),
            99,
            DropZone::Centre,
            CONTAINER,
            &metrics(),
            &mut alloc,
        );
        assert_eq!(verdict_of(&invalid), None);
    }

    // ── pane moves ─────────────────────────────────────────────────────

    #[test]
    fn centre_swaps_inside_a_tab() {
        let layout = row(&[1, 2, 3]);
        let mut alloc = alloc_from(100);
        let plan = plan_pane_drop(
            &layout,
            None,
            1,
            3,
            DropZone::Centre,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        assert_eq!(plan.dst.leaves(), vec![3, 2, 1]);
        assert_eq!(plan.src, SourceAfter::Same);
        assert_eq!(plan.focus, 1);
        assert_eq!(plan.dst.active, Some(1));
    }

    #[test]
    fn edge_moves_a_pane_next_to_the_target_inside_a_tab() {
        let layout = row(&[1, 2, 3]);
        let mut alloc = alloc_from(100);
        let plan = plan_pane_drop(
            &layout,
            None,
            3,
            1,
            DropZone::Bottom,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        assert_eq!(plan.dst.len(), 3);
        assert!(plan.dst.validate_normalized().is_ok());
        let solved = tiling::solve(&plan.dst.group, CONTAINER, 6.0);
        let (top, bottom) = (solved.pane_rect(1).unwrap(), solved.pane_rect(3).unwrap());
        assert!(bottom.y > top.y, "pane 3 sits below pane 1");
    }

    #[test]
    fn a_pane_never_drops_onto_itself_or_a_missing_target() {
        let layout = row(&[1, 2]);
        let mut alloc = alloc_from(100);
        let m = metrics();
        for (pane, target) in [(1, 1), (1, 99), (99, 1)] {
            assert_eq!(
                plan_pane_drop(
                    &layout,
                    None,
                    pane,
                    target,
                    DropZone::Right,
                    CONTAINER,
                    &m,
                    &mut alloc
                ),
                Err(DropError::Invalid)
            );
        }
        // A single pane has nothing to move against.
        let lone = single(1);
        assert_eq!(
            plan_pane_drop(
                &lone,
                None,
                1,
                1,
                DropZone::Centre,
                CONTAINER,
                &m,
                &mut alloc
            ),
            Err(DropError::Invalid)
        );
    }

    #[test]
    fn moving_to_another_tab_detaches_and_splits() {
        let src = row(&[1, 2]);
        let dst = single(10);
        let mut alloc = alloc_from(100);
        let plan = plan_pane_drop(
            &src,
            Some(&dst),
            2,
            10,
            DropZone::Right,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        assert_eq!(plan.dst.leaves(), vec![10, 2]);
        assert_eq!(plan.dst.active, Some(2));
        match plan.src {
            SourceAfter::Layout(rest) => assert_eq!(rest.leaves(), vec![1]),
            other => panic!("source should keep pane 1, got {other:?}"),
        }
        // Inputs are untouched.
        assert_eq!(src.leaves(), vec![1, 2]);
        assert_eq!(dst.leaves(), vec![10]);
    }

    #[test]
    fn moving_the_last_pane_closes_the_source_tab() {
        let src = single(1);
        let dst = row(&[10, 11]);
        let mut alloc = alloc_from(100);
        let plan = plan_pane_drop(
            &src,
            Some(&dst),
            1,
            11,
            DropZone::Centre,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        assert_eq!(plan.src, SourceAfter::Removed);
        assert_eq!(plan.dst.len(), 3);
        assert!(plan.dst.validate_normalized().is_ok());
    }

    #[test]
    fn a_pane_already_in_the_target_tab_cannot_be_inserted_twice() {
        let src = row(&[1, 2]);
        let dst = row(&[2, 3]);
        let mut alloc = alloc_from(100);
        assert_eq!(
            plan_pane_drop(
                &src,
                Some(&dst),
                2,
                3,
                DropZone::Right,
                CONTAINER,
                &metrics(),
                &mut alloc
            ),
            Err(DropError::Invalid)
        );
    }

    #[test]
    fn no_room_rejects_without_changing_anything() {
        // Four panes in a row at the width minimum: 4 * 240 + 3 * 6 = 978 of 1000.
        let narrow = PxRect::new(0.0, 0.0, 1000.0, 130.0);
        let dst = row(&[10, 11, 12, 13]);
        let src = single(1);
        let mut alloc = alloc_from(100);
        let before_dst = dst.clone();
        let result = plan_pane_drop(
            &src,
            Some(&dst),
            1,
            10,
            DropZone::Right,
            narrow,
            &metrics(),
            &mut alloc,
        );
        assert_eq!(result, Err(DropError::NoRoom));
        assert_eq!(dst, before_dst);
        assert_eq!(src, single(1));
    }

    // ── tab merges ─────────────────────────────────────────────────────

    #[test]
    fn a_tab_merges_its_whole_group_next_to_the_target() {
        let src = row(&[1, 2]);
        let dst = single(10);
        let mut alloc = alloc_from(100);
        let plan = plan_tab_drop(
            &src,
            &dst,
            10,
            DropZone::Bottom,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        assert_eq!(plan.src, SourceAfter::Removed);
        assert_eq!(plan.dst.len(), 3);
        assert!(plan.dst.validate_normalized().is_ok());
        let solved = tiling::solve(&plan.dst.group, CONTAINER, 6.0);
        let top = solved.pane_rect(10).unwrap();
        for moved in [1, 2] {
            assert!(solved.pane_rect(moved).unwrap().y > top.y);
        }
        assert_eq!(plan.dst.active, Some(plan.focus));
    }

    #[test]
    fn centre_merge_uses_the_longer_axis_of_the_target() {
        let src = single(1);
        let dst = single(10);
        let mut alloc = alloc_from(100);
        // 1200 x 800 is wide: side by side.
        let plan = plan_tab_drop(
            &src,
            &dst,
            10,
            DropZone::Centre,
            CONTAINER,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        let solved = tiling::solve(&plan.dst.group, CONTAINER, 6.0);
        assert!(solved.pane_rect(1).unwrap().x > solved.pane_rect(10).unwrap().x);
        // A tall container stacks.
        let tall = PxRect::new(0.0, 0.0, 600.0, 900.0);
        let plan = plan_tab_drop(
            &src,
            &dst,
            10,
            DropZone::Centre,
            tall,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        let solved = tiling::solve(&plan.dst.group, tall, 6.0);
        assert!(solved.pane_rect(1).unwrap().y > solved.pane_rect(10).unwrap().y);
    }

    #[test]
    fn centre_merge_falls_back_to_the_other_axis() {
        // 500 x 300: a side-by-side split leaves 247 wide (fine) but only the
        // stacked option exists when the width cannot hold two 240 columns.
        let area = PxRect::new(0.0, 0.0, 400.0, 300.0);
        let src = single(1);
        let dst = single(10);
        let mut alloc = alloc_from(100);
        let plan = plan_tab_drop(
            &src,
            &dst,
            10,
            DropZone::Centre,
            area,
            &metrics(),
            &mut alloc,
        )
        .unwrap();
        let solved = tiling::solve(&plan.dst.group, area, 6.0);
        assert!(solved.pane_rect(1).unwrap().y > solved.pane_rect(10).unwrap().y);
    }

    #[test]
    fn merge_with_no_room_is_rejected_and_leaves_state_alone() {
        let area = PxRect::new(0.0, 0.0, 300.0, 200.0);
        let src = single(1);
        let dst = single(10);
        let mut alloc = alloc_from(100);
        for zone in [DropZone::Centre, DropZone::Left, DropZone::Top] {
            assert_eq!(
                plan_tab_drop(&src, &dst, 10, zone, area, &metrics(), &mut alloc),
                Err(DropError::NoRoom)
            );
        }
        assert_eq!(dst, single(10));
        assert_eq!(src, single(1));
    }

    #[test]
    fn merging_a_tab_into_itself_is_invalid() {
        let layout = row(&[1, 2]);
        let mut alloc = alloc_from(100);
        assert_eq!(
            plan_tab_drop(
                &layout,
                &layout,
                1,
                DropZone::Right,
                CONTAINER,
                &metrics(),
                &mut alloc
            ),
            Err(DropError::Invalid)
        );
    }

    // ── detach ─────────────────────────────────────────────────────────

    #[test]
    fn detach_splits_a_group_and_refuses_a_lone_pane() {
        let layout = row(&[1, 2, 3]);
        let (rest, lone) = plan_detach(&layout, 2).unwrap();
        assert_eq!(rest.leaves(), vec![1, 3]);
        assert_eq!(lone.leaves(), vec![2]);
        assert!(plan_detach(&single(1), 1).is_none());
        assert!(plan_detach(&layout, 99).is_none());
    }

    // ── invariants under random drops ──────────────────────────────────

    #[test]
    fn random_drops_keep_trees_valid_and_panes_conserved() {
        let mut rng = Rng(0xD0D0_CAFE);
        let zones = [
            DropZone::Left,
            DropZone::Right,
            DropZone::Top,
            DropZone::Bottom,
            DropZone::Centre,
        ];
        let mut next = 1000;
        for _ in 0..300 {
            let n_src = 1 + rng.below(4);
            let n_dst = 1 + rng.below(4);
            let src_ids: Vec<PaneId> = (0..n_src as u64).map(|i| 1 + i).collect();
            let dst_ids: Vec<PaneId> = (0..n_dst as u64).map(|i| 50 + i).collect();
            let src = row(&src_ids);
            let dst = row(&dst_ids);
            let zone = zones[rng.below(zones.len())];
            let target = dst_ids[rng.below(dst_ids.len())];
            let mut alloc = || {
                next += 1;
                next
            };
            if rng.below(2) == 0 {
                let pane = src_ids[rng.below(src_ids.len())];
                if let Ok(plan) = plan_pane_drop(
                    &src,
                    Some(&dst),
                    pane,
                    target,
                    zone,
                    CONTAINER,
                    &metrics(),
                    &mut alloc,
                ) {
                    assert!(plan.dst.validate_normalized().is_ok());
                    let left = match &plan.src {
                        SourceAfter::Layout(l) => {
                            assert!(l.validate_normalized().is_ok());
                            l.len()
                        }
                        _ => 0,
                    };
                    assert_eq!(plan.dst.len() + left, n_src + n_dst);
                }
            } else if let Ok(plan) =
                plan_tab_drop(&src, &dst, target, zone, CONTAINER, &metrics(), &mut alloc)
            {
                assert!(plan.dst.validate_normalized().is_ok());
                assert_eq!(plan.dst.len(), n_src + n_dst);
                assert!(tiling::solve(&plan.dst.group, CONTAINER, 6.0).all_meet_minimum(&metrics()));
            }
        }
    }

    // ── auto-scroll ────────────────────────────────────────────────────

    #[test]
    fn edge_scroll_pushes_toward_the_nearest_end() {
        // Strip spans 100..700.
        assert_eq!(edge_scroll(400.0, 100.0, 700.0), 0.0);
        assert!(edge_scroll(105.0, 100.0, 700.0) > 0.0);
        assert!(edge_scroll(695.0, 100.0, 700.0) < 0.0);
        // Nearer the edge scrolls faster, never beyond the step.
        assert!(edge_scroll(101.0, 100.0, 700.0) > edge_scroll(125.0, 100.0, 700.0));
        assert!(edge_scroll(100.0, 100.0, 700.0) <= EDGE_SCROLL_STEP);
        // A strip too small for two bands never scrolls.
        assert_eq!(edge_scroll(10.0, 0.0, 50.0), 0.0);
    }
}
