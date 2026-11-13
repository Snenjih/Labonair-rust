//! Session v3 capture and restore on `Workspace` (R10-002, ADR 0005).
//!
//! A tab is captured as its pane tree: every leaf is `{kind, state}`
//! ([`crate::session_v3::PaneState`]). Terminals keep their scrollback ids;
//! editors carry their editor-owned session payload (including unsaved-buffer
//! recovery, for every pane, whichever tab is active); previews and SFTP
//! browsers carry their location. Transient kinds (git graph, diffs, keymap,
//! hosts) are not persisted and the tree collapses around them.
//!
//! Restore is defensive: `restore_tree` drops unknown or invalid leaves and
//! anything whose file or host no longer exists, collapses the tree and never
//! panics on corrupt input. Fresh pane ids are minted, so file-local ids never
//! collide with live panes.
//!
//! SSH policy: restored SSH panes reconnect eagerly, one connection per pane,
//! each behind its own loading screen (no lazy placeholder state exists in
//! the pane model); hosts that need a typed password prompt again.

use std::cell::RefCell;

use gpui::{AppContext, Context, Window};

use crate::pane::{PaneId, WorkspaceLayout};
use crate::pane_kind::PaneKind;
use crate::resume::{self, Resume};
use crate::session::{PaneSessionKind, PaneSessionSnapshot, RestoreResult};
use crate::session_v3::{
    restore_tree, EditorPaneState, PaneGroupSnapshot, PaneState, PreviewPaneState, SerializedTree,
    SessionSnapshotV3, SftpPaneState, SpaceEntryV3, SpacesSnapshotV3,
};
use crate::tabs::{Tab, TabData, TabKind};
use crate::views::preview::PreviewView;
use crate::{terminal_settings, PaneEntry, ScrollbackCapture, Workspace};

impl Workspace {
    pub(crate) fn session_snapshot_collect(
        &self,
        cx: &gpui::App,
        scrollbacks: &mut Vec<ScrollbackCapture>,
    ) -> SpacesSnapshotV3 {
        let store = self.tabs.read(cx);
        let space_store = self.spaces.read(cx);
        let active_id = store.active_id();
        let active_space_id = space_store.active_id();
        let captured = RefCell::new(std::mem::take(scrollbacks));
        let mut entries = Vec::with_capacity(space_store.len());
        let mut active_space_index = 0;
        for (index, space) in space_store.spaces().iter().enumerate() {
            if space.id == active_space_id {
                active_space_index = index;
            }
            let mut tabs = Vec::new();
            let mut active_index = 0;
            for tab in store.tabs_in_space(space.id) {
                if let Some(group) = self.snapshot_tab(tab, cx, &captured) {
                    if tab.id == active_id {
                        active_index = tabs.len();
                    }
                    tabs.push(group);
                }
            }
            entries.push(SpaceEntryV3 {
                name: space.name.clone(),
                session: SessionSnapshotV3::new(space.identity.clone(), tabs, active_index),
            });
        }
        *scrollbacks = captured.into_inner();
        SpacesSnapshotV3::new(entries, active_space_index)
    }

    fn snapshot_tab(
        &self,
        tab: &Tab,
        cx: &gpui::App,
        scrollbacks: &RefCell<Vec<ScrollbackCapture>>,
    ) -> Option<PaneGroupSnapshot> {
        let layout = match self.layouts.get(&tab.id) {
            Some(layout) => layout.clone(),
            // A tab that was never rendered has no tree yet: it is its one pane.
            None if tab.kind != TabKind::Workspace => WorkspaceLayout::new(tab.id),
            None => return None,
        };
        let tree = SerializedTree::capture(&layout, |pane| {
            self.pane_state_for_session(tab, pane, cx, scrollbacks)
        });
        tree.root.as_ref()?;
        Some(PaneGroupSnapshot {
            // Only terminal groups keep a user-visible title; file / host
            // titles are derived again on restore.
            title: (tab.kind == TabKind::Workspace)
                .then(|| tab.custom_title.clone())
                .flatten(),
            tree,
        })
    }

