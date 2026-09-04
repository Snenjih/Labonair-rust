//! Startup wiring for [`AppShell`] — extracted from `AppShell::new` in T17-006.
//!
//! [`bootstrap`] builds every child entity (workspace, panels, palette,
//! updater, docks, status bar, modal + toast layers, titlebar), runs the
//! ordered startup sequence (MCP-prefs hydrate → session snapshot → theme
//! preference → `apply_prefs_to_theme` → keybinds → settings deps → updater
//! check) and sets up the reactive edges that used to be a dozen
//! `cx.observe(&x, |_,_,cx| cx.notify())` lines plus the per-frame
//! `drain_pending_*` / `sync_live_bridge` calls.
//!
//! Ordering is load-bearing (see the task `## Warnungen`): MCP port before MCP
//! enable, theme preference before the first render, session snapshot before
//! the default tabs.

use std::sync::Arc;

use gpui::{App, AppContext, Context, Entity, Window, WindowBounds};
use labonair_backend::modules::mcp::{
    mcp_set_auto_revoke_minutes, mcp_set_enabled, mcp_set_max_command_timeout_secs, mcp_set_port,
};
use labonair_backend::modules::settings::mcp::mcp_prefs_load;
use labonair_backend::App as Backend;
use labonair_notifications::NotificationCenter;
use labonair_terminal::TerminalRegistry;
use tokio::runtime::Handle as TokioHandle;

use labonair_command_palette::{CommandPalette, PaletteEvent};
use labonair_panel_ai::{AiChatEvent, AiChatStore, AiChatView};
use labonair_panel_explorer::{BookmarkEvent, BookmarksView, ExplorerView};
use labonair_panel_git_graph::GitGraphView;
use labonair_panel_scm::GitPanelView;
use labonair_panel_snippets::SnippetsView;
use labonair_settings_ui::{set_settings_deps, PreferencesStore};
use labonair_workspace::agent_access::AgentAccessStore;
use labonair_workspace::dock::DockData;
use labonair_workspace::live_bridge::{LiveSnapshot, WorkspaceLiveBridge};
use labonair_workspace::modal_layer::ModalLayer;
use labonair_workspace::status_bar::StatusBar;
use labonair_workspace::toast_layer::ToastLayer;

use crate::app_shell::{AppShell, ShellPanels};
use crate::background::BackgroundStore;
use crate::status_items::register_builtin_status_items;
use crate::theme::{ThemePreference, ThemeStore};
use crate::titlebar::Titlebar;
use crate::updater::UpdaterView;
use crate::window_state;
use crate::workspace::Workspace;

/// How often the AI live-bridge command queue is drained on the main thread.
/// The queue is only ever fed by AI agent tool calls (seconds apart), so this
/// replaces the former per-frame drain with a light background poll — the same
/// idiom `Workspace` uses for its SSH / transfer event bridges.
const LIVE_DRAIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(120);

/// Rebuild the AI live-bridge [`LiveSnapshot`] from the current workspace +
/// explorer state. Called event-driven (T17-006) from `cx.observe` on the
/// workspace + explorer, instead of every frame.
pub(crate) fn refresh_live_snapshot(
    workspace: &Entity<Workspace>,
    explorer: &Entity<ExplorerView>,
    bridge: &WorkspaceLiveBridge,
    cx: &App,
) {
    let ws = workspace.read(cx);
    bridge.set_snapshot(LiveSnapshot {
        cwd: ws.active_cwd(cx),
        workspace_root: explorer.read(cx).root().map(|p| p.display().to_string()),
        terminal_lines: ws.active_terminal_lines(200, cx),
        ssh_tab_id: ws.active_remote_target(cx).map(|(_, sid)| sid),
        has_terminal: ws.active_is_terminal(cx),
    });
}

/// Register the five built-in panels on the workspace's `PanelRegistry`.
///
/// This is the **only** place in the app that names concrete panel types.
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

