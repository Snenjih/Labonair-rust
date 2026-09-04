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

use crate::background::{BackgroundStore, LayerScope};
use crate::bar_items;
use crate::menu;
use crate::pane::SplitAxis;
use crate::theme::{ThemePreference, ThemeStore};
use crate::updater::UpdaterView;
use crate::window_state;
use crate::workspace::Workspace;
use labonair_command_palette::{
    CommandId, CommandPalette, Page as PalettePage, PaletteChoice, PaletteData, PaletteEvent,
};
use labonair_notifications::{self as notifications, NotificationCenter};
use labonair_panel::DockPosition;
use labonair_panel_ai::{AiChatEvent, AiChatStore, AiChatView};
use labonair_panel_explorer::{BookmarkEvent, BookmarksView, ExplorerView};
use labonair_panel_git_graph::GitGraphView;
use labonair_panel_scm::GitPanelView;
use labonair_panel_snippets::SnippetsView;
use labonair_settings_ui::{
    open_settings_window, set_settings_deps, PreferencesStore, SettingsTab,
};
use labonair_ui_kit::IconName;
use labonair_workspace::agent_access::AgentAccessStore;
use labonair_workspace::dock::{position_slug, DockData, RESIZE_HANDLE_SIZE};
use labonair_workspace::status_bar::StatusBar;

use crate::status_items::register_builtin_status_items;

const HEADER_H: f32 = 40.0;
/// Left inset reserved for the macOS traffic-light buttons.
const TRAFFIC_LIGHT_INSET: f32 = 78.0;
/// Minimum interval between window-geometry writes.
const SAVE_THROTTLE: Duration = Duration::from_millis(1000);

/// Register the five built-in panels on the workspace's
/// [`PanelRegistry`](labonair_panel::PanelRegistry).
///
/// This is the **only** place in the app that names concrete panel types.
/// Adding a panel is exactly: a new `labonair-panel-*` crate + an `impl
/// labonair_panel::Panel` for its view + one `reg(&…)` line below (T17-001
/// acceptance criterion — a sixth panel would be a single new array entry).
///
/// The registry constructors hand back a clone of the shell's already-built
/// panel entity rather than lazily spawning a fresh view: the shell keeps
/// direct handles anyway (commands, event subscriptions, cwd feeds), so a
/// second instance would only drift. Cloning an `Entity<T>` is a refcount bump.
fn register_builtin_panels(
    workspace: &Entity<Workspace>,
    explorer: &Entity<ExplorerView>,
    git_panel: &Entity<GitPanelView>,
    git_graph: &Entity<GitGraphView>,
    snippets: &Entity<SnippetsView>,
    ai_chat: &Entity<AiChatView>,
    cx: &mut App,
) {
    use labonair_panel::{AnyPanelHandle, Panel, PanelRegistration};

    fn reg<T: Panel + 'static>(view: &Entity<T>, cx: &App) -> PanelRegistration {
        let handle = view.clone();
        PanelRegistration {
            persistent_name: T::persistent_name(),
            default_position: view.read(cx).position(cx),
            icon: view.read(cx).icon(),
            build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyPanelHandle),
        }
    }

    let registrations = [
        reg(explorer, cx),
        reg(git_panel, cx),
        reg(git_graph, cx),
        reg(snippets, cx),
        reg(ai_chat, cx),
    ];
    workspace.update(cx, |w, _cx| {
        let registry = w.panel_registry_mut();
        for registration in registrations {
            registry.register(registration);
        }
    });
}

/// Value carried while dragging a dock's edge handle — the position tells the
/// drop handler which dock to resize and along which axis.
#[derive(Clone, Copy)]
struct DockResize(DockPosition);

/// Build the persisted [`DockData`] array from the legacy `sidebar_*`
/// preferences (T17-002 first-run migration). Panel membership is left empty so
/// every panel falls back to its registry `default_position`; only the
/// open / size / active-panel state carries over.
fn migrate_dock_layout(
    p: &labonair_backend::modules::settings::preferences::Preferences,
) -> String {
    let migrate_name = |raw: &str, fallback: &str| match raw {
        "hosts" | "tabs" | "" => fallback.to_string(),
        other => other.to_string(),
    };
    let docks = [
        DockData {
            position: "left".to_string(),
            open: p.sidebar_open,
            size: p.sidebar_width as f32,
            zoomed: false,
            active_panel: Some(migrate_name(&p.sidebar_active_panel, "explorer")),
            panel_order: Vec::new(),
        },
        DockData {
            position: "right".to_string(),
            open: p.sidebar_right_open,
            size: p.sidebar_right_width as f32,
            zoomed: false,
            active_panel: Some(migrate_name(&p.sidebar_right_active_panel, "ai")),
            panel_order: Vec::new(),
        },
        DockData {
            position: "bottom".to_string(),
            open: false,
            size: 320.0,
            zoomed: false,
            active_panel: None,
            panel_order: Vec::new(),
        },
    ];
    serde_json::to_string(&docks).unwrap_or_default()
}

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
    bookmarks: Entity<BookmarksView>,
    git_panel: Entity<GitPanelView>,
    snippets: Entity<SnippetsView>,
    ai_chat: Entity<AiChatView>,
    command_palette: Entity<CommandPalette<PreferencesStore, Workspace, ThemeStore>>,
    prefs: Entity<PreferencesStore>,
    updater: Entity<UpdaterView>,
    /// Palette picks awaiting a `&mut Window` (drained in `render`) — same
    /// pattern `Workspace` uses for its window-less subscriptions.
    pending_commands: Vec<PaletteEvent>,
    /// Bookmark picks awaiting a `&mut Window` (drained in `render`).
    pending_bookmarks: Vec<BookmarkEvent>,
    /// AI-panel events (run-in-terminal) awaiting a `&mut Window`.
    pending_ai: Vec<AiChatEvent>,
    /// Real `LiveBridge` for the AI agent — snapshot refreshed + command queue
    /// drained each render.
    live_bridge: crate::live_bridge::WorkspaceLiveBridge,
    /// The status bar (T17-003) — renders purely from the workspace's
    /// [`StatusItemRegistry`](labonair_panel::StatusItemRegistry).
    status_bar: Entity<StatusBar>,
    /// Whether the `⋯` header app-menu dropdown is open.
    app_menu_open: bool,
    search_open: bool,
    search_query: String,
    search_focus: FocusHandle,
    focus_handle: FocusHandle,
    last_saved: Option<(Bounds<Pixels>, Instant)>,
}

