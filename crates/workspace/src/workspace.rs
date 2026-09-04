//! Workspace: tab bar + split-pane content (T04-001, T04-002).
//!
//! [`Workspace`] owns the [`TabStore`], the shared [`TerminalRegistry`], the
//! per-workspace-tab [`WorkspaceLayout`] (its split-pane tree) and the content
//! view for every open pane. It renders the tab bar over the active tab's
//! split-pane tree; the window chrome around it (header, sidebar, status bar,
//! native titlebar) is composed by `AppShell` (in `labonair-ui`) (T04-003),
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
//!
//! Crate root (`labonair-workspace`, extracted in T16-006). Besides [`Workspace`]
//! it hosts the pane tree, session snapshot/replay, the tab store, the AI live
//! bridge, the per-workspace stores (`agent_access`, `background`) and every
//! tab-content view under [`views`]. `hosts`/`transfers` are acknowledged
//! temporary residents here — `Workspace` owns those view entities today; long
//! term `hosts` becomes `labonair-panel-hosts` (T16-008) and `transfers` moves
//! to `labonair-shell`.

pub mod agent_access;
pub mod backend_event_bridge;
pub mod background;
pub mod bell;
pub mod dock;
pub mod drag;
pub mod live_bridge;
pub mod markdown;
pub mod modal_layer;
pub mod pane;
pub mod pane_group;
pub mod prefs;
pub mod search_overlay;
pub mod session;
pub mod status_bar;
pub mod status_placements;
pub mod syntax_theme;
pub mod tabs;
pub mod toast_layer;
pub mod transfers;
pub mod views;

/// Re-export shim so `crate::theme::…` paths in the moved modules keep
/// resolving. The runtime theme store lives in `labonair_theme::store`
/// (relocated in T16-006).
pub mod theme {
    pub use labonair_theme::store::*;
}

gpui::actions!(
    labonair,
    [
        // "Ask about Selection" — dispatched from the terminal view's selection
        // context menu; handled by `menu` / `AppShell` in `crates/ui`. Defined
        // here (not in `labonair_ui::menu`) so `views::terminal` can name it
        // without a dependency cycle (T16-006).
        AskAboutSelection,
    ]
);

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::agent_access::AgentAccessStore;
use crate::backend_event_bridge::BackendEventBridge;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, point, px, relative, Animation, AnimationExt, App, AppContext, ClickEvent, Context,
    DragMoveEvent, Entity, ExternalPaths, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Task, Window,
};
use labonair_backend::modules::mcp::{
    mcp_set_session_grant, mcp_tab_op_response, SessionKind, TabOpResult,
};
use labonair_backend::modules::scrollback::{
    scrollback_cleanup, scrollback_delete, scrollback_load, scrollback_save,
};
use labonair_backend::modules::settings::preferences::CursorStyle as PrefCursorStyle;
use labonair_backend::modules::settings::preferences::StartupTab;
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
use crate::dock::{position_slug, RESIZE_HANDLE_SIZE};
use crate::live_bridge::LiveCommand;
use crate::pane::{CloseOutcome, Member, PaneId, SplitAxis, SplitDirection, WorkspaceLayout};
use crate::prefs::GlobalPreferences;
use crate::session::{
    plan_restore, PaneSessionKind, PaneSessionSnapshot, RestoreAction, RestoreResult,
    SerializedLayout, SessionSnapshot, TabSnapshot, WorkspaceTabSnapshot,
};
use crate::tabs::{Tab, TabData, TabKind, TabStore};
use crate::theme::ThemeStore;
use crate::transfers::{TransferBusEvent, TransfersEvent, TransfersView};
use crate::views::editor::{EditorEvent, EditorView};
use crate::views::preview::PreviewView;
use crate::views::sftp::{SftpEvent, SftpView};
use crate::views::terminal::TerminalView;
use labonair_hosts_ui::ssh_connection::{
    ConnStage, ConnectionKind, ConnectionState, ConnectionStatusStore, StageStatus,
};
use labonair_hosts_ui::{ActiveTunnelRow, HostManagerEvent, HostManagerView, HostStatus};
use labonair_panel_git_graph::GitGraphView;
use labonair_ui_kit::{context_menu, IconName, MenuItem};

/// Interval for draining backend SSH events into the workspace.
const SSH_POLL_INTERVAL: Duration = Duration::from_millis(40);

/// Which surface the `Cmd+F` search overlay (T18-002) drives for the active tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    /// A code editor tab — literal in-buffer search.
    Editor,
    /// A terminal pane — literal scrollback search.
    Terminal,
    /// SFTP list, git graph, host manager, … — search is not offered.
    Unavailable,
}

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
        Some(j) => format!("SSH \u{00b7} {host_label}  \u{2192} {j}"),
        None => format!("SSH \u{00b7} {host_label}"),
    }
}

/// Small action button for the SSH loading screen (pill radius, matching the
/// reference button treatment). Callers add `.on_click(..)`.
fn loading_btn(
    id: &'static str,
    label: &'static str,
    tint: gpui::Hsla,
    border: gpui::Hsla,
    primary: bool,
    fg: gpui::Hsla,
) -> gpui::Stateful<gpui::Div> {
    let base = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .px_3()
        .py_1()
        .rounded(px(13.0))
        .border_1()
        .text_xs()
        .cursor_pointer()
        .child(label);
    if primary {
        base.bg(tint)
            .text_color(fg)
            .border_color(gpui::transparent_black())
            .hover(|s| s.opacity(0.85))
    } else {
        base.text_color(tint)
            .border_color(border)
            .hover(|s| s.bg(border).text_color(fg))
    }
}

/// A blocking prompt raised mid-connect by the SSH backend.
enum SshPrompt {
    Trust { ssh_id: String },
    Password { ssh_id: String, buffer: String },
    Passphrase { ssh_id: String, buffer: String },
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
/// Height of `AppShell`'s overlay titlebar. The tab strip now lives inside it,
/// so tab-menu / new-tab-menu anchors (captured in window coords) are shifted
/// up by this much to land in the `Workspace`'s own coordinate space.
const TITLEBAR_OFFSET: f32 = 40.0;

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

/// Value carried while dragging a split divider: which axis, and which
/// boundary within it (between members `boundary` and `boundary + 1`).
struct PaneResize {
    axis_id: PaneId,
    boundary: usize,
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

/// Persists the serialized dock layout (installed by the shell — see
/// [`Workspace::set_dock_persist_hook`]).
type DockPersistHook = Arc<dyn Fn(String, &mut App) + Send + Sync>;

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
    /// Native preview view per `Preview` tab id (T15-006 — WebView replacement).
    previews: HashMap<u64, Entity<PreviewView>>,
    /// The shared commit-graph view, lazily created for the `GitGraph` tab.
    git_graph: Option<Entity<GitGraphView>>,
    /// Panel-type registry (T17-001) — populated once by
    /// `labonair_shell::register_builtin_panels`. The shell's dock rendering
    /// and status-bar toggles read it instead of a hard-coded `enum`. T17-002
    /// replaces the shell's ad-hoc per-side slot state with a real `Dock` that
    /// will also hang off the `Workspace`.
    panel_registry: labonair_panel::PanelRegistry,
    /// Status-bar item registry (T17-003) — populated once by
    /// `labonair_shell::register_builtin_status_items`. The `StatusBar`
    /// component reads it instead of a hard-coded `render_bar_item` match.
    status_item_registry: labonair_panel::StatusItemRegistry,
    /// Set once by the shell: persists the serialized `[DockData; 3]` layout
    /// (the shell owns the `PreferencesStore`, which this crate cannot depend
    /// on — hence the callback indirection). T17-003 moved this off `AppShell`.
    dock_persist_hook: Option<DockPersistHook>,
    /// Debounce for [`Workspace::persist_docks`].
    last_dock_save: Option<std::time::Instant>,
    /// The three edge docks (T17-002). Empty at construction; populated by
    /// [`Workspace::init_docks`] once the shell has registered the builtin
    /// panels. Replaces the shell's former ad-hoc `left_slot`/`right_slot`.
    left_dock: crate::dock::Dock,
    right_dock: crate::dock::Dock,
    bottom_dock: crate::dock::Dock,
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
    /// Anchor position of the open "+" new-tab dropdown, if any.
    new_tab_menu: Option<gpui::Point<gpui::Pixels>>,
    /// Tab whose title is being edited inline: `(tab id, buffer)`.
    rename_tab: Option<(u64, String)>,
    rename_focus: FocusHandle,
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
    /// Per-session connection state machine + stage progress + live log,
    /// driving the full-pane `SshLoadingScreen` (T16-015).
    ssh_connection: Entity<ConnectionStatusStore>,
    /// Tab ids the loading screen asked to close (needs `&mut Window`).
    pending_tab_close: Vec<u64>,
    /// Backend event bus → UI bridge (T17-008): decodes `AppEvent` /
    /// `TransferBusEvent` off `backend.events` on the GPUI foreground and pushes
    /// them straight into this entity — event-driven, no poll drain.
    _backend_event_bridge: Entity<BackendEventBridge>,
    /// Periodic tunnel-liveness refresh (state poll, not event-driven).
    _ssh_poll: Task<()>,
    /// Periodic session-snapshot writer (T14-001) — covers force-quit; the
    /// window-close hook in `AppShell` covers the normal path.
    _session_save: Task<()>,

    // ── SFTP transfers (T08-002) ──────────────────────────────────────────
    transfers: Entity<TransfersView>,

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