    fn pane_state_for_session(
        &self,
        tab: &Tab,
        pane: PaneId,
        cx: &gpui::App,
        scrollbacks: &RefCell<Vec<ScrollbackCapture>>,
    ) -> Option<PaneState> {
        match self.pane_kind(pane, cx) {
            PaneKind::Terminal => {
                let entry = self.panes.get(&pane)?;
                let ts = terminal_settings(cx);
                let ssh = self.ssh_tabs.get(&entry.session_id);
                let scrollback_id = ssh.is_none().then(|| {
                    if let Some(ansi) = entry
                        .view
                        .read(cx)
                        .handle()
                        .serialize_scrollback(ts.session_scrollback_lines())
                    {
                        scrollbacks.borrow_mut().push(ScrollbackCapture {
                            session_id: entry.scrollback_id.clone(),
                            ansi,
                            max_bytes: Some(ts.scrollback_max_bytes()),
                        });
                    }
                    entry.scrollback_id.clone()
                });
                let view = entry.view.read(cx);
                let cwd = view.cwd();
                // An offer the user has not answered yet is kept, so a second
                // quit/crash before they decide does not lose it.
                let resume_command = view.resume_offer().map(str::to_string).or_else(|| {
                    let foreground = view.handle().foreground_process();
                    resume::snapshot_command(
                        ssh.is_some(),
                        cwd.as_deref(),
                        foreground.as_ref().map(|f| f.name.as_str()),
                        &ts.resume_commands(),
                    )
                });
                Some(PaneState::Terminal(PaneSessionSnapshot {
                    kind: if ssh.is_none() {
                        PaneSessionKind::Local
                    } else {
                        PaneSessionKind::Ssh
                    },
                    cwd,
                    host_id: ssh.map(|s| s.host_id.clone()),
                    scrollback_id,
                    resume_command,
                }))
            }
            PaneKind::Editor => {
                let slot = self.editor_slot_of_pane(pane)?;
                if self.remote_edits.contains_key(&slot) {
                    return None;
                }
                let (editor, _) = self.editor_binding(pane, cx)?;
                let editor = editor.read(cx);
                let path = editor.path()?.to_string_lossy().into_owned();
                Some(PaneState::Editor(EditorPaneState {
                    path,
                    cwd: self.pane_anchors.get(&pane).cloned(),
                    state: Some(editor.session_snapshot()),
                }))
            }
            PaneKind::Preview => {
                let url = self.previews.get(&pane)?.read(cx).url().to_string();
                (!url.is_empty()).then_some(PaneState::Preview(PreviewPaneState { url }))
            }
            PaneKind::Sftp => {
                let view = self.sftp_views.get(&pane)?.read(cx);
                let _ = tab;
                Some(PaneState::Sftp(SftpPaneState {
                    host_id: view.host_id().to_string(),
                    title: None,
                }))
            }
            // Transient kinds — never persisted.
            _ => None,
        }
    }

    /// Replay one space's tabs, recreating pane groups and the active tab.
    pub fn restore_session(
        &mut self,
        snapshot: &SessionSnapshotV3,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> RestoreResult {
        let known_hosts: std::collections::HashSet<String> =
            self.host_view.host_ids(cx).into_iter().collect();
        let mut result = RestoreResult::default();
        let mut created: Vec<Option<u64>> = Vec::with_capacity(snapshot.tabs.len());
        for (index, group) in snapshot.tabs.iter().enumerate() {
            let restored = restore_tree(&group.tree, &mut crate::pane::next_id, &|state| {
                state.is_valid()
                    && match state {
                        PaneState::Terminal(t) => {
                            t.kind == PaneSessionKind::Local
                                || t.host_id
                                    .as_deref()
                                    .is_some_and(|h| known_hosts.contains(h))
                        }
                        PaneState::Editor(e) => std::path::Path::new(&e.path).is_file(),
                        PaneState::Preview(_) => true,
                        PaneState::Sftp(s) => known_hosts.contains(&s.host_id),
                    }
            });
            if restored.is_empty() {
                result.failed.push((
                    format!("tab {}", index + 1),
                    "nothing could be restored".into(),
                ));
                created.push(None);
                continue;
            }
            let tab_id = self.restore_tab(group, restored.layout, restored.panes, window, cx);
            if tab_id.is_some() {
                result.restored += 1;
            }
            created.push(tab_id);
        }
        let target = created
            .get(snapshot.active_tab_index)
            .copied()
            .flatten()
            .or_else(|| created.iter().find_map(|c| *c));
        if let Some(id) = target {
            self.select_tab(id, window, cx);
        }
        result
    }

    /// Open the tab for one restored pane tree and create each pane's view.
    fn restore_tab(
        &mut self,
        group: &PaneGroupSnapshot,
        layout: WorkspaceLayout,
        panes: Vec<(PaneId, PaneState)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let (_, first_state) = panes.first()?;
        let (kind, data) = match first_state {
            PaneState::Terminal(t) => (
                TabKind::Workspace,
                TabData {
                    cwd: t.cwd.clone(),
                    host_id: t.host_id.clone(),
                    ..TabData::default()
                },
            ),
            PaneState::Editor(e) => (
                TabKind::Editor,
                TabData {
                    path: Some(e.path.clone()),
                    ..TabData::default()
                },
            ),
            PaneState::Preview(p) => (
                TabKind::Preview,
                TabData {
                    url: Some(p.url.clone()),
                    ..TabData::default()
                },
            ),
            PaneState::Sftp(s) => (
                TabKind::Sftp,
                TabData {
                    host_id: Some(s.host_id.clone()),
                    ..TabData::default()
                },
            ),
        };
        let tab_id = self.tabs.update(cx, |s, cx| s.open(kind, data, cx));
        self.layouts.insert(tab_id, layout);
        let mut documents: Vec<(String, PaneId)> = Vec::new();
        for (pane, state) in panes {
            self.restore_pane(tab_id, pane, state, &mut documents, window, cx);
        }
        // Panes whose resources could not be created (a shell that would not
        // start) leave the tree: the tab keeps whatever did come back.
        let missing: Vec<PaneId> = self
            .tab_pane_ids(tab_id)
            .into_iter()
            .filter(|p| !self.pane_has_resources(*p))
            .collect();
        for pane in missing {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                layout.close_pane(pane);
            }
        }
        if let Some(title) = &group.title {
            self.tabs.update(cx, |s, cx| {
                s.set_custom_title(tab_id, Some(title.clone()), cx)
            });
        }
        // Terminal panes are not registered by `restore_pane`; without metadata
        // the group label reads "Empty" until the tab is first selected.
        self.ensure_layout(tab_id, cx);
        self.sync_pane_meta(cx);
        self.sync_tab_kind(tab_id, cx);
        Some(tab_id)
    }

