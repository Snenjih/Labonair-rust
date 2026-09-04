//! Recursive n-ary split-pane tree (T17-004).
//!
//! A workspace tab hosts a **pane tree** modelled after Zed's
//! `crates/workspace/src/pane_group.rs`:
//!
//! * [`Member`] is either a single content slot ([`Member::Pane`]) or an
//!   [`PaneAxis`] that arranges several members along one [`SplitAxis`].
//! * [`PaneAxis`] carries `flexes` — one fraction per member, invariantly
//!   summing to `1.0` — so splits can be resized to arbitrary ratios and any
//!   depth of nesting is possible.
//! * [`PaneGroup`]'s `root` is **optional**: a workspace with no open panes has
//!   `root == None` and renders nothing (the empty surface is drawn one level
//!   up, in [`Workspace`](crate::Workspace); see `docs/architecture.md §8.2`).
//!   Removing the last pane collapses `root` to `None` — that is a valid state,
//!   not an error.
//!
//! Zed-specific concerns (collab-cursor decoration, `bounding_boxes` for
//! item drag-drop, pixel min-sizes) are deliberately left out until needed.
//!
//! The tree is pure data; session persistence lives in [`crate::session`]
//! (`SerializedPaneGroup`), which also migrates the pre-T17-004 flat binary
//! `split` snapshots into this model.

use crate::pane::{CloseOutcome, PaneId, SplitAxis, MIN_RATIO};

/// Which edge of the target pane a new pane is inserted on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SplitDirection {
    /// The axis a split in this direction runs along.
    pub fn axis(self) -> SplitAxis {
        match self {
            SplitDirection::Up | SplitDirection::Down => SplitAxis::Vertical,
            SplitDirection::Left | SplitDirection::Right => SplitAxis::Horizontal,
        }
    }

    /// Whether the new pane is placed *after* the target along the axis
    /// (`Down` / `Right`) rather than before it (`Up` / `Left`).
    pub fn increasing(self) -> bool {
        matches!(self, SplitDirection::Down | SplitDirection::Right)
    }

    /// The natural direction for a bare `SplitAxis` request (`Right` /
    /// `Down`) — used by the current split actions which only carry an axis.
    pub fn along(axis: SplitAxis) -> Self {
        match axis {
            SplitAxis::Horizontal => SplitDirection::Right,
            SplitAxis::Vertical => SplitDirection::Down,
        }
    }
}

/// Even `flexes` for an `n`-member axis (`1/n` each).
fn even_flexes(n: usize) -> Vec<f32> {
    if n == 0 {
        Vec::new()
    } else {
        vec![1.0 / n as f32; n]
    }
}

/// A node in the pane tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    /// A single content slot, keyed by [`PaneId`].
    Pane(PaneId),
    /// A horizontal or vertical arrangement of two-or-more members.
    Axis(PaneAxis),
}

impl Member {
    /// A bare leaf.
    pub fn leaf(id: PaneId) -> Self {
        Member::Pane(id)
    }

    /// Append every leaf id, left → right / top → bottom.
    pub fn collect_leaves(&self, out: &mut Vec<PaneId>) {
        match self {
            Member::Pane(id) => out.push(*id),
            Member::Axis(ax) => {
                for m in &ax.members {
                    m.collect_leaves(out);
                }
            }
        }
    }

    /// All leaf ids in visual order.
    pub fn leaves(&self) -> Vec<PaneId> {
        let mut v = Vec::new();
        self.collect_leaves(&mut v);
        v
    }

    /// The first (left-most / top-most) leaf.
    pub fn first_leaf(&self) -> PaneId {
        match self {
            Member::Pane(id) => *id,
            Member::Axis(ax) => ax.members[0].first_leaf(),
        }
    }

    /// The last (right-most / bottom-most) leaf.
    pub fn last_leaf(&self) -> PaneId {
        match self {
            Member::Pane(id) => *id,
            Member::Axis(ax) => ax.members[ax.members.len() - 1].last_leaf(),
        }
    }

