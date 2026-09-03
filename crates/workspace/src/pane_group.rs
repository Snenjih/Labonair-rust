//! Recursive split-pane tree (T04-002).
//!
//! A workspace tab hosts a **pane tree**: a binary tree where every leaf
//! ([`PaneNode::Pane`]) is one content slot (a terminal today, an editor
//! later) and every inner node ([`PaneNode::Split`]) divides its area between
//! two children — horizontally or vertically — at an adjustable `ratio`.
//!
//! This mirrors the reference `PaneNode = PaneSplit | PaneLeaf` model in
//! `reference-src/src/modules/tabs/types.ts` and the `splitPane` / `closePane`
//! reducers in `store/tabsStore.ts`, ported to Rust. Pane ids are allocated by
//! the caller (the [`Workspace`](crate::Workspace) view keeps a process-wide
//! counter so ids stay unique across tabs); the layout itself is pure data and
//! is [`Serialize`]/[`Deserialize`] for session persistence.
//!
//! Split out of [`crate::pane`] in T16-006 to prepare T17-004.

use serde::{Deserialize, Serialize};

use crate::pane::{CloseOutcome, PaneId, SplitAxis, MIN_RATIO};

/// A node in the pane tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PaneNode {
    /// A single content slot.
    Pane { id: PaneId },
    /// A binary split; `ratio` is the fraction of the axis taken by `first`.
    Split {
        id: PaneId,
        axis: SplitAxis,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// A bare leaf.
    pub fn leaf(id: PaneId) -> Self {
        PaneNode::Pane { id }
    }

    /// Append every leaf id, left to right.
    pub fn collect_leaves(&self, out: &mut Vec<PaneId>) {
        match self {
            PaneNode::Pane { id } => out.push(*id),
            PaneNode::Split { first, second, .. } => {
                first.collect_leaves(out);
                second.collect_leaves(out);
            }
        }
    }

    /// All leaf ids, left to right.
    pub fn leaves(&self) -> Vec<PaneId> {
        let mut v = Vec::new();
        self.collect_leaves(&mut v);
        v
    }

    /// The first (left-most / top-most) leaf id.
    pub fn first_leaf(&self) -> PaneId {
        match self {
            PaneNode::Pane { id } => *id,
            PaneNode::Split { first, .. } => first.first_leaf(),
        }
    }

    /// Whether `target` is a leaf somewhere in this subtree.
    pub fn contains(&self, target: PaneId) -> bool {
        match self {
            PaneNode::Pane { id } => *id == target,
            PaneNode::Split { first, second, .. } => {
                first.contains(target) || second.contains(target)
            }
        }
    }

    fn replace_leaf(&mut self, target: PaneId, replacement: PaneNode) -> bool {
        match self {
            PaneNode::Pane { id } if *id == target => {
                *self = replacement;
                true
            }
            PaneNode::Pane { .. } => false,
            PaneNode::Split { first, second, .. } => {
                first.replace_leaf(target, replacement.clone())
                    || second.replace_leaf(target, replacement)
            }
        }
    }

    /// Collapse the split that has `target` as a direct leaf child, promoting
    /// `target`'s sibling in its place. Returns the first leaf of that promoted
    /// sibling subtree (the natural new active pane), or `None` if `target`
    /// isn't a direct-or-nested leaf child of any split here.
    fn remove_leaf(&mut self, target: PaneId) -> Option<PaneId> {
        let PaneNode::Split { first, second, .. } = self else {
            return None;
        };
        if matches!(**first, PaneNode::Pane { id } if id == target) {
            let sibling = (**second).clone();
            let first_leaf = sibling.first_leaf();
            *self = sibling;
            return Some(first_leaf);
        }
        if matches!(**second, PaneNode::Pane { id } if id == target) {
            let sibling = (**first).clone();
            let first_leaf = sibling.first_leaf();
            *self = sibling;
            return Some(first_leaf);
        }
        first
            .remove_leaf(target)
            .or_else(|| second.remove_leaf(target))
    }

    fn set_ratio(&mut self, split_id: PaneId, ratio: f32) -> bool {
        match self {
            PaneNode::Pane { .. } => false,
            PaneNode::Split {
                id,
                ratio: r,
                first,
                second,
                ..
            } => {
                if *id == split_id {
                    *r = ratio.clamp(MIN_RATIO, 1.0 - MIN_RATIO);
                    true
                } else {
                    first.set_ratio(split_id, ratio) || second.set_ratio(split_id, ratio)
                }
            }
        }
    }
}

/// The pane tree of one workspace tab, plus which leaf is active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub root: PaneNode,
    pub active: PaneId,
}

impl WorkspaceLayout {
    /// A fresh single-pane layout.
    pub fn new(first: PaneId) -> Self {
        Self {
            root: PaneNode::leaf(first),
            active: first,
        }
    }

    /// All leaf ids, left to right.
    pub fn leaves(&self) -> Vec<PaneId> {
        self.root.leaves()
    }

    /// Number of panes.
    pub fn len(&self) -> usize {
        self.root.leaves().len()
    }

    /// Always at least one pane.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Split the active pane along `axis`, inserting `new_pane` next to it under
    /// a new split node with id `split_id`. The new pane becomes active.
    /// `split_id` / `new_pane` are caller-allocated and must be unused.
    pub fn split(&mut self, split_id: PaneId, new_pane: PaneId, axis: SplitAxis) -> PaneId {
        let target = self.active;
        let split = PaneNode::Split {
            id: split_id,
            axis,
            ratio: 0.5,
            first: Box::new(PaneNode::leaf(target)),
            second: Box::new(PaneNode::leaf(new_pane)),
        };
        if self.root.replace_leaf(target, split) {
            self.active = new_pane;
        }
        new_pane
    }

