//! Single-pane primitives (T04-002).
//!
//! A workspace tab hosts a **pane tree**; this module carries the leaf-level
//! primitives shared across it — the pane id, the split orientation, the
//! per-split minimum ratio and the close outcome. The recursive tree itself
//! ([`Member`] / [`PaneAxis`] / [`PaneGroup`] / [`WorkspaceLayout`]) lives in
//! [`crate::pane_group`] (n-ary Zed-style split tree, T17-004); those types are
//! re-exported here so existing `crate::pane::…` paths keep resolving.

use serde::{Deserialize, Serialize};

pub use crate::pane_group::{
    Member, PaneAxis, PaneGroup, RemoveOutcome, SplitDirection, WorkspaceLayout,
};

/// Identifier for a pane leaf or split node. Doubles as a leaf's content-slot
/// key on the view side.
pub type PaneId = u64;

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The next process-wide id. Tab ids, pane ids and axis ids all come from this
/// one counter, so a tab id can never equal a pane or axis id and per-pane
/// maps may share the id space of the tabs that created them.
pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The smallest fraction a split child may be dragged down to.
pub const MIN_RATIO: f32 = 0.1;

/// Orientation of a [`PaneAxis`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitAxis {
    /// Children sit side by side, left → right.
    Horizontal,
    /// Children stack top → bottom.
    Vertical,
}

/// Outcome of [`WorkspaceLayout::close`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    /// The pane was closed; `new_active` is the pane now focused.
    Closed { new_active: PaneId },
    /// `target` was the only pane — the whole tab should close instead.
    LastPane,
    /// `target` is not in this layout.
    NotFound,
}
