//! Session snapshot format v3: every tab is a pane group whose leaves are
//! `{kind, state}` (R10-002, ADR 0005).
//!
//! This is a self-contained, pure data module. [`crate::session`] still owns
//! the live v2 format; wave 2 switches the capture / restore wiring to the
//! types below. Nothing here touches GPUI.
//!
//! * **Wire types** — [`SessionSnapshotV3`] / [`SpacesSnapshotV3`] mirror the
//!   v2 envelopes, but a tab is a [`PaneGroupSnapshot`] (title + a
//!   [`SerializedTree`]) and a leaf is a [`SerializedPane`] (`kind` string +
//!   opaque `state` JSON). The leaf payload is deliberately untyped on the wire
//!   so an unknown kind, or a kind this build cannot read, never makes the
//!   whole file unreadable; it is simply dropped at restore.
//! * **Typed payload** — [`PaneState`] is what the workspace actually works
//!   with; [`SerializedPane::parse`] / [`PaneState::to_wire`] convert.
//!   Transient kinds (git graph, diffs, keymap, hosts, AI diff) have no
//!   `PaneState` and are not persisted, as in v2.
//! * **Migration** — [`migrate_tab`], [`migrate_session`], [`migrate_spaces`]
//!   and [`parse_any`] turn v2 data (and the older flat shape) into v3.
//! * **Restore** — [`restore_tree`] rebuilds a [`WorkspaceLayout`] with fresh
//!   pane ids, dropping unknown / invalid leaves and collapsing the tree.
//!   [`SerializedTree::capture`] is the inverse used when saving.
//!
//! Space colours are not part of v3 (the Spaces rework removes them); a v2
//! file that still carries `color` loads fine, the field is ignored.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::context::WorkspaceIdentity;
use crate::pane::{PaneId, SplitAxis, WorkspaceLayout};
use crate::pane_group::{Member, PaneAxis, PaneGroup};
use crate::pane_kind::PaneKind;
use crate::session::{
    PaneSessionKind, PaneSessionSnapshot, SerializedPaneGroup, SessionSnapshot, SpacesSnapshot,
    TabSnapshot,
};

/// Version stamp of the v3 format (v2 is [`crate::session::SNAPSHOT_VERSION`]).
pub const SNAPSHOT_VERSION: u32 = 3;

/// Deepest tree [`restore_tree`] will follow. Real layouts nest a handful of
/// levels; this only bounds corrupt or hostile input.
pub const MAX_TREE_DEPTH: usize = 64;

// ─────────────────────────────── typed state ─────────────────────────────

/// Persisted state of an editor pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorPaneState {
    pub path: String,
    /// Editor-owned view/session payload (opaque to the workspace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<labonair_editor::EditorSessionSnapshot>,
}

/// Persisted state of a preview pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPaneState {
    pub url: String,
}

/// Persisted state of an SFTP pane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpPaneState {
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// What a persisted leaf holds, per kind. The terminal payload is the v2
/// per-pane record unchanged (so scrollback ids stay addressable).
#[derive(Debug, Clone, PartialEq)]
pub enum PaneState {
    Terminal(PaneSessionSnapshot),
    Editor(EditorPaneState),
    Preview(PreviewPaneState),
    Sftp(SftpPaneState),
}

const KIND_TERMINAL: &str = "terminal";
const KIND_EDITOR: &str = "editor";
const KIND_PREVIEW: &str = "preview";
const KIND_SFTP: &str = "sftp";

impl PaneState {
    pub fn kind(&self) -> PaneKind {
        match self {
            PaneState::Terminal(_) => PaneKind::Terminal,
            PaneState::Editor(_) => PaneKind::Editor,
            PaneState::Preview(_) => PaneKind::Preview,
            PaneState::Sftp(_) => PaneKind::Sftp,
        }
    }

    /// Whether this state can be restored at all: required identifiers must be
    /// present and non-empty (a file path, a URL, a host id; an SSH terminal
    /// needs its host).
    pub fn is_valid(&self) -> bool {
        let non_empty = |s: &str| !s.trim().is_empty();
        match self {
            PaneState::Terminal(t) => {
                t.kind == PaneSessionKind::Local || t.host_id.as_deref().is_some_and(non_empty)
            }
            PaneState::Editor(e) => non_empty(&e.path),
            PaneState::Preview(p) => non_empty(&p.url),
            PaneState::Sftp(s) => non_empty(&s.host_id),
        }
    }

    /// The wire form of this state, `None` only if it cannot be encoded.
    pub fn to_wire(&self) -> Option<SerializedPane> {
        let (kind, state) = match self {
            PaneState::Terminal(t) => (KIND_TERMINAL, serde_json::to_value(t)),
            PaneState::Editor(e) => (KIND_EDITOR, serde_json::to_value(e)),
            PaneState::Preview(p) => (KIND_PREVIEW, serde_json::to_value(p)),
            PaneState::Sftp(s) => (KIND_SFTP, serde_json::to_value(s)),
        };
        Some(SerializedPane {
            kind: kind.to_string(),
            state: state.ok()?,
        })
    }
}

// ───────────────────────────────── wire ──────────────────────────────────

/// A persisted leaf: a kind tag plus that kind's state, kept untyped on the
/// wire (see the module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SerializedPane {
    pub kind: String,
    #[serde(default)]
    pub state: Value,
}