        // Backend event bus → UI (T17-008): one foreground subscription that
        // decodes each raw event and pushes it straight into this entity. No
        // `tokio::spawn` + `mpsc` + poll-drain hop.
        let backend_event_bridge = {
            let workspace = cx.entity().downgrade();
            let backend = backend.clone();
            cx.new(|cx| BackendEventBridge::new(backend, workspace, cx))
        };

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
                .update(cx, |this, cx| this.refresh_active_tunnels(cx))
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
            previews: HashMap::new(),
            git_graph: None,
            panel_registry: labonair_panel::PanelRegistry::new(),
            status_item_registry: labonair_panel::StatusItemRegistry::new(),
            dock_persist_hook: None,
            last_dock_save: None,
            left_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Left),
            right_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Right),
            bottom_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Bottom),
            sftp_sessions: HashMap::new(),
            remote_edits: HashMap::new(),
            pending_sftp: Vec::new(),
            pending_open: Vec::new(),
            next_pane_id: 1,
            confirm_close: None,
            context_menu: None,
            new_tab_menu: None,
            rename_tab: None,
            rename_focus: cx.focus_handle(),
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
            ssh_connection: cx.new(|_| ConnectionStatusStore::new()),
            pending_tab_close: Vec::new(),
            _backend_event_bridge: backend_event_bridge,
            _ssh_poll: ssh_poll,
            transfers,
            pending_mcp: Vec::new(),
            agent_access,
            pending_snippet_ssh: HashMap::new(),
        };
        cx.observe(&this.agent_access, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&this.ssh_connection, |_, _, cx| cx.notify())
            .detach();

        // Startup (T14-001 / T17-009). A passed-in snapshot means session
        // restore is on *and* a snapshot was found — replay it verbatim, even
        // if it restores zero tabs (an empty workspace is a valid state). With
        // no snapshot, honour the `startup_tab` preference: `terminal` opens
        // one local terminal, `empty` opens nothing. There is no automatic
        // host-manager tab any more.
        match restore {
            Some(snap) => {
                let result = this.restore_session(&snap, window, cx);
                for (title, reason) in &result.failed {
                    tracing::warn!(title, reason, "session tab not restored");
                }
            }
            None => {
                let startup = cx
                    .try_global::<GlobalPreferences>()
                    .map(|g| g.0.default_startup_tab)
                    .unwrap_or_default();
                match startup {
                    StartupTab::Terminal => this.open_terminal_tab(window, cx),
                    StartupTab::Empty => {}
                }
            }
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
                // The host-manager tab is an on-demand helper (removed in
                // T19-010) — not persisted, like the transient diff kinds.
                TabKind::Hosts
                | TabKind::AiDiff
                | TabKind::GitGraph
                | TabKind::GitDiff
                | TabKind::CommitDiff => None,
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
            layout: SerializedLayout::from_layout(&layout),
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
                // Legacy snapshots (pre-T17-009) carried a "home" tab. That
                // tab kind no longer exists — skip the slot silently, as if it
                // had held nothing.
                RestoreAction::Home => None,
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
                RestoreAction::Preview { url } => {
                    self.open_preview(url, window, cx);
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
        let leaves = layout.leaves();
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

    /// The file path of the active editor tab, if the active tab is an editor
    /// with a saved file — feeds the breadcrumb's "file mode" + the
    /// `cursorPosition` bar item.
    pub fn active_file_path(&self, cx: &App) -> Option<String> {
        let active = self.tabs.read(cx).active()?.clone();
        if active.kind != TabKind::Editor {
            return None;
        }
        self.editors
            .get(&active.id)?
            .read(cx)
            .path()
            .map(|p| p.to_string_lossy().into_owned())
    }

    /// 1-based `(line, column)` of the active editor's caret, if an editor tab
    /// is active.
    pub fn active_editor_cursor(&self, cx: &App) -> Option<(usize, usize)> {
        let active = self.tabs.read(cx).active()?.clone();
        if active.kind != TabKind::Editor {
            return None;
        }
        Some(self.editors.get(&active.id)?.read(cx).cursor_line_col())
    }

    /// Document symbols of the active editor tab (palette "Go to Symbol").
    pub fn active_editor_symbols(&self, cx: &App) -> Vec<labonair_editor::DocumentSymbol> {
        let Some(active) = self.tabs.read(cx).active().cloned() else {
            return Vec::new();
        };
        if active.kind != TabKind::Editor {
            return Vec::new();
        }
        self.editors
            .get(&active.id)
            .map(|e| e.read(cx).document_symbols())
            .unwrap_or_default()
    }

    /// Jump the active editor's caret to `line0` (0-based).
    pub fn active_editor_goto_line(&self, line0: usize, cx: &mut Context<Self>) {
        let Some(active) = self.tabs.read(cx).active().cloned() else {
            return;
        };
        if active.kind != TabKind::Editor {
            return;
        }
        if let Some(e) = self.editors.get(&active.id) {
            e.update(cx, |e, cx| e.goto_line(line0, cx));
        }
    }

    /// `{hostId, sessionId}` of the SSH session backing the active pane, when
    /// it's remote — lets the breadcrumb browse through the same session.
    pub fn active_remote_target(&self, cx: &App) -> Option<(String, String)> {
        let active = self.tabs.read(cx).active()?.clone();
        let host_id = active.data.host_id.clone()?;
        let session_id = active.data.session_id.map(|s| s.to_string())?;
        Some((host_id, session_id))
    }

    /// Send a `cd <path>` into the active pane's shell (breadcrumb navigation).
    pub fn send_cd(&self, path: &str, cx: &App) {
        let cmd = format!("cd {}\n", shell_quote(path));
        self.inject_into_active_terminal(&cmd, cx);
    }

    /// Open the SFTP transfer-queue panel (statusbar `transfers` bar item).
    pub fn reveal_transfers(&self, cx: &mut Context<Self>) {
        self.transfers.update(cx, |t, cx| t.reveal(cx));
    }

    /// The transfer-queue view entity, so the shell can observe it directly
    /// for the `transfers` statusbar item (T18-004).
    pub fn transfers_entity(&self) -> Entity<TransfersView> {
        self.transfers.clone()
    }

    /// Open a new local terminal tab rooted at `path` (breadcrumb "open in new
    /// terminal").
    pub fn cd_in_new_tab(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.run_snippet_local(Some(path), String::new(), window, cx);
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

    /// Which surface the `Cmd+F` search overlay (T18-002) targets for the
    /// active tab.
    pub fn active_search_target(&self, cx: &App) -> SearchTarget {
        let id = self.tabs.read(cx).active_id();
        if self.editors.contains_key(&id) {
            SearchTarget::Editor
        } else if self.active_pane_view(cx).is_some() {
            SearchTarget::Terminal
        } else {
            SearchTarget::Unavailable
        }
    }

    /// One-line pre-fill for the search overlay (editor selection only).
    pub fn search_seed(&self, cx: &App) -> Option<String> {
        let id = self.tabs.read(cx).active_id();
        self.editors.get(&id).and_then(|e| e.read(cx).search_seed())
    }

    /// Start / update the search against whichever surface the active tab owns.
    /// Returns `(current_1_based, total)`; `None` when the tab is not searchable.
    pub fn search_set(
        &mut self,
        query: &str,
        case_sensitive: bool,
        cx: &mut Context<Self>,
    ) -> Option<(usize, usize)> {
        let id = self.tabs.read(cx).active_id();
        if let Some(e) = self.editors.get(&id).cloned() {
            return Some(e.update(cx, |e, cx| e.search_set(query, case_sensitive, cx)));
        }
        let view = self.active_pane_view(cx)?;
        Some(view.update(cx, |v, cx| v.search_set(query, case_sensitive, cx)))
    }

    /// Step to the next / previous match. Returns `(current, total)`.
    pub fn search_step(&mut self, forward: bool, cx: &mut Context<Self>) -> Option<(usize, usize)> {
        let id = self.tabs.read(cx).active_id();
        if let Some(e) = self.editors.get(&id).cloned() {
            return Some(e.update(cx, |e, cx| e.search_step(forward, cx)));
        }
        let view = self.active_pane_view(cx)?;
        Some(view.update(cx, |v, cx| v.search_step(forward, cx)))
    }

    /// Clear all search state / match highlights on the active surface.
    pub fn search_end(&mut self, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        if let Some(e) = self.editors.get(&id).cloned() {
            e.update(cx, |e, cx| e.search_close(cx));
        }
        if let Some(view) = self.active_pane_view(cx) {
            view.update(cx, |v, cx| v.search_end(cx));
        }
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
    /// Local dev-server URL detected in the active terminal's output, if any
    /// (statusbar `previewUrl` item).
    pub fn active_preview_url(&self, cx: &App) -> Option<String> {
        self.active_pane_view(cx)?.read(cx).preview_url()
    }

    /// Last `n` lines of the active terminal's buffer (AI live bridge).
    pub fn active_terminal_lines(&self, n: usize, cx: &App) -> Vec<String> {
        self.active_pane_view(cx)
            .map(|v| v.read(cx).recent_lines(n))
            .unwrap_or_default()
    }

    pub fn inject_into_active_terminal(&self, text: &str, cx: &App) {
        if let Some(view) = self.active_pane_view(cx) {
            let _ = view.read(cx).handle().write(text.as_bytes());
        }
    }

    /// Execute `command` in the active terminal (appends a newline). If no
    /// terminal is focused, opens a new local tab in the active cwd and runs it
    /// there. Used by the AI panel's "Run" affordances (command snippets,
    /// AI⇄Shell composer mode).
    pub fn run_in_active_terminal(
        &mut self,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cmd = command.trim_end().to_string();
        if cmd.is_empty() {
            return;
        }
        if let Some(view) = self.active_pane_view(cx) {
            let _ = view.read(cx).handle().write(format!("{cmd}\n").as_bytes());
            self.focus_active(window, cx);
        } else {
            let cwd = self.active_cwd(cx);
            self.run_snippet_local(cwd, cmd, window, cx);
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

    /// File ▸ New Preview Tab — an empty native preview pane (the WebView
    /// replacement). Opens the address bar ready for a path or URL.
    pub fn new_preview_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open(TabKind::Preview, TabData::default(), cx));
        let theme = self.theme.clone();
        let view = cx.new(|cx| PreviewView::new(theme, cx));
        self.previews.insert(tab_id, view);
        self.focus_active(window, cx);
    }

    /// Open `target` (a local path or URL) in a preview tab, reusing the active
    /// preview tab if one is already open.
    pub fn open_preview(&mut self, target: String, window: &mut Window, cx: &mut Context<Self>) {
        let title = std::path::Path::new(&target)
            .file_name()
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| target.clone());

        let existing = self
            .tabs
            .read(cx)
            .tabs_by_kind(TabKind::Preview)
            .first()
            .map(|t| t.id);

        let tab_id = if let Some(tab_id) = existing {
            self.tabs.update(cx, |s, cx| {
                s.set_custom_title(tab_id, Some(title.clone()), cx);
                s.set_active(tab_id, cx);
            });
            tab_id
        } else {
            let tab_id = self.tabs.update(cx, |s, cx| {
                let id = s.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                s.set_custom_title(id, Some(title.clone()), cx);
                id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(tab_id, view);
            tab_id
        };

        if let Some(view) = self.previews.get(&tab_id).cloned() {
            view.update(cx, |p, cx| p.set_url(target, cx));
        }
        self.focus_active(window, cx);
    }

    /// Save the active editor tab (`Cmd-S`).
    pub fn save_active(&mut self, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        if let Some(view) = self.editors.get(&id).cloned() {
            view.update(cx, |e, cx| e.save(cx));
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

    /// Split the active workspace pane in `dir`.
    pub fn split(&mut self, dir: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        self.split_active(dir, window, cx);
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

    /// Jump to the tab at position `index` (0-based). No-op when no tab holds
    /// that slot. Port of `useTabsStore.selectByIndex` (T13-005).
    pub fn select_tab_by_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.tabs.read(cx).tabs().get(index).map(|t| t.id) {
            self.select_tab(id, window, cx);
        }
    }

    /// Cycle focus to the next split leaf of the active workspace tab. No-op on
    /// a single-pane tab or a non-workspace tab. Port of `pane.focusNext`
    /// (`collectLeafIds` → next index, wrap) (T13-005).
    pub fn focus_next_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((leaves, active)) = self.active_layout(cx).map(|l| (l.leaves(), l.active)) else {
            return;
        };
        if leaves.len() <= 1 {
            return;
        }
        let current = active.and_then(|a| leaves.iter().position(|&p| p == a));
        let next = leaves[current.map_or(0, |i| (i + 1) % leaves.len())];
        self.set_pane_active(next, window, cx);
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
            .try_global::<crate::prefs::GlobalPreferences>()
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
    fn split_active(&mut self, dir: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab_id) = self.active_ws_tab(cx) else {
            return;
        };
        let cwd = self.active_pane_view(cx).and_then(|v| v.read(cx).cwd());
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd, None, cx) else {
            return;
        };
        let axis_id = self.alloc_pane();
        let new_pane = self.alloc_pane();
        if let Some(layout) = self.layouts.get_mut(&tab_id) {
            layout.split(axis_id, new_pane, dir);
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
        let Some(pane) = self.layouts.get(&tab_id).and_then(|l| l.active) else {
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

    fn resize_axis(&mut self, axis_id: PaneId, boundary: usize, frac: f32, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                if layout.set_boundary(axis_id, boundary, frac) {
                    cx.notify();
                }
            }
        }
    }

    fn reset_axis(&mut self, axis_id: PaneId, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.active_ws_tab(cx) {
            if let Some(layout) = self.layouts.get_mut(&tab_id) {
                if layout.reset_axis(axis_id) {
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
        let pane = self.active_layout(cx)?.active?;
        self.panes.get(&pane).map(|e| e.view.clone())
    }

    /// The active editor / terminal selection, tagged with its source label,
    /// for "Ask AI about Selection".
    pub fn active_selection(&self, cx: &App) -> Option<(&'static str, String)> {
        let active_id = self.tabs.read(cx).active_id();
        if let Some(editor) = self.editors.get(&active_id) {
            return editor.read(cx).selected_text().map(|t| ("editor", t));
        }
        self.active_pane_view(cx)
            .and_then(|v| v.read(cx).selection_text())
            .map(|t| ("terminal", t))
    }

    fn select_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename_tab.is_some() {
            self.commit_tab_rename(cx);
        }
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

    /// Close every tab (tab context menu "Close All"). Routed through
    /// `request_close` per tab so unsaved editors still prompt; the workspace
    /// is left showing its empty surface.
    fn close_all_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<u64> = self.tabs.read(cx).tabs().iter().map(|t| t.id).collect();
        for id in ids {
            self.request_close(id, window, cx);
        }
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

    /// The `CommandContext` the active tab exposes, driving which
    /// context-scoped palette commands are offered.
    pub fn active_context(&self, cx: &App) -> Option<labonair_command_palette::CommandContext> {
        let active = self.tabs.read(cx).active()?;
        let is_ssh = self.ssh_tabs.values().any(|t| t.tab_id == active.id);
        labonair_command_palette::context_of(palette_tab_kind(active.kind), is_ssh)
    }

    /// `(id, name)` for every known host — feeds the path-bookmarks popover's
    /// section titles and orphan detection (T12-003).
    pub fn known_hosts(&self, cx: &App) -> Vec<(String, String)> {
        let hm = self.host_manager.read(cx);
        hm.host_ids()
            .into_iter()
            .map(|id| {
                let name = hm.host_name(&id).unwrap_or_else(|| id.clone());
                (id, name)
            })
            .collect()
    }

    /// The host the active tab targets (SSH terminal or SFTP browser), if any.
    pub fn active_host_id(&self, cx: &App) -> Option<String> {
        let active = self.tabs.read(cx).active()?;
        if let Some(t) = self.ssh_tabs.values().find(|t| t.tab_id == active.id) {
            return Some(t.host_id.clone());
        }
        if active.kind == TabKind::Sftp {
            return active.data.host_id.clone();
        }
        None
    }

    /// Open (or re-focus) an SFTP browser tab for `host_id` — path-bookmarks jump.
    pub fn open_sftp_tab(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_sftp(host_id, window, cx);
    }

    /// Open (or re-focus) the host-manager dashboard as a normal, closable
    /// [`TabKind::Hosts`] tab. Wired to the `Open Host Manager` menu item /
    /// `CommandId::OpenHostManager`. T19-010 replaces this with a Settings
    /// entry and removes the tab kind.
    pub fn open_host_manager(&mut self, cx: &mut Context<Self>) {
        let existing = self
            .tabs
            .read(cx)
            .tabs()
            .iter()
            .find(|t| t.kind == TabKind::Hosts)
            .map(|t| t.id);
        self.tabs.update(cx, |s, cx| match existing {
            Some(id) => s.set_active(id, cx),
            None => {
                s.open(TabKind::Hosts, TabData::default(), cx);
            }
        });
    }

    /// Up to `n` known hosts, most-recently-connected first — feeds the `+`
    /// new-tab dropdown's SSH / SFTP submenus.
    pub fn recent_hosts(&self, cx: &App, n: usize) -> Vec<(String, String)> {
        self.host_manager.read(cx).recent_hosts(n)
    }

    /// Open (or reuse) an SSH terminal tab for `host_id` (`+` dropdown / menu).
    pub fn open_ssh_tab(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.connect_host(host_id, window, cx);
    }

    /// Open (or focus) the commit-graph tab (`+` dropdown / command palette).
    /// Share the app-shell's `GitGraphView` entity so the Git Graph tab and
    /// the shell's CWD feed operate on the same view.
    pub fn set_git_graph(&mut self, view: Entity<GitGraphView>) {
        self.git_graph = Some(view);
    }

    /// The app's [`PanelRegistry`](labonair_panel::PanelRegistry). Populated
    /// once by `labonair_shell::register_builtin_panels` right after the
    /// `Workspace` is constructed; read by the shell's dock rendering and
    /// status-bar toggles so adding a panel never touches a `match`.
    pub fn panel_registry(&self) -> &labonair_panel::PanelRegistry {
        &self.panel_registry
    }

    /// Mutable access, for the one-time builtin-panel registration in the shell.
    pub fn panel_registry_mut(&mut self) -> &mut labonair_panel::PanelRegistry {
        &mut self.panel_registry
    }

    /// The app's [`StatusItemRegistry`](labonair_panel::StatusItemRegistry).
    /// Populated once by `labonair_shell::register_builtin_status_items`; read
    /// by the [`StatusBar`](crate::status_bar::StatusBar) component.
    pub fn status_item_registry(&self) -> &labonair_panel::StatusItemRegistry {
        &self.status_item_registry
    }

    /// Mutable access, for the one-time builtin status-item registration.
    pub fn status_item_registry_mut(&mut self) -> &mut labonair_panel::StatusItemRegistry {
        &mut self.status_item_registry
    }

    /// Re-reads the persisted `statusBarItemPlacements` blob from disk and
    /// applies it to the [`StatusItemRegistry`](labonair_panel::StatusItemRegistry)
    /// overrides (T18-005). Called once at startup and whenever another
    /// window bumps [`status_placements::StatusBarLayoutTick`].
    pub fn reload_status_bar_placements(&mut self) {
        let blob = labonair_backend::modules::settings::status_bar_item_placements_load();
        let overrides = status_placements::overrides_from_blob(&blob);
        self.status_item_registry.set_overrides(overrides);
    }

    /// The right-click "move left/right" / "hide" action on a status-bar item
    /// (T18-005): applies the change to the local registry immediately (so
    /// this window's `StatusBar` re-renders without waiting on the write),
    /// then persists it through the backend's atomic read-merge-write and, on
    /// completion, bumps [`status_placements::StatusBarLayoutTick`] so every
    /// *other* window's `StatusBar` reloads the blob and picks up the change
    /// too. The tick bump is deliberately deferred until after the write
    /// lands — bumping it eagerly would race the still-in-flight write, and
    /// this window's own tick observer would reload the pre-write blob from
    /// disk and clobber the override just set above.
    pub fn set_status_bar_placement(
        &mut self,
        id: &'static str,
        side: Option<labonair_panel::StatusSide>,
        hidden: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        self.status_item_registry.set_override(id, side, hidden);
        cx.notify();

        let patch = status_placements::placement_patch(side, hidden);
        let backend = self.backend.clone();
        let item_id = id.to_string();
        let jh = self.tokio.spawn(async move {
            labonair_backend::modules::settings::settings_set_status_bar_placement(
                &backend.status_bar_lock,
                item_id,
                patch,
            )
            .await
        });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// "Reset to default" on the Personalization settings pane (T18-007):
    /// clears every user override in the local registry immediately, then
    /// deletes the persisted `statusBarItemPlacements` blob and bumps
    /// [`status_placements::StatusBarLayoutTick`] so every window (including
    /// this one) re-reads the now-empty blob. Only the statusbar layout is
    /// reset — panel-toggle visibility is untouched (Notizen: scoped to this
    /// pane's own data, not a global reset).
    pub fn reset_status_bar_placements(&mut self, cx: &mut Context<Self>) {
        self.status_item_registry.set_overrides(HashMap::new());
        cx.notify();

        let backend = self.backend.clone();
        let jh = self.tokio.spawn(async move {
            labonair_backend::modules::settings::settings_clear_status_bar_placements(
                &backend.status_bar_lock,
            )
            .await
        });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// The persisted `panelToggleVisibility` blob (T18-007): whether `name`'s
    /// toggle shows in the status bar's fixed-left panel-toggle cluster. A
    /// panel absent from the blob defaults to visible. Does not affect the
    /// panel's dock position or whether it can still be opened from the
    /// command palette.
    pub fn panel_toggle_visible(name: &str) -> bool {
        labonair_backend::modules::settings::panel_toggle_visibility_load()
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
    }

    /// The single write path for panel-toggle visibility (T18-007): both the
    /// status bar's own "Hide from toggle bar" right-click action and the
    /// Personalization settings pane's per-panel switch call this. Persists
    /// through the backend's atomic read-merge-write and bumps
    /// [`status_placements::StatusBarLayoutTick`] so every window's panel
    /// toggle cluster (which observes that global) re-reads the blob and
    /// reflects the change live.
    pub fn set_panel_toggle_visible(
        &mut self,
        name: String,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        let backend = self.backend.clone();
        let panel_name = name.clone();
        let jh = self.tokio.spawn(async move {
            labonair_backend::modules::settings::settings_set_panel_toggle_visibility(
                &backend.panel_toggle_visibility_lock,
                panel_name,
                visible,
            )
            .await
        });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// Install the shell's dock-layout persistence callback (see
    /// [`Workspace::dock_persist_hook`]).
    pub fn set_dock_persist_hook(
        &mut self,
        hook: impl Fn(String, &mut App) + Send + Sync + 'static,
    ) {
        self.dock_persist_hook = Some(Arc::new(hook));
    }

    /// The "primary" edge per the `sidebarPosition` preference (read from the
    /// [`GlobalPreferences`](crate::prefs::GlobalPreferences) global).
    pub fn primary_dock(&self, cx: &App) -> labonair_panel::DockPosition {
        let right = cx
            .try_global::<crate::prefs::GlobalPreferences>()
            .map(|g| g.0.sidebar_position == "right")
            .unwrap_or(false);
        if right {
            labonair_panel::DockPosition::Right
        } else {
            labonair_panel::DockPosition::Left
        }
    }

    /// Which dock hosts `name` — live membership, falling back to the primary
    /// edge when the panel is somehow unregistered.
    pub fn dock_for_panel(&self, name: &str, cx: &App) -> labonair_panel::DockPosition {
        self.dock_of_panel(name)
            .unwrap_or_else(|| self.primary_dock(cx))
    }

    /// Whether `name` is the active panel of an open dock.
    pub fn panel_is_active(&self, name: &str) -> bool {
        self.docks()
            .iter()
            .any(|d| d.is_open() && d.active_name() == Some(name))
    }

    /// Status-bar-toggle intent: open + activate `name`, or close its dock if it
    /// is already the active panel there. Persists the layout.
    pub fn select_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        let pos = self.dock_for_panel(name, cx);
        self.dock_mut(pos).toggle_panel(name);
        self.persist_docks(cx);
        cx.notify();
    }

    /// "show me X" — never closes the dock (palette / menu intent).
    pub fn open_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        let pos = self.dock_for_panel(name, cx);
        {
            let dock = self.dock_mut(pos);
            dock.activate_panel(name);
            dock.set_open(true);
        }
        self.persist_docks(cx);
        cx.notify();
    }

    /// Debounced write of the full `[DockData; 3]` layout through the shell's
    /// persistence hook (mirrors the reference `onLayoutChanged` 300ms persist).
    pub fn persist_docks(&mut self, cx: &mut Context<Self>) {
        let now = std::time::Instant::now();
        if let Some(last) = self.last_dock_save {
            if now.duration_since(last) < Duration::from_millis(300) {
                return;
            }
        }
        self.last_dock_save = Some(now);
        let Some(hook) = self.dock_persist_hook.clone() else {
            return;
        };
        let data: Vec<crate::dock::DockData> = self.docks().iter().map(|d| d.to_data()).collect();
        let json = serde_json::to_string(&data).unwrap_or_default();
        hook(json, cx);
    }

    /// One of the three edge docks (T17-002).
    pub fn dock(&self, position: labonair_panel::DockPosition) -> &crate::dock::Dock {
        use labonair_panel::DockPosition::*;
        match position {
            Left => &self.left_dock,
            Right => &self.right_dock,
            Bottom => &self.bottom_dock,
        }
    }

    /// Mutable access to one of the three edge docks.
    pub fn dock_mut(&mut self, position: labonair_panel::DockPosition) -> &mut crate::dock::Dock {
        use labonair_panel::DockPosition::*;
        match position {
            Left => &mut self.left_dock,
            Right => &mut self.right_dock,
            Bottom => &mut self.bottom_dock,
        }
    }

    /// `[left, right, bottom]`, for read-only iteration (status-bar toggles,
    /// persistence).
    pub fn docks(&self) -> [&crate::dock::Dock; 3] {
        [&self.left_dock, &self.right_dock, &self.bottom_dock]
    }

    /// Which dock currently holds the panel named `name`, if any.
    pub fn dock_of_panel(&self, name: &str) -> Option<labonair_panel::DockPosition> {
        labonair_panel::DockPosition::ALL
            .into_iter()
            .find(|p| self.dock(*p).has_panel(name))
    }

    /// Populate the docks from the [`PanelRegistry`](labonair_panel::PanelRegistry)
    /// and an optional persisted layout (`layout_json` is a JSON array of
    /// [`dock::DockData`](crate::dock::DockData); empty / invalid = first run).
    ///
    /// Each registered panel is built once here and placed in the dock its
    /// persisted `panel_order` names, else its registry `default_position`.
    pub fn init_docks(&mut self, layout_json: &str, window: &mut Window, cx: &mut App) {
        use crate::dock::{position_from_slug, DockData};

        let parsed: Vec<DockData> = serde_json::from_str(layout_json).unwrap_or_default();

        let regs: Vec<(
            &'static str,
            labonair_panel::DockPosition,
            labonair_panel::PanelConstructor,
        )> = self
            .panel_registry
            .iter()
            .map(|r| (r.persistent_name, r.default_position, r.build.clone()))
            .collect();

        for (name, default_pos, build) in regs {
            let handle = build(window, cx);
            let target = parsed
                .iter()
                .find(|d| d.panel_order.iter().any(|n| n == name))
                .and_then(|d| position_from_slug(&d.position))
                .filter(|pos| handle.position_is_valid(*pos, cx))
                .unwrap_or(default_pos);
            self.dock_mut(target).add_panel(handle);
        }

        for pos in labonair_panel::DockPosition::ALL {
            if let Some(data) = parsed
                .iter()
                .find(|d| position_from_slug(&d.position) == Some(pos))
            {
                let dock = self.dock_mut(pos);
                dock.apply_order(&data.panel_order);
                dock.apply_scalars(data);
            }
        }

        // First run (no persisted layout at all): open the left dock so the
        // explorer is visible, matching the pre-T17-002 default.
        if parsed.is_empty() {
            self.left_dock.set_open(true);
        }
    }

    /// Move the panel `name` to the dock at `to`, if the panel allows that
    /// position. Returns `true` when the move happened. (T17-002 — the UI that
    /// calls this lands in T18-007; a debug shortcut exercises it meanwhile.)
    pub fn move_panel(&mut self, name: &str, to: labonair_panel::DockPosition, cx: &App) -> bool {
        let Some(from) = self.dock_of_panel(name) else {
            return false;
        };
        if from == to {
            return true;
        }
        if !self.dock(from).panel_allows(name, to, cx) {
            return false;
        }
        let Some(handle) = self.dock_mut(from).remove_panel(name) else {
            return false;
        };
        let dest = self.dock_mut(to);
        dest.add_panel(handle);
        dest.activate_panel(name);
        dest.set_open(true);
        true
    }

    pub fn open_git_graph_tab(&mut self, cx: &mut Context<Self>) {
        if self.git_graph.is_none() {
            let view = cx.new(|cx| {
                GitGraphView::new(
                    self.backend.clone(),
                    self.tokio.clone(),
                    self.theme.clone(),
                    cx,
                )
            });
            cx.observe(&view, |_, _, cx| cx.notify()).detach();
            self.git_graph = Some(view);
        }
        let existing = self
            .tabs
            .read(cx)
            .tabs()
            .iter()
            .find(|t| t.kind == TabKind::GitGraph)
            .map(|t| t.id);
        self.tabs.update(cx, |s, cx| match existing {
            Some(id) => s.set_active(id, cx),
            None => {
                s.open(TabKind::GitGraph, TabData::default(), cx);
            }
        });
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
        self.previews.remove(&tab.id);

        // SFTP browser tab: drop the view and close its SFTP/SSH session.
        self.sftp_views.remove(&tab.id);
        if let Some(session_id) = self.sftp_sessions.remove(&tab.id) {
            self.ssh_connection
                .update(cx, |s, cx| s.remove(&session_id, cx));
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
                self.ssh_connection
                    .update(cx, |s, cx| s.remove(&t.ssh_id, cx));
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
        self.ssh_connection.update(cx, |s, cx| {
            s.begin(
                session_id.clone(),
                host_id.clone(),
                label.clone(),
                ConnectionKind::Sftp,
                None,
                cx,
            );
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
            SftpEvent::ConnResult { session_id, error } => {
                self.ssh_connection.update(cx, |s, cx| match error {
                    None => s.set_state(session_id, ConnectionState::Connected, cx),
                    Some(msg) => {
                        // Only surface the error screen if we never connected.
                        let connected = s
                            .get(session_id)
                            .map(|e| e.state == ConnectionState::Connected)
                            .unwrap_or(false);
                        if !connected {
                            s.set_error(session_id, msg.clone(), cx);
                        }
                    }
                });
            }
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
        self.ssh_connection.update(cx, |s, cx| {
            s.begin(
                ssh_id.clone(),
                host_id.clone(),
                host_label.clone(),
                ConnectionKind::Terminal,
                jump_label.clone(),
                cx,
            );
        });
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
                        this.ssh_connection
                            .update(cx, |s, cx| s.set_error(&ssh_id, err.clone(), cx));
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
        self.ssh_connection.update(cx, |s, cx| s.resume(ssh_id, cx));
        self.spawn_ssh_connect(ssh_id.to_string(), host_id, passphrase, password, feed, cx);
    }

    /// Apply one transfer-worker bus event to the transfers view. Called by
    /// [`BackendEventBridge`](crate::backend_event_bridge::BackendEventBridge).
    pub(crate) fn apply_transfer_bus_event(
        &mut self,
        ev: TransferBusEvent,
        cx: &mut Context<Self>,
    ) {
        let view = self.transfers.clone();
        view.update(cx, |t, cx| t.apply(ev, cx));
    }

    pub(crate) fn handle_ssh_event(&mut self, ev: AppEvent, cx: &mut Context<Self>) {
        match ev {
            AppEvent::SshConnectLog {
                session_id,
                message,
            } => {
                self.ssh_connection
                    .update(cx, |s, cx| s.push_log(&session_id, &message, cx));
            }
            AppEvent::SshKnownHostsWarning {
                session_id,
                fingerprint,
                host,
                is_mismatch,
            } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_trust(&session_id, fingerprint.clone(), is_mismatch, cx)
                });
                let _ = (host, fingerprint, is_mismatch);
                self.ssh_prompt = Some(SshPrompt::Trust { ssh_id: session_id });
            }
            AppEvent::SshAuthRequired {
                session_id,
                prompt_message,
                is_2fa,
            } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_auth_prompt(&session_id, prompt_message.clone(), is_2fa, cx)
                });
                self.ssh_prompt = Some(SshPrompt::Password {
                    ssh_id: session_id,
                    buffer: String::new(),
                });
            }
            AppEvent::SshPassphraseRequired { session_id } => {
                self.ssh_connection
                    .update(cx, |s, cx| s.set_passphrase(&session_id, cx));
                self.ssh_prompt = Some(SshPrompt::Passphrase {
                    ssh_id: session_id,
                    buffer: String::new(),
                });
            }
            AppEvent::SshSessionEstablished { session_id, .. } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_state(&session_id, ConnectionState::Connected, cx)
                });
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
                let known = self.ssh_connection.read(cx).get(&session_id).is_some();
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
                // Only surface the error screen if the connection never reached
                // the shell — a drop after `Connected` belongs in the terminal.
                if known {
                    self.ssh_connection.update(cx, |s, cx| {
                        let was_connected = s
                            .get(&session_id)
                            .map(|e| e.state == ConnectionState::Connected)
                            .unwrap_or(false);
                        if !was_connected {
                            s.set_error(
                                &session_id,
                                "Connection lost before the session was ready.",
                                cx,
                            );
                        }
                    });
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
                let center = labonair_notifications::notification_center(cx);
                center.update(cx, |c, cx| {
                    c.push_action_result(
                        labonair_notifications::Notification::error(
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
                    let center = labonair_notifications::notification_center(cx);
                    center.update(cx, |c, cx| {
                        c.push(
                            labonair_notifications::Notification::info(
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
                self.ssh_connection
                    .update(cx, |s, cx| s.resume(&ssh_id, cx));
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
        match self.ssh_prompt.take() {
            Some(SshPrompt::Trust { ssh_id, .. }) => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_error(&ssh_id, "Host key was not trusted.", cx)
                });
                let app = self.backend.clone();
                self.tokio.spawn(async move {
                    let _ = ssh_trust_host(ssh_id, false, &app.trust).await;
                });
            }
            Some(SshPrompt::Password { ssh_id, .. } | SshPrompt::Passphrase { ssh_id, .. }) => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_error(&ssh_id, "Authentication cancelled.", cx)
                });
            }
            None => {}
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

    /// Full-pane SSH connecting screen (T16-015) — port of
    /// `reference-src/src/modules/terminal/SshLoadingScreen.tsx`. Shown in the
    /// tab body in place of the terminal until `session_established`, driving:
    /// the 4-stage progress indicator, the live connection-log panel, and the
    /// state-specific card (quick-connect password / trust / auth / passphrase
    /// / error with retry+edit-host).
    fn render_ssh_loading(
        &mut self,
        entry: labonair_hosts_ui::ssh_connection::ConnectionEntry,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme.read(cx);
        let core = theme.theme().core.clone();
        let (bg, card, fg, border, accent, muted) = (
            theme.background(),
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let destructive = core.destructive;
        let ssh_id = entry.session_id.clone();
        let is_sftp = matches!(entry.kind, ConnectionKind::Sftp);
        let tab_id = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == ssh_id)
            .map(|t| t.tab_id)
            .or_else(|| {
                self.sftp_sessions
                    .iter()
                    .find(|(_, sid)| **sid == ssh_id)
                    .map(|(tid, _)| *tid)
            });
        let host_id = entry.host_id.clone();

        // ── 4-stage progress row ──────────────────────────────────────────
        let stage_count = ConnStage::ORDER.len();
        let stages =
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(ConnStage::ORDER.iter().enumerate().map(|(i, &stage)| {
                    let status = entry.stage_status(stage);
                    let (dot_bg, dot_fg, txt) = match status {
                        StageStatus::Done => (accent, bg, accent),
                        StageStatus::Active => (accent.opacity(0.2), accent, fg),
                        StageStatus::Pending => (gpui::transparent_black(), muted, muted),
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded_full()
                                .border_1()
                                .border_color(if status == StageStatus::Pending {
                                    border
                                } else {
                                    accent
                                })
                                .bg(dot_bg)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(dot_fg)
                                .child(if status == StageStatus::Done {
                                    "\u{2713}".to_string()
                                } else {
                                    (i + 1).to_string()
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(txt)
                                .child(SharedString::from(stage.label(entry.kind))),
                        )
                        .when(i + 1 < stage_count, |d| {
                            d.child(div().w(px(16.0)).h(px(1.0)).bg(border))
                        })
                }));

        // ── state card ────────────────────────────────────────────────────
        let is_prompt = matches!(
            entry.state,
            ConnectionState::WaitingAuth
                | ConnectionState::WaitingPassphrase
                | ConnectionState::QuickConnectPassword
        );
        let buffer_dots = self
            .ssh_prompt
            .as_ref()
            .and_then(|p| match p {
                SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. } => {
                    Some("\u{2022}".repeat(buffer.chars().count()))
                }
                _ => None,
            })
            .unwrap_or_default();

        let (card_title, card_body): (String, String) = match &entry.state {
            ConnectionState::Error => (
                "Connection failed".to_string(),
                entry
                    .error
                    .clone()
                    .unwrap_or_else(|| "The connection could not be established.".to_string()),
            ),
            ConnectionState::WaitingTrust => (
                if entry.trust_mismatch {
                    "Host key CHANGED".to_string()
                } else {
                    "Unknown host key".to_string()
                },
                format!(
                    "Fingerprint: {}\n\n{}",
                    entry.trust_fingerprint.as_deref().unwrap_or("(unknown)"),
                    if entry.trust_mismatch {
                        "The key differs from the one on record. Only continue if you know why."
                    } else {
                        "This host is not yet in known_hosts."
                    }
                ),
            ),
            ConnectionState::WaitingAuth => (
                if entry.is_2fa {
                    "Two-factor code required".to_string()
                } else {
                    "Password required".to_string()
                },
                format!(
                    "{}\n{}",
                    entry
                        .prompt_message
                        .as_deref()
                        .unwrap_or("Enter your password"),
                    buffer_dots
                ),
            ),
            ConnectionState::WaitingPassphrase | ConnectionState::QuickConnectPassword => (
                "Key passphrase".to_string(),
                format!("Enter the passphrase for the private key.\n{buffer_dots}"),
            ),
            _ => (
                format!("Connecting to {}\u{2026}", entry.host_label),
                "Negotiating a secure channel.".to_string(),
            ),
        };

        let mut actions = div().flex().gap_2().justify_end().pt_1();
        match entry.state {
            ConnectionState::Error => {
                actions = actions
                    .child(
                        loading_btn("ssh-l-close", "Close", muted, border, false, fg).on_click(
                            cx.listener(move |this, _: &ClickEvent, _w, _cx| {
                                if let Some(id) = tab_id {
                                    this.pending_tab_close.push(id);
                                }
                            }),
                        ),
                    )
                    .child(
                        loading_btn("ssh-l-edit", "Edit Host", muted, border, false, fg).on_click(
                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.open_host_manager(cx)
                            }),
                        ),
                    )
                    .child(
                        loading_btn("ssh-l-retry", "Retry", accent, border, true, fg).on_click(
                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.ssh_prompt = None;
                                if is_sftp {
                                    this.ssh_connection
                                        .update(cx, |s, cx| s.resume(&ssh_id, cx));
                                    if let Some(tid) = tab_id {
                                        if let Some(view) = this.sftp_views.get(&tid).cloned() {
                                            view.update(cx, |v, cx| v.reconnect(cx));
                                        }
                                    }
                                } else {
                                    this.retry_ssh(&ssh_id, None, None, cx);
                                }
                            }),
                        ),
                    );
                let _ = host_id;
            }
            ConnectionState::WaitingTrust => {
                actions = actions
                    .child(
                        loading_btn("ssh-l-abort", "Abort", muted, border, false, fg).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.cancel_prompt(cx)),
                        ),
                    )
                    .child(
                        loading_btn(
                            "ssh-l-trust",
                            if entry.trust_mismatch {
                                "Accept anyway"
                            } else {
                                "Trust & Connect"
                            },
                            accent,
                            border,
                            true,
                            fg,
                        )
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_prompt(cx)),
                        ),
                    );
            }
            _ if is_prompt => {
                actions = actions
                    .child(
                        loading_btn("ssh-l-cancel", "Cancel", muted, border, false, fg).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.cancel_prompt(cx)),
                        ),
                    )
                    .child(
                        loading_btn("ssh-l-submit", "Submit", accent, border, true, fg).on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_prompt(cx)),
                        ),
                    );
            }
            _ => {
                actions = actions.child(
                    loading_btn("ssh-l-cancel2", "Cancel", muted, border, false, fg).on_click(
                        cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.ssh_connection
                                .update(cx, |s, cx| s.set_error(&ssh_id, "Cancelled by user.", cx));
                        }),
                    ),
                );
            }
        }

        let card_border = if matches!(entry.state, ConnectionState::Error)
            || entry.trust_mismatch && entry.state == ConnectionState::WaitingTrust
        {
            destructive
        } else {
            border
        };

        let state_card = div()
            .track_focus(&self.prompt_focus)
            .key_context("SshPrompt")
            .on_key_down(cx.listener(Self::on_prompt_key))
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_4()
            .rounded_lg()
            .bg(card)
            .border_1()
            .border_color(card_border)
            .child(
                div()
                    .text_sm()
                    .text_color(fg)
                    .child(SharedString::from(card_title)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .whitespace_normal()
                    .child(SharedString::from(card_body)),
            )
            .child(actions);

        // ── live connection log ───────────────────────────────────────────
        let log_lines = entry.log.clone();
        let log_panel = div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().size(px(6.0)).rounded_full().bg(accent))
                    .child(div().text_xs().text_color(muted).child("Connection Log")),
            )
            .child(
                div()
                    .id("ssh-connect-log")
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .w_full()
                    .max_h(px(180.0))
                    .overflow_y_scroll()
                    .p_2()
                    .rounded_md()
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .font_family("monospace")
                    .text_size(px(11.0))
                    .text_color(muted)
                    .children(if log_lines.is_empty() {
                        vec![div()
                            .child("Waiting for the backend\u{2026}")
                            .into_any_element()]
                    } else {
                        log_lines
                            .iter()
                            .map(|l| {
                                div()
                                    .child(SharedString::from(l.clone()))
                                    .into_any_element()
                            })
                            .collect()
                    }),
            );

        let jump_badge = entry.jump_host_name.clone().map(|j| {
            div()
                .px_2()
                .py_0p5()
                .rounded_md()
                .border_1()
                .border_color(border)
                .text_xs()
                .text_color(muted)
                .child(SharedString::from(format!("via {j}")))
        });

        div()
            .size_full()
            .bg(bg)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(520.0))
                    .max_w_full()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(fg)
                                    .child(SharedString::from(entry.host_label.clone())),
                            )
                            .children(jump_badge),
                    )
                    .child(stages)
                    .child(state_card)
                    .child(log_panel),
            )
            .into_any_element()
    }

    fn sync_meta(&mut self, cx: &mut Context<Self>) {
        let updates: Vec<(u64, Option<String>, Option<String>)> = self
            .layouts
            .iter()
            .filter_map(|(tab_id, layout)| {
                let v = self.panes.get(&layout.active?)?.view.read(cx);
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
        // Every tab is closable now — closing the last one just leaves the
        // empty-workspace surface (T17-009).
        let closable = true;
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
        // T19-002: real `ThemeSettings::get(cx)` consumer (was
        // `GlobalPreferences`) — `SettingsStore` merges default < user for
        // `appearance.reduceMotion`. `try_get` (not `get`) so a headless test
        // harness that never called `labonair_settings::init` still renders.
        use labonair_settings::Settings as _;
        let reduce_motion = labonair_settings::ThemeSettings::try_get(cx)
            .map(|s| s.reduce_motion())
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
            .child(div().child(tab.kind.indicator().svg(muted)))
            .child(
                match self.rename_tab.as_ref().filter(|(rid, _)| *rid == id) {
                    Some((_, buf)) => div()
                        .track_focus(&self.rename_focus)
                        .key_context("TabRename")
                        .on_key_down(cx.listener(Self::on_rename_key))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _: &MouseDownEvent, _w, cx| cx.stop_propagation()),
                        )
                        .min_w(px(80.0))
                        .max_w(px(180.0))
                        .px_1()
                        .rounded_sm()
                        .border_1()
                        .border_color(accent)
                        .child(SharedString::from(format!("{buf}\u{2502}"))),
                    None => div()
                        .max_w(px(180.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .when(tab.kind == TabKind::Editor && tab.peek, |d| d.italic())
                        .child(label.clone()),
                },
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
                    this.context_menu =
                        Some((id, ev.position - point(px(0.0), px(TITLEBAR_OFFSET))));
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

    /// The tab strip. Rendered inside `AppShell`'s single overlay titlebar
    /// (the header) between the left/right bar-item buckets — mirrors the
    /// reference `Header.tsx` where the `TabBar` lives in the titlebar.
    pub fn render_tab_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (muted, fg, border) = (theme.muted_foreground(), theme.foreground(), theme.border());
        let tabs = self.tabs.read(cx).tabs().to_vec();

        div()
            .flex()
            .items_center()
            .gap_1()
            .h(px(28.0))
            .w_full()
            .flex_shrink_0()
            // Right-click anywhere on the empty strip → the new-tab menu
            // (reference `TabBar` empty-area menu).
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, ev: &MouseDownEvent, _w, cx| {
                    this.new_tab_menu = Some(ev.position - point(px(0.0), px(TITLEBAR_OFFSET)));
                    this.context_menu = None;
                    cx.notify();
                }),
            )
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                            this.new_tab_menu =
                                Some(ev.position - point(px(0.0), px(TITLEBAR_OFFSET)));
                            this.context_menu = None;
                            cx.notify();
                        }),
                    ),
            )
    }

    /// The "+" new-tab dropdown (port of `NewTabDropdownItems`): Terminal /
    /// Editor / Preview / Git Graph, then flattened "SSH · <host>" /
    /// "SFTP · <host>" recent-host entries + "Open Host Manager".
    fn render_new_tab_menu(
        &mut self,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let recent = self.recent_hosts(cx, 5);
        let view = cx.entity();
        let mut items: Vec<MenuItem> = vec![
            MenuItem::new("nt-term", "Terminal")
                .icon(IconName::Terminal)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_terminal_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-editor", "Editor")
                .icon(IconName::File)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_editor_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-preview", "Preview")
                .icon(IconName::Globe)
                .on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.new_preview_tab(w, cx)
                        })
                    }
                }),
            MenuItem::new("nt-gitgraph", "Git Graph")
                .icon(IconName::GitBranch)
                .on_click({
                    let v = view.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.open_git_graph_tab(cx)
                        })
                    }
                }),
        ];
        if !recent.is_empty() {
            items.push(MenuItem::separator());
            items.push(MenuItem::label("SSH"));
            for (id, name) in &recent {
                let (id, name) = (id.clone(), name.clone());
                items.push(
                    MenuItem::new(
                        SharedString::from(format!("nt-ssh-{id}")),
                        format!("\u{00b7} {name}"),
                    )
                    .on_click({
                        let v = view.clone();
                        move |_, w, cx| {
                            let id = id.clone();
                            v.update(cx, |this, cx| {
                                this.new_tab_menu = None;
                                this.open_ssh_tab(id, w, cx)
                            })
                        }
                    }),
                );
            }
            items.push(MenuItem::label("SFTP"));
            for (id, name) in &recent {
                let (id, name) = (id.clone(), name.clone());
                items.push(
                    MenuItem::new(
                        SharedString::from(format!("nt-sftp-{id}")),
                        format!("\u{00b7} {name}"),
                    )
                    .on_click({
                        let v = view.clone();
                        move |_, w, cx| {
                            let id = id.clone();
                            v.update(cx, |this, cx| {
                                this.new_tab_menu = None;
                                this.open_sftp_tab(id, w, cx)
                            })
                        }
                    }),
                );
            }
        }
        items.push(MenuItem::separator());
        items.push(
            MenuItem::new("nt-hosts", "All hosts\u{2026}")
                .icon(IconName::Server)
                .on_click({
                    let v = view.clone();
                    move |_, _w, cx| {
                        v.update(cx, |this, cx| {
                            this.new_tab_menu = None;
                            this.open_host_manager(cx)
                        })
                    }
                }),
        );

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.new_tab_menu = None;
                    cx.notify()
                })
            }
        };
        context_menu(pos, self.theme.read(cx), dismiss, items)
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active = self.tabs.read(cx).active().cloned();
        let Some(active) = active else {
            return self.render_empty_surface(cx).into_any_element();
        };

        match active.kind {
            TabKind::Hosts => self.host_manager.clone().into_any_element(),
            TabKind::Workspace => {
                // While an SSH session for this tab is still connecting (or in
                // a prompt / error state), the loading screen replaces the
                // terminal (T16-015).
                if let Some(ssh_id) = self
                    .ssh_tabs
                    .values()
                    .find(|t| t.tab_id == active.id)
                    .map(|t| t.ssh_id.clone())
                {
                    if let Some(entry) = self
                        .ssh_connection
                        .read(cx)
                        .get(&ssh_id)
                        .filter(|e| e.state.is_blocking())
                        .cloned()
                    {
                        return self.render_ssh_loading(entry, cx);
                    }
                }
                if let Some(layout) = self.layouts.get(&active.id).cloned() {
                    let multi = layout.len() > 1;
                    let active_pane = layout.active;
                    match &layout.group.root {
                        Some(root) => div()
                            .size_full()
                            .child(self.render_member(root, active_pane, multi, cx))
                            .into_any_element(),
                        // Empty tree — the empty-workspace surface (T17-009 /
                        // T18-001) will replace this placeholder.
                        None => self.placeholder("Terminal", cx).into_any_element(),
                    }
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
                if let Some(session_id) = self.sftp_sessions.get(&active.id).cloned() {
                    if let Some(entry) = self
                        .ssh_connection
                        .read(cx)
                        .get(&session_id)
                        .filter(|e| e.state.is_blocking())
                        .cloned()
                    {
                        return self.render_ssh_loading(entry, cx);
                    }
                }
                if let Some(view) = self.sftp_views.get(&active.id) {
                    view.clone().into_any_element()
                } else {
                    self.placeholder("SFTP", cx).into_any_element()
                }
            }
            TabKind::Preview => {
                if let Some(view) = self.previews.get(&active.id) {
                    view.clone().into_any_element()
                } else {
                    self.placeholder("Preview", cx).into_any_element()
                }
            }
            TabKind::GitGraph => match &self.git_graph {
                Some(view) => view.clone().into_any_element(),
                None => self.placeholder("Git Graph", cx).into_any_element(),
            },
            other => self
                .placeholder(other.default_title(), cx)
                .into_any_element(),
        }
    }

    /// Render one node of the recursive pane tree: a leaf pane, or an axis of
    /// members laid out along `flexes` with a resize handle between each pair.
    fn render_member(
        &mut self,
        node: &Member,
        active_pane: Option<PaneId>,
        multi: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme.read(cx);
        let (bg, border, accent) = (theme.background(), theme.border(), theme.accent());

        match node {
            Member::Pane(id) => {
                let id = *id;
                let is_active = active_pane == Some(id);
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
            Member::Axis(ax) => {
                let axis_id = ax.id;
                let row = ax.axis == SplitAxis::Horizontal;
                let n = ax.members.len();
                let flexes = ax.flexes.clone();

                let mut container = div()
                    .flex()
                    .size_full()
                    .min_w_0()
                    .min_h_0()
                    .when(row, |d| d.flex_row())
                    .when(!row, |d| d.flex_col());

                for (i, member) in ax.members.iter().enumerate() {
                    let child_el = self.render_member(member, active_pane, multi, cx);
                    let last = i + 1 == n;
                    let basis = flexes.get(i).copied().unwrap_or(1.0 / n as f32);
                    container = container.child(
                        div()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .flex_basis(relative(basis))
                            .when(last, |d| d.flex_grow())
                            .child(child_el),
                    );
                    if !last {
                        let boundary = i;
                        container = container.child(
                            div()
                                .id(SharedString::from(format!(
                                    "axis-handle-{axis_id}-{boundary}"
                                )))
                                .flex_shrink_0()
                                .bg(border)
                                .hover(|s| s.bg(accent))
                                .when(row, |d| d.w(px(HANDLE)).h_full().cursor_col_resize())
                                .when(!row, |d| d.h(px(HANDLE)).w_full().cursor_row_resize())
                                .on_drag(PaneResize { axis_id, boundary }, |_, _, _, cx| {
                                    cx.new(|_| DragGhost)
                                })
                                .on_click(cx.listener(
                                    move |this, ev: &ClickEvent, _window, cx| {
                                        if ev.click_count() >= 2 {
                                            this.reset_axis(axis_id, cx);
                                        }
                                    },
                                )),
                        );
                    }
                }

                container
                    .on_drag_move(cx.listener(
                        move |this, ev: &DragMoveEvent<PaneResize>, _window, cx| {
                            let drag = ev.drag(cx);
                            if drag.axis_id != axis_id {
                                return;
                            }
                            let boundary = drag.boundary;
                            let b = ev.bounds;
                            let p = ev.event.position;
                            let frac = if row {
                                f32::from(p.x - b.origin.x) / f32::from(b.size.width).max(1.0)
                            } else {
                                f32::from(p.y - b.origin.y) / f32::from(b.size.height).max(1.0)
                            };
                            this.resize_axis(axis_id, boundary, frac, cx);
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

    /// The empty-workspace surface, shown when no tabs are open (T17-009).
    /// Deliberately minimal — the styled version plus the `＋▾` menu and
    /// file-drop land in T18-001. Double-click opens a local terminal so the
    /// area is not a dead end.
    /// The surface shown when zero tabs are open (T17-009 gates it; T18-001
    /// gives it its final look). A small wordmark over a column of
    /// keyboard-shortcut hints. Double-click anywhere → new local terminal;
    /// drop files → one editor tab each. No own state — pure `Workspace` read.
    fn render_empty_surface(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, fg, muted, border) = (
            theme.background(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
        );

        let hint = move |keys: &'static str, label: &'static str| {
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .min_w(px(56.0))
                        .flex()
                        .justify_center()
                        .px_2()
                        .py_0p5()
                        .rounded_sm()
                        .border_1()
                        .border_color(border)
                        .text_color(muted)
                        .text_xs()
                        .child(keys),
                )
                .child(div().text_sm().text_color(muted).child(label))
        };

        div()
            .id("empty-workspace")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(bg)
            .child(div().text_sm().text_color(fg).child("Labonair"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(hint("\u{2318}T", "New Terminal"))
                    .child(hint("\u{2318}E", "Editor"))
                    .child(hint("\u{2318}K", "Commands"))
                    .child(hint("\u{2318},", "Settings"))
                    .child(hint("\u{2318}\u{21e7}N", "Hosts")),
            )
            .on_click(cx.listener(|this, ev: &ClickEvent, window, cx| {
                if ev.click_count() >= 2 {
                    this.new_terminal_tab(window, cx);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                for path in paths.paths() {
                    if let Some(p) = path.to_str() {
                        this.open_file(p.to_string(), false, window, cx);
                    }
                }
            }))
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

    /// Start editing a tab's title inline (from the tab context menu).
    fn begin_tab_rename(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.context_menu = None;
        let current = self
            .tabs
            .read(cx)
            .get(id)
            .map(|t| t.label().to_string())
            .unwrap_or_default();
        self.rename_tab = Some((id, current));
        window.focus(&self.rename_focus);
        cx.notify();
    }

    fn commit_tab_rename(&mut self, cx: &mut Context<Self>) {
        if let Some((id, buf)) = self.rename_tab.take() {
            let trimmed = buf.trim();
            let title = (!trimmed.is_empty()).then(|| trimmed.to_string());
            self.tabs
                .update(cx, |s, cx| s.set_custom_title(id, title, cx));
        }
        cx.notify();
    }

    fn on_rename_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, buf)) = self.rename_tab.as_mut() else {
            return;
        };
        match ev.keystroke.key.as_str() {
            "enter" => self.commit_tab_rename(cx),
            "escape" => {
                self.rename_tab = None;
                cx.notify();
            }
            "backspace" => {
                buf.pop();
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    buf.push_str(ch);
                    cx.notify();
                }
            }
        }
    }

    fn render_context_menu(
        &mut self,
        id: u64,
        pos: gpui::Point<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab = self.tabs.read(cx).get(id).cloned();
        let kind = tab.as_ref().map(|t| t.kind);
        let is_peek = tab.as_ref().map(|t| t.peek).unwrap_or(false);
        let plural = kind.map(|k| k.plural_label().to_string());
        let grant_target = self.mcp_grant_target(id, cx);
        let is_granted = self.agent_access.read(cx).is_granted(id);
        let multi = self.tabs.read(cx).len() > 1;
        let view = cx.entity();

        let mut items: Vec<MenuItem> = Vec::new();
        if kind == Some(TabKind::Editor) && is_peek {
            items.push(
                MenuItem::new("keep", "Keep Tab Open")
                    .icon(IconName::Eye)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            v.update(cx, |this, cx| {
                                this.context_menu = None;
                                this.tabs.update(cx, |s, cx| s.set_peek(id, false, cx));
                            })
                        }
                    }),
            );
        }
        if kind != Some(TabKind::Hosts) {
            items.push(
                MenuItem::new("rename", "Rename")
                    .icon(IconName::Pencil)
                    .on_click({
                        let v = view.clone();
                        move |_, w, cx| v.update(cx, |this, cx| this.begin_tab_rename(id, w, cx))
                    }),
            );
            items.push(
                MenuItem::new("dup", "Duplicate")
                    .icon(IconName::Copy)
                    .on_click({
                        let v = view.clone();
                        move |_, w, cx| {
                            v.update(cx, |this, cx| {
                                this.context_menu = None;
                                this.tabs.update(cx, |s, cx| s.set_active(id, cx));
                                this.duplicate_active_tab(w, cx);
                            })
                        }
                    }),
            );
        }
        items.push(MenuItem::separator());
        if multi {
            items.push(MenuItem::new("others", "Close Others").on_click({
                let v = view.clone();
                move |_, w, cx| {
                    v.update(cx, |this, cx| {
                        this.context_menu = None;
                        this.close_others(id, w, cx)
                    })
                }
            }));
            items.push(MenuItem::new("all", "Close All").on_click({
                let v = view.clone();
                move |_, w, cx| {
                    v.update(cx, |this, cx| {
                        this.context_menu = None;
                        this.close_all_tabs(w, cx)
                    })
                }
            }));
            if let (Some(k), Some(pl)) = (kind, plural) {
                items.push(MenuItem::new("kind", format!("Close All {pl}")).on_click({
                    let v = view.clone();
                    move |_, w, cx| {
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            this.close_by_kind(k, w, cx)
                        })
                    }
                }));
            }
        }
        items.push(MenuItem::new("close", "Close").on_click({
            let v = view.clone();
            move |_, w, cx| {
                v.update(cx, |this, cx| {
                    this.context_menu = None;
                    this.request_close(id, w, cx)
                })
            }
        }));
        if let Some((session_id, label, gkind, host_id, pty)) = grant_target {
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("mcp-grant", "Grant AI Agent Access")
                    .icon(IconName::Shield)
                    .checked(is_granted)
                    .on_click({
                        let v = view.clone();
                        move |_, _w, cx| {
                            let (session_id, label, host_id) =
                                (session_id.clone(), label.clone(), host_id.clone());
                            v.update(cx, |this, cx| {
                                this.context_menu = None;
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
                            })
                        }
                    }),
            );
        }

        let dismiss = {
            let v = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.context_menu = None;
                    cx.notify()
                })
            }
        };
        context_menu(pos, self.theme.read(cx), dismiss, items)
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

