//! Editor view groups (R10-002, ADR 0005).
//!
//! A *group* is one independent view (selection, scroll, folds) onto the
//! single document an editor owns. The Workspace places each group in its own
//! pane, so this module carries no layout: there is no split tree, only the
//! ordered set of groups and which one is focused. It is UI-free.

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupError {
    CannotCloseLastGroup,
    GroupNotFound(EditorGroupId),
}

impl fmt::Display for GroupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CannotCloseLastGroup => f.write_str("the last editor group cannot be closed"),
            Self::GroupNotFound(id) => write!(f, "editor group {} not found", id.value()),
        }
    }
}

impl std::error::Error for GroupError {}

/// The ordered groups of one editor and the focused one. Never empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorGroups {
    order: Vec<EditorGroupId>,
    focused: EditorGroupId,
    next: u64,
}

impl Default for EditorGroups {
    fn default() -> Self {
        Self::new(EditorGroupId::new(1))
    }
}

impl EditorGroups {
    pub fn new(first: EditorGroupId) -> Self {
        Self {
            order: vec![first],
            focused: first,
            next: first.value() + 1,
        }
    }

    pub fn groups(&self) -> Vec<EditorGroupId> {
        self.order.clone()
    }

    pub fn group_count(&self) -> usize {
        self.order.len()
    }

    pub fn contains(&self, id: EditorGroupId) -> bool {
        self.order.contains(&id)
    }

    pub fn focused_group(&self) -> EditorGroupId {
        self.focused
    }

    /// Focus `id`; `GroupNotFound` leaves the focus unchanged.
    pub fn focus(&mut self, id: EditorGroupId) -> Result<(), GroupError> {
        if !self.contains(id) {
            return Err(GroupError::GroupNotFound(id));
        }
        self.focused = id;
        Ok(())
    }

    /// Add a group right after the focused one and focus it.
    pub fn add(&mut self) -> EditorGroupId {
        let id = EditorGroupId::new(self.next);
        self.next += 1;
        let at = self
            .order
            .iter()
            .position(|g| *g == self.focused)
            .map_or(self.order.len(), |i| i + 1);
        self.order.insert(at, id);
        self.focused = id;
        id
    }

    /// Remove `id`. When it was focused, focus moves to the group before it
    /// (or the new first one). The last group cannot be removed.
    pub fn remove(&mut self, id: EditorGroupId) -> Result<(), GroupError> {
        let Some(at) = self.order.iter().position(|g| *g == id) else {
            return Err(GroupError::GroupNotFound(id));
        };
        if self.order.len() == 1 {
            return Err(GroupError::CannotCloseLastGroup);
        }
        self.order.remove(at);
        if self.focused == id {
            self.focused = self.order[at.saturating_sub(1).min(self.order.len() - 1)];
        }
        Ok(())
    }

    /// Replace the groups wholesale (session restore). Falls back to a single
    /// group `1` when `ids` is empty or `focused` is unknown.
    pub fn restore(ids: &[EditorGroupId], focused: EditorGroupId) -> Self {
        let mut order: Vec<EditorGroupId> = Vec::new();
        for id in ids {
            if !order.contains(id) {
                order.push(*id);
            }
        }
        if order.is_empty() {
            return Self::default();
        }
        let focused = if order.contains(&focused) {
            focused
        } else {
            order[0]
        };
        let next = order.iter().map(|g| g.value()).max().unwrap_or(0) + 1;
        Self {
            order,
            focused,
            next,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_focuses_the_new_group_after_the_focused_one() {
        let mut groups = EditorGroups::default();
        let second = groups.add();
        assert_eq!(groups.focused_group(), second);
        assert_eq!(groups.group_count(), 2);
        let third = groups.add();
        assert_eq!(groups.groups(), vec![EditorGroupId::new(1), second, third]);
    }

    #[test]
    fn remove_refuses_the_last_group_and_moves_focus() {
        let mut groups = EditorGroups::default();
        let first = groups.focused_group();
        assert_eq!(groups.remove(first), Err(GroupError::CannotCloseLastGroup));
        let second = groups.add();
        groups.remove(second).expect("remove focused");
        assert_eq!(groups.focused_group(), first);
        assert_eq!(
            groups.remove(second),
            Err(GroupError::GroupNotFound(second))
        );
    }

    #[test]
    fn restore_repairs_bad_input() {
        let bad = EditorGroups::restore(&[], EditorGroupId::new(9));
        assert_eq!(bad, EditorGroups::default());
        let ids = [
            EditorGroupId::new(4),
            EditorGroupId::new(4),
            EditorGroupId::new(7),
        ];
        let mut restored = EditorGroups::restore(&ids, EditorGroupId::new(99));
        assert_eq!(restored.group_count(), 2);
        assert_eq!(restored.focused_group(), EditorGroupId::new(4));
        assert_eq!(restored.add(), EditorGroupId::new(8));
    }
}
