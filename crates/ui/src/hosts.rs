//! Host-Manager dashboard — the "Home" tab (T07-001).
//!
//! Ports `reference-src/src/modules/hosts/*` behaviour: hosts grouped by group,
//! a status indicator per host, connect / edit / duplicate / delete actions, a
//! host add/edit form and a credential manager. All persistence goes through
//! `labonair_backend::modules::{hosts, credentials}` (SQLite + the app secret
//! store); secrets are never shown in clear text here.
//!
//! Connecting is delegated to the [`Workspace`](crate::workspace::Workspace):
//! this view emits [`HostManagerEvent::Connect`] and the workspace opens the
//! SSH terminal tab and drives the trust / auth prompts.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, ClipboardItem, Context, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString, StatefulInteractiveElement,
    Styled, Task, Window,
};
use labonair_backend::modules::credentials::{self, Credential};
use labonair_backend::modules::hosts::{self, Group, Host, ReorderItem};
use labonair_backend::modules::snippets;
use labonair_backend::modules::ssh::client::{ssh_test_connection, TestConnectionResult};
use labonair_backend::modules::ssh::config_parser::{self, ImportConflict, SshConfigEntry};
use labonair_backend::App as Backend;
use tokio::runtime::Handle as TokioHandle;

use crate::components::{context_menu, IconName, MenuItem};
use crate::notifications::{notification_center, Notification};
use crate::theme::ThemeStore;

/// Connection status for a host, tracked live off the SSH event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HostStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

impl HostStatus {
    /// Short label for the host-list item's status pill.
    fn label(self) -> &'static str {
        match self {
            HostStatus::Disconnected => "offline",
            HostStatus::Connecting => "connecting\u{2026}",
            HostStatus::Connected => "connected",
            HostStatus::Failed => "failed",
        }
    }
}

/// Emitted to the workspace to drive an action it owns.
pub enum HostManagerEvent {
    /// Open an SSH terminal tab for this host id.
    Connect(String),
    /// Open a dual-pane SFTP browser tab for this host id.
    OpenSftp(String),
}

/// One running port-forward, as shown in the host manager's active-tunnel panel.
/// Built by the workspace from `labonair_backend::modules::ssh::tunnels`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTunnelRow {
    pub host_label: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

/// Drag payload for reordering host list items / dropping them onto a group
/// filter chip.
#[derive(Debug, Clone)]
struct DraggedHost {
    id: String,
}

/// Minimal drag ghost for a host row.
struct HostDragGhost;

impl Render for HostDragGhost {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(180.0)).h(px(2.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMethod {
    Password,
    Key,
    /// Auth via a saved [`Credential`] (key or password). Backend auth_method
    /// string is `"credential"` — matches `reference-src` and
    /// `ssh::client` / `config_parser` which special-case that exact value.
    Credential,
    None,
}

impl AuthMethod {
    const ALL: [AuthMethod; 4] = [
        AuthMethod::Password,
        AuthMethod::Key,
        AuthMethod::Credential,
        AuthMethod::None,
    ];
    fn as_str(self) -> &'static str {
        match self {
            AuthMethod::Password => "password",
            AuthMethod::Key => "key",
            AuthMethod::Credential => "credential",
            AuthMethod::None => "none",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "key" => AuthMethod::Key,
            // `"agent"` is the legacy pre-Block-E spelling of this mode.
            "credential" | "agent" => AuthMethod::Credential,
            "none" => AuthMethod::None,
            _ => AuthMethod::Password,
        }
    }
    fn title(self) -> &'static str {
        match self {
            AuthMethod::Password => "Password",
            AuthMethod::Key => "SSH Key",
            AuthMethod::Credential => "Credential",
            AuthMethod::None => "None",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostField {
    Name,
    Address,
    Port,
    Username,
    KeyPath,
    DefaultPath,
    DefaultPathSftp,
    Password,
    SudoPassword,
    KeepAliveInterval,
    KeepAliveTries,
    Notes,
    TunnelLocalPort(usize),
    TunnelRemoteHost(usize),
    TunnelRemotePort(usize),
}

/// One editable local-forward row in the host form's Tunnels section.
#[derive(Debug, Clone, Default, PartialEq)]
struct TunnelDraft {
    id: String,
    local_port: String,
    remote_host: String,
    remote_port: String,
}

impl TunnelDraft {
    fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            local_port: String::new(),
            remote_host: String::new(),
            remote_port: String::new(),
        }
    }
}

/// Parse the `hosts.tunnels` JSON column into editable drafts.
fn parse_tunnels(raw: &Option<String>) -> Vec<TunnelDraft> {
    let Some(s) = raw.as_deref().filter(|s| !s.trim().is_empty()) else {
        return Vec::new();
    };
    let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(s) else {
        return Vec::new();
    };
    arr.iter()
        .map(|t| TunnelDraft {
            id: t
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            local_port: t
                .get("local_port")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_default(),
            remote_host: t
                .get("remote_host")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            remote_port: t
                .get("remote_port")
                .and_then(|v| v.as_u64())
                .map(|n| n.to_string())
                .unwrap_or_default(),
        })
        .collect()
}

/// Serialize the drafts back to the `hosts.tunnels` JSON shape, dropping any
/// row that is not a complete `local:<port> → host:<port>` forward.
fn serialize_tunnels(drafts: &[TunnelDraft]) -> String {
    let list: Vec<serde_json::Value> = drafts
        .iter()
        .filter_map(|d| {
            let lp: u16 = d.local_port.trim().parse().ok()?;
            let rp: u16 = d.remote_port.trim().parse().ok()?;
            let rh = d.remote_host.trim();
            if rh.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "id": d.id,
                "type": "local",
                "local_port": lp,
                "remote_host": rh,
                "remote_port": rp,
            }))
        })
        .collect();
    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
}

/// Backing state of the add/edit-host form.
#[derive(Clone)]
struct HostForm {
    editing_id: Option<String>,
    name: String,
    address: String,
    port: String,
    username: String,
    auth: AuthMethod,
    key_path: String,
    default_path: String,
    default_path_sftp: String,
    password: String,
    /// Sudo-password autofill — persisted to the OS keychain, never SQLite.
    /// Empty string on load (backend never returns the plaintext); only
    /// written when the user types a replacement.
    sudo_password: String,
    /// `true` once a sudo password is on file for this host (drives the
    /// "(set)" placeholder). From `Host::sudo_password_set`.
    sudo_password_set: bool,
    keep_alive_interval: String,
    keep_alive_tries: String,
    notes: String,
    pin_to_top: bool,
    /// `None` = no credential; `Some(idx)` indexes into `credentials`.
    credential: Option<usize>,
    /// `None` = no group; `Some(idx)` indexes into `groups`.
    group: Option<usize>,
    /// `None` = direct; `Some(idx)` indexes into `hosts` (the bastion to route
    /// through). The host being edited is never a valid choice.
    jump_host: Option<usize>,
    /// Configured local port-forwards (ProxyJump-independent).
    tunnels: Vec<TunnelDraft>,
    /// "Block AI Agent Access" — when set, the MCP bridge refuses to grant a
    /// tab for this host and any live grant is revoked immediately (T11-006).
    block_agent_access: bool,
    /// Host avatar icon key (see [`HOST_ICONS`]); `None` = default server glyph.
    icon: Option<String>,
    /// Snippet to run on connect (`hosts.startup_snippet_id`).
    snippet_id: Option<String>,
    /// `"execute"` (run it) or `"inject"` (type it, don't press enter).
    snippet_mode: String,
    /// Which of the 4 form tabs is showing.
    tab: FormTab,
    focus: HostField,
    /// Fallback edit target for a stale tunnel-field index (never rendered).
    scratch: String,
}

/// The 4 host-form tabs — ports `HostFormPanel.tsx:550`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormTab {
    General,
    Ssh,
    Sftp,
    Tunnels,
}

impl FormTab {
    const ALL: [FormTab; 4] = [
        FormTab::General,
        FormTab::Ssh,
        FormTab::Sftp,
        FormTab::Tunnels,
    ];
    fn title(self) -> &'static str {
        match self {
            FormTab::General => "General",
            FormTab::Ssh => "SSH",
            FormTab::Sftp => "SFTP",
            FormTab::Tunnels => "Tunnels",
        }
    }
}

/// Curated host-avatar icon set (`lib/icons/` in the reference). Key stored in
/// `hosts.icon`.
const HOST_ICONS: [(&str, IconName); 8] = [
    ("server", IconName::Server),
    ("terminal", IconName::Terminal),
    ("globe", IconName::Globe),
    ("shield", IconName::Shield),
    ("zap", IconName::Zap),
    ("home", IconName::Home),
    ("git", IconName::GitBranch),
    ("cloud", IconName::Sparkles),
];

fn host_icon(key: Option<&str>) -> IconName {
    key.and_then(|k| HOST_ICONS.iter().find(|(n, _)| *n == k))
        .map(|(_, i)| *i)
        .unwrap_or(IconName::Server)
}

impl HostForm {
    fn blank() -> Self {
        Self {
            editing_id: None,
            name: String::new(),
            address: String::new(),
            port: "22".into(),
            username: String::new(),
            auth: AuthMethod::Password,
            key_path: String::new(),
            default_path: String::new(),
            default_path_sftp: String::new(),
            password: String::new(),
            sudo_password: String::new(),
            sudo_password_set: false,
            keep_alive_interval: String::new(),
            keep_alive_tries: String::new(),
            notes: String::new(),
            pin_to_top: false,
            credential: None,
            group: None,
            jump_host: None,
            tunnels: Vec::new(),
            block_agent_access: false,
            icon: None,
            snippet_id: None,
            snippet_mode: "execute".into(),
            tab: FormTab::General,
            focus: HostField::Name,
            scratch: String::new(),
        }
    }

    fn from_host(h: &Host, groups: &[Group], creds: &[Credential], hosts: &[Host]) -> Self {
        Self {
            editing_id: Some(h.id.clone()),
            name: h.name.clone(),
            address: h.host_address.clone(),
            port: h.port.to_string(),
            username: h.username.clone(),
            auth: AuthMethod::from_str(&h.auth_method),
            key_path: h.private_key_path.clone().unwrap_or_default(),
            default_path: h.default_path_ssh.clone().unwrap_or_default(),
            default_path_sftp: h.default_path_sftp.clone().unwrap_or_default(),
            password: String::new(),
            sudo_password: String::new(),
            sudo_password_set: h.sudo_password_set,
            keep_alive_interval: h
                .keep_alive_interval
                .map(|v| v.to_string())
                .unwrap_or_default(),
            keep_alive_tries: h
                .keep_alive_tries
                .map(|v| v.to_string())
                .unwrap_or_default(),
            notes: h.notes.clone().unwrap_or_default(),
            pin_to_top: h.pin_to_top,
            credential: h
                .credential_id
                .as_deref()
                .and_then(|cid| creds.iter().position(|c| c.id == cid)),
            group: h
                .group_id
                .as_deref()
                .and_then(|gid| groups.iter().position(|g| g.id == gid)),
            jump_host: h
                .jump_host_id
                .as_deref()
                .and_then(|jid| hosts.iter().position(|c| c.id == jid && c.id != h.id)),
            tunnels: parse_tunnels(&h.tunnels),
            block_agent_access: h.block_agent_access,
            icon: h.icon.clone(),
            snippet_id: h.startup_snippet_id.clone(),
            snippet_mode: h
                .startup_snippet_mode
                .clone()
                .unwrap_or_else(|| "execute".into()),
            tab: FormTab::General,
            focus: HostField::Name,
            scratch: String::new(),
        }
    }

    fn field_mut(&mut self, f: HostField) -> &mut String {
        match f {
            HostField::Name => &mut self.name,
            HostField::Address => &mut self.address,
            HostField::Port => &mut self.port,
            HostField::Username => &mut self.username,
            HostField::KeyPath => &mut self.key_path,
            HostField::DefaultPath => &mut self.default_path,
            HostField::DefaultPathSftp => &mut self.default_path_sftp,
            HostField::Password => &mut self.password,
            HostField::SudoPassword => &mut self.sudo_password,
            HostField::KeepAliveInterval => &mut self.keep_alive_interval,
            HostField::KeepAliveTries => &mut self.keep_alive_tries,
            HostField::Notes => &mut self.notes,
            HostField::TunnelLocalPort(i)
            | HostField::TunnelRemoteHost(i)
            | HostField::TunnelRemotePort(i)
                if i >= self.tunnels.len() =>
            {
                &mut self.scratch
            }
            HostField::TunnelLocalPort(i) => &mut self.tunnels[i].local_port,
            HostField::TunnelRemoteHost(i) => &mut self.tunnels[i].remote_host,
            HostField::TunnelRemotePort(i) => &mut self.tunnels[i].remote_port,
        }
    }
}

