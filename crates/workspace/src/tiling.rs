//! Smart tiling for the universal pane model (R10-002, ADR 0005).
//!
//! Everything here is pure geometry over [`crate::pane_group`] trees and plain
//! `f32` pixel rectangles ([`PxRect`]); it has no GPUI types so it is fully
//! unit-testable. The UI converts `gpui::Bounds<Pixels>` at the boundary.
//!
//! * [`solve`] lays a tree out in a container: every pane gets a [`PxRect`],
//!   every axis an [`AxisGeometry`] with the gap strips between its members
//!   (`handles`). Panes + handles partition the container exactly.
//! * [`try_split`] / [`smart_split`] split a pane only if every pane involved
//!   stays at least [`TilingMetrics::min_width`] x [`TilingMetrics::min_height`]
//!   pixels, otherwise they report [`SplitRejection::NoRoom`].
//! * [`neighbor`] / [`cycle_focus`] drive directional and sequential focus.
//! * [`drop_zone`] hit-tests the five drop zones of a pane.
//! * [`AxisGeometry::boundary_fraction`] turns a pointer position into the
//!   cumulative fraction `PaneGroup::set_boundary` expects, correct in the
//!   presence of gaps and with pixel minimum sizes.
//!
//! # Gaps and handles
//!
//! `gap` is the distance between two siblings of one axis. Resize handles live
//! *inside* that strip (drawn centred in it, or as the 1px divider in compact
//! mode), so the solver never needs a separate handle width: pass the
//! comfortable gap (6px) or the compact divider thickness (1px). Container
//! size minus all gaps is what the flexes share.

use crate::pane::{PaneId, SplitAxis, MIN_RATIO};
use crate::pane_group::{Member, PaneAxis, PaneGroup, SplitDirection, WorkspaceLayout};
pub use crate::pane_ops::DropZone;

/// Default minimum pane width in pixels.
pub const DEFAULT_MIN_WIDTH: f32 = 240.0;
/// Default minimum pane height in pixels.
pub const DEFAULT_MIN_HEIGHT: f32 = 120.0;
/// Gap between panes in the `comfortable` pane layout.
pub const COMFORTABLE_GAP: f32 = 6.0;
/// Divider thickness (the whole gap) in the `compact` pane layout.
pub const COMPACT_GAP: f32 = 1.0;
/// Width of each outer edge band that selects an edge drop zone.
pub const DROP_EDGE_FRACTION: f32 = 0.25;

/// Slack for float comparisons, in pixels.
const EPS: f32 = 0.5;

/// A rectangle in pixels (origin top-left).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PxRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PxRect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn area(&self) -> f32 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Half-open containment: left/top edges inside, right/bottom outside.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Start of the rect along `axis` (x for horizontal, y for vertical).
    fn origin(&self, axis: SplitAxis) -> f32 {
        match axis {
            SplitAxis::Horizontal => self.x,
            SplitAxis::Vertical => self.y,
        }
    }

    /// Length of the rect along `axis`.
    fn extent(&self, axis: SplitAxis) -> f32 {
        match axis {
            SplitAxis::Horizontal => self.w,
            SplitAxis::Vertical => self.h,
        }
    }
}

/// Pixel sizes that drive the tiling rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TilingMetrics {
    /// Distance between siblings (see the module docs).
    pub gap: f32,
    pub min_width: f32,
    pub min_height: f32,
}

impl TilingMetrics {
    pub fn comfortable() -> Self {
        Self {
            gap: COMFORTABLE_GAP,
            min_width: DEFAULT_MIN_WIDTH,
            min_height: DEFAULT_MIN_HEIGHT,
        }
    }

    pub fn compact() -> Self {
        Self {
            gap: COMPACT_GAP,
            ..Self::comfortable()
        }
    }

    /// Minimum size along `axis`.
    pub fn min_along(&self, axis: SplitAxis) -> f32 {
        match axis {
            SplitAxis::Horizontal => self.min_width,
            SplitAxis::Vertical => self.min_height,
        }
    }
}

impl Default for TilingMetrics {
    fn default() -> Self {
        Self::comfortable()
    }
}

// ─────────────────────────────── geometry ────────────────────────────────

/// Resolved geometry of one axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisGeometry {
    pub id: PaneId,
    pub axis: SplitAxis,
    pub rect: PxRect,
    pub gap: f32,
    /// The axis' flexes at solve time (one per member).
    pub flexes: Vec<f32>,
    /// The gap strips between member `i` and `i + 1` (`len == members - 1`).
    pub handles: Vec<PxRect>,
}

impl AxisGeometry {
    /// Pixels shared by the members: the axis extent minus all gaps.
    pub fn available(&self) -> f32 {
        let n = self.flexes.len();
        (self.rect.extent(self.axis) - self.gap * n.saturating_sub(1) as f32).max(0.0)
    }

