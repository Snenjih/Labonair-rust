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

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, relative, App, AppContext, ClickEvent, Context, DragMoveEvent, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Task, Window,
};
use labonair_backend::modules::ssh::client::{ssh_connect, ssh_disconnect, ssh_trust_host};
use labonair_backend::modules::ssh::pty::SshPtyEvent;
use labonair_backend::{App as Backend, AppEvent, EventChannel};
use labonair_terminal::{
    RemoteFeed, RemoteResizer, RemoteWriter, SessionHandle, SessionId, SessionOptions,
    TermDimensions, TerminalColors, TerminalRegistry,
};
use tokio::runtime::Handle as TokioHandle;

use crate::background::BackgroundStore;
use crate::editor::{EditorEvent, EditorView};
use crate::hosts::{HostManagerEvent, HostManagerView, HostStatus};
use crate::pane::{CloseOutcome, PaneId, PaneNode, SplitAxis, WorkspaceLayout};
use crate::tabs::{Tab, TabData, TabKind, TabStore};
use crate::terminal::TerminalView;
use crate::theme::ThemeStore;

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

/// Thickness of a split-divider resize handle.
const HANDLE: f32 = 6.0;

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
}

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Arc<TerminalRegistry>,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        backend: Backend,
        tokio: TokioHandle,
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
            },
        )
        .detach();

        // Forward the backend's broadcast event bus into a plain channel the
        // GPUI poll loop can drain without an async runtime.
        let (ev_tx, ev_rx) = std::sync::mpsc::channel::<AppEvent>();
        {
            let mut bus = backend.events.subscribe();
            tokio.spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match bus.recv().await {
                        Ok(raw) => {
                            if let Some(ev) = AppEvent::from_raw(&raw) {
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
                })
                .is_ok();
            if !ok {
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
            next_pane_id: 1,
            confirm_close: None,
            context_menu: None,
            focus_handle: cx.focus_handle(),
            _meta_sync: meta_sync,
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
        };
        // Landing tab: the host-manager dashboard.
        this.tabs
            .update(cx, |s, cx| s.open(TabKind::Home, TabData::default(), cx));
        this.open_terminal_tab(window, cx);
        this
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
        let Some((session_id, handle)) = self.spawn_session(cwd.clone(), cx) else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(pane_id, PaneEntry { session_id, view });
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
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
                    this.retire_tab(&removed);
                }
                cx.notify();
                return;
            }
            let dirty = view.read(cx).is_dirty();
            this.tabs.update(cx, |s, cx| {
                s.set_dirty(tab_id, dirty, cx);
                if matches!(ev, EditorEvent::Edited) {
                    s.set_peek(tab_id, false, cx);
                }
                if let Some(title) = view
                    .read(cx)
                    .path()
                    .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
                {
                    s.set_custom_title(tab_id, Some(title), cx);
                }
            });
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

    /// Spawn a local terminal session in `cwd` and return its id + handle.
    fn spawn_session(&self, cwd: Option<String>, cx: &App) -> Option<(SessionId, SessionHandle)> {
        let options = SessionOptions {
            working_directory: cwd,
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
        Some((session_id, handle))
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
        let Some((session_id, handle)) = self.spawn_session(cwd.clone(), cx) else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(pane_id, PaneEntry { session_id, view });
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
        let Some((session_id, handle)) = self.spawn_session(cwd, cx) else {
            return;
        };
        let split_id = self.alloc_pane();
        let new_pane = self.alloc_pane();
        if let Some(layout) = self.layouts.get_mut(&tab_id) {
            layout.split(split_id, new_pane, axis);
        }
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(new_pane, PaneEntry { session_id, view });
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
            self.retire_tab(&removed);
        }
        self.focus_active(window, cx);
    }

    fn close_others(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_others(id, cx));
        for tab in &removed {
            self.retire_tab(tab);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    fn close_by_kind(&mut self, kind: TabKind, window: &mut Window, cx: &mut Context<Self>) {
        let removed = self.tabs.update(cx, |s, cx| s.close_by_kind(kind, cx));
        for tab in &removed {
            self.retire_tab(tab);
        }
        self.confirm_close = None;
        self.focus_active(window, cx);
    }

    /// Tear down one pane's session + content view.
    fn retire_pane(&mut self, pane_id: PaneId) {
        if let Some(entry) = self.panes.remove(&pane_id) {
            self.registry.close(entry.session_id);
        }
    }

    /// Tear down a removed tab's whole pane tree.
    fn retire_tab(&mut self, tab: &Tab) {
        if let Some(layout) = self.layouts.remove(&tab.id) {
            for leaf in layout.leaves() {
                self.retire_pane(leaf);
            }
        }
        self.editors.remove(&tab.id);

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
                self.tokio.spawn(async move {
                    let _ = ssh_disconnect(ssh_id, &app.ssh).await;
                });
                if let Some(p) = &self.ssh_prompt {
                    if p.ssh_id() == t.ssh_id {
                        self.ssh_prompt = None;
                    }
                }
            }
        }
    }

    // ── SSH connection flow (T07-001) ──────────────────────────────────────

    fn set_host_status(&mut self, host_id: &str, status: HostStatus, cx: &mut Context<Self>) {
        self.host_manager
            .update(cx, |h, cx| h.set_status(host_id, status, cx));
    }

    /// Open an SSH terminal tab for `host_id` and start the connection.
    fn connect_host(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
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
        let Some(handle) = self.registry.handle(session_id) else {
            return;
        };
        feed.feed(b"Connecting\xe2\x80\xa6\r\n");

        let pane_id = self.alloc_pane();
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
            s.set_custom_title(id, Some(format!("SSH · {host_id}")), cx);
            id
        });
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(pane_id, PaneEntry { session_id, view });
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
        self.spawn_ssh_connect(ssh_id, host_id, None, None, feed, cx);
        self.focus_active(window, cx);
        cx.notify();
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
                }
                if self.ssh_prompt.as_ref().map(|p| p.ssh_id()) == Some(session_id.as_str()) {
                    self.ssh_prompt = None;
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
            .bg(gpui::rgba(0x000000aa))
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
            .bg(gpui::rgba(0x00000080))
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
                    }),
            )
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
            self.connect_host(host_id, window, cx);
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
