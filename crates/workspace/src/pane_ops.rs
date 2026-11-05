//! Tree operations for the universal pane model (R10-002, ADR 0005).
//!
//! [`crate::pane_group`] already stores a leaf as a bare [`PaneId`], so any
//! pane kind fits the existing tree; what the UI additionally needs is the
//! operation set below. It is added as extra inherent `impl` blocks on
//! [`PaneGroup`] / [`WorkspaceLayout`]. The terminal-only API in `pane_group.rs`
//! (`split`, `close`, `remove`, ...) is untouched.
//!
//! Invariants every operation here preserves (and `validate` checks):
//!
//! * every leaf id and every axis id occurs exactly once (one id namespace);
//! * an axis has at least two members, `flexes.len() == members.len()`, all
//!   flexes are finite and positive and sum to `1.0`;
//! * after [`PaneGroup::normalize`] no axis has a direct child axis running the
//!   same way (that child is spliced into the parent, preserving geometry).
//!
//! Equalisation policy: whenever a pane is *inserted* next to an existing
//! sibling on the same axis, the axis ends up with three or more members and
//! all of its flexes are reset to `1/n` (smart tiling). Removing a pane keeps
//! the survivors' ratios (they absorb the freed space proportionally).
//! `normalize` itself never equalises an axis it did not have to merge, so a
//! user's manual resize survives unrelated operations.

use crate::pane::{CloseOutcome, PaneId, MIN_RATIO};
use crate::pane_group::{
    Member, PaneAxis, PaneGroup, RemoveOutcome, SplitDirection, WorkspaceLayout,
};

/// Where a dragged pane lands relative to a target pane. The four edge zones
/// split the target (the dragged pane takes that side); [`DropZone::Centre`]
/// swaps the two panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
    Centre,
}

impl DropZone {
    /// The split direction of an edge zone; `None` for [`DropZone::Centre`].
    pub fn direction(self) -> Option<SplitDirection> {
        match self {
            DropZone::Left => Some(SplitDirection::Left),
            DropZone::Right => Some(SplitDirection::Right),
            DropZone::Top => Some(SplitDirection::Up),
            DropZone::Bottom => Some(SplitDirection::Down),
            DropZone::Centre => None,
        }
    }
}

/// A violated tree invariant, reported by `validate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeError {
    /// An id (leaf or axis) occurs more than once.
    DuplicateId(PaneId),
    /// An axis has fewer than two members.
    DegenerateAxis(PaneId),
    /// `flexes.len() != members.len()`.
    FlexLenMismatch(PaneId),
    /// A flex is not finite, or not positive.
    BadFlex(PaneId),
    /// The flexes of an axis do not sum to 1.0.
    FlexSum(PaneId),
    /// The layout's active pane is not a leaf (checked on layouts only).
    ActiveMissing,
    /// An axis directly contains an axis along the same direction
    /// (only reported by `validate_normalized`).
    NestedSameAxis(PaneId),
}

const FLEX_EPS: f32 = 1e-4;

fn even(n: usize) -> Vec<f32> {
    if n == 0 {
        Vec::new()
    } else {
        vec![1.0 / n as f32; n]
    }
}

/// Smallest flex an axis of `n` members may hold: `MIN_RATIO`, relaxed for
/// crowded axes so the floor is always satisfiable.
pub fn flex_floor(n: usize) -> f32 {
    if n == 0 {
        MIN_RATIO
    } else {
        MIN_RATIO.min(0.5 / n as f32)
    }
}

/// Make `flexes` a well-formed distribution for `n` members: wrong length or
/// non-finite / non-positive values fall back to an even split, otherwise the
/// values are rescaled to sum to exactly 1.0 and raised to [`flex_floor`].
fn sanitize_flexes(flexes: &mut Vec<f32>, n: usize) {
    let ok = flexes.len() == n && flexes.iter().all(|f| f.is_finite() && *f > 0.0);
    if !ok {
        *flexes = even(n);
        return;
    }
    let sum: f32 = flexes.iter().sum();
    if !(sum.is_finite() && sum > 0.0) {
        *flexes = even(n);
        return;
    }
    for f in flexes.iter_mut() {
        *f /= sum;
    }
    // Raise sub-floor members to the floor, paying for it proportionally out
    // of the members that have surplus above the floor.
    let floor = flex_floor(n);
    let deficit: f32 = flexes.iter().map(|f| (floor - *f).max(0.0)).sum();
    if deficit > 0.0 {
        let surplus: f32 = flexes.iter().map(|f| (*f - floor).max(0.0)).sum();
        if surplus > 0.0 {
            for f in flexes.iter_mut() {
                if *f < floor {
                    *f = floor;
                } else {
                    *f -= (*f - floor) * deficit / surplus;
                }
            }
        }
        let sum: f32 = flexes.iter().sum();
        for f in flexes.iter_mut() {
            *f /= sum;
        }
    }
}

