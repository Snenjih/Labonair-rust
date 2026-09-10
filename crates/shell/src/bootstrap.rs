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

use std::{rc::Rc, sync::Arc};

use gpui::{App, AppContext, Context, Entity, PathPromptOptions, Window, WindowBounds};
use labonair_hosts_ui::{open_hosts_window, HostManagerEvent, HostManagerView};
use labonair_mcp_core::{
    preferences::McpPreferences, McpEventSource, McpSessionAccessService, McpTabOperationService,
};
use labonair_mcp_server::contract::{McpEventSourceAdapter, McpSessionAccessAdapter};
use labonair_mcp_server::{
    mcp_set_auto_revoke_minutes, mcp_set_enabled, mcp_set_max_command_timeout_secs, mcp_set_port,
};
use labonair_notifications::{notification_center, Notification, NotificationCenter};
use labonair_sftp::{SftpBrowserService, SftpSessionService};
use labonair_ssh::{
    SshConfigService, SshConnectionService, SshConnectionTester, SshEventSource, SshPtyService,
    SshRemoteCommandService, SshRemoteFileService, SshTunnelService,
};
use labonair_terminal::TerminalRegistry;
use labonair_transfers::{TransferEventSource, TransferService};
use labonair_transfers_ui::{TransferUiEvent, TransfersView};
use tokio::runtime::Handle as TokioHandle;