    /// Whether `target` is a leaf somewhere in this subtree.
    pub fn contains(&self, target: PaneId) -> bool {
        match self {
            Member::Pane(id) => *id == target,
            Member::Axis(ax) => ax.members.iter().any(|m| m.contains(target)),
        }
    }

    fn split(
        &mut self,
        new_axis_id: PaneId,
        target: PaneId,
        new_pane: PaneId,
        dir: SplitDirection,
    ) -> bool {
        match self {
            Member::Pane(id) => {
                if *id == target {
                    *self = Member::Axis(PaneAxis::pair(new_axis_id, *id, new_pane, dir));
                    true
                } else {
                    false
                }
            }
            Member::Axis(ax) => ax.split(new_axis_id, target, new_pane, dir),
        }
    }

    fn set_boundary(&mut self, axis_id: PaneId, boundary: usize, frac: f32) -> bool {
        let Member::Axis(ax) = self else {
            return false;
        };
        if ax.id == axis_id {
            return ax.set_boundary(boundary, frac);
        }
        ax.members
            .iter_mut()
            .any(|m| m.set_boundary(axis_id, boundary, frac))
    }

    fn reset_axis(&mut self, axis_id: PaneId) -> bool {
        let Member::Axis(ax) = self else {
            return false;
        };
        if ax.id == axis_id {
            ax.flexes = even_flexes(ax.members.len());
            return true;
        }
        ax.members.iter_mut().any(|m| m.reset_axis(axis_id))
    }
}

/// One axis of the split tree: `members` arranged along `axis`, sized by
/// `flexes` (`flexes.len() == members.len()`, `sum(flexes) == 1.0`).
#[derive(Debug, Clone, PartialEq)]
pub struct PaneAxis {
    /// Stable id, used to address this axis for resize / reset.
    pub id: PaneId,
    pub axis: SplitAxis,
    pub members: Vec<Member>,
    pub flexes: Vec<f32>,
}

impl PaneAxis {
    /// A new axis with `members` split evenly.
    pub fn new(id: PaneId, axis: SplitAxis, members: Vec<Member>) -> Self {
        let flexes = even_flexes(members.len());
        Self {
            id,
            axis,
            members,
            flexes,
        }
    }

    /// A new axis, adopting `flexes` if they are well-formed (right length,
    /// all finite & positive, sum ≈ 1.0) and falling back to an even split
    /// otherwise. Used when rebuilding a persisted tree.
    pub fn with_flexes(
        id: PaneId,
        axis: SplitAxis,
        members: Vec<Member>,
        flexes: Vec<f32>,
    ) -> Self {
        let n = members.len();
        let ok = flexes.len() == n
            && flexes.iter().all(|f| f.is_finite() && *f > 0.0)
            && (flexes.iter().sum::<f32>() - 1.0).abs() < 1e-3;
        let flexes = if ok {
            let sum: f32 = flexes.iter().sum();
            flexes.iter().map(|f| f / sum).collect()
        } else {
            even_flexes(n)
        };
        Self {
            id,
            axis,
            members,
            flexes,
        }
    }

    /// Two panes side by side along `dir`.
    fn pair(id: PaneId, old: PaneId, new: PaneId, dir: SplitDirection) -> Self {
        let members = if dir.increasing() {
            vec![Member::Pane(old), Member::Pane(new)]
        } else {
            vec![Member::Pane(new), Member::Pane(old)]
        };
        PaneAxis::new(id, dir.axis(), members)
    }

