//! [`AppShell`] — the app's root coordinator (T04-003).
//!
//! Mirrors `reference-src/src/app/components/AppShell.tsx`: it is *only*
//! composition. It wires together, top to bottom:
//!
//! * a **header** bar (`bg-toolbar`, `h-10`) — sidebar toggle, app title,
//!   inline search, menu affordance;
//! * a **body** row — an optional, resizable left sidebar (a panel switcher
//!   rail + the active panel's content, both slots for later phases) next to
//!   the [`Workspace`] (tab bar + split-pane content, from T04-001/002);
//! * a **status bar** (`bg-status-bar`, `h-8`) — the active pane's cwd
//!   breadcrumb on the left, pane count + (empty) connection / AI slots on
//!   the right.
//!
//! No feature logic lives here — the header's inline search is forwarded to
//! [`Workspace::search_active`], the breadcrumb reads [`Workspace::active_cwd`],
//! etc. Window geometry is persisted via [`crate::window_state`] so the window
//! size/position survive a restart (full session persistence is T14-001).

use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, Bounds, ClickEvent, Context, DragMoveEvent, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds,
};
use labonair_backend::modules::mcp::{
    mcp_set_auto_revoke_minutes, mcp_set_enabled, mcp_set_max_command_timeout_secs, mcp_set_port,
};
use labonair_backend::modules::settings::mcp::mcp_prefs_load;
use labonair_backend::App as Backend;
use labonair_terminal::TerminalRegistry;
use tokio::runtime::Handle as TokioHandle;

use crate::agent_access::{AgentAccessEntry, AgentAccessStore};
use crate::ai_chat::{AiChatStore, AiChatView};
use crate::background::{BackgroundStore, LayerScope};
use crate::command_palette::{CommandId, CommandPalette, PaletteEvent};
use crate::explorer::ExplorerView;
use crate::git::GitPanelView;
use crate::git_graph::GitGraphView;
use crate::menu;
use crate::notifications::{self, NotificationCenter};
use crate::pane::SplitAxis;
use crate::snippets::SnippetsView;
use crate::theme::ThemeStore;
use crate::window_state;
use crate::workspace::Workspace;

const HEADER_H: f32 = 40.0;
const STATUS_H: f32 = 32.0;
/// Left inset reserved for the macOS traffic-light buttons.
const TRAFFIC_LIGHT_INSET: f32 = 78.0;
/// Panel-switcher rail width.
const RAIL_W: f32 = 44.0;
const SIDEBAR_DEFAULT: f32 = 260.0;
const SIDEBAR_MIN: f32 = 180.0;
const SIDEBAR_MAX: f32 = 520.0;
/// Sidebar resize-handle thickness.
const HANDLE: f32 = 6.0;
/// Minimum interval between window-geometry writes.
const SAVE_THROTTLE: Duration = Duration::from_millis(1000);

/// A dockable sidebar panel. Later phases register their panel by adding a
/// variant here + an arm in [`AppShell::render_panel_body`]; the switcher rail
/// and toggle logic then pick it up automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    Explorer,
    Snippets,
    SourceControl,
    GitGraph,
    Ai,
}

impl SidebarPanel {
    /// All panels, in rail order.
    const ALL: [SidebarPanel; 5] = [
        SidebarPanel::Explorer,
        SidebarPanel::Snippets,
        SidebarPanel::SourceControl,
        SidebarPanel::GitGraph,
        SidebarPanel::Ai,
    ];

    fn label(self) -> &'static str {
        match self {
            SidebarPanel::Explorer => "Explorer",
            SidebarPanel::Snippets => "Snippets",
            SidebarPanel::SourceControl => "Source Control",
            SidebarPanel::GitGraph => "Git Graph",
            SidebarPanel::Ai => "AI",
        }
    }

    /// A single glyph for the switcher rail (real icons arrive with the panels).
    fn glyph(self) -> &'static str {
        match self {
            SidebarPanel::Explorer => "\u{1F4C1}",
            SidebarPanel::Snippets => "\u{2702}",
            SidebarPanel::SourceControl => "\u{2325}",
            SidebarPanel::GitGraph => "\u{26D3}",
            SidebarPanel::Ai => "\u{2728}",
        }
    }
}

/// Value carried while dragging the sidebar's edge handle.
struct SidebarResize;

struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The root view: window chrome around the [`Workspace`].
pub struct AppShell {
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    notifications: Entity<NotificationCenter>,
    workspace: Entity<Workspace>,
    explorer: Entity<ExplorerView>,
    git_panel: Entity<GitPanelView>,
    git_graph: Entity<GitGraphView>,
    snippets: Entity<SnippetsView>,
    ai_chat: Entity<AiChatView>,
    command_palette: Entity<CommandPalette>,
    /// Palette picks awaiting a `&mut Window` (drained in `render`) — same
    /// pattern `Workspace` uses for its window-less subscriptions.
    pending_commands: Vec<PaletteEvent>,
    /// Client-side mirror of the MCP bridge's per-tab agent-access grants,
    /// shared with `Workspace` (T11-006).
    agent_access: Entity<AgentAccessStore>,
    /// Whether the header agent-access badge popover is open.
    agent_badge_open: bool,
    sidebar_open: bool,
    sidebar_width: f32,
    active_panel: SidebarPanel,
    search_open: bool,
    search_query: String,
    search_focus: FocusHandle,
    focus_handle: FocusHandle,
    last_saved: Option<(Bounds<Pixels>, Instant)>,
}

impl AppShell {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        notifications: Entity<NotificationCenter>,
        backend: Backend,
        tokio: TokioHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();
        cx.observe(&notifications, |_, _, cx| cx.notify()).detach();

        // Demo: a startup toast proves the system is reachable from anywhere
        // (acceptance criterion). Debug builds only.
        #[cfg(debug_assertions)]
        notifications.update(cx, |center, cx| {
            center.push(
                crate::notifications::Notification::info(
                    "Welcome to Labonair",
                    "Notifications appear here. This demo toast auto-dismisses.",
                ),
                cx,
            );
        });

        let agent_access = cx.new(|_| AgentAccessStore::new(backend.clone(), tokio.clone()));
        cx.observe(&agent_access, |_, _, cx| cx.notify()).detach();

        // The Rust `McpState` boots with no persistence of its own — mirror the
        // saved preferences into it once at startup (matches the reference
        // `useMcpTabBridge.ts` re-sync). Port/timeout/auto-revoke first so the
        // listener, if enabled, comes up on the right port.
        {
            let prefs = mcp_prefs_load();
            agent_access.update(cx, |s, cx| {
                s.hydrate(prefs.bridge_enabled, prefs.notify_on_activity, cx)
            });
            let app = backend.clone();
            tokio.spawn(async move {
                let _ = mcp_set_port(prefs.bridge_port, app.clone(), &app.mcp, &app.secrets).await;
                let _ = mcp_set_max_command_timeout_secs(prefs.max_command_timeout_secs, &app.mcp)
                    .await;
                let _ = mcp_set_auto_revoke_minutes(prefs.auto_revoke_minutes, &app.mcp).await;
                if prefs.bridge_enabled {
                    let _ = mcp_set_enabled(true, app.clone(), &app.mcp, &app.secrets).await;
                }
            });
        }

        let registry = Arc::new(TerminalRegistry::new());
        let workspace = cx.new(|cx| {
            Workspace::new(
                registry,
                theme.clone(),
                background.clone(),
                backend.clone(),
                tokio.clone(),
                agent_access.clone(),
                window,
                cx,
            )
        });
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();

        let git_panel =
            cx.new(|cx| GitPanelView::new(backend.clone(), tokio.clone(), theme.clone(), cx));
        cx.observe(&git_panel, |_, _, cx| cx.notify()).detach();

        let git_graph =
            cx.new(|cx| GitGraphView::new(backend.clone(), tokio.clone(), theme.clone(), cx));
        cx.observe(&git_graph, |_, _, cx| cx.notify()).detach();

        let snippets = cx.new(|cx| {
            SnippetsView::new(backend, tokio.clone(), theme.clone(), workspace.clone(), cx)
        });
        cx.observe(&snippets, |_, _, cx| cx.notify()).detach();

        let ai_store = cx.new(|_| AiChatStore::new(tokio));
        let ai_chat = cx.new(|cx| AiChatView::new(ai_store, theme.clone(), cx));
        cx.observe(&ai_chat, |_, _, cx| cx.notify()).detach();

        let command_palette =
            cx.new(|cx| CommandPalette::new(theme.clone(), workspace.clone(), cx));
        cx.observe(&command_palette, |_, _, cx| cx.notify())
            .detach();
        cx.subscribe(&command_palette, |this, _, event: &PaletteEvent, cx| {
            this.pending_commands.push(event.clone());
            cx.notify();
        })
        .detach();

