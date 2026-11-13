//! Composition of the SSH / SFTP / transfer services (ADR 0009).
//!
//! The in-process services are always built. In front of them sits the router
//! of `labonair-terminal-host`: with `terminal.sessionHost = detached` a new SSH
//! connection is made in the background session host, otherwise nothing here
//! touches the daemon. Credentials for a daemon connect are resolved from the
//! app's own hosts database and secret store.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use gpui::App;
use labonair_notifications::{notification_center, Notification};
use labonair_settings::content::terminal::SessionHostMode;
use labonair_settings::{Settings as _, SettingsStore, TerminalSettings};
use labonair_terminal_host::protocol::TransferConfig;
use labonair_terminal_host::proxy::router::{shared_access, LocalServices, RoutedServices};
use labonair_terminal_host::proxy::{install_context, snapshot, ProxyContext};
use labonair_transfers::TransferSettings;

use crate::composition::AppComposition;

fn transfer_config(settings: &TransferSettings) -> TransferConfig {
    let text = |value: &std::sync::Mutex<String>| {
        value
            .lock()
            .map(|v| v.clone())
            .unwrap_or_else(|_| "ask".into())
    };
    TransferConfig {
        max_concurrent: settings.max_concurrent.load(Ordering::Relaxed),
        chunk_size: settings.chunk_size.load(Ordering::Relaxed),
        default_conflict_resolution: text(&settings.default_conflict_resolution),
        on_folder_file_error: text(&settings.on_folder_file_error),
    }
}

/// The routed SSH / SFTP / transfer services of the app. Also installs the
/// hosted-session access remote git, snippets and the MCP bridge consult, and
/// rejoins a background host that is already running.
pub(crate) fn routed_services(composition: &AppComposition, cx: &mut App) -> RoutedServices {
    let ssh = composition.ssh().clone();
    let events = composition.events().clone();
    let sftp = Arc::new(labonair_sftp_ssh::contract::SftpTransportService::new(
        ssh.clone(),
        events.clone(),
    ));
    let remote = Arc::new(
        labonair_ssh_transport::contract::SshRemoteServiceAdapter::new(ssh.clone(), events.clone()),
    );
    let local = LocalServices {
        connection: Arc::new(
            labonair_ssh_transport::contract::SshConnectionServiceAdapter::new(
                ssh.clone(),
                composition.trust().clone(),
                composition.db().clone(),
                composition.secrets().clone(),
                events.clone(),
            ),
        ),
        pty: Arc::new(labonair_ssh_transport::contract::SshPtyServiceAdapter::new(
            ssh,
        )),
        remote_command: remote.clone(),
        remote_file: remote,
        tunnels: Arc::new(
            labonair_ssh_transport::contract::SshTunnelServiceAdapter::new(
                composition.tunnels().clone(),
                composition.db().clone(),
                composition.secrets().clone(),
                composition.trust().clone(),
                events.clone(),
            ),
        ),
        sftp_session: sftp.clone(),
        sftp_browser: sftp,
        transfers: Arc::new(
            labonair_transfers_ssh::adapter::TransferServiceAdapter::new(
                composition.transfer().clone(),
            ),
        ),
    };

    let (db, secrets) = (composition.db().clone(), composition.secrets().clone());
    let transfer_settings = composition.transfer().settings.clone();
    install_context(ProxyContext {
        bus: events,
        hosts: {
            let db = db.clone();
            Arc::new(move |host_id| snapshot::build(&db, &secrets, host_id))
        },
        transfer_config: Arc::new(move || transfer_config(&transfer_settings)),
        // The daemon records nothing in the app's database: keep the recents.
        connected: Arc::new(move |host_id| snapshot::record_connected(&db, host_id)),
    });

    // The router reads the mode on every connect, from any thread.
    let detached = Arc::new(AtomicBool::new(false));
    let scrollback = Arc::new(AtomicU32::new(5_000));
    let sync = {
        let (detached, scrollback) = (Arc::clone(&detached), Arc::clone(&scrollback));
        move |cx: &App| {
            if let Some(settings) = TerminalSettings::try_get(cx) {
                detached.store(
                    settings.session_host() == SessionHostMode::Detached,
                    Ordering::Relaxed,
                );
                scrollback.store(settings.detached_scrollback_lines(), Ordering::Relaxed);
            }
        }
    };
    sync(cx);
    if cx.has_global::<SettingsStore>() {
        cx.observe_global::<SettingsStore>(move |cx| sync(cx))
            .detach();
    }

    // A daemon that cannot be used is reported; the connection stays in-app.
    let (report, mut reports) = tokio::sync::mpsc::unbounded_channel::<String>();
    cx.spawn(async move |cx| {
        while let Some(error) = reports.recv().await {
            let _ = cx.update(|app| {
                notification_center(app).update(app, |center, cx| {
                    center.push(
                        Notification::error(
                            "Background session host unavailable",
                            "The SSH session runs inside the app and ends when the app quits.",
                        )
                        .source("ssh")
                        .details(error)
                        .dedupe_key("terminal:session-host-unavailable"),
                        cx,
                    );
                });
            });
        }
    })
    .detach();

    let routed = RoutedServices::new(
        local,
        shared_access(
            labonair_filesystem::paths::data_dir(),
            Arc::new(move || scrollback.load(Ordering::Relaxed)),
        ),
        Arc::new(move || detached.load(Ordering::Relaxed)),
        Arc::new(move |error| {
            let _ = report.send(error);
        }),
    );
    labonair_ssh_transport::hosted::install(Arc::new(routed.clone()));
    // Jobs and sessions that outlived the last run show up again (replay).
    let rejoin = routed.clone();
    std::thread::spawn(move || rejoin.attach_running_daemon());
    routed
}
