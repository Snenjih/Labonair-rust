//! Session persistence — restore open tabs / layout on restart (T14-001).
//!
//! Port of the reference `src/modules/session/` (`types.ts`, `capture.ts`,
//! `restore.ts`, `store.ts`). On quit a serialisable [`SessionSnapshot`] of the
//! open tabs and each workspace tab's split-pane tree is written to
//! `<data_dir>/labonair/session.json`; on the next launch (when the
//! `sessionRestore` preference is on) it is read back and the tabs are
//! recreated — local terminals re-spawn (PTYs cannot survive a restart), editor
//! tabs re-open their file, SSH workspace tabs reconnect lazily, and SFTP tabs
//! re-open their browser.
//!
//! The GPUI-side wiring (capture on window-close, restore in `Workspace::new`)
//! lives in [`crate::workspace`]; this module is the pure data model + the
//! restore *plan* (decision logic), both fully unit-tested without GPUI.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::pane::{PaneId, PaneNode, WorkspaceLayout};

/// Bumped whenever the snapshot layout changes incompatibly; an older/newer
/// file is discarded rather than mis-read.
pub const SNAPSHOT_VERSION: u32 = 1;

// ─────────────────────────────── model ───────────────────────────────────

/// A full snapshot of the workbench: every persistable tab plus which one was
/// active.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub version: u32,
    /// Unix seconds the snapshot was taken (diagnostics only).
    pub saved_at: u64,
    /// Index into [`SessionSnapshot::tabs`] of the tab that was active.
    pub active_tab_index: usize,
    pub tabs: Vec<TabSnapshot>,
}

impl SessionSnapshot {
    /// Build a snapshot from already-collected tab descriptors, stamping the
    /// version + timestamp.
    pub fn new(tabs: Vec<TabSnapshot>, active_tab_index: usize) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            active_tab_index,
            tabs,
        }
    }
}