        let explorer = cx.new(|cx| ExplorerView::new(theme.clone(), workspace.clone(), cx));
        cx.observe(&explorer, |_, _, cx| cx.notify()).detach();
        // Root tracks the active terminal's cwd (falls back to $HOME).
        {
            let initial = workspace
                .read(cx)
                .active_cwd(cx)
                .or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().to_string()));
            explorer.update(cx, |e, cx| e.set_root_str(initial, cx));
        }
        cx.observe(&workspace, {
            let explorer = explorer.clone();
            move |_, workspace, cx| {
                let cwd = workspace
                    .read(cx)
                    .active_cwd(cx)
                    .or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().to_string()));
                explorer.update(cx, |e, cx| e.set_root_str(cwd, cx));
            }
        })
        .detach();
        cx.observe(&workspace, {
            let git_panel = git_panel.clone();
            let git_graph = git_graph.clone();
            move |_, workspace, cx| {
                let cwd = workspace.read(cx).active_cwd(cx);
                git_panel.update(cx, |g, cx| g.set_root(cwd.clone(), cx));
                git_graph.update(cx, |g, cx| g.set_root(cwd, cx));
            }
        })
        .detach();
        {
            let cwd = workspace.read(cx).active_cwd(cx);
            git_panel.update(cx, |g, cx| g.set_root(cwd.clone(), cx));
            git_graph.update(cx, |g, cx| g.set_root(cwd, cx));
        }

        // Persist the final window geometry on close (the throttled per-render
        // save covers force-quit within the last second).
        window.on_window_should_close(cx, |window, _cx| {
            if let WindowBounds::Windowed(bounds) = window.window_bounds() {
                window_state::save(bounds);
            }
            true
        });

        Self {
            theme,
            background,
            notifications,
            workspace,
            explorer,
            git_panel,
            git_graph,
            snippets,
            ai_chat,
            command_palette,
            pending_commands: Vec::new(),
            agent_access,
            agent_badge_open: false,
            sidebar_open: true,
            sidebar_width: SIDEBAR_DEFAULT,
            active_panel: SidebarPanel::Explorer,
            search_open: false,
            search_query: String::new(),
            search_focus: cx.focus_handle(),
            focus_handle: cx.focus_handle(),
            last_saved: None,
        }
    }

    /// The workspace view (for later command-palette / menu wiring).
    pub fn workspace(&self) -> &Entity<Workspace> {
        &self.workspace
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn select_panel(&mut self, panel: SidebarPanel, cx: &mut Context<Self>) {
        if self.active_panel == panel && self.sidebar_open {
            self.sidebar_open = false;
        } else {
            self.active_panel = panel;
            self.sidebar_open = true;
        }
        cx.notify();
    }

    fn set_sidebar_width(&mut self, width: f32, cx: &mut Context<Self>) {
        let clamped = (width - RAIL_W).clamp(SIDEBAR_MIN, SIDEBAR_MAX);
        if (clamped - self.sidebar_width).abs() > 0.5 {
            self.sidebar_width = clamped;
            cx.notify();
        }
    }

    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = true;
        window.focus(&self.search_focus);
        cx.notify();
    }

    fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_query.clear();
        self.workspace.update(cx, |w, cx| w.search_active("", cx));
        self.workspace.update(cx, |w, cx| w.focus(window, cx));
        cx.notify();
    }

    fn run_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_query.clone();
        self.workspace
            .update(cx, |w, cx| w.search_active(&query, cx));
    }

    // ── Menu / shortcut action handlers (T04-005) ──────────────────────────
    // Bound as GPUI actions on the root element in `render`, so the native
    // menu bar and the keyboard shortcuts run identical code.

    fn act_new_terminal_tab(
        &mut self,
        _: &menu::NewTerminalTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_terminal_tab(window, cx));
    }

    fn act_close_tab(&mut self, _: &menu::CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace
            .update(cx, |w, cx| w.close_active(window, cx));
    }

    fn act_close_pane(&mut self, _: &menu::ClosePane, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.close_pane(window, cx));
    }

    fn act_split_right(
        &mut self,
        _: &menu::SplitPaneRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.split(SplitAxis::Horizontal, window, cx));
    }

    fn act_split_down(
        &mut self,
        _: &menu::SplitPaneDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.split(SplitAxis::Vertical, window, cx));
    }

    fn act_find(&mut self, _: &menu::Find, window: &mut Window, cx: &mut Context<Self>) {
        let handled = self
            .workspace
            .update(cx, |w, cx| w.find_in_active_editor(cx));
        if !handled {
            self.open_search(window, cx);
        }
    }

    fn act_save(&mut self, _: &menu::Save, _: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.save_active(cx));
    }

    fn act_new_editor_tab(
        &mut self,
        _: &menu::NewEditorTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_editor_tab(window, cx));
    }

    fn act_toggle_sidebar(
        &mut self,
        _: &menu::ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar(cx);
    }

    fn act_toggle_fullscreen(
        &mut self,
        _: &menu::ToggleFullScreen,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.toggle_fullscreen();
    }

    fn act_minimize(&mut self, _: &menu::Minimize, window: &mut Window, _: &mut Context<Self>) {
        window.minimize_window();
    }

    fn act_zoom_window(
        &mut self,
        _: &menu::ZoomWindow,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.zoom_window();
    }

    fn act_next_tab(&mut self, _: &menu::NextTab, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.cycle(true, window, cx));
    }

    fn act_prev_tab(&mut self, _: &menu::PrevTab, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace
            .update(cx, |w, cx| w.cycle(false, window, cx));
    }

    fn act_command_palette(
        &mut self,
        _: &menu::CommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.command_palette
            .update(cx, |p, cx| p.toggle(window, cx));
    }

    /// Drain palette picks queued by the `PaletteEvent` subscription, now
    /// that a `&mut Window` is available (called from `render`).
    fn drain_pending_commands(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for event in std::mem::take(&mut self.pending_commands) {
            match event {
                PaletteEvent::SwitchToTab(id) => {
                    self.workspace
                        .update(cx, |w, cx| w.reveal_tab(id, window, cx));
                }
                PaletteEvent::Run(id) => self.run_palette_command(id, window, cx),
            }
        }
    }

    fn run_palette_command(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        // Commands that map onto a menu action dispatch it — identical code
        // path as the native menu, and later phases that add the handler
        // light the command up for free (same "one source of truth" the
        // menu module documents). The rest we service directly.
        macro_rules! dispatch {
            ($action:expr) => {
                window.dispatch_action(Box::new($action), cx)
            };
        }
        match id {
            CommandId::NewTerminalTab => dispatch!(menu::NewTerminalTab),
            CommandId::NewEditorTab => dispatch!(menu::NewEditorTab),
            CommandId::CloseTab => dispatch!(menu::CloseTab),
            CommandId::ClosePane => dispatch!(menu::ClosePane),
            CommandId::SplitRight => dispatch!(menu::SplitPaneRight),
            CommandId::SplitDown => dispatch!(menu::SplitPaneDown),
            CommandId::NextTab => dispatch!(menu::NextTab),
            CommandId::PrevTab => dispatch!(menu::PrevTab),
            CommandId::Find => dispatch!(menu::Find),
            CommandId::ToggleSidebar => dispatch!(menu::ToggleSidebar),
            CommandId::ToggleFullScreen => dispatch!(menu::ToggleFullScreen),
            CommandId::ZoomIn => dispatch!(menu::ZoomIn),
            CommandId::ZoomOut => dispatch!(menu::ZoomOut),
            CommandId::ZoomReset => dispatch!(menu::ResetZoom),
            CommandId::AskSelection => dispatch!(menu::AskAboutSelection),
            CommandId::NewAiSession => dispatch!(menu::NewAiSession),
            CommandId::OpenHostManager => dispatch!(menu::OpenHostManager),
            CommandId::OpenShortcuts => dispatch!(menu::OpenShortcuts),
            CommandId::OpenSettings => dispatch!(menu::OpenSettings),
            CommandId::DuplicateTab => self
                .workspace
                .update(cx, |w, cx| w.duplicate_active_tab(window, cx)),
            CommandId::CloseOtherTabs => self
                .workspace
                .update(cx, |w, cx| w.close_other_tabs(window, cx)),
            CommandId::ClearTerminal => self
                .workspace
                .update(cx, |w, cx| w.clear_active_terminal(cx)),
            CommandId::ToggleAiPanel => self.select_panel(SidebarPanel::Ai, cx),
            CommandId::OpenSnippetsPanel => self.select_panel(SidebarPanel::Snippets, cx),
            CommandId::OpenGitGraph => self.select_panel(SidebarPanel::GitGraph, cx),
            CommandId::FocusSourceControl => self.select_panel(SidebarPanel::SourceControl, cx),
            // Resolved inside the palette (opens a follow-up page).
            CommandId::SwitchTab => {}
            // No handler yet — the editor formatter arrives with its phase,
            // at which point this command starts working (see menu.rs on the
            // same "stub now, wire later" convention).
            CommandId::FormatDocument => {}
        }
    }

    fn on_search_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        if m.control || m.alt {
            return;
        }
        if m.platform {
            // Let Cmd-F / Cmd-W etc. bubble; don't type them.
            return;
        }
        match ks.key.as_str() {
            "escape" => self.close_search(window, cx),
            "enter" => self.run_search(cx),
            "backspace" => {
                self.search_query.pop();
                self.run_search(cx);
                cx.notify();
            }
            key => {
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(ch) = ch {
                    self.search_query.push_str(&ch);
                    self.run_search(cx);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    /// Save the window geometry at most once per [`SAVE_THROTTLE`].
    fn maybe_persist_geometry(&mut self, window: &Window) {
        let WindowBounds::Windowed(bounds) = window.window_bounds() else {
            return;
        };
        let now = Instant::now();
        let stale = match self.last_saved {
            None => true,
            Some((last, at)) => {
                now.duration_since(at) >= SAVE_THROTTLE && bounds_differ(last, bounds)
            }
        };
        if stale {
            window_state::save(bounds);
            self.last_saved = Some((bounds, now));
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_header(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (toolbar, fg, muted, border) = (
            theme.toolbar(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
        );

        let aa = self.agent_access.read(cx);
        let agent_entries = aa.entries();
        let agent_badge_visible = aa.bridge_enabled() && !agent_entries.is_empty();
        let agent_badge_open = self.agent_badge_open;

        div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(HEADER_H))
            .w_full()
            .flex_shrink_0()
            .pl(px(TRAFFIC_LIGHT_INSET))
            .pr_2()
            .bg(toolbar)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .id("sidebar-toggle")
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{2630}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.toggle_sidebar(cx);
                    })),
            )
            .child(div().text_xs().text_color(fg).child("Labonair"))
            .child(div().flex_1())
            .when(self.search_open, |d| d.child(self.render_search(cx)))
            .when(agent_badge_visible, |d| {
                d.child(self.render_agent_badge(agent_entries, agent_badge_open, cx))
            })
            .child(
                div()
                    .id("app-menu")
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{22EF}"),
            )
    }

    /// Header badge listing the SSH/local tabs the user has granted MCP agent
    /// access to — click to open a popover to jump to or revoke each one.
    /// Port of the reference `AgentAccessBadge`; hidden entirely (by the
    /// caller) when the bridge is off or nothing is granted.
    fn render_agent_badge(
        &mut self,
        entries: Vec<AgentAccessEntry>,
        open: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, border, card, accent) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.card(),
            theme.accent(),
        );
        let count = entries.len();

        div()
            .relative()
            .flex_shrink_0()
            .child(
                div()
                    .id("agent-access-badge")
                    .relative()
                    .size(px(24.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child("\u{1F6E1}")
                    .child(
                        div()
                            .absolute()
                            .top(px(-2.0))
                            .right(px(-2.0))
                            .min_w(px(13.0))
                            .h(px(13.0))
                            .px(px(2.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(accent)
                            .text_color(fg)
                            .text_size(px(8.0))
                            .child(SharedString::from(count.to_string())),
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.agent_badge_open = !this.agent_badge_open;
                        cx.notify();
                    })),
            )
            .when(open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(28.0))
                        .right(px(0.0))
                        .w(px(300.0))
                        .flex()
                        .flex_col()
                        .rounded_md()
                        .bg(card)
                        .border_1()
                        .border_color(border)
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(border)
                                .text_xs()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(fg)
                                .child("AI Agent Access"),
                        )
                        .children(entries.into_iter().map(|entry| {
                            let tab_id = entry.tab_id;
                            let session_id = entry.session_id.clone();
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .hover(|s| s.bg(border))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("agent-jump-{tab_id}")))
                                        .flex_1()
                                        .min_w_0()
                                        .text_xs()
                                        .text_color(fg)
                                        .truncate()
                                        .child(SharedString::from(entry.label.clone()))
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, window, cx| {
                                                this.agent_badge_open = false;
                                                this.workspace.update(cx, |w, cx| {
                                                    w.reveal_tab(tab_id, window, cx)
                                                });
                                                cx.notify();
                                            },
                                        )),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!("agent-revoke-{tab_id}")))
                                        .px_1()
                                        .rounded_sm()
                                        .text_xs()
                                        .text_color(muted)
                                        .hover(|s| s.text_color(fg))
                                        .child("\u{2715}")
                                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                            let session_id = session_id.clone();
                                            this.agent_access.update(cx, |s, cx| {
                                                s.set_grant(
                                                    tab_id,
                                                    session_id,
                                                    false,
                                                    String::new(),
                                                    labonair_backend::modules::mcp::SessionKind::Ssh,
                                                    None,
                                                    None,
                                                    cx,
                                                );
                                            });
                                            cx.notify();
                                        })),
                                )
                        })),
                )
            })
    }

    fn render_search(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (bg, fg, muted, ring) = (
            theme.background(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.accent(),
        );
        let is_terminal = self.workspace.read(cx).active_is_terminal(cx);
        let placeholder = if is_terminal {
            "Search terminal\u{2026}"
        } else {
            "Search\u{2026}"
        };
        let (text, color) = if self.search_query.is_empty() {
            (placeholder.to_string(), muted)
        } else {
            (self.search_query.clone(), fg)
        };

        div()
            .id("header-search")
            .track_focus(&self.search_focus)
            .key_context("HeaderSearch")
            .flex()
            .items_center()
            .h(px(24.0))
            .w(px(240.0))
            .px_2()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(ring)
            .text_xs()
            .text_color(color)
            .child(SharedString::from(text))
            .on_key_down(cx.listener(Self::on_search_key))
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (sidebar_bg, sidebar_fg, sidebar_border, accent, muted) = (
            theme.sidebar_bg(),
            theme.sidebar_fg(),
            theme.sidebar_border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let active = self.active_panel;

        let rail = div()
            .w(px(RAIL_W))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .py_2()
            .border_r_1()
            .border_color(sidebar_border)
            .children(SidebarPanel::ALL.into_iter().map(|panel| {
                let is_active = panel == active;
                div()
                    .id(SharedString::from(panel.label()))
                    .size(px(30.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(if is_active { sidebar_fg } else { muted })
                    .when(is_active, |d| d.bg(accent))
                    .when(!is_active, |d| d.hover(|s| s.bg(sidebar_border)))
                    .child(panel.glyph())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.select_panel(panel, cx);
                    }))
            }));

        let panel = div()
            .w(px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(sidebar_bg)
            .text_color(sidebar_fg)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(active.label().to_uppercase())),
            )
            .child(self.render_panel_body(active, cx));

        let handle = div()
            .id("sidebar-handle")
            .w(px(HANDLE))
            .h_full()
            .flex_shrink_0()
            .cursor_col_resize()
            .bg(sidebar_border)
            .hover(|s| s.bg(accent))
            .on_drag(SidebarResize, |_, _, _, cx| cx.new(|_| DragGhost));

        div()
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_row()
            .child(rail)
            .child(panel)
            .child(handle)
    }

    /// Placeholder body for each sidebar panel. Later phases replace the arm
    /// for their panel with the real view.
    fn render_panel_body(&self, panel: SidebarPanel, cx: &mut Context<Self>) -> gpui::AnyElement {
        if panel == SidebarPanel::Explorer {
            return self.explorer.clone().into_any_element();
        }
        if panel == SidebarPanel::SourceControl {
            return self.git_panel.clone().into_any_element();
        }
        if panel == SidebarPanel::GitGraph {
            return self.git_graph.clone().into_any_element();
        }
        if panel == SidebarPanel::Snippets {
            return self.snippets.clone().into_any_element();
        }
        if panel == SidebarPanel::Ai {
            return self.ai_chat.clone().into_any_element();
        }
        let muted = self.theme.read(cx).muted_foreground();
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .px_3()
            .text_center()
            .text_xs()
            .text_color(muted)
            .child(SharedString::from(format!(
                "{} \u{2014} coming in a later phase",
                panel.label()
            )))
            .into_any_element()
    }

    fn render_statusbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let workspace = self.workspace.read(cx);
        let cwd = workspace.active_cwd(cx);
        let panes = workspace.active_pane_count(cx);
        let label = workspace.active_tab_label(cx);

        let theme = self.theme.read(cx);
        let (status_bar, fg, muted, border) = (
            theme.status_bar(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
        );

        let breadcrumb = match cwd.as_deref().map(display_path) {
            Some(path) => {
                let segments: Vec<SharedString> = path
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .map(|s| SharedString::from(s.to_string()))
                    .collect();
                let last = segments.len().saturating_sub(1);
                div()
                    .flex()
                    .items_center()
                    .min_w_0()
                    .when(path.starts_with('/'), |d| {
                        d.child(div().text_color(muted).child("/"))
                    })
                    .children(segments.into_iter().enumerate().map(|(i, seg)| {
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_color(if i == last { fg } else { muted })
                                    .child(seg),
                            )
                            .when(i != last, |d| {
                                d.child(div().px_0p5().text_color(muted).child("/"))
                            })
                    }))
                    .into_any_element()
            }
            None => div()
                .text_color(muted)
                .child(SharedString::from(label))
                .into_any_element(),
        };

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(STATUS_H))
            .w_full()
            .flex_shrink_0()
            .px_3()
            .bg(status_bar)
            .border_t_1()
            .border_color(border)
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .child(breadcrumb),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    // Connection / jump-host / AI badge slots stay empty until
                    // their phases (06 / 10) provide data.
                    .when(panes > 1, |d| {
                        d.child(SharedString::from(format!("{panes} panes")))
                    }),
            )
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.maybe_persist_geometry(window);
        self.drain_pending_commands(window, cx);

        let bg = self.theme.read(cx).background();
        let background_layer = self.background.read(cx).layer(LayerScope::App);
        let toasts = notifications::render_overlay(&self.notifications, &self.theme, cx);
        let header = self.render_header(cx);
        let sidebar = self
            .sidebar_open
            .then(|| self.render_sidebar(cx).into_any_element());
        let statusbar = self.render_statusbar(cx);
        let workspace = self.workspace.clone();
        let can_split = self.workspace.read(cx).active_is_terminal(cx);
        let has_split = self.workspace.read(cx).active_has_split(cx);

        div()
            .track_focus(&self.focus_handle)
            .key_context("AppShell")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .text_xs()
            .on_action(cx.listener(Self::act_new_terminal_tab))
            .on_action(cx.listener(Self::act_new_editor_tab))
            .on_action(cx.listener(Self::act_save))
            .on_action(cx.listener(Self::act_close_tab))
            .on_action(cx.listener(Self::act_find))
            .on_action(cx.listener(Self::act_toggle_sidebar))
            .on_action(cx.listener(Self::act_toggle_fullscreen))
            .on_action(cx.listener(Self::act_minimize))
            .on_action(cx.listener(Self::act_zoom_window))
            .on_action(cx.listener(Self::act_next_tab))
            .on_action(cx.listener(Self::act_prev_tab))
            .on_action(cx.listener(Self::act_command_palette))
            .when(can_split, |d| {
                d.on_action(cx.listener(Self::act_split_right))
                    .on_action(cx.listener(Self::act_split_down))
            })
            .when(has_split, |d| {
                d.on_action(cx.listener(Self::act_close_pane))
            })
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .children(sidebar)
                    .child(div().flex_1().min_w_0().child(workspace))
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<SidebarResize>, _window, cx| {
                            let w = f32::from(ev.event.position.x - ev.bounds.origin.x);
                            this.set_sidebar_width(w, cx);
                        },
                    )),
            )
            .child(statusbar)
            .children(background_layer)
            .child(self.command_palette.clone())
            .children(toasts)
    }
}

/// Substitute the user's home directory with `~` for display.
fn display_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir().and_then(|p| p.to_str().map(str::to_string)) {
        if path == home {
            return "~".to_string();
        }
        if let Some(rest) = path.strip_prefix(&format!("{home}/")) {
            return format!("~/{rest}");
        }
    }
    path.to_string()
}

fn bounds_differ(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    let d = |x: Pixels, y: Pixels| (f32::from(x) - f32::from(y)).abs() > 2.0;
    d(a.origin.x, b.origin.x)
        || d(a.origin.y, b.origin.y)
        || d(a.size.width, b.size.width)
        || d(a.size.height, b.size.height)
}