/// Value carried while dragging a dock's edge handle — the position tells the
/// drop handler which dock to resize and along which axis. Moved off `AppShell`
/// in T17-006 together with [`Workspace::render_dock`].
#[derive(Clone, Copy)]
struct DockResize(labonair_panel::DockPosition);

impl Workspace {
    /// Resize the dock at `pos`, clamped by the active panel's `min_size` and
    /// the dock's own bounds, then persist. Ported verbatim off `AppShell`
    /// (T17-006).
    fn set_dock_size(
        &mut self,
        pos: labonair_panel::DockPosition,
        size: f32,
        cx: &mut Context<Self>,
    ) {
        let floor = self.dock(pos).active_panel().and_then(|p| p.min_size(cx));
        let changed = {
            let dock = self.dock_mut(pos);
            let before = dock.size();
            dock.set_size(px(size), floor);
            (f32::from(dock.size()) - f32::from(before)).abs() > 0.5
        };
        if changed {
            self.persist_docks(cx);
            cx.notify();
        }
    }

    /// Move `name` to dock `to`, persisting + notifying on a real move.
    fn move_panel_persist(
        &mut self,
        name: &str,
        to: labonair_panel::DockPosition,
        cx: &mut Context<Self>,
    ) {
        if self.move_panel(name, to, cx) {
            self.persist_docks(cx);
            cx.notify();
        }
    }