    /// The cumulative axis fraction to pass to `PaneGroup::set_boundary` when
    /// the handle between member `boundary` and `boundary + 1` is dragged so
    /// its centre sits at `pointer` (absolute coordinate along the axis).
    ///
    /// Both neighbours are kept at least `min_px` wide; if the pair cannot
    /// afford that, the result is clamped to the ratio floor
    /// ([`MIN_RATIO`] of the pair) instead. `None` when `boundary` is out of
    /// range or there is no room to share.
    pub fn boundary_fraction(&self, boundary: usize, pointer: f32, min_px: f32) -> Option<f32> {
        if boundary + 1 >= self.flexes.len() {
            return None;
        }
        let avail = self.available();
        if avail <= 0.0 {
            return None;
        }
        // Pixels of member content left of the handle: everything up to the
        // handle's centre, minus the previous gaps and half of this one.
        let before_px =
            pointer - self.gap / 2.0 - self.rect.origin(self.axis) - self.gap * boundary as f32;
        let raw = before_px / avail;

        let before: f32 = self.flexes[..boundary].iter().sum();
        let pair = self.flexes[boundary] + self.flexes[boundary + 1];
        let min_frac = min_px / avail;
        let margin = if 2.0 * min_frac < pair {
            min_frac
        } else {
            MIN_RATIO * pair
        };
        Some(raw.clamp(before + margin, before + pair - margin))
    }
}

/// A solved layout: pane rectangles in visual order plus per-axis geometry.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Solved {
    pub panes: Vec<(PaneId, PxRect)>,
    pub axes: Vec<AxisGeometry>,
}

impl Solved {
    pub fn pane_rect(&self, id: PaneId) -> Option<PxRect> {
        self.panes.iter().find(|(p, _)| *p == id).map(|(_, r)| *r)
    }

    pub fn axis(&self, id: PaneId) -> Option<&AxisGeometry> {
        self.axes.iter().find(|a| a.id == id)
    }

    /// Whether every pane is at least `m`'s minimum size.
    pub fn all_meet_minimum(&self, m: &TilingMetrics) -> bool {
        self.panes
            .iter()
            .all(|(_, r)| r.w >= m.min_width - EPS && r.h >= m.min_height - EPS)
    }
}

/// Lay `group` out inside `container`. Siblings of an axis share
/// `extent - gap * (n - 1)` according to their flexes; the last sibling is
/// snapped to the far edge so rounding never leaves a seam.
pub fn solve(group: &PaneGroup, container: PxRect, gap: f32) -> Solved {
    let mut out = Solved::default();
    if let Some(root) = &group.root {
        solve_member(root, container, gap.max(0.0), &mut out);
    }
    out
}

fn solve_member(m: &Member, rect: PxRect, gap: f32, out: &mut Solved) {
    match m {
        Member::Pane(id) => out.panes.push((*id, rect)),
        Member::Axis(ax) => solve_axis(ax, rect, gap, out),
    }
}

fn solve_axis(ax: &PaneAxis, rect: PxRect, gap: f32, out: &mut Solved) {
    let n = ax.members.len();
    if n == 0 {
        return;
    }
    let even = 1.0 / n as f32;
    let flexes: Vec<f32> = if ax.flexes.len() == n {
        ax.flexes.clone()
    } else {
        vec![even; n]
    };
    let extent = rect.extent(ax.axis);
    let avail = (extent - gap * (n - 1) as f32).max(0.0);
    let mut handles = Vec::with_capacity(n.saturating_sub(1));
    let mut children = Vec::with_capacity(n);
    let mut cursor = rect.origin(ax.axis);
    let end = cursor + extent;
    for (i, (child, f)) in ax.members.iter().zip(&flexes).enumerate() {
        let size = if i + 1 == n {
            (end - cursor).max(0.0)
        } else {
            f * avail
        };
        let cell = match ax.axis {
            SplitAxis::Horizontal => PxRect::new(cursor, rect.y, size, rect.h),
            SplitAxis::Vertical => PxRect::new(rect.x, cursor, rect.w, size),
        };
        children.push((child, cell));
        cursor += size;
        if i + 1 < n {
            handles.push(match ax.axis {
                SplitAxis::Horizontal => PxRect::new(cursor, rect.y, gap, rect.h),
                SplitAxis::Vertical => PxRect::new(rect.x, cursor, rect.w, gap),
            });
            cursor += gap;
        }
    }
    out.axes.push(AxisGeometry {
        id: ax.id,
        axis: ax.axis,
        rect,
        gap,
        flexes,
        handles,
    });
    for (child, cell) in children {
        solve_member(child, cell, gap, out);
    }
}

// ───────────────────────────── smart splitting ───────────────────────────

/// Why a split was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitRejection {
    /// Some pane would end up below the pixel minimum.
    NoRoom,
    /// The target pane is not in the layout.
    TargetMissing,
    /// The new pane id is already in the layout.
    DuplicatePane,
}