impl SerializedPane {
    /// Decode into a [`PaneState`]. `None` for an unknown kind, a payload that
    /// does not match the kind, or a state that [`PaneState::is_valid`]
    /// rejects.
    pub fn parse(&self) -> Option<PaneState> {
        let state = self.state.clone();
        let parsed = match self.kind.as_str() {
            KIND_TERMINAL => PaneState::Terminal(serde_json::from_value(state).ok()?),
            KIND_EDITOR => PaneState::Editor(serde_json::from_value(state).ok()?),
            KIND_PREVIEW => PaneState::Preview(serde_json::from_value(state).ok()?),
            KIND_SFTP => PaneState::Sftp(serde_json::from_value(state).ok()?),
            _ => return None,
        };
        parsed.is_valid().then_some(parsed)
    }
}

/// A node of the persisted pane tree. Unknown node types deserialize to
/// [`SerializedNode::Unknown`] and are dropped at restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SerializedNode {
    /// A leaf. `id` is only meaningful within this file (ids are re-minted at
    /// restore); it keys the `active` leaf.
    Pane {
        id: PaneId,
        #[serde(flatten)]
        pane: SerializedPane,
    },
    /// An n-ary split sized by `flexes`.
    Axis {
        #[serde(default)]
        id: PaneId,
        axis: SplitAxis,
        members: Vec<SerializedNode>,
        #[serde(default)]
        flexes: Vec<f32>,
    },
    #[serde(other)]
    Unknown,
}

/// A persisted pane tree plus the active leaf (by file-local id).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedTree {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<SerializedNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<PaneId>,
}

/// One persisted tab: a pane group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneGroupSnapshot {
    /// User rename (`custom_title`), if set.
    #[serde(default)]
    pub title: Option<String>,
    pub tree: SerializedTree,
}

/// v3 counterpart of [`SessionSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshotV3 {
    pub version: u32,
    /// Unix seconds the snapshot was taken (diagnostics only).
    #[serde(default)]
    pub saved_at: u64,
    #[serde(default)]
    pub identity: WorkspaceIdentity,
    /// Index into `tabs` of the tab that was active.
    #[serde(default)]
    pub active_tab_index: usize,
    pub tabs: Vec<PaneGroupSnapshot>,
}

impl SessionSnapshotV3 {
    pub fn new(
        identity: WorkspaceIdentity,
        tabs: Vec<PaneGroupSnapshot>,
        active_tab_index: usize,
    ) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            identity,
            active_tab_index,
            tabs,
        }
    }
}

/// One persisted Space (no colour in v3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceEntryV3 {
    pub name: String,
    pub session: SessionSnapshotV3,
}

/// v3 counterpart of [`SpacesSnapshot`]: the full persisted state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpacesSnapshotV3 {
    pub version: u32,
    #[serde(default)]
    pub active_space_index: usize,
    pub spaces: Vec<SpaceEntryV3>,
}

impl SpacesSnapshotV3 {
    pub fn new(spaces: Vec<SpaceEntryV3>, active_space_index: usize) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            active_space_index,
            spaces,
        }
    }
}

// ────────────────────────────── flex helpers ─────────────────────────────

/// The flexes of the members selected by `keep`, renormalised to sum 1.0.
/// Malformed input (wrong length, non-finite / non-positive values) yields an
/// even split of the survivors.
fn surviving_flexes(flexes: &[f32], keep: &[bool]) -> Vec<f32> {
    let n_keep = keep.iter().filter(|k| **k).count();
    let well_formed =
        flexes.len() == keep.len() && flexes.iter().all(|f| f.is_finite() && *f > 0.0);
    let picked: Vec<f32> = if well_formed {
        flexes
            .iter()
            .zip(keep)
            .filter(|(_, k)| **k)
            .map(|(f, _)| *f)
            .collect()
    } else {
        Vec::new()
    };
    let sum: f32 = picked.iter().sum();
    if picked.len() == n_keep && sum.is_finite() && sum > 0.0 {
        picked.into_iter().map(|f| f / sum).collect()
    } else if n_keep == 0 {
        Vec::new()
    } else {
        vec![1.0 / n_keep as f32; n_keep]
    }
}

// ─────────────────────────────── capture ─────────────────────────────────

impl SerializedTree {
    /// Capture a runtime layout. `state_of` supplies the persistable state of
    /// each leaf; leaves it returns `None` for (transient kinds, panes that
    /// cannot be saved) are left out and the tree collapses around them.
    pub fn capture(
        layout: &WorkspaceLayout,
        state_of: impl Fn(PaneId) -> Option<PaneState>,
    ) -> Self {
        let root = layout
            .group
            .root
            .as_ref()
            .and_then(|m| capture_member(m, &state_of));
        let active = layout
            .active
            .filter(|a| root.as_ref().is_some_and(|r| node_has_leaf(r, *a)))
            .or_else(|| root.as_ref().and_then(first_leaf_id));
        Self { root, active }
    }

    /// Number of leaves.
    pub fn leaf_count(&self) -> usize {
        self.root.as_ref().map_or(0, count_leaves)
    }
}

fn capture_member(
    m: &Member,
    state_of: &impl Fn(PaneId) -> Option<PaneState>,
) -> Option<SerializedNode> {
    match m {
        Member::Pane(id) => {
            let pane = state_of(*id)?.to_wire()?;
            Some(SerializedNode::Pane { id: *id, pane })
        }
        Member::Axis(ax) => {
            let kids: Vec<Option<SerializedNode>> = ax
                .members
                .iter()
                .map(|c| capture_member(c, state_of))
                .collect();
            let keep: Vec<bool> = kids.iter().map(Option::is_some).collect();
            let flexes = surviving_flexes(&ax.flexes, &keep);
            let mut members: Vec<SerializedNode> = kids.into_iter().flatten().collect();
            match members.len() {
                0 => None,
                1 => members.pop(),
                _ => Some(SerializedNode::Axis {
                    id: ax.id,
                    axis: ax.axis,
                    members,
                    flexes,
                }),
            }
        }
    }
}