    /// Apply one queued [`LiveCommand`] from the AI live-bridge to the active
    /// terminal (T17-006 — replaces `AppShell::sync_live_bridge`'s per-frame
    /// `drain_commands` loop). Queued commands only ever exist while a terminal
    /// is active (the bridge gates `enqueue` on `has_terminal`), so no
    /// new-tab / `&mut Window` path is needed here.
    pub fn apply_live_command(&mut self, cmd: LiveCommand, cx: &mut Context<Self>) {
        let Some(view) = self.active_pane_view(cx) else {
            return;
        };
        let payload = if cmd.execute {
            format!("{}\n", cmd.text.trim_end())
        } else {
            cmd.text.clone()
        };
        let _ = view.read(cx).handle().write(payload.as_bytes());
        cx.notify();
    }

    /// Render one edge dock (T17-002): a header (active panel title + a
    /// per-panel switcher when the dock holds more than one + a "move to next
    /// dock" affordance), the active panel's body, and a resize handle on the
    /// inner edge. Left/right docks are vertical + width-resizable; the bottom
    /// dock is horizontal + height-resizable. A zoomed dock fills its axis and
    /// drops the handle. Ported off `AppShell` in T17-006.
    fn render_dock(
        &mut self,
        pos: labonair_panel::DockPosition,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use labonair_panel::DockPosition;

        let (sidebar_bg, sidebar_fg, sidebar_border, accent, muted) = {
            let theme = self.theme.read(cx);
            (
                theme.sidebar_bg(),
                theme.sidebar_fg(),
                theme.sidebar_border(),
                theme.accent(),
                theme.muted_foreground(),
            )
        };

        let is_bottom = pos == DockPosition::Bottom;
        let (size, zoomed, tabs, body, title) = {
            let dock = self.dock(pos);
            let tabs: Vec<(SharedString, SharedString, bool)> = dock
                .panels()
                .iter()
                .map(|p| {
                    (
                        SharedString::from(p.persistent_name()),
                        p.title(cx),
                        dock.active_name() == Some(p.persistent_name()),
                    )
                })
                .collect();
            let body: Option<gpui::AnyElement> = dock
                .active_panel()
                .map(|handle| handle.to_any().into_any_element());
            let title: SharedString = match dock.active_panel() {
                Some(handle) => handle.title(cx).to_string().to_uppercase().into(),
                None => SharedString::from(""),
            };
            (f32::from(dock.size()), dock.is_zoomed(), tabs, body, title)
        };

        let multi = tabs.len() > 1;
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_2()
            .text_xs()
            .text_color(muted)
            .child(if multi {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(tabs.into_iter().map(|(name, label, is_active)| {
                        let n = name.clone();
                        div()
                            .id(SharedString::from(format!("dock-tab-{name}")))
                            .px_1p5()
                            .rounded_sm()
                            .cursor_pointer()
                            .when(is_active, |d| {
                                d.bg(accent.opacity(0.2)).text_color(sidebar_fg)
                            })
                            .when(!is_active, |d| d.hover(|s| s.text_color(sidebar_fg)))
                            .child(label)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.dock_mut(pos).activate_panel(&n);
                                this.persist_docks(cx);
                                cx.notify();
                            }))
                    }))
                    .into_any_element()
            } else {
                div().child(title).into_any_element()
            })
            .child(
                div()
                    .id(SharedString::from(format!(
                        "dock-move-{}",
                        position_slug(pos)
                    )))
                    .cursor_pointer()
                    .text_color(muted)
                    .hover(|s| s.text_color(sidebar_fg))
                    .child(if is_bottom { "\u{2191}" } else { "\u{21C4}" })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if let Some(name) = this.dock(pos).active_name() {
                            let name = name.to_owned();
                            this.move_panel_persist(&name, pos.next(), cx);
                        }
                    })),
            );

        let panel = div()
            .when(!zoomed && !is_bottom, |d| d.w(px(size)).flex_shrink_0())
            .when(!zoomed && is_bottom, |d| d.h(px(size)).flex_shrink_0())
            .when(zoomed, |d| d.flex_1())
            .when(!is_bottom, |d| d.h_full())
            .when(is_bottom, |d| d.w_full())
            .flex()
            .flex_col()
            .min_h_0()
            .bg(sidebar_bg)
            .text_color(sidebar_fg)
            .child(header)
            .children(body);

        let handle = (!zoomed).then(|| {
            div()
                .id(SharedString::from(format!(
                    "dock-handle-{}",
                    position_slug(pos)
                )))
                .flex_shrink_0()
                .flex()
                .when(!is_bottom, |d| {
                    d.w(RESIZE_HANDLE_SIZE)
                        .h_full()
                        .justify_center()
                        .cursor_col_resize()
                })
                .when(is_bottom, |d| {
                    d.h(RESIZE_HANDLE_SIZE)
                        .w_full()
                        .items_center()
                        .cursor_row_resize()
                })
                .hover(|s| s.bg(accent.opacity(0.4)))
                .child(
                    div()
                        .when(!is_bottom, |d| d.w(px(1.0)).h_full())
                        .when(is_bottom, |d| d.h(px(1.0)).w_full())
                        .bg(sidebar_border),
                )
                .on_drag(DockResize(pos), |_, _, _, cx| cx.new(|_| DragGhost))
        });

        let container = div()
            .flex_shrink_0()
            .flex()
            .when(!is_bottom, |d| d.h_full().flex_row())
            .when(is_bottom, |d| d.w_full().flex_col())
            .when(zoomed, |d| d.flex_1());

        // Handle sits on the inner edge: right of a left dock, above a bottom
        // dock, left of a right dock.
        match pos {
            DockPosition::Left => container.child(panel).children(handle),
            DockPosition::Right | DockPosition::Bottom => container.children(handle).child(panel),
        }
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
        for tab_id in std::mem::take(&mut self.pending_tab_close) {
            self.request_close(tab_id, window, cx);
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
        let content = self.render_content(cx);
        let confirm = self
            .confirm_close
            .map(|id| self.render_confirm(id, cx).into_any_element());
        let context_menu = self
            .context_menu
            .map(|(id, pos)| self.render_context_menu(id, pos, cx).into_any_element());
        let new_tab_menu = self
            .new_tab_menu
            .map(|pos| self.render_new_tab_menu(pos, cx).into_any_element());

        // T17-006: the three edge docks + the drag-to-resize handler used to
        // live in `AppShell::render`; they compose here now so the shell only
        // has to `.child(workspace.clone())`.
        use labonair_panel::DockPosition;
        let (left_open, right_open, bottom_open) = (
            self.dock(DockPosition::Left).is_open(),
            self.dock(DockPosition::Right).is_open(),
            self.dock(DockPosition::Bottom).is_open(),
        );
        let left_dock =
            left_open.then(|| self.render_dock(DockPosition::Left, cx).into_any_element());
        let right_dock =
            right_open.then(|| self.render_dock(DockPosition::Right, cx).into_any_element());
        let bottom_dock = bottom_open.then(|| {
            self.render_dock(DockPosition::Bottom, cx)
                .into_any_element()
        });
        let center = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().min_w_0().child(content))
            .children(bottom_dock);
        let docked = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_row()
            .children(left_dock)
            .child(center)
            .children(right_dock)
            .on_drag_move(
                cx.listener(|this, ev: &DragMoveEvent<DockResize>, _window, cx| {
                    let pos = ev.drag(cx).0;
                    let b = ev.bounds;
                    let p = ev.event.position;
                    let size = match pos {
                        DockPosition::Left => f32::from(p.x - b.origin.x),
                        DockPosition::Right => f32::from(b.origin.x + b.size.width - p.x),
                        DockPosition::Bottom => f32::from(b.origin.y + b.size.height - p.y),
                    };
                    this.set_dock_size(pos, size, cx);
                }),
            );

        div()
            .track_focus(&self.focus_handle)
            .key_context("Workspace")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .on_key_down(cx.listener(Self::on_key_down))
            .child(docked)
            .children(confirm)
            .children(context_menu)
            .children(new_tab_menu)
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
        // now (see `labonair_ui::menu`), bound so the native menu shares the path.
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