fn pair(old: Member, new: Member, dir: SplitDirection) -> Vec<Member> {
    if dir.increasing() {
        vec![old, new]
    } else {
        vec![new, old]
    }
}

/// Insert `new` next to the leaf `target` on the `dir` edge. A same-axis
/// parent gains a sibling (and is equalised); otherwise the leaf is replaced
/// by a fresh two-member axis taking over its slot.
fn insert_beside(
    m: &mut Member,
    target: PaneId,
    new: &mut Option<Member>,
    dir: SplitDirection,
    alloc: &mut dyn FnMut() -> PaneId,
) -> bool {
    match m {
        Member::Pane(id) => {
            if *id != target {
                return false;
            }
            let Some(nm) = new.take() else { return false };
            let members = pair(Member::Pane(target), nm, dir);
            *m = Member::Axis(PaneAxis::new(alloc(), dir.axis(), members));
            true
        }
        Member::Axis(ax) => {
            for i in 0..ax.members.len() {
                match &ax.members[i] {
                    Member::Pane(id) if *id == target => {
                        let Some(nm) = new.take() else { return false };
                        if dir.axis() == ax.axis {
                            let ix = if dir.increasing() { i + 1 } else { i };
                            ax.members.insert(ix, nm);
                            ax.flexes = even(ax.members.len());
                        } else {
                            let members = pair(Member::Pane(target), nm, dir);
                            ax.members[i] =
                                Member::Axis(PaneAxis::new(alloc(), dir.axis(), members));
                        }
                        return true;
                    }
                    Member::Pane(_) => {}
                    Member::Axis(_) => {
                        if insert_beside(&mut ax.members[i], target, new, dir, alloc) {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }
}

enum Step {
    NotFound,
    /// Removed; `neighbor` is a leaf adjacent to the removed one.
    Done(PaneId),
    /// The axis collapsed to one member, which the parent must promote.
    Collapse(Member, PaneId),
}

/// Remove the leaf `pane` from an axis subtree, keeping the survivors'
/// ratios.
fn remove_leaf(ax: &mut PaneAxis, pane: PaneId) -> Step {
    if ax.flexes.len() != ax.members.len() {
        ax.flexes = even(ax.members.len());
    }
    for i in 0..ax.members.len() {
        match &mut ax.members[i] {
            Member::Pane(id) if *id == pane => {
                let neighbor = if i > 0 {
                    ax.members[i - 1].last_leaf()
                } else {
                    // `ax` has >= 2 members by invariant; guard anyway.
                    match ax.members.get(1) {
                        Some(next) => next.first_leaf(),
                        None => pane,
                    }
                };
                ax.members.remove(i);
                ax.flexes.remove(i);
                if ax.members.len() == 1 {
                    if let Some(only) = ax.members.pop() {
                        return Step::Collapse(only, neighbor);
                    }
                }
                let rest: f32 = ax.flexes.iter().sum();
                if rest > 1e-6 {
                    for f in ax.flexes.iter_mut() {
                        *f /= rest;
                    }
                } else {
                    ax.flexes = even(ax.members.len());
                }
                return Step::Done(neighbor);
            }
            Member::Pane(_) => {}
            Member::Axis(child) => match remove_leaf(child, pane) {
                Step::NotFound => {}
                Step::Done(n) => return Step::Done(n),
                Step::Collapse(promoted, n) => {
                    ax.members[i] = promoted;
                    return Step::Done(n);
                }
            },
        }
    }
    Step::NotFound
}

/// Recursive normalisation, see the module docs. Returns whether `m` is still
/// non-empty (an axis whose members all vanished reports `false`).
fn normalize_member(m: &mut Member) -> bool {
    let Member::Axis(ax) = m else { return true };

    // Children first; drop any that came out empty.
    let old_flexes = std::mem::take(&mut ax.flexes);
    let old_members = std::mem::take(&mut ax.members);
    let mut flexes = if old_flexes.len() == old_members.len() {
        old_flexes
    } else {
        even(old_members.len())
    };
    let mut members = Vec::with_capacity(old_members.len());
    let mut kept_flexes = Vec::with_capacity(old_members.len());
    for (mut child, f) in old_members.into_iter().zip(flexes.drain(..)) {
        if normalize_member(&mut child) {
            members.push(child);
            // Non-finite / non-positive flexes are repaired below.
            kept_flexes.push(f);
        }
    }
    let n = members.len();
    sanitize_flexes(&mut kept_flexes, n);

    // Splice same-axis children into this axis (geometry preserving).
    let mut merged = false;
    let mut out_members = Vec::with_capacity(n);
    let mut out_flexes = Vec::with_capacity(n);
    for (child, f) in members.into_iter().zip(kept_flexes) {
        match child {
            Member::Axis(c) if c.axis == ax.axis => {
                merged = true;
                for (cm, cf) in c.members.into_iter().zip(c.flexes) {
                    out_members.push(cm);
                    out_flexes.push(cf * f);
                }
            }
            other => {
                out_members.push(other);
                out_flexes.push(f);
            }
        }
    }

    match out_members.len() {
        0 => return false,
        1 => {
            // A one-member axis is just its member.
            if let Some(only) = out_members.pop() {
                *m = only;
            }
            return true;
        }
        _ => {}
    }
    let n = out_members.len();
    if merged && n >= 3 {
        out_flexes = even(n);
    } else {
        sanitize_flexes(&mut out_flexes, n);
    }
    ax.members = out_members;
    ax.flexes = out_flexes;
    true
}

fn swap_ids(m: &mut Member, a: PaneId, b: PaneId) {
    match m {
        Member::Pane(id) => {
            if *id == a {
                *id = b;
            } else if *id == b {
                *id = a;
            }
        }
        Member::Axis(ax) => {
            for c in &mut ax.members {
                swap_ids(c, a, b);
            }
        }
    }
}

fn collect_ids(m: &Member, out: &mut Vec<PaneId>) {
    match m {
        Member::Pane(id) => out.push(*id),
        Member::Axis(ax) => {
            out.push(ax.id);
            for c in &ax.members {
                collect_ids(c, out);
            }
        }
    }
}

fn validate_member(m: &Member, flattened: bool) -> Result<(), TreeError> {
    let Member::Axis(ax) = m else { return Ok(()) };
    if ax.members.len() < 2 {
        return Err(TreeError::DegenerateAxis(ax.id));
    }
    if ax.flexes.len() != ax.members.len() {
        return Err(TreeError::FlexLenMismatch(ax.id));
    }
    if ax.flexes.iter().any(|f| !f.is_finite() || *f <= 0.0) {
        return Err(TreeError::BadFlex(ax.id));
    }
    if (ax.flexes.iter().sum::<f32>() - 1.0).abs() > FLEX_EPS {
        return Err(TreeError::FlexSum(ax.id));
    }
    for c in &ax.members {
        if flattened {
            if let Member::Axis(child) = c {
                if child.axis == ax.axis {
                    return Err(TreeError::NestedSameAxis(ax.id));
                }
            }
        }
        validate_member(c, flattened)?;
    }
    Ok(())
}

impl PaneGroup {
    /// All ids in the tree: leaf ids and axis ids (one shared namespace).
    pub fn all_ids(&self) -> Vec<PaneId> {
        let mut v = Vec::new();
        if let Some(root) = &self.root {
            collect_ids(root, &mut v);
        }
        v
    }

    /// Check the structural and flex invariants (see module docs).
    pub fn validate(&self) -> Result<(), TreeError> {
        self.validate_inner(false)
    }

    /// [`PaneGroup::validate`] plus "no same-axis nesting".
    pub fn validate_normalized(&self) -> Result<(), TreeError> {
        self.validate_inner(true)
    }

    fn validate_inner(&self, flattened: bool) -> Result<(), TreeError> {
        let mut ids = self.all_ids();
        ids.sort_unstable();
        if let Some(w) = ids.windows(2).find(|w| w[0] == w[1]) {
            return Err(TreeError::DuplicateId(w[0]));
        }
        match &self.root {
            Some(root) => validate_member(root, flattened),
            None => Ok(()),
        }
    }

    /// Flatten same-axis nesting, promote one-member axes, repair flexes and
    /// enforce the flex floor. Idempotent.
    pub fn normalize(&mut self) {
        if let Some(root) = &mut self.root {
            if !normalize_member(root) {
                self.root = None;
            }
        }
    }

    /// Split the leaf `target`, inserting the *existing* member `new` (a leaf
    /// or a whole subtree) on the `dir` edge. Strict: returns `false` (tree
    /// unchanged) when `target` is absent or the new member's ids collide
    /// with the tree. Normalises on success. `alloc` mints axis ids when a
    /// fresh axis is needed.
    pub fn split_member(
        &mut self,
        target: PaneId,
        new: Member,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        let mut incoming = Vec::new();
        collect_ids(&new, &mut incoming);
        let existing = self.all_ids();
        if incoming.iter().any(|id| existing.contains(id)) || !self.find_pane(target) {
            return false;
        }
        let mut slot = Some(new);
        let ok = match &mut self.root {
            Some(root) => insert_beside(root, target, &mut slot, dir, alloc),
            None => false,
        };
        if ok {
            self.normalize();
        }
        ok
    }

    /// Strict split with a brand-new leaf; see [`PaneGroup::split_member`].
    pub fn split_pane(
        &mut self,
        target: PaneId,
        new_pane: PaneId,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        self.split_member(target, Member::Pane(new_pane), dir, alloc)
    }

    /// Remove the leaf `pane`. Survivors keep their ratios; the tree is
    /// normalised (collapsed axes may expose same-axis nesting). Removing the
    /// last pane empties the tree.
    pub fn remove_pane(&mut self, pane: PaneId) -> RemoveOutcome {
        let outcome = match &mut self.root {
            None => RemoveOutcome::NotFound,
            Some(Member::Pane(id)) => {
                if *id == pane {
                    self.root = None;
                    RemoveOutcome::Removed { neighbor: None }
                } else {
                    RemoveOutcome::NotFound
                }
            }
            Some(Member::Axis(ax)) => match remove_leaf(ax, pane) {
                Step::NotFound => RemoveOutcome::NotFound,
                Step::Done(n) => RemoveOutcome::Removed { neighbor: Some(n) },
                Step::Collapse(m, n) => {
                    self.root = Some(m);
                    RemoveOutcome::Removed { neighbor: Some(n) }
                }
            },
        };
        if matches!(outcome, RemoveOutcome::Removed { .. }) {
            self.normalize();
        }
        outcome
    }

    /// Exchange two leaves in place; geometry (flexes) is unchanged.
    pub fn swap(&mut self, a: PaneId, b: PaneId) -> bool {
        if a == b || !self.find_pane(a) || !self.find_pane(b) {
            return false;
        }
        if let Some(root) = &mut self.root {
            swap_ids(root, a, b);
        }
        true
    }

    /// Merge a whole subtree (another tab's tree) into this one next to the
    /// leaf `target` on the `dir` edge, as a nested axis. `target == None`
    /// wraps the entire tree instead. An empty tree simply adopts `subtree`.
    /// Rejects (returns `false`, unchanged) an empty subtree, an unknown
    /// `target`, or any id shared between the two trees.
    pub fn insert_subtree(
        &mut self,
        target: Option<PaneId>,
        subtree: Member,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        if self.root.is_none() {
            self.root = Some(subtree);
            self.normalize();
            return true;
        }
        match target {
            Some(t) => self.split_member(t, subtree, dir, alloc),
            None => self.wrap_root(subtree, dir, alloc),
        }
    }

    /// Wrap the whole tree: the existing root and `subtree` become siblings
    /// on a new root axis running `dir.axis()`, `subtree` on the `dir` edge.
    pub fn wrap_root(
        &mut self,
        subtree: Member,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        let mut incoming = Vec::new();
        collect_ids(&subtree, &mut incoming);
        let existing = self.all_ids();
        if incoming.iter().any(|id| existing.contains(id)) {
            return false;
        }
        match self.root.take() {
            None => self.root = Some(subtree),
            Some(old) => {
                let members = pair(old, subtree, dir);
                self.root = Some(Member::Axis(PaneAxis::new(alloc(), dir.axis(), members)));
            }
        }
        self.normalize();
        true
    }
}

impl WorkspaceLayout {
    /// Check the tree invariants plus "`active` is a leaf".
    pub fn validate(&self) -> Result<(), TreeError> {
        self.group.validate()?;
        self.check_active()
    }

    /// [`WorkspaceLayout::validate`] plus "no same-axis nesting".
    pub fn validate_normalized(&self) -> Result<(), TreeError> {
        self.group.validate_normalized()?;
        self.check_active()
    }

    fn check_active(&self) -> Result<(), TreeError> {
        match self.active {
            Some(a) if !self.group.find_pane(a) => Err(TreeError::ActiveMissing),
            None if !self.group.is_empty() => Err(TreeError::ActiveMissing),
            _ => Ok(()),
        }
    }

    /// Normalise the tree and repair `active`.
    pub fn normalize(&mut self) {
        self.group.normalize();
        if !self.active.is_some_and(|a| self.group.find_pane(a)) {
            self.active = self.group.first_pane();
        }
    }

    /// Split `target` with a new pane on the `dir` edge; the new pane becomes
    /// active. Strict (see [`PaneGroup::split_pane`]); on an empty layout the
    /// new pane becomes the sole root and `target` is ignored.
    pub fn split_pane(
        &mut self,
        target: PaneId,
        new_pane: PaneId,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        if self.group.is_empty() {
            self.group.root = Some(Member::Pane(new_pane));
            self.active = Some(new_pane);
            return true;
        }
        let ok = self.group.split_pane(target, new_pane, dir, alloc);
        if ok {
            self.active = Some(new_pane);
        }
        ok
    }

    /// Close `target`: survivors keep their ratios and the tree is
    /// normalised. The last pane is not closed ([`CloseOutcome::LastPane`]);
    /// the caller closes the tab instead.
    pub fn close_pane(&mut self, target: PaneId) -> CloseOutcome {
        if !self.group.find_pane(target) {
            return CloseOutcome::NotFound;
        }
        if self.group.panes().len() == 1 {
            return CloseOutcome::LastPane;
        }
        match self.group.remove_pane(target) {
            RemoveOutcome::NotFound => CloseOutcome::NotFound,
            RemoveOutcome::Removed { neighbor } => {
                let active_valid = self.active.is_some_and(|a| self.group.find_pane(a));
                if self.active == Some(target) || !active_valid {
                    self.active = neighbor.or_else(|| self.group.first_pane());
                }
                let new_active = self
                    .active
                    .or_else(|| self.group.first_pane())
                    .unwrap_or(target);
                CloseOutcome::Closed { new_active }
            }
        }
    }

    /// Exchange two panes' positions.
    pub fn swap_panes(&mut self, a: PaneId, b: PaneId) -> bool {
        self.group.swap(a, b)
    }

    /// Move `pane` onto `target` at `zone`: an edge zone re-inserts it next
    /// to `target` (splitting `target`), [`DropZone::Centre`] swaps the two.
    /// The moved pane becomes active. Returns `false` (unchanged) if either
    /// pane is missing, they are the same, or the tree has a single pane.
    pub fn move_pane(
        &mut self,
        pane: PaneId,
        target: PaneId,
        zone: DropZone,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        if pane == target
            || !self.group.find_pane(pane)
            || !self.group.find_pane(target)
            || self.group.panes().len() < 2
        {
            return false;
        }
        let Some(dir) = zone.direction() else {
            let ok = self.group.swap(pane, target);
            if ok {
                self.active = Some(pane);
            }
            return ok;
        };
        let backup = self.group.clone();
        self.group.remove_pane(pane);
        if self.group.split_pane(target, pane, dir, alloc) {
            self.active = Some(pane);
            true
        } else {
            // Unreachable while the invariants hold; restore rather than lose a pane.
            self.group = backup;
            false
        }
    }

    /// Take `pane` out of this tree into a layout of its own (a tab of its
    /// own, "detach to tab"). `None` when `pane` is absent or the only pane.
    pub fn detach(&mut self, pane: PaneId) -> Option<WorkspaceLayout> {
        if !self.group.find_pane(pane) || self.group.panes().len() < 2 {
            return None;
        }
        match self.close_pane(pane) {
            CloseOutcome::Closed { .. } => Some(WorkspaceLayout::new(pane)),
            _ => None,
        }
    }

    /// Merge another tab's whole layout in as a nested axis next to `target`
    /// (or around the whole tree when `target` is `None`) on the `dir` edge.
    /// The incoming layout's active pane becomes active. Rejected (unchanged)
    /// on empty input, unknown target, or shared ids.
    pub fn insert_subtree(
        &mut self,
        target: Option<PaneId>,
        subtree: WorkspaceLayout,
        dir: SplitDirection,
        alloc: &mut dyn FnMut() -> PaneId,
    ) -> bool {
        let Some(root) = subtree.group.root else {
            return false;
        };
        let incoming_active = subtree.active.or_else(|| Some(root.first_leaf()));
        let ok = self.group.insert_subtree(target, root, dir, alloc);
        if ok {
            self.active = incoming_active;
        }
        ok
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    /// Tiny deterministic xorshift generator for property-style tests.
    pub struct Rng(pub u64);

    impl Rng {
        pub fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        pub fn below(&mut self, n: usize) -> usize {
            (self.next() % n.max(1) as u64) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::Rng;
    use super::*;
    use crate::pane::SplitAxis;

    struct Ids(PaneId);
    impl Ids {
        fn alloc(&mut self) -> PaneId {
            self.0 += 1;
            self.0
        }
    }

    fn axis_flexes(g: &PaneGroup) -> Vec<f32> {
        match &g.root {
            Some(Member::Axis(a)) => a.flexes.clone(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn split_pane_is_strict_and_activates_the_new_pane() {
        let mut ids = Ids(1000);
        let mut l = WorkspaceLayout::new(1);
        assert!(l.split_pane(1, 2, SplitDirection::Right, &mut || 500));
        assert!(!l.split_pane(99, 3, SplitDirection::Right, &mut || 501));
        // Duplicate leaf id is rejected.
        assert!(!l.split_pane(1, 2, SplitDirection::Down, &mut || 502));
        assert_eq!(l.leaves(), vec![1, 2]);
        assert_eq!(l.active, Some(2));
        let mut alloc = || ids.alloc();
        assert!(l.split_pane(2, 3, SplitDirection::Down, &mut alloc));
        assert_eq!(l.leaves(), vec![1, 2, 3]);
        assert_eq!(l.validate_normalized(), Ok(()));
    }

    #[test]
    fn same_axis_inserts_equalise_and_never_nest() {
        let mut ids = Ids(1000);
        let mut l = WorkspaceLayout::new(1);
        l.split_pane(1, 2, SplitDirection::Right, &mut || ids.alloc());
        l.group.set_boundary(1001, 0, 0.8);
        l.split_pane(2, 3, SplitDirection::Right, &mut || ids.alloc());
        let f = axis_flexes(&l.group);
        assert_eq!(f.len(), 3);
        assert!(f.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-5), "{f:?}");
        assert!(matches!(&l.group.root, Some(Member::Axis(a)) if a.members.len() == 3));
    }

    #[test]
    fn close_keeps_survivor_ratios_and_flattens() {
        let mut ids = Ids(1000);
        let mut l = WorkspaceLayout::new(1);
        l.split_pane(1, 2, SplitDirection::Right, &mut || ids.alloc());
        l.split_pane(2, 3, SplitDirection::Down, &mut || ids.alloc());
        // H[1, V[2, 3]]; split 3 to the right -> V[2, H[3, 4]]
        l.split_pane(3, 4, SplitDirection::Right, &mut || ids.alloc());
        assert_eq!(l.validate_normalized(), Ok(()));
        // Closing 2 promotes H[3, 4] into the root H axis => H[1, 3, 4].
        assert_eq!(l.close_pane(2), CloseOutcome::Closed { new_active: 4 });
        assert_eq!(l.leaves(), vec![1, 3, 4]);
        assert_eq!(l.validate_normalized(), Ok(()));
        assert!(matches!(&l.group.root, Some(Member::Axis(a)) if a.members.len() == 3));
        assert_eq!(l.close_pane(42), CloseOutcome::NotFound);
    }

    #[test]
    fn removal_redistributes_proportionally() {
        let mut g = PaneGroup::new(1);
        let mut n = 100;
        let mut alloc = || {
            n += 1;
            n
        };
        g.split_pane(1, 2, SplitDirection::Right, &mut alloc);
        g.split_pane(2, 3, SplitDirection::Right, &mut alloc);
        g.set_boundary(101, 0, 0.5);
        g.set_boundary(101, 1, 0.75);
        // flexes ~ [0.5, 0.25, 0.25]; remove the middle one -> [2/3, 1/3].
        assert!(matches!(g.remove_pane(2), RemoveOutcome::Removed { .. }));
        let f = axis_flexes(&g);
        assert!(
            (f[0] - 2.0 / 3.0).abs() < 1e-4 && (f[1] - 1.0 / 3.0).abs() < 1e-4,
            "{f:?}"
        );
    }

    #[test]
    fn swap_and_move_to_zones() {
        let mut ids = Ids(1000);
        let mut l = WorkspaceLayout::new(1);
        l.split_pane(1, 2, SplitDirection::Right, &mut || ids.alloc());
        l.split_pane(2, 3, SplitDirection::Right, &mut || ids.alloc());
        assert_eq!(l.leaves(), vec![1, 2, 3]);

        assert!(l.swap_panes(1, 3));
        assert_eq!(l.leaves(), vec![3, 2, 1]);
        assert!(!l.swap_panes(1, 1));
        assert!(!l.swap_panes(1, 77));

        // Centre = swap.
        assert!(l.move_pane(3, 1, DropZone::Centre, &mut || ids.alloc()));
        assert_eq!(l.leaves(), vec![1, 2, 3]);
        assert_eq!(l.active, Some(3));

        // Edge: drop 3 below 1 => V[1, 3] nested in the row.
        assert!(l.move_pane(3, 1, DropZone::Bottom, &mut || ids.alloc()));
        assert_eq!(l.leaves(), vec![1, 3, 2]);
        assert_eq!(l.validate_normalized(), Ok(()));
        assert!(matches!(
            &l.group.root,
            Some(Member::Axis(a)) if a.axis == SplitAxis::Horizontal
                && matches!(&a.members[0], Member::Axis(c) if c.axis == SplitAxis::Vertical)
        ));

        assert!(!l.move_pane(1, 1, DropZone::Left, &mut || ids.alloc()));
        assert!(!l.move_pane(1, 99, DropZone::Left, &mut || ids.alloc()));
        let mut single = WorkspaceLayout::new(5);
        assert!(!single.move_pane(5, 6, DropZone::Left, &mut || 0));
    }

    #[test]
    fn detach_leaves_a_valid_remainder() {
        let mut ids = Ids(1000);
        let mut l = WorkspaceLayout::new(1);
        l.split_pane(1, 2, SplitDirection::Right, &mut || ids.alloc());
        l.split_pane(2, 3, SplitDirection::Down, &mut || ids.alloc());
        let own = l.detach(3).expect("detaches");
        assert_eq!(own.leaves(), vec![3]);
        assert_eq!(own.active, Some(3));
        assert_eq!(l.leaves(), vec![1, 2]);
        assert_eq!(l.validate_normalized(), Ok(()));
        assert!(l.detach(99).is_none());
        let mut one = WorkspaceLayout::new(9);
        assert!(one.detach(9).is_none());
    }

    #[test]
    fn insert_subtree_nests_another_tabs_tree() {
        let mut ids = Ids(1000);
        let mut a = WorkspaceLayout::new(1);
        a.split_pane(1, 2, SplitDirection::Right, &mut || ids.alloc());
        let mut b = WorkspaceLayout::new(11);
        b.split_pane(11, 12, SplitDirection::Right, &mut || ids.alloc());
        b.split_pane(12, 13, SplitDirection::Down, &mut || ids.alloc());
        let b_active = b.active;

        // Next to leaf 2, on its bottom edge: V[2, <b's tree>].
        assert!(
            a.insert_subtree(Some(2), b.clone(), SplitDirection::Down, &mut || ids
                .alloc())
        );
        assert_eq!(a.leaves(), vec![1, 2, 11, 12, 13]);
        assert_eq!(a.active, b_active);
        assert_eq!(a.validate_normalized(), Ok(()));

        // Shared ids are rejected and leave the tree unchanged.
        let before = a.clone();
        assert!(!a.insert_subtree(Some(1), b, SplitDirection::Left, &mut || ids.alloc()));
        assert_eq!(a, before);
        // Empty subtree and unknown target are rejected too.
        assert!(!a.insert_subtree(
            Some(1),
            WorkspaceLayout::empty(),
            SplitDirection::Left,
            &mut || 0
        ));
        let c = WorkspaceLayout::new(21);
        assert!(
            !a.insert_subtree(Some(999), c.clone(), SplitDirection::Left, &mut || ids
                .alloc())
        );
        // None wraps the whole tree.
        assert!(a.insert_subtree(None, c, SplitDirection::Up, &mut || ids.alloc()));
        assert_eq!(a.leaves()[0], 21);
        assert_eq!(a.validate_normalized(), Ok(()));
    }

    #[test]
    fn normalize_flattens_preserving_geometry_and_is_idempotent() {
        // H[a, H[b, c]] with flexes [0.5, 0.5] / [0.5, 0.5] (hand-built).
        let inner = PaneAxis::new(
            11,
            SplitAxis::Horizontal,
            vec![Member::Pane(2), Member::Pane(3)],
        );
        let outer = PaneAxis {
            id: 10,
            axis: SplitAxis::Horizontal,
            members: vec![Member::Pane(1), Member::Axis(inner)],
            flexes: vec![0.5, 0.5],
        };
        let mut g = PaneGroup {
            root: Some(Member::Axis(outer)),
        };
        assert_eq!(g.validate_normalized(), Err(TreeError::NestedSameAxis(10)));
        g.normalize();
        assert_eq!(g.validate_normalized(), Ok(()));
        // Merged to three siblings => equalised.
        assert_eq!(g.panes(), vec![1, 2, 3]);
        let f = axis_flexes(&g);
        assert!(f.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-5));
        let snapshot = g.clone();
        g.normalize();
        assert_eq!(g, snapshot);
    }

    #[test]
    fn normalize_repairs_bad_flexes_and_degenerate_axes() {
        let bad = PaneAxis {
            id: 10,
            axis: SplitAxis::Vertical,
            members: vec![Member::Pane(1), Member::Pane(2)],
            flexes: vec![f32::NAN, 3.0],
        };
        let single = PaneAxis {
            id: 20,
            axis: SplitAxis::Horizontal,
            members: vec![Member::Axis(bad)],
            flexes: vec![1.0],
        };
        let mut g = PaneGroup {
            root: Some(Member::Axis(single)),
        };
        g.normalize();
        assert_eq!(g.validate_normalized(), Ok(()));
        assert!(matches!(&g.root, Some(Member::Axis(a)) if a.id == 10));

        let mut empty = PaneGroup {
            root: Some(Member::Axis(PaneAxis {
                id: 1,
                axis: SplitAxis::Vertical,
                members: Vec::new(),
                flexes: Vec::new(),
            })),
        };
        empty.normalize();
        assert!(empty.is_empty());
    }

    #[test]
    fn flex_floor_is_enforced() {
        let mut f = vec![0.001, 0.499, 0.5];
        sanitize_flexes(&mut f, 3);
        assert!(f.iter().all(|x| *x >= flex_floor(3) - 1e-6), "{f:?}");
        assert!((f.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    }

    /// Random split / close / move / swap / detach / insert sequences keep the
    /// tree valid, unique and normalised.
    #[test]
    fn random_operations_preserve_tree_invariants() {
        for seed in 1..=60u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut next_id: PaneId = 1;
            let fresh = |n: &mut PaneId| {
                *n += 1;
                *n
            };
            let mut layout = WorkspaceLayout::new(fresh(&mut next_id));
            for step in 0..80 {
                let leaves = layout.leaves();
                let pick = |rng: &mut Rng| leaves[rng.below(leaves.len())];
                let dir = [
                    SplitDirection::Up,
                    SplitDirection::Down,
                    SplitDirection::Left,
                    SplitDirection::Right,
                ][rng.below(4)];
                let mut counter = next_id + 10_000;
                match rng.below(7) {
                    0 | 1 => {
                        let t = pick(&mut rng);
                        let new = fresh(&mut next_id);
                        assert!(layout.split_pane(t, new, dir, &mut || {
                            counter += 1;
                            counter
                        }));
                    }
                    2 => {
                        let t = pick(&mut rng);
                        let _ = layout.close_pane(t);
                    }
                    3 => {
                        let (a, b) = (pick(&mut rng), pick(&mut rng));
                        let zone = [
                            DropZone::Left,
                            DropZone::Right,
                            DropZone::Top,
                            DropZone::Bottom,
                            DropZone::Centre,
                        ][rng.below(5)];
                        let _ = layout.move_pane(a, b, zone, &mut || {
                            counter += 1;
                            counter
                        });
                    }
                    4 => {
                        let (a, b) = (pick(&mut rng), pick(&mut rng));
                        let _ = layout.swap_panes(a, b);
                    }
                    5 => {
                        let t = pick(&mut rng);
                        let before = layout.leaves().len();
                        if let Some(own) = layout.detach(t) {
                            assert_eq!(layout.leaves().len(), before - 1);
                            // Re-attach somewhere else half of the time.
                            if rng.below(2) == 0 {
                                let target = pick_other(&layout, &mut rng);
                                assert!(layout.insert_subtree(target, own, dir, &mut || {
                                    counter += 1;
                                    counter
                                }));
                                assert_eq!(layout.leaves().len(), before);
                            }
                        }
                    }
                    _ => {
                        let a = fresh(&mut next_id);
                        let mut sub = WorkspaceLayout::new(a);
                        let t = pick(&mut rng);
                        let b = fresh(&mut next_id);
                        sub.split_pane(a, b, dir, &mut || {
                            counter += 1;
                            counter
                        });
                        assert!(layout.insert_subtree(Some(t), sub, dir, &mut || {
                            counter += 1;
                            counter
                        }));
                    }
                }
                // Axis ids minted from the throwaway counter may collide
                // across steps; re-seat them so ids stay globally unique.
                reassign_axis_ids(&mut layout, &mut next_id);
                assert_eq!(
                    layout.validate_normalized(),
                    Ok(()),
                    "seed {seed} step {step}"
                );
                let mut leaves = layout.leaves();
                let n = leaves.len();
                leaves.sort_unstable();
                leaves.dedup();
                assert_eq!(leaves.len(), n, "seed {seed} step {step}: duplicate leaf");
                assert!(n >= 1);
            }
        }
    }

    fn pick_other(layout: &WorkspaceLayout, rng: &mut Rng) -> Option<PaneId> {
        let l = layout.leaves();
        if l.is_empty() || rng.below(4) == 0 {
            None
        } else {
            Some(l[rng.below(l.len())])
        }
    }

    /// Give every axis a fresh id from `next` (leaf ids are untouched).
    fn reassign_axis_ids(layout: &mut WorkspaceLayout, next: &mut PaneId) {
        fn walk(m: &mut Member, next: &mut PaneId) {
            if let Member::Axis(ax) = m {
                *next += 1;
                ax.id = *next;
                for c in &mut ax.members {
                    walk(c, next);
                }
            }
        }
        // Axis ids must not collide with leaf ids either; leaf ids are all
        // <= the allocator before this call, so bump well past them.
        *next += 100_000;
        if let Some(root) = &mut layout.group.root {
            walk(root, next);
        }
    }
}