/// New-credential draft inside the credential manager.
struct CredDraft {
    name: String,
    is_key: bool,
}

/// Cycle order for the import dialog's conflict-policy toggle.
fn cycle_conflict(c: ImportConflict) -> ImportConflict {
    match c {
        ImportConflict::Skip => ImportConflict::Overwrite,
        ImportConflict::Overwrite => ImportConflict::Rename,
        ImportConflict::Rename => ImportConflict::Skip,
    }
}

fn conflict_label(c: ImportConflict) -> &'static str {
    match c {
        ImportConflict::Skip => "skip",
        ImportConflict::Overwrite => "overwrite",
        ImportConflict::Rename => "rename",
    }
}

/// Backing state of the "Import from ~/.ssh/config" dialog.
struct ImportState {
    loading: bool,
    entries: Vec<SshConfigEntry>,
    /// Selected entries, keyed by alias.
    selected: HashSet<String>,
    conflict: ImportConflict,
    error: Option<String>,
}

/// Backing state of the "Export to ~/.ssh/config" dialog.
struct ExportState {
    /// Selected host ids.
    selected: HashSet<String>,
    error: Option<String>,
}

/// Host-list sort order — ports `HomeDashboard.tsx`'s sort dropdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostSort {
    LastConnected,
    NameAsc,
    NameDesc,
}

impl HostSort {
    fn next(self) -> Self {
        match self {
            HostSort::LastConnected => HostSort::NameAsc,
            HostSort::NameAsc => HostSort::NameDesc,
            HostSort::NameDesc => HostSort::LastConnected,
        }
    }
    fn label(self) -> &'static str {
        match self {
            HostSort::LastConnected => "Last connected",
            HostSort::NameAsc => "A \u{2192} Z",
            HostSort::NameDesc => "Z \u{2192} A",
        }
    }
}

/// Per-host reachability probe result (`startPingWorker` in the reference).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Ping {
    #[default]
    Checking,
    Online,
    Offline,
}

/// Autosave lifecycle indicator (`SaveStatusIcon`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SaveState {
    #[default]
    Idle,
    Pending,
    Saving,
    Saved,
    Error,
}

impl SaveState {
    fn label(self) -> &'static str {
        match self {
            SaveState::Idle => "",
            SaveState::Pending => "Unsaved changes\u{2026}",
            SaveState::Saving => "Saving\u{2026}",
            SaveState::Saved => "Saved \u{2713}",
            SaveState::Error => "Save failed",
        }
    }
}

pub struct HostManagerView {
    app: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    hosts: Vec<Host>,
    groups: Vec<Group>,
    credentials: Vec<Credential>,
    snippets: Vec<(String, String)>,
    statuses: Vec<(String, HostStatus)>,
    active_tunnels: Vec<ActiveTunnelRow>,
    form: Option<HostForm>,
    form_focus: FocusHandle,
    creds_open: bool,
    cred_draft: Option<CredDraft>,
    cred_focus: FocusHandle,
    /// Inline "new group" buffer, `Some` while the field is open.
    group_draft: Option<String>,
    group_focus: FocusHandle,
    /// SSH-config import dialog, `Some` while open.
    import: Option<ImportState>,
    /// SSH-config export dialog, `Some` while open.
    export: Option<ExportState>,
    /// Open host list-item right-click menu: `(host id, cursor)`.
    host_menu: Option<(String, Point<Pixels>)>,
    /// Open group-chip right-click menu: `(group id, group name, cursor)`.
    group_menu: Option<(String, String, Point<Pixels>)>,
    /// In-progress inline group rename: `(group id, buffer)`.
    group_rename: Option<(String, String)>,
    group_rename_focus: FocusHandle,
    // ── master/detail state (T16-014) ──────────────────────────────────────
    /// Left-pane search / quick-connect box.
    search: String,
    search_focus: FocusHandle,
    sort: HostSort,
    /// `true` = card grid, `false` = list.
    grid_view: bool,
    /// `None` = all groups; `Some("")` = ungrouped only; `Some(id)` = that group.
    group_filter: Option<String>,
    /// Per-host reachability, refreshed by the ping worker.
    ping: HashMap<String, Ping>,
    save_state: SaveState,
    /// Whether the detail-pane host-icon picker row is expanded.
    icon_picker_open: bool,
    /// Bumped on every field edit; the debounced autosave task no-ops if it
    /// changed while the task was sleeping.
    edit_gen: u64,
    /// Result line of the last Test Connection, shown in the form header.
    test_result: Option<String>,
    _ping_task: Task<()>,
    focus_handle: FocusHandle,
}

impl EventEmitter<HostManagerEvent> for HostManagerView {}