/// POSIX single-quote a path for a `cd` command (breadcrumb navigation).
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Maps a [`TabKind`] onto the palette crate's owned tab-kind enum so
/// `crates/ui` never passes its `TabKind` across the crate boundary (T16-004).
fn palette_tab_kind(kind: TabKind) -> labonair_command_palette::PaletteTabKind {
    use labonair_command_palette::PaletteTabKind as K;
    match kind {
        TabKind::Workspace => K::Workspace,
        TabKind::Editor => K::Editor,
        TabKind::Sftp => K::Sftp,
        TabKind::Hosts => K::Home,
        _ => K::Other,
    }
}

/// Bridges [`Workspace`] to the `labonair-command-palette` view's
/// [`PaletteWorkspace`](labonair_command_palette::PaletteWorkspace) contract
/// (T16-004 decoupling). Both accessors mirror what the palette view read
/// directly from `Workspace` before the split.
impl labonair_command_palette::PaletteWorkspace for Workspace {
    fn palette_active_context(&self, cx: &App) -> Option<labonair_command_palette::CommandContext> {
        self.active_context(cx)
    }

    fn palette_tab_rows(&self, cx: &App) -> Vec<labonair_command_palette::PaletteTabRow> {
        self.tabs
            .read(cx)
            .tabs()
            .iter()
            .map(|t| labonair_command_palette::PaletteTabRow {
                id: t.id,
                label: t.label(),
                kind_title: t.kind.default_title().to_string(),
                is_ssh: self.ssh_tabs.values().any(|s| s.tab_id == t.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{shell_quote, ssh_tab_title};

    #[test]
    fn shell_quote_wraps_and_escapes() {
        assert_eq!(shell_quote("/a/b"), "'/a/b'");
        assert_eq!(shell_quote("/it's here"), "'/it'\\''s here'");
    }

    #[test]
    fn ssh_tab_title_annotates_jump_route() {
        assert_eq!(ssh_tab_title("prod-web", None), "SSH \u{00b7} prod-web");
        assert_eq!(
            ssh_tab_title("prod-web", Some("bastion")),
            "SSH \u{00b7} prod-web  \u{2192} bastion"
        );
    }
}
