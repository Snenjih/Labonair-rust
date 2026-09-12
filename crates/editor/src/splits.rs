//! Editor-owned groups and split layout.
//!
//! This is deliberately UI-free. Workspace owns the outer capability pane,
//! while this tree only describes views inside one Editor surface.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EditorGroupId(u64);

impl EditorGroupId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SplitOrientation {
    /// Two groups laid out left-to-right.
    Horizontal,
    /// Two groups laid out top-to-bottom.
    Vertical,
}

/// A stable location of a split node inside the editor-owned tree.
///
/// Paths are preferable to group IDs for resize gestures: a leaf ID identifies
/// a group, but a nested editor surface can contain several independent
/// dividers on the route to that group. The path is intentionally not
/// persisted; it is rebuilt from the current tree for each rendered frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SplitSide {
    First,
    Second,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct SplitPath(Vec<SplitSide>);

impl SplitPath {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn child(&self, side: SplitSide) -> Self {
        let mut path = self.clone();
        path.0.push(side);
        path
    }

    /// Stable frame-local key suitable for a UI element ID. A string avoids
    /// collisions when a deeply nested tree exceeds integer bit capacity.
    pub fn key(&self) -> String {
        self.0
            .iter()
            .map(|side| match side {
                SplitSide::First => 'f',
                SplitSide::Second => 's',
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SplitHandle {
    pub path: SplitPath,
    pub orientation: SplitOrientation,
    pub first_size: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SplitNode {
    Leaf(EditorGroupId),
    Split {
        orientation: SplitOrientation,
        /// Fraction occupied by the first child, clamped to 10%..90%.
        first_size: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorSplitTree {
    root: SplitNode,
    focused: EditorGroupId,
    next_id: u64,
}

impl Default for EditorSplitTree {
    fn default() -> Self {
        Self::new(EditorGroupId::new(1))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitError {
    GroupNotFound(EditorGroupId),
    CannotCloseLastGroup,
}

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GroupNotFound(id) => write!(f, "editor group {} does not exist", id.value()),
            Self::CannotCloseLastGroup => write!(f, "the last editor group cannot be closed"),
        }
    }
}

impl std::error::Error for SplitError {}

impl EditorSplitTree {
    pub fn new(initial_group: EditorGroupId) -> Self {
        Self {
            root: SplitNode::Leaf(initial_group),
            focused: initial_group,
            next_id: initial_group.value().saturating_add(1).max(1),
        }
    }

    pub fn root(&self) -> &SplitNode {
        &self.root
    }

    pub fn focused_group(&self) -> EditorGroupId {
        self.focused
    }

    pub fn groups(&self) -> Vec<EditorGroupId> {
        let mut result = Vec::new();
        collect_groups(&self.root, &mut result);
        result
    }

    pub fn group_count(&self) -> usize {
        self.groups().len()
    }

    /// Returns every divider in depth-first layout order. The paths are
    /// frame-local handles for UI input and must be resolved against this tree
    /// again before applying a resize.
    pub fn split_handles(&self) -> Vec<SplitHandle> {
        let mut handles = Vec::new();
        collect_split_handles(&self.root, &SplitPath::root(), &mut handles);
        handles
    }

    /// Validate invariants at persistence and composition boundaries.
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut groups = Vec::new();
        validate_node(&self.root, &mut groups)?;
        if groups.is_empty() {
            return Err("tree has no groups");
        }
        if groups
            .iter()
            .filter(|group| **group == self.focused)
            .count()
            != 1
        {
            return Err("focused group is missing or duplicated");
        }
        Ok(())
    }

    pub fn contains(&self, group: EditorGroupId) -> bool {
        contains_group(&self.root, group)
    }

    pub fn split_right(&mut self) -> Result<EditorGroupId, SplitError> {
        self.split(SplitOrientation::Horizontal)
    }

    pub fn split_down(&mut self) -> Result<EditorGroupId, SplitError> {
        self.split(SplitOrientation::Vertical)
    }

    pub fn split(&mut self, orientation: SplitOrientation) -> Result<EditorGroupId, SplitError> {
        let new_group = self
            .allocate_group_id()
            .ok_or(SplitError::GroupNotFound(self.focused))?;
        if !split_leaf(&mut self.root, self.focused, orientation, new_group) {
            return Err(SplitError::GroupNotFound(self.focused));
        }
        self.focused = new_group;
        Ok(new_group)
    }

    pub fn focus(&mut self, group: EditorGroupId) -> Result<(), SplitError> {
        if !self.contains(group) {
            return Err(SplitError::GroupNotFound(group));
        }
        self.focused = group;
        Ok(())
    }

    pub fn focus_next(&mut self) -> EditorGroupId {
        let groups = self.groups();
        if let Some(index) = groups.iter().position(|group| *group == self.focused) {
            self.focused = groups[(index + 1) % groups.len()];
        }
        self.focused
    }

    pub fn focus_previous(&mut self) -> EditorGroupId {
        let groups = self.groups();
        if let Some(index) = groups.iter().position(|group| *group == self.focused) {
            self.focused = groups[(index + groups.len() - 1) % groups.len()];
        }
        self.focused
    }

    pub fn close_focused(&mut self) -> Result<EditorGroupId, SplitError> {
        self.close(self.focused)
    }

    pub fn close(&mut self, group: EditorGroupId) -> Result<EditorGroupId, SplitError> {
        if self.group_count() == 1 {
            return Err(SplitError::CannotCloseLastGroup);
        }
        let replacement = sibling_of(&self.root, group).ok_or(SplitError::GroupNotFound(group))?;
        if !remove_group(&mut self.root, group) {
            return Err(SplitError::GroupNotFound(group));
        }
        if self.focused == group {
            self.focused = replacement;
        }
        Ok(self.focused)
    }

    pub fn close_other_groups(&mut self) -> Result<(), SplitError> {
        let focused = self.focused;
        if !self.contains(focused) {
            return Err(SplitError::GroupNotFound(focused));
        }
        self.root = SplitNode::Leaf(focused);
        Ok(())
    }

    /// Resize the split containing `group`. Positive values give more space
    /// to the first child; negative values give more space to the second.
    pub fn resize(&mut self, group: EditorGroupId, delta: f32) -> Result<(), SplitError> {
        if !self.contains(group) {
            return Err(SplitError::GroupNotFound(group));
        }
        resize_nearest(&mut self.root, group, delta);
        Ok(())
    }

    /// Resize exactly the divider represented by a frame-local path.
    pub fn resize_at(&mut self, path: &SplitPath, delta: f32) -> Result<(), SplitError> {
        let node =
            node_at_mut(&mut self.root, path).ok_or(SplitError::GroupNotFound(self.focused))?;
        let SplitNode::Split { first_size, .. } = node else {
            return Err(SplitError::GroupNotFound(self.focused));
        };
        if delta.is_finite() {
            *first_size = (*first_size + delta).clamp(0.1, 0.9);
        }
        Ok(())
    }

    fn allocate_group_id(&mut self) -> Option<EditorGroupId> {
        let mut candidate = self.next_id.max(1);
        loop {
            let id = EditorGroupId::new(candidate);
            if !self.contains(id) {
                self.next_id = candidate.saturating_add(1).max(1);
                return Some(id);
            }
            if candidate == u64::MAX {
                return None;
            }
            candidate += 1;
        }
    }
}

fn collect_groups(node: &SplitNode, groups: &mut Vec<EditorGroupId>) {
    match node {
        SplitNode::Leaf(group) => groups.push(*group),
        SplitNode::Split { first, second, .. } => {
            collect_groups(first, groups);
            collect_groups(second, groups);
        }
    }
}

fn contains_group(node: &SplitNode, group: EditorGroupId) -> bool {
    match node {
        SplitNode::Leaf(candidate) => *candidate == group,
        SplitNode::Split { first, second, .. } => {
            contains_group(first, group) || contains_group(second, group)
        }
    }
}

fn collect_split_handles(node: &SplitNode, path: &SplitPath, handles: &mut Vec<SplitHandle>) {
    let SplitNode::Split {
        orientation,
        first_size,
        first,
        second,
    } = node
    else {
        return;
    };
    handles.push(SplitHandle {
        path: path.clone(),
        orientation: *orientation,
        first_size: *first_size,
    });
    collect_split_handles(first, &path.child(SplitSide::First), handles);
    collect_split_handles(second, &path.child(SplitSide::Second), handles);
}

fn validate_node(node: &SplitNode, groups: &mut Vec<EditorGroupId>) -> Result<(), &'static str> {
    match node {
        SplitNode::Leaf(group) => {
            if groups.contains(group) {
                return Err("group IDs must be unique");
            }
            groups.push(*group);
        }
        SplitNode::Split {
            first_size,
            first,
            second,
            ..
        } => {
            if !first_size.is_finite() || !(0.1..=0.9).contains(first_size) {
                return Err("split sizes must be finite and clamped");
            }
            validate_node(first, groups)?;
            validate_node(second, groups)?;
        }
    }
    Ok(())
}

fn node_at_mut<'a>(node: &'a mut SplitNode, path: &SplitPath) -> Option<&'a mut SplitNode> {
    let mut current = node;
    for side in &path.0 {
        let SplitNode::Split { first, second, .. } = current else {
            return None;
        };
        current = match side {
            SplitSide::First => first,
            SplitSide::Second => second,
        };
    }
    Some(current)
}

fn split_leaf(
    node: &mut SplitNode,
    target: EditorGroupId,
    orientation: SplitOrientation,
    new_group: EditorGroupId,
) -> bool {
    match node {
        SplitNode::Leaf(group) if *group == target => {
            *node = SplitNode::Split {
                orientation,
                first_size: 0.5,
                first: Box::new(SplitNode::Leaf(target)),
                second: Box::new(SplitNode::Leaf(new_group)),
            };
            true
        }
        SplitNode::Leaf(_) => false,
        SplitNode::Split { first, second, .. } => {
            split_leaf(first, target, orientation, new_group)
                || split_leaf(second, target, orientation, new_group)
        }
    }
}

fn sibling_of(node: &SplitNode, target: EditorGroupId) -> Option<EditorGroupId> {
    match node {
        SplitNode::Leaf(_) => None,
        SplitNode::Split { first, second, .. } => {
            if contains_group(first, target) {
                sibling_of(first, target).or_else(|| first_leaf(second))
            } else if contains_group(second, target) {
                sibling_of(second, target).or_else(|| first_leaf(first))
            } else {
                sibling_of(first, target).or_else(|| sibling_of(second, target))
            }
        }
    }
}

fn first_leaf(node: &SplitNode) -> Option<EditorGroupId> {
    match node {
        SplitNode::Leaf(group) => Some(*group),
        SplitNode::Split { first, .. } => first_leaf(first),
    }
}

fn remove_group(node: &mut SplitNode, target: EditorGroupId) -> bool {
    let SplitNode::Split { first, second, .. } = node else {
        return false;
    };
    if matches!(first.as_ref(), SplitNode::Leaf(group) if *group == target) {
        *node = (**second).clone();
        return true;
    }
    if matches!(second.as_ref(), SplitNode::Leaf(group) if *group == target) {
        *node = (**first).clone();
        return true;
    }
    remove_group(first, target) || remove_group(second, target)
}

fn resize_nearest(node: &mut SplitNode, group: EditorGroupId, delta: f32) -> bool {
    let SplitNode::Split {
        first,
        second,
        first_size,
        ..
    } = node
    else {
        return false;
    };
    if matches!(first.as_ref(), SplitNode::Leaf(candidate) if *candidate == group)
        || matches!(second.as_ref(), SplitNode::Leaf(candidate) if *candidate == group)
    {
        *first_size = (*first_size + delta).clamp(0.1, 0.9);
        true
    } else {
        if contains_group(first, group) {
            resize_nearest(first, group, delta)
        } else if contains_group(second, group) {
            resize_nearest(second, group, delta)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_focus_close_and_resize_preserve_tree_invariants() {
        let root = EditorGroupId::new(1);
        let mut tree = EditorSplitTree::new(root);
        let right = tree.split_right().expect("split");
        assert_eq!(tree.groups(), [root, right]);
        assert_eq!(tree.focused_group(), right);
        tree.resize(right, 0.25).expect("resize");
        tree.focus(root).expect("focus root");
        tree.focus_next();
        assert_eq!(tree.focused_group(), right);
        tree.close_focused().expect("close right");
        assert_eq!(tree.groups(), [root]);
        assert_eq!(tree.focused_group(), root);
        assert_eq!(tree.close_focused(), Err(SplitError::CannotCloseLastGroup));
    }

    #[test]
    fn close_other_groups_collapses_nested_splits() {
        let mut tree = EditorSplitTree::new(EditorGroupId::new(1));
        let second = tree.split_down().expect("split");
        tree.focus(EditorGroupId::new(1)).expect("focus");
        tree.split_right().expect("nested split");
        tree.focus(second).expect("focus second");
        tree.close_other_groups().expect("close others");
        assert_eq!(tree.groups(), [second]);
    }

    #[test]
    fn nested_handles_traverse_each_divider_with_local_paths() {
        let root = EditorGroupId::new(1);
        let mut tree = EditorSplitTree::new(root);
        let right = tree.split_right().expect("right split");
        tree.focus(root).expect("focus root");
        let bottom = tree.split_down().expect("nested split");

        let handles = tree.split_handles();
        assert_eq!(handles.len(), 2);
        assert_eq!(handles[0].path, SplitPath::root());
        assert_eq!(handles[0].orientation, SplitOrientation::Horizontal);
        assert_eq!(handles[1].path, SplitPath::root().child(SplitSide::First));
        assert_eq!(handles[1].orientation, SplitOrientation::Vertical);

        tree.resize_at(&handles[1].path, 0.2)
            .expect("nested resize");
        assert!((tree.split_handles()[1].first_size - 0.7).abs() < f32::EPSILON);
        tree.resize(bottom, 10.0).expect("resize clamps high");
        assert!((tree.split_handles()[1].first_size - 0.9).abs() < f32::EPSILON);
        tree.resize(bottom, -10.0).expect("resize clamps low");
        assert!((tree.split_handles()[1].first_size - 0.1).abs() < f32::EPSILON);
        assert!(tree.contains(right));
        assert!(tree.contains(bottom));
        tree.validate().expect("tree remains valid");
    }

    #[test]
    fn stale_resize_paths_are_rejected_without_mutating_the_tree() {
        let mut tree = EditorSplitTree::default();
        let second = tree.split_right().expect("split");
        let path = tree.split_handles()[0].path.clone();
        tree.close(second).expect("close");
        assert_eq!(
            tree.resize_at(&path, 0.2),
            Err(SplitError::GroupNotFound(tree.focused_group()))
        );
        tree.validate().expect("single leaf remains valid");
    }

    #[test]
    fn validation_rejects_duplicate_groups_and_invalid_sizes() {
        let duplicate = EditorSplitTree {
            root: SplitNode::Split {
                orientation: SplitOrientation::Horizontal,
                first_size: 0.5,
                first: Box::new(SplitNode::Leaf(EditorGroupId::new(1))),
                second: Box::new(SplitNode::Leaf(EditorGroupId::new(1))),
            },
            focused: EditorGroupId::new(1),
            next_id: 2,
        };
        assert_eq!(duplicate.validate(), Err("group IDs must be unique"));

        let invalid_size = EditorSplitTree {
            root: SplitNode::Split {
                orientation: SplitOrientation::Vertical,
                first_size: f32::NAN,
                first: Box::new(SplitNode::Leaf(EditorGroupId::new(1))),
                second: Box::new(SplitNode::Leaf(EditorGroupId::new(2))),
            },
            focused: EditorGroupId::new(1),
            next_id: 3,
        };
        assert_eq!(
            invalid_size.validate(),
            Err("split sizes must be finite and clamped")
        );
    }
}