/// Build the persisted [`DockData`] array from the legacy `sidebar_*`
/// preferences (T17-002 first-run migration).
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn bootstrap(
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    notifications: Entity<NotificationCenter>,
    backend: Backend,
    tokio: TokioHandle,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> AppShell {
    // T18-006: one-time migration of the legacy `barItemPlacements` blob into
    // `statusBarItemPlacements`. Must run before the first `StatusItemRegistry`
    // build (`register_builtin_status_items` below reloads placements right
    // after registering every item).
    match labonair_backend::modules::settings::migrations::migrate_bar_item_placements(
        &labonair_backend::modules::fs::paths::config_dir(),
    ) {
        Ok(outcome) => tracing::info!("bar item placement migration: {outcome:?}"),
        Err(err) => tracing::warn!("bar item placement migration failed: {err}"),
    }

    cx.observe(&background, |_, _, cx| cx.notify()).detach();

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

    // The Rust `McpState` boots with no persistence of its own — mirror the
    // saved preferences into it once at startup. Port/timeout/auto-revoke first
    // so the listener, if enabled, comes up on the right port.
    {
        let prefs = mcp_prefs_load();
        agent_access.update(cx, |s, cx| {
            s.hydrate(prefs.bridge_enabled, prefs.notify_on_activity, cx)
        });
        let app = backend.clone();
        tokio.spawn(async move {
            let _ = mcp_set_port(prefs.bridge_port, app.clone(), &app.mcp, &app.secrets).await;
            let _ =
                mcp_set_max_command_timeout_secs(prefs.max_command_timeout_secs, &app.mcp).await;
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
    // Shell re-render on workspace change — keeps the `.when(can_split)` action
    // bindings in `render` in sync with the active tab.
    cx.observe(&workspace, |_, _, cx| cx.notify()).detach();

    let git_panel =
        cx.new(|cx| GitPanelView::new(backend.clone(), tokio.clone(), theme.clone(), cx));

    let git_graph =
        cx.new(|cx| GitGraphView::new(backend.clone(), tokio.clone(), theme.clone(), cx));
    // The workspace renders the Git Graph as a `TabKind::GitGraph` tab — share
    // this single entity so the app-shell keeps feeding it the active CWD.
    workspace.update(cx, |w, _cx| w.set_git_graph(git_graph.clone()));

    // Central preferences store (T13-001); the settings UI lives in its own OS
    // window. Apply the persisted theme preference to the ThemeStore once at
    // startup; further changes flow through `SettingsView::set_pref`.
    let prefs = cx.new(|_| PreferencesStore::new());
    // Shell re-render on preference change — `render` reads
    // `zen_mode_show_statusbar`; the titlebar observes `prefs` on its own.
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
    {
        let p = prefs.read(cx).get().clone();
        prefs.update(cx, |s, cx| s.publish_global(cx));
        labonair_settings_ui::apply_prefs_to_theme(&p, &theme, cx);
    }
    // `keymap.json` (T19-008): load + merge + bind, publish the display
    // global, then live-watch the file so an edit takes effect with no
    // restart. Must run after the theme/prefs wiring above so a startup
    // banner (if the shipped default asset somehow fails to parse) has a
    // notification center to post into.
    crate::keymap_loader::reload_and_apply(cx);
    crate::keymap_loader::watch(cx);
    set_settings_deps(
        prefs.clone(),
        backend.clone(),
        tokio.clone(),
        workspace.clone(),
        cx,
    );
    labonair_settings_ui::set_keybind_apply_hook(crate::keymap_loader::reload_and_apply, cx);

    // Auto-updater (T15-005). Kicks a quiet background check at startup when the
    // `checkForUpdates` preference is on (6 h backoff inside the store).
    let updater = cx.new(|cx| UpdaterView::new(tokio.clone(), theme.clone(), cx));
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

    let live_bridge = WorkspaceLiveBridge::new();
    let ai_store = cx.new(|_| AiChatStore::new(tokio.clone()));
    ai_store.update(cx, {
        let lb = live_bridge.clone();
        move |s, _| s.set_live_bridge(Arc::new(lb))
    });
    let ai_chat = cx.new(|cx| AiChatView::new(ai_store, theme.clone(), cx));
    // AI-panel "run in terminal" — serviced straight from the event (no
    // `pending_ai` buffer / `drain_pending_ai`, T17-006).
    cx.subscribe_in(
        &ai_chat,
        window,
        |this, _, event: &AiChatEvent, window, cx| {
            let AiChatEvent::RunInTerminal(cmd) = event;
            let cmd = cmd.clone();
            this.workspace
                .update(cx, |w, cx| w.run_in_active_terminal(cmd, window, cx));
        },
    )
    .detach();

    let command_palette =
        cx.new(|cx| CommandPalette::new(theme.clone(), workspace.clone(), prefs.clone(), cx));
    cx.subscribe_in(
        &command_palette,
        window,
        |this, _, event: &PaletteEvent, window, cx| {
            this.handle_palette_event(event.clone(), window, cx);
        },
    )
    .detach();

    let explorer = cx.new(|cx| ExplorerView::new(theme.clone(), workspace.clone(), cx));

    let bookmarks =
        cx.new(|cx| BookmarksView::new(theme.clone(), workspace.clone(), explorer.clone(), cx));
    cx.subscribe_in(
        &bookmarks,
        window,
        |this, _, event: &BookmarkEvent, window, cx| {
            this.handle_bookmark_event(event.clone(), window, cx);
        },
    )
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

    // AI live-bridge snapshot — event-driven refresh (T17-006). The command
    // queue is drained by a light background poll below.
    refresh_live_snapshot(&workspace, &explorer, &live_bridge, cx);
    cx.observe(&workspace, {
        let workspace = workspace.clone();
        let explorer = explorer.clone();
        let bridge = live_bridge.clone();
        move |_, _, cx| refresh_live_snapshot(&workspace, &explorer, &bridge, cx)
    })
    .detach();
    cx.observe(&explorer, {
        let workspace = workspace.clone();
        let explorer = explorer.clone();
        let bridge = live_bridge.clone();
        move |_, _, cx| refresh_live_snapshot(&workspace, &explorer, &bridge, cx)
    })
    .detach();
    let live_drain = cx.spawn(async move |this, cx| loop {
        cx.background_executor().timer(LIVE_DRAIN_INTERVAL).await;
        let ok = this
            .update(cx, |this, cx| {
                let cmds = this.live_bridge.drain_commands();
                if !cmds.is_empty() {
                    this.workspace.update(cx, |w, cx| {
                        for cmd in cmds {
                            w.apply_live_command(cmd, cx);
                        }
                    });
                }
            })
            .is_ok();
        if !ok {
            break;
        }
    });

    // Persist the final window geometry on close (the throttled per-render save
    // covers force-quit within the last second).
    window.on_window_should_close(cx, {
        let workspace = workspace.clone();
        let prefs = prefs.clone();
        move |window, cx| {
            if let WindowBounds::Windowed(bounds) = window.window_bounds() {
                window_state::save(bounds);
            }
            if prefs.read(cx).get().session_restore {
                let snapshot = workspace.read(cx).session_snapshot(cx);
                crate::session::save_snapshot(&snapshot);
            } else {
                crate::session::clear_snapshot();
                labonair_backend::modules::scrollback::scrollback_cleanup(&[], None);
            }
            true
        }
    });

    // The registry must be populated before the docks are built from it.
    register_builtin_panels(
        &workspace, &explorer, &git_panel, &git_graph, &snippets, &ai_chat, cx,
    );

    // Build the three docks from the registry + the persisted layout (falling
    // back to a migration of the legacy `sidebar_*` prefs).
    let dock_layout = {
        let p = prefs.read(cx).get();
        if p.dock_layout.trim().is_empty() {
            migrate_dock_layout(p)
        } else {
            p.dock_layout.clone()
        }
    };
    workspace.update(cx, |w, cx| w.init_docks(&dock_layout, window, cx));

    // Dock-layout persistence lives on the `Workspace` now (T17-003); the shell
    // only supplies the write path into its `PreferencesStore`.
    workspace.update(cx, |w, _| {
        let prefs = prefs.clone();
        w.set_dock_persist_hook(move |json, cx| {
            prefs.update(cx, |s, cx| {
                s.set_value("dockLayout", serde_json::Value::String(json), cx);
            });
        });
    });

    // Populate the status-bar item registry, then build the `StatusBar` view.
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

    let modal_layer = cx.new(|_| ModalLayer::new());
    let toast_layer = cx.new(|cx| ToastLayer::new(notifications.clone(), theme.clone(), cx));

    let titlebar = cx.new(|cx| Titlebar::new(theme.clone(), prefs.clone(), workspace.clone(), cx));

    // The command table — the single definition site for every menu / keybind
    // / palette command (T17-007).
    let command_registry = crate::commands::register_builtin_commands();

    // `git_graph` is not kept on the shell: the workspace owns the shared
    // `Entity<GitGraphView>` (via `set_git_graph`) and the CWD-feed closure
    // above captured its own clone.
    let panels = ShellPanels {
        explorer,
        bookmarks,
        git_panel,
        snippets,
        ai_chat,
        updater,
        command_palette,
    };

    AppShell::from_parts(
        theme,
        background,
        prefs,
        workspace,
        titlebar,
        panels,
        status_bar,
        command_registry,
        modal_layer,
        toast_layer,
        live_bridge,
        live_drain,
        cx,
    )
}
