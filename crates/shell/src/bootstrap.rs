//! Startup wiring for [`AppShell`] — extracted from `AppShell::new` in T17-006.
//!
//! [`bootstrap`] builds every child entity (workspace, panels, palette,
//! updater, docks, status bar, modal layer, titlebar), runs the
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

use gpui::{App, AppContext, Context, Entity, PathPromptOptions, Window, WindowBounds};
use labonair_backend::modules::mcp::contract::{BackendMcpEventSource, BackendMcpSessionAccess};
use labonair_backend::modules::mcp::{
    mcp_set_auto_revoke_minutes, mcp_set_enabled, mcp_set_max_command_timeout_secs, mcp_set_port,
};
use labonair_backend::modules::settings::mcp::mcp_prefs_load;
use labonair_hosts_ui::{open_hosts_window, HostManagerEvent, HostManagerView};
use labonair_mcp_core::{McpEventSource, McpSessionAccessService, McpTabOperationService};
use labonair_notifications::{notification_center, Notification, NotificationCenter};
use labonair_sftp::{SftpBrowserService, SftpSessionService};
use labonair_ssh::{
    SshConfigService, SshConnectionService, SshConnectionTester, SshEventSource, SshPtyService,
    SshRemoteCommandService, SshRemoteFileService, SshTunnelService,
};
use labonair_terminal::TerminalRegistry;
use labonair_transfers::{TransferEventSource, TransferService};
use labonair_transfers_ui::TransfersView;
use tokio::runtime::Handle as TokioHandle;

use labonair_command_palette::{CommandPalette, PaletteEvent};
use labonair_panel_explorer::ExplorerView;
use labonair_panel_git_graph::GitGraphView;
use labonair_panel_scm::{GitPanelView, ScmEvent};
use labonair_panel_snippets::SnippetsView;
use labonair_settings::{GeneralSettings, Settings as _};
use labonair_settings_ui::set_settings_deps;
use labonair_workspace::agent_access::AgentAccessStore;
use labonair_workspace::live_bridge::{LiveSnapshot, WorkspaceLiveBridge};
use labonair_workspace::modal_layer::ModalLayer;
use labonair_workspace::status_bar::StatusBar;

use crate::app_shell::{AppShell, ShellPanels};
use crate::backend::BackendComposition;
use crate::settings_services::settings_services;
use crate::status_items::register_builtin_status_items;
use crate::theme::ThemeStore;
use crate::titlebar::{Titlebar, TitlebarEvent};
use crate::updater::UpdaterView;
use crate::window_state;
use crate::workspace::{Workspace, WorkspaceEvent};
use labonair_background::BackgroundStore;

/// How often the AI live-bridge command queue is drained on the main thread.
/// The queue is only ever fed by AI agent tool calls (seconds apart), so this
/// replaces the former per-frame drain with a light background poll — the same
/// idiom `Workspace` uses for its SSH / transfer event bridges.
const LIVE_DRAIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(120);

/// Ask the platform for one project directory and apply the result through
/// the workspace's explicit identity boundary. The shell owns the platform
/// picker, but it does not own project state.
fn open_project_picker(workspace: Entity<Workspace>, cx: &mut Context<AppShell>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Open Project".into()),
    });

    cx.spawn(async move |_this, cx| {
        let result = receiver.await;
        match result {
            Ok(Ok(Some(paths))) => {
                let Some(root) = paths.into_iter().find(|path| path.is_dir()) else {
                    let _ = cx.update(|app| {
                        notification_center(app).update(app, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Open Project",
                                    "The selected path is not an accessible folder.",
                                ),
                                cx,
                            );
                        });
                    });
                    return;
                };
                let _ = workspace.update(cx, |workspace, cx| {
                    workspace.apply_transition(
                        labonair_workspace::context::WorkspaceTransition::OpenProject { root },
                        cx,
                    );
                });
            }
            Ok(Err(error)) => {
                let message = format!("The project folder picker could not be opened: {error}");
                let _ = cx.update(|app| {
                    notification_center(app).update(app, |center, cx| {
                        center.push(Notification::error("Open Project", message), cx);
                    });
                });
            }
            Ok(Ok(None)) | Err(_) => {}
        }
    })
    .detach();
}