/// One persisted tab. Transient tab kinds (AI diff, git diff/graph, commit
/// diff) are deliberately not represented — like the reference skipping
/// `ai-diff` — they are re-derived from the repo/agent state, not the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TabSnapshot {
    /// The landing / host-manager dashboard.
    Home,
    /// A terminal workspace tab with its split-pane tree.
    Workspace(WorkspaceTabSnapshot),
    /// A local code-editor tab (untitled editors are not persisted).
    Editor(EditorTabSnapshot),
    /// A markdown / web preview tab.
    Preview(PreviewTabSnapshot),
    /// An SFTP dual-pane browser.
    Sftp(SftpTabSnapshot),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTabSnapshot {
    /// User rename (`custom_title`), if set.
    pub title: Option<String>,
    /// The split-pane tree (structure + ratios + active leaf).
    pub layout: WorkspaceLayout,
    /// Per-pane session metadata, in `layout.leaves()` order.
    pub sessions: Vec<PaneSessionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneSessionSnapshot {
    pub kind: PaneSessionKind,
    /// Last known working directory (OSC 7) — the re-spawned shell starts here.
    pub cwd: Option<String>,
    /// Host id for an SSH pane.
    pub host_id: Option<String>,
    /// Stable UUID keying this pane's persisted scrollback file (T14-002).
    /// `None` for older snapshots / panes with no captured scrollback.
    #[serde(default)]
    pub scrollback_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaneSessionKind {
    Local,
    Ssh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorTabSnapshot {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewTabSnapshot {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SftpTabSnapshot {
    pub host_id: String,
    pub title: Option<String>,
}

// ───────────────────────────── restore plan ──────────────────────────────

/// What the restore step should do for one snapshot tab. Pure decision output
/// of [`plan_restore`]; [`crate::workspace::Workspace`] executes it.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreAction {
    /// Open the home dashboard tab.
    Home,
    /// Re-spawn a local terminal workspace. `layout` has fresh pane ids;
    /// `cwds[i]` is the start directory for `layout.leaves()[i]`.
    LocalWorkspace {
        layout: WorkspaceLayout,
        cwds: Vec<Option<String>>,
        /// Scrollback-file UUID per leaf (same order as `cwds`), to replay each
        /// re-spawned pane's persisted history (T14-002).
        scrollback_ids: Vec<Option<String>>,
    },
    /// Reconnect an SSH workspace tab (single pane) for `host_id`.
    SshWorkspace {
        host_id: String,
        title: Option<String>,
    },
    /// Re-open a local editor tab for `path`.
    Editor { path: String },
    /// Re-open an SFTP browser for `host_id`.
    Sftp {
        host_id: String,
        title: Option<String>,
    },
    /// Nothing could be restored for this tab; surface `reason` to the user.
    Skip { title: String, reason: String },
}

/// Result of a restore run — how many tabs came back and which were dropped.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RestoreResult {
    pub restored: usize,
    pub failed: Vec<(String, String)>,
}

/// Translate a snapshot into an ordered list of [`RestoreAction`]s (one per
/// snapshot tab, same order). `host_exists` / `file_exists` gate SSH/SFTP and
/// editor tabs so a deleted host or moved file becomes a clean `Skip` instead
/// of a broken tab. `alloc_pane` mints fresh, process-unique pane ids for the
/// re-spawned layouts.
pub fn plan_restore(
    snapshot: &SessionSnapshot,
    host_exists: impl Fn(&str) -> bool,
    file_exists: impl Fn(&str) -> bool,
    mut alloc_pane: impl FnMut() -> PaneId,
) -> Vec<RestoreAction> {
    snapshot
        .tabs
        .iter()
        .map(|tab| match tab {
            TabSnapshot::Home => RestoreAction::Home,
            TabSnapshot::Editor(e) => {
                if file_exists(&e.path) {
                    RestoreAction::Editor {
                        path: e.path.clone(),
                    }
                } else {
                    RestoreAction::Skip {
                        title: e.path.clone(),
                        reason: format!("File not found: {}", e.path),
                    }
                }
            }
            TabSnapshot::Preview(p) => RestoreAction::Skip {
                title: p.url.clone(),
                reason: "Preview tabs are not restored".to_string(),
            },
            TabSnapshot::Sftp(s) => {
                let title = s.title.clone().unwrap_or_else(|| s.host_id.clone());
                if host_exists(&s.host_id) {
                    RestoreAction::Sftp {
                        host_id: s.host_id.clone(),
                        title: s.title.clone(),
                    }
                } else {
                    RestoreAction::Skip {
                        title,
                        reason: "Host no longer exists".to_string(),
                    }
                }
            }
            TabSnapshot::Workspace(w) => plan_workspace(w, &host_exists, &mut alloc_pane),
        })
        .collect()
}

fn plan_workspace(
    w: &WorkspaceTabSnapshot,
    host_exists: &impl Fn(&str) -> bool,
    alloc_pane: &mut impl FnMut() -> PaneId,
) -> RestoreAction {
    let title = w.title.clone().unwrap_or_default();
    let leaves = w.layout.root.leaves();
    if leaves.is_empty() || w.sessions.is_empty() {
        return RestoreAction::Skip {
            title,
            reason: "Empty workspace tab".to_string(),
        };
    }

    // A single-pane SSH tab reconnects lazily via the host manager. Multi-pane
    // trees are re-spawned as local terminals (mirrors the reference falling
    // back to `kind: "local"` for panes it cannot reconnect).
    let only_ssh = w.sessions.len() == 1 && w.sessions[0].kind == PaneSessionKind::Ssh;
    if only_ssh {
        return match w.sessions[0].host_id.as_deref() {
            Some(host) if host_exists(host) => RestoreAction::SshWorkspace {
                host_id: host.to_string(),
                title: w.title.clone(),
            },
            Some(_) => RestoreAction::Skip {
                title,
                reason: "SSH host no longer exists".to_string(),
            },
            None => RestoreAction::Skip {
                title,
                reason: "SSH session has no host".to_string(),
            },
        };
    }

    let (layout, order) = remap_layout(&w.layout, alloc_pane);
    // Re-order the per-leaf cwds to match the freshly-remapped leaf order
    // (`remap_layout` preserves left→right order, so this is a straight zip).
    let _ = order;
    let leaf_count = w.layout.root.leaves().len();
    let cwds = (0..leaf_count)
        .map(|i| w.sessions.get(i).and_then(|s| s.cwd.clone()))
        .collect();
    let scrollback_ids = (0..leaf_count)
        .map(|i| w.sessions.get(i).and_then(|s| s.scrollback_id.clone()))
        .collect();
    RestoreAction::LocalWorkspace {
        layout,
        cwds,
        scrollback_ids,
    }
}

/// Rebuild a [`WorkspaceLayout`] with fresh pane/split ids, preserving the tree
/// shape, split axes/ratios and the active leaf. Returns the new layout and the
/// new leaf ids in left→right order.
pub fn remap_layout(
    layout: &WorkspaceLayout,
    alloc: &mut impl FnMut() -> PaneId,
) -> (WorkspaceLayout, Vec<PaneId>) {
    let mut leaf_map: Vec<(PaneId, PaneId)> = Vec::new();
    let root = remap_node(&layout.root, alloc, &mut leaf_map);
    let order = leaf_map.iter().map(|(_, new)| *new).collect();
    let active = leaf_map
        .iter()
        .find(|(old, _)| *old == layout.active)
        .map(|(_, new)| *new)
        .or_else(|| leaf_map.first().map(|(_, new)| *new))
        .unwrap_or_else(alloc);
    (WorkspaceLayout { root, active }, order)
}

fn remap_node(
    node: &PaneNode,
    alloc: &mut impl FnMut() -> PaneId,
    leaf_map: &mut Vec<(PaneId, PaneId)>,
) -> PaneNode {
    match node {
        PaneNode::Pane { id } => {
            let new = alloc();
            leaf_map.push((*id, new));
            PaneNode::Pane { id: new }
        }
        PaneNode::Split {
            axis,
            ratio,
            first,
            second,
            ..
        } => PaneNode::Split {
            id: alloc(),
            axis: *axis,
            ratio: *ratio,
            first: Box::new(remap_node(first, alloc, leaf_map)),
            second: Box::new(remap_node(second, alloc, leaf_map)),
        },
    }
}

// ─────────────────────────────── storage ─────────────────────────────────

fn snapshot_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("labonair")
        .join("session.json")
}

/// The persisted snapshot, or `None` if there is none / it is unreadable / its
/// version does not match (a stale file is deleted).
pub fn load_snapshot() -> Option<SessionSnapshot> {
    load_from(&snapshot_path())
}

/// Persist `snapshot` (best-effort; failures are logged, never propagated —
/// losing the session must not block quitting).
pub fn save_snapshot(snapshot: &SessionSnapshot) {
    save_to(&snapshot_path(), snapshot);
}

/// Remove any persisted snapshot (called when the preference is turned off).
pub fn clear_snapshot() {
    let _ = std::fs::remove_file(snapshot_path());
}

pub(crate) fn load_from(path: &Path) -> Option<SessionSnapshot> {
    let raw = std::fs::read_to_string(path).ok()?;
    let snap: SessionSnapshot = serde_json::from_str(&raw).ok()?;
    if snap.version != SNAPSHOT_VERSION {
        let _ = std::fs::remove_file(path);
        return None;
    }
    Some(snap)
}

pub(crate) fn save_to(path: &Path, snapshot: &SessionSnapshot) {
    if let Some(dir) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(dir) {
            tracing::warn!(%err, "failed to create session-state dir");
            return;
        }
    }
    match serde_json::to_string_pretty(snapshot) {
        Ok(raw) => {
            if let Err(err) = std::fs::write(path, raw) {
                tracing::warn!(%err, "failed to write session snapshot");
            }
        }
        Err(err) => tracing::warn!(%err, "failed to serialize session snapshot"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::SplitAxis;

    /// A fresh unique temp directory (same approach as the backend prefs tests;
    /// avoids pulling in `tempfile`).
    fn tmp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-session-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn split_layout() -> WorkspaceLayout {
        let mut l = WorkspaceLayout::new(1);
        l.split(100, 2, SplitAxis::Horizontal);
        l.split(101, 3, SplitAxis::Vertical);
        l.set_ratio(100, 0.4);
        l.set_active(2);
        l
    }

    fn sample() -> SessionSnapshot {
        SessionSnapshot::new(
            vec![
                TabSnapshot::Home,
                TabSnapshot::Workspace(WorkspaceTabSnapshot {
                    title: Some("build".into()),
                    layout: split_layout(),
                    sessions: vec![
                        PaneSessionSnapshot {
                            kind: PaneSessionKind::Local,
                            cwd: Some("/a".into()),
                            host_id: None,
                            scrollback_id: Some("sb-a".into()),
                        },
                        PaneSessionSnapshot {
                            kind: PaneSessionKind::Local,
                            cwd: Some("/b".into()),
                            host_id: None,
                            scrollback_id: None,
                        },
                        PaneSessionSnapshot {
                            kind: PaneSessionKind::Local,
                            cwd: None,
                            host_id: None,
                            scrollback_id: None,
                        },
                    ],
                }),
                TabSnapshot::Workspace(WorkspaceTabSnapshot {
                    title: None,
                    layout: WorkspaceLayout::new(9),
                    sessions: vec![PaneSessionSnapshot {
                        kind: PaneSessionKind::Ssh,
                        cwd: Some("/srv".into()),
                        host_id: Some("host-1".into()),
                        scrollback_id: None,
                    }],
                }),
                TabSnapshot::Editor(EditorTabSnapshot {
                    path: "/present.rs".into(),
                }),
                TabSnapshot::Editor(EditorTabSnapshot {
                    path: "/gone.rs".into(),
                }),
                TabSnapshot::Preview(PreviewTabSnapshot {
                    url: "https://x".into(),
                }),
                TabSnapshot::Sftp(SftpTabSnapshot {
                    host_id: "host-1".into(),
                    title: Some("SFTP · a".into()),
                }),
                TabSnapshot::Sftp(SftpTabSnapshot {
                    host_id: "host-dead".into(),
                    title: None,
                }),
            ],
            1,
        )
    }

    #[test]
    fn snapshot_round_trips_through_json() {
        let snap = sample();
        let json = serde_json::to_string(&snap).unwrap();
        let back: SessionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap, back);
        assert!(json.contains("\"kind\":\"workspace\""));
        assert!(json.contains("\"activeTabIndex\":1"));
    }

    #[test]
    fn load_rejects_version_mismatch_and_deletes_file() {
        let dir = tmp_dir();
        let path = dir.join("session.json");
        let mut snap = sample();
        snap.version = SNAPSHOT_VERSION + 1;
        save_to(&path, &snap);
        assert!(path.exists());
        assert_eq!(load_from(&path), None);
        assert!(!path.exists(), "stale snapshot is removed");
    }

    #[test]
    fn save_then_load_round_trips_on_disk() {
        let dir = tmp_dir();
        let path = dir.join("nested/session.json");
        let snap = sample();
        save_to(&path, &snap);
        assert_eq!(load_from(&path), Some(snap));
    }

    #[test]
    fn pane_snapshot_without_scrollback_id_defaults_to_none() {
        let json = r#"{"kind":"local","cwd":"/x","hostId":null}"#;
        let p: PaneSessionSnapshot = serde_json::from_str(json).unwrap();
        assert_eq!(p.scrollback_id, None);
        assert_eq!(p.cwd.as_deref(), Some("/x"));
    }

    #[test]
    fn missing_file_loads_as_none() {
        let dir = tmp_dir();
        assert_eq!(load_from(&dir.join("nope.json")), None);
    }

    #[test]
    fn remap_layout_preserves_shape_and_active() {
        let l = split_layout();
        let mut next = 500u64;
        let (remapped, order) = remap_layout(&l, &mut || {
            next += 1;
            next
        });
        assert_eq!(order.len(), 3);
        assert_eq!(remapped.root.leaves(), order);
        // All ids are fresh (>= 501) and none collide with the originals.
        assert!(order.iter().all(|id| *id >= 501));
        assert!(remapped.root.contains(remapped.active));
        // Active leaf (index 1, the "2" pane) maps to the 2nd new leaf.
        assert_eq!(remapped.active, order[1]);
        // Ratio survives.
        match &remapped.root {
            PaneNode::Split { ratio, .. } => assert!((*ratio - 0.4).abs() < 1e-6),
            _ => panic!("root must stay a split"),
        }
    }

    #[test]
    fn plan_restore_covers_every_tab_kind() {
        let snap = sample();
        let mut next = 0u64;
        let actions = plan_restore(
            &snap,
            |h| h == "host-1",
            |f| f == "/present.rs",
            || {
                next += 1;
                next
            },
        );
        assert_eq!(actions.len(), snap.tabs.len());
        assert_eq!(actions[0], RestoreAction::Home);

        match &actions[1] {
            RestoreAction::LocalWorkspace {
                layout,
                cwds,
                scrollback_ids,
            } => {
                assert_eq!(layout.root.leaves().len(), 3);
                assert_eq!(
                    cwds,
                    &vec![Some("/a".to_string()), Some("/b".to_string()), None]
                );
                assert_eq!(scrollback_ids, &vec![Some("sb-a".to_string()), None, None]);
            }
            other => panic!("expected local workspace, got {other:?}"),
        }

        assert_eq!(
            actions[2],
            RestoreAction::SshWorkspace {
                host_id: "host-1".into(),
                title: None,
            }
        );
        assert_eq!(
            actions[3],
            RestoreAction::Editor {
                path: "/present.rs".into()
            }
        );
        assert!(
            matches!(actions[4], RestoreAction::Skip { .. }),
            "missing file"
        );
        assert!(matches!(actions[5], RestoreAction::Skip { .. }), "preview");
        assert_eq!(
            actions[6],
            RestoreAction::Sftp {
                host_id: "host-1".into(),
                title: Some("SFTP · a".into()),
            }
        );
        assert!(
            matches!(actions[7], RestoreAction::Skip { .. }),
            "sftp host gone"
        );
    }

    #[test]
    fn plan_restore_skips_ssh_workspace_when_host_deleted() {
        let snap = sample();
        let actions = plan_restore(&snap, |_| false, |_| true, || 0);
        assert!(matches!(
            actions[2],
            RestoreAction::Skip { ref reason, .. } if reason.contains("SSH host")
        ));
    }
}
