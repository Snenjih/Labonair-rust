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

use std::collections::HashSet;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::credentials::{self, Credential};
use labonair_backend::modules::hosts::{self, Group, Host};
use labonair_backend::modules::ssh::config_parser::{self, ImportConflict, SshConfigEntry};
use labonair_backend::App as Backend;
use tokio::runtime::Handle as TokioHandle;

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
    fn glyph(self) -> &'static str {
        match self {
            HostStatus::Disconnected => "\u{25cb}", // ○
            HostStatus::Connecting => "\u{25d0}",   // ◐
            HostStatus::Connected => "\u{25cf}",    // ●
            HostStatus::Failed => "\u{26a0}",       // ⚠
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMethod {
    Password,
    Key,
    Agent,
    None,
}

impl AuthMethod {
    const ALL: [AuthMethod; 4] = [
        AuthMethod::Password,
        AuthMethod::Key,
        AuthMethod::Agent,
        AuthMethod::None,
    ];
    fn as_str(self) -> &'static str {
        match self {
            AuthMethod::Password => "password",
            AuthMethod::Key => "key",
            AuthMethod::Agent => "agent",
            AuthMethod::None => "none",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "key" => AuthMethod::Key,
            "agent" => AuthMethod::Agent,
            "none" => AuthMethod::None,
            _ => AuthMethod::Password,
        }
    }
    fn title(self) -> &'static str {
        match self {
            AuthMethod::Password => "Password",
            AuthMethod::Key => "SSH Key",
            AuthMethod::Agent => "Agent",
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
    Tags,
    Password,
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
struct HostForm {
    editing_id: Option<String>,
    name: String,
    address: String,
    port: String,
    username: String,
    auth: AuthMethod,
    key_path: String,
    default_path: String,
    tags: String,
    password: String,
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
    focus: HostField,
    /// Fallback edit target for a stale tunnel-field index (never rendered).
    scratch: String,
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
            tags: String::new(),
            password: String::new(),
            credential: None,
            group: None,
            jump_host: None,
            tunnels: Vec::new(),
            block_agent_access: false,
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
            tags: h.tags.clone().unwrap_or_default(),
            password: String::new(),
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
            HostField::Tags => &mut self.tags,
            HostField::Password => &mut self.password,
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

pub struct HostManagerView {
    app: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    hosts: Vec<Host>,
    groups: Vec<Group>,
    credentials: Vec<Credential>,
    statuses: Vec<(String, HostStatus)>,
    active_tunnels: Vec<ActiveTunnelRow>,
    collapsed: HashSet<String>,
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
        let this = Self {
            app,
            tokio,
            theme,
            hosts: Vec::new(),
            groups: Vec::new(),
            credentials: Vec::new(),
            statuses: Vec::new(),
            active_tunnels: Vec::new(),
            collapsed: HashSet::new(),
            form: None,
            form_focus: cx.focus_handle(),
            creds_open: false,
            cred_draft: None,
            cred_focus: cx.focus_handle(),
            group_draft: None,
            group_focus: cx.focus_handle(),
            import: None,
            export: None,
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
            (hosts, groups, creds)
        });
        cx.spawn(async move |this, cx| {
            if let Ok((h, g, c)) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    this.hosts = h;
                    this.groups = g;
                    this.credentials = c;
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

    fn submit_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.form.take() else { return };
        let app = self.app.clone();
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
        let tags = (!form.tags.trim().is_empty()).then(|| form.tags.trim().to_string());
        let password = (!form.password.is_empty()).then(|| form.password.clone());
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
                        Some(name),                                     // name
                        Some(addr),                                     // host_address
                        Some(port),                                     // port
                        Some(user),                                     // username
                        Some(auth),                                     // auth_method
                        key_path,                                       // private_key_path
                        group_id,                                       // group_id
                        tags,                                           // tags
                        password,                                       // password
                        None,                                           // sudo_password
                        default_path,                                   // default_path_ssh
                        None,                                           // default_path_sftp
                        None,                                           // pin_to_top
                        None,                                           // keep_alive_interval
                        None,                                           // keep_alive_tries
                        None,                                           // sort_order
                        Some(tunnels_json),                             // tunnels
                        None,                                           // startup_snippet_id
                        None,                                           // startup_snippet_mode
                        Some(cred_id.clone().unwrap_or_default()),      // credential_id ("" clears)
                        Some(jump_host_id.clone().unwrap_or_default()), // jump_host_id ("" clears)
                        None,                                           // notes
                        None,                                           // icon
                        Some(block_agent_access),                       // block_agent_access
                    )
                    .await;
                }
                None => {
                    let _ = hosts::db::hosts_create(
                        app.clone(),
                        &app.db,
                        &app.secrets,
                        name,                     // name
                        addr,                     // host_address
                        port,                     // port
                        user,                     // username
                        auth,                     // auth_method
                        key_path,                 // private_key_path
                        group_id,                 // group_id
                        tags,                     // tags
                        password,                 // password
                        None,                     // sudo_password
                        default_path,             // default_path_ssh
                        None,                     // default_path_sftp
                        None,                     // pin_to_top
                        None,                     // keep_alive_interval
                        None,                     // keep_alive_tries
                        None,                     // sort_order
                        Some(tunnels_json),       // tunnels
                        None,                     // startup_snippet_id
                        None,                     // startup_snippet_mode
                        cred_id,                  // credential_id
                        jump_host_id,             // jump_host_id
                        None,                     // notes
                        None,                     // icon
                        Some(block_agent_access), // block_agent_access
                    )
                    .await;
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
        cx.notify();
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
                self.form = None;
            }
            "enter" => self.submit_form(cx),
            _ => {
                if let Some(form) = self.form.as_mut() {
                    let f = form.focus;
                    Self::edit_str(form.field_mut(f), ks);
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

    fn btn(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        p: &Palette,
        primary: bool,
    ) -> gpui::Stateful<gpui::Div> {
        let base = div()
            .id(id)
            .px_2()
            .py_1()
            .rounded_md()
            .text_xs()
            .cursor_pointer();
        if primary {
            base.bg(p.accent)
                .text_color(p.fg)
                .hover(|s| s.opacity(0.85))
        } else {
            base.text_color(p.muted)
                .border_1()
                .border_color(p.border)
                .hover(|s| s.bg(p.border).text_color(p.fg))
        }
        .child(label.into())
    }

    fn render_host_row(
        &self,
        host: &Host,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let status = self.status_of(&host.id);
        let id = host.id.clone();
        let (id_c, id_e, id_d, id_x) = (id.clone(), id.clone(), id.clone(), id.clone());
        let id_s = id.clone();
        let subtitle = format!(
            "{}@{}:{}  \u{00b7}  {}",
            host.username,
            host.host_address,
            host.port,
            status.label()
        );
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(p.card)
            .border_1()
            .border_color(p.border)
            .child(
                div()
                    .w(px(14.0))
                    .text_color(match status {
                        HostStatus::Connected => p.accent,
                        HostStatus::Failed => p.fg,
                        _ => p.muted,
                    })
                    .child(status.glyph()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_sm()
                            .text_color(p.fg)
                            .child(SharedString::from(host.name.clone())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(p.muted)
                            .child(SharedString::from(subtitle)),
                    ),
            )
            .child(
                self.btn("host-connect", "Connect", p, true)
                    .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                        cx.emit(HostManagerEvent::Connect(id_c.clone()));
                    })),
            )
            .child(
                self.btn("host-sftp", "SFTP", p, false)
                    .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                        cx.emit(HostManagerEvent::OpenSftp(id_s.clone()));
                    })),
            )
            .child(
                self.btn("host-edit", "Edit", p, false)
                    .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                        if let Some(h) = this.hosts.iter().find(|h| h.id == id_e).cloned() {
                            this.form = Some(HostForm::from_host(
                                &h,
                                &this.groups,
                                &this.credentials,
                                &this.hosts,
                            ));
                            w.focus(&this.form_focus);
                            cx.notify();
                        }
                    })),
            )
            .child(
                self.btn("host-dup", "Duplicate", p, false)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.duplicate_host(id_d.clone(), cx)
                    })),
            )
            .child(self.btn("host-del", "Delete", p, false).on_click(
                cx.listener(move |this, _: &ClickEvent, _w, cx| this.delete_host(id_x.clone(), cx)),
            ))
    }

    fn render_group_block(
        &self,
        group: Option<&Group>,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (gid, gname): (Option<String>, String) = match group {
            Some(g) => (Some(g.id.clone()), g.name.clone()),
            None => (None, "Ungrouped".to_string()),
        };
        let key = gid.clone().unwrap_or_default();
        let collapsed = self.collapsed.contains(&key);
        let rows: Vec<Host> = self
            .hosts
            .iter()
            .filter(|h| h.group_id.clone().unwrap_or_default() == key)
            .cloned()
            .collect();
        if rows.is_empty() && group.is_some() {
            // still show empty named groups so they can be managed
        }
        let key_toggle = key.clone();
        let header = div()
            .flex()
            .items_center()
            .gap_2()
            .py_1()
            .child(
                div()
                    .id("group-toggle")
                    .cursor_pointer()
                    .text_xs()
                    .text_color(p.muted)
                    .child(if collapsed { "\u{25b8}" } else { "\u{25be}" })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if !this.collapsed.remove(&key_toggle) {
                            this.collapsed.insert(key_toggle.clone());
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .text_color(p.muted)
                    .child(SharedString::from(format!(
                        "{}  ({})",
                        gname.to_uppercase(),
                        rows.len()
                    ))),
            )
            .when_some(gid.clone(), |el, id| {
                el.child(
                    self.btn("group-del", "Delete group", p, false)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.delete_group(id.clone(), cx)
                        })),
                )
            });

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(header)
            .when(!collapsed, |el| {
                el.children(
                    rows.iter()
                        .map(|h| self.render_host_row(h, p, cx).into_any_element()),
                )
            })
            .into_any_element()
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

    fn render_form(&self, p: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(form) = self.form.as_ref() else {
            return div().into_any_element();
        };
        let title = if form.editing_id.is_some() {
            "Edit Host"
        } else {
            "New Host"
        };
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

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                div()
                    .track_focus(&self.form_focus)
                    .key_context("HostForm")
                    .on_key_down(cx.listener(Self::on_form_key))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w(px(460.0))
                    .max_h(px(620.0))
                    .overflow_hidden()
                    .p_4()
                    .rounded_lg()
                    .bg(p.card)
                    .border_1()
                    .border_color(p.border)
                    .child(div().text_sm().text_color(p.fg).child(title))
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
                                                AuthMethod::Agent => "auth-agent",
                                                AuthMethod::None => "auth-none",
                                            },
                                            m.title(),
                                            p,
                                            m == auth,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                                if let Some(f) = this.form.as_mut() {
                                                    f.auth = m;
                                                }
                                                cx.notify();
                                            }),
                                        )
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
                    .child(self.labelled_field(
                        "Start directory",
                        &form.default_path,
                        HostField::DefaultPath,
                        form.focus == HostField::DefaultPath,
                        p,
                        cx,
                    ))
                    .child(self.labelled_field(
                        "Tags (comma separated)",
                        &form.tags,
                        HostField::Tags,
                        form.focus == HostField::Tags,
                        p,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                self.btn(
                                    "cred-cycle",
                                    format!("Credential: {cred_label}"),
                                    p,
                                    false,
                                )
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _w, cx| {
                                        let n = this.credentials.len();
                                        if let Some(f) = this.form.as_mut() {
                                            f.credential = match f.credential {
                                                None if n > 0 => Some(0),
                                                Some(i) if i + 1 < n => Some(i + 1),
                                                _ => None,
                                            };
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
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
                                        cx.notify();
                                    })),
                            ),
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
                                            .filter(|(_, h)| {
                                                Some(h.id.as_str()) != editing.as_deref()
                                            })
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
                                        cx.notify();
                                    })),
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
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(self.render_tunnels_section(form, p, cx))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .pt_2()
                            .child(self.btn("form-cancel", "Cancel", p, false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.form = None;
                                    cx.notify();
                                }),
                            ))
                            .child(self.btn("form-save", "Save", p, true).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_form(cx)),
                            )),
                    ),
            )
            .into_any_element()
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
                    .child(div().w(px(14.0)).text_color(p.accent).child(if checked {
                        "\u{2611}"
                    } else {
                        "\u{2610}"
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
                    .child(div().w(px(14.0)).text_color(p.accent).child(if checked {
                        "\u{2611}"
                    } else {
                        "\u{2610}"
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

        // Group blocks: ungrouped first, then each named group.
        let mut blocks: Vec<gpui::AnyElement> = Vec::new();
        blocks.push(self.render_group_block(None, &p, cx));
        let groups = self.groups.clone();
        for g in &groups {
            blocks.push(self.render_group_block(Some(g), &p, cx));
        }

        let toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().text_sm().text_color(p.fg).child("Hosts"))
            .child(
                self.btn("new-host", "New Host", &p, true)
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.form = Some(HostForm::blank());
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
                    .border_color(p.accent)
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
                self.btn("open-creds", "Credentials", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.creds_open = true;
                        cx.notify();
                    })),
            )
            .child(
                self.btn("import-ssh-config", "Import SSH config", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.open_import(cx))),
            )
            .child(
                self.btn("export-ssh-config", "Export SSH config", &p, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.open_export(cx))),
            );

        let tunnels_panel = (!self.active_tunnels.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
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

        let form_overlay = self
            .form
            .is_some()
            .then(|| self.render_form(&p, cx).into_any_element());
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
                div()
                    .id("host-manager-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(toolbar)
                    .children(tunnels_panel)
                    .children(blocks),
            )
            .children(form_overlay)
            .children(cred_overlay)
            .children(import_overlay)
            .children(export_overlay)
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
}