/// Rebuild the AI live-bridge [`LiveSnapshot`] from the current workspace +
/// explorer state. Called event-driven (T17-006) from `cx.observe` on the
/// workspace + explorer, instead of every frame.
pub(crate) fn refresh_live_snapshot(
    workspace: &Entity<Workspace>,
    explorer: &Entity<ExplorerView>,
    bridge: &WorkspaceLiveBridge,
    cx: &App,
) {
    let _span = tracing::trace_span!(target: "labonair::perf", "live_snapshot_recompute").entered();
    let ws = workspace.read(cx);
    bridge.set_snapshot(LiveSnapshot {
        cwd: ws.active_cwd(cx),
        workspace_root: explorer.read(cx).root().map(|p| p.display().to_string()),
        terminal_lines: ws.active_terminal_lines(200, cx),
        ssh_tab_id: ws.active_remote_target(cx).map(|(_, sid)| sid),
        has_terminal: ws.active_is_terminal(cx),
    });
}

/// Register the built-in panels on the workspace's `PanelRegistry`.
///
/// This is the **only** place in the app that names concrete panel types.
fn register_builtin_panels(
    workspace: &Entity<Workspace>,
    explorer: &Entity<ExplorerView>,
    git_panel: &Entity<GitPanelView>,
    git_graph: &Entity<GitGraphView>,
    snippets: &Entity<SnippetsView>,
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
    ];
    workspace.update(cx, |w, _cx| {
        let registry = w.panel_registry_mut();
        for registration in registrations {
            registry.register(registration);
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bootstrap(
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    notifications: Entity<NotificationCenter>,
    backend: BackendComposition,
    tokio: TokioHandle,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> AppShell {
    // T18-006: one-time migration of the legacy `barItemPlacements` blob into
    // `statusBarItemPlacements`. Must run before the first `StatusItemRegistry`
    // build (`register_builtin_status_items` below reloads placements right
    // after registering every item).
    match labonair_backend::modules::settings::migrations::migrate_bar_item_placements(
        &labonair_filesystem::paths::config_dir(),
    ) {
        Ok(outcome) => tracing::info!("bar item placement migration: {outcome:?}"),
        Err(err) => tracing::warn!("bar item placement migration failed: {err}"),
    }

    cx.observe(&background, |_, _, cx| cx.notify()).detach();

    let backend_mcp = Arc::new(BackendMcpSessionAccess::new(
        backend.mcp.clone(),
        backend.db.clone(),
    ));
    let agent_access_service: Arc<dyn McpSessionAccessService> = backend_mcp.clone();
    let mcp_tab_operations: Arc<dyn McpTabOperationService> = backend_mcp;
    let mcp_server_access = labonair_backend::modules::mcp::McpServerAccess::new(
        backend.ssh.clone(),
        backend.pty.clone(),
        backend.db.clone(),
        backend.secrets.clone(),
        backend.events.clone(),
    );
    let agent_access =
        cx.new(|_| AgentAccessStore::new(agent_access_service.clone(), tokio.clone()));

    // The Rust `McpState` boots with no persistence of its own — mirror the
    // saved preferences into it once at startup. Port/timeout/auto-revoke first
    // so the listener, if enabled, comes up on the right port.
    {
        let prefs = mcp_prefs_load();
        agent_access.update(cx, |s, cx| {
            s.hydrate(prefs.bridge_enabled, prefs.notify_on_activity, cx)
        });
        let mcp_server_access = mcp_server_access.clone();
        let mcp_state = backend.mcp.clone();
        tokio.spawn(async move {
            let _ = mcp_set_port(prefs.bridge_port, mcp_server_access.clone(), &mcp_state).await;
            let _ =
                mcp_set_max_command_timeout_secs(prefs.max_command_timeout_secs, &mcp_state).await;
            let _ = mcp_set_auto_revoke_minutes(prefs.auto_revoke_minutes, &mcp_state).await;
            if prefs.bridge_enabled {
                let _ = mcp_set_enabled(true, mcp_server_access, &mcp_state).await;
            }
        });
    }

    let registry = Arc::new(TerminalRegistry::new());
    // Session restore (T14-001): load the previous snapshot up-front so the
    // workspace can replay it instead of opening the default tabs.
    let session_snapshot = GeneralSettings::try_get(cx)
        .map(|s| s.session_restore())
        .unwrap_or(false)
        .then(crate::session::load_snapshot)
        .flatten();
    let ssh_service: Arc<dyn SshConnectionService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshConnectionService::new(
            backend.ssh.clone(),
            backend.trust.clone(),
            backend.db.clone(),
            backend.secrets.clone(),
            backend.events.clone(),
        ),
    );
    let ssh_pty_service: Arc<dyn SshPtyService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshPtyService::new(backend.ssh.clone()),
    );
    let ssh_remote_service: Arc<dyn SshRemoteCommandService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshRemoteService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ),
    );
    let ssh_remote_file_service: Arc<dyn SshRemoteFileService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshRemoteService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ),
    );
    let ssh_tunnel_service: Arc<dyn SshTunnelService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshTunnelService::new(
            backend.tunnels.clone(),
            backend.db.clone(),
            backend.secrets.clone(),
            backend.trust.clone(),
            backend.events.clone(),
        ),
    );
    let ssh_tester: Arc<dyn SshConnectionTester> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshConnectionTester::new(
            backend.trust.clone(),
            backend.db.clone(),
            backend.secrets.clone(),
            backend.events.clone(),
        ),
    );
    let ssh_config: Arc<dyn SshConfigService> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshConfigService::new(backend.db.clone()),
    );
    let ssh_event_source: Arc<dyn SshEventSource> = Arc::new(
        labonair_backend::modules::ssh::contract::BackendSshEventSource::new(
            backend.events.clone(),
        ),
    );
    let mcp_event_source: Arc<dyn McpEventSource> =
        Arc::new(BackendMcpEventSource::new(backend.events.clone()));
    let host_manager = {
        let mcp_state_for_host_events = backend.mcp.clone();
        let events_for_host_events = backend.events.clone();
        let host_event_handler = Arc::new(move |event| {
            labonair_backend::modules::mcp::revoke_agent_access(
                &mcp_state_for_host_events,
                &events_for_host_events,
                event,
            )
        });
        cx.new(|cx| {
            HostManagerView::new(
                backend.db.clone(),
                backend.secrets.clone(),
                labonair_filesystem::paths::data_dir(),
                Some(host_event_handler),
                ssh_tester,
                ssh_config,
                tokio.clone(),
                theme.clone(),
                cx,
            )
        })
    };
    let sftp_session_service: Arc<dyn SftpSessionService> = Arc::new(
        labonair_backend::modules::sftp::contract::BackendSftpService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ),
    );
    let sftp_browser_service: Arc<dyn SftpBrowserService> = Arc::new(
        labonair_backend::modules::sftp::contract::BackendSftpService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ),
    );
    let transfer_service: Arc<dyn TransferService> = Arc::new(
        labonair_backend::modules::transfers::BackendTransferService::new(backend.transfer.clone()),
    );
    let transfer_events: Arc<dyn TransferEventSource> = Arc::new(
        labonair_backend::modules::transfers::BackendTransferEventSource::new(
            backend.events.clone(),
        ),
    );
    let git_service: Arc<dyn labonair_git::GitService> =
        Arc::new(labonair_backend::modules::git::BackendGitService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ));
    let git_graph_service: Arc<dyn labonair_git::GitGraphService> =
        Arc::new(labonair_backend::modules::git::BackendGitGraphService::new(
            backend.ssh.clone(),
            backend.events.clone(),
        ));
    let transfers = cx.new(|cx| {
        TransfersView::new(
            transfer_service.clone(),
            transfer_events,
            tokio.clone(),
            theme.clone(),
            cx,
        )
    });
    let workspace = cx.new(|cx| {
        Workspace::new(
            registry,
            theme.clone(),
            background.clone(),
            ssh_service.clone(),
            ssh_pty_service.clone(),
            ssh_remote_service.clone(),
            ssh_remote_file_service.clone(),
            ssh_tunnel_service.clone(),
            sftp_session_service.clone(),
            sftp_browser_service.clone(),
            git_service.clone(),
            git_graph_service.clone(),
            ssh_event_source,
            mcp_event_source,
            agent_access_service,
            mcp_tab_operations,
            transfer_service,
            tokio.clone(),
            agent_access.clone(),
            host_manager.clone(),
            session_snapshot,
            window,
            cx,
        )
    });
    // Workspace emits intent for cross-surface navigation; it does not hold a
    // shell callback or know how the Hosts surface is presented.
    cx.subscribe_in(
        &workspace,
        window,
        |this, _, event: &WorkspaceEvent, _window, cx| match event {
            WorkspaceEvent::OpenHosts => {
                open_hosts_window(this.panels.hosts.clone(), cx);
            }
            WorkspaceEvent::OpenProject => open_project_picker(this.workspace.clone(), cx),
        },
    )
    .detach();
    // Shell re-render on workspace change — keeps the `.when(can_split)` action
    // bindings in `render` in sync with the active tab.
    cx.observe(&workspace, |_, _, cx| cx.notify()).detach();

    let git_panel =
        cx.new(|cx| GitPanelView::new(git_service.clone(), tokio.clone(), theme.clone(), cx));
    // Source Control → workspace Project Diff (Zed-parity Phase 4, §12.6). The
    // panel emits a neutral `ProjectDiffRequest`; the workspace owns the single
    // Project Diff item's lifecycle (idempotent open/focus).
    cx.subscribe_in(
        &git_panel,
        window,
        |this, _, event: &ScmEvent, _window, cx| {
            let ScmEvent::OpenProjectDiff(req) = event;
            let req = req.clone();
            this.workspace
                .update(cx, |w, cx| w.open_project_diff(req, cx));
        },
    )
    .detach();

    let git_graph =
        cx.new(|cx| GitGraphView::new(git_graph_service, tokio.clone(), theme.clone(), cx));
    // The workspace renders the Git Graph as a `TabKind::GitGraph` tab — share
    // this single entity so the app-shell keeps feeding it the active CWD.
    workspace.update(cx, |w, _cx| w.set_git_graph(git_graph.clone()));

    // Apply the persisted theme preference + font/registry state to the
    // ThemeStore once at startup; further changes flow through the layered
    // `SettingsStore` (`SettingsView`'s generated field grid writes straight
    // to it) and the `SettingsStore` observer below.
    labonair_settings_ui::apply_prefs_to_theme(&theme, cx);
    // T20-007: re-derive the `ThemeMetrics` (font scales / UI density /
    // corner-radius scale / reduce-motion) whenever the layered settings
    // change. Generated `appearance` fields write straight to `SettingsStore`,
    // bypassing `apply_prefs_to_theme`, so the observer is what keeps density
    // & co. live. Idempotent — `set_metrics` no-ops on an unchanged value.
    if cx.has_global::<labonair_settings::SettingsStore>() {
        let theme_m = theme.clone();
        cx.observe_global::<labonair_settings::SettingsStore>(move |_this, cx| {
            labonair_settings_ui::apply_theme_metrics(&theme_m, cx);
        })
        .detach();
    }
    cx.observe(&host_manager, |_, _, cx| cx.notify()).detach();
    cx.subscribe(&host_manager, |this, _, event: &HostManagerEvent, cx| {
        let HostManagerEvent::Open(request) = event;
        this.workspace.update(cx, |workspace, cx| {
            workspace.enqueue_host_request(request.clone(), cx)
        });
    })
    .detach();
    // `keymap.json` (T19-008): load + merge + bind, publish the display
    // global, then live-watch the file so an edit takes effect with no
    // restart. Must run after the theme/prefs wiring above so a startup
    // banner (if the shipped default asset somehow fails to parse) has a
    // notification center to post into.
    // Build the command metadata and behaviour registry before keymap loading:
    // default shortcut resolution must use the same descriptors that the
    // palette receives later.
    let command_registry = crate::commands::register_builtin_commands();
    crate::keymap_loader::reload_and_apply(cx, &command_registry);
    crate::keymap_loader::watch(cx, command_registry.clone());
    set_settings_deps(settings_services(), tokio.clone(), cx);
    // Auto-updater (T15-005). Kicks a quiet background check at startup when the
    // `checkForUpdates` preference is on (6 h backoff inside the store).
    let updater = cx.new(|cx| UpdaterView::new(tokio.clone(), theme.clone(), cx));
    if GeneralSettings::try_get(cx)
        .map(|s| s.check_for_updates())
        .unwrap_or(true)
    {
        updater.update(cx, |u, cx| u.run_check(false, cx));
    }

    let snippets = cx.new(|cx| {
        let ssh_executor = std::sync::Arc::new(
            labonair_backend::modules::snippets::exec::BackendSshExecutor::new(
                backend.ssh.clone(),
                backend.snippet_run.clone(),
            ),
        );
        SnippetsView::new(
            backend.db.clone(),
            tokio.clone(),
            theme.clone(),
            workspace.clone(),
            ssh_executor,
            cx,
        )
    });

    // The AI live-bridge stays wired to the workspace (snapshot feed + command
    // drain below) even though the frontend AI panel is parked — the bridge is
    // the reusable seam for the AI-system rebuild.
    let live_bridge = WorkspaceLiveBridge::new();

    let command_palette = cx.new(|cx| CommandPalette::new(theme.clone(), workspace.clone(), cx));
    cx.subscribe_in(
        &command_palette,
        window,
        |this, _, event: &PaletteEvent, window, cx| {
            this.handle_palette_event(event.clone(), window, cx);
        },
    )
    .detach();

    let explorer = cx.new(|cx| ExplorerView::new(theme.clone(), workspace.clone(), cx));

    // Project identity is authoritative; standalone falls back to the active
    // terminal's cwd and then $HOME.
    {
        let initial = workspace.read(cx).filesystem_root(cx);
        explorer.update(cx, |e, cx| e.set_root_str(initial, cx));
    }
    cx.observe(&workspace, {
        let explorer = explorer.clone();
        move |_, workspace, cx| {
            let root = workspace.read(cx).filesystem_root(cx);
            explorer.update(cx, |e, cx| e.set_root_str(root, cx));
        }
    })
    .detach();
    cx.observe(&workspace, {
        let git_panel = git_panel.clone();
        let git_graph = git_graph.clone();
        move |_, workspace, cx| {
            let root = workspace.read(cx).git_root(cx);
            git_panel.update(cx, |g, cx| g.set_root(root.clone(), cx));
            git_graph.update(cx, |g, cx| g.set_root(root, cx));
        }
    })
    .detach();
    {
        let root = workspace.read(cx).git_root(cx);
        git_panel.update(cx, |g, cx| g.set_root(root.clone(), cx));
        git_graph.update(cx, |g, cx| g.set_root(root, cx));
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
        move |window, cx| {
            if let WindowBounds::Windowed(bounds) = window.window_bounds() {
                window_state::save(bounds);
            }
            let session_restore = GeneralSettings::try_get(cx)
                .map(|s| s.session_restore())
                .unwrap_or(false);
            if session_restore {
                let snapshot = workspace.read(cx).session_snapshot(cx);
                crate::session::save_snapshot(&snapshot);
            } else {
                crate::session::clear_snapshot();
                labonair_terminal::scrollback::cleanup(
                    &labonair_filesystem::paths::data_dir(),
                    &[],
                    None,
                );
            }
            true
        }
    });

    // The registry must be populated before the docks are built from it.
    register_builtin_panels(&workspace, &explorer, &git_panel, &git_graph, &snippets, cx);

    // Build the three docks from the workspace-owned layout file. The app
    // startup migration has already moved legacy Settings values there.
    let layout = labonair_workspace::layout::load();
    workspace.update(cx, |w, cx| {
        w.set_primary_dock(layout.primary_position());
        w.init_docks(&layout.docks_json(), window, cx);
    });

    // Populate the status-bar item registry, then build the `StatusBar` view.
    register_builtin_status_items(
        &workspace,
        &theme,
        &notifications,
        &updater,
        &agent_access,
        &transfers,
        cx,
    );
    let status_bar = cx.new(|cx| StatusBar::new(workspace.clone(), theme.clone(), cx));

    let modal_layer = cx.new(|_| ModalLayer::new());
    let titlebar = cx.new(|cx| Titlebar::new(theme.clone(), workspace.clone(), cx));
    cx.subscribe_in(
        &titlebar,
        window,
        |this, _, event: &TitlebarEvent, window, cx| match event {
            TitlebarEvent::Settings => {
                labonair_settings_ui::open_settings_window(None, cx);
            }
            TitlebarEvent::Keymap => {
                let descriptors = this.command_registry.descriptors();
                let workspace = this.workspace.clone();
                labonair_keymap_ui::open_keymap_window(
                    descriptors,
                    move |window, cx| {
                        workspace.update(cx, |workspace, cx| {
                            workspace.open_or_create_user_keymap_json(window, cx);
                        });
                    },
                    cx,
                );
            }
            TitlebarEvent::Hosts => {
                open_hosts_window(this.panels.hosts.clone(), cx);
            }
            TitlebarEvent::Palette(page) => {
                this.show_command_palette(Some(*page), window, cx);
            }
        },
    )
    .detach();

    // `git_graph` is not kept on the shell: the workspace owns the shared
    // `Entity<GitGraphView>` (via `set_git_graph`) and the CWD-feed closure
    // above captured its own clone.
    let panels = ShellPanels {
        git_panel,
        snippets,
        updater,
        command_palette,
        hosts: host_manager,
    };

    AppShell::from_parts(
        theme,
        background,
        workspace,
        titlebar,
        panels,
        status_bar,
        command_registry,
        modal_layer,
        live_bridge,
        live_drain,
        cx,
    )
}