fn count_leaves(n: &SerializedNode) -> usize {
    match n {
        SerializedNode::Pane { .. } => 1,
        SerializedNode::Axis { members, .. } => members.iter().map(count_leaves).sum(),
        SerializedNode::Unknown => 0,
    }
}

fn node_has_leaf(n: &SerializedNode, target: PaneId) -> bool {
    match n {
        SerializedNode::Pane { id, .. } => *id == target,
        SerializedNode::Axis { members, .. } => members.iter().any(|m| node_has_leaf(m, target)),
        SerializedNode::Unknown => false,
    }
}

fn first_leaf_id(n: &SerializedNode) -> Option<PaneId> {
    match n {
        SerializedNode::Pane { id, .. } => Some(*id),
        SerializedNode::Axis { members, .. } => members.iter().find_map(first_leaf_id),
        SerializedNode::Unknown => None,
    }
}

// ─────────────────────────────── restore ─────────────────────────────────

/// Result of [`restore_tree`].
#[derive(Debug, Clone, PartialEq)]
pub struct RestoredTree {
    /// The rebuilt layout with fresh ids (empty when nothing survived).
    pub layout: WorkspaceLayout,
    /// State per restored leaf, `(new id, state)` in visual order.
    pub panes: Vec<(PaneId, PaneState)>,
    /// `(file-local id, new id)` for every restored leaf, in visual order.
    /// Duplicate old ids in a corrupt file simply appear twice.
    pub remap: Vec<(PaneId, PaneId)>,
    /// Leaves / nodes discarded as unknown, invalid or too deep.
    pub dropped: usize,
}

impl RestoredTree {
    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
    }

    /// The new id of the leaf that had file-local id `old` (first match).
    pub fn new_id(&self, old: PaneId) -> Option<PaneId> {
        self.remap.iter().find(|(o, _)| *o == old).map(|(_, n)| *n)
    }
}

struct RestoreCtx<'a> {
    alloc: &'a mut dyn FnMut() -> PaneId,
    keep: &'a dyn Fn(&PaneState) -> bool,
    panes: Vec<(PaneId, PaneState)>,
    remap: Vec<(PaneId, PaneId)>,
    dropped: usize,
}

fn restore_node(node: &SerializedNode, depth: usize, cx: &mut RestoreCtx<'_>) -> Option<Member> {
    if depth > MAX_TREE_DEPTH {
        cx.dropped += 1;
        return None;
    }
    match node {
        SerializedNode::Pane { id, pane } => match pane.parse().filter(|s| (cx.keep)(s)) {
            Some(state) => {
                let new = (cx.alloc)();
                cx.remap.push((*id, new));
                cx.panes.push((new, state));
                Some(Member::Pane(new))
            }
            None => {
                cx.dropped += 1;
                None
            }
        },
        SerializedNode::Axis {
            axis,
            members,
            flexes,
            ..
        } => {
            let kids: Vec<Option<Member>> = members
                .iter()
                .map(|m| restore_node(m, depth + 1, cx))
                .collect();
            let keep: Vec<bool> = kids.iter().map(Option::is_some).collect();
            let flexes = surviving_flexes(flexes, &keep);
            let mut members: Vec<Member> = kids.into_iter().flatten().collect();
            match members.len() {
                0 => None,
                1 => members.pop(),
                _ => Some(Member::Axis(PaneAxis::with_flexes(
                    (cx.alloc)(),
                    *axis,
                    members,
                    flexes,
                ))),
            }
        }
        SerializedNode::Unknown => {
            cx.dropped += 1;
            None
        }
    }
}

/// Rebuild a layout from a persisted tree with fresh ids from `alloc`.
///
/// Leaves whose kind is unknown, whose state does not parse / validate, or
/// that `keep` rejects (the caller's existence checks: file present, host
/// still configured) are dropped; their siblings absorb the space and axes
/// left with one member collapse into it. Never panics on malformed input;
/// an unusable tree yields an empty layout.
pub fn restore_tree(
    tree: &SerializedTree,
    alloc: &mut dyn FnMut() -> PaneId,
    keep: &dyn Fn(&PaneState) -> bool,
) -> RestoredTree {
    let mut cx = RestoreCtx {
        alloc,
        keep,
        panes: Vec::new(),
        remap: Vec::new(),
        dropped: 0,
    };
    let root = tree.root.as_ref().and_then(|r| restore_node(r, 0, &mut cx));
    let mut layout = WorkspaceLayout {
        group: PaneGroup { root },
        active: None,
    };
    layout.group.normalize();
    // `normalize` only splices / repairs axes; leaf ids are untouched.
    let active = tree
        .active
        .and_then(|old| cx.remap.iter().find(|(o, _)| *o == old).map(|(_, n)| *n))
        .or_else(|| layout.group.first_pane());
    layout.active = active;
    RestoredTree {
        layout,
        panes: cx.panes,
        remap: cx.remap,
        dropped: cx.dropped,
    }
}

// ─────────────────────────── v2 -> v3 migration ──────────────────────────

fn default_terminal() -> PaneSessionSnapshot {
    PaneSessionSnapshot {
        kind: PaneSessionKind::Local,
        cwd: None,
        host_id: None,
        scrollback_id: None,
    }
}

