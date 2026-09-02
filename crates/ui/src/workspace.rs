//! Workspace: tab bar + split-pane content (T04-001, T04-002).
//!
//! [`Workspace`] owns the [`TabStore`], the shared [`TerminalRegistry`], the
//! per-workspace-tab [`WorkspaceLayout`] (its split-pane tree) and the content
//! view for every open pane. It renders the tab bar over the active tab's
//! split-pane tree; the window chrome around it (header, sidebar, status bar,
//! native titlebar) is composed by [`crate::app_shell::AppShell`] (T04-003),
//! which queries this view for the data it surfaces (active cwd, tab label,
//! pane count) and forwards the header's inline search here.
//!
//! Panes / tabs / sessions are three distinct things, mirroring the reference:
//! a *session* is a PTY that lives in the [`TerminalRegistry`] and never pauses;
//! a *pane* is one slot in a workspace tab's [`WorkspaceLayout`] tree, bound to
//! one session; a *tab* selects which pane tree is on screen. Splitting a pane
//! (`Cmd-D` / `Cmd-Shift-D`) spawns a new session in the active pane's cwd;
//! closing the last pane of a tab closes the tab; closing any tab tears down
//! every session in its layout so no shell is orphaned.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::agent_access::AgentAccessStore;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, relative, Animation, AnimationExt, App, AppContext, ClickEvent, Context,
    DragMoveEvent, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Task, Window,
};
use labonair_backend::modules::mcp::{
    mcp_set_session_grant, mcp_tab_op_response, SessionKind, TabOpResult,
};
use labonair_backend::modules::scrollback::{
    scrollback_cleanup, scrollback_delete, scrollback_load, scrollback_save,
};
use labonair_backend::modules::settings::preferences::CursorStyle as PrefCursorStyle;
use labonair_backend::modules::sftp::commands::enqueue_transfer;
use labonair_backend::modules::sftp::connection::sftp_disconnect as sftp_tab_disconnect;
use labonair_backend::modules::ssh::client::{ssh_connect, ssh_disconnect, ssh_trust_host};
use labonair_backend::modules::ssh::pty::SshPtyEvent;
use labonair_backend::modules::ssh::sftp::{
    cleanup_remote_edit_temp, prepare_remote_edit, save_remote_edit,
};
use labonair_backend::modules::ssh::tunnels::{
    active_tunnels, ssh_start_tunnels, ssh_stop_tunnels,
};
use labonair_backend::{App as Backend, AppEvent, EventChannel};
use labonair_terminal::{
    RemoteFeed, RemoteResizer, RemoteWriter, SessionHandle, SessionId, SessionOptions,
    TermDimensions, TerminalColors, TerminalRegistry,
};
use tokio::runtime::Handle as TokioHandle;

use crate::background::BackgroundStore;
use crate::editor::{EditorEvent, EditorView};
use crate::hosts::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};
use crate::pane::{CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout};
use crate::session::{
    plan_restore, PaneSessionKind, PaneSessionSnapshot, RestoreAction, RestoreResult,
    SessionSnapshot, TabSnapshot, WorkspaceTabSnapshot,
};
use crate::settings::GlobalPreferences;
use crate::sftp::{SftpEvent, SftpView};
use crate::tabs::{Tab, TabData, TabKind, TabStore};
use crate::terminal::TerminalView;
use crate::theme::ThemeStore;
use crate::transfers::{TransferBusEvent, TransfersEvent, TransfersView};

/// Interval for draining backend SSH events into the workspace.
const SSH_POLL_INTERVAL: Duration = Duration::from_millis(40);

/// One open SSH terminal tab: its backend session id, the host it targets and
/// the [`RemoteFeed`] used to push remote output / signal disconnects.
struct SshTab {
    ssh_id: String,
    host_id: String,
    feed: RemoteFeed,
    tab_id: u64,
}

/// A tab-lifecycle request from the MCP bridge (`modules::mcp::server`), queued
/// off the event bus and drained in `render` where a `&mut Window` is available
/// — the bridge itself cannot touch tab state (tabs are pure UI), so it emits a
/// request event and waits on a `oneshot` for [`mcp_tab_op_response`].
/// `(session_id, label, kind, host_id, local_pty_id)` for a tab that can be
/// granted MCP agent access — see [`Workspace::mcp_grant_target`].
type McpGrantTarget = (String, String, SessionKind, Option<String>, Option<u32>);

enum McpTabOp {
    Open {
        request_id: String,
        host_id: String,
    },
    Close {
        request_id: String,
        session_id: String,
    },
}

/// Title for an SSH terminal tab, annotated with the bastion when the
/// connection is routed through a jump host (task T07-002, item 5).
fn ssh_tab_title(host_label: &str, jump_label: Option<&str>) -> String {
    match jump_label {
        Some(j) => format!("SSH \u{00b7} {host_label}  \u{2933} {j}"),
        None => format!("SSH \u{00b7} {host_label}"),
    }
}

/// A blocking prompt raised mid-connect by the SSH backend.
enum SshPrompt {
    Trust {
        ssh_id: String,
        host: String,
        fingerprint: String,
        mismatch: bool,
    },
    Password {
        ssh_id: String,
        message: String,
        buffer: String,
        is_2fa: bool,
    },
    Passphrase {
        ssh_id: String,
        buffer: String,
    },
}

impl SshPrompt {
    fn ssh_id(&self) -> &str {
        match self {
            SshPrompt::Trust { ssh_id, .. }
            | SshPrompt::Password { ssh_id, .. }
            | SshPrompt::Passphrase { ssh_id, .. } => ssh_id,
        }
    }
}

/// Interval for syncing terminal cwd/title into their tab labels.
const META_SYNC_INTERVAL: Duration = Duration::from_millis(400);

/// How often the session snapshot is re-written while the app runs (T14-001).
const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(30);

/// Thickness of a split-divider resize handle.
const HANDLE: f32 = 6.0;

/// Per-file byte ceiling for a persisted scrollback, from the `scrollbackMaxSizeMb`
/// preference (T14-002).
fn scrollback_max_bytes(
    p: &labonair_backend::modules::settings::preferences::Preferences,
) -> usize {
    (p.scrollback_max_size_mb.max(1) as usize) * 1024 * 1024
}

/// Retention window in seconds for persisted scrollback files (`None` = keep for
/// the session's lifetime), from the `scrollbackRetentionDays` preference.
fn scrollback_retention_secs(
    p: &labonair_backend::modules::settings::preferences::Preferences,
) -> Option<u64> {
    (p.scrollback_retention_days > 0).then(|| p.scrollback_retention_days as u64 * 86_400)
}

/// Rows of scrollback to persist per pane (`None` = all), from the
/// `sessionScrollbackLines` preference.
fn session_scrollback_lines(
    p: &labonair_backend::modules::settings::preferences::Preferences,
) -> Option<usize> {
    (p.session_scrollback_lines > 0).then_some(p.session_scrollback_lines as usize)
}

/// Value carried by a tab drag.
struct DraggedTab {
    id: u64,
    label: SharedString,
}

/// Value carried while dragging a split divider.
struct PaneResize {
    split_id: PaneId,
}

/// Minimal drag preview for the resize handles (the cursor does the work).
struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Drag image shown while a tab is being reordered.
struct TabDragPreview {
    label: SharedString,
}

impl Render for TabDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_xs()
            .rounded_md()
            .bg(gpui::rgba(0x00000099))
            .text_color(gpui::white())
            .child(self.label.clone())
    }
}

/// One pane's backing session + content view.
struct PaneEntry {
    session_id: SessionId,
    view: Entity<TerminalView>,
    /// Stable UUID keying this pane's persisted scrollback file (T14-002).
    scrollback_id: String,
}

/// A remote file staged into a local temp copy for editing (T08-001). Keyed
/// by the editor tab id; the temp file is uploaded back on save and removed
/// when the tab closes.
struct RemoteEdit {
    session_id: String,
    remote_path: String,
    temp_path: String,
    /// Last-seen dirty flag — a `true → false` transition means the editor
    /// just saved, so the temp copy is pushed back to the host.
    dirty: bool,
}

/// A file-open request queued by an [`SftpView`], drained in `render` where a
/// `&mut Window` is available.
enum PendingOpen {
    Local(String),
    RemoteEdit {
        session_id: String,
        remote_path: String,
        host_id: String,
        temp_path: String,
    },
}

/// The tabbed, split-pane workspace shell.
pub struct Workspace {
    registry: Arc<TerminalRegistry>,
    tabs: Entity<TabStore>,
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    /// Split-pane tree per `Workspace` tab id — survives tab switches so the
    /// layout is never lost.
    layouts: HashMap<u64, WorkspaceLayout>,
    /// Content view + session per pane id (pane ids are process-unique).
    panes: HashMap<PaneId, PaneEntry>,
    /// Editor view per `Editor` tab id.
    editors: HashMap<u64, Entity<EditorView>>,
    /// SFTP browser view per `Sftp` tab id (T08-001).
    sftp_views: HashMap<u64, Entity<SftpView>>,
    /// SFTP session id per `Sftp` tab id — kept alongside the view so the
    /// session can be torn down from `retire_tab` (which has no `cx`).
    sftp_sessions: HashMap<u64, String>,
    /// Active remote-edit temp copies, keyed by editor tab id.
    remote_edits: HashMap<u64, RemoteEdit>,
    /// SFTP tabs queued by the host manager, drained in `render`.
    pending_sftp: Vec<String>,
    /// File-open requests queued by SFTP views, drained in `render`.
    pending_open: Vec<PendingOpen>,
    next_pane_id: PaneId,
    /// Tab id whose close is awaiting unsaved-changes confirmation.
    confirm_close: Option<u64>,
    /// Open tab context menu: `(tab id, anchor position)`.
    context_menu: Option<(u64, gpui::Point<gpui::Pixels>)>,
    focus_handle: FocusHandle,
    _meta_sync: Task<()>,

    // ── SSH (T07-001) ──────────────────────────────────────────────────────
    backend: Backend,
    tokio: TokioHandle,
    host_manager: Entity<HostManagerView>,
    /// Live SSH terminal tabs, keyed by registry session id.
    ssh_tabs: HashMap<SessionId, SshTab>,
    /// Host ids queued by the host manager for connection, drained in `render`
    /// (where a `&mut Window` is available).
    pending_connect: Vec<String>,
    /// The active connect prompt (trust / password / passphrase), if any.
    ssh_prompt: Option<SshPrompt>,
    prompt_focus: FocusHandle,
    prompt_shown: bool,
    /// Backend → workspace SSH events, forwarded off the broadcast bus.
    ssh_events: std::sync::mpsc::Receiver<AppEvent>,
    _ssh_poll: Task<()>,
    /// Periodic session-snapshot writer (T14-001) — covers force-quit; the
    /// window-close hook in `AppShell` covers the normal path.
    _session_save: Task<()>,