/// The axis a smart split runs along: the *longer* side of the target.
/// A wider-than-tall pane is split side by side
/// ([`SplitAxis::Horizontal`]); a tall pane, or an exactly square one, is
/// stacked ([`SplitAxis::Vertical`]).
pub fn smart_axis(target: PxRect) -> SplitAxis {
    if target.w > target.h {
        SplitAxis::Horizontal
    } else {
        SplitAxis::Vertical
    }
}

/// [`smart_axis`] as a direction (the new pane goes right / down).
pub fn smart_direction(target: PxRect) -> SplitDirection {
    SplitDirection::along(smart_axis(target))
}

/// Does `after` respect the minimum, ignoring panes that were already too
/// small and did not shrink (a split must not be blocked by an unrelated
/// pane that a tiny window squeezed earlier)?
pub(crate) fn respects_minimum(before: &Solved, after: &Solved, m: &TilingMetrics) -> bool {
    after.panes.iter().all(|(id, r)| {
        let prev = before.pane_rect(*id);
        let dim_ok = |now: f32, min: f32, was: Option<f32>| {
            now >= min - EPS || was.is_some_and(|w| now >= w - EPS)
        };
        dim_ok(r.w, m.min_width, prev.map(|p| p.w)) && dim_ok(r.h, m.min_height, prev.map(|p| p.h))
    })
}

/// Split `target` with `new_pane` on the `dir` edge, but only if the result
/// keeps every pane at the pixel minimum. On success the layout is replaced
/// (the new pane is active); on error it is untouched. `alloc` mints axis ids
/// (it may be called even when the split is rejected).
pub fn try_split(
    layout: &mut WorkspaceLayout,
    target: PaneId,
    new_pane: PaneId,
    dir: SplitDirection,
    alloc: &mut dyn FnMut() -> PaneId,
    container: PxRect,
    m: &TilingMetrics,
) -> Result<(), SplitRejection> {
    if layout.group.is_empty() {
        layout.split_pane(target, new_pane, dir, alloc);
        return Ok(());
    }
    if !layout.group.find_pane(target) {
        return Err(SplitRejection::TargetMissing);
    }
    if layout.group.find_pane(new_pane) {
        return Err(SplitRejection::DuplicatePane);
    }
    let mut sim = layout.clone();
    if !sim.split_pane(target, new_pane, dir, alloc) {
        return Err(SplitRejection::DuplicatePane);
    }
    let before = solve(&layout.group, container, m.gap);
    let after = solve(&sim.group, container, m.gap);
    if !respects_minimum(&before, &after, m) {
        return Err(SplitRejection::NoRoom);
    }
    *layout = sim;
    Ok(())
}

/// Smart split: split `target` along its longer side; if that would violate
/// the minimum, try the other side before giving up with
/// [`SplitRejection::NoRoom`]. Returns the direction that was used.
pub fn smart_split(
    layout: &mut WorkspaceLayout,
    target: PaneId,
    new_pane: PaneId,
    alloc: &mut dyn FnMut() -> PaneId,
    container: PxRect,
    m: &TilingMetrics,
) -> Result<SplitDirection, SplitRejection> {
    let rect = solve(&layout.group, container, m.gap)
        .pane_rect(target)
        .unwrap_or(container);
    let first = smart_direction(rect);
    let second = match first {
        SplitDirection::Right => SplitDirection::Down,
        _ => SplitDirection::Right,
    };
    match try_split(layout, target, new_pane, first, alloc, container, m) {
        Ok(()) => Ok(first),
        Err(SplitRejection::NoRoom) => {
            try_split(layout, target, new_pane, second, alloc, container, m).map(|()| second)
        }
        Err(e) => Err(e),
    }
}

// ─────────────────────────────── focus moves ─────────────────────────────

/// Overlap of two intervals (negative when disjoint).
fn overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    a1.min(b1) - a0.max(b0)
}