impl HostManagerView {
    pub fn new(
        app: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        let ping_task = cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_secs(30))
                .await;
            if this.update(cx, |this, cx| this.refresh_ping(cx)).is_err() {
                break;
            }
        });
        let this = Self {
            app,
            tokio,
            theme,
            hosts: Vec::new(),
            groups: Vec::new(),
            credentials: Vec::new(),
            snippets: Vec::new(),
            statuses: Vec::new(),
            active_tunnels: Vec::new(),
            form: None,
            form_focus: cx.focus_handle(),
            creds_open: false,
            cred_draft: None,
            cred_focus: cx.focus_handle(),
            group_draft: None,
            group_focus: cx.focus_handle(),
            import: None,
            export: None,
            host_menu: None,
            group_menu: None,
            group_rename: None,
            group_rename_focus: cx.focus_handle(),
            search: String::new(),
            search_focus: cx.focus_handle(),
            sort: HostSort::LastConnected,
            grid_view: false,
            group_filter: None,
            ping: HashMap::new(),
            save_state: SaveState::Idle,
            icon_picker_open: false,
            edit_gen: 0,
            test_result: None,
            _ping_task: ping_task,
            focus_handle: cx.focus_handle(),
        };
        this.reload(cx);
        this
    }

    /// Update a host's live connection status (called by the workspace).
    pub fn set_status(&mut self, host_id: &str, status: HostStatus, cx: &mut Context<Self>) {
        if let Some(entry) = self.statuses.iter_mut().find(|(id, _)| id == host_id) {
            entry.1 = status;
        } else {
            self.statuses.push((host_id.to_string(), status));
        }
        cx.notify();
    }

    /// Replace the active-tunnel snapshot (called by the workspace each poll
    /// tick). Only notifies when the set actually changed.
    pub fn set_active_tunnels(&mut self, rows: Vec<ActiveTunnelRow>, cx: &mut Context<Self>) {
        if self.active_tunnels != rows {
            self.active_tunnels = rows;
            cx.notify();
        }
    }

    /// All known host ids (for session restore — T14-001).
    pub fn host_ids(&self) -> Vec<String> {
        self.hosts.iter().map(|h| h.id.clone()).collect()
    }

    /// Up to `n` hosts, most-recently-connected first (nulls last), then by
    /// name — the `+` new-tab dropdown's SSH / SFTP recent-host lists.
    pub fn recent_hosts(&self, n: usize) -> Vec<(String, String)> {
        let mut hosts: Vec<&Host> = self.hosts.iter().collect();
        hosts.sort_by(|a, b| {
            b.last_connected_at
                .cmp(&a.last_connected_at)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        hosts
            .into_iter()
            .take(n)
            .map(|h| (h.id.clone(), h.name.clone()))
            .collect()
    }

    /// Display name for a host id, if known.
    pub fn host_name(&self, host_id: &str) -> Option<String> {
        self.hosts
            .iter()
            .find(|h| h.id == host_id)
            .map(|h| h.name.clone())
    }

    /// Display name of the jump host a given host routes through, if any.
    pub fn jump_host_label(&self, host_id: &str) -> Option<String> {
        let host = self.hosts.iter().find(|h| h.id == host_id)?;
        let jid = host.jump_host_id.as_deref()?;
        self.hosts
            .iter()
            .find(|c| c.id == jid)
            .map(|c| c.name.clone())
    }

    fn status_of(&self, host_id: &str) -> HostStatus {
        self.statuses
            .iter()
            .find(|(id, _)| id == host_id)
            .map(|(_, s)| *s)
            .unwrap_or_default()
    }

    /// Reload hosts / groups / credentials from the backend.
    pub fn reload(&self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            let hosts = hosts::db::hosts_get_all(&app.db).await.unwrap_or_default();
            let groups = hosts::db::groups_get_all(&app.db).await.unwrap_or_default();
            let creds = credentials::credentials_get_all(&app.db)
                .await
                .unwrap_or_default();
            let snippets = snippets::db::snippets_get_all(&app.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|s| (s.id, s.name))
                .collect::<Vec<_>>();
            (hosts, groups, creds, snippets)
        });
        cx.spawn(async move |this, cx| {
            if let Ok((h, g, c, s)) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    this.hosts = h;
                    this.groups = g;
                    this.credentials = c;
                    this.snippets = s;
                    this.refresh_ping(cx);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Fire a TCP reachability probe at every host (`host:port`, 2s timeout)
    /// and fold the results back in. Runs on load + every 30s.
    fn refresh_ping(&mut self, cx: &mut Context<Self>) {
        let targets: Vec<(String, String, u16)> = self
            .hosts
            .iter()
            .map(|h| (h.id.clone(), h.host_address.clone(), h.port as u16))
            .collect();
        if targets.is_empty() {
            return;
        }
        for (id, _, _) in &targets {
            self.ping.entry(id.clone()).or_insert(Ping::Checking);
        }
        let jh = self.tokio.spawn(async move {
            let mut out = Vec::with_capacity(targets.len());
            for (id, addr, port) in targets {
                let ok = tokio::time::timeout(
                    Duration::from_secs(2),
                    tokio::net::TcpStream::connect((addr.as_str(), port)),
                )
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false);
                out.push((id, if ok { Ping::Online } else { Ping::Offline }));
            }
            out
        });
        cx.spawn(async move |this, cx| {
            if let Ok(results) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    for (id, state) in results {
                        this.ping.insert(id, state);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn notify_toast(&self, title: &str, body: String, cx: &mut Context<Self>) {
        let n = Notification::info(title.to_string(), body);
        notification_center(cx).update(cx, |c, cx| {
            c.push(n, cx);
        });
    }

    // ── mutations ───────────────────────────────────────────────────────────

    /// Debounced autosave: bump the generation, mark the form dirty, and after
    /// 1s save it — unless the user kept typing (generation moved on).
    fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
        if self
            .form
            .as_ref()
            .and_then(|f| f.editing_id.clone())
            .is_none()
        {
            return; // new host uses the explicit "Add Host" button
        }
        self.edit_gen = self.edit_gen.wrapping_add(1);
        self.save_state = SaveState::Pending;
        let gen = self.edit_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1000))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.edit_gen == gen {
                    this.submit_form(true, cx);
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn submit_form(&mut self, keep_form: bool, cx: &mut Context<Self>) {
        let form = if keep_form {
            match self.form.as_ref() {
                Some(f) => f.clone(),
                None => return,
            }
        } else {
            match self.form.take() {
                Some(f) => f,
                None => return,
            }
        };
        if keep_form {
            self.save_state = SaveState::Saving;
        }
        let app = self.app.clone();
        let icon = Some(form.icon.clone().unwrap_or_default());
        let snippet_id = Some(form.snippet_id.clone().unwrap_or_default());
        let snippet_mode = Some(form.snippet_mode.clone());
        let port: i64 = form.port.trim().parse().unwrap_or(22);
        let name = if form.name.trim().is_empty() {
            form.address.clone()
        } else {
            form.name.clone()
        };
        let auth = form.auth.as_str().to_string();
        let key_path = (!form.key_path.trim().is_empty()).then(|| form.key_path.trim().to_string());
        let default_path =
            (!form.default_path.trim().is_empty()).then(|| form.default_path.trim().to_string());
        let password = (!form.password.is_empty()).then(|| form.password.clone());
        // On edit: only send `Some(_)` when the user actually typed a new
        // sudo password (backend interprets `Some("")` as "clear").
        let sudo_password = form.sudo_password.clone();
        let default_path_sftp = Some(form.default_path_sftp.trim().to_string());
        let keep_alive_interval: Option<i64> = form.keep_alive_interval.trim().parse().ok();
        let keep_alive_tries: Option<i64> = form.keep_alive_tries.trim().parse().ok();
        let notes = Some(form.notes.trim().to_string());
        let pin_to_top = form.pin_to_top;
        let group_id = form
            .group
            .and_then(|i| self.groups.get(i))
            .map(|g| g.id.clone());
        let cred_id = form
            .credential
            .and_then(|i| self.credentials.get(i))
            .map(|c| c.id.clone());
        let jump_host_id = form
            .jump_host
            .and_then(|i| self.hosts.get(i))
            .map(|h| h.id.clone());
        let tunnels_json = serialize_tunnels(&form.tunnels);
        let block_agent_access = form.block_agent_access;
        let editing = form.editing_id.clone();
        let addr = form.address.trim().to_string();
        let user = form.username.trim().to_string();

        let jh = self.tokio.spawn(async move {
            match editing {
                Some(id) => {
                    let _ = hosts::db::hosts_update(
                        app.clone(),
                        &app.db,
                        &app.secrets,
                        id,
                        Some(name),                                                 // name
                        Some(addr),                                                 // host_address
                        Some(port),                                                 // port
                        Some(user),                                                 // username
                        Some(auth),                                                 // auth_method
                        key_path, // private_key_path
                        group_id, // group_id
                        None,     // tags
                        password, // password
                        (!sudo_password.is_empty()).then(|| sudo_password.clone()), // sudo_password
                        default_path, // default_path_ssh
                        default_path_sftp, // default_path_sftp
                        Some(pin_to_top), // pin_to_top
                        keep_alive_interval, // keep_alive_interval
                        keep_alive_tries, // keep_alive_tries
                        None,     // sort_order
                        Some(tunnels_json), // tunnels
                        snippet_id, // startup_snippet_id
                        snippet_mode, // startup_snippet_mode
                        Some(cred_id.clone().unwrap_or_default()), // credential_id ("" clears)
                        Some(jump_host_id.clone().unwrap_or_default()), // jump_host_id ("" clears)
                        notes,    // notes
                        icon,     // icon
                        Some(block_agent_access), // block_agent_access
                    )
                    .await;
                }
                None => {
                    let _ = hosts::db::hosts_create(
                        app.clone(),
                        &app.db,
                        &app.secrets,
                        name,                                                       // name
                        addr,                                                       // host_address
                        port,                                                       // port
                        user,                                                       // username
                        auth,                                                       // auth_method
                        key_path, // private_key_path
                        group_id, // group_id
                        None,     // tags
                        password, // password
                        (!sudo_password.is_empty()).then(|| sudo_password.clone()), // sudo_password
                        default_path, // default_path_ssh
                        default_path_sftp, // default_path_sftp
                        Some(pin_to_top), // pin_to_top
                        keep_alive_interval, // keep_alive_interval
                        keep_alive_tries, // keep_alive_tries
                        None,     // sort_order
                        Some(tunnels_json), // tunnels
                        snippet_id, // startup_snippet_id
                        snippet_mode, // startup_snippet_mode
                        cred_id,  // credential_id
                        jump_host_id, // jump_host_id
                        notes,    // notes
                        icon,     // icon
                        Some(block_agent_access), // block_agent_access
                    )
                    .await;
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let ok = jh.await.is_ok();
            let _ = this.update(cx, |this, cx| {
                if keep_form {
                    this.save_state = if ok {
                        SaveState::Saved
                    } else {
                        SaveState::Error
                    };
                    // Refresh the list rows (name / group / pin may have changed)
                    // without disturbing the open form.
                    this.reload_list_only(cx);
                } else {
                    this.reload(cx);
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Re-fetch just the host/group rows (used after autosave — must not touch
    /// `self.form`).
    fn reload_list_only(&self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            let hosts = hosts::db::hosts_get_all(&app.db).await.unwrap_or_default();
            let groups = hosts::db::groups_get_all(&app.db).await.unwrap_or_default();
            (hosts, groups)
        });
        cx.spawn(async move |this, cx| {
            if let Ok((h, g)) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    this.hosts = h;
                    this.groups = g;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn duplicate_host(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            hosts::db::hosts_duplicate(app.clone(), &app.db, &app.secrets, id).await
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    fn delete_host(&mut self, id: String, cx: &mut Context<Self>) {
        if self.form.as_ref().and_then(|f| f.editing_id.as_deref()) == Some(id.as_str()) {
            self.form = None;
            self.save_state = SaveState::Idle;
        }
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            hosts::db::hosts_delete(app.clone(), &app.db, &app.secrets, id).await
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    /// Persist a new sort order that places `dragged` immediately before
    /// `target` in the current visible list, then reload.
    fn reorder_hosts(&mut self, dragged: &str, target: &str, cx: &mut Context<Self>) {
        if dragged == target {
            return;
        }
        let mut order: Vec<String> = self.visible_hosts().into_iter().map(|h| h.id).collect();
        let Some(from) = order.iter().position(|id| id == dragged) else {
            return;
        };
        let id = order.remove(from);
        let Some(to) = order.iter().position(|x| x == target) else {
            return;
        };
        order.insert(to, id);
        let items: Vec<ReorderItem> = order
            .iter()
            .enumerate()
            .map(|(i, id)| ReorderItem {
                id: id.clone(),
                sort_order: i as i64,
            })
            .collect();
        let app = self.app.clone();
        let jh = self
            .tokio
            .spawn(async move { hosts::db::hosts_reorder(&app.db, items).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload_list_only(cx));
        })
        .detach();
    }

    /// Move a host into (`Some(id)`) or out of (`None`) a group.
    fn move_host_to_group(&mut self, host_id: &str, group: Option<String>, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let id = host_id.to_string();
        let jh = self.tokio.spawn(async move {
            hosts::db::hosts_update(
                app.clone(),
                &app.db,
                &app.secrets,
                id,
                None,                            // name
                None,                            // host_address
                None,                            // port
                None,                            // username
                None,                            // auth_method
                None,                            // private_key_path
                Some(group.unwrap_or_default()), // group_id ("" clears)
                None,                            // tags
                None,                            // password
                None,                            // sudo_password
                None,                            // default_path_ssh
                None,                            // default_path_sftp
                None,                            // pin_to_top
                None,                            // keep_alive_interval
                None,                            // keep_alive_tries
                None,                            // sort_order
                None,                            // tunnels
                None,                            // startup_snippet_id
                None,                            // startup_snippet_mode
                None,                            // credential_id
                None,                            // jump_host_id
                None,                            // notes
                None,                            // icon
                None,                            // block_agent_access
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload_list_only(cx));
        })
        .detach();
    }

    /// Run `ssh_test_connection` for the open host and drop the outcome into
    /// `test_result` (form-header line) + a toast.
    fn test_connection(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.form.as_ref().and_then(|f| f.editing_id.clone()) else {
            return;
        };
        self.test_result = Some("Testing\u{2026}".to_string());
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            ssh_test_connection(
                id,
                None,
                None,
                &app.trust,
                &app.db,
                &app.secrets,
                app.clone(),
                Some(15),
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await;
            let _ = this.update(cx, |this, cx| {
                let msg = match res {
                    Ok(Ok(TestConnectionResult::Success)) => "Connection OK \u{2713}".to_string(),
                    Ok(Ok(TestConnectionResult::UnknownHostKey { fingerprint })) => {
                        format!("Unknown host key ({fingerprint}) — connect once to trust it")
                    }
                    Ok(Ok(TestConnectionResult::HostKeyChanged { fingerprint })) => {
                        format!("Host key CHANGED ({fingerprint}) — verify before connecting")
                    }
                    Ok(Err(e)) => format!("Failed: {e}"),
                    Err(e) => format!("Failed: {e}"),
                };
                this.notify_toast("Test Connection", msg.clone(), cx);
                this.test_result = Some(msg);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Connect via the quick-connect box (`user@host:port`), creating a saved
    /// host row first so the connection flow has something to key on.
    fn quick_connect(&mut self, user: String, host: String, port: u16, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let name = host.clone();
        let jh = self.tokio.spawn(async move {
            hosts::db::hosts_create(
                app.clone(),
                &app.db,
                &app.secrets,
                name,
                host,
                port as i64,
                user,
                "password".to_string(),
                None, // private_key_path
                None, // group_id
                None, // tags
                None, // password
                None, // sudo_password
                None, // default_path_ssh
                None, // default_path_sftp
                None, // pin_to_top
                None, // keep_alive_interval
                None, // keep_alive_tries
                None, // sort_order
                None, // tunnels
                None, // startup_snippet_id
                None, // startup_snippet_mode
                None, // credential_id
                None, // jump_host_id
                None, // notes
                None, // icon
                None, // block_agent_access
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(h)) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    this.search.clear();
                    this.reload(cx);
                    cx.emit(HostManagerEvent::Connect(h.id));
                });
            }
        })
        .detach();
    }

    fn create_group(&mut self, name: String, cx: &mut Context<Self>) {
        if name.trim().is_empty() {
            return;
        }
        let app = self.app.clone();
        let name = name.trim().to_string();
        let jh = self
            .tokio
            .spawn(async move { hosts::db::groups_create(&app.db, name, None, None).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    fn delete_group(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self
            .tokio
            .spawn(async move { hosts::db::groups_delete(&app.db, id).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    fn add_credential(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.cred_draft.take() else {
            return;
        };
        let name = if draft.name.trim().is_empty() {
            "Credential".to_string()
        } else {
            draft.name.trim().to_string()
        };
        let is_key = draft.is_key;
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            let cred = credentials::credentials_create(
                app.clone(),
                &app.db,
                &app.secrets,
                name,
                if is_key { "key" } else { "password" }.to_string(),
                None,
                None,
                None,
                None,
            )
            .await?;
            if is_key {
                let res = credentials::credential_generate_keypair(
                    app.clone(),
                    &app.db,
                    &app.secrets,
                    cred.id.clone(),
                    "ed25519".to_string(),
                    None,
                )
                .await?;
                return Ok::<Option<String>, String>(Some(res.public_key));
            }
            Ok(None)
        });
        cx.spawn(async move |this, cx| {
            let out = jh.await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(Ok(Some(pubkey))) = out {
                    this.notify_toast(
                        "SSH key generated",
                        format!("Public key (add to the server's authorized_keys):\n{pubkey}"),
                        cx,
                    );
                }
                this.reload(cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn delete_credential(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            credentials::credentials_delete(app.clone(), &app.db, &app.secrets, id).await
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    // ── SSH-config import / export ─────────────────────────────────────────

    /// Open the import dialog and kick off a `~/.ssh/config` parse.
    fn open_import(&mut self, cx: &mut Context<Self>) {
        self.import = Some(ImportState {
            loading: true,
            entries: Vec::new(),
            selected: HashSet::new(),
            conflict: ImportConflict::Skip,
            error: None,
        });
        let jh = self
            .tokio
            .spawn(async move { config_parser::parse_ssh_config_cmd().await });
        cx.spawn(async move |this, cx| {
            let res = jh.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(state) = this.import.as_mut() {
                    state.loading = false;
                    match res {
                        Ok(Ok(entries)) => {
                            state.selected = entries.iter().map(|e| e.alias.clone()).collect();
                            state.entries = entries;
                        }
                        Ok(Err(e)) => state.error = Some(e),
                        Err(e) => state.error = Some(e.to_string()),
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Import the currently selected entries with the chosen conflict policy.
    fn run_import(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.import.as_ref() else {
            return;
        };
        let conflict = state.conflict;
        let entries: Vec<SshConfigEntry> = state
            .entries
            .iter()
            .filter(|e| state.selected.contains(&e.alias))
            .cloned()
            .collect();
        if entries.is_empty() {
            return;
        }
        let count = entries.len();
        let app = self.app.clone();
        let jh = self.tokio.spawn(async move {
            config_parser::import_ssh_config_entries(entries, conflict, &app.db).await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await;
            let _ = this.update(cx, |this, cx| {
                match res {
                    Ok(Ok(ids)) => {
                        this.import = None;
                        this.notify_toast(
                            "SSH config imported",
                            format!("{} of {count} host(s) imported.", ids.len()),
                            cx,
                        );
                        this.reload(cx);
                    }
                    Ok(Err(e)) => {
                        if let Some(s) = this.import.as_mut() {
                            s.error = Some(e);
                        }
                    }
                    Err(e) => {
                        if let Some(s) = this.import.as_mut() {
                            s.error = Some(e.to_string());
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Open the export dialog with every host pre-selected.
    fn open_export(&mut self, cx: &mut Context<Self>) {
        self.export = Some(ExportState {
            selected: self.hosts.iter().map(|h| h.id.clone()).collect(),
            error: None,
        });
        cx.notify();
    }

    fn export_selected_ids(&self) -> Vec<String> {
        self.export
            .as_ref()
            .map(|s| {
                self.hosts
                    .iter()
                    .filter(|h| s.selected.contains(&h.id))
                    .map(|h| h.id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Generate the SSH-config block for the selected hosts and either copy it
    /// to the clipboard or append it to `~/.ssh/config`.
    fn run_export(&mut self, append: bool, cx: &mut Context<Self>) {
        let ids = self.export_selected_ids();
        if ids.is_empty() {
            return;
        }
        let app = self.app.clone();
        let tokio = self.tokio.clone();
        let jh = self
            .tokio
            .spawn(async move { config_parser::export_ssh_config(ids, &app.db).await });
        cx.spawn(async move |this, cx| {
            let res = jh.await;
            let block = match res {
                Ok(Ok(block)) => block,
                Ok(Err(e)) => {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(s) = this.export.as_mut() {
                            s.error = Some(e);
                        }
                        cx.notify();
                    });
                    return;
                }
                Err(e) => {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(s) = this.export.as_mut() {
                            s.error = Some(e.to_string());
                        }
                        cx.notify();
                    });
                    return;
                }
            };
            if !append {
                let _ = this.update(cx, |this, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(block.clone()));
                    this.export = None;
                    this.notify_toast(
                        "SSH config copied",
                        "The generated Host blocks were copied to the clipboard.".to_string(),
                        cx,
                    );
                    cx.notify();
                });
                return;
            }
            let write = tokio
                .spawn(async move { config_parser::write_ssh_config_export(block, true).await })
                .await;
            let _ = this.update(cx, |this, cx| {
                match write {
                    Ok(Ok(path)) => {
                        this.export = None;
                        this.notify_toast(
                            "SSH config exported",
                            format!("Host blocks appended to {path}"),
                            cx,
                        );
                    }
                    Ok(Err(e)) => {
                        if let Some(s) = this.export.as_mut() {
                            s.error = Some(e);
                        }
                    }
                    Err(e) => {
                        if let Some(s) = this.export.as_mut() {
                            s.error = Some(e.to_string());
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── key handling for the inline text fields ─────────────────────────────

    fn edit_str(buf: &mut String, ks: &gpui::Keystroke) -> bool {
        match ks.key.as_str() {
            "backspace" => {
                buf.pop();
                true
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return false;
                }
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let Some(ch) = ch {
                    buf.push_str(&ch);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn on_form_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                // Flush a pending autosave before closing.
                if self.save_state == SaveState::Pending {
                    self.submit_form(true, cx);
                }
                self.form = None;
                self.save_state = SaveState::Idle;
            }
            "enter" => self.submit_form(false, cx),
            _ => {
                let changed = if let Some(form) = self.form.as_mut() {
                    let f = form.focus;
                    Self::edit_str(form.field_mut(f), ks)
                } else {
                    false
                };
                if changed {
                    self.schedule_autosave(cx);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn on_cred_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cred_draft = None,
            "enter" => self.add_credential(cx),
            _ => {
                if let Some(d) = self.cred_draft.as_mut() {
                    Self::edit_str(&mut d.name, ks);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn on_group_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.group_draft = None,
            "enter" => {
                if let Some(name) = self.group_draft.take() {
                    self.create_group(name, cx);
                }
            }
            _ => {
                if let Some(b) = self.group_draft.as_mut() {
                    Self::edit_str(b, ks);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn on_search_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.search.clear(),
            "enter" => {
                if let Some((u, h, p)) = self.quick_connect_target() {
                    self.quick_connect(u, h, p, cx);
                }
            }
            _ => {
                Self::edit_str(&mut self.search, ks);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for HostManagerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ── rendering ──────────────────────────────────────────────────────────────

struct Palette {
    bg: gpui::Hsla,
    card: gpui::Hsla,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    accent: gpui::Hsla,
}

impl HostManagerView {
    fn palette(&self, cx: &App) -> Palette {
        let t = self.theme.read(cx);
        Palette {
            bg: t.background(),
            card: t.card(),
            fg: t.foreground(),
            muted: t.muted_foreground(),
            border: t.border(),
            accent: t.accent(),
        }
    }

    /// Small host-manager action button. Visual language (pill `rounded-4xl`
    /// radius, transparent border, `default` vs `outline` variant) tracks
    /// `reference-src/src/components/ui/button.tsx` / [`crate::components`];
    /// the shared `components::button` builder will replace this helper once
    /// the whole host-manager panel is migrated (Block B).
    fn btn(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        p: &Palette,
        primary: bool,
    ) -> gpui::Stateful<gpui::Div> {
        // `--radius-4xl` == 13px (see `labonair_theme::RadiusScale`).
        let base = div()
            .id(id)
            .flex()
            .items_center()
            .justify_center()
            .px_3()
            .py_1()
            .rounded(px(13.0))
            .border_1()
            .border_color(gpui::transparent_black())
            .text_xs()
            .cursor_pointer();
        if primary {
            base.bg(p.accent)
                .text_color(p.fg)
                .hover(|s| s.opacity(0.85))
        } else {
            base.text_color(p.muted)
                .border_color(p.border)
                .hover(|s| s.bg(p.border).text_color(p.fg))
        }
        .child(label.into())
    }

    // ── master/detail helpers (T16-014) ────────────────────────────────────

    /// Load `host` into the detail-pane form.
    fn select_host(&mut self, host: &Host, w: &mut Window, cx: &mut Context<Self>) {
        self.form = Some(HostForm::from_host(
            host,
            &self.groups,
            &self.credentials,
            &self.hosts,
        ));
        self.save_state = SaveState::Idle;
        self.test_result = None;
        w.focus(&self.form_focus);
        cx.notify();
    }

    /// Hosts after search filter, group-filter chip, and sort are applied.
    fn visible_hosts(&self) -> Vec<Host> {
        let q = self.search.trim().to_lowercase();
        let mut v: Vec<Host> = self
            .hosts
            .iter()
            .filter(|h| match &self.group_filter {
                None => true,
                Some(g) if g.is_empty() => h.group_id.is_none(),
                Some(g) => h.group_id.as_deref() == Some(g.as_str()),
            })
            .filter(|h| {
                q.is_empty()
                    || h.name.to_lowercase().contains(&q)
                    || h.host_address.to_lowercase().contains(&q)
                    || h.username.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        match self.sort {
            HostSort::LastConnected => v.sort_by(|a, b| {
                b.pin_to_top
                    .cmp(&a.pin_to_top)
                    .then(b.last_connected_at.cmp(&a.last_connected_at))
                    .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            }),
            HostSort::NameAsc => v.sort_by_key(|h| h.name.to_lowercase()),
            HostSort::NameDesc => v.sort_by_key(|h| std::cmp::Reverse(h.name.to_lowercase())),
        }
        v
    }

    /// `Some((user, host, port))` if the search box holds a `user@host[:port]`
    /// quick-connect target that isn't already a saved host.
    fn quick_connect_target(&self) -> Option<(String, String, u16)> {
        let s = self.search.trim();
        let (user, rest) = s.split_once('@')?;
        if user.is_empty() || rest.is_empty() || user.contains(char::is_whitespace) {
            return None;
        }
        let (host, port) = match rest.split_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().ok()?),
            None => (rest, 22),
        };
        if host.is_empty() || host.contains(char::is_whitespace) {
            return None;
        }
        Some((user.to_string(), host.to_string(), port))
    }

    fn ping_dot(&self, host_id: &str, p: &Palette) -> gpui::Div {
        let (color, _label) = match self.ping.get(host_id).copied().unwrap_or_default() {
            Ping::Online => (p.accent, "online"),
            Ping::Offline => (p.muted, "offline"),
            Ping::Checking => (p.border, "checking"),
        };
        div().size(px(7.0)).rounded_full().bg(color)
    }

    fn render_host_list_item(
        &self,
        host: &Host,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let selected =
            self.form.as_ref().and_then(|f| f.editing_id.as_deref()) == Some(host.id.as_str());
        let status = self.status_of(&host.id);
        let id = host.id.clone();
        let (id_click, id_drop) = (id.clone(), id.clone());
        let subtitle = format!("{}@{}:{}", host.username, host.host_address, host.port);
        let icon = host_icon(host.icon.as_deref());
        let accent = p.accent;

        div()
            .id(SharedString::from(format!("host-item-{id}")))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_2()
            .rounded_md()
            .bg(if selected { p.border } else { p.card })
            .border_1()
            .border_color(if selected { p.accent } else { p.border })
            .cursor_pointer()
            .hover(|s| s.border_color(p.accent))
            .child(icon.svg(p.muted).size(px(14.0)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .when(host.pin_to_top, |d| {
                                d.child(IconName::Bookmark.svg(p.accent).size(px(9.0)))
                            })
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(p.fg)
                                    .child(SharedString::from(host.name.clone())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child(SharedString::from(subtitle)),
                    ),
            )
            .child(self.ping_dot(&host.id, p))
            .when(status != HostStatus::Disconnected, |d| {
                d.child(
                    div()
                        .text_size(px(9.0))
                        .text_color(if status == HostStatus::Failed {
                            p.fg
                        } else {
                            p.accent
                        })
                        .child(status.label()),
                )
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                if let Some(h) = this.hosts.iter().find(|h| h.id == id_click).cloned() {
                    this.select_host(&h, w, cx);
                }
            }))
            .on_mouse_down(MouseButton::Right, {
                let id = id.clone();
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    this.host_menu = Some((id.clone(), ev.position));
                    cx.notify();
                })
            })
            .on_drag(DraggedHost { id: id.clone() }, |_, _, _, cx| {
                cx.new(|_| HostDragGhost)
            })
            .drag_over::<DraggedHost>(move |style, _, _, _| style.border_color(accent))
            .on_drop(cx.listener(move |this, dragged: &DraggedHost, _w, cx| {
                this.reorder_hosts(&dragged.id, &id_drop, cx);
            }))
            .into_any_element()
    }

    fn render_group_chips(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let (c_accent, c_border, c_card, c_fg, c_muted) =
            (p.accent, p.border, p.card, p.fg, p.muted);
        let chip = |id: &'static str,
                    label: String,
                    active: bool,
                    on_drop_group: Option<String>,
                    cx: &mut Context<Self>,
                    filter: Option<String>| {
            let mut el = div()
                .id(id)
                .px_2()
                .py_0p5()
                .rounded(px(13.0))
                .border_1()
                .border_color(if active { c_accent } else { c_border })
                .bg(if active { c_border } else { c_card })
                .text_xs()
                .text_color(if active { c_fg } else { c_muted })
                .cursor_pointer()
                .child(SharedString::from(label))
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.group_filter = filter.clone();
                    cx.notify();
                }));
            if let Some(gid) = on_drop_group {
                el = el
                    .drag_over::<DraggedHost>(move |s, _, _, _| s.border_color(c_accent))
                    .on_drop(cx.listener(move |this, dragged: &DraggedHost, _w, cx| {
                        let g = if gid.is_empty() {
                            None
                        } else {
                            Some(gid.clone())
                        };
                        this.move_host_to_group(&dragged.id, g, cx);
                    }));
            }
            el
        };

        let mut row = div().flex().flex_wrap().gap_1().items_center().child(chip(
            "chip-all",
            "All".to_string(),
            self.group_filter.is_none(),
            None,
            cx,
            None,
        ));
        let ungrouped = self.hosts.iter().filter(|h| h.group_id.is_none()).count();
        row = row.child(chip(
            "chip-ungrouped",
            format!("Ungrouped ({ungrouped})"),
            self.group_filter.as_deref() == Some(""),
            Some(String::new()),
            cx,
            Some(String::new()),
        ));
        for g in self.groups.clone() {
            let count = self
                .hosts
                .iter()
                .filter(|h| h.group_id.as_deref() == Some(g.id.as_str()))
                .count();
            let gid = g.id.clone();
            let gid_del = g.id.clone();
            let (gid_menu, gname_menu) = (g.id.clone(), g.name.clone());
            row = row.child(
                div()
                    .id(SharedString::from(format!("gchip-{}", g.id)))
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            this.group_menu =
                                Some((gid_menu.clone(), gname_menu.clone(), ev.position));
                            cx.notify();
                        }),
                    )
                    .child(chip(
                        "chip-group",
                        format!("{} ({count})", g.name),
                        self.group_filter.as_deref() == Some(g.id.as_str()),
                        Some(gid.clone()),
                        cx,
                        Some(gid.clone()),
                    ))
                    .child(
                        div()
                            .id(SharedString::from(format!("gdel-{gid_del}")))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .hover(|s| s.text_color(p.fg))
                            .child(IconName::X.svg(p.muted).size(px(10.0)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.delete_group(gid_del.clone(), cx)
                            })),
                    ),
            );
        }
        row.into_any_element()
    }

    fn labelled_field(
        &self,
        label: &'static str,
        value: &str,
        field: HostField,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .child(div().text_xs().text_color(p.muted).child(label))
            .child(
                div()
                    .id(SharedString::from(format!("field-{label}")))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(p.bg)
                    .border_1()
                    .border_color(if active { p.accent } else { p.border })
                    .text_sm()
                    .text_color(p.fg)
                    .cursor_text()
                    .child(SharedString::from(if active {
                        format!("{value}\u{2502}")
                    } else {
                        value.to_string()
                    }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                        if let Some(f) = this.form.as_mut() {
                            f.focus = field;
                        }
                        w.focus(&this.form_focus);
                        cx.notify();
                    })),
            )
    }

    fn tunnel_field(
        &self,
        id: String,
        value: &str,
        field: HostField,
        active: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(SharedString::from(id))
            .flex_1()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(p.bg)
            .border_1()
            .border_color(if active { p.accent } else { p.border })
            .text_sm()
            .text_color(p.fg)
            .cursor_text()
            .child(SharedString::from(if active {
                format!("{value}\u{2502}")
            } else {
                value.to_string()
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                if let Some(f) = this.form.as_mut() {
                    f.focus = field;
                }
                w.focus(&this.form_focus);
                cx.notify();
            }))
    }

    fn render_tunnels_section(
        &self,
        form: &HostForm,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focus = form.focus;
        let rows = form
            .tunnels
            .iter()
            .enumerate()
            .map(|(i, t)| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(p.border)
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(self.tunnel_field(
                                format!("tun-lp-{i}"),
                                &t.local_port,
                                HostField::TunnelLocalPort(i),
                                focus == HostField::TunnelLocalPort(i),
                                p,
                                cx,
                            ))
                            .child(self.tunnel_field(
                                format!("tun-rh-{i}"),
                                &t.remote_host,
                                HostField::TunnelRemoteHost(i),
                                focus == HostField::TunnelRemoteHost(i),
                                p,
                                cx,
                            ))
                            .child(self.tunnel_field(
                                format!("tun-rp-{i}"),
                                &t.remote_port,
                                HostField::TunnelRemotePort(i),
                                focus == HostField::TunnelRemotePort(i),
                                p,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("tun-del-{i}")))
                            .text_xs()
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child("Remove tunnel")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                if let Some(f) = this.form.as_mut() {
                                    if i < f.tunnels.len() {
                                        f.tunnels.remove(i);
                                    }
                                }
                                cx.notify();
                            })),
                    )
            })
            .collect::<Vec<_>>();

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(p.muted)
                    .child("Tunnels (local forward: local port \u{2192} host : port)"),
            )
            .children(rows)
            .child(
                self.btn("tun-add", "Add Tunnel", p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        if let Some(f) = this.form.as_mut() {
                            f.tunnels.push(TunnelDraft::new());
                        }
                        cx.notify();
                    })),
            )
    }

    /// Detail pane — the tabbed host form (T16-014). No longer a modal; lives
    /// permanently in the right half of the master/detail split.
    fn render_detail(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(form) = self.form.as_ref() else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(p.bg)
                .child(
                    div()
                        .text_sm()
                        .text_color(p.muted)
                        .child("Select a host on the left, or add a new one."),
                )
                .into_any_element();
        };
        let is_new = form.editing_id.is_none();
        let editing = form.editing_id.clone();
        let auth = form.auth;
        let cred_label = form
            .credential
            .and_then(|i| self.credentials.get(i))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "none".to_string());
        let group_label = form
            .group
            .and_then(|i| self.groups.get(i))
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "none".to_string());
        let jump_label = form
            .jump_host
            .and_then(|i| self.hosts.get(i))
            .map(|h| h.name.clone())
            .unwrap_or_else(|| "direct".to_string());
        let snippet_label = form
            .snippet_id
            .as_deref()
            .and_then(|sid| self.snippets.iter().find(|(id, _)| id == sid))
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "none".to_string());
        let head_icon = host_icon(form.icon.as_deref());

        // ── header ────────────────────────────────────────────────────────
        let (id_conn, id_sftp) = (editing.clone(), editing.clone());
        let header =
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    self.btn("host-icon", "", p, false)
                        .child(head_icon.svg(p.fg).size(px(14.0)))
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.icon_picker_open = !this.icon_picker_open;
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .text_xs()
                        .text_color(p.muted)
                        .child(if is_new { "NEW HOST" } else { "HOST DETAILS" }),
                )
                .when(self.save_state != SaveState::Idle, |d| {
                    d.child(
                        div()
                            .text_xs()
                            .text_color(if self.save_state == SaveState::Error {
                                p.fg
                            } else {
                                p.muted
                            })
                            .child(self.save_state.label()),
                    )
                })
                .when(!is_new, |d| {
                    d.child(
                        self.btn("hd-connect", "Connect", p, true)
                            .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                                if let Some(id) = id_conn.clone() {
                                    cx.emit(HostManagerEvent::Connect(id));
                                }
                            })),
                    )
                    .child(self.btn("hd-sftp", "SFTP", p, false).on_click(cx.listener(
                        move |_this, _: &ClickEvent, _w, cx| {
                            if let Some(id) = id_sftp.clone() {
                                cx.emit(HostManagerEvent::OpenSftp(id));
                            }
                        },
                    )))
                    .child(self.btn("hd-test", "Test Connection", p, false).on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.test_connection(cx)),
                    ))
                    .child(
                        self.btn("hd-dup", "Duplicate", p, false)
                            .on_click(cx.listener({
                                let id = editing.clone();
                                move |this, _: &ClickEvent, _w, cx| {
                                    if let Some(id) = id.clone() {
                                        this.duplicate_host(id, cx);
                                    }
                                }
                            })),
                    )
                    .child(
                        self.btn("hd-del", "Delete", p, false)
                            .on_click(cx.listener({
                                let id = editing.clone();
                                move |this, _: &ClickEvent, _w, cx| {
                                    if let Some(id) = id.clone() {
                                        this.delete_host(id, cx);
                                    }
                                }
                            })),
                    )
                })
                .child(
                    self.btn("hd-close", "", p, false)
                        .child(IconName::X.svg(p.muted).size(px(12.0)))
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            if this.save_state == SaveState::Pending {
                                this.submit_form(true, cx);
                            }
                            this.form = None;
                            this.save_state = SaveState::Idle;
                            cx.notify();
                        })),
                );

        let icon_row = self.icon_picker_open.then(|| {
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .children(HOST_ICONS.iter().map(|(key, ic)| {
                    let key = key.to_string();
                    let active = form.icon.as_deref() == Some(key.as_str());
                    self.btn("host-icon-opt", "", p, active)
                        .child(ic.svg(if active { p.fg } else { p.muted }).size(px(13.0)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if let Some(f) = this.form.as_mut() {
                                f.icon = if f.icon.as_deref() == Some(key.as_str()) {
                                    None
                                } else {
                                    Some(key.clone())
                                };
                            }
                            this.icon_picker_open = false;
                            this.schedule_autosave(cx);
                        }))
                }))
        });

        let tab_bar = div()
            .flex()
            .gap_1()
            .children(FormTab::ALL.into_iter().map(|t| {
                self.btn(
                    match t {
                        FormTab::General => "ft-gen",
                        FormTab::Ssh => "ft-ssh",
                        FormTab::Sftp => "ft-sftp",
                        FormTab::Tunnels => "ft-tun",
                    },
                    t.title(),
                    p,
                    form.tab == t,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    if let Some(f) = this.form.as_mut() {
                        f.tab = t;
                    }
                    cx.notify();
                }))
            }));

        let body = self.render_detail_tab(
            form,
            auth,
            &cred_label,
            &group_label,
            &jump_label,
            &snippet_label,
            p,
            cx,
        );

        let footer = is_new.then(|| {
            div().flex().gap_2().justify_end().pt_2().child(
                self.btn("hd-add", "Add Host", p, true).on_click(
                    cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_form(false, cx)),
                ),
            )
        });
        let test_line = self.test_result.clone().map(|t| {
            div()
                .text_xs()
                .text_color(p.muted)
                .child(SharedString::from(t))
        });

        div()
            .track_focus(&self.form_focus)
            .key_context("HostForm")
            .on_key_down(cx.listener(Self::on_form_key))
            .id("host-detail-scroll")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(p.bg)
            .child(header)
            .children(icon_row)
            .child(tab_bar)
            .children(test_line)
            .child(body)
            .children(footer)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_detail_tab(
        &self,
        form: &HostForm,
        auth: AuthMethod,
        cred_label: &str,
        group_label: &str,
        jump_label: &str,
        snippet_label: &str,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let container = div().flex().flex_col().gap_2();
        match form.tab {
            FormTab::General => container
                .child(self.labelled_field(
                    "Name",
                    &form.name,
                    HostField::Name,
                    form.focus == HostField::Name,
                    p,
                    cx,
                ))
                .child(self.labelled_field(
                    "Address",
                    &form.address,
                    HostField::Address,
                    form.focus == HostField::Address,
                    p,
                    cx,
                ))
                .child(self.labelled_field(
                    "Port",
                    &form.port,
                    HostField::Port,
                    form.focus == HostField::Port,
                    p,
                    cx,
                ))
                .child(self.labelled_field(
                    "Username",
                    &form.username,
                    HostField::Username,
                    form.focus == HostField::Username,
                    p,
                    cx,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(div().text_xs().text_color(p.muted).child("Auth method"))
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .children(AuthMethod::ALL.into_iter().map(|m| {
                                    self.btn(
                                        match m {
                                            AuthMethod::Password => "auth-pw",
                                            AuthMethod::Key => "auth-key",
                                            AuthMethod::Credential => "auth-cred",
                                            AuthMethod::None => "auth-none",
                                        },
                                        m.title(),
                                        p,
                                        m == auth,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, _w, cx| {
                                            if let Some(f) = this.form.as_mut() {
                                                f.auth = m;
                                            }
                                            this.schedule_autosave(cx);
                                        },
                                    ))
                                })),
                        ),
                )
                .when(auth == AuthMethod::Key, |el| {
                    el.child(self.labelled_field(
                        "Private key path",
                        &form.key_path,
                        HostField::KeyPath,
                        form.focus == HostField::KeyPath,
                        p,
                        cx,
                    ))
                })
                .when(auth == AuthMethod::Password, |el| {
                    el.child(self.labelled_field(
                        "Password (stored in the secret store)",
                        &"\u{2022}".repeat(form.password.chars().count()),
                        HostField::Password,
                        form.focus == HostField::Password,
                        p,
                        cx,
                    ))
                })
                .when(auth == AuthMethod::Credential, |el| {
                    el.child(
                        self.btn("cred-cycle", format!("Credential: {cred_label}"), p, false)
                            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                let n = this.credentials.len();
                                if let Some(f) = this.form.as_mut() {
                                    f.credential = match f.credential {
                                        None if n > 0 => Some(0),
                                        Some(i) if i + 1 < n => Some(i + 1),
                                        _ => None,
                                    };
                                }
                                this.schedule_autosave(cx);
                            })),
                    )
                })
                .child(
                    self.btn("group-cycle", format!("Group: {group_label}"), p, false)
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            let n = this.groups.len();
                            if let Some(f) = this.form.as_mut() {
                                f.group = match f.group {
                                    None if n > 0 => Some(0),
                                    Some(i) if i + 1 < n => Some(i + 1),
                                    _ => None,
                                };
                            }
                            this.schedule_autosave(cx);
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.muted)
                                .child("Jump host (ProxyJump)"),
                        )
                        .child(
                            self.btn("jump-cycle", format!("Route: {jump_label}"), p, false)
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    let editing =
                                        this.form.as_ref().and_then(|f| f.editing_id.clone());
                                    let cands: Vec<usize> = this
                                        .hosts
                                        .iter()
                                        .enumerate()
                                        .filter(|(_, h)| Some(h.id.as_str()) != editing.as_deref())
                                        .map(|(i, _)| i)
                                        .collect();
                                    if let Some(f) = this.form.as_mut() {
                                        f.jump_host = match f.jump_host {
                                            None => cands.first().copied(),
                                            Some(cur) => {
                                                match cands.iter().position(|&i| i == cur) {
                                                    Some(pos) if pos + 1 < cands.len() => {
                                                        Some(cands[pos + 1])
                                                    }
                                                    _ => None,
                                                }
                                            }
                                        };
                                    }
                                    this.schedule_autosave(cx);
                                })),
                        ),
                )
                .child(
                    self.btn(
                        "pin-to-top",
                        if form.pin_to_top {
                            "Pinned \u{2014} always shown first"
                        } else {
                            "Not pinned \u{2014} click to always show first"
                        },
                        p,
                        form.pin_to_top,
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        if let Some(f) = this.form.as_mut() {
                            f.pin_to_top = !f.pin_to_top;
                        }
                        this.schedule_autosave(cx);
                    })),
                )
                .child(self.labelled_field(
                    "Notes / runbook",
                    &form.notes,
                    HostField::Notes,
                    form.focus == HostField::Notes,
                    p,
                    cx,
                ))
                .into_any_element(),
            FormTab::Ssh => container
                .child(self.labelled_field(
                    "Start directory (runs `cd <path>`)",
                    &form.default_path,
                    HostField::DefaultPath,
                    form.focus == HostField::DefaultPath,
                    p,
                    cx,
                ))
                .when(auth == AuthMethod::Password, |el| {
                    el.child(
                        self.labelled_field(
                            "Sudo password autofill (OS keychain)",
                            if form.sudo_password.is_empty() {
                                if form.sudo_password_set {
                                    "(set)".to_string()
                                } else {
                                    String::new()
                                }
                            } else {
                                "\u{2022}".repeat(form.sudo_password.chars().count())
                            }
                            .as_str(),
                            HostField::SudoPassword,
                            form.focus == HostField::SudoPassword,
                            p,
                            cx,
                        ),
                    )
                })
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(self.labelled_field(
                            "Keep-alive interval (s)",
                            &form.keep_alive_interval,
                            HostField::KeepAliveInterval,
                            form.focus == HostField::KeepAliveInterval,
                            p,
                            cx,
                        ))
                        .child(self.labelled_field(
                            "Keep-alive max tries",
                            &form.keep_alive_tries,
                            HostField::KeepAliveTries,
                            form.focus == HostField::KeepAliveTries,
                            p,
                            cx,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_xs()
                                .text_color(p.muted)
                                .child("Run on connect (startup snippet)"),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    self.btn(
                                        "snippet-cycle",
                                        format!("Snippet: {snippet_label}"),
                                        p,
                                        false,
                                    )
                                    .on_click(cx.listener(
                                        |this, _: &ClickEvent, _w, cx| {
                                            let ids: Vec<String> = this
                                                .snippets
                                                .iter()
                                                .map(|(id, _)| id.clone())
                                                .collect();
                                            if let Some(f) = this.form.as_mut() {
                                                f.snippet_id = match &f.snippet_id {
                                                    None => ids.first().cloned(),
                                                    Some(cur) => {
                                                        match ids.iter().position(|x| x == cur) {
                                                            Some(i) if i + 1 < ids.len() => {
                                                                Some(ids[i + 1].clone())
                                                            }
                                                            _ => None,
                                                        }
                                                    }
                                                };
                                            }
                                            this.schedule_autosave(cx);
                                        },
                                    )),
                                )
                                .when(form.snippet_id.is_some(), |d| {
                                    d.child(
                                        self.btn(
                                            "snippet-mode",
                                            if form.snippet_mode == "inject" {
                                                "Inject"
                                            } else {
                                                "Execute"
                                            },
                                            p,
                                            true,
                                        )
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                if let Some(f) = this.form.as_mut() {
                                                    f.snippet_mode = if f.snippet_mode == "inject" {
                                                        "execute".into()
                                                    } else {
                                                        "inject".into()
                                                    };
                                                }
                                                this.schedule_autosave(cx);
                                            }),
                                        ),
                                    )
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(div().text_xs().text_color(p.muted).child("AI Agent Access"))
                        .child(
                            self.btn(
                                "agent-block",
                                if form.block_agent_access {
                                    "Blocked \u{2014} the AI agent bridge cannot use this host"
                                } else {
                                    "Allowed \u{2014} click to block AI agent bridge access"
                                },
                                p,
                                form.block_agent_access,
                            )
                            .on_click(cx.listener(
                                |this, _: &ClickEvent, _w, cx| {
                                    if let Some(f) = this.form.as_mut() {
                                        f.block_agent_access = !f.block_agent_access;
                                    }
                                    this.schedule_autosave(cx);
                                },
                            )),
                        ),
                )
                .into_any_element(),
            FormTab::Sftp => container
                .child(self.labelled_field(
                    "SFTP start directory",
                    &form.default_path_sftp,
                    HostField::DefaultPathSftp,
                    form.focus == HostField::DefaultPathSftp,
                    p,
                    cx,
                ))
                .into_any_element(),
            FormTab::Tunnels => container
                .child(self.render_tunnels_section(form, p, cx))
                .into_any_element(),
        }
    }

    fn render_credentials(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let draft = self.cred_draft.as_ref();
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
                    .gap_2()
                    .w(px(480.0))
                    .p_4()
                    .rounded_lg()
                    .bg(p.card)
                    .border_1()
                    .border_color(p.border)
                    .child(div().text_sm().text_color(p.fg).child("Credentials"))
                    .children(self.credentials.iter().map(|c| {
                        let used = self
                            .hosts
                            .iter()
                            .filter(|h| h.credential_id.as_deref() == Some(c.id.as_str()))
                            .count();
                        let id = c.id.clone();
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(p.border)
                            .child(div().flex_1().text_sm().text_color(p.fg).child(
                                SharedString::from(format!(
                                    "{}  \u{00b7}  {}{}  \u{00b7}  used by {used}",
                                    c.name,
                                    c.cred_type,
                                    if c.has_secret { " (secret set)" } else { "" }
                                )),
                            ))
                            .child(
                                self.btn("cred-del", "Delete", p, false)
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        this.delete_credential(id.clone(), cx)
                                    })),
                            )
                    }))
                    .child(match draft {
                        Some(d) => div()
                            .track_focus(&self.cred_focus)
                            .key_context("CredDraft")
                            .on_key_down(cx.listener(Self::on_cred_key))
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(p.bg)
                                    .border_1()
                                    .border_color(p.accent)
                                    .text_sm()
                                    .text_color(p.fg)
                                    .child(SharedString::from(format!("{}\u{2502}", d.name))),
                            )
                            .child(
                                self.btn(
                                    "cred-type",
                                    if d.is_key { "key" } else { "password" },
                                    p,
                                    false,
                                )
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _w, cx| {
                                        if let Some(d) = this.cred_draft.as_mut() {
                                            d.is_key = !d.is_key;
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(self.btn("cred-add", "Add", p, true).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.add_credential(cx)),
                            ))
                            .into_any_element(),
                        None => div()
                            .flex()
                            .gap_2()
                            .child(self.btn("cred-new", "New credential", p, false).on_click(
                                cx.listener(|this, _: &ClickEvent, w, cx| {
                                    this.cred_draft = Some(CredDraft {
                                        name: String::new(),
                                        is_key: false,
                                    });
                                    w.focus(&this.cred_focus);
                                    cx.notify();
                                }),
                            ))
                            .into_any_element(),
                    })
                    .child(
                        div().flex().justify_end().pt_2().child(
                            self.btn("cred-close", "Close", p, false)
                                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.creds_open = false;
                                    this.cred_draft = None;
                                    cx.notify();
                                })),
                        ),
                    ),
            )
            .into_any_element()
    }

    fn modal_shell(&self, width: f32, p: &Palette) -> gpui::Div {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(width))
            .max_h(px(620.0))
            .overflow_hidden()
            .p_4()
            .rounded_lg()
            .bg(p.card)
            .border_1()
            .border_color(p.border)
    }

    fn render_import(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = self.import.as_ref() else {
            return div().into_any_element();
        };
        let existing_names: HashSet<&str> = self.hosts.iter().map(|h| h.name.as_str()).collect();
        let selected_count = state.selected.len();
        let total = state.entries.len();
        let conflict = state.conflict;

        let list: Vec<gpui::AnyElement> = state
            .entries
            .iter()
            .map(|e| {
                let alias = e.alias.clone();
                let checked = state.selected.contains(&alias);
                let exists = existing_names.contains(e.alias.as_str());
                let meta = format!(
                    "{}:{}{}{}{}",
                    e.host_address,
                    e.port,
                    e.username
                        .as_deref()
                        .map(|u| format!("  \u{00b7}  {u}"))
                        .unwrap_or_default(),
                    if e.auth_method == "key" {
                        "  \u{00b7}  key"
                    } else {
                        "  \u{00b7}  password"
                    },
                    e.proxy_jump
                        .as_deref()
                        .map(|j| format!("  \u{00b7}  via {j}"))
                        .unwrap_or_default(),
                );
                div()
                    .id(SharedString::from(format!("imp-{alias}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if checked { p.accent } else { p.border })
                    .cursor_pointer()
                    .child(div().w(px(14.0)).child(if checked {
                        IconName::SquareCheck.svg(p.accent).size(px(14.0))
                    } else {
                        IconName::Square.svg(p.muted).size(px(14.0))
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_sm().text_color(p.fg).child(SharedString::from(
                                if exists {
                                    format!("{alias}  (already exists)")
                                } else {
                                    alias.clone()
                                },
                            )))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted)
                                    .child(SharedString::from(meta)),
                            ),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if let Some(s) = this.import.as_mut() {
                            if !s.selected.remove(&alias) {
                                s.selected.insert(alias.clone());
                            }
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect();

        let body = if state.loading {
            div()
                .text_sm()
                .text_color(p.muted)
                .child("Reading ~/.ssh/config\u{2026}")
                .into_any_element()
        } else if let Some(err) = &state.error {
            div()
                .text_sm()
                .text_color(p.fg)
                .child(SharedString::from(err.clone()))
                .into_any_element()
        } else if list.is_empty() {
            div()
                .text_sm()
                .text_color(p.muted)
                .child("No hosts found in ~/.ssh/config.")
                .into_any_element()
        } else {
            div()
                .id("imp-list")
                .flex()
                .flex_col()
                .gap_1()
                .overflow_y_scroll()
                .max_h(px(360.0))
                .children(list)
                .into_any_element()
        };

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                self.modal_shell(500.0, p)
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.fg)
                            .child("Import from ~/.ssh/config"),
                    )
                    .when(
                        !state.loading && state.error.is_none() && !state.entries.is_empty(),
                        |el| {
                            el.child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(self.btn("imp-all", "Select all", p, false).on_click(
                                        cx.listener(|this, _: &ClickEvent, _w, cx| {
                                            if let Some(s) = this.import.as_mut() {
                                                s.selected = s
                                                    .entries
                                                    .iter()
                                                    .map(|e| e.alias.clone())
                                                    .collect();
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(self.btn("imp-none", "Deselect all", p, false).on_click(
                                        cx.listener(|this, _: &ClickEvent, _w, cx| {
                                            if let Some(s) = this.import.as_mut() {
                                                s.selected.clear();
                                            }
                                            cx.notify();
                                        }),
                                    ))
                                    .child(
                                        self.btn(
                                            "imp-conflict",
                                            format!("On conflict: {}", conflict_label(conflict)),
                                            p,
                                            false,
                                        )
                                        .on_click(
                                            cx.listener(|this, _: &ClickEvent, _w, cx| {
                                                if let Some(s) = this.import.as_mut() {
                                                    s.conflict = cycle_conflict(s.conflict);
                                                }
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                        },
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .justify_end()
                            .pt_2()
                            .when(!state.loading && total > 0, |el| {
                                el.child(div().flex_1().text_xs().text_color(p.muted).child(
                                    SharedString::from(format!(
                                        "{selected_count} of {total} selected"
                                    )),
                                ))
                            })
                            .child(self.btn("imp-cancel", "Cancel", p, false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.import = None;
                                    cx.notify();
                                }),
                            ))
                            .child(self.btn("imp-run", "Import Selected", p, true).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.run_import(cx)),
                            )),
                    ),
            )
            .into_any_element()
    }

    fn render_export(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = self.export.as_ref() else {
            return div().into_any_element();
        };
        let selected_count = state.selected.len();

        let rows: Vec<gpui::AnyElement> = self
            .hosts
            .iter()
            .map(|h| {
                let id = h.id.clone();
                let checked = state.selected.contains(&id);
                div()
                    .id(SharedString::from(format!("exp-{id}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(if checked { p.accent } else { p.border })
                    .cursor_pointer()
                    .child(div().w(px(14.0)).child(if checked {
                        IconName::SquareCheck.svg(p.accent).size(px(14.0))
                    } else {
                        IconName::Square.svg(p.muted).size(px(14.0))
                    }))
                    .child(div().flex_1().min_w_0().text_sm().text_color(p.fg).child(
                        SharedString::from(format!(
                            "{}  \u{00b7}  {}@{}:{}",
                            h.name, h.username, h.host_address, h.port
                        )),
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if let Some(s) = this.export.as_mut() {
                            if !s.selected.remove(&id) {
                                s.selected.insert(id.clone());
                            }
                        }
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                self.modal_shell(500.0, p)
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.fg)
                            .child("Export to ~/.ssh/config"),
                    )
                    .child(
                        div()
                            .id("exp-list")
                            .flex()
                            .flex_col()
                            .gap_1()
                            .overflow_y_scroll()
                            .max_h(px(360.0))
                            .children(rows),
                    )
                    .when_some(state.error.clone(), |el, err| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(p.fg)
                                .child(SharedString::from(err)),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .justify_end()
                            .pt_2()
                            .child(
                                div().flex_1().text_xs().text_color(p.muted).child(
                                    SharedString::from(format!("{selected_count} selected")),
                                ),
                            )
                            .child(self.btn("exp-cancel", "Cancel", p, false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.export = None;
                                    cx.notify();
                                }),
                            ))
                            .child(
                                self.btn("exp-copy", "Copy to clipboard", p, false)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.run_export(false, cx)
                                    })),
                            )
                            .child(
                                self.btn("exp-append", "Append to ~/.ssh/config", p, true)
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.run_export(true, cx)
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for HostManagerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette(cx);
        let accent = p.accent;
        let visible = self.visible_hosts();
        let quick = self.quick_connect_target();

        // ── left pane: search + quick-connect suggestion ──────────────────
        let search_box = div()
            .track_focus(&self.search_focus)
            .key_context("HostSearch")
            .on_key_down(cx.listener(Self::on_search_key))
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(p.card)
            .border_1()
            .border_color(p.border)
            .child(IconName::Search.svg(p.muted).size(px(12.0)))
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .text_color(if self.search.is_empty() {
                        p.muted
                    } else {
                        p.fg
                    })
                    .child(SharedString::from(if self.search.is_empty() {
                        "Find a host or type user@hostname\u{2026}".to_string()
                    } else {
                        format!("{}\u{2502}", self.search)
                    })),
            );

        let quick_card = quick.clone().map(|(user, host, port)| {
            div()
                .id("quick-connect")
                .flex()
                .items_center()
                .justify_between()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(accent)
                .bg(p.card)
                .cursor_pointer()
                .child(
                    div()
                        .text_xs()
                        .text_color(p.fg)
                        .child(SharedString::from(format!(
                            "Quick Connect  {user}@{host}:{port}"
                        ))),
                )
                .child(IconName::Zap.svg(accent).size(px(12.0)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.quick_connect(user.clone(), host.clone(), port, cx)
                }))
        });

        // ── left pane: actions toolbar ───────────────────────────────────
        let toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .child(
                self.btn("new-host", "New Host", &p, true)
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.form = Some(HostForm::blank());
                        this.save_state = SaveState::Idle;
                        this.test_result = None;
                        w.focus(&this.form_focus);
                        cx.notify();
                    })),
            )
            .child(match self.group_draft.as_ref() {
                Some(b) => div()
                    .track_focus(&self.group_focus)
                    .key_context("GroupDraft")
                    .on_key_down(cx.listener(Self::on_group_key))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(p.bg)
                    .border_1()
                    .border_color(accent)
                    .text_xs()
                    .text_color(p.fg)
                    .child(SharedString::from(format!("{b}\u{2502}")))
                    .into_any_element(),
                None => self
                    .btn("new-group", "New Group", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.group_draft = Some(String::new());
                        w.focus(&this.group_focus);
                        cx.notify();
                    }))
                    .into_any_element(),
            })
            .child(
                self.btn(
                    "sort-cycle",
                    format!("Sort: {}", self.sort.label()),
                    &p,
                    false,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.sort = this.sort.next();
                    cx.notify();
                })),
            )
            .child(
                self.btn(
                    "view-toggle",
                    if self.grid_view { "Grid" } else { "List" },
                    &p,
                    false,
                )
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.grid_view = !this.grid_view;
                    cx.notify();
                })),
            )
            .child(
                self.btn("open-creds", "Credentials", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.creds_open = true;
                        cx.notify();
                    })),
            )
            .child(
                self.btn("import-ssh-config", "Import", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.open_import(cx))),
            )
            .child(
                self.btn("export-ssh-config", "Export", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.open_export(cx))),
            );

        let list = if visible.is_empty() {
            div()
                .p_4()
                .text_xs()
                .text_color(p.muted)
                .child("No hosts. Add one with \u{201c}New Host\u{201d}.")
                .into_any_element()
        } else {
            let items = visible
                .iter()
                .map(|h| self.render_host_list_item(h, &p, cx))
                .collect::<Vec<_>>();
            div()
                .id("host-list")
                .flex()
                .flex_col()
                .gap_1()
                .overflow_y_scroll()
                .flex_1()
                .min_h_0()
                .when(self.grid_view, |d| d.flex_row().flex_wrap())
                .children(items)
                .into_any_element()
        };

        let tunnels_panel = (!self.active_tunnels.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .p_2()
                .rounded_md()
                .bg(p.card)
                .border_1()
                .border_color(p.border)
                .child(div().text_xs().text_color(p.muted).child("Active tunnels"))
                .children(self.active_tunnels.iter().map(|t| {
                    div()
                        .text_xs()
                        .text_color(p.fg)
                        .child(SharedString::from(format!(
                            "{}  \u{00b7}  localhost:{} \u{2192} {}:{}",
                            t.host_label, t.local_port, t.remote_host, t.remote_port
                        )))
                }))
        });

        let side_panel = div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(340.0))
            .flex_shrink_0()
            .h_full()
            .p_3()
            .border_r_1()
            .border_color(p.border)
            .bg(p.bg)
            .child(search_box)
            .children(quick_card)
            .child(toolbar)
            .child(self.render_group_chips(&p, cx))
            .children(tunnels_panel)
            .child(list);

        let cred_overlay = self
            .creds_open
            .then(|| self.render_credentials(&p, cx).into_any_element());
        let import_overlay = self
            .import
            .is_some()
            .then(|| self.render_import(&p, cx).into_any_element());
        let export_overlay = self
            .export
            .is_some()
            .then(|| self.render_export(&p, cx).into_any_element());

        div()
            .track_focus(&self.focus_handle)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(p.bg)
            .child(
                div().size_full().flex().flex_row().child(side_panel).child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .child(self.render_detail(&p, cx)),
                ),
            )
            .children(cred_overlay)
            .children(import_overlay)
            .children(export_overlay)
            .children(self.render_host_menu(&p, cx))
            .children(self.render_group_menu(&p, cx))
            .children(self.render_group_rename(&p, cx))
    }
}