/// Generate a `menu::SelectTabN` action handler that jumps to the tab at a
/// fixed 0-based index (T13-005 — `Cmd+1..9`).
macro_rules! select_tab_action {
    ($name:ident, $action:ident, $idx:expr) => {
        fn $name(&mut self, _: &menu::$action, window: &mut Window, cx: &mut Context<Self>) {
            self.workspace
                .update(cx, |w, cx| w.select_tab_by_index($idx, window, cx));
        }
    };
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
                labonair_notifications::Notification::info(
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
        // Session restore (T14-001): load the previous snapshot up-front so the
        // workspace can replay it instead of opening the default tabs.
        let session_snapshot = {
            use labonair_backend::modules::settings::preferences::preferences_load;
            preferences_load()
                .session_restore
                .then(crate::session::load_snapshot)
                .flatten()
        };
        let workspace = cx.new(|cx| {
            Workspace::new(
                registry,
                theme.clone(),
                background.clone(),
                backend.clone(),
                tokio.clone(),
                agent_access.clone(),
                session_snapshot,
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
        // The workspace renders the Git Graph as a `TabKind::GitGraph` tab
        // (Block B) — share this single entity so the app-shell keeps feeding
        // it the active CWD.
        workspace.update(cx, |w, _cx| w.set_git_graph(git_graph.clone()));

        // Central preferences store (T13-001); the settings UI lives in its own
        // OS window (`open_settings_window`). Apply the persisted theme
        // preference to the ThemeStore once at startup; further changes flow
        // through `SettingsView::set_pref`.
        let prefs = cx.new(|_| PreferencesStore::new());
        cx.observe(&prefs, |_, _, cx| cx.notify()).detach();
        {
            use labonair_backend::modules::settings::preferences::ThemePref;
            let pref = match prefs.read(cx).get().theme {
                ThemePref::System => ThemePreference::System,
                ThemePref::Light => ThemePreference::Light,
                ThemePref::Dark => ThemePreference::Dark,
            };
            theme.update(cx, |t, cx| t.set_preference(pref, cx));
        }
        // Publish the global snapshot and push font / editor-syntax settings
        // into the ThemeStore so terminals + editors start with them (T13-003).
        {
            let p = prefs.read(cx).get().clone();
            prefs.update(cx, |s, cx| s.publish_global(cx));
            labonair_settings_ui::apply_prefs_to_theme(&p, &theme, cx);
            // Apply the persisted keyboard-shortcut overrides (T13-004).
            crate::menu::apply_keybinds(cx, &p.keybinds);
        }
        // Publish the shared handles the settings window (its own OS window,
        // T16-009) builds from — it is opened lazily on `Cmd+,`.
        set_settings_deps(prefs.clone(), backend.clone(), tokio.clone(), cx);
        // The Shortcuts pane lives in `labonair-settings-ui` (which cannot depend
        // on this crate's concrete `actions!`); hand it the keybind applier.
        labonair_settings_ui::set_keybind_apply_hook(crate::menu::apply_keybinds, cx);
        // Re-render when the settings window edits the (transitional) bar-item
        // placement blob. T18-005 repurposes this tick to re-resolve each
        // status item's side from `statusBarItemPlacements`; keeping the
        // observer wired here means that reactivity is not lost in the interim.
        cx.observe_global::<bar_items::BarLayoutTick>(|_, cx| cx.notify())
            .detach();

        // Auto-updater (T15-005). Kicks a quiet background check at startup when
        // the `checkForUpdates` preference is on (6 h backoff inside the store).
        let updater = cx.new(|cx| UpdaterView::new(tokio.clone(), theme.clone(), cx));
        cx.observe(&updater, |_, _, cx| cx.notify()).detach();
        if prefs.read(cx).get().check_for_updates {
            updater.update(cx, |u, cx| u.run_check(false, cx));
        }

        let snippets = cx.new(|cx| {
            SnippetsView::new(
                backend.clone(),
                tokio.clone(),
                theme.clone(),
                workspace.clone(),
                cx,
            )
        });
        cx.observe(&snippets, |_, _, cx| cx.notify()).detach();

        let live_bridge = crate::live_bridge::WorkspaceLiveBridge::new();
        let ai_store = cx.new(|_| AiChatStore::new(tokio.clone()));
        ai_store.update(cx, {
            let lb = live_bridge.clone();
            move |s, _| s.set_live_bridge(std::sync::Arc::new(lb))
        });
        let ai_chat = cx.new(|cx| AiChatView::new(ai_store, theme.clone(), cx));
        cx.observe(&ai_chat, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&ai_chat, |this, _, event: &AiChatEvent, cx| {
            this.pending_ai.push(event.clone());
            cx.notify();
        })
        .detach();

        let command_palette =
            cx.new(|cx| CommandPalette::new(theme.clone(), workspace.clone(), prefs.clone(), cx));
        cx.observe(&command_palette, |_, _, cx| cx.notify())
            .detach();
        cx.subscribe(&command_palette, |this, _, event: &PaletteEvent, cx| {
            this.pending_commands.push(event.clone());
            cx.notify();
        })
        .detach();

        let explorer = cx.new(|cx| ExplorerView::new(theme.clone(), workspace.clone(), cx));
        cx.observe(&explorer, |_, _, cx| cx.notify()).detach();

        let bookmarks =
            cx.new(|cx| BookmarksView::new(theme.clone(), workspace.clone(), explorer.clone(), cx));
        cx.observe(&bookmarks, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&bookmarks, |this, _, event: &BookmarkEvent, cx| {
            this.pending_bookmarks.push(event.clone());
            cx.notify();
        })
        .detach();
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
        window.on_window_should_close(cx, {
            let workspace = workspace.clone();
            let prefs = prefs.clone();
            move |window, cx| {
                if let WindowBounds::Windowed(bounds) = window.window_bounds() {
                    window_state::save(bounds);
                }
                // Session snapshot (T14-001): capture on the normal quit path,
                // or wipe a stale snapshot when the preference is off.
                if prefs.read(cx).get().session_restore {
                    let snapshot = workspace.read(cx).session_snapshot(cx);
                    crate::session::save_snapshot(&snapshot);
                } else {
                    crate::session::clear_snapshot();
                    // Session restore is off — no persisted scrollback is ever
                    // replayed, so wipe it all (T14-002).
                    labonair_backend::modules::scrollback::scrollback_cleanup(&[], None);
                }
                true
            }
        });

        // The registry must be populated before the docks are built from it
        // (T17-001 / T17-002).
        register_builtin_panels(
            &workspace, &explorer, &git_panel, &git_graph, &snippets, &ai_chat, cx,
        );

        // Build the three docks from the registry + the persisted layout
        // (falling back to a migration of the legacy `sidebar_*` prefs).
        let dock_layout = {
            let p = prefs.read(cx).get();
            if p.dock_layout.trim().is_empty() {
                migrate_dock_layout(p)
            } else {
                p.dock_layout.clone()
            }
        };
        workspace.update(cx, |w, cx| w.init_docks(&dock_layout, window, cx));

        // Dock-layout persistence lives on the `Workspace` now (T17-003); the
        // shell only supplies the write path into its `PreferencesStore`.
        workspace.update(cx, |w, _| {
            let prefs = prefs.clone();
            w.set_dock_persist_hook(move |json, cx| {
                prefs.update(cx, |s, cx| {
                    s.set_value("dockLayout", serde_json::Value::String(json), cx);
                });
            });
        });

        // Populate the status-bar item registry (the single place that names
        // concrete `StatusItem` types), then build the `StatusBar` view that
        // renders from it.
        register_builtin_status_items(
            &workspace,
            &theme,
            &notifications,
            &updater,
            &agent_access,
            &bookmarks,
            cx,
        );
        let status_bar = cx.new(|cx| StatusBar::new(workspace.clone(), theme.clone(), cx));
        cx.observe(&status_bar, |_, _, cx| cx.notify()).detach();

        Self {
            theme,
            background,
            notifications,
            workspace,
            explorer,
            bookmarks,
            git_panel,
            snippets,
            ai_chat,
            command_palette,
            prefs,
            updater,
            pending_commands: Vec::new(),
            pending_bookmarks: Vec::new(),
            pending_ai: Vec::new(),
            live_bridge,
            status_bar,
            app_menu_open: false,
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

    /// The central preferences store (T13-001).
    pub fn preferences(&self) -> &Entity<PreferencesStore> {
        &self.prefs
    }

    fn act_open_settings(
        &mut self,
        _: &menu::OpenSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(None, cx);
    }

    fn act_open_ai_settings(
        &mut self,
        _: &menu::AiSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(Some(SettingsTab::Ai), cx);
    }

    fn act_check_for_updates(
        &mut self,
        _: &menu::CheckForUpdates,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.updater.update(cx, |u, cx| u.run_check(true, cx));
    }

    /// The primary edge as a [`DockPosition`] (per `sidebarPosition`).
    fn primary_dock(&self, cx: &App) -> DockPosition {
        self.workspace.read(cx).primary_dock(cx)
    }

    /// `Cmd+B` — toggle the primary dock open/closed.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        let pos = self.primary_dock(cx);
        self.workspace.update(cx, |w, cx| {
            w.dock_mut(pos).toggle_open();
            w.persist_docks(cx);
        });
        cx.notify();
    }

    /// Status-bar-toggle intent: open + activate `name`, or close its dock if
    /// it is already the active panel there.
    fn select_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.select_panel(name, cx));
        cx.notify();
    }

    /// "show me X" — never closes the dock (palette / menu intent).
    fn open_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_panel(name, cx));
        cx.notify();
    }

    /// Move a panel to another dock (T17-002 API; the UI lands in T18-007, a
    /// debug shortcut exercises it now).
    fn move_panel(&mut self, name: &str, to: DockPosition, cx: &mut Context<Self>) {
        let moved = self.workspace.update(cx, |w, cx| {
            let moved = w.move_panel(name, to, cx);
            if moved {
                w.persist_docks(cx);
            }
            moved
        });
        if moved {
            cx.notify();
        }
    }

    /// Resize the dock at `pos`, clamped by the active panel's `min_size` and
    /// the dock's own bounds.
    fn set_dock_size(&mut self, pos: DockPosition, size: f32, cx: &mut Context<Self>) {
        let floor = self
            .workspace
            .read(cx)
            .dock(pos)
            .active_panel()
            .and_then(|p| p.min_size(cx));
        let changed = self.workspace.update(cx, |w, cx| {
            let changed = {
                let dock = w.dock_mut(pos);
                let before = dock.size();
                dock.set_size(px(size), floor);
                (f32::from(dock.size()) - f32::from(before)).abs() > 0.5
            };
            if changed {
                w.persist_docks(cx);
            }
            changed
        });
        if changed {
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

    fn act_new_preview_tab(
        &mut self,
        _: &menu::NewPreviewTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_preview_tab(window, cx));
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

    fn act_toggle_ai_panel(
        &mut self,
        _: &menu::ToggleAiPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_panel("ai", cx);
    }

    /// Temporary T17-002 debug shortcut (`Cmd+Alt+Shift+M`): move the active
    /// panel of the primary dock to the next dock position.
    fn act_debug_cycle_panel_dock(
        &mut self,
        _: &menu::DebugCyclePanelDock,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = self.primary_dock(cx);
        let Some(name) = self
            .workspace
            .read(cx)
            .dock(pos)
            .active_name()
            .map(str::to_owned)
        else {
            return;
        };
        self.move_panel(&name, pos.next(), cx);
    }

    /// Temporary T17-002 debug shortcut (`Cmd+Alt+Shift+Z`): toggle the primary
    /// dock's zoom state.
    fn act_debug_toggle_dock_zoom(
        &mut self,
        _: &menu::DebugToggleDockZoom,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = self.primary_dock(cx);
        self.workspace.update(cx, |w, cx| {
            let z = w.dock(pos).is_zoomed();
            w.dock_mut(pos).set_zoomed(!z);
            w.persist_docks(cx);
        });
        cx.notify();
    }

    /// "Ask AI about Selection" — capture the active editor/terminal selection
    /// into the AI composer and reveal the panel.
    fn act_ask_about_selection(
        &mut self,
        _: &menu::AskAboutSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((label, text)) = self.workspace.read(cx).active_selection(cx) else {
            return;
        };
        self.ai_chat
            .update(cx, |v, cx| v.attach_selection(label, text, cx));
        self.open_panel("ai", cx);
    }

    fn act_new_ai_session(
        &mut self,
        _: &menu::NewAiSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ai_chat.update(cx, |v, cx| v.new_session(cx));
        self.open_panel("ai", cx);
    }

    fn act_clear_chat(&mut self, _: &menu::ClearChat, _: &mut Window, cx: &mut Context<Self>) {
        self.ai_chat.update(cx, |v, cx| v.clear_active_chat(cx));
    }

    /// `Open Host Manager` / `New SSH Tab` / `New SFTP Tab` / `New SSH
    /// Connection…` / `New Quick SSH…` all just focus the Home dashboard in the
    /// reference (`useMenuBridge` → `actions.openHomeTab()`).
    fn act_open_host_manager(
        &mut self,
        _: &menu::OpenHostManager,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    fn act_new_ssh_tab(&mut self, _: &menu::NewSshTab, _: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    fn act_new_sftp_tab(&mut self, _: &menu::NewSftpTab, _: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    fn act_new_ssh_connection(
        &mut self,
        _: &menu::NewSshConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `Cmd+Shift+N` now opens the command palette straight to the Hosts page
        // (`Enter` = SSH, `Shift+Enter` = SFTP) instead of the Host-Manager tab.
        self.command_palette
            .update(cx, |p, cx| p.open_to_page(PalettePage::Hosts, window, cx));
    }

    fn act_new_quick_ssh(&mut self, _: &menu::NewQuickSsh, _: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
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

    fn act_open_path_bookmarks(
        &mut self,
        _: &menu::OpenPathBookmarks,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.bookmarks.update(cx, |b, cx| b.toggle(window, cx));
    }

    fn act_focus_next_pane(
        &mut self,
        _: &menu::FocusNextPane,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.focus_next_pane(window, cx));
    }

    fn act_toggle_zen_mode(
        &mut self,
        _: &menu::ToggleZenMode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_zen_mode(cx);
    }

    /// `view.zenMode`: both bars visible → hide both, otherwise show both.
    /// Port of `useShortcutHandlers`' `"view.zenMode"` handler.
    fn toggle_zen_mode(&mut self, cx: &mut Context<Self>) {
        let p = self.prefs.read(cx).get();
        let next = !(p.zen_mode_show_header || p.zen_mode_show_statusbar);
        self.prefs.update(cx, |s, cx| {
            s.set_value("zenModeShowHeader", serde_json::Value::Bool(next), cx);
            s.set_value("zenModeShowStatusbar", serde_json::Value::Bool(next), cx);
        });
    }

    fn toggle_zen_pref(&mut self, key: &str, cx: &mut Context<Self>) {
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        self.prefs.update(cx, |s, cx| {
            s.set_value(key, serde_json::Value::Bool(!cur), cx)
        });
    }

    select_tab_action!(act_select_tab_1, SelectTab1, 0);
    select_tab_action!(act_select_tab_2, SelectTab2, 1);
    select_tab_action!(act_select_tab_3, SelectTab3, 2);
    select_tab_action!(act_select_tab_4, SelectTab4, 3);
    select_tab_action!(act_select_tab_5, SelectTab5, 4);
    select_tab_action!(act_select_tab_6, SelectTab6, 5);
    select_tab_action!(act_select_tab_7, SelectTab7, 6);
    select_tab_action!(act_select_tab_8, SelectTab8, 7);
    select_tab_action!(act_select_tab_9, SelectTab9, 8);

    /// Snapshot the live state the command palette needs for its dynamic
    /// sub-pages and `rightLabel` states. Domains not yet ported (hosts,
    /// snippets, AI sessions, git branches, editor outline) stay empty — the
    /// pages exist and show an empty state until their block lands.
    fn build_palette_data(&self, cx: &App) -> PaletteData {
        let ts = self.workspace.read(cx).tab_store();
        let ts = ts.read(cx);
        let active = ts.active_id();
        let tabs = ts
            .tabs()
            .iter()
            .map(|t| PaletteChoice {
                id: t.id.to_string(),
                title: t.label(),
                subtitle: Some(t.kind.default_title().to_string()),
                active: t.id == active,
            })
            .collect();

        let hosts = self
            .workspace
            .read(cx)
            .known_hosts(cx)
            .into_iter()
            .map(|(id, name)| PaletteChoice {
                id,
                title: name,
                subtitle: None,
                active: false,
            })
            .collect();

        let recent_hosts = self
            .workspace
            .read(cx)
            .recent_hosts(cx, 5)
            .into_iter()
            .map(|(id, name)| PaletteChoice {
                id,
                title: name,
                subtitle: None,
                active: false,
            })
            .collect();

        let theme = self.theme.read(cx);
        let p = self.prefs.read(cx).get();

        let active_theme_id = if p.app_theme.is_empty() {
            "default"
        } else {
            p.app_theme.as_str()
        };
        let snippets = self
            .snippets
            .read(cx)
            .snippet_choices()
            .into_iter()
            .map(|(id, name, mode)| PaletteChoice {
                id,
                title: name,
                subtitle: Some(mode),
                active: false,
            })
            .collect();

        let ai_sessions = self
            .ai_chat
            .read(cx)
            .session_choices(cx)
            .into_iter()
            .map(|(id, title, active)| PaletteChoice {
                id,
                title,
                subtitle: None,
                active,
            })
            .collect();

        let git_branches = self
            .git_panel
            .read(cx)
            .branch_choices()
            .into_iter()
            .map(|(name, current, remote)| PaletteChoice {
                active: current,
                subtitle: remote.then(|| "remote".to_string()),
                id: name.clone(),
                title: name,
            })
            .collect();

        let symbols = self
            .workspace
            .read(cx)
            .active_editor_symbols(cx)
            .into_iter()
            .map(|s| PaletteChoice {
                id: s.line.to_string(),
                title: s.name,
                subtitle: Some(format!("{}  ·  line {}", s.kind.label(), s.line + 1)),
                active: false,
            })
            .collect();

        let app_themes = labonair_settings_ui::theme_choices()
            .into_iter()
            .map(|(id, name)| PaletteChoice {
                active: id == active_theme_id,
                id,
                title: name,
                subtitle: None,
            })
            .collect();
        let mut toggles = std::collections::HashMap::new();
        toggles.insert("zenModeShowHeader", p.zen_mode_show_header);
        toggles.insert("zenModeShowStatusbar", p.zen_mode_show_statusbar);
        toggles.insert("editorWordWrap", p.editor_word_wrap);
        toggles.insert("editorLineNumbers", p.editor_line_numbers);
        toggles.insert("editorFormatOnSave", p.editor_format_on_save);
        toggles.insert("terminalCursorBlink", p.terminal_cursor_blink);
        toggles.insert("terminalShowPaneHeader", p.terminal_show_pane_header);
        toggles.insert("terminalShowPaneFooter", p.terminal_show_pane_footer);
        toggles.insert("vimMode", p.editor_vim_mode);

        PaletteData {
            tabs,
            hosts,
            recent_hosts,
            snippets,
            ai_sessions,
            git_branches,
            symbols,
            app_themes,
            color_mode: theme.preference(),
            editor_theme: theme.editor_theme(),
            font_size: Some(p.terminal_font_size),
            toggles,
        }
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
                PaletteEvent::ConnectHost { host_id, sftp } => {
                    self.workspace.update(cx, |w, cx| {
                        if sftp {
                            w.open_sftp_tab(host_id, window, cx);
                        } else {
                            w.open_ssh_tab(host_id, window, cx);
                        }
                    });
                }
                PaletteEvent::SetAppTheme(id) => {
                    labonair_settings_ui::activate_app_theme(&id, &self.prefs, &self.theme, cx);
                }
                PaletteEvent::PreviewAppTheme(id) => {
                    labonair_settings_ui::preview_app_theme(
                        id.as_deref(),
                        &self.prefs,
                        &self.theme,
                        cx,
                    );
                }
                PaletteEvent::RunSnippet(id) => {
                    self.snippets
                        .update(cx, |s, cx| s.run_by_id(&id, window, cx));
                }
                PaletteEvent::SwitchAiSession(id) => {
                    self.ai_chat
                        .update(cx, |v, cx| v.switch_to_session(&id, cx));
                    self.open_panel("ai", cx);
                }
                PaletteEvent::SwitchBranch(name) => {
                    self.git_panel.update(cx, |g, cx| g.checkout(name, cx));
                }
                PaletteEvent::GoToLine(line) => {
                    self.workspace
                        .update(cx, |w, cx| w.active_editor_goto_line(line, cx));
                }
                PaletteEvent::SetColorMode(pref) => {
                    let key = match pref {
                        crate::theme::ThemePreference::System => "system",
                        crate::theme::ThemePreference::Light => "light",
                        crate::theme::ThemePreference::Dark => "dark",
                    };
                    self.prefs.update(cx, |s, cx| {
                        s.set_value("theme", serde_json::Value::String(key.into()), cx)
                    });
                    let p = self.prefs.read(cx).get().clone();
                    labonair_settings_ui::apply_prefs_to_theme(&p, &self.theme, cx);
                }
                PaletteEvent::SetEditorTheme(id) => {
                    self.prefs.update(cx, |s, cx| {
                        s.set_value(
                            "editorTheme",
                            serde_json::Value::String(id.slug().into()),
                            cx,
                        )
                    });
                    let p = self.prefs.read(cx).get().clone();
                    labonair_settings_ui::apply_prefs_to_theme(&p, &self.theme, cx);
                }
            }
        }
    }

    /// Drain bookmark picks queued by the `BookmarkEvent` subscription.
    fn drain_pending_bookmarks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for event in std::mem::take(&mut self.pending_bookmarks) {
            match event {
                BookmarkEvent::OpenLocal(path) => {
                    self.explorer
                        .update(cx, |e, cx| e.set_root_str(Some(path), cx));
                    self.select_panel("explorer", cx);
                }
                BookmarkEvent::OpenRemote { host_id, .. } => {
                    self.workspace
                        .update(cx, |w, cx| w.open_sftp_tab(host_id, window, cx));
                }
            }
        }
    }

    /// Refresh the AI live-bridge snapshot from the workspace + explorer and
    /// drain any commands the agent's terminal tools queued.
    fn sync_live_bridge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ws = self.workspace.read(cx);
        let snap = crate::live_bridge::LiveSnapshot {
            cwd: ws.active_cwd(cx),
            workspace_root: self
                .explorer
                .read(cx)
                .root()
                .map(|p| p.display().to_string()),
            terminal_lines: ws.active_terminal_lines(200, cx),
            ssh_tab_id: ws.active_remote_target(cx).map(|(_, sid)| sid),
            has_terminal: ws.active_is_terminal(cx),
        };
        self.live_bridge.set_snapshot(snap);
        for cmd in self.live_bridge.drain_commands() {
            if cmd.execute {
                self.workspace
                    .update(cx, |w, cx| w.run_in_active_terminal(cmd.text, window, cx));
            } else {
                self.workspace
                    .read(cx)
                    .inject_into_active_terminal(&cmd.text, cx);
            }
        }
    }

    fn drain_pending_ai(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for event in std::mem::take(&mut self.pending_ai) {
            match event {
                AiChatEvent::RunInTerminal(cmd) => {
                    self.workspace
                        .update(cx, |w, cx| w.run_in_active_terminal(cmd, window, cx));
                }
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
            CommandId::CheckForUpdates => dispatch!(menu::CheckForUpdates),
            CommandId::DuplicateTab => self
                .workspace
                .update(cx, |w, cx| w.duplicate_active_tab(window, cx)),
            CommandId::CloseOtherTabs => self
                .workspace
                .update(cx, |w, cx| w.close_other_tabs(window, cx)),
            CommandId::ClearTerminal => self
                .workspace
                .update(cx, |w, cx| w.clear_active_terminal(cx)),
            CommandId::ToggleAiPanel => self.select_panel("ai", cx),
            CommandId::OpenSnippetsPanel => self.open_panel("snippets", cx),
            CommandId::OpenPathBookmarks => self.bookmarks.update(cx, |b, cx| b.toggle(window, cx)),
            CommandId::OpenGitGraph => self.workspace.update(cx, |w, cx| w.open_git_graph_tab(cx)),
            CommandId::FocusSourceControl => self.open_panel("source-control", cx),
            CommandId::ToggleZenMode => self.toggle_zen_mode(cx),
            CommandId::ToggleZenModeHeader => self.toggle_zen_pref("zenModeShowHeader", cx),
            CommandId::ToggleZenModeStatusbar => self.toggle_zen_pref("zenModeShowStatusbar", cx),
            CommandId::OpenAiSettings => dispatch!(menu::AiSettings),
            CommandId::ToggleEditorWordWrap => self.toggle_zen_pref("editorWordWrap", cx),
            CommandId::ToggleLineNumbers => self.toggle_zen_pref("editorLineNumbers", cx),
            CommandId::ToggleFormatOnSave => self.toggle_zen_pref("editorFormatOnSave", cx),
            CommandId::ToggleCursorBlink => self.toggle_zen_pref("terminalCursorBlink", cx),
            CommandId::TogglePaneHeader => self.toggle_zen_pref("terminalShowPaneHeader", cx),
            CommandId::TogglePaneFooter => self.toggle_zen_pref("terminalShowPaneFooter", cx),
            CommandId::ToggleVimMode => self.toggle_zen_pref("vimMode", cx),
            // Sub-page navigators — resolved inside the palette, never emitted
            // as `Run`. Listed so the match stays exhaustive.
            CommandId::SwitchTab
            | CommandId::AdjustFontSize
            | CommandId::ConnectSsh
            | CommandId::OpenSftp
            | CommandId::ChangeAppTheme
            | CommandId::ChangeColorMode
            | CommandId::ChangeEditorTheme
            | CommandId::SwitchAiSession
            | CommandId::RunSnippet
            | CommandId::GitSwitchBranch
            | CommandId::GoToSymbol => {}
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
        let (toolbar, border) = {
            let theme = self.theme.read(cx);
            (theme.toolbar(), theme.border())
        };

        // `tabsLocation === "sidebar"` moves the tab strip out of the titlebar
        // and into the Tabs sidebar panel.
        let tabs_in_sidebar = self.prefs.read(cx).get().tabs_location == "sidebar";
        let tabs = (!tabs_in_sidebar).then(|| {
            self.workspace
                .update(cx, |w, cx| w.render_tab_bar(cx).into_any_element())
        });

        // T17-003: the titlebar carries no bar items — every former badge /
        // toggle is a `StatusItem` in the status bar now.
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
            .child(div().flex_1().min_w_0().children(tabs))
            .when(self.search_open, |d| d.child(self.render_search(cx)))
            .child(self.render_app_menu(cx))
    }

    /// The `⋯` header app-menu button + its dropdown (port of `Header.tsx`
    /// `sideButtons`: Settings / Keyboard Shortcuts / Themes…).
    fn render_app_menu(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (fg, muted, border, card) = (
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.card(),
        );
        let open = self.app_menu_open;

        let item = |label: &str, key: SharedString| {
            div()
                .id(key)
                .px_2()
                .py_1()
                .text_xs()
                .rounded_sm()
                .text_color(fg)
                .hover(|s| s.bg(border))
                .child(SharedString::from(label.to_string()))
        };

        div()
            .relative()
            .flex_shrink_0()
            .child(
                div()
                    .id("app-menu")
                    .size(px(26.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .text_color(muted)
                    .hover(|s| s.bg(border).text_color(fg))
                    .child(IconName::Menu.svg(muted))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.app_menu_open = !this.app_menu_open;
                        cx.notify();
                    })),
            )
            .when(open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(30.0))
                        .right(px(0.0))
                        .w(px(208.0))
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .p_1()
                        .rounded_md()
                        .bg(card)
                        .border_1()
                        .border_color(border)
                        .child(item("Settings", "am-settings".into()).on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.app_menu_open = false;
                                open_settings_window(None, cx);
                            },
                        )))
                        .child(item("Keyboard Shortcuts", "am-shortcuts".into()).on_click(
                            cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.app_menu_open = false;
                                window.dispatch_action(Box::new(menu::OpenShortcuts), cx);
                            }),
                        ))
                        .child(
                            item("Themes\u{2026}", "am-themes".into()).on_click(cx.listener(
                                |this, _: &ClickEvent, _window, cx| {
                                    this.app_menu_open = false;
                                    open_settings_window(Some(SettingsTab::Themes), cx);
                                },
                            )),
                        ),
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

    /// Render one edge dock (T17-002): a header (active panel title + a
    /// per-panel switcher when the dock holds more than one + a "move to next
    /// dock" affordance), the active panel's body, and a resize handle on the
    /// inner edge. Left/right docks are vertical + width-resizable; the bottom
    /// dock is horizontal + height-resizable. A zoomed dock fills its axis and
    /// drops the handle.
    fn render_dock(
        &mut self,
        pos: DockPosition,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
            let ws = self.workspace.read(cx);
            let dock = ws.dock(pos);
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
                                this.workspace.update(cx, |w, cx| {
                                    w.dock_mut(pos).activate_panel(&n);
                                    w.persist_docks(cx);
                                });
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
                        if let Some(name) = this.workspace.read(cx).dock(pos).active_name() {
                            let name = name.to_owned();
                            this.move_panel(&name, pos.next(), cx);
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

impl Focusable for AppShell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.maybe_persist_geometry(window);
        self.drain_pending_commands(window, cx);
        self.drain_pending_bookmarks(window, cx);
        self.drain_pending_ai(window, cx);
        self.sync_live_bridge(window, cx);

        let palette_data = self.build_palette_data(cx);
        self.command_palette
            .update(cx, |p, _| p.set_data(palette_data));

        let bg = self.theme.read(cx).background();
        let ui_font = self.theme.read(cx).ui_font();
        let ui_font_size = self.theme.read(cx).ui_font_size();
        let background_layer = self.background.read(cx).layer(LayerScope::App);
        let toasts = notifications::render_overlay(&self.notifications, &self.theme, cx);
        let show_header = self.prefs.read(cx).get().zen_mode_show_header;
        let show_statusbar = self.prefs.read(cx).get().zen_mode_show_statusbar;
        let header = show_header.then(|| self.render_header(cx).into_any_element());
        let dock_open = |pos: DockPosition, this: &Workspace| this.dock(pos).is_open();
        let (left_open, right_open, bottom_open) = {
            let ws = self.workspace.read(cx);
            (
                dock_open(DockPosition::Left, ws),
                dock_open(DockPosition::Right, ws),
                dock_open(DockPosition::Bottom, ws),
            )
        };
        let left_dock = left_open.then(|| {
            self.render_dock(DockPosition::Left, window, cx)
                .into_any_element()
        });
        let right_dock = right_open.then(|| {
            self.render_dock(DockPosition::Right, window, cx)
                .into_any_element()
        });
        let bottom_dock = bottom_open.then(|| {
            self.render_dock(DockPosition::Bottom, window, cx)
                .into_any_element()
        });
        let statusbar = show_statusbar.then(|| self.status_bar.clone().into_any_element());
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
            .font(ui_font)
            .text_size(px(ui_font_size))
            .on_action(cx.listener(Self::act_new_terminal_tab))
            .on_action(cx.listener(Self::act_new_editor_tab))
            .on_action(cx.listener(Self::act_new_preview_tab))
            .on_action(cx.listener(Self::act_save))
            .on_action(cx.listener(Self::act_close_tab))
            .on_action(cx.listener(Self::act_find))
            .on_action(cx.listener(Self::act_toggle_sidebar))
            .on_action(cx.listener(Self::act_toggle_fullscreen))
            .on_action(cx.listener(Self::act_minimize))
            .on_action(cx.listener(Self::act_zoom_window))
            .on_action(cx.listener(Self::act_next_tab))
            .on_action(cx.listener(Self::act_prev_tab))
            .on_action(cx.listener(Self::act_toggle_ai_panel))
            .on_action(cx.listener(Self::act_debug_cycle_panel_dock))
            .on_action(cx.listener(Self::act_debug_toggle_dock_zoom))
            .on_action(cx.listener(Self::act_ask_about_selection))
            .on_action(cx.listener(Self::act_new_ai_session))
            .on_action(cx.listener(Self::act_clear_chat))
            .on_action(cx.listener(Self::act_open_host_manager))
            .on_action(cx.listener(Self::act_new_ssh_tab))
            .on_action(cx.listener(Self::act_new_sftp_tab))
            .on_action(cx.listener(Self::act_new_ssh_connection))
            .on_action(cx.listener(Self::act_new_quick_ssh))
            .on_action(cx.listener(Self::act_focus_next_pane))
            .on_action(cx.listener(Self::act_toggle_zen_mode))
            .on_action(cx.listener(Self::act_select_tab_1))
            .on_action(cx.listener(Self::act_select_tab_2))
            .on_action(cx.listener(Self::act_select_tab_3))
            .on_action(cx.listener(Self::act_select_tab_4))
            .on_action(cx.listener(Self::act_select_tab_5))
            .on_action(cx.listener(Self::act_select_tab_6))
            .on_action(cx.listener(Self::act_select_tab_7))
            .on_action(cx.listener(Self::act_select_tab_8))
            .on_action(cx.listener(Self::act_select_tab_9))
            .on_action(cx.listener(Self::act_command_palette))
            .on_action(cx.listener(Self::act_open_path_bookmarks))
            .on_action(cx.listener(Self::act_open_settings))
            .on_action(cx.listener(Self::act_open_ai_settings))
            .on_action(cx.listener(Self::act_check_for_updates))
            .when(can_split, |d| {
                d.on_action(cx.listener(Self::act_split_right))
                    .on_action(cx.listener(Self::act_split_down))
            })
            .when(has_split, |d| {
                d.on_action(cx.listener(Self::act_close_pane))
            })
            .children(header)
            .child({
                // Center column: workspace on top, the bottom dock beneath it
                // (inside this column so it never overlaps the side docks —
                // Zed's `workspace.rs` nesting).
                let center = div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(div().flex_1().min_h_0().min_w_0().child(workspace))
                    .children(bottom_dock);
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .children(left_dock)
                    .child(center)
                    .children(right_dock)
                    .on_drag_move(cx.listener(
                        |this, ev: &DragMoveEvent<DockResize>, _window, cx| {
                            let pos = ev.drag(cx).0;
                            let b = ev.bounds;
                            let p = ev.event.position;
                            let size = match pos {
                                DockPosition::Left => f32::from(p.x - b.origin.x),
                                DockPosition::Right => f32::from(b.origin.x + b.size.width - p.x),
                                DockPosition::Bottom => f32::from(b.origin.y + b.size.height - p.y),
                            };
                            this.set_dock_size(pos, size, cx);
                        },
                    ))
            })
            .children(statusbar)
            .children(background_layer)
            .child(self.command_palette.clone())
            .child(self.bookmarks.clone())
            .child(self.updater.clone())
            .children(toasts)
    }
}

fn bounds_differ(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    let d = |x: Pixels, y: Pixels| (f32::from(x) - f32::from(y)).abs() > 2.0;
    d(a.origin.x, b.origin.x)
        || d(a.origin.y, b.origin.y)
        || d(a.size.width, b.size.width)
        || d(a.size.height, b.size.height)
}