/// The pane to focus when moving from `from` in `dir`.
///
/// Candidates must lie entirely beyond `from`'s edge in `dir` and overlap it
/// on the cross axis. The nearest candidates win (within a pixel); among
/// those the one with the largest cross overlap, then the one whose centre is
/// closest to `from`'s centre, then visual order.
///
/// Wrap policy: with `wrap == false` (the default for `pane::Focus*`) moving
/// past the edge returns `None`. With `wrap == true` it jumps to the far edge:
/// the farthest pane in the opposite direction that overlaps `from` on the
/// cross axis (largest overlap wins ties); `None` if `from` is alone in that
/// row/column.
pub fn neighbor(
    panes: &[(PaneId, PxRect)],
    from: PaneId,
    dir: SplitDirection,
    wrap: bool,
) -> Option<PaneId> {
    let a = panes.iter().find(|(p, _)| *p == from)?.1;
    let horizontal = matches!(dir, SplitDirection::Left | SplitDirection::Right);
    let forward = dir.increasing();
    let (a_lo, a_hi, a_cross_lo, a_cross_hi) = if horizontal {
        (a.x, a.right(), a.y, a.bottom())
    } else {
        (a.y, a.bottom(), a.x, a.right())
    };
    let a_cross_mid = (a_cross_lo + a_cross_hi) / 2.0;

    struct Cand {
        idx: usize,
        id: PaneId,
        dist: f32,
        overlap: f32,
        cross_mid_dist: f32,
    }
    let mut cands: Vec<Cand> = Vec::new();
    for (idx, (id, r)) in panes.iter().enumerate() {
        if *id == from {
            continue;
        }
        let (lo, hi, c_lo, c_hi) = if horizontal {
            (r.x, r.right(), r.y, r.bottom())
        } else {
            (r.y, r.bottom(), r.x, r.right())
        };
        let ov = overlap(a_cross_lo, a_cross_hi, c_lo, c_hi);
        if ov <= EPS {
            continue;
        }
        let dist = if forward { lo - a_hi } else { a_lo - hi };
        cands.push(Cand {
            idx,
            id: *id,
            dist,
            overlap: ov,
            cross_mid_dist: ((c_lo + c_hi) / 2.0 - a_cross_mid).abs(),
        });
    }

    let ahead: Vec<&Cand> = cands.iter().filter(|c| c.dist >= -EPS).collect();
    if let Some(nearest) = ahead.iter().map(|c| c.dist).min_by(f32::total_cmp) {
        return ahead
            .into_iter()
            .filter(|c| c.dist <= nearest + 1.0)
            .min_by(|x, y| {
                y.overlap
                    .total_cmp(&x.overlap)
                    .then(x.cross_mid_dist.total_cmp(&y.cross_mid_dist))
                    .then(x.idx.cmp(&y.idx))
            })
            .map(|c| c.id);
    }
    if !wrap {
        return None;
    }
    // Far edge: the candidate farthest "behind" (negative distance means it
    // lies on the opposite side).
    cands
        .iter()
        .filter(|c| c.dist < -EPS)
        .min_by(|x, y| {
            x.dist
                .total_cmp(&y.dist)
                .then(y.overlap.total_cmp(&x.overlap))
                .then(x.idx.cmp(&y.idx))
        })
        .map(|c| c.id)
}

/// Next / previous pane in visual order, wrapping around. `None` when `order`
/// has no other pane or does not contain `from`.
pub fn cycle_focus(order: &[PaneId], from: PaneId, forward: bool) -> Option<PaneId> {
    let i = order.iter().position(|p| *p == from)?;
    if order.len() < 2 {
        return None;
    }
    let n = order.len();
    Some(
        order[if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        }],
    )
}

// ───────────────────────────── drop zones ────────────────────────────────

/// Classify a point inside `rect`: the outer [`DROP_EDGE_FRACTION`] bands are
/// the four edge zones, the middle is [`DropZone::Centre`]. In the corners
/// the nearer edge (by normalised distance) wins; exact ties prefer
/// Left, Right, Top, Bottom in that order. `None` outside `rect` or for an
/// empty rect.
pub fn drop_zone(rect: PxRect, x: f32, y: f32) -> Option<DropZone> {
    if rect.w <= 0.0 || rect.h <= 0.0 || !rect.contains(x, y) {
        return None;
    }
    let fx = (x - rect.x) / rect.w;
    let fy = (y - rect.y) / rect.h;
    let candidates = [
        (fx, DropZone::Left),
        (1.0 - fx, DropZone::Right),
        (fy, DropZone::Top),
        (1.0 - fy, DropZone::Bottom),
    ];
    let mut best = (f32::INFINITY, DropZone::Centre);
    for (d, zone) in candidates {
        if d < best.0 {
            best = (d, zone);
        }
    }
    Some(if best.0 < DROP_EDGE_FRACTION {
        best.1
    } else {
        DropZone::Centre
    })
}

/// The pane under a point and its drop zone. Points in a gap hit nothing.
pub fn pane_drop_target(panes: &[(PaneId, PxRect)], x: f32, y: f32) -> Option<(PaneId, DropZone)> {
    panes
        .iter()
        .find_map(|(id, r)| drop_zone(*r, x, y).map(|z| (*id, z)))
}