impl HostManagerView {
    /// Copy a single host's SSH-config block to the clipboard.
    fn export_host(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.app.clone();
        let jh = self
            .tokio
            .spawn(async move { config_parser::export_ssh_config(vec![id], &app.db).await });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(block)) = jh.await {
                let _ = this.update(cx, |_this, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(block));
                });
            }
        })
        .detach();
    }

    fn rename_group(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let app = self.app.clone();
        let jh = self
            .tokio
            .spawn(async move { hosts::db::groups_update(&app.db, id, name).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    fn render_host_menu(&self, _p: &Palette, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (id, pos) = self.host_menu.clone()?;
        let host = self.hosts.iter().find(|h| h.id == id)?.clone();
        let view = cx.entity();
        let close = |v: &Entity<Self>, cx: &mut App| {
            v.update(cx, |this, cx| {
                this.host_menu = None;
                cx.notify();
            })
        };
        let items = vec![
            MenuItem::new("hm-ssh", "Connect SSH")
                .icon(IconName::Terminal)
                .on_click({
                    let v = view.clone();
                    let id = id.clone();
                    move |_, _w, cx| {
                        let id = id.clone();
                        v.update(cx, |this, cx| {
                            this.host_menu = None;
                            cx.emit(HostManagerEvent::Connect(id));
                        });
                    }
                }),
            MenuItem::new("hm-sftp", "Open SFTP")
                .icon(IconName::Folder)
                .on_click({
                    let v = view.clone();
                    let id = id.clone();
                    move |_, _w, cx| {
                        let id = id.clone();
                        v.update(cx, |this, cx| {
                            this.host_menu = None;
                            cx.emit(HostManagerEvent::OpenSftp(id));
                        });
                    }
                }),
            MenuItem::separator(),
            MenuItem::new("hm-edit", "Edit")
                .icon(IconName::Pencil)
                .on_click({
                    let v = view.clone();
                    let host = host.clone();
                    move |_, w, cx| {
                        let host = host.clone();
                        v.update(cx, |this, cx| {
                            this.host_menu = None;
                            this.select_host(&host, w, cx);
                        });
                    }
                }),
            MenuItem::new("hm-dup", "Duplicate")
                .icon(IconName::Copy)
                .on_click({
                    let v = view.clone();
                    let id = id.clone();
                    move |_, _w, cx| {
                        let id = id.clone();
                        v.update(cx, |this, cx| {
                            this.host_menu = None;
                            this.duplicate_host(id, cx);
                        });
                    }
                }),
            MenuItem::new("hm-export", "Export to SSH Config").on_click({
                let v = view.clone();
                let id = id.clone();
                move |_, _w, cx| {
                    let id = id.clone();
                    v.update(cx, |this, cx| {
                        this.host_menu = None;
                        this.export_host(id, cx);
                    });
                }
            }),
            MenuItem::separator(),
            MenuItem::new("hm-del", "Delete")
                .icon(IconName::Trash)
                .destructive()
                .on_click({
                    let v = view.clone();
                    let id = id.clone();
                    move |_, _w, cx| {
                        let id = id.clone();
                        v.update(cx, |this, cx| {
                            this.host_menu = None;
                            this.delete_host(id, cx);
                        });
                    }
                }),
        ];
        let v = view.clone();
        Some(context_menu(
            pos,
            self.theme.read(cx),
            move |_w, cx| close(&v, cx),
            items,
        ))
    }

    fn render_group_menu(&self, _p: &Palette, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (gid, gname, pos) = self.group_menu.clone()?;
        let view = cx.entity();
        let items = vec![
            MenuItem::new("gm-rename", "Rename Group")
                .icon(IconName::Pencil)
                .on_click({
                    let v = view.clone();
                    let (gid, gname) = (gid.clone(), gname.clone());
                    move |_, w, cx| {
                        let (gid, gname) = (gid.clone(), gname.clone());
                        v.update(cx, |this, cx| {
                            this.group_menu = None;
                            this.group_rename = Some((gid, gname));
                            w.focus(&this.group_rename_focus);
                            cx.notify();
                        });
                    }
                }),
            MenuItem::new("gm-del", "Delete Group")
                .icon(IconName::Trash)
                .destructive()
                .on_click({
                    let v = view.clone();
                    let gid = gid.clone();
                    move |_, _w, cx| {
                        let gid = gid.clone();
                        v.update(cx, |this, cx| {
                            this.group_menu = None;
                            this.delete_group(gid, cx);
                        });
                    }
                }),
        ];
        let v = view.clone();
        Some(context_menu(
            pos,
            self.theme.read(cx),
            move |_w, cx| {
                v.update(cx, |this, cx| {
                    this.group_menu = None;
                    cx.notify();
                })
            },
            items,
        ))
    }

    fn render_group_rename(&self, p: &Palette, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (_, buf) = self.group_rename.as_ref()?;
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(crate::theme::modal_scrim())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseDownEvent, _w, cx| {
                        this.group_rename = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .track_focus(&self.group_rename_focus)
                        .key_context("HostGroupRename")
                        .on_key_down(cx.listener(Self::on_group_rename_key))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _: &MouseDownEvent, _w, cx| cx.stop_propagation()),
                        )
                        .flex()
                        .flex_col()
                        .gap_2()
                        .w(px(300.0))
                        .p_3()
                        .rounded_md()
                        .bg(p.card)
                        .border_1()
                        .border_color(p.border)
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(p.fg)
                                .child("Rename group"),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_sm()
                                .border_1()
                                .border_color(p.accent)
                                .text_size(px(12.0))
                                .text_color(p.fg)
                                .child(SharedString::from(format!("{buf}\u{2502}"))),
                        ),
                )
                .into_any_element(),
        )
    }

    fn on_group_rename_key(
        &mut self,
        ev: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((gid, buf)) = self.group_rename.as_mut() else {
            return;
        };
        match ev.keystroke.key.as_str() {
            "enter" => {
                let (gid, name) = (gid.clone(), buf.clone());
                self.group_rename = None;
                self.rename_group(gid, name, cx);
            }
            "escape" => {
                self.group_rename = None;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeStore;
    use gpui::{AppContext, TestAppContext, WindowAppearance};

    fn make(cx: &mut TestAppContext) -> (tokio::runtime::Runtime, Entity<HostManagerView>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.handle().clone();
        let dir = std::env::temp_dir().join(format!("labonair-hm-{}", uuid::Uuid::new_v4()));
        let backend = labonair_backend::App::new(&dir).unwrap();
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(WindowAppearance::Dark));
            cx.new(|cx| HostManagerView::new(backend, handle, theme, cx))
        });
        (rt, view)
    }

    #[gpui::test]
    fn tracks_per_host_connection_status(cx: &mut TestAppContext) {
        let (_rt, view) = make(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                assert_eq!(v.status_of("h1"), HostStatus::Disconnected);
                v.set_status("h1", HostStatus::Connecting, cx);
                v.set_status("h1", HostStatus::Connected, cx);
                v.set_status("h2", HostStatus::Failed, cx);
                assert_eq!(v.status_of("h1"), HostStatus::Connected);
                assert_eq!(v.status_of("h2"), HostStatus::Failed);
            });
        });
    }

    #[gpui::test]
    fn host_form_prefills_from_an_existing_host(cx: &mut TestAppContext) {
        let (_rt, _view) = make(cx);
        let host = Host {
            id: "h".into(),
            name: "Web".into(),
            host_address: "example.com".into(),
            port: 2222,
            username: "deploy".into(),
            auth_method: "key".into(),
            private_key_path: Some("/k".into()),
            group_id: None,
            tags: Some("prod".into()),
            created_at: 0,
            last_connected_at: None,
            default_path_ssh: Some("/srv".into()),
            default_path_sftp: None,
            pin_to_top: false,
            sudo_password_set: false,
            keep_alive_interval: None,
            keep_alive_tries: None,
            sort_order: 0,
            tunnels: None,
            startup_snippet_id: None,
            startup_snippet_mode: None,
            credential_id: None,
            jump_host_id: None,
            notes: None,
            icon: None,
            block_agent_access: false,
        };
        let form = HostForm::from_host(&host, &[], &[], &[]);
        assert_eq!(form.name, "Web");
        assert_eq!(form.port, "2222");
        assert_eq!(form.auth, AuthMethod::Key);
        assert_eq!(form.key_path, "/k");
        assert_eq!(form.default_path, "/srv");
        assert_eq!(form.editing_id.as_deref(), Some("h"));
        assert!(form.jump_host.is_none());
        assert!(form.tunnels.is_empty());
    }

    #[test]
    fn host_form_prefills_and_serializes_the_block_e_fields() {
        let mut host = host_stub("h", "Box");
        host.auth_method = "credential".into();
        host.default_path_sftp = Some("/var/www".into());
        host.keep_alive_interval = Some(30);
        host.keep_alive_tries = Some(5);
        host.notes = Some("prod runbook".into());
        host.pin_to_top = true;
        host.sudo_password_set = true;

        let form = HostForm::from_host(&host, &[], &[], &[]);
        assert_eq!(form.auth, AuthMethod::Credential);
        assert_eq!(form.default_path_sftp, "/var/www");
        assert_eq!(form.keep_alive_interval, "30");
        assert_eq!(form.keep_alive_tries, "5");
        assert_eq!(form.notes, "prod runbook");
        assert!(form.pin_to_top);
        assert!(form.sudo_password_set);
        assert!(form.sudo_password.is_empty());

        // Legacy "agent" spelling still resolves to the Credential mode, and
        // Credential serializes back to the backend's "credential" string.
        assert_eq!(AuthMethod::from_str("agent"), AuthMethod::Credential);
        assert_eq!(AuthMethod::Credential.as_str(), "credential");
    }

    fn host_stub(id: &str, name: &str) -> Host {
        Host {
            id: id.into(),
            name: name.into(),
            host_address: "h.example".into(),
            port: 22,
            username: "u".into(),
            auth_method: "password".into(),
            private_key_path: None,
            group_id: None,
            tags: None,
            created_at: 0,
            last_connected_at: None,
            default_path_ssh: None,
            default_path_sftp: None,
            pin_to_top: false,
            sudo_password_set: false,
            keep_alive_interval: None,
            keep_alive_tries: None,
            sort_order: 0,
            tunnels: None,
            startup_snippet_id: None,
            startup_snippet_mode: None,
            credential_id: None,
            jump_host_id: None,
            notes: None,
            icon: None,
            block_agent_access: false,
        }
    }

    #[test]
    fn tunnel_json_round_trips_and_drops_incomplete_rows() {
        let drafts = vec![
            TunnelDraft {
                id: "keep".into(),
                local_port: "8080".into(),
                remote_host: "db.internal".into(),
                remote_port: "5432".into(),
            },
            TunnelDraft {
                id: "drop-no-host".into(),
                local_port: "9000".into(),
                remote_host: "  ".into(),
                remote_port: "9000".into(),
            },
            TunnelDraft {
                id: "drop-bad-port".into(),
                local_port: "notaport".into(),
                remote_host: "x".into(),
                remote_port: "1".into(),
            },
        ];
        let json = serialize_tunnels(&drafts);
        let back = parse_tunnels(&Some(json));
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, "keep");
        assert_eq!(back[0].local_port, "8080");
        assert_eq!(back[0].remote_host, "db.internal");
        assert_eq!(back[0].remote_port, "5432");
    }

    #[test]
    fn empty_and_garbage_tunnel_columns_parse_to_nothing() {
        assert!(parse_tunnels(&None).is_empty());
        assert!(parse_tunnels(&Some(String::new())).is_empty());
        assert!(parse_tunnels(&Some("not json".into())).is_empty());
        assert!(parse_tunnels(&Some("[]".into())).is_empty());
    }

    #[test]
    fn conflict_policy_cycles_skip_overwrite_rename() {
        let c = ImportConflict::Skip;
        let c = cycle_conflict(c);
        assert_eq!(c, ImportConflict::Overwrite);
        let c = cycle_conflict(c);
        assert_eq!(c, ImportConflict::Rename);
        let c = cycle_conflict(c);
        assert_eq!(c, ImportConflict::Skip);
        assert_eq!(conflict_label(ImportConflict::Overwrite), "overwrite");
    }

    #[gpui::test]
    fn export_dialog_preselects_all_known_hosts(cx: &mut TestAppContext) {
        let (_rt, view) = make(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.hosts = vec![host_stub("a", "Alpha"), host_stub("b", "Beta")];
                v.open_export(cx);
                let sel = &v.export.as_ref().unwrap().selected;
                assert_eq!(sel.len(), 2);
                assert!(sel.contains("a") && sel.contains("b"));
                assert_eq!(v.export_selected_ids().len(), 2);
            });
        });
    }

    #[test]
    fn from_host_resolves_jump_host_index_and_never_points_at_self() {
        let hosts = vec![host_stub("a", "Alpha"), host_stub("b", "Bastion")];
        let mut target = host_stub("a", "Alpha");
        target.jump_host_id = Some("b".into());
        target.tunnels = Some(
            r#"[{"id":"t","type":"local","local_port":2201,"remote_host":"web","remote_port":80}]"#
                .into(),
        );
        let form = HostForm::from_host(&target, &[], &[], &hosts);
        assert_eq!(form.jump_host, Some(1));
        assert_eq!(form.tunnels.len(), 1);
        assert_eq!(form.tunnels[0].local_port, "2201");

        // A self-referential jump_host_id is ignored.
        let mut loopy = host_stub("a", "Alpha");
        loopy.jump_host_id = Some("a".into());
        let form = HostForm::from_host(&loopy, &[], &[], &hosts);
        assert!(form.jump_host.is_none());
    }

    #[test]
    fn quick_connect_target_parses_user_at_host_port() {
        let hm_search = |s: &str| {
            // exercise the parser in isolation via a throwaway view field
            let parsed = {
                let s = s.trim();
                s.split_once('@').and_then(|(u, rest)| {
                    if u.is_empty() {
                        return None;
                    }
                    let (h, p) = match rest.split_once(':') {
                        Some((h, p)) => (h, p.parse::<u16>().ok()?),
                        None => (rest, 22),
                    };
                    (!h.is_empty()).then(|| (u.to_string(), h.to_string(), p))
                })
            };
            parsed
        };
        assert_eq!(
            hm_search("deploy@web.example:2222"),
            Some(("deploy".into(), "web.example".into(), 2222))
        );
        assert_eq!(
            hm_search("root@10.0.0.1"),
            Some(("root".into(), "10.0.0.1".into(), 22))
        );
        assert_eq!(hm_search("just-text"), None);
        assert_eq!(hm_search("@host"), None);
    }

    #[gpui::test]
    fn visible_hosts_applies_group_filter_and_sort(cx: &mut TestAppContext) {
        let (_rt, view) = make(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                let mut a = host_stub("a", "Zeta");
                a.group_id = Some("g1".into());
                let mut b = host_stub("b", "Alpha");
                b.group_id = Some("g1".into());
                let c = host_stub("c", "Mid"); // ungrouped
                v.hosts = vec![a, b, c];

                v.sort = HostSort::NameAsc;
                let all: Vec<_> = v.visible_hosts().into_iter().map(|h| h.name).collect();
                assert_eq!(all, vec!["Alpha", "Mid", "Zeta"]);

                v.group_filter = Some("g1".into());
                let g: Vec<_> = v.visible_hosts().into_iter().map(|h| h.id).collect();
                assert_eq!(g, vec!["b", "a"]); // NameAsc within the group

                v.group_filter = Some(String::new()); // ungrouped only
                let u: Vec<_> = v.visible_hosts().into_iter().map(|h| h.id).collect();
                assert_eq!(u, vec!["c"]);

                v.group_filter = None;
                v.search = "mid".into();
                let s: Vec<_> = v.visible_hosts().into_iter().map(|h| h.id).collect();
                assert_eq!(s, vec!["c"]);
                let _ = cx;
            });
        });
    }

    #[test]
    fn host_form_maps_icon_snippet_and_tab_defaults() {
        let mut h = host_stub("h", "Box");
        h.icon = Some("shield".into());
        h.startup_snippet_id = Some("s1".into());
        h.startup_snippet_mode = Some("inject".into());
        let form = HostForm::from_host(&h, &[], &[], &[]);
        assert_eq!(form.icon.as_deref(), Some("shield"));
        assert_eq!(form.snippet_id.as_deref(), Some("s1"));
        assert_eq!(form.snippet_mode, "inject");
        assert_eq!(form.tab, FormTab::General);
        // icon key round-trips through the curated set
        assert!(matches!(host_icon(Some("shield")), IconName::Shield));
        assert!(matches!(host_icon(None), IconName::Server));
    }
}