use labonair_command_palette::{CommandPalette, Page as PalettePage, PaletteEvent};
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
use crate::composition::AppComposition;
use crate::local_terminal_access::TerminalRegistryAccess;
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn bootstrap(
    theme: Entity<ThemeStore>,
    background: Entity<BackgroundStore>,
    notifications: Entity<NotificationCenter>,
    composition: AppComposition,
    tokio: TokioHandle,
    window: &mut Window,
    cx: &mut Context<AppShell>,
) -> AppShell {
    // T18-006: one-time migration of the legacy `barItemPlacements` blob into
    // `statusBarItemPlacements`. Must run before the first `StatusItemRegistry`
    // build (`register_builtin_status_items` below reloads placements right
    // after registering every item).
    match labonair_workspace::status_placements::migrate_legacy_status_bar_placements(
        &labonair_filesystem::paths::config_dir(),
    ) {
        Ok(outcome) => tracing::info!("bar item placement migration: {outcome:?}"),
        Err(err) => tracing::warn!("bar item placement migration failed: {err}"),
    }

    cx.observe(&background, |_, _, cx| cx.notify()).detach();

    let registry = Arc::new(TerminalRegistry::new());
    let local_terminal_access = Arc::new(TerminalRegistryAccess::new(registry.clone()));
    let mcp_access = Arc::new(McpSessionAccessAdapter::new(
        composition.mcp().clone(),
        composition.db().clone(),
    ));
    let agent_access_service: Arc<dyn McpSessionAccessService> = mcp_access.clone();
    let mcp_tab_operations: Arc<dyn McpTabOperationService> = mcp_access;
    let mcp_server_access = labonair_mcp_server::McpServerAccess::new(
        composition.ssh().clone(),
        local_terminal_access,
        composition.db().clone(),
        composition.secrets().clone(),
        composition.events().clone(),
    );
    let agent_access =
        cx.new(|_| AgentAccessStore::new(agent_access_service.clone(), tokio.clone()));

    // The Rust `McpState` boots with no persistence of its own — mirror the
    // saved preferences into it once at startup. Port/timeout/auto-revoke first
    // so the listener, if enabled, comes up on the right port.
    {
        let prefs = McpPreferences::load_from(&labonair_filesystem::paths::config_dir());
        agent_access.update(cx, |s, cx| {
            s.hydrate(prefs.bridge_enabled, prefs.notify_on_activity, cx)
        });
        let mcp_server_access = mcp_server_access.clone();
        let mcp_state = composition.mcp().clone();
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

    // Session restore (T14-001): load the previous snapshot up-front so the
    // workspace can replay it instead of opening the default tabs.
    let session_snapshot = GeneralSettings::try_get(cx)
        .map(|s| s.session_restore())
        .unwrap_or(false)
        .then(crate::session::load_snapshot)
        .flatten();
    let ssh_service: Arc<dyn SshConnectionService> = Arc::new(
        labonair_ssh_transport::contract::SshConnectionServiceAdapter::new(
            composition.ssh().clone(),
            composition.trust().clone(),
            composition.db().clone(),
            composition.secrets().clone(),
            composition.events().clone(),
        ),
    );
    let ssh_pty_service: Arc<dyn SshPtyService> = Arc::new(
        labonair_ssh_transport::contract::SshPtyServiceAdapter::new(composition.ssh().clone()),
    );
    let ssh_remote_service: Arc<dyn SshRemoteCommandService> = Arc::new(
        labonair_ssh_transport::contract::SshRemoteServiceAdapter::new(
            composition.ssh().clone(),
            composition.events().clone(),
        ),
    );
    let ssh_remote_file_service: Arc<dyn SshRemoteFileService> = Arc::new(
        labonair_ssh_transport::contract::SshRemoteServiceAdapter::new(
            composition.ssh().clone(),
            composition.events().clone(),
        ),
    );
    let ssh_tunnel_service: Arc<dyn SshTunnelService> = Arc::new(
        labonair_ssh_transport::contract::SshTunnelServiceAdapter::new(
            composition.tunnels().clone(),
            composition.db().clone(),
            composition.secrets().clone(),
            composition.trust().clone(),
            composition.events().clone(),
        ),
    );
    let ssh_tester: Arc<dyn SshConnectionTester> = Arc::new(
        labonair_ssh_transport::contract::SshConnectionTesterAdapter::new(
            composition.trust().clone(),
            composition.db().clone(),
            composition.secrets().clone(),
            composition.events().clone(),
        ),
    );
    let ssh_config: Arc<dyn SshConfigService> = Arc::new(
        labonair_ssh_transport::contract::SshConfigServiceAdapter::new(composition.db().clone()),
    );
    let ssh_event_source: Arc<dyn SshEventSource> = Arc::new(
        labonair_ssh_transport::contract::SshEventSourceAdapter::new(composition.events().clone()),
    );
    let mcp_event_source: Arc<dyn McpEventSource> =
        Arc::new(McpEventSourceAdapter::new(composition.events().clone()));
    let host_manager = {
        let mcp_state_for_host_events = composition.mcp().clone();
        let events_for_host_events = composition.events().clone();
        let host_event_handler = Arc::new(move |event| {
            labonair_mcp_server::revoke_agent_access(
                &mcp_state_for_host_events,
                &events_for_host_events,
                event,
            )
        });
        cx.new(|cx| {
            HostManagerView::new(
                composition.db().clone(),
                composition.secrets().clone(),
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
    let sftp_session_service: Arc<dyn SftpSessionService> =
        Arc::new(labonair_sftp_ssh::contract::SftpTransportService::new(
            composition.ssh().clone(),
            composition.events().clone(),
        ));
    let sftp_browser_service: Arc<dyn SftpBrowserService> =
        Arc::new(labonair_sftp_ssh::contract::SftpTransportService::new(
            composition.ssh().clone(),
            composition.events().clone(),
        ));
    let transfer_service: Arc<dyn TransferService> = Arc::new(
        labonair_transfers_ssh::adapter::TransferServiceAdapter::new(
            composition.transfer().clone(),
        ),
    );
    let transfer_events: Arc<dyn TransferEventSource> = Arc::new(
        labonair_transfers_ssh::adapter::TransferEventSourceAdapter::new(
            composition.events().clone(),
        ),
    );
    let git_service: Arc<dyn labonair_git::GitService> =
        Arc::new(labonair_git_transport::GitTransportService::new(
            composition.ssh().clone(),
            composition.events().clone(),
        ));
    let git_graph_service: Arc<dyn labonair_git::GitGraphService> =
        Arc::new(labonair_git_transport::GitGraphTransportService::new(
            composition.ssh().clone(),
            composition.events().clone(),
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
    // Workspace/Terminal render a background layer but must not depend on
    // `labonair-background` for image storage/import/decoding (B02): inject
    // the narrow presentation capability instead of the concrete store.
    let background_host = labonair_background::host(&background, cx);
    let workspace = cx.new(|cx| {
        Workspace::new(
            registry,
            theme.clone(),
            background_host,
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
            {
                // R08-012: Workspace reaches the Hosts UI only through this
                // narrow `HostView` contract, not an `Entity<HostManagerView>`.
                let hm = host_manager.clone();
                labonair_hosts_host::HostView::new(
                    {
                        let hm = hm.clone();
                        move |cx| hm.read(cx).host_ids()
                    },
                    {
                        let hm = hm.clone();
                        move |id, cx| hm.read(cx).host_name(id)
                    },
                    {
                        let hm = hm.clone();
                        move |id, cx| hm.read(cx).jump_host_label(id)
                    },
                    {
                        let hm = hm.clone();
                        move |cx| hm.read(cx).picker_rows()
                    },
                    {
                        let hm = hm.clone();
                        move |n, cx| hm.read(cx).recent_picker_rows(n)
                    },
                    {
                        let hm = hm.clone();
                        move |id, status, cx| {
                            hm.update(cx, |h, cx| h.set_status(id, status, cx));
                        }
                    },
                    move |rows, cx| {
                        hm.update(cx, |h, cx| h.set_active_tunnels(rows, cx));
                    },
                )
            },
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

    // Route the Transfers UI completion signal to the SFTP pane refresh. The
    // transfer view emits only the typed event (`docs/registries.md`); the
    // composition root owns the Workspace-side wiring so `transfers-ui` does
    // not depend on `labonair-workspace`.
    cx.subscribe(&transfers, {
        let workspace = workspace.clone();
        move |_this, _transfers, event: &TransferUiEvent, cx| {
            let TransferUiEvent::Completed {
                session_id,
                direction,
            } = event;
            workspace.update(cx, |workspace, cx| {
                workspace.refresh_sftp_after_transfer(session_id, *direction, cx);
            });
        }
    })
    .detach();

    let git_panel =
        cx.new(|cx| GitPanelView::new(git_service.clone(), tokio.clone(), theme.clone(), cx));
    // Source Control → workspace surfaces. The panel emits neutral requests; the
    // workspace owns each surface's lifecycle (idempotent open/focus): the single
    // Project Diff item (Zed-parity Phase 4, §12.6) and the on-demand Git Graph tab.
    cx.subscribe_in(
        &git_panel,
        window,
        |this, _, event: &ScmEvent, _window, cx| match event {
            ScmEvent::OpenProjectDiff(req) => {
                let req = req.clone();
                this.workspace
                    .update(cx, |w, cx| w.open_project_diff(req, cx));
            }
            ScmEvent::OpenGitGraph => {
                this.workspace.update(cx, |w, cx| w.open_git_graph_tab(cx));
            }
        },
    )
    .detach();

    let git_graph =
        cx.new(|cx| GitGraphView::new(git_graph_service, tokio.clone(), theme.clone(), cx));
    // The workspace renders the Git Graph as a `TabKind::GitGraph` tab — share
    // this single entity so the app-shell keeps feeding it the active CWD.
    workspace.update(cx, |w, _cx| w.set_git_graph(git_graph.clone()));

    // Git Graph → workspace Diff tab. The panel no longer renders commit diffs
    // inline; "View Changes" emits a neutral request the workspace opens as the
    // single read-only Diff item (mirrors the Source-Control panel's flow).
    cx.subscribe_in(
        &git_graph,
        window,
        |this, _, event: &labonair_panel_git_graph::GitGraphEvent, _window, cx| match event {
            labonair_panel_git_graph::GitGraphEvent::OpenCommitDiff {
                repo_root,
                session_id,
                hash,
                subject,
            } => {
                let req = labonair_panel::ProjectDiffRequest {
                    repo_root: repo_root.clone(),
                    session_id: session_id.clone(),
                    source: labonair_panel::DiffSource::Commit {
                        hash: hash.clone(),
                        subject: subject.clone(),
                    },
                    files: Vec::new(),
                    selected: None,
                    mode: labonair_panel::ProjectDiffMode::Unified,
                };
                this.workspace
                    .update(cx, |w, cx| w.open_project_diff(req, cx));
            }
        },
    )
    .detach();

    // Apply the persisted theme preference + font/registry state to the
    // ThemeStore once at startup; further changes flow through the layered
    // `SettingsStore` (`SettingsView`'s generated field grid writes straight
    // to it) and the `SettingsStore` observer below.
    labonair_theme_ui::apply_prefs_to_theme(&theme, cx);
    // T20-007: re-derive the `ThemeMetrics` (font scales / UI density /
    // corner-radius scale / reduce-motion) whenever the layered settings
    // change. Generated `appearance` fields write straight to `SettingsStore`,
    // bypassing `apply_prefs_to_theme`, so the observer is what keeps density
    // & co. live. Idempotent — `set_metrics` no-ops on an unchanged value.
    if cx.has_global::<labonair_settings::SettingsStore>() {
        let theme_m = theme.clone();
        cx.observe_global::<labonair_settings::SettingsStore>(move |_this, cx| {
            labonair_theme_ui::apply_theme_metrics(&theme_m, cx);
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
    // Auto-updater (T15-005). Create its owner entity before composing the
    // command registry so its command handler can be registered at startup.
    let updater = cx.new(|cx| UpdaterView::new(tokio.clone(), theme.clone(), cx));
    if GeneralSettings::try_get(cx)
        .map(|s| s.check_for_updates())
        .unwrap_or(true)
    {
        updater.update(cx, |u, cx| u.run_check(false, cx));
    }

    // `keymap.json` (T19-008): load + merge + bind, publish the display
    // global, then live-watch the file so an edit takes effect with no
    // restart. Must run after the theme/prefs wiring above so a startup
    // banner (if the shipped default asset somehow fails to parse) has a
    // notification center to post into.
    // Build the command metadata and behaviour registry before keymap loading:
    // default shortcut resolution must use the same descriptors that the
    // palette receives later.
    let shell = cx.entity();
    let search_shell = shell.clone();
    let search_toggle: labonair_workspace::command_provider::SearchToggleHandler =
        Rc::new(move |window: &mut Window, app: &mut App| {
            search_shell.update(app, |shell, cx| shell.toggle_search_overlay(window, cx));
        });
    let palette_shell = shell;
    let host_picker_shell = palette_shell.clone();
    let palette_toggle: labonair_command_palette::command_provider::ToggleHandler =
        Rc::new(move |window: &mut Window, app: &mut App| {
            palette_shell.update(app, |shell, cx| shell.toggle_command_palette(window, cx));
        });
    let host_picker: labonair_hosts_ui::command_provider::HostPickerHandler =
        Rc::new(move |window: &mut Window, app: &mut App| {
            host_picker_shell.update(app, |shell, cx| {
                shell.show_command_palette(Some(PalettePage::Hosts), window, cx);
            });
        });
    let mut command_registry = crate::commands::register_builtin_commands_for(
        &workspace,
        &updater,
        &host_manager,
        palette_toggle,
        search_toggle,
        host_picker,
    );
    crate::keymap_loader::reload_and_apply(cx, &command_registry);
    crate::keymap_loader::watch(cx, command_registry.clone());
    set_settings_deps(settings_services(), tokio.clone(), cx);
    // Snippets no longer holds the workspace entity (R08-003): it runs
    // snippets through this narrow execution-host contract wired to the
    // active Workspace.
    let snippet_exec_host = {
        let ws = workspace.clone();
        labonair_snippets_host::SnippetExecutionHost::new(
            {
                let ws = ws.clone();
                move |command, cx| {
                    ws.update(cx, |w, cx| w.inject_into_active_terminal(command, cx));
                }
            },
            {
                let ws = ws.clone();
                move |cwd, command, window, cx| {
                    ws.update(cx, |w, cx| w.run_snippet_local(cwd, command, window, cx));
                }
            },
            {
                let ws = ws.clone();
                move |host_id, command, window, cx| {
                    ws.update(cx, |w, cx| {
                        w.run_snippet_ssh_terminal(host_id, command, window, cx)
                    });
                }
            },
            move |host_id, cx| ws.read(cx).ssh_session_for_host(host_id),
        )
    };
    let snippets = cx.new(|cx| {
        let ssh_executor =
            std::sync::Arc::new(labonair_snippets_ssh::exec::SshSnippetExecutor::new(
                composition.ssh().clone(),
                composition.snippet_run().clone(),
            ));
        SnippetsView::new(
            composition.db().clone(),
            tokio.clone(),
            theme.clone(),
            snippet_exec_host,
            ssh_executor,
            cx,
        )
    });

    // Dynamic palette actions are contributed by their owning capabilities.
    // The shell supplies only narrow composition callbacks for operations
    // that cross an entity boundary (for example opening a host in the
    // workspace). It does not decode feature-specific action variants.
    labonair_workspace::command_provider::register_palette_action_handlers(
        command_registry.palette_action_handlers_mut(),
        &workspace,
    );
    let host_workspace = workspace.clone();
    let open_host: labonair_hosts_ui::command_provider::HostOpenHandler =
        Rc::new(move |request, window, cx| {
            host_workspace.update(cx, |workspace, cx| {
                workspace.open_host_request(request, window, cx);
            });
        });
    labonair_hosts_ui::command_provider::register_palette_action_handlers(
        command_registry.palette_action_handlers_mut(),
        open_host,
    );
    labonair_theme_ui::command_provider::register_palette_action_handlers(
        command_registry.palette_action_handlers_mut(),
        &theme,
    );
    labonair_panel_scm::register_palette_action_handlers(
        command_registry.palette_action_handlers_mut(),
        &git_panel,
    );
    labonair_panel_snippets::register_palette_action_handlers(
        command_registry.palette_action_handlers_mut(),
        &snippets,
    );

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

    // Explorer no longer holds the workspace entity (R07-004): it opens files,
    // terminals and previews through this narrow host contract, and the
    // observer below re-notifies it when the active editor changes.
    let explorer_host = {
        let ws = workspace.clone();
        labonair_explorer_host::ExplorerHost::new(
            {
                let ws = ws.clone();
                move |path, peek, window, cx| {
                    ws.update(cx, |w, cx| w.open_file(path, peek, window, cx));
                }
            },
            {
                let ws = ws.clone();
                move |cwd, window, cx| {
                    ws.update(cx, |w, cx| w.new_terminal_tab_in(Some(cwd), window, cx));
                }
            },
            {
                let ws = ws.clone();
                move |target, window, cx| {
                    ws.update(cx, |w, cx| w.open_preview(target, window, cx));
                }
            },
            move |cx| ws.read(cx).active_file_path(cx),
        )
    };
    let explorer = cx.new(|cx| ExplorerView::new(theme.clone(), explorer_host, cx));

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
            explorer.update(cx, |e, cx| {
                e.set_root_str(root, cx);
                e.notify_active_file_changed(cx);
            });
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

    // The Tabs sidebar panel is owned by `labonair-workspace` (it renders the
    // workspace tab list); composition only creates the view and forwards its
    // contribution like the other panel owners.
    let tabs_panel = cx.new(|cx| {
        labonair_workspace::tabs_panel::TabsPanel::new(workspace.clone(), theme.clone(), cx)
    });

    // Each panel owner builds its typed contribution; composition only inserts
    // the contributions into the shared workspace registry.
    let panel_contributions = [
        labonair_panel_explorer::panel_registration(&explorer, cx),
        labonair_panel_scm::panel_registration(&git_panel, cx),
        labonair_panel_snippets::panel_registration(&snippets, cx),
        labonair_workspace::tabs_panel::tabs_panel_registration(&tabs_panel, cx),
    ];
    workspace.update(cx, |workspace, _cx| {
        let registry = workspace.panel_registry_mut();
        for contribution in panel_contributions {
            registry.register(contribution);
        }
    });

    // Build the three docks from the workspace-owned layout file. The app
    // startup migration has already moved legacy Settings values there.
    let layout = labonair_workspace::layout::load();
    workspace.update(cx, |w, cx| {
        w.set_primary_dock(layout.primary_position());
        w.init_docks(&layout.docks_json(), window, cx);
        // Place / remove the Tabs panel to match `tabsLocation`.
        w.sync_tabs_in_sidebar(cx);
    });

    // Keep the Tabs panel's dock membership in sync when `tabsLocation`
    // changes at runtime, and push the live-tunable `terminal` settings
    // (scrollback depth, cursor shape/blink/interval, word separators,
    // opacity) into every open terminal pane. The generated Settings fields
    // write straight to `SettingsStore`, so the observer is what keeps these
    // live for already-open terminals.
    if cx.has_global::<labonair_settings::SettingsStore>() {
        let workspace_for_settings = workspace.clone();
        cx.observe_global::<labonair_settings::SettingsStore>(move |_, cx| {
            workspace_for_settings.update(cx, |w, cx| {
                w.sync_tabs_in_sidebar(cx);
                w.reapply_terminal_settings(cx);
            });
        })
        .detach();
    }

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
                this.workspace.update(cx, |workspace, cx| {
                    workspace.open_keymap_tab(descriptors, window, cx);
                });
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