/// The highlight rectangle previewing where a pane dropped in `zone` would
/// land: the matching half of `rect` for edge zones, all of it for `Centre`.
pub fn drop_preview(rect: PxRect, zone: DropZone) -> PxRect {
    match zone {
        DropZone::Left => PxRect::new(rect.x, rect.y, rect.w / 2.0, rect.h),
        DropZone::Right => PxRect::new(rect.x + rect.w / 2.0, rect.y, rect.w / 2.0, rect.h),
        DropZone::Top => PxRect::new(rect.x, rect.y, rect.w, rect.h / 2.0),
        DropZone::Bottom => PxRect::new(rect.x, rect.y + rect.h / 2.0, rect.w, rect.h / 2.0),
        DropZone::Centre => rect,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_ops::test_support::Rng;

    const CONTAINER: PxRect = PxRect {
        x: 10.0,
        y: 20.0,
        w: 1200.0,
        h: 800.0,
    };

    fn alloc_from(start: PaneId) -> impl FnMut() -> PaneId {
        let mut n = start;
        move || {
            n += 1;
            n
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn solve_single_pane_fills_the_container() {
        let g = PaneGroup::new(1);
        let s = solve(&g, CONTAINER, 6.0);
        assert_eq!(s.panes, vec![(1, CONTAINER)]);
        assert!(s.axes.is_empty());
        assert!(solve(&PaneGroup::empty(), CONTAINER, 6.0).panes.is_empty());
    }

    #[test]
    fn solve_subtracts_gaps_from_the_shared_space() {
        let mut g = PaneGroup::new(1);
        let mut alloc = alloc_from(100);
        g.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        g.split_pane(2, 3, SplitDirection::Right, &mut alloc); // equal thirds
        let s = solve(&g, CONTAINER, 6.0);
        // (1200 - 2*6) / 3 = 396 each.
        let w = 396.0;
        assert!(close(s.pane_rect(1).unwrap().w, w));
        assert!(close(s.pane_rect(2).unwrap().x, 10.0 + w + 6.0));
        assert!(close(s.pane_rect(3).unwrap().right(), CONTAINER.right()));
        let ax = &s.axes[0];
        assert_eq!(ax.handles.len(), 2);
        assert!(close(ax.handles[0].x, 10.0 + w));
        assert!(close(ax.handles[0].w, 6.0));
        assert!(close(ax.available(), 1188.0));
    }

    #[test]
    fn zero_gap_and_tiny_container_do_not_panic() {
        let mut g = PaneGroup::new(1);
        let mut alloc = alloc_from(100);
        g.split_pane(1, 2, SplitDirection::Down, &mut alloc);
        let s = solve(&g, PxRect::new(0.0, 0.0, 0.0, 3.0), 6.0);
        assert!(s.panes.iter().all(|(_, r)| r.w >= 0.0 && r.h >= 0.0));
        let s = solve(&g, PxRect::new(0.0, 0.0, 100.0, 100.0), 0.0);
        assert!(close(s.pane_rect(2).unwrap().h, 50.0));
    }

    #[test]
    fn smart_axis_follows_the_longer_side() {
        assert_eq!(
            smart_axis(PxRect::new(0.0, 0.0, 800.0, 400.0)),
            SplitAxis::Horizontal
        );
        assert_eq!(
            smart_axis(PxRect::new(0.0, 0.0, 400.0, 800.0)),
            SplitAxis::Vertical
        );
        // Tie -> vertical (stack).
        assert_eq!(
            smart_axis(PxRect::new(0.0, 0.0, 500.0, 500.0)),
            SplitAxis::Vertical
        );
        assert_eq!(
            smart_direction(PxRect::new(0.0, 0.0, 800.0, 400.0)),
            SplitDirection::Right
        );
        assert_eq!(
            smart_direction(PxRect::new(0.0, 0.0, 500.0, 500.0)),
            SplitDirection::Down
        );
    }

    #[test]
    fn smart_split_alternates_like_a_tiler() {
        let m = TilingMetrics::comfortable();
        let mut l = WorkspaceLayout::new(1);
        let mut alloc = alloc_from(100);
        // 1200x800: wide -> right.
        assert_eq!(
            smart_split(&mut l, 1, 2, &mut alloc, CONTAINER, &m),
            Ok(SplitDirection::Right)
        );
        // pane 2 is 597x800: tall -> down.
        assert_eq!(
            smart_split(&mut l, 2, 3, &mut alloc, CONTAINER, &m),
            Ok(SplitDirection::Down)
        );
        assert_eq!(l.leaves(), vec![1, 2, 3]);
        assert_eq!(l.validate_normalized(), Ok(()));
        assert_eq!(l.active, Some(3));
    }

    #[test]
    fn split_is_rejected_when_no_room_and_leaves_the_layout_alone() {
        let m = TilingMetrics::comfortable();
        let tiny = PxRect::new(0.0, 0.0, 300.0, 150.0);
        let mut l = WorkspaceLayout::new(1);
        let before = l.clone();
        let mut alloc = alloc_from(100);
        // 300x150: right gives 147 wide, down gives 72 high: both too small.
        assert_eq!(
            smart_split(&mut l, 1, 2, &mut alloc, tiny, &m),
            Err(SplitRejection::NoRoom)
        );
        assert_eq!(l, before);
        // Falls back to the other axis when only that one fits.
        let wide_short = PxRect::new(0.0, 0.0, 400.0, 300.0);
        assert_eq!(
            smart_split(&mut l, 1, 2, &mut alloc, wide_short, &m),
            Ok(SplitDirection::Down)
        );
        assert_eq!(
            try_split(
                &mut l,
                99,
                5,
                SplitDirection::Right,
                &mut alloc,
                wide_short,
                &m
            ),
            Err(SplitRejection::TargetMissing)
        );
        assert_eq!(
            try_split(
                &mut l,
                1,
                2,
                SplitDirection::Right,
                &mut alloc,
                wide_short,
                &m
            ),
            Err(SplitRejection::DuplicatePane)
        );
    }

    #[test]
    fn already_small_panes_do_not_block_unrelated_splits() {
        let m = TilingMetrics::comfortable();
        let mut l = WorkspaceLayout::new(1);
        let mut alloc = alloc_from(100);
        l.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        // Pane 1 was dragged down to ~119px (below the 240px minimum). Splitting
        // pane 2 downwards leaves pane 1 untouched, so it must not block it.
        assert!(l.group.set_boundary(101, 0, 0.0));
        let big = PxRect::new(0.0, 0.0, 1200.0, 800.0);
        let s = solve(&l.group, big, m.gap);
        assert!(s.pane_rect(1).unwrap().w < m.min_width);
        assert!(try_split(&mut l, 2, 3, SplitDirection::Down, &mut alloc, big, &m).is_ok());
        // Stacking a new pane under pane 1 would create another 119px-wide pane.
        assert_eq!(
            try_split(&mut l, 1, 4, SplitDirection::Down, &mut alloc, big, &m),
            Err(SplitRejection::NoRoom)
        );
    }

    #[test]
    fn neighbour_search_handles_nested_layouts() {
        // [ 1 | (2 over 3) ]  with 3 further split into [4 | 5]
        let mut l = WorkspaceLayout::new(1);
        let mut alloc = alloc_from(100);
        l.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        l.split_pane(2, 3, SplitDirection::Down, &mut alloc);
        l.split_pane(3, 4, SplitDirection::Right, &mut alloc);
        let s = solve(&l.group, CONTAINER, 6.0);
        let p = &s.panes;
        assert_eq!(neighbor(p, 1, SplitDirection::Right, false), Some(2));
        assert_eq!(neighbor(p, 2, SplitDirection::Left, false), Some(1));
        assert_eq!(neighbor(p, 2, SplitDirection::Down, false), Some(3));
        assert_eq!(neighbor(p, 4, SplitDirection::Up, false), Some(2));
        assert_eq!(neighbor(p, 3, SplitDirection::Right, false), Some(4));
        // From the tall pane 1 going right: 2 and 3 both overlap; 2 is
        // nearest-equal and has the larger/equal overlap -> first in order.
        assert!(matches!(
            neighbor(p, 1, SplitDirection::Right, false),
            Some(2 | 3)
        ));
        // Edges: no wrap -> None.
        assert_eq!(neighbor(p, 1, SplitDirection::Left, false), None);
        assert_eq!(neighbor(p, 2, SplitDirection::Up, false), None);
        assert_eq!(neighbor(p, 9, SplitDirection::Up, false), None);
    }

    #[test]
    fn neighbour_wrap_jumps_to_the_far_edge() {
        let mut l = WorkspaceLayout::new(1);
        let mut alloc = alloc_from(100);
        l.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        l.split_pane(2, 3, SplitDirection::Right, &mut alloc);
        let s = solve(&l.group, CONTAINER, 6.0);
        assert_eq!(neighbor(&s.panes, 3, SplitDirection::Right, true), Some(1));
        assert_eq!(neighbor(&s.panes, 1, SplitDirection::Left, true), Some(3));
        // Alone in the row: nothing to wrap to.
        let solo = solve(&PaneGroup::new(7), CONTAINER, 6.0);
        assert_eq!(neighbor(&solo.panes, 7, SplitDirection::Right, true), None);
    }

    #[test]
    fn neighbour_prefers_the_closer_centre_among_equal_overlaps() {
        // 1 | (2 over 3) with equal flexes: from the tall left pane, both
        // right neighbours overlap by the same amount; 2 is first.
        let mut l = WorkspaceLayout::new(1);
        let mut alloc = alloc_from(100);
        l.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        l.split_pane(2, 3, SplitDirection::Down, &mut alloc);
        let s = solve(&l.group, CONTAINER, 6.0);
        // From 3 (bottom right) going left: only pane 1 qualifies.
        assert_eq!(neighbor(&s.panes, 3, SplitDirection::Left, false), Some(1));
        // From 1 the centres are symmetric -> visual order decides.
        assert_eq!(neighbor(&s.panes, 1, SplitDirection::Right, false), Some(2));
    }

    #[test]
    fn cycle_focus_wraps() {
        assert_eq!(cycle_focus(&[1, 2, 3], 3, true), Some(1));
        assert_eq!(cycle_focus(&[1, 2, 3], 1, false), Some(3));
        assert_eq!(cycle_focus(&[1], 1, true), None);
        assert_eq!(cycle_focus(&[1, 2], 9, true), None);
    }

    #[test]
    fn drop_zone_bands_and_corners() {
        let r = PxRect::new(100.0, 100.0, 400.0, 200.0);
        // Outer 25%: 100px wide bands horizontally, 50px vertically.
        assert_eq!(drop_zone(r, 110.0, 200.0), Some(DropZone::Left));
        assert_eq!(drop_zone(r, 490.0, 200.0), Some(DropZone::Right));
        assert_eq!(drop_zone(r, 300.0, 110.0), Some(DropZone::Top));
        assert_eq!(drop_zone(r, 300.0, 290.0), Some(DropZone::Bottom));
        assert_eq!(drop_zone(r, 300.0, 200.0), Some(DropZone::Centre));
        // Band boundary: exactly 25% is already centre.
        assert_eq!(drop_zone(r, 200.0, 200.0), Some(DropZone::Centre));
        assert_eq!(drop_zone(r, 199.9, 200.0), Some(DropZone::Left));
        // Corner: nearer edge by normalised distance wins.
        assert_eq!(drop_zone(r, 105.0, 150.0), Some(DropZone::Left)); // fx .0125 < fy .25
        assert_eq!(drop_zone(r, 150.0, 102.0), Some(DropZone::Top)); // fy .01 < fx .125
                                                                     // Outside / empty.
        assert_eq!(drop_zone(r, 99.0, 150.0), None);
        assert_eq!(drop_zone(r, 500.0, 150.0), None);
        assert_eq!(drop_zone(PxRect::new(0.0, 0.0, 0.0, 10.0), 0.0, 1.0), None);
    }

    #[test]
    fn pane_drop_target_ignores_gaps_and_previews_halves() {
        let mut l = WorkspaceLayout::new(1);
        l.split_pane(1, 2, SplitDirection::Right, &mut alloc_from(100));
        let s = solve(&l.group, CONTAINER, 6.0);
        let r1 = s.pane_rect(1).unwrap();
        let gap_x = r1.right() + 3.0;
        assert_eq!(pane_drop_target(&s.panes, gap_x, 300.0), None);
        let (id, zone) = pane_drop_target(&s.panes, r1.x + 5.0, 300.0).unwrap();
        assert_eq!((id, zone), (1, DropZone::Left));
        let half = drop_preview(r1, DropZone::Left);
        assert!(close(half.w, r1.w / 2.0) && close(half.x, r1.x));
        assert_eq!(drop_preview(r1, DropZone::Centre), r1);
    }

    #[test]
    fn boundary_fraction_accounts_for_gaps() {
        let mut g = PaneGroup::new(1);
        let mut alloc = alloc_from(100);
        g.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        g.split_pane(2, 3, SplitDirection::Right, &mut alloc);
        let s = solve(&g, CONTAINER, 6.0);
        let ax = s.axis(101).unwrap();
        // Dragging the first handle to where its centre already is leaves the
        // boundary at 1/3 of the shared space.
        let center = ax.handles[0].x + 3.0;
        let f = ax.boundary_fraction(0, center, 240.0).unwrap();
        assert!(close(f * 1000.0, 1000.0 / 3.0), "{f}");
        // Second handle: cumulative fraction 2/3.
        let center = ax.handles[1].x + 3.0;
        let f = ax.boundary_fraction(1, center, 240.0).unwrap();
        assert!(close(f * 1000.0, 2000.0 / 3.0), "{f}");
        // Out of range / no room.
        assert_eq!(ax.boundary_fraction(2, 0.0, 0.0), None);
    }

    #[test]
    fn boundary_fraction_clamps_to_pixel_minimums() {
        let mut g = PaneGroup::new(1);
        g.split_pane(1, 2, SplitDirection::Right, &mut alloc_from(100));
        let s = solve(&g, CONTAINER, 6.0);
        let ax = s.axis(101).unwrap();
        // Far left: first pane keeps >= 240px of the 1194 shared px.
        let f = ax.boundary_fraction(0, -500.0, 240.0).unwrap();
        assert!(close(f * 1194.0, 240.0), "{f}");
        let f = ax.boundary_fraction(0, 5000.0, 240.0).unwrap();
        assert!(close((1.0 - f) * 1194.0, 240.0), "{f}");
        // If the pair cannot afford the minimum, fall back to the ratio floor.
        let tiny = solve(&g, PxRect::new(0.0, 0.0, 300.0, 200.0), 6.0);
        let ax = tiny.axis(101).unwrap();
        let f = ax.boundary_fraction(0, -100.0, 240.0).unwrap();
        assert!(close(f, MIN_RATIO));
        // The result is directly usable with the existing set_boundary.
        let mut g2 = g.clone();
        assert!(g2.set_boundary(101, 0, f));
    }

    // ───────────────────────────── properties ────────────────────────────

    fn overlaps(a: &PxRect, b: &PxRect) -> bool {
        a.x < b.right() - 0.01
            && b.x < a.right() - 0.01
            && a.y < b.bottom() - 0.01
            && b.y < a.bottom() - 0.01
    }

    /// Panes + handles partition the container exactly: areas add up, nothing
    /// overlaps and everything lies inside.
    fn assert_tiles(layout: &WorkspaceLayout, container: PxRect, gap: f32) {
        let s = solve(&layout.group, container, gap);
        let handles: Vec<PxRect> = s.axes.iter().flat_map(|a| a.handles.clone()).collect();
        let panes: Vec<PxRect> = s.panes.iter().map(|(_, r)| *r).collect();
        let total: f64 = panes
            .iter()
            .chain(handles.iter())
            .map(|r| r.area() as f64)
            .sum();
        let want = container.area() as f64;
        assert!(
            (total - want).abs() <= want * 1e-4 + 1.0,
            "area {total} != {want}"
        );
        for r in panes.iter().chain(handles.iter()) {
            assert!(r.x >= container.x - 0.01 && r.y >= container.y - 0.01);
            assert!(r.right() <= container.right() + 0.01);
            assert!(r.bottom() <= container.bottom() + 0.01);
            assert!(r.w >= -0.001 && r.h >= -0.001);
        }
        for (i, a) in panes.iter().enumerate() {
            for b in panes.iter().skip(i + 1) {
                assert!(!overlaps(a, b), "panes overlap: {a:?} {b:?}");
            }
            for h in &handles {
                assert!(!overlaps(a, h), "pane overlaps handle: {a:?} {h:?}");
            }
        }
    }

    #[test]
    fn random_layouts_tile_the_container_and_respect_minimums() {
        for seed in 1..=60u64 {
            let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
            let m = if seed % 2 == 0 {
                TilingMetrics::comfortable()
            } else {
                TilingMetrics::compact()
            };
            let container = PxRect::new(
                rng.below(40) as f32,
                rng.below(40) as f32,
                400.0 + rng.below(1800) as f32,
                300.0 + rng.below(900) as f32,
            );
            let mut next: PaneId = 1;
            let mut layout = WorkspaceLayout::new(next);
            for step in 0..60 {
                let leaves = layout.leaves();
                let t = leaves[rng.below(leaves.len())];
                let mut axis_ids = 50_000 + (step as u64) * 100;
                let mut alloc = || {
                    axis_ids += 1;
                    axis_ids + seed * 1_000_000
                };
                match rng.below(5) {
                    0..=2 => {
                        next += 1;
                        let before = solve(&layout.group, container, m.gap);
                        let res = if rng.below(2) == 0 {
                            smart_split(&mut layout, t, next, &mut alloc, container, &m).map(|_| ())
                        } else {
                            let dir = [
                                SplitDirection::Up,
                                SplitDirection::Down,
                                SplitDirection::Left,
                                SplitDirection::Right,
                            ][rng.below(4)];
                            try_split(&mut layout, t, next, dir, &mut alloc, container, &m)
                        };
                        let after = solve(&layout.group, container, m.gap);
                        match res {
                            Ok(()) => {
                                assert!(layout.group.find_pane(next));
                                assert!(
                                    respects_minimum(&before, &after, &m),
                                    "seed {seed} step {step}: pane below minimum"
                                );
                            }
                            Err(_) => {
                                assert!(!layout.group.find_pane(next));
                                assert_eq!(before, after, "rejected split changed layout");
                            }
                        }
                    }
                    3 => {
                        let _ = layout.close_pane(t);
                    }
                    _ => {
                        let o = leaves[rng.below(leaves.len())];
                        let zone = [
                            DropZone::Left,
                            DropZone::Right,
                            DropZone::Top,
                            DropZone::Bottom,
                            DropZone::Centre,
                        ][rng.below(5)];
                        let _ = layout.move_pane(t, o, zone, &mut alloc);
                    }
                }
                assert_eq!(
                    layout.validate_normalized(),
                    Ok(()),
                    "seed {seed} step {step}"
                );
                assert_tiles(&layout, container, m.gap);

                // Neighbour answers are real panes lying in the right direction.
                let s = solve(&layout.group, container, m.gap);
                for (id, r) in &s.panes {
                    for dir in [
                        SplitDirection::Left,
                        SplitDirection::Right,
                        SplitDirection::Up,
                        SplitDirection::Down,
                    ] {
                        if let Some(n) = neighbor(&s.panes, *id, dir, false) {
                            let nr = s.pane_rect(n).unwrap();
                            assert_ne!(n, *id);
                            match dir {
                                SplitDirection::Left => assert!(nr.right() <= r.x + EPS),
                                SplitDirection::Right => assert!(nr.x >= r.right() - EPS),
                                SplitDirection::Up => assert!(nr.bottom() <= r.y + EPS),
                                SplitDirection::Down => assert!(nr.y >= r.bottom() - EPS),
                            }
                        }
                    }
                }
            }
        }
    }
}