/// Convert a v2 terminal layout, pairing the `i`-th leaf (visual order) with
/// `sessions[i]`; a leaf without a session becomes a plain local shell.
fn convert_terminal_group(
    g: &SerializedPaneGroup,
    sessions: &[PaneSessionSnapshot],
    next: &mut usize,
) -> Option<SerializedNode> {
    match g {
        SerializedPaneGroup::Pane { id } => {
            let session = sessions
                .get(*next)
                .cloned()
                .unwrap_or_else(default_terminal);
            *next += 1;
            let pane = PaneState::Terminal(session).to_wire()?;
            Some(SerializedNode::Pane { id: *id, pane })
        }
        SerializedPaneGroup::Axis {
            id,
            axis,
            members,
            flexes,
        } => {
            let members: Vec<SerializedNode> = members
                .iter()
                .filter_map(|m| convert_terminal_group(m, sessions, next))
                .collect();
            Some(SerializedNode::Axis {
                id: *id,
                axis: *axis,
                members,
                flexes: flexes.clone(),
            })
        }
        SerializedPaneGroup::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => {
            let first = convert_terminal_group(first, sessions, next)?;
            let second = convert_terminal_group(second, sessions, next)?;
            let r = if ratio.is_finite() {
                ratio.clamp(0.0, 1.0)
            } else {
                0.5
            };
            Some(SerializedNode::Axis {
                id: *id,
                axis: *axis,
                members: vec![first, second],
                flexes: vec![r, 1.0 - r],
            })
        }
    }
}

fn single_leaf(state: PaneState) -> Option<SerializedTree> {
    let pane = state.to_wire()?;
    Some(SerializedTree {
        root: Some(SerializedNode::Pane { id: 1, pane }),
        active: Some(1),
    })
}

/// Migrate one v2 tab. `None` for tabs that carry nothing to restore: the
/// legacy `Home` tab and an empty workspace tab (what `plan_restore` turns
/// into a no-op / `Skip`).
pub fn migrate_tab(tab: &TabSnapshot) -> Option<PaneGroupSnapshot> {
    match tab {
        TabSnapshot::Home => None,
        TabSnapshot::Workspace(w) => {
            if w.layout.leaf_count() == 0 || w.sessions.is_empty() {
                return None;
            }
            let root = w
                .layout
                .root
                .as_ref()
                .and_then(|r| convert_terminal_group(r, &w.sessions, &mut 0))?;
            let active = w.layout.active.filter(|a| node_has_leaf(&root, *a));
            let active = active.or_else(|| first_leaf_id(&root));
            Some(PaneGroupSnapshot {
                title: w.title.clone(),
                tree: SerializedTree {
                    root: Some(root),
                    active,
                },
            })
        }
        TabSnapshot::Editor(e) => Some(PaneGroupSnapshot {
            title: None,
            tree: single_leaf(PaneState::Editor(EditorPaneState {
                path: e.path.clone(),
                state: e.state.clone(),
            }))?,
        }),
        TabSnapshot::Preview(p) => Some(PaneGroupSnapshot {
            title: None,
            tree: single_leaf(PaneState::Preview(PreviewPaneState { url: p.url.clone() }))?,
        }),
        TabSnapshot::Sftp(s) => Some(PaneGroupSnapshot {
            title: None,
            tree: single_leaf(PaneState::Sftp(SftpPaneState {
                host_id: s.host_id.clone(),
                title: s.title.clone(),
            }))?,
        }),
    }
}

/// Migrate a v2 session. Dropped tabs shift `active_tab_index` so it keeps
/// pointing at the same tab (or the next surviving one if the active tab
/// itself was dropped).
pub fn migrate_session(s: &SessionSnapshot) -> SessionSnapshotV3 {
    let mut tabs = Vec::new();
    let mut active = 0;
    for (i, tab) in s.tabs.iter().enumerate() {
        if i == s.active_tab_index {
            active = tabs.len();
        }
        if let Some(t) = migrate_tab(tab) {
            tabs.push(t);
        }
    }
    let active = active.min(tabs.len().saturating_sub(1));
    SessionSnapshotV3 {
        version: SNAPSHOT_VERSION,
        saved_at: s.saved_at,
        identity: s.identity.clone(),
        active_tab_index: active,
        tabs,
    }
}

/// Migrate a v2 multi-space snapshot (space colours are discarded).
pub fn migrate_spaces(s: &SpacesSnapshot) -> SpacesSnapshotV3 {
    SpacesSnapshotV3 {
        version: SNAPSHOT_VERSION,
        active_space_index: s.active_space_index,
        spaces: s
            .spaces
            .iter()
            .map(|e| SpaceEntryV3 {
                name: e.name.clone(),
                session: migrate_session(&e.session),
            })
            .collect(),
    }
}