    // ── SFTP transfers (T08-002) ──────────────────────────────────────────
    transfers: Entity<TransfersView>,
    /// Transfer-worker events forwarded off the same broadcast bus.
    transfer_events: std::sync::mpsc::Receiver<TransferBusEvent>,

    // ── MCP bridge (T11-005) ──────────────────────────────────────────────
    /// Tab open/close requests from the MCP bridge, drained in `render`.
    pending_mcp: Vec<McpTabOp>,
    /// Client-side mirror of the bridge's per-tab agent-access grants
    /// (T11-006) — shared with the header badge in `AppShell`.
    agent_access: Entity<AgentAccessStore>,

    // ── Snippets (T12-001) ────────────────────────────────────────────────
    /// Snippet commands waiting to be typed into a freshly-opened SSH tab
    /// once its session is established, keyed by `ssh_id`.
    pending_snippet_ssh: HashMap<String, String>,
}

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Arc<TerminalRegistry>,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        backend: Backend,
        tokio: TokioHandle,
        agent_access: Entity<AgentAccessStore>,
        restore: Option<SessionSnapshot>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tabs = cx.new(|_| TabStore::new());
        cx.observe(&tabs, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&tabs, |this, _, ev: &crate::tabs::ActiveTabChanged, cx| {
            if let Some(editor) = this.editors.get(&ev.0).cloned() {
                editor.update(cx, |e, cx| e.check_external(cx));
            }
        })
        .detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();

        let meta_sync = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(META_SYNC_INTERVAL).await;
            if this.update(cx, |this, cx| this.sync_meta(cx)).is_err() {
                break;
            }
        });

        let host_manager =
            cx.new(|cx| HostManagerView::new(backend.clone(), tokio.clone(), theme.clone(), cx));
        cx.observe(&host_manager, |_, _, cx| cx.notify()).detach();
        cx.subscribe(
            &host_manager,
            |this, _, ev: &HostManagerEvent, cx| match ev {
                HostManagerEvent::Connect(host_id) => {
                    this.pending_connect.push(host_id.clone());
                    cx.notify();
                }
                HostManagerEvent::OpenSftp(host_id) => {
                    this.pending_sftp.push(host_id.clone());
                    cx.notify();
                }
            },
        )
        .detach();

        // Forward the backend's broadcast event bus into a plain channel the
        // GPUI poll loop can drain without an async runtime.
        let (ev_tx, ev_rx) = std::sync::mpsc::channel::<AppEvent>();
        let (tev_tx, tev_rx) = std::sync::mpsc::channel::<TransferBusEvent>();
        {
            let mut bus = backend.events.subscribe();
            tokio.spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match bus.recv().await {
                        Ok(raw) => {
                            if let Some(tev) = TransferBusEvent::from_raw(&raw.name, &raw.payload) {
                                if tev_tx.send(tev).is_err() {
                                    break;
                                }
                            } else if let Some(ev) = AppEvent::from_raw(&raw) {
                                if ev_tx.send(ev).is_err() {
                                    break;
                                }
                            }
                        }
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    }
                }
            });
        }
        let transfers =
            cx.new(|cx| TransfersView::new(backend.clone(), tokio.clone(), theme.clone(), cx));
        cx.observe(&transfers, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&transfers, |this, _, ev: &TransfersEvent, cx| {
            this.on_transfers_event(ev, cx)
        })
        .detach();

        let ssh_poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(SSH_POLL_INTERVAL).await;
            let ok = this
                .update(cx, |this, cx| {
                    let mut events = Vec::new();
                    while let Ok(ev) = this.ssh_events.try_recv() {
                        events.push(ev);
                    }
                    for ev in events {
                        this.handle_ssh_event(ev, cx);
                    }
                    let mut tevents = Vec::new();
                    while let Ok(ev) = this.transfer_events.try_recv() {
                        tevents.push(ev);
                    }
                    if !tevents.is_empty() {
                        let view = this.transfers.clone();
                        view.update(cx, |t, cx| {
                            for ev in tevents {
                                t.apply(ev, cx);
                            }
                        });
                    }
                    this.refresh_active_tunnels(cx);
                })
                .is_ok();
            if !ok {
                break;
            }
        });

        let session_save = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(SESSION_SAVE_INTERVAL).await;
            if this
                .update(cx, |this, cx| this.maybe_save_session(cx))
                .is_err()
            {
                break;
            }
        });

        let mut this = Self {
            registry,
            tabs,
            theme,
            background,
            layouts: HashMap::new(),
            panes: HashMap::new(),
            editors: HashMap::new(),
            sftp_views: HashMap::new(),
            sftp_sessions: HashMap::new(),
            remote_edits: HashMap::new(),
            pending_sftp: Vec::new(),
            pending_open: Vec::new(),
            next_pane_id: 1,
            confirm_close: None,
            context_menu: None,
            focus_handle: cx.focus_handle(),
            _meta_sync: meta_sync,
            _session_save: session_save,
            backend,
            tokio,
            host_manager,
            ssh_tabs: HashMap::new(),
            pending_connect: Vec::new(),
            ssh_prompt: None,
            prompt_focus: cx.focus_handle(),
            prompt_shown: false,
            ssh_events: ev_rx,
            _ssh_poll: ssh_poll,
            transfers,
            transfer_events: tev_rx,
            pending_mcp: Vec::new(),
            agent_access,
            pending_snippet_ssh: HashMap::new(),
        };
        cx.observe(&this.agent_access, |_, _, cx| cx.notify())
            .detach();

        // Session restore (T14-001): replay the previous tabs / layout if a
        // snapshot was passed in, otherwise open the default Home + terminal.
        let restored = restore
            .filter(|s| !s.tabs.is_empty())
            .map(|snap| this.restore_session(&snap, window, cx));
        let opened_any = restored.as_ref().map(|r| r.restored > 0).unwrap_or(false);
        if let Some(result) = &restored {
            for (title, reason) in &result.failed {
                tracing::warn!(title, reason, "session tab not restored");
            }
        }
        if !opened_any {
            // Landing tab: the host-manager dashboard.
            this.tabs
                .update(cx, |s, cx| s.open(TabKind::Home, TabData::default(), cx));
            this.open_terminal_tab(window, cx);
        } else if !this
            .tabs
            .read(cx)
            .tabs()
            .iter()
            .any(|t| t.kind == TabKind::Home)
        {
            // Home is never closable, so a snapshot should always carry one;
            // guard anyway so the dashboard is always reachable.
            this.tabs
                .update(cx, |s, cx| s.open(TabKind::Home, TabData::default(), cx));
        }
        // Scrollback cleanup (T14-002): drop files for panes that no longer
        // exist plus anything past the retention window. Runs once on startup,
        // after restore has re-registered the panes it could bring back.
        this.cleanup_scrollback(cx);
        this
    }

    // ── Session persistence (T14-001) ─────────────────────────────────────

    /// Snapshot every persistable tab + each workspace tab's split-pane tree.
    pub fn session_snapshot(&self, cx: &App) -> SessionSnapshot {
        let store = self.tabs.read(cx);
        let active_id = store.active_id();
        let mut tabs = Vec::new();
        let mut active_index = 0;
        for tab in store.tabs() {
            let snap = match tab.kind {
                TabKind::Home => Some(TabSnapshot::Home),
                TabKind::Workspace => self.snapshot_workspace_tab(tab, cx),
                TabKind::Editor => tab
                    .data
                    .path
                    .clone()
                    .filter(|_| tab.data.host_id.is_none())
                    .map(|path| TabSnapshot::Editor(crate::session::EditorTabSnapshot { path })),
                TabKind::Preview => tab
                    .data
                    .url
                    .clone()
                    .map(|url| TabSnapshot::Preview(crate::session::PreviewTabSnapshot { url })),
                TabKind::Sftp => tab.data.host_id.clone().map(|host_id| {
                    TabSnapshot::Sftp(crate::session::SftpTabSnapshot {
                        host_id,
                        title: tab.custom_title.clone(),
                    })
                }),
                TabKind::AiDiff | TabKind::GitGraph | TabKind::GitDiff | TabKind::CommitDiff => {
                    None
                }
            };
            if let Some(snap) = snap {
                if tab.id == active_id {
                    active_index = tabs.len();
                }
                tabs.push(snap);
            }
        }
        SessionSnapshot::new(tabs, active_index)
    }

    fn snapshot_workspace_tab(&self, tab: &Tab, cx: &App) -> Option<TabSnapshot> {
        let layout = self.layouts.get(&tab.id)?.clone();
        let prefs = cx.try_global::<GlobalPreferences>().map(|g| g.0.clone());
        let max_lines = prefs.as_ref().and_then(session_scrollback_lines);
        let max_bytes = prefs.as_ref().map(scrollback_max_bytes);
        let sessions = layout
            .root
            .leaves()
            .iter()
            .map(|leaf| {
                let entry = self.panes.get(leaf);
                let ssh = entry.and_then(|e| self.ssh_tabs.get(&e.session_id));
                let is_local = ssh.is_none();
                // Persist this local pane's scrollback so the restored shell
                // shows its prior history above the fresh prompt (T14-002).
                let scrollback_id = entry.filter(|_| is_local).map(|e| {
                    if let Some(ansi) = e.view.read(cx).handle().serialize_scrollback(max_lines) {
                        let _ = scrollback_save(&e.scrollback_id, &ansi, max_bytes);
                    }
                    e.scrollback_id.clone()
                });
                PaneSessionSnapshot {
                    kind: if is_local {
                        PaneSessionKind::Local
                    } else {
                        PaneSessionKind::Ssh
                    },
                    cwd: entry.and_then(|e| e.view.read(cx).cwd()),
                    host_id: ssh.map(|s| s.host_id.clone()),
                    scrollback_id,
                }
            })
            .collect();
        Some(TabSnapshot::Workspace(WorkspaceTabSnapshot {
            title: tab.custom_title.clone(),
            layout,
            sessions,
        }))
    }

    /// Scrollback-file UUIDs of every currently-open local pane.
    fn known_scrollback_ids(&self) -> Vec<String> {
        self.panes
            .values()
            .map(|e| e.scrollback_id.clone())
            .collect()
    }

    /// Remove orphaned / stale persisted scrollback files (T14-002).
    pub fn cleanup_scrollback(&self, cx: &App) {
        let retention = cx
            .try_global::<GlobalPreferences>()
            .and_then(|g| scrollback_retention_secs(&g.0));
        scrollback_cleanup(&self.known_scrollback_ids(), retention);
    }

    /// Persist the snapshot now if the `sessionRestore` preference is on;
    /// otherwise remove any stale snapshot. Returns nothing — best effort.
    fn maybe_save_session(&self, cx: &App) {
        if cx
            .try_global::<GlobalPreferences>()
            .map(|g| g.0.session_restore)
            .unwrap_or(false)
        {
            crate::session::save_snapshot(&self.session_snapshot(cx));
        }
    }

    /// Replay `snapshot`, recreating tabs and the active selection.
    pub fn restore_session(
        &mut self,
        snapshot: &SessionSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> RestoreResult {
        let known_hosts: std::collections::HashSet<String> =
            self.host_manager.read(cx).host_ids().into_iter().collect();
        let mut next_pane = self.next_pane_id;
        let actions = plan_restore(
            snapshot,
            |h| known_hosts.contains(h),
            |f| std::path::Path::new(f).is_file(),
            || {
                let id = next_pane;
                next_pane += 1;
                id
            },
        );
        self.next_pane_id = next_pane;

        let mut result = RestoreResult::default();
        let mut created: Vec<Option<u64>> = Vec::with_capacity(actions.len());
        for action in actions {
            let tab_id = match action {
                RestoreAction::Home => Some(
                    self.tabs
                        .update(cx, |s, cx| s.open(TabKind::Home, TabData::default(), cx)),
                ),
                RestoreAction::LocalWorkspace {
                    layout,
                    cwds,
                    scrollback_ids,
                } => self.restore_local_workspace(layout, cwds, scrollback_ids, window, cx),
                RestoreAction::SshWorkspace { host_id, title } => {
                    self.connect_host(host_id, window, cx);
                    let id = self.tabs.read(cx).active_id();
                    if let Some(title) = title {
                        self.tabs
                            .update(cx, |s, cx| s.set_custom_title(id, Some(title), cx));
                    }
                    Some(id)
                }
                RestoreAction::Editor { path } => {
                    self.open_file(path, false, window, cx);
                    Some(self.tabs.read(cx).active_id())
                }
                RestoreAction::Sftp { host_id, .. } => {
                    self.open_sftp(host_id, window, cx);
                    Some(self.tabs.read(cx).active_id())
                }
                RestoreAction::Skip { title, reason } => {
                    result.failed.push((title, reason));
                    None
                }
            };
            if tab_id.is_some() {
                result.restored += 1;
            }
            created.push(tab_id);
        }

        // Re-activate the previously-active tab (fall back to the first that
        // came back if it was dropped).
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

    /// Re-spawn a local terminal workspace tab from a remapped layout.
    fn restore_local_workspace(
        &mut self,
        layout: WorkspaceLayout,
        cwds: Vec<Option<String>>,
        scrollback_ids: Vec<Option<String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let leaves = layout.root.leaves();
        // Spawn one session per leaf; bail if the very first cannot start.
        let mut spawned: Vec<(PaneId, SessionId, SessionHandle, String)> = Vec::new();
        for (i, (leaf, cwd)) in leaves.iter().zip(cwds.iter()).enumerate() {
            let replay_id = scrollback_ids.get(i).and_then(|s| s.as_deref());
            match self.spawn_session(cwd.clone(), replay_id, cx) {
                Some((sid, handle, sb_id)) => spawned.push((*leaf, sid, handle, sb_id)),
                None if spawned.is_empty() => return None,
                None => {}
            }
        }
        let (_, first_sid, _, _) = spawned.first()?;
        let first_cwd = cwds.first().cloned().flatten();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(*first_sid, first_cwd, cx));
        for (leaf, sid, handle, scrollback_id) in spawned {
            let view = self.new_terminal_view(handle, window, cx);
            self.panes.insert(
                leaf,
                PaneEntry {
                    session_id: sid,
                    view,
                    scrollback_id,
                },
            );
        }
        // Keep only the leaves that actually got a pane (should be all).
        self.layouts.insert(tab_id, layout);
        Some(tab_id)
    }

    /// The tab store (for later phases / command palette wiring).
    pub fn tab_store(&self) -> &Entity<TabStore> {
        &self.tabs
    }

    /// The working directory of the active pane's shell, if known — feeds the
    /// status-bar cwd breadcrumb (T04-003).
    pub fn active_cwd(&self, cx: &App) -> Option<String> {
        self.active_pane_view(cx).and_then(|v| v.read(cx).cwd())
    }

    /// The active tab's display label.
    pub fn active_tab_label(&self, cx: &App) -> String {
        self.tabs
            .read(cx)
            .active()
            .map(Tab::label)
            .unwrap_or_default()
    }

    /// Number of panes in the active workspace tab (0 for non-workspace tabs).
    pub fn active_pane_count(&self, cx: &App) -> usize {
        self.active_ws_tab(cx)
            .and_then(|id| self.layouts.get(&id))
            .map(WorkspaceLayout::len)
            .unwrap_or(0)
    }

    /// Whether the active tab targets a terminal (vs. an editor / other) —
    /// drives which surface the header's inline search dispatches to.
    pub fn active_is_terminal(&self, cx: &App) -> bool {
        self.tabs
            .read(cx)
            .active()
            .map(|t| t.kind == TabKind::Workspace)
            .unwrap_or(false)
    }

    /// Run the header's inline search against the active terminal pane.
    pub fn search_active(&mut self, query: &str, cx: &mut Context<Self>) -> bool {
        let Some(view) = self.active_pane_view(cx) else {
            return false;
        };
        view.update(cx, |v, cx| v.search(query, cx))
    }

    /// Focus the active pane (called by the app shell after closing an overlay).
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_active(window, cx);
    }

    // ── Menu / shortcut entry points (T04-005) ──────────────────────────────
    // Thin `pub` wrappers so the native menu and keyboard shortcuts drive the
    // exact same code path.

    /// Open a new local terminal tab.
    pub fn new_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_terminal_tab(window, cx);
    }

    /// Open a new local terminal tab rooted at `cwd` — the Explorer's
    /// "Open in Terminal" context action.
    pub fn new_terminal_tab_in(
        &mut self,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
    }

    // ── Snippets (T12-001) ────────────────────────────────────────────────

    /// Type `command` (without executing beyond the trailing newline the caller
    /// includes, if any) into the active terminal pane — snippet "inject" mode.
    /// No-op when the active pane is not a terminal.
    pub fn inject_into_active_terminal(&self, text: &str, cx: &App) {
        if let Some(view) = self.active_pane_view(cx) {
            let _ = view.read(cx).handle().write(text.as_bytes());
        }
    }

    /// Open a new local terminal tab in `cwd` and run `command` in it —
    /// snippet "terminal" mode, local target.
    pub fn run_snippet_local(
        &mut self,
        cwd: Option<String>,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle.clone(), window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
        let _ = handle.write(format!("{}\n", command.trim_end()).as_bytes());
        cx.notify();
    }

    /// Open (or reuse a connection to) an SSH tab for `host_id` and run
    /// `command` in it once the session is established — snippet "terminal"
    /// mode, SSH target.
    pub fn run_snippet_ssh_terminal(
        &mut self,
        host_id: String,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ssh_id) = self.connect_host(host_id, window, cx) {
            self.pending_snippet_ssh.insert(ssh_id, command);
        }
    }

    /// The `ssh_id` of a live SSH session for `host_id`, if one is open — used
    /// by snippet "silent" mode against SSH targets.
    pub fn ssh_session_for_host(&self, host_id: &str) -> Option<String> {
        self.ssh_tabs
            .values()
            .find(|t| t.host_id == host_id)
            .map(|t| t.ssh_id.clone())
    }

    /// Open a file from the Explorer in the code editor. `peek` opens it as a
    /// reusable preview tab (single click); a non-peek call (double click, or a
    /// file already open) makes/keeps it permanent. Clicking a different file
    /// while a peek tab is open replaces that tab's content instead of piling
    /// up tabs — the VS Code / Labonair "peek" behaviour.
    pub fn open_file(
        &mut self,
        path: String,
        peek: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pathbuf = std::path::PathBuf::from(&path);
        let title = pathbuf
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path.as_str())
            .to_string();

        // Already open (permanent) → just activate it.
        let existing = self.editors.iter().find_map(|(tab_id, view)| {
            (view.read(cx).path().as_deref() == Some(pathbuf.as_path())).then_some(*tab_id)
        });
        if let Some(tab_id) = existing {
            self.tabs.update(cx, |s, cx| {
                if !peek {
                    s.set_peek(tab_id, false, cx);
                }
                s.set_active(tab_id, cx);
            });
            self.focus_active(window, cx);
            return;
        }

        // Reuse an existing peek tab if present.
        let peek_tab = self
            .tabs
            .read(cx)
            .tabs_by_kind(TabKind::Editor)
            .iter()
            .find(|t| t.peek)
            .map(|t| t.id);

        let tab_id = if let Some(tab_id) = peek_tab {
            self.tabs.update(cx, |s, cx| {
                s.set_path(tab_id, Some(path.clone()), cx);
                s.set_custom_title(tab_id, Some(title.clone()), cx);
                s.set_peek(tab_id, peek, cx);
                s.set_active(tab_id, cx);
            });
            tab_id
        } else {
            let tab_id = self.tabs.update(cx, |s, cx| {
                let id = s.open(
                    TabKind::Editor,
                    TabData {
                        path: Some(path.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                s.set_custom_title(id, Some(title.clone()), cx);
                s.set_peek(id, peek, cx);
                id
            });
            let view = self.new_editor_view(cx);
            self.watch_editor(tab_id, &view, cx);
            self.editors.insert(tab_id, view);
            tab_id
        };

        let view = self.editors.get(&tab_id).cloned();
        if let Some(view) = view {
            view.update(cx, |e, cx| e.open_path(pathbuf, cx));
        }
        self.focus_active(window, cx);
    }

    /// `Cmd-E` / File ▸ New Editor Tab — an empty, pathless editor.
    pub fn new_editor_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open(TabKind::Editor, TabData::default(), cx));
        let view = self.new_editor_view(cx);
        self.watch_editor(tab_id, &view, cx);
        self.editors.insert(tab_id, view);
        self.focus_active(window, cx);
    }

    /// Save the active editor tab (`Cmd-S`).
    pub fn save_active(&mut self, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        if let Some(view) = self.editors.get(&id).cloned() {
            view.update(cx, |e, cx| e.save(cx));
        }
    }

    /// Route the header's Find action: editor tab → editor find bar (returns
    /// `true`); otherwise let the caller open the terminal search.
    pub fn find_in_active_editor(&mut self, cx: &mut Context<Self>) -> bool {
        let id = self.tabs.read(cx).active_id();
        if let Some(view) = self.editors.get(&id).cloned() {
            view.update(cx, |e, cx| e.toggle_find(cx));
            true
        } else {
            false
        }
    }

    fn new_editor_view(&self, cx: &mut Context<Self>) -> Entity<EditorView> {
        let theme = self.theme.clone();
        cx.new(|cx| EditorView::new(theme, cx))
    }

    fn watch_editor(&self, tab_id: u64, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        cx.subscribe(view, move |this, view, ev: &EditorEvent, cx| {
            if matches!(ev, EditorEvent::CloseRequested) {
                // Vim `:q` / `:wq` — close this editor's tab.
                if let Some(removed) = this.tabs.update(cx, |s, cx| s.close(tab_id, cx)) {
                    this.retire_tab(&removed, cx);
                }
                cx.notify();
                return;
            }
            let dirty = view.read(cx).is_dirty();
            let is_remote_edit = this.remote_edits.contains_key(&tab_id);
            this.tabs.update(cx, |s, cx| {
                s.set_dirty(tab_id, dirty, cx);
                if matches!(ev, EditorEvent::Edited) {
                    s.set_peek(tab_id, false, cx);
                }
                if !is_remote_edit {
                    if let Some(title) = view
                        .read(cx)
                        .path()
                        .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
                    {
                        s.set_custom_title(tab_id, Some(title), cx);
                    }
                }
            });

            // Remote-edit tab: a `dirty → clean` transition means the editor
            // just saved the local temp copy — push it back to the host.
            if let Some(re) = this.remote_edits.get_mut(&tab_id) {
                let saved = re.dirty && !dirty;
                re.dirty = dirty;
                if saved {
                    let app = this.backend.clone();
                    let (sid, rpath, tpath) = (
                        re.session_id.clone(),
                        re.remote_path.clone(),
                        re.temp_path.clone(),
                    );
                    let jh = this.tokio.spawn(async move {
                        save_remote_edit(sid, rpath, tpath, &app.ssh, app.clone())
                            .await
                            .map_err(|e| e.to_string())
                    });
                    cx.spawn(async move |_this, _cx| {
                        if let Ok(Err(e)) = jh.await {
                            tracing::warn!(%e, "save_remote_edit failed");
                        }
                    })
                    .detach();
                }
            }
            cx.notify();
        })
        .detach();
    }

    /// Split the active workspace pane along `axis`.
    pub fn split(&mut self, axis: SplitAxis, window: &mut Window, cx: &mut Context<Self>) {
        self.split_active(axis, window, cx);
    }

    /// `Close Tab`: close the active pane if the tab is split, else the tab.
    pub fn close_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_active_pane_or_tab(window, cx);
    }

    /// `Close Pane`: close just the active pane.
    pub fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_active_pane(window, cx);
    }

    /// Cycle to the next (`forward`) or previous tab.
    pub fn cycle(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(forward, window, cx);
    }

    /// Whether the active tab is a workspace tab whose layout is split.
    pub fn active_has_split(&self, cx: &App) -> bool {
        self.active_layout(cx).map(|l| l.len() > 1).unwrap_or(false)
    }

    fn theme_colors(&self, cx: &App) -> TerminalColors {
        TerminalColors::from_theme(self.theme.read(cx).theme())
    }

    fn alloc_pane(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    /// Spawn a local terminal session in `cwd` and return its id + handle +
    /// scrollback-file UUID. When `replay_scrollback_id` is `Some`, that id is
    /// reused (session restore) and its persisted scrollback — if any — is fed
    /// into the emulator before the shell starts (T14-002); otherwise a fresh
    /// id is minted.
    fn spawn_session(
        &self,
        cwd: Option<String>,
        replay_scrollback_id: Option<&str>,
        cx: &App,
    ) -> Option<(SessionId, SessionHandle, String)> {
        let p = cx
            .try_global::<crate::settings::GlobalPreferences>()
            .map(|g| g.0.clone())
            .unwrap_or_default();
        let shell = (!p.terminal_shell.trim().is_empty()).then(|| p.terminal_shell.clone());
        let cursor_shape = Some(match p.terminal_cursor_style {
            PrefCursorStyle::Block => labonair_terminal::CursorShape::Block,
            PrefCursorStyle::Underline => labonair_terminal::CursorShape::Underline,
            PrefCursorStyle::Bar => labonair_terminal::CursorShape::Beam,
        });
        let scrollback_id = replay_scrollback_id
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let replay_scrollback =
            replay_scrollback_id.and_then(|id| scrollback_load(id, Some(scrollback_max_bytes(&p))));
        let options = SessionOptions {
            working_directory: cwd,
            shell,
            scrollback: Some(p.terminal_scrollback.max(1) as usize),
            replay_scrollback,
            cursor_shape,
            cursor_blink: Some(p.terminal_cursor_blink),
            ..SessionOptions::default()
        };
        let session_id =
            match self
                .registry
                .create(self.theme_colors(cx), TermDimensions::new(80, 24), options)
            {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!(%err, "failed to spawn terminal session");
                    return None;
                }
            };
        let handle = self.registry.handle(session_id)?;
        Some((session_id, handle, scrollback_id))
    }

    fn new_terminal_view(
        &self,
        handle: SessionHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        let theme = self.theme.clone();
        let background = self.background.clone();
        cx.new(|cx| TerminalView::new(handle, theme, background, window, cx))
    }

    /// Spawn a new local terminal session and open a workspace tab for it.
    fn open_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.active_pane_view(cx).and_then(|v| v.read(cx).cwd());
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
    }

    /// Split the active pane of the active workspace tab, spawning a new
    /// terminal in the same cwd.
    fn split_active(&mut self, axis: SplitAxis, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab_id) = self.active_ws_tab(cx) else {
            return;
        };
        let cwd = self.active_pane_view(cx).and_then(|v| v.read(cx).cwd());
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd, None, cx) else {
            return;
        };
        let split_id = self.alloc_pane();
        let new_pane = self.alloc_pane();
        if let Some(layout) = self.layouts.get_mut(&tab_id) {
            layout.split(split_id, new_pane, axis);
        }
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            new_pane,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.focus_active(window, cx);
        cx.notify();
    }

    /// `Cmd-W`: close the active pane if the tab is split, otherwise close the
    /// whole tab.
    fn close_active_pane_or_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            let multi = self
                .layouts
                .get(&tab_id)
                .map(WorkspaceLayout::len)
                .unwrap_or(0)
                > 1;
            if multi {
                self.close_active_pane(window, cx);
                return;
            }
        }
        let id = self.tabs.read(cx).active_id();
        self.request_close(id, window, cx);
    }

    fn close_active_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab_id) = self.active_ws_tab(cx) else {
            return;
        };
        let Some(pane) = self.layouts.get(&tab_id).map(|l| l.active) else {
            return;
        };
        let outcome = self
            .layouts
            .get_mut(&tab_id)
            .map(|l| l.close(pane))
            .unwrap_or(CloseOutcome::NotFound);
        match outcome {
            CloseOutcome::LastPane => self.request_close(tab_id, window, cx),
            CloseOutcome::Closed { .. } => {
                self.retire_pane(pane);
                self.focus_active(window, cx);
                cx.notify();
            }
            CloseOutcome::NotFound => {}
        }
    }

    fn set_pane_active(&mut self, pane_id: PaneId, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                if layout.set_active(pane_id) {
                    cx.notify();
                }
            }
        }
        self.focus_active(window, cx);
    }

    fn resize_split(&mut self, split_id: PaneId, ratio: f32, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                if layout.set_ratio(split_id, ratio) {
                    cx.notify();
                }
            }
        }
    }

    fn reset_split(&mut self, split_id: PaneId, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                if layout.reset_ratio(split_id) {
                    cx.notify();
                }
            }
        }
    }

    // ── Lookups ─────────────────────────────────────────────────────────────

    fn active_ws_tab(&self, cx: &App) -> Option<u64> {
        let tab = self.tabs.read(cx).active()?;
        (tab.kind == TabKind::Workspace).then_some(tab.id)
    }

    fn active_layout<'a>(&'a self, cx: &App) -> Option<&'a WorkspaceLayout> {
        self.layouts.get(&self.active_ws_tab(cx)?)
    }

    fn active_pane_view(&self, cx: &App) -> Option<Entity<TerminalView>> {
        let pane = self.active_layout(cx)?.active;
        self.panes.get(&pane).map(|e| e.view.clone())
    }

    fn select_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        self.focus_active(window, cx);
    }

    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active_id = self.tabs.read(cx).active_id();
        if let Some(editor) = self.editors.get(&active_id) {
            editor.read(cx).focus(window);
        } else if let Some(view) = self.active_pane_view(cx) {
            view.read(cx).focus(window);
        } else {
            window.focus(&self.focus_handle);
        }
    }

    fn cycle_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.cycle(forward, cx));
        self.focus_active(window, cx);
    }

    /// Request closing a tab. Editor tabs with unsaved changes first ask for
    /// confirmation; everything else closes immediately, sessions torn down.
    fn request_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let needs_confirm = self
            .tabs
            .read(cx)
            .get(id)
            .map(Tab::needs_close_confirm)
            .unwrap_or(false);
        if needs_confirm && self.confirm_close != Some(id) {
            self.confirm_close = Some(id);
            cx.notify();
            return;
        }
        self.do_close(id, window, cx);
    }

    fn do_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_close == Some(id) {
            self.confirm_close = None;
        }
        if let Some(removed) = self.tabs.update(cx, |s, cx| s.close(id, cx)) {
            self.retire_tab(&removed, cx);
        }
        self.focus_active(window, cx);
    }

    fn close_others(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_others(id, cx));
        for tab in &removed {
            self.retire_tab(tab, cx);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    fn close_by_kind(&mut self, kind: TabKind, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_by_kind(kind, cx));
        for tab in &removed {
            self.retire_tab(tab, cx);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    /// Activate a tab by id and move focus into it — used by the header
    /// agent-access badge's "jump to tab".
    pub fn reveal_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        self.focus_active(window, cx);
    }

    // ── Command-palette entry points (T12-002) ─────────────────────────────

    /// The [`CommandContext`] the active tab exposes, driving which
    /// context-scoped palette commands are offered.
    pub fn active_context(&self, cx: &App) -> Option<crate::command_palette::CommandContext> {
        let active = self.tabs.read(cx).active()?;
        let is_ssh = self.ssh_tabs.values().any(|t| t.tab_id == active.id);
        crate::command_palette::context_of(active.kind, is_ssh)
    }

    /// Close every tab except the active one (palette "Close Other Tabs").
    pub fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        self.close_others(id, window, cx);
    }

    /// Open a fresh copy of the active tab (palette "Duplicate Tab").
    /// Terminal tabs re-spawn (inheriting cwd); editor tabs re-open the file.
    pub fn duplicate_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = self.tabs.read(cx).active().cloned() else {
            return;
        };
        match active.kind {
            TabKind::Workspace => self.open_terminal_tab(window, cx),
            TabKind::Editor => {
                if let Some(path) = active.data.path.clone() {
                    self.open_file(path, false, window, cx);
                }
            }
            _ => {}
        }
    }

    /// Send the ANSI clear-screen sequence to the active terminal pane
    /// (palette "Clear Terminal").
    pub fn clear_active_terminal(&self, cx: &App) {
        self.inject_into_active_terminal("\x1b[2J\x1b[H", cx);
    }

    /// Tear down one pane's session + content view.
    fn retire_pane(&mut self, pane_id: PaneId) {
        if let Some(entry) = self.panes.remove(&pane_id) {
            self.registry.close(entry.session_id);
            // Drop this pane's persisted scrollback — a closed pane is never
            // restored (T14-002).
            scrollback_delete(&entry.scrollback_id);
        }
    }

    /// Tear down a removed tab's whole pane tree.
    fn retire_tab(&mut self, tab: &Tab, cx: &mut Context<Self>) {
        // Closing a tab revokes any agent-access grant that followed it —
        // backend (so the bridge stops exposing the session) and local mirror.
        if self.agent_access.read(cx).is_granted(tab.id) {
            self.agent_access.update(cx, |s, cx| {
                s.set_grant(
                    tab.id,
                    String::new(),
                    false,
                    String::new(),
                    SessionKind::Ssh,
                    None,
                    None,
                    cx,
                );
            });
        }
        if let Some(layout) = self.layouts.remove(&tab.id) {
            for leaf in layout.leaves() {
                self.retire_pane(leaf);
            }
        }
        self.editors.remove(&tab.id);

        // SFTP browser tab: drop the view and close its SFTP/SSH session.
        self.sftp_views.remove(&tab.id);
        if let Some(session_id) = self.sftp_sessions.remove(&tab.id) {
            let _ = sftp_tab_disconnect(session_id, &self.backend.ssh);
        }

        // Editor tab backed by a remote-edit temp copy: clean the temp file.
        if let Some(re) = self.remote_edits.remove(&tab.id) {
            let temp = re.temp_path;
            self.tokio.spawn(async move {
                let _ = cleanup_remote_edit_temp(temp).await;
            });
        }

        // Disconnect any SSH session owned by this tab.
        let dead: Vec<SessionId> = self
            .ssh_tabs
            .iter()
            .filter(|(_, t)| t.tab_id == tab.id)
            .map(|(k, _)| *k)
            .collect();
        for sid in dead {
            if let Some(t) = self.ssh_tabs.remove(&sid) {
                let app = self.backend.clone();
                let ssh_id = t.ssh_id.clone();
                let host_id = t.host_id.clone();
                let tab_key = t.tab_id.to_string();
                self.tokio.spawn(async move {
                    // Closing a tab revokes any MCP bridge grant that followed it.
                    let _ = mcp_set_session_grant(
                        tab_key,
                        String::new(),
                        false,
                        String::new(),
                        SessionKind::Ssh,
                        None,
                        None,
                        app.clone(),
                        &app.mcp,
                    )
                    .await;
                    let _ = ssh_disconnect(ssh_id, &app.ssh).await;
                    let _ = ssh_stop_tunnels(host_id, &app.tunnels).await;
                });
                if let Some(p) = &self.ssh_prompt {
                    if p.ssh_id() == t.ssh_id {
                        self.ssh_prompt = None;
                    }
                }
            }
        }
    }

    // ── SFTP browser (T08-001) ────────────────────────────────────────────

    /// Open (or re-focus) a dual-pane SFTP browser tab for `host_id`.
    fn open_sftp(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let existing = self.sftp_views.keys().copied().find(|id| {
            self.tabs
                .read(cx)
                .get(*id)
                .and_then(|t| t.data.host_id.clone())
                == Some(host_id.clone())
        });
        if let Some(tab_id) = existing {
            self.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
            self.focus_active(window, cx);
            return;
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let label = self
            .host_manager
            .read(cx)
            .host_name(&host_id)
            .unwrap_or_else(|| host_id.clone());
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Sftp,
                TabData {
                    host_id: Some(host_id.clone()),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(format!("SFTP \u{00b7} {label}")), cx);
            id
        });
        let view = cx.new(|cx| {
            SftpView::new(
                self.backend.clone(),
                self.tokio.clone(),
                self.theme.clone(),
                session_id.clone(),
                host_id,
                label,
                cx,
            )
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&view, |this, _view, ev: &SftpEvent, cx| {
            this.on_sftp_event(ev, cx)
        })
        .detach();
        self.sftp_views.insert(tab_id, view);
        self.sftp_sessions.insert(tab_id, session_id);
        self.focus_active(window, cx);
        cx.notify();
    }

    fn on_sftp_event(&mut self, ev: &SftpEvent, cx: &mut Context<Self>) {
        match ev {
            SftpEvent::OpenLocalFile(path) => {
                self.pending_open.push(PendingOpen::Local(path.clone()));
                cx.notify();
            }
            SftpEvent::OpenRemoteFile {
                session_id,
                remote_path,
                host_id,
            } => {
                let app = self.backend.clone();
                let (sid, rpath, hid) = (session_id.clone(), remote_path.clone(), host_id.clone());
                let (jh_sid, jh_rpath) = (sid.clone(), rpath.clone());
                let jh = self.tokio.spawn(async move {
                    prepare_remote_edit(jh_sid, jh_rpath, None, &app.ssh, app.clone())
                        .await
                        .map_err(|e| e.to_string())
                });
                cx.spawn(async move |this, cx| {
                    let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
                    let _ = this.update(cx, |this, cx| match res {
                        Ok(temp_path) => {
                            this.pending_open.push(PendingOpen::RemoteEdit {
                                session_id: sid,
                                remote_path: rpath,
                                host_id: hid,
                                temp_path,
                            });
                            cx.notify();
                        }
                        Err(e) => tracing::warn!(%e, "prepare_remote_edit failed"),
                    });
                })
                .detach();
            }
            SftpEvent::Enqueue {
                session_id,
                src_path,
                dest_path,
                direction,
            } => {
                let worker = self.backend.transfer.clone();
                let (sid, src, dest, dir) = (
                    session_id.clone(),
                    src_path.clone(),
                    dest_path.clone(),
                    direction.to_string(),
                );
                self.tokio.spawn(async move {
                    if let Err(e) = enqueue_transfer(sid, src, dest, dir, &worker).await {
                        tracing::warn!(%e, "enqueue_transfer failed");
                    }
                });
                // Surface the transfer panel so the user sees the new job.
                self.transfers.update(cx, |t, cx| {
                    t.reveal(cx);
                });
            }
        }
    }

    fn on_transfers_event(&mut self, ev: &TransfersEvent, cx: &mut Context<Self>) {
        match ev {
            TransfersEvent::Completed {
                session_id,
                direction,
            } => {
                let remote = matches!(
                    direction,
                    labonair_backend::modules::sftp::TransferDirection::Upload
                );
                let view = self
                    .sftp_views
                    .iter()
                    .find_map(|(_, v)| (v.read(cx).session_id() == session_id).then(|| v.clone()));
                if let Some(view) = view {
                    view.update(cx, |v, cx| v.reload_side(remote, cx));
                }
            }
        }
    }

    /// Open an editor tab on a remote-edit temp copy and register it so a
    /// save uploads it back to the host.
    fn open_remote_edit(
        &mut self,
        session_id: String,
        remote_path: String,
        host_id: String,
        temp_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Already open → just activate.
        if let Some(tab_id) = self
            .remote_edits
            .iter()
            .find(|(_, re)| re.temp_path == temp_path)
            .map(|(id, _)| *id)
        {
            self.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
            self.focus_active(window, cx);
            return;
        }

        let base = std::path::Path::new(&remote_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
            .to_string();
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Editor,
                TabData {
                    path: Some(temp_path.clone()),
                    host_id: Some(host_id),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(format!("{base} (remote)")), cx);
            id
        });
        let view = self.new_editor_view(cx);
        self.watch_editor(tab_id, &view, cx);
        view.update(cx, |e, cx| {
            e.open_path(std::path::PathBuf::from(&temp_path), cx)
        });
        self.editors.insert(tab_id, view);
        self.remote_edits.insert(
            tab_id,
            RemoteEdit {
                session_id,
                remote_path,
                temp_path,
                dirty: false,
            },
        );
        self.focus_active(window, cx);
        cx.notify();
    }

    // ── SSH connection flow (T07-001) ──────────────────────────────────────

    fn set_host_status(&mut self, host_id: &str, status: HostStatus, cx: &mut Context<Self>) {
        self.host_manager
            .update(cx, |h, cx| h.set_status(host_id, status, cx));
    }

    /// Push a completed [`TabOpResult`] back to a pending MCP `open_tab` /
    /// `close_tab` tool call waiting on its `oneshot` in `modules::mcp::server`.
    fn respond_mcp_tab_op(&self, request_id: String, result: TabOpResult) {
        let mcp = self.backend.mcp.clone();
        self.tokio.spawn(async move {
            let _ = mcp_tab_op_response(request_id, result, &mcp).await;
        });
    }

    /// Handle an MCP `open_tab` request: open a real SSH tab to `host_id`,
    /// auto-grant the bridge access to it (the agent explicitly asked for it),
    /// and answer the pending tool call.
    fn mcp_open_tab(
        &mut self,
        request_id: String,
        host_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let label = match self.host_manager.read(cx).host_name(&host_id) {
            Some(name) => name,
            None => {
                self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some(format!("host '{host_id}' not found")),
                        ..Default::default()
                    },
                );
                return;
            }
        };
        let tab = self
            .connect_host(host_id.clone(), window, cx)
            .and_then(|ssh_id| {
                self.ssh_tabs
                    .values()
                    .find(|t| t.ssh_id == ssh_id)
                    .map(|t| (ssh_id.clone(), t.tab_id))
            });
        let Some((ssh_id, tab_id)) = tab else {
            self.respond_mcp_tab_op(
                request_id,
                TabOpResult {
                    ok: false,
                    error: Some("failed to open tab".to_string()),
                    ..Default::default()
                },
            );
            return;
        };
        let tab_id_str = tab_id.to_string();
        let app = self.backend.clone();
        self.tokio.spawn(async move {
            let _ = mcp_set_session_grant(
                tab_id_str.clone(),
                ssh_id.clone(),
                true,
                label,
                SessionKind::Ssh,
                None,
                Some(host_id),
                app.clone(),
                &app.mcp,
            )
            .await;
            let _ = mcp_tab_op_response(
                request_id,
                TabOpResult {
                    ok: true,
                    session_id: Some(ssh_id),
                    tab_id: Some(tab_id_str),
                    error: None,
                },
                &app.mcp,
            )
            .await;
        });
    }

    /// Handle an MCP `close_tab` request: close the SSH tab whose backend
    /// session matches `session_id` (its grant is revoked by `retire_tab`).
    fn mcp_close_tab(
        &mut self,
        request_id: String,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab_id) = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == session_id)
            .map(|t| t.tab_id)
        else {
            self.respond_mcp_tab_op(
                request_id,
                TabOpResult {
                    ok: false,
                    error: Some("no tab found for that session_id".to_string()),
                    ..Default::default()
                },
            );
            return;
        };
        self.do_close(tab_id, window, cx);
        let result = if self.tabs.read(cx).get(tab_id).is_some() {
            TabOpResult {
                ok: false,
                error: Some("tab could not be closed".to_string()),
                ..Default::default()
            }
        } else {
            TabOpResult {
                ok: true,
                ..Default::default()
            }
        };
        self.respond_mcp_tab_op(request_id, result);
    }

    /// Open an SSH terminal tab for `host_id` and start the connection. Returns
    /// the backend SSH session id (`ssh_id`) of the new tab, or `None` if the
    /// terminal session could not be created.
    fn connect_host(
        &mut self,
        host_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let ssh_id = uuid::Uuid::new_v4().to_string();
        let colors = self.theme_colors(cx);
        let dims = TermDimensions::new(80, 24);

        let writer: RemoteWriter = {
            let (id, st, tk) = (ssh_id.clone(), self.backend.ssh.clone(), self.tokio.clone());
            Arc::new(move |bytes: Vec<u8>| {
                let (id, st) = (id.clone(), st.clone());
                tk.spawn(async move {
                    let _ = labonair_backend::modules::ssh::pty::ssh_pty_write(
                        id,
                        String::from_utf8_lossy(&bytes).into_owned(),
                        &st,
                    )
                    .await;
                });
            })
        };
        let resizer: RemoteResizer = {
            let (id, st, tk) = (ssh_id.clone(), self.backend.ssh.clone(), self.tokio.clone());
            Arc::new(move |cols: u16, rows: u16| {
                let (id, st) = (id.clone(), st.clone());
                tk.spawn(async move {
                    let _ = labonair_backend::modules::ssh::pty::ssh_pty_resize(
                        id,
                        cols as u32,
                        rows as u32,
                        &st,
                    )
                    .await;
                });
            })
        };

        let (session_id, feed) = self.registry.create_remote(colors, dims, writer, resizer);
        let handle = self.registry.handle(session_id)?;
        feed.feed(b"Connecting\xe2\x80\xa6\r\n");

        let pane_id = self.alloc_pane();
        let (host_label, jump_label) = {
            let hm = self.host_manager.read(cx);
            (
                hm.host_name(&host_id).unwrap_or_else(|| host_id.clone()),
                hm.jump_host_label(&host_id),
            )
        };
        let tab_title = ssh_tab_title(&host_label, jump_label.as_deref());
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Workspace,
                TabData {
                    session_id: Some(session_id),
                    host_id: Some(host_id.clone()),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(tab_title.clone()), cx);
            id
        });
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                // SSH panes are not persisted with scrollback (T14-002).
                scrollback_id: uuid::Uuid::new_v4().to_string(),
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.ssh_tabs.insert(
            session_id,
            SshTab {
                ssh_id: ssh_id.clone(),
                host_id: host_id.clone(),
                feed: feed.clone(),
                tab_id,
            },
        );
        self.set_host_status(&host_id, HostStatus::Connecting, cx);
        self.spawn_ssh_connect(ssh_id.clone(), host_id, None, None, feed, cx);
        self.focus_active(window, cx);
        cx.notify();
        Some(ssh_id)
    }

    /// (Re)run `ssh_connect` for `ssh_id`, streaming remote output into `feed`.
    fn spawn_ssh_connect(
        &self,
        ssh_id: String,
        host_id: String,
        passphrase: Option<String>,
        password: Option<String>,
        feed: RemoteFeed,
        cx: &mut Context<Self>,
    ) {
        let app = self.backend.clone();
        let ev_feed = feed.clone();
        let on_event = EventChannel::new(move |ev: SshPtyEvent| {
            match ev {
                SshPtyEvent::Data { data } => ev_feed.feed(data.as_bytes()),
            }
            Ok(())
        });
        let connect_id = ssh_id.clone();
        let jh = self.tokio.spawn(async move {
            ssh_connect(
                connect_id,
                host_id,
                passphrase,
                password,
                Some(80),
                Some(24),
                false,
                on_event,
                &app.ssh,
                &app.trust,
                &app.db,
                &app.secrets,
                app.clone(),
                Some(20),
            )
            .await
            .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(err)) = jh.await {
                let low = err.to_lowercase();
                let expected_prompt = low.contains("passphrase")
                    || low.contains("auth")
                    || low.contains("trust")
                    || low.contains("host key");
                if !expected_prompt {
                    let _ = this.update(cx, |this, cx| {
                        feed.feed(
                            format!("\r\n\x1b[31mConnection failed: {err}\x1b[0m\r\n").as_bytes(),
                        );
                        if let Some(host) = this
                            .ssh_tabs
                            .values()
                            .find(|t| t.ssh_id == ssh_id)
                            .map(|t| t.host_id.clone())
                        {
                            this.set_host_status(&host, HostStatus::Failed, cx);
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Bring up this host's configured local port-forwards on a dedicated
    /// background SSH connection (ref-counted per host by the backend). Mirrors
    /// the reference app's `ssh_start_tunnels` call on `session_established`.
    fn start_tunnels(&self, host_id: &str, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let hid = host_id.to_string();
        let jh = self.tokio.spawn(async move {
            ssh_start_tunnels(
                hid,
                &app.tunnels,
                &app.db,
                &app.secrets,
                &app.trust,
                app.clone(),
                Some(20),
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(e)) = jh.await {
                tracing::warn!(%e, "failed to start SSH tunnels");
            }
            let _ = this.update(cx, |this, cx| this.refresh_active_tunnels(cx));
        })
        .detach();
    }

    /// Push the current set of running forwards into the host manager panel.
    fn refresh_active_tunnels(&self, cx: &mut Context<Self>) {
        let raw = active_tunnels(&self.backend.tunnels);
        let hm = self.host_manager.read(cx);
        let rows: Vec<ActiveTunnelRow> = raw
            .into_iter()
            .map(|t| ActiveTunnelRow {
                host_label: hm.host_name(&t.host_id).unwrap_or(t.host_id),
                local_port: t.local_port,
                remote_host: t.remote_host,
                remote_port: t.remote_port,
            })
            .collect();
        self.host_manager
            .update(cx, |h, cx| h.set_active_tunnels(rows, cx));
    }

    fn retry_ssh(
        &mut self,
        ssh_id: &str,
        passphrase: Option<String>,
        password: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some((host_id, feed)) = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == ssh_id)
            .map(|t| (t.host_id.clone(), t.feed.clone()))
        else {
            return;
        };
        self.spawn_ssh_connect(ssh_id.to_string(), host_id, passphrase, password, feed, cx);
    }

    fn handle_ssh_event(&mut self, ev: AppEvent, cx: &mut Context<Self>) {
        match ev {
            AppEvent::SshKnownHostsWarning {
                session_id,
                fingerprint,
                host,
                is_mismatch,
            } => {
                self.ssh_prompt = Some(SshPrompt::Trust {
                    ssh_id: session_id,
                    host,
                    fingerprint,
                    mismatch: is_mismatch,
                });
            }
            AppEvent::SshAuthRequired {
                session_id,
                prompt_message,
                is_2fa,
            } => {
                self.ssh_prompt = Some(SshPrompt::Password {
                    ssh_id: session_id,
                    message: prompt_message,
                    buffer: String::new(),
                    is_2fa,
                });
            }
            AppEvent::SshPassphraseRequired { session_id } => {
                self.ssh_prompt = Some(SshPrompt::Passphrase {
                    ssh_id: session_id,
                    buffer: String::new(),
                });
            }
            AppEvent::SshSessionEstablished { session_id, .. } => {
                if let Some(host) = self
                    .ssh_tabs
                    .values()
                    .find(|t| t.ssh_id == session_id)
                    .map(|t| t.host_id.clone())
                {
                    self.set_host_status(&host, HostStatus::Connected, cx);
                    self.start_tunnels(&host, cx);
                }
                if self.ssh_prompt.as_ref().map(|p| p.ssh_id()) == Some(session_id.as_str()) {
                    self.ssh_prompt = None;
                }
                if let Some(command) = self.pending_snippet_ssh.remove(&session_id) {
                    if let Some(sid) = self
                        .ssh_tabs
                        .iter()
                        .find(|(_, t)| t.ssh_id == session_id)
                        .map(|(sid, _)| *sid)
                    {
                        if let Some(handle) = self.registry.handle(sid) {
                            let _ = handle.write(format!("{}\n", command.trim_end()).as_bytes());
                        }
                    }
                }
            }
            AppEvent::SshConnectionLost { session_id } => {
                if let Some(host) =
                    self.ssh_tabs
                        .values()
                        .find(|t| t.ssh_id == session_id)
                        .map(|t| {
                            t.feed.mark_disconnected();
                            t.host_id.clone()
                        })
                {
                    self.set_host_status(&host, HostStatus::Failed, cx);
                }
            }
            AppEvent::McpOpenTabRequest {
                request_id,
                host_id,
                ..
            } => match host_id {
                Some(host_id) => self.pending_mcp.push(McpTabOp::Open {
                    request_id,
                    host_id,
                }),
                None => self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some("open_tab requires a host_id".to_string()),
                        ..Default::default()
                    },
                ),
            },
            AppEvent::McpCloseTabRequest {
                request_id,
                session_id,
            } => match session_id {
                Some(session_id) => self.pending_mcp.push(McpTabOp::Close {
                    request_id,
                    session_id,
                }),
                None => self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some("close_tab requires a session_id".to_string()),
                        ..Default::default()
                    },
                ),
            },
            AppEvent::McpServerError { message } => {
                let center = crate::notifications::notification_center(cx);
                center.update(cx, |c, cx| {
                    c.push_action_result(
                        crate::notifications::Notification::error(
                            "AI Agent Bridge failed to start",
                            message,
                        ),
                        cx,
                    );
                });
            }
            AppEvent::McpGrantExpired { tab_id } => {
                // Auto-revoke sweep or a host's "Block AI Agent Access" flag
                // being switched on: Rust already dropped the grant, clear the
                // local mirror so the badge / context-menu checkbox catch up.
                if let Ok(id) = tab_id.parse::<u64>() {
                    self.agent_access.update(cx, |s, cx| s.clear_local(id, cx));
                }
            }
            AppEvent::McpActivity {
                label,
                action,
                detail,
            } => {
                tracing::debug!(%label, %action, %detail, "mcp agent activity");
                if self.agent_access.read(cx).notify_on_activity() {
                    let center = crate::notifications::notification_center(cx);
                    center.update(cx, |c, cx| {
                        c.push(
                            crate::notifications::Notification::info(
                                format!("Agent: {action} \u{2014} {label}"),
                                detail.clone(),
                            ),
                            cx,
                        );
                    });
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn submit_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(prompt) = self.ssh_prompt.take() else {
            return;
        };
        match prompt {
            SshPrompt::Trust { ssh_id, .. } => {
                let app = self.backend.clone();
                self.tokio.spawn(async move {
                    let _ = ssh_trust_host(ssh_id, true, &app.trust).await;
                });
            }
            SshPrompt::Password { ssh_id, buffer, .. } => {
                self.retry_ssh(&ssh_id, None, Some(buffer), cx);
            }
            SshPrompt::Passphrase { ssh_id, buffer } => {
                self.retry_ssh(&ssh_id, Some(buffer), None, cx);
            }
        }
        cx.notify();
    }

    fn cancel_prompt(&mut self, cx: &mut Context<Self>) {
        if let Some(SshPrompt::Trust { ssh_id, .. }) = self.ssh_prompt.take() {
            let app = self.backend.clone();
            self.tokio.spawn(async move {
                let _ = ssh_trust_host(ssh_id, false, &app.trust).await;
            });
        }
        cx.notify();
    }

    fn on_prompt_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cancel_prompt(cx),
            "enter" => self.submit_prompt(cx),
            "backspace" => {
                if let Some(
                    SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. },
                ) = self.ssh_prompt.as_mut()
                {
                    buffer.pop();
                }
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let (
                    Some(ch),
                    Some(SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. }),
                ) = (ch, self.ssh_prompt.as_mut())
                {
                    buffer.push_str(&ch);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn render_ssh_prompt(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border, accent, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let (title, body, ok_label): (String, String, &str) = match self.ssh_prompt.as_ref() {
            Some(SshPrompt::Trust {
                host,
                fingerprint,
                mismatch,
                ..
            }) => (
                if *mismatch {
                    "\u{26a0} Host key CHANGED".to_string()
                } else {
                    "Unknown host key".to_string()
                },
                format!(
                    "{host}\nFingerprint: {fingerprint}\n\n{}",
                    if *mismatch {
                        "The key differs from the one on record. Only continue if you know why."
                    } else {
                        "This host is not yet in known_hosts."
                    }
                ),
                "Trust & Continue",
            ),
            Some(SshPrompt::Password {
                message,
                buffer,
                is_2fa,
                ..
            }) => (
                if *is_2fa {
                    "Two-factor code".to_string()
                } else {
                    "Password required".to_string()
                },
                format!("{message}\n{}", "\u{2022}".repeat(buffer.chars().count())),
                "Submit",
            ),
            Some(SshPrompt::Passphrase { buffer, .. }) => (
                "Key passphrase".to_string(),
                format!(
                    "Enter the passphrase for the private key.\n{}",
                    "\u{2022}".repeat(buffer.chars().count())
                ),
                "Submit",
            ),
            None => return div(),
        };

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                div()
                    .track_focus(&self.prompt_focus)
                    .key_context("SshPrompt")
                    .on_key_down(cx.listener(Self::on_prompt_key))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(420.0))
                    .p_4()
                    .rounded_lg()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(fg)
                    .child(div().text_sm().child(SharedString::from(title)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted)
                            .whitespace_normal()
                            .child(SharedString::from(body)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                div()
                                    .id("ssh-prompt-cancel")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .text_color(muted)
                                    .hover(|s| s.bg(border).text_color(fg))
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.cancel_prompt(cx)
                                    })),
                            )
                            .child(
                                div()
                                    .id("ssh-prompt-ok")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(accent)
                                    .text_color(fg)
                                    .hover(|s| s.opacity(0.85))
                                    .child(SharedString::from(ok_label.to_string()))
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.submit_prompt(cx)
                                    })),
                            ),
                    ),
            )
    }

    fn sync_meta(&mut self, cx: &mut Context<Self>) {
        let updates: Vec<(u64, Option<String>, Option<String>)> = self
            .layouts
            .iter()
            .filter_map(|(tab_id, layout)| {
                let v = self.panes.get(&layout.active)?.view.read(cx);
                Some((*tab_id, v.cwd(), v.shell_title()))
            })
            .collect();
        self.tabs.update(cx, |store, cx| {
            for (id, cwd, title) in updates {
                store.sync_workspace_meta(id, cwd, title, cx);
            }
        });
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_tab(&self, tab: &Tab, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, accent, border) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.accent(),
            theme.border(),
        );
        let id = tab.id;
        let active = self.tabs.read(cx).active_id() == id;
        let total = self.tabs.read(cx).len();
        let closable = total > 1 && tab.kind != TabKind::Home;
        let label = SharedString::from(tab.label());

        // D4 — tab-entrance animation (`@keyframes labonair-tab-in` in the
        // reference `globals.css`: fade + `scale(0.86) → 1` over `--dur-base`
        // with `--ease-premium`). GPUI 0.2.2 `Div` can't take a scale
        // transform, so we animate opacity only. Reduce-motion mirrors the
        // reference's `animation-duration: 0.01ms` clamp rather than dropping
        // the keyframe entirely.
        let (ease, dur_base) = {
            let a = self.theme.read(cx).animation();
            (a.ease_premium, a.dur_base)
        };
        let reduce_motion = cx
            .try_global::<GlobalPreferences>()
            .map(|p| p.0.reduce_motion)
            .unwrap_or(false);
        let tab_in_dur = if reduce_motion {
            Duration::from_micros(10)
        } else {
            dur_base
        };

        let close_btn = div()
            .id(("tab-close", id))
            .px_1()
            .rounded_sm()
            .text_color(muted)
            .hover(|s| s.bg(border).text_color(fg))
            .child("\u{2715}")
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                cx.stop_propagation();
                this.request_close(id, window, cx);
            }));

        div()
            .id(("tab", id))
            .flex()
            .items_center()
            .gap_1p5()
            .h(px(28.0))
            .px_2()
            .rounded_md()
            .text_xs()
            .whitespace_nowrap()
            .cursor_pointer()
            .text_color(if active { fg } else { muted })
            .when(active, |d| d.bg(accent))
            .when(!active, |d| d.hover(|s| s.bg(border)))
            .child(div().text_color(muted).child(tab.kind.indicator()))
            .child(
                div()
                    .max_w(px(180.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(tab.kind == TabKind::Editor && tab.peek, |d| d.italic())
                    .child(label.clone()),
            )
            .when(tab.kind == TabKind::Editor && tab.dirty, |d| {
                d.child(div().size(px(6.0)).rounded_full().bg(fg).opacity(0.7))
            })
            .when(closable, |d| d.child(close_btn))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.select_tab(id, window, cx);
            }))
            .when(closable, |d| {
                d.on_mouse_down(
                    MouseButton::Middle,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        this.request_close(id, window, cx);
                    }),
                )
            })
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                    this.context_menu = Some((id, ev.position));
                    cx.notify();
                }),
            )
            .on_drag(
                DraggedTab {
                    id,
                    label: label.clone(),
                },
                |dragged, _, _, cx| {
                    cx.new(|_| TabDragPreview {
                        label: dragged.label.clone(),
                    })
                },
            )
            .drag_over::<DraggedTab>(move |style, _, _, _| style.border_l_2().border_color(fg))
            .on_drop(cx.listener(move |this, dragged: &DraggedTab, _window, cx| {
                this.tabs.update(cx, |s, cx| s.reorder(dragged.id, id, cx));
            }))
            .with_animation(
                ("tab-in", id),
                Animation::new(tab_in_dur).with_easing(move |t| ease.eval(t)),
                |el, delta| el.opacity(delta),
            )
    }

    fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, muted, fg, border) = (
            theme.background(),
            theme.muted_foreground(),
            theme.foreground(),
            theme.border(),
        );
        let tabs = self.tabs.read(cx).tabs().to_vec();

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(36.0))
            .w_full()
            .flex_shrink_0()
            .px_2()
            .bg(bg)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .id("tab-strip")
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .min_w_0()
                    .overflow_x_scroll()
                    .children(tabs.iter().map(|t| self.render_tab(t, cx))),
            )
            .child(
                div()
                    .id("tab-new")
                    .flex_shrink_0()
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("+")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.open_terminal_tab(window, cx);
                    })),
            )
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active = self.tabs.read(cx).active().cloned();
        let Some(active) = active else {
            return div().size_full().into_any_element();
        };

        match active.kind {
            TabKind::Home => self.host_manager.clone().into_any_element(),
            TabKind::Workspace => {
                if let Some(layout) = self.layouts.get(&active.id).cloned() {
                    let multi = layout.len() > 1;
                    div()
                        .size_full()
                        .child(self.render_pane_node(&layout.root, layout.active, multi, cx))
                        .into_any_element()
                } else {
                    self.placeholder("Terminal", cx).into_any_element()
                }
            }
            TabKind::Editor => {
                if let Some(view) = self.editors.get(&active.id) {
                    view.clone().into_any_element()
                } else {
                    self.placeholder("Editor", cx).into_any_element()
                }
            }
            TabKind::Sftp => {
                if let Some(view) = self.sftp_views.get(&active.id) {
                    view.clone().into_any_element()
                } else {
                    self.placeholder("SFTP", cx).into_any_element()
                }
            }
            other => self
                .placeholder(other.default_title(), cx)
                .into_any_element(),
        }
    }

    fn render_pane_node(
        &mut self,
        node: &PaneNode,
        active_pane: PaneId,
        multi: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme.read(cx);
        let (bg, border, accent) = (theme.background(), theme.border(), theme.accent());

        match node {
            PaneNode::Pane { id } => {
                let id = *id;
                let is_active = id == active_pane;
                let content: gpui::AnyElement = match self.panes.get(&id) {
                    Some(entry) => entry.view.clone().into_any_element(),
                    None => div().size_full().into_any_element(),
                };
                div()
                    .id(("pane", id))
                    .relative()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .overflow_hidden()
                    .border_1()
                    .border_color(if multi && is_active { accent } else { bg })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                            this.set_pane_active(id, window, cx);
                        }),
                    )
                    .child(content)
                    .into_any_element()
            }
            PaneNode::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let row = *axis == SplitAxis::Horizontal;
                let ratio = *ratio;
                let first_el = self.render_pane_node(first, active_pane, multi, cx);
                let second_el = self.render_pane_node(second, active_pane, multi, cx);

                let handle = div()
                    .id(("split", split_id))
                    .flex_shrink_0()
                    .bg(border)
                    .hover(|s| s.bg(accent))
                    .when(row, |d| d.w(px(HANDLE)).h_full().cursor_col_resize())
                    .when(!row, |d| d.h(px(HANDLE)).w_full().cursor_row_resize())
                    .on_drag(PaneResize { split_id }, |_, _, _, cx| cx.new(|_| DragGhost))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, _window, cx| {
                        if ev.click_count() >= 2 {
                            this.reset_split(split_id, cx);
                        }
                    }));

                div()
                    .flex()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .when(row, |d| d.flex_row())
                    .when(!row, |d| d.flex_col())
                    .child(
                        div()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .flex_basis(relative(ratio))
                            .child(first_el),
                    )
                    .child(handle)
                    .child(
                        div()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .flex_grow()
                            .flex_basis(relative(1.0 - ratio))
                            .child(second_el),
                    )
                    .on_drag_move(cx.listener(
                        move |this, ev: &DragMoveEvent<PaneResize>, _window, cx| {
                            if ev.drag(cx).split_id != split_id {
                                return;
                            }
                            let b = ev.bounds;
                            let p = ev.event.position;
                            let frac = if row {
                                f32::from(p.x - b.origin.x) / f32::from(b.size.width).max(1.0)
                            } else {
                                f32::from(p.y - b.origin.y) / f32::from(b.size.height).max(1.0)
                            };
                            this.resize_split(split_id, frac, cx);
                        },
                    ))
                    .into_any_element()
            }
        }
    }

    fn placeholder(&self, title: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background())
            .text_color(theme.muted_foreground())
            .text_sm()
            .child(SharedString::from(format!(
                "{title} — coming in a later phase"
            )))
    }

    fn render_confirm(&mut self, id: u64, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border, accent, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let label = self
            .tabs
            .read(cx)
            .get(id)
            .map(Tab::label)
            .unwrap_or_default();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .rounded_lg()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(fg)
                    .child(SharedString::from(format!(
                        "Discard unsaved changes to \u{201c}{label}\u{201d}?"
                    )))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                div()
                                    .id("confirm-cancel")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .text_color(muted)
                                    .hover(|s| s.bg(border).text_color(fg))
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.confirm_close = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("confirm-discard")
                                    .px_3()
                                    .py_1()
                                    .rounded_md()
                                    .bg(accent)
                                    .text_color(fg)
                                    .hover(|s| s.opacity(0.85))
                                    .child("Discard")
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, window, cx| {
                                            this.do_close(id, window, cx);
                                        },
                                    )),
                            ),
                    ),
            )
    }

    fn render_context_menu(
        &mut self,
        id: u64,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.muted_foreground(),
        );
        let kind = self.tabs.read(cx).get(id).map(|t| t.kind);
        let grant_target = self.mcp_grant_target(id, cx);
        let is_granted = self.agent_access.read(cx).is_granted(id);

        let item = |label: &str, key: &'static str| {
            div()
                .id(key)
                .px_3()
                .py_1()
                .text_xs()
                .rounded_sm()
                .text_color(fg)
                .hover(|s| s.bg(border))
                .child(SharedString::from(label.to_string()))
        };

        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _w, cx| {
                    this.context_menu = None;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(pos.x)
                    .top(pos.y)
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .p_1()
                    .min_w(px(160.0))
                    .rounded_md()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(muted)
                    .child(item("Close", "close").on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.context_menu = None;
                            this.request_close(id, window, cx);
                        },
                    )))
                    .child(item("Close Others", "others").on_click(cx.listener(
                        move |this, _: &ClickEvent, window, cx| {
                            this.context_menu = None;
                            this.close_others(id, window, cx);
                        },
                    )))
                    .when_some(kind, |el, kind| {
                        el.child(item("Close All Of This Type", "kind").on_click(cx.listener(
                            move |this, _: &ClickEvent, window, cx| {
                                this.context_menu = None;
                                this.close_by_kind(kind, window, cx);
                            },
                        )))
                    })
                    .when_some(
                        grant_target,
                        |el, (session_id, label, gkind, host_id, pty)| {
                            let text = if is_granted {
                                "\u{2713} Grant AI Agent Access"
                            } else {
                                "Grant AI Agent Access"
                            };
                            el.child(item(text, "mcp-grant").on_click(cx.listener(
                                move |this, _: &ClickEvent, _window, cx| {
                                    this.context_menu = None;
                                    let (session_id, label, host_id) =
                                        (session_id.clone(), label.clone(), host_id.clone());
                                    this.agent_access.update(cx, |s, cx| {
                                        s.set_grant(
                                            id,
                                            session_id,
                                            !is_granted,
                                            label,
                                            gkind,
                                            host_id,
                                            pty,
                                            cx,
                                        );
                                    });
                                    cx.notify();
                                },
                            )))
                        },
                    ),
            )
    }

    /// Resolve the MCP agent-access grant target for a tab, when it's an SSH or
    /// local terminal tab and the bridge is enabled.
    fn mcp_grant_target(&self, tab_id: u64, cx: &mut Context<Self>) -> Option<McpGrantTarget> {
        if !self.agent_access.read(cx).bridge_enabled() {
            return None;
        }
        let tab = self.tabs.read(cx).get(tab_id)?.clone();
        let label = tab.label();
        if let Some(ssh) = self.ssh_tabs.values().find(|t| t.tab_id == tab_id) {
            return Some((
                ssh.ssh_id.clone(),
                label,
                SessionKind::Ssh,
                Some(ssh.host_id.clone()),
                None,
            ));
        }
        if tab.kind == TabKind::Workspace {
            if let Some(sid) = tab.data.session_id {
                return Some((
                    String::new(),
                    label,
                    SessionKind::Local,
                    None,
                    Some(sid as u32),
                ));
            }
        }
        None
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Drain host-manager connect requests here — `connect_host` needs a
        // `&mut Window` and `cx.subscribe` does not provide one.
        for host_id in std::mem::take(&mut self.pending_connect) {
            let _ = self.connect_host(host_id, window, cx);
        }
        for op in std::mem::take(&mut self.pending_mcp) {
            match op {
                McpTabOp::Open {
                    request_id,
                    host_id,
                } => self.mcp_open_tab(request_id, host_id, window, cx),
                McpTabOp::Close {
                    request_id,
                    session_id,
                } => self.mcp_close_tab(request_id, session_id, window, cx),
            }
        }
        for host_id in std::mem::take(&mut self.pending_sftp) {
            self.open_sftp(host_id, window, cx);
        }
        for open in std::mem::take(&mut self.pending_open) {
            match open {
                PendingOpen::Local(path) => self.open_file(path, false, window, cx),
                PendingOpen::RemoteEdit {
                    session_id,
                    remote_path,
                    host_id,
                    temp_path,
                } => self.open_remote_edit(session_id, remote_path, host_id, temp_path, window, cx),
            }
        }
        let want_prompt = self.ssh_prompt.is_some();
        if want_prompt && !self.prompt_shown {
            window.focus(&self.prompt_focus);
        }
        self.prompt_shown = want_prompt;

        let bg = self.theme.read(cx).background();
        let tab_bar = self.render_tab_bar(cx);
        let content = self.render_content(cx);
        let confirm = self
            .confirm_close
            .map(|id| self.render_confirm(id, cx).into_any_element());
        let context_menu = self
            .context_menu
            .map(|(id, pos)| self.render_context_menu(id, pos, cx).into_any_element());
        let ssh_prompt = self
            .ssh_prompt
            .is_some()
            .then(|| self.render_ssh_prompt(cx).into_any_element());

        div()
            .track_focus(&self.focus_handle)
            .key_context("Workspace")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .on_key_down(cx.listener(Self::on_key_down))
            .child(tab_bar)
            .child(div().flex_1().min_h_0().child(content))
            .children(confirm)
            .children(context_menu)
            .children(ssh_prompt)
            .child(self.transfers.clone())
    }
}

impl Workspace {
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        // Cmd-based shortcuts (full configurability lands in Phase 12).
        if !m.platform || m.control || m.alt {
            return;
        }
        // Cmd-T / Cmd-W / Cmd-D / Cmd-Shift-D and tab cycling are GPUI actions
        // now (see `crate::menu`), bound so the native menu shares the path.
        match (m.shift, ks.key.as_str()) {
            (true, "]") | (false, "}") => {
                self.cycle_tab(true, window, cx);
                cx.stop_propagation();
            }
            (true, "[") | (false, "{") => {
                self.cycle_tab(false, window, cx);
                cx.stop_propagation();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ssh_tab_title;

    #[test]
    fn ssh_tab_title_annotates_jump_route() {
        assert_eq!(ssh_tab_title("prod-web", None), "SSH \u{00b7} prod-web");
        assert_eq!(
            ssh_tab_title("prod-web", Some("bastion")),
            "SSH \u{00b7} prod-web  \u{2933} bastion"
        );
    }
}