    fn pane_has_resources(&self, pane: PaneId) -> bool {
        self.panes.contains_key(&pane)
            || self.editors.contains_key(&pane)
            || self.editor_aliases.contains_key(&pane)
            || self.previews.contains_key(&pane)
            || self.sftp_views.contains_key(&pane)
    }

    fn restore_pane(
        &mut self,
        tab_id: u64,
        pane: PaneId,
        state: PaneState,
        documents: &mut Vec<(String, PaneId)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match state {
            PaneState::Terminal(t) => match (t.kind, t.host_id) {
                (PaneSessionKind::Ssh, Some(host_id)) => {
                    if let Some((ssh_id, session_id, feed, handle)) =
                        self.new_ssh_session(&host_id, cx)
                    {
                        self.attach_ssh_terminal(
                            host_id, ssh_id, session_id, feed, handle, pane, tab_id, window, cx,
                        );
                    }
                }
                _ => {
                    // Evaluated now, not at save time: the setting may have
                    // changed since. A pane without cwd never resumes.
                    let resume = if t.kind == PaneSessionKind::Local && t.cwd.is_some() {
                        resume::decide(
                            terminal_settings(cx).resume_on_restore(),
                            t.resume_command.as_deref(),
                        )
                    } else {
                        Resume::None
                    };
                    let startup = match &resume {
                        Resume::Run(command) => Some(command.clone()),
                        _ => None,
                    };
                    if let Some((session_id, handle, scrollback_id)) = self.spawn_session_with(
                        t.cwd.clone(),
                        t.scrollback_id.as_deref(),
                        startup,
                        cx,
                    ) {
                        let view = self.new_terminal_view(handle, window, cx);
                        if let Resume::Offer(command) = resume {
                            view.update(cx, |v, cx| v.offer_resume(command, cx));
                        }
                        self.panes.insert(
                            pane,
                            PaneEntry {
                                session_id,
                                view,
                                scrollback_id,
                            },
                        );
                        if tab_id != 0
                            && self.tabs.read(cx).get(tab_id).is_some_and(|tab| {
                                tab.data.session_id.is_none() && tab.kind == TabKind::Workspace
                            })
                        {
                            let cwd = t.cwd;
                            self.tabs
                                .update(cx, |s, cx| s.set_session(tab_id, session_id, cwd, cx));
                        }
                    }
                }
            },
            PaneState::Editor(e) => {
                self.set_pane_anchor(
                    pane,
                    e.cwd.clone().or_else(|| {
                        let parent = std::path::Path::new(&e.path).parent()?;
                        Some(parent.to_string_lossy().into_owned())
                    }),
                );
                let shared = documents
                    .iter()
                    .find(|(path, _)| *path == e.path)
                    .map(|(_, owner)| *owner);
                if let Some(owner) = shared {
                    // Same file twice in one tree: the second pane is another
                    // view group on the first pane's document.
                    if self.clone_editor_pane_for_restore(owner, pane, cx) {
                        self.register_pane(pane, PaneKind::Editor);
                    }
                    return;
                }
                let view = self.new_editor_view(pane, cx);
                self.watch_editor(pane, &view, cx);
                self.editors.insert(pane, view.clone());
                self.register_pane(pane, PaneKind::Editor);
                let path = std::path::PathBuf::from(&e.path);
                if let Some(state) = e.state {
                    view.update(cx, |editor, _| editor.set_pending_session(state));
                }
                view.update(cx, |editor, cx| editor.open_path(path.clone(), cx));
                documents.push((e.path.clone(), pane));
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if pane == tab_id || self.tab_pane_ids(tab_id).first() == Some(&pane) {
                        let name = name.to_string();
                        self.tabs
                            .update(cx, |s, cx| s.set_custom_title(tab_id, Some(name), cx));
                    }
                }
            }
            PaneState::Preview(p) => {
                let theme = self.theme.clone();
                let view = cx.new(|cx| PreviewView::new(theme, cx));
                view.update(cx, |v, cx| v.set_url(p.url.clone(), cx));
                self.previews.insert(pane, view);
                self.register_pane(pane, PaneKind::Preview);
                let title = std::path::Path::new(&p.url)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string);
                if let Some(title) = title {
                    if self.tab_pane_ids(tab_id).first() == Some(&pane) {
                        self.tabs
                            .update(cx, |s, cx| s.set_custom_title(tab_id, Some(title), cx));
                    }
                }
            }
            PaneState::Sftp(s) => {
                let label = self
                    .host_view
                    .host_name(&s.host_id, cx)
                    .unwrap_or_else(|| s.host_id.clone());
                let session_id = uuid::Uuid::new_v4().to_string();
                self.ssh_connection.update(cx, |store, cx| {
                    store.begin(
                        session_id.clone(),
                        s.host_id.clone(),
                        label.clone(),
                        crate::ssh_connection::ConnectionKind::Sftp,
                        None,
                        cx,
                    );
                });
                let view = self.new_sftp_view(session_id.clone(), s.host_id, label.clone(), cx);
                self.sftp_views.insert(pane, view);
                self.sftp_sessions.insert(pane, session_id);
                self.register_pane(pane, PaneKind::Sftp);
                if self.tab_pane_ids(tab_id).first() == Some(&pane) {
                    let title = format!("SFTP \u{00b7} {label}");
                    self.tabs
                        .update(cx, |st, cx| st.set_custom_title(tab_id, Some(title), cx));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_v3::{parse_any, restore_tree};

    /// A real v2 file: one split terminal tab (local + SSH to a host that no
    /// longer exists) and one editor tab on a missing file.
    const V2_FIXTURE: &str = r#"{
      "version": 2, "activeSpaceIndex": 0,
      "spaces": [{"name": "Default",
        "session": {"version": 2, "savedAt": 0, "activeTabIndex": 0, "tabs": [
          {"kind": "workspace", "title": "dev",
           "layout": {"root": {"type": "axis", "id": 9, "axis": "horizontal",
              "members": [{"type": "pane", "id": 1}, {"type": "pane", "id": 2}],
              "flexes": [0.5, 0.5]}, "active": 1},
           "sessions": [{"kind": "local", "cwd": "/tmp", "hostId": null},
                        {"kind": "ssh", "cwd": null, "hostId": "gone"}]},
          {"kind": "editor", "path": "/definitely/not/here.rs"}]}}]}"#;

    #[test]
    fn v2_fixture_restores_into_collapsed_v3_trees() {
        let snap = parse_any(V2_FIXTURE).expect("v2 migrates");
        let session = &snap.spaces[0].session;
        assert_eq!(session.tabs.len(), 2);
        let keep = |state: &PaneState| match state {
            PaneState::Terminal(t) => t.kind == PaneSessionKind::Local,
            PaneState::Editor(e) => std::path::Path::new(&e.path).is_file(),
            _ => true,
        };
        let mut next = 1000;
        let mut alloc = || {
            next += 1;
            next
        };
        let terminals = restore_tree(&session.tabs[0].tree, &mut alloc, &keep);
        // The SSH leaf is dropped; the axis collapses to the one local pane.
        assert_eq!(terminals.panes.len(), 1);
        assert_eq!(terminals.dropped, 1);
        assert_eq!(terminals.layout.len(), 1);
        // An editor on a missing file restores nothing.
        let editor = restore_tree(&session.tabs[1].tree, &mut alloc, &keep);
        assert!(editor.is_empty());
    }

    #[test]
    fn corrupt_files_never_panic() {
        for raw in [
            "",
            "{",
            "[]",
            r#"{"version":3}"#,
            r#"{"version":3,"spaces":[{}]}"#,
        ] {
            let _ = parse_any(raw);
        }
    }
}