    fn split(
        &mut self,
        new_axis_id: PaneId,
        target: PaneId,
        new_pane: PaneId,
        dir: SplitDirection,
    ) -> bool {
        for i in 0..self.members.len() {
            match &mut self.members[i] {
                Member::Axis(child) => {
                    if child.split(new_axis_id, target, new_pane, dir) {
                        return true;
                    }
                }
                Member::Pane(id) => {
                    if *id == target {
                        if dir.axis() == self.axis {
                            let ix = if dir.increasing() { i + 1 } else { i };
                            self.members.insert(ix, Member::Pane(new_pane));
                            self.flexes = even_flexes(self.members.len());
                        } else {
                            self.members[i] =
                                Member::Axis(PaneAxis::pair(new_axis_id, target, new_pane, dir));
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Remove `pane`. Returns [`RemoveStep::Collapse`] with the sole remaining
    /// member when this axis drops to one child (the caller promotes it).
    fn remove(&mut self, pane: PaneId, neighbor: &mut Option<PaneId>) -> RemoveStep {
        for i in 0..self.members.len() {
            match &mut self.members[i] {
                Member::Pane(id) if *id == pane => {
                    *neighbor = Some(self.sibling_leaf(i));
                    self.members.remove(i);
                    if self.members.len() == 1 {
                        return RemoveStep::Collapse(self.members.pop().unwrap());
                    }
                    self.flexes = even_flexes(self.members.len());
                    return RemoveStep::Handled;
                }
                Member::Pane(_) => {}
                Member::Axis(child) => match child.remove(pane, neighbor) {
                    RemoveStep::NotFound => {}
                    RemoveStep::Handled => return RemoveStep::Handled,
                    RemoveStep::Collapse(m) => {
                        self.members[i] = m;
                        return RemoveStep::Handled;
                    }
                },
            }
        }
        RemoveStep::NotFound
    }

    /// A leaf adjacent to member `ix` — the previous member's last leaf, or
    /// (for `ix == 0`) the next member's first leaf.
    fn sibling_leaf(&self, ix: usize) -> PaneId {
        if ix > 0 {
            self.members[ix - 1].last_leaf()
        } else {
            self.members[ix + 1].first_leaf()
        }
    }

    /// Set the boundary between member `boundary` and `boundary + 1` to the
    /// axis-fraction `frac`, adjusting only those two flexes (their sum — and
    /// therefore the whole axis' sum — is preserved exactly).
    fn set_boundary(&mut self, boundary: usize, frac: f32) -> bool {
        if boundary + 1 >= self.members.len() {
            return false;
        }
        let before: f32 = self.flexes[..boundary].iter().sum();
        let pair = self.flexes[boundary] + self.flexes[boundary + 1];
        let min = MIN_RATIO * pair;
        let first = (frac - before).clamp(min, pair - min);
        self.flexes[boundary] = first;
        self.flexes[boundary + 1] = pair - first;
        true
    }
}

enum RemoveStep {
    /// `pane` was not in this subtree.
    NotFound,
    /// `pane` was removed; nothing to promote.
    Handled,
    /// This axis collapsed to a single member — promote it in place.
    Collapse(Member),
}

/// Outcome of [`PaneGroup::remove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// The pane was removed; `neighbor` is the natural new active leaf, or
    /// `None` when the tree is now empty.
    Removed { neighbor: Option<PaneId> },
    /// `pane` is not in this tree.
    NotFound,
}

/// The recursive split tree of one workspace tab. `root` is `None` when the
/// tab holds no panes.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneGroup {
    pub root: Option<Member>,
}

impl PaneGroup {
    /// A single-pane group.
    pub fn new(first: PaneId) -> Self {
        Self {
            root: Some(Member::Pane(first)),
        }
    }

    /// A group with no panes.
    pub fn empty() -> Self {
        Self { root: None }
    }

    /// Whether the tree holds no panes.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Every leaf id, in visual order (empty when `root == None`).
    pub fn panes(&self) -> Vec<PaneId> {
        let mut v = Vec::new();
        if let Some(root) = &self.root {
            root.collect_leaves(&mut v);
        }
        v
    }

    /// The first leaf, if any.
    pub fn first_pane(&self) -> Option<PaneId> {
        self.root.as_ref().map(Member::first_leaf)
    }

    /// Whether `id` is a leaf in this tree.
    pub fn find_pane(&self, id: PaneId) -> bool {
        self.root.as_ref().is_some_and(|m| m.contains(id))
    }

    /// Split `target`, inserting `new_pane` on the `dir` edge. A brand-new
    /// axis (when `target`'s parent axis runs the other way, or `target` is
    /// the whole root) is given id `new_axis_id`. From an empty tree the new
    /// pane simply becomes the root. Returns `false` only if `target` is
    /// absent *and* the tree has no pane to fall back on.
    pub fn split(
        &mut self,
        new_axis_id: PaneId,
        target: PaneId,
        new_pane: PaneId,
        dir: SplitDirection,
    ) -> bool {
        match &mut self.root {
            None => {
                self.root = Some(Member::Pane(new_pane));
                true
            }
            Some(root) => {
                if root.split(new_axis_id, target, new_pane, dir) {
                    return true;
                }
                // Fallback: target vanished — split the first pane instead.
                let first = root.first_leaf();
                root.split(new_axis_id, first, new_pane, dir)
            }
        }
    }

    /// Remove `pane`, collapsing any axis that drops to a single member and
    /// clearing `root` if it was the last pane.
    pub fn remove(&mut self, pane: PaneId) -> RemoveOutcome {
        let mut neighbor = None;
        match &mut self.root {
            None => RemoveOutcome::NotFound,
            Some(Member::Pane(id)) => {
                if *id == pane {
                    self.root = None;
                    RemoveOutcome::Removed { neighbor: None }
                } else {
                    RemoveOutcome::NotFound
                }
            }
            Some(Member::Axis(ax)) => match ax.remove(pane, &mut neighbor) {
                RemoveStep::NotFound => RemoveOutcome::NotFound,
                RemoveStep::Handled => RemoveOutcome::Removed { neighbor },
                RemoveStep::Collapse(m) => {
                    self.root = Some(m);
                    RemoveOutcome::Removed { neighbor }
                }
            },
        }
    }

    /// Move the boundary after member `boundary` of axis `axis_id` to
    /// `frac` (fraction of that axis). Only the two adjacent flexes change.
    pub fn set_boundary(&mut self, axis_id: PaneId, boundary: usize, frac: f32) -> bool {
        self.root
            .as_mut()
            .is_some_and(|m| m.set_boundary(axis_id, boundary, frac))
    }

    /// Reset axis `axis_id` to an even split.
    pub fn reset_axis(&mut self, axis_id: PaneId) -> bool {
        self.root.as_mut().is_some_and(|m| m.reset_axis(axis_id))
    }
}

/// The pane tree of one workspace tab, plus which leaf is active (`None` only
/// while the tree is empty). Stored per tab in [`Workspace`](crate::Workspace)
/// and serialised for session restore via [`crate::session`].
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceLayout {
    pub group: PaneGroup,
    pub active: Option<PaneId>,
}

impl WorkspaceLayout {
    /// A fresh single-pane layout.
    pub fn new(first: PaneId) -> Self {
        Self {
            group: PaneGroup::new(first),
            active: Some(first),
        }
    }

    /// An empty layout (no panes).
    pub fn empty() -> Self {
        Self {
            group: PaneGroup::empty(),
            active: None,
        }
    }

    /// All leaf ids in visual order.
    pub fn leaves(&self) -> Vec<PaneId> {
        self.group.panes()
    }

    /// Number of panes.
    pub fn len(&self) -> usize {
        self.group.panes().len()
    }

    /// Whether the layout holds no panes.
    pub fn is_empty(&self) -> bool {
        self.group.is_empty()
    }

    /// Split the active pane along `dir`, inserting `new_pane`; a fresh axis
    /// (if needed) takes id `new_axis_id`. The new pane becomes active.
    pub fn split(&mut self, new_axis_id: PaneId, new_pane: PaneId, dir: SplitDirection) -> PaneId {
        if self.group.is_empty() {
            self.group.root = Some(Member::Pane(new_pane));
        } else {
            let target = self
                .active
                .or_else(|| self.group.first_pane())
                .unwrap_or(new_pane);
            self.group.split(new_axis_id, target, new_pane, dir);
        }
        self.active = Some(new_pane);
        new_pane
    }

    /// Close a pane, collapsing its parent axis into the surviving members.
    /// If `target` is the only pane the tree is left untouched and
    /// [`CloseOutcome::LastPane`] is returned (the caller decides whether the
    /// tab itself closes).
    pub fn close(&mut self, target: PaneId) -> CloseOutcome {
        if !self.group.find_pane(target) {
            return CloseOutcome::NotFound;
        }
        if self.group.panes().len() == 1 {
            return CloseOutcome::LastPane;
        }
        match self.group.remove(target) {
            RemoveOutcome::NotFound => CloseOutcome::NotFound,
            RemoveOutcome::Removed { neighbor } => {
                let active_valid = self.active.is_some_and(|a| self.group.find_pane(a));
                if self.active == Some(target) || !active_valid {
                    self.active = neighbor;
                }
                let new_active = self
                    .active
                    .or(neighbor)
                    .or_else(|| self.group.first_pane())
                    .unwrap_or(target);
                CloseOutcome::Closed { new_active }
            }
        }
    }

    /// Make `id` the active pane, if it is a leaf here.
    pub fn set_active(&mut self, id: PaneId) -> bool {
        if self.group.find_pane(id) {
            self.active = Some(id);
            true
        } else {
            false
        }
    }

    /// Move a resize boundary — see [`PaneGroup::set_boundary`].
    pub fn set_boundary(&mut self, axis_id: PaneId, boundary: usize, frac: f32) -> bool {
        self.group.set_boundary(axis_id, boundary, frac)
    }

    /// Reset an axis to an even split.
    pub fn reset_axis(&mut self, axis_id: PaneId) -> bool {
        self.group.reset_axis(axis_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis_at(m: &Member, id: PaneId) -> Option<&PaneAxis> {
        match m {
            Member::Pane(_) => None,
            Member::Axis(ax) => {
                if ax.id == id {
                    Some(ax)
                } else {
                    ax.members.iter().find_map(|c| axis_at(c, id))
                }
            }
        }
    }

    fn flex_sum_ok(m: &Member) -> bool {
        match m {
            Member::Pane(_) => true,
            Member::Axis(ax) => {
                (ax.flexes.iter().sum::<f32>() - 1.0).abs() < 1e-4
                    && ax.flexes.len() == ax.members.len()
                    && ax.members.iter().all(flex_sum_ok)
            }
        }
    }

    #[test]
    fn split_from_empty_makes_root() {
        let mut g = PaneGroup::empty();
        assert!(g.is_empty());
        assert!(g.split(100, 0, 1, SplitDirection::Right));
        assert_eq!(g.panes(), vec![1]);
        assert!(!g.is_empty());
    }

    #[test]
    fn split_in_all_four_directions() {
        // Right: [1 | 2]
        let mut g = PaneGroup::new(1);
        g.split(100, 1, 2, SplitDirection::Right);
        assert_eq!(g.panes(), vec![1, 2]);
        // Left of 1: [3 | 1 | 2] (same axis, inserted before)
        g.split(101, 1, 3, SplitDirection::Left);
        assert_eq!(g.panes(), vec![3, 1, 2]);
        // Down on 1: 1 becomes a vertical sub-axis [1 / 4]
        g.split(102, 1, 4, SplitDirection::Down);
        assert_eq!(g.panes(), vec![3, 1, 4, 2]);
        // Up on 4: [3 | (1 / 5 / 4) | 2]
        g.split(103, 4, 5, SplitDirection::Up);
        assert_eq!(g.panes(), vec![3, 1, 5, 4, 2]);

        let root = g.root.as_ref().unwrap();
        assert!(flex_sum_ok(root));
        let inner = axis_at(root, 102).unwrap();
        assert_eq!(inner.axis, SplitAxis::Vertical);
        assert_eq!(inner.members.len(), 3);
    }

    #[test]
    fn remove_collapses_axes_and_can_empty_the_tree() {
        let mut g = PaneGroup::new(1);
        g.split(100, 1, 2, SplitDirection::Right);
        g.split(101, 2, 3, SplitDirection::Down); // [1 | (2 / 3)]

        // Remove 3 → inner axis collapses, back to [1 | 2].
        assert_eq!(g.remove(3), RemoveOutcome::Removed { neighbor: Some(2) });
        assert_eq!(g.panes(), vec![1, 2]);
        assert!(matches!(g.root, Some(Member::Axis(_))));

        // Remove 2 → outer axis collapses to a bare leaf.
        assert_eq!(g.remove(2), RemoveOutcome::Removed { neighbor: Some(1) });
        assert!(matches!(g.root, Some(Member::Pane(1))));

        // Remove the last pane → empty, no panic.
        assert_eq!(g.remove(1), RemoveOutcome::Removed { neighbor: None });
        assert!(g.is_empty());
        assert_eq!(g.remove(1), RemoveOutcome::NotFound);
    }

    #[test]
    fn resize_only_touches_two_adjacent_flexes() {
        let mut g = PaneGroup::new(1);
        g.split(100, 1, 2, SplitDirection::Right);
        g.split(100, 2, 3, SplitDirection::Right); // 3-member axis, id 100
        g.split(100, 3, 4, SplitDirection::Right); // 4-member axis: [1|2|3|4] each .25

        // Drag boundary 1 (between member 1 and 2) to 0.6 of the axis.
        assert!(g.set_boundary(100, 1, 0.6));
        let ax = axis_at(g.root.as_ref().unwrap(), 100).unwrap();
        assert!((ax.flexes[0] - 0.25).abs() < 1e-4, "outer flex untouched");
        assert!((ax.flexes[3] - 0.25).abs() < 1e-4, "outer flex untouched");
        assert!((ax.flexes[1] - 0.35).abs() < 1e-4); // 0.6 - 0.25
        assert!((ax.flexes[2] - 0.15).abs() < 1e-4); // pair 0.5 - 0.35
        assert!((ax.flexes.iter().sum::<f32>() - 1.0).abs() < 1e-5);

        // Clamped: dragging far past the neighbour keeps a minimum sliver.
        assert!(g.set_boundary(100, 1, 5.0));
        let ax = axis_at(g.root.as_ref().unwrap(), 100).unwrap();
        assert!(ax.flexes[2] > 0.0);
        assert!((ax.flexes.iter().sum::<f32>() - 1.0).abs() < 1e-5);

        assert!(g.reset_axis(100));
        let ax = axis_at(g.root.as_ref().unwrap(), 100).unwrap();
        assert!(ax.flexes.iter().all(|f| (*f - 0.25).abs() < 1e-4));

        assert!(!g.set_boundary(999, 0, 0.5));
    }

    #[test]
    fn layout_close_keeps_a_sensible_active_neighbour() {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitDirection::Right); // active 2
        l.split(101, 3, SplitDirection::Down); // active 3, tree [1 | (2 / 3)]
        assert_eq!(l.active, Some(3));

        assert_eq!(l.close(3), CloseOutcome::Closed { new_active: 2 });
        assert_eq!(l.active, Some(2));

        assert_eq!(l.close(2), CloseOutcome::Closed { new_active: 1 });
        assert_eq!(l.active, Some(1));

        assert_eq!(l.close(1), CloseOutcome::LastPane);
        assert_eq!(l.close(42), CloseOutcome::NotFound);
    }

    #[test]
    fn deep_nesting_round_trips_leaf_order() {
        let mut l = WorkspaceLayout::new(1);
        for i in 2..=8 {
            let dir = if i % 2 == 0 {
                SplitDirection::Right
            } else {
                SplitDirection::Down
            };
            l.split(1000 + i, i, dir);
        }
        assert_eq!(l.len(), 8);
        let mut leaves = l.leaves();
        leaves.sort_unstable();
        assert_eq!(leaves, (1..=8).collect::<Vec<_>>());

        for id in [2u64, 4, 6, 8, 3, 5, 7] {
            l.close(id);
            assert!(!l.leaves().is_empty());
            assert!(l.active.is_some_and(|a| l.group.find_pane(a)));
        }
        assert_eq!(l.leaves(), vec![1]);
    }
}