    /// Close a pane, collapsing its parent split into the sibling. If `target`
    /// is the only pane, returns [`CloseOutcome::LastPane`] and leaves the tree
    /// untouched.
    pub fn close(&mut self, target: PaneId) -> CloseOutcome {
        if !self.root.contains(target) {
            return CloseOutcome::NotFound;
        }
        if matches!(self.root, PaneNode::Pane { .. }) {
            return CloseOutcome::LastPane;
        }
        match self.root.remove_leaf(target) {
            Some(sibling_first) => {
                if self.active == target || !self.root.contains(self.active) {
                    self.active = sibling_first;
                }
                CloseOutcome::Closed {
                    new_active: self.active,
                }
            }
            None => CloseOutcome::NotFound,
        }
    }

    /// Make `id` the active pane, if it is a leaf here.
    pub fn set_active(&mut self, id: PaneId) -> bool {
        if self.root.contains(id) {
            self.active = id;
            true
        } else {
            false
        }
    }

    /// Set the split `split_id`'s first-child fraction (clamped to
    /// `[MIN_RATIO, 1 - MIN_RATIO]`).
    pub fn set_ratio(&mut self, split_id: PaneId, ratio: f32) -> bool {
        self.root.set_ratio(split_id, ratio)
    }

    /// Reset the split `split_id` back to an even 50/50.
    pub fn reset_ratio(&mut self, split_id: PaneId) -> bool {
        self.root.set_ratio(split_id, 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_nests_and_activates_new_pane() {
        let mut l = WorkspaceLayout::new(1);
        assert_eq!(l.len(), 1);

        l.split(100, 2, SplitAxis::Horizontal);
        assert_eq!(l.active, 2);
        assert_eq!(l.leaves(), vec![1, 2]);

        // Split the now-active pane 2 vertically.
        l.split(101, 3, SplitAxis::Vertical);
        assert_eq!(l.active, 3);
        assert_eq!(l.leaves(), vec![1, 2, 3]);

        match &l.root {
            PaneNode::Split { axis, first, .. } => {
                assert_eq!(*axis, SplitAxis::Horizontal);
                assert!(matches!(**first, PaneNode::Pane { id: 1 }));
            }
            _ => panic!("root must be a split"),
        }
    }

    #[test]
    fn close_collapses_split_into_sibling() {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitAxis::Horizontal);
        l.split(101, 3, SplitAxis::Vertical); // tree: 1 | (2 / 3), active 3

        assert_eq!(l.close(3), CloseOutcome::Closed { new_active: 2 });
        assert_eq!(l.leaves(), vec![1, 2]);
        assert_eq!(l.active, 2);

        // Closing again collapses the last split — root becomes a bare leaf.
        assert_eq!(l.close(2), CloseOutcome::Closed { new_active: 1 });
        assert_eq!(l.leaves(), vec![1]);
        assert!(matches!(l.root, PaneNode::Pane { id: 1 }));

        assert_eq!(l.close(1), CloseOutcome::LastPane);
        assert_eq!(l.close(999), CloseOutcome::NotFound);
    }

    #[test]
    fn close_keeps_active_valid_when_a_non_active_subtree_is_removed() {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitAxis::Horizontal); // 1 | 2, active 2
        l.split(101, 3, SplitAxis::Horizontal); // 1 | (2 | 3), active 3
        l.set_active(1);
        assert_eq!(l.close(3), CloseOutcome::Closed { new_active: 1 });
        assert_eq!(l.active, 1);
        assert_eq!(l.leaves(), vec![1, 2]);
    }

    #[test]
    fn ratio_changes_are_clamped() {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitAxis::Vertical);

        assert!(l.set_ratio(100, 0.72));
        assert!(l.set_ratio(100, -5.0));
        match &l.root {
            PaneNode::Split { ratio, .. } => assert!((*ratio - MIN_RATIO).abs() < 1e-6),
            _ => panic!(),
        }
        assert!(l.set_ratio(100, 5.0));
        match &l.root {
            PaneNode::Split { ratio, .. } => assert!((*ratio - (1.0 - MIN_RATIO)).abs() < 1e-6),
            _ => panic!(),
        }
        l.reset_ratio(100);
        match &l.root {
            PaneNode::Split { ratio, .. } => assert!((*ratio - 0.5).abs() < 1e-6),
            _ => panic!(),
        }
        assert!(!l.set_ratio(999, 0.5));
    }

    #[test]
    fn deep_nesting_stays_consistent() {
        let mut l = WorkspaceLayout::new(1);
        // Build 8 panes.
        for i in 2..=8 {
            let axis = if i % 2 == 0 {
                SplitAxis::Horizontal
            } else {
                SplitAxis::Vertical
            };
            l.split(1000 + i, i, axis);
        }
        assert_eq!(l.len(), 8);
        let mut leaves = l.leaves();
        leaves.sort_unstable();
        assert_eq!(leaves, (1..=8).collect::<Vec<_>>());

        // Close them back down to one; tree never goes empty.
        for id in [2u64, 4, 6, 8, 3, 5, 7] {
            l.close(id);
            assert!(!l.leaves().is_empty());
            assert!(l.root.contains(l.active));
        }
        assert_eq!(l.leaves(), vec![1]);
    }

    #[test]
    fn layout_round_trips_through_json() {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitAxis::Horizontal);
        l.split(101, 3, SplitAxis::Vertical);
        l.set_ratio(100, 0.4);

        let json = serde_json::to_string(&l).unwrap();
        let back: WorkspaceLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(l, back);
        assert!(json.contains("\"type\":\"split\""));
        assert!(json.contains("\"axis\":\"horizontal\""));
    }
}