/// Parse a stored session file of any supported generation into v3: a v3
/// file as-is, a v2 [`SpacesSnapshot`] (migrated), or the pre-Spaces flat
/// [`SessionSnapshot`] (migrated into one `Default` space, like
/// `session::load_from`). Returns `None` for corrupt input or an unsupported
/// version; never panics.
pub fn parse_any(raw: &str) -> Option<SpacesSnapshotV3> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let version = value.get("version").and_then(Value::as_u64);
    if version == Some(u64::from(SNAPSHOT_VERSION)) {
        return serde_json::from_value::<SpacesSnapshotV3>(value).ok();
    }
    if version == Some(u64::from(crate::session::SNAPSHOT_VERSION)) {
        if let Ok(v2) = serde_json::from_value::<SpacesSnapshot>(value.clone()) {
            return Some(migrate_spaces(&v2));
        }
    }
    // Pre-Spaces flat file: has `tabs` at the top level and no `spaces`.
    if value.get("spaces").is_none() {
        let flat: SessionSnapshot = serde_json::from_value(value).ok()?;
        return Some(SpacesSnapshotV3 {
            version: SNAPSHOT_VERSION,
            active_space_index: 0,
            spaces: vec![SpaceEntryV3 {
                name: "Default".to_string(),
                session: migrate_session(&flat),
            }],
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_group::SplitDirection;
    use crate::pane_ops::test_support::Rng;
    use crate::session::{
        EditorTabSnapshot, PreviewTabSnapshot, SerializedLayout, SftpTabSnapshot, SpaceEntry,
        WorkspaceTabSnapshot,
    };

    fn counter(start: PaneId) -> impl FnMut() -> PaneId {
        let mut n = start;
        move || {
            n += 1;
            n
        }
    }

    fn keep_all(_: &PaneState) -> bool {
        true
    }

    fn local(cwd: &str, sb: Option<&str>) -> PaneSessionSnapshot {
        PaneSessionSnapshot {
            kind: PaneSessionKind::Local,
            cwd: Some(cwd.into()),
            host_id: None,
            scrollback_id: sb.map(Into::into),
        }
    }

    /// The v2 fixture: `[1 | (2 / 3)]`, outer boundary 0.4, active leaf 2.
    fn v2_split_tab() -> TabSnapshot {
        TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: Some("build".into()),
            layout: SerializedLayout {
                root: Some(SerializedPaneGroup::Axis {
                    id: 100,
                    axis: SplitAxis::Horizontal,
                    members: vec![
                        SerializedPaneGroup::Pane { id: 1 },
                        SerializedPaneGroup::Axis {
                            id: 101,
                            axis: SplitAxis::Vertical,
                            members: vec![
                                SerializedPaneGroup::Pane { id: 2 },
                                SerializedPaneGroup::Pane { id: 3 },
                            ],
                            flexes: vec![0.5, 0.5],
                        },
                    ],
                    flexes: vec![0.4, 0.6],
                }),
                active: Some(2),
            },
            sessions: vec![
                local("/a", Some("sb-a")),
                local("/b", None),
                local("/c", None),
            ],
        })
    }

    fn v2_session() -> SessionSnapshot {
        SessionSnapshot::new(
            vec![
                TabSnapshot::Home,
                v2_split_tab(),
                TabSnapshot::Editor(EditorTabSnapshot {
                    path: "/present.rs".into(),
                    state: None,
                }),
                TabSnapshot::Preview(PreviewTabSnapshot {
                    url: "https://x".into(),
                }),
                TabSnapshot::Sftp(SftpTabSnapshot {
                    host_id: "host-1".into(),
                    title: Some("SFTP".into()),
                }),
            ],
            2,
        )
    }

    #[test]
    fn terminal_layout_becomes_a_terminal_group() {
        let g = migrate_tab(&v2_split_tab()).expect("migrates");
        assert_eq!(g.title.as_deref(), Some("build"));
        assert_eq!(g.tree.leaf_count(), 3);
        assert_eq!(g.tree.active, Some(2));

        let r = restore_tree(&g.tree, &mut counter(500), &keep_all);
        assert_eq!(r.dropped, 0);
        assert_eq!(r.layout.leaves().len(), 3);
        assert_eq!(r.layout.validate_normalized(), Ok(()));
        // Sessions pair with leaves in visual order.
        let cwds: Vec<_> = r
            .panes
            .iter()
            .map(|(_, s)| match s {
                PaneState::Terminal(t) => (t.cwd.clone(), t.scrollback_id.clone()),
                other => panic!("not a terminal: {other:?}"),
            })
            .collect();
        assert_eq!(
            cwds,
            vec![
                (Some("/a".into()), Some("sb-a".into())),
                (Some("/b".into()), None),
                (Some("/c".into()), None),
            ]
        );
        // The active leaf follows the remap (old 2 -> second leaf).
        assert_eq!(r.layout.active, r.new_id(2));
        assert_eq!(r.layout.active, Some(r.layout.leaves()[1]));
        // Outer flexes survive the round trip.
        match &r.layout.group.root {
            Some(Member::Axis(a)) => {
                assert!((a.flexes[0] - 0.4).abs() < 1e-4 && (a.flexes[1] - 0.6).abs() < 1e-4)
            }
            other => panic!("expected axis root, got {other:?}"),
        }
    }

    #[test]
    fn editor_preview_and_sftp_tabs_become_single_leaves() {
        let s = migrate_session(&v2_session());
        // Home dropped; the active v2 tab (editor, index 2) is now index 1.
        assert_eq!(s.tabs.len(), 4);
        assert_eq!(s.active_tab_index, 1);
        let kinds: Vec<PaneKind> = s
            .tabs
            .iter()
            .map(|t| {
                let r = restore_tree(&t.tree, &mut counter(900), &keep_all);
                assert_eq!(r.panes.len().min(1), 1);
                r.panes[0].1.kind()
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                PaneKind::Terminal,
                PaneKind::Editor,
                PaneKind::Preview,
                PaneKind::Sftp
            ]
        );
        let sftp = &s.tabs[3];
        assert_eq!(sftp.tree.leaf_count(), 1);
        match restore_tree(&sftp.tree, &mut counter(0), &keep_all).panes[0]
            .1
            .clone()
        {
            PaneState::Sftp(st) => {
                assert_eq!(st.host_id, "host-1");
                assert_eq!(st.title.as_deref(), Some("SFTP"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn active_index_survives_a_dropped_active_tab() {
        let mut s = v2_session();
        s.active_tab_index = 0; // the Home tab itself
        let m = migrate_session(&s);
        assert_eq!(m.active_tab_index, 0);
        s.active_tab_index = 99;
        assert_eq!(migrate_session(&s).active_tab_index, 0);
        assert!(migrate_session(&SessionSnapshot::new(vec![], 3))
            .tabs
            .is_empty());
    }

    #[test]
    fn empty_workspace_tabs_and_missing_sessions_are_handled() {
        let empty = TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: None,
            layout: SerializedLayout::default(),
            sessions: vec![],
        });
        assert!(migrate_tab(&empty).is_none());

        // More leaves than sessions: the extra leaf becomes a plain local shell.
        let short = TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: None,
            layout: SerializedLayout {
                root: Some(SerializedPaneGroup::Axis {
                    id: 1,
                    axis: SplitAxis::Vertical,
                    members: vec![
                        SerializedPaneGroup::Pane { id: 1 },
                        SerializedPaneGroup::Pane { id: 2 },
                    ],
                    flexes: vec![0.5, 0.5],
                }),
                active: None,
            },
            sessions: vec![local("/only", None)],
        });
        let g = migrate_tab(&short).unwrap();
        let r = restore_tree(&g.tree, &mut counter(0), &keep_all);
        assert_eq!(r.panes.len(), 2);
        assert_eq!(r.layout.active, Some(r.layout.leaves()[0]));
        match &r.panes[1].1 {
            PaneState::Terminal(t) => assert_eq!(t.cwd, None),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn legacy_binary_split_is_migrated_to_an_axis() {
        let legacy = TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: None,
            layout: SerializedLayout {
                root: Some(SerializedPaneGroup::Split {
                    id: 9,
                    axis: SplitAxis::Horizontal,
                    ratio: 0.3,
                    first: Box::new(SerializedPaneGroup::Pane { id: 1 }),
                    second: Box::new(SerializedPaneGroup::Pane { id: 2 }),
                }),
                active: Some(2),
            },
            sessions: vec![local("/l", None), local("/r", None)],
        });
        let g = migrate_tab(&legacy).unwrap();
        let r = restore_tree(&g.tree, &mut counter(0), &keep_all);
        match &r.layout.group.root {
            Some(Member::Axis(a)) => {
                assert!((a.flexes[0] - 0.3).abs() < 1e-4);
                assert_eq!(a.axis, SplitAxis::Horizontal);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(r.layout.active, Some(r.layout.leaves()[1]));
    }

    #[test]
    fn single_ssh_terminal_keeps_its_host() {
        let ssh = TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: None,
            layout: SerializedLayout {
                root: Some(SerializedPaneGroup::Pane { id: 9 }),
                active: Some(9),
            },
            sessions: vec![PaneSessionSnapshot {
                kind: PaneSessionKind::Ssh,
                cwd: Some("/srv".into()),
                host_id: Some("host-1".into()),
                scrollback_id: None,
            }],
        });
        let g = migrate_tab(&ssh).unwrap();
        let r = restore_tree(&g.tree, &mut counter(0), &keep_all);
        assert!(
            matches!(&r.panes[0].1, PaneState::Terminal(t) if t.host_id.as_deref() == Some("host-1"))
        );
        // An SSH terminal with no host is invalid and is dropped.
        let mut tree = g.tree.clone();
        if let Some(SerializedNode::Pane { pane, .. }) = &mut tree.root {
            pane.state["hostId"] = Value::Null;
        }
        assert!(restore_tree(&tree, &mut counter(0), &keep_all).is_empty());
    }

    #[test]
    fn v3_round_trips_through_json_and_is_stamped_3() {
        let snap = SpacesSnapshotV3::new(
            vec![SpaceEntryV3 {
                name: "Default".into(),
                session: SessionSnapshotV3 {
                    version: SNAPSHOT_VERSION,
                    saved_at: 7,
                    identity: WorkspaceIdentity::standalone(),
                    active_tab_index: 0,
                    tabs: migrate_session(&v2_session()).tabs,
                },
            }],
            0,
        );
        assert_eq!(snap.version, 3);
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"kind\":\"terminal\""), "{json}");
        assert!(json.contains("\"type\":\"axis\""));
        let back = parse_any(&json).expect("v3 parses");
        assert_eq!(back, snap);
    }

    #[test]
    fn parse_any_reads_v2_spaces_and_flat_files() {
        let v2 = SpacesSnapshot::new(
            vec![SpaceEntry {
                name: "Work".into(),
                session: v2_session(),
            }],
            0,
        );
        let json = serde_json::to_string(&v2).unwrap();
        let m = parse_any(&json).expect("v2 migrates");
        assert_eq!(m.version, 3);
        assert_eq!(m.spaces[0].name, "Work");
        assert_eq!(m.spaces[0].session.tabs.len(), 4);

        // Pre-Spaces flat file (no `spaces`).
        let flat = serde_json::to_string(&v2_session()).unwrap();
        let m = parse_any(&flat).expect("flat migrates");
        assert_eq!(m.spaces.len(), 1);
        assert_eq!(m.spaces[0].name, "Default");

        // A hand-written v2 file with the old Home tab and an unknown field.
        let raw = r#"{"version":2,"activeSpaceIndex":0,"spaces":[{"name":"D","color":"blue",
            "extra":1,"session":{"version":2,"savedAt":1,"activeTabIndex":1,
            "tabs":[{"kind":"home"},{"kind":"editor","path":"/x.rs"}]}}]}"#;
        let m = parse_any(raw).expect("old file parses");
        assert_eq!(m.spaces[0].session.tabs.len(), 1);
        assert_eq!(m.spaces[0].session.active_tab_index, 0);
    }

    /// Space colours were removed (R10-002): an old v2 file that still carries
    /// `color` must load, and v3 writes must not emit it.
    #[test]
    fn legacy_space_color_loads_and_is_never_written() {
        let raw = r#"{"version":2,"activeSpaceIndex":1,"spaces":[
            {"name":"A","color":"orange","session":{"version":2,"savedAt":1,
                "activeTabIndex":0,"tabs":[]}},
            {"name":"B","color":"blue","session":{"version":2,"savedAt":1,
                "activeTabIndex":0,"tabs":[]}}]}"#;
        let m = parse_any(raw).expect("v2 file with colours loads");
        assert_eq!(m.version, 3);
        assert_eq!(m.spaces.len(), 2);
        assert_eq!(m.spaces[1].name, "B");
        assert_eq!(m.active_space_index, 1);

        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("color"), "{json}");
        let back = parse_any(&json).expect("v3 round trip");
        assert_eq!(back, m);
    }

    #[test]
    fn corrupt_and_unsupported_input_is_rejected_not_fatal() {
        for raw in [
            "",
            "garbage",
            "{}",
            "[]",
            "null",
            r#"{"version":3}"#,
            r#"{"version":3,"spaces":"nope"}"#,
            r#"{"version":9,"spaces":[]}"#,
            r#"{"version":2,"spaces":"x"}"#,
        ] {
            assert!(parse_any(raw).is_none(), "{raw}");
        }
    }

    fn pane(id: PaneId, kind: &str, state: Value) -> SerializedNode {
        SerializedNode::Pane {
            id,
            pane: SerializedPane {
                kind: kind.into(),
                state,
            },
        }
    }

    fn editor(id: PaneId, path: &str) -> SerializedNode {
        pane(id, "editor", serde_json::json!({ "path": path }))
    }

    #[test]
    fn restore_drops_bad_leaves_and_collapses_the_tree() {
        // H[ editor, V[ unknown-kind, preview(empty url) ], editor(bad payload), editor ]
        let tree = SerializedTree {
            root: Some(SerializedNode::Axis {
                id: 1,
                axis: SplitAxis::Horizontal,
                members: vec![
                    editor(1, "/a.rs"),
                    SerializedNode::Axis {
                        id: 2,
                        axis: SplitAxis::Vertical,
                        members: vec![
                            pane(2, "hologram", Value::Null),
                            pane(3, "preview", serde_json::json!({ "url": "  " })),
                        ],
                        flexes: vec![0.5, 0.5],
                    },
                    pane(4, "editor", serde_json::json!({ "path": 42 })),
                    editor(5, "/b.rs"),
                    SerializedNode::Unknown,
                ],
                flexes: vec![0.2, 0.2, 0.2, 0.3, 0.1],
            }),
            active: Some(5),
        };
        let r = restore_tree(&tree, &mut counter(100), &keep_all);
        assert_eq!(r.dropped, 4);
        assert_eq!(r.layout.leaves().len(), 2);
        assert_eq!(r.panes.len(), 2);
        assert_eq!(r.layout.validate_normalized(), Ok(()));
        // Survivors 0.2 and 0.3 renormalise to 0.4 / 0.6.
        match &r.layout.group.root {
            Some(Member::Axis(a)) => {
                assert!((a.flexes[0] - 0.4).abs() < 1e-4, "{:?}", a.flexes);
                assert!((a.flexes[1] - 0.6).abs() < 1e-4);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(r.layout.active, r.new_id(5));
        assert_eq!(
            r.remap.iter().map(|(o, _)| *o).collect::<Vec<_>>(),
            vec![1, 5]
        );
    }

    #[test]
    fn unknown_node_types_and_kinds_survive_deserialisation() {
        let raw = r#"{"root":{"type":"axis","axis":"horizontal","flexes":[0.5,0.25,0.25],
            "members":[
              {"type":"pane","id":1,"kind":"editor","state":{"path":"/a.rs"}},
              {"type":"hologram","id":2,"shape":"cube"},
              {"type":"pane","id":3,"kind":"from-the-future","state":{"x":1}}]},
            "active":1}"#;
        let tree: SerializedTree = serde_json::from_str(raw).expect("tolerant parse");
        assert_eq!(tree.leaf_count(), 2);
        let r = restore_tree(&tree, &mut counter(0), &keep_all);
        assert_eq!(r.dropped, 2);
        assert_eq!(r.panes.len(), 1);
        assert!(matches!(r.layout.group.root, Some(Member::Pane(_))));
    }

    #[test]
    fn restore_applies_the_callers_keep_filter() {
        let tree = SerializedTree {
            root: Some(SerializedNode::Axis {
                id: 1,
                axis: SplitAxis::Vertical,
                members: vec![editor(1, "/gone.rs"), editor(2, "/here.rs")],
                flexes: vec![0.5, 0.5],
            }),
            active: Some(1),
        };
        let keep = |s: &PaneState| matches!(s, PaneState::Editor(e) if e.path != "/gone.rs");
        let r = restore_tree(&tree, &mut counter(0), &keep);
        assert_eq!(r.layout.leaves().len(), 1);
        // The active leaf was dropped -> falls back to the first survivor.
        assert_eq!(r.layout.active, r.layout.group.first_pane());
        // Collapsed to a bare leaf (no one-member axis left).
        assert!(matches!(r.layout.group.root, Some(Member::Pane(_))));
    }

    #[test]
    fn restore_of_nothing_is_an_empty_layout() {
        let r = restore_tree(&SerializedTree::default(), &mut counter(0), &keep_all);
        assert!(r.is_empty() && r.layout.active.is_none() && r.panes.is_empty());
        let all_bad = SerializedTree {
            root: Some(pane(1, "x", Value::Null)),
            active: Some(1),
        };
        let r = restore_tree(&all_bad, &mut counter(0), &keep_all);
        assert!(r.is_empty());
        assert_eq!(r.dropped, 1);
    }

    #[test]
    fn restore_assigns_fresh_unique_ids_even_for_duplicate_old_ids() {
        let tree = SerializedTree {
            root: Some(SerializedNode::Axis {
                id: 7,
                axis: SplitAxis::Horizontal,
                members: vec![editor(1, "/a"), editor(1, "/b")],
                flexes: vec![],
            }),
            active: Some(1),
        };
        let r = restore_tree(&tree, &mut counter(40), &keep_all);
        assert_eq!(r.layout.validate_normalized(), Ok(()));
        let leaves = r.layout.leaves();
        assert_eq!(leaves.len(), 2);
        assert_ne!(leaves[0], leaves[1]);
        assert!(leaves.iter().all(|l| *l > 40));
        // Missing flexes fall back to an even split; first duplicate wins `active`.
        assert_eq!(r.layout.active, Some(leaves[0]));
    }

    #[test]
    fn absurdly_deep_trees_are_bounded() {
        let mut node = editor(1, "/leaf");
        for i in 0..(MAX_TREE_DEPTH * 3) {
            node = SerializedNode::Axis {
                id: i as PaneId,
                axis: if i % 2 == 0 {
                    SplitAxis::Horizontal
                } else {
                    SplitAxis::Vertical
                },
                members: vec![node, editor(1000 + i as PaneId, "/x")],
                flexes: vec![0.5, 0.5],
            };
        }
        let tree = SerializedTree {
            root: Some(node),
            active: None,
        };
        let r = restore_tree(&tree, &mut counter(0), &keep_all);
        assert!(r.dropped > 0);
        assert_eq!(r.layout.validate_normalized(), Ok(()));
    }

    #[test]
    fn capture_then_restore_preserves_shape_and_skips_transient_panes() {
        let mut l = WorkspaceLayout::new(1);
        let mut axis = counter(1000);
        l.split_pane(1, 2, SplitDirection::Right, &mut axis);
        l.split_pane(2, 3, SplitDirection::Down, &mut axis);
        l.split_pane(3, 4, SplitDirection::Down, &mut axis);
        l.group.set_boundary(1001, 0, 0.3);
        l.set_active(2);

        // 4 is a transient pane (e.g. a git graph) and is not persisted.
        let state_of = |id: PaneId| match id {
            1 => Some(PaneState::Terminal(local("/t", None))),
            2 | 3 => Some(PaneState::Editor(EditorPaneState {
                path: format!("/f{id}.rs"),
                state: None,
            })),
            _ => None,
        };
        let tree = SerializedTree::capture(&l, state_of);
        assert_eq!(tree.leaf_count(), 3);
        assert_eq!(tree.active, Some(2));

        let json = serde_json::to_string(&tree).unwrap();
        let tree: SerializedTree = serde_json::from_str(&json).unwrap();
        let r = restore_tree(&tree, &mut counter(50), &keep_all);
        assert_eq!(r.layout.leaves().len(), 3);
        assert_eq!(r.layout.validate_normalized(), Ok(()));
        let kinds: Vec<_> = r.panes.iter().map(|(_, s)| s.kind()).collect();
        assert_eq!(
            kinds,
            vec![PaneKind::Terminal, PaneKind::Editor, PaneKind::Editor]
        );
        assert_eq!(r.layout.active, Some(r.layout.leaves()[1]));
        // Outer ratio kept (0.3 / 0.7).
        match &r.layout.group.root {
            Some(Member::Axis(a)) => assert!((a.flexes[0] - 0.3).abs() < 1e-4, "{:?}", a.flexes),
            other => panic!("{other:?}"),
        }
        // Capturing an empty layout yields an empty tree.
        let empty = SerializedTree::capture(&WorkspaceLayout::empty(), |_| None);
        assert_eq!(empty, SerializedTree::default());
    }

    /// Deterministic fuzz: mutate a valid file and make sure parsing and
    /// restoring never panic and always return something coherent.
    #[test]
    fn mutated_files_never_panic() {
        let v2 = SpacesSnapshot::new(
            vec![SpaceEntry {
                name: "D".into(),
                session: v2_session(),
            }],
            0,
        );
        let seeds = [
            serde_json::to_string(&v2).unwrap(),
            serde_json::to_string(&migrate_spaces(&v2)).unwrap(),
        ];
        let mut rng = Rng(0xC0FF_EE12_3457);
        for seed in &seeds {
            let bytes = seed.as_bytes();
            for _ in 0..400 {
                let mut b = bytes.to_vec();
                match rng.below(4) {
                    0 => b.truncate(rng.below(b.len())),
                    1 => {
                        let i = rng.below(b.len());
                        b[i] = b"{}[],:\"0aZ -"[rng.below(12)];
                    }
                    2 => {
                        let i = rng.below(b.len());
                        b.remove(i);
                    }
                    _ => {
                        let i = rng.below(b.len());
                        let j = rng.below(b.len());
                        b.swap(i, j);
                    }
                }
                let raw = String::from_utf8_lossy(&b).into_owned();
                if let Some(snap) = parse_any(&raw) {
                    for space in &snap.spaces {
                        for tab in &space.session.tabs {
                            let r = restore_tree(&tab.tree, &mut counter(0), &keep_all);
                            assert_eq!(r.layout.validate_normalized(), Ok(()), "{raw}");
                        }
                    }
                }
            }
        }
    }
}
