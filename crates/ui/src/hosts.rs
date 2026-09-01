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
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::credentials::{self, Credential};
use labonair_backend::modules::hosts::{self, Group, Host};
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
    focus: HostField,
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
            focus: HostField::Name,
        }
    }

    fn from_host(h: &Host, groups: &[Group], creds: &[Credential]) -> Self {
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
            focus: HostField::Name,
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
        }
    }
}

/// New-credential draft inside the credential manager.
struct CredDraft {
    name: String,
    is_key: bool,
}

pub struct HostManagerView {
    app: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    hosts: Vec<Host>,
    groups: Vec<Group>,
    credentials: Vec<Credential>,
    statuses: Vec<(String, HostStatus)>,
    collapsed: HashSet<String>,
    form: Option<HostForm>,
    form_focus: FocusHandle,
    creds_open: bool,
    cred_draft: Option<CredDraft>,
    cred_focus: FocusHandle,
    /// Inline "new group" buffer, `Some` while the field is open.
    group_draft: Option<String>,
    group_focus: FocusHandle,
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
            collapsed: HashSet::new(),
            form: None,
            form_focus: cx.focus_handle(),
            creds_open: false,
            cred_draft: None,
            cred_focus: cx.focus_handle(),
            group_draft: None,
            group_focus: cx.focus_handle(),
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
                        Some(name),                                // name
                        Some(addr),                                // host_address
                        Some(port),                                // port
                        Some(user),                                // username
                        Some(auth),                                // auth_method
                        key_path,                                  // private_key_path
                        group_id,                                  // group_id
                        tags,                                      // tags
                        password,                                  // password
                        None,                                      // sudo_password
                        default_path,                              // default_path_ssh
                        None,                                      // default_path_sftp
                        None,                                      // pin_to_top
                        None,                                      // keep_alive_interval
                        None,                                      // keep_alive_tries
                        None,                                      // sort_order
                        None,                                      // tunnels
                        None,                                      // startup_snippet_id
                        None,                                      // startup_snippet_mode
                        Some(cred_id.clone().unwrap_or_default()), // credential_id ("" clears)
                        None,                                      // jump_host_id
                        None,                                      // notes
                        None,                                      // icon
                        None,                                      // block_agent_access
                    )
                    .await;
                }
                None => {
                    let _ = hosts::db::hosts_create(
                        app.clone(),
                        &app.db,
                        &app.secrets,
                        name,         // name
                        addr,         // host_address
                        port,         // port
                        user,         // username
                        auth,         // auth_method
                        key_path,     // private_key_path
                        group_id,     // group_id
                        tags,         // tags
                        password,     // password
                        None,         // sudo_password
                        default_path, // default_path_ssh
                        None,         // default_path_sftp
                        None,         // pin_to_top
                        None,         // keep_alive_interval
                        None,         // keep_alive_tries
                        None,         // sort_order
                        None,         // tunnels
                        None,         // startup_snippet_id
                        None,         // startup_snippet_mode
                        cred_id,      // credential_id
                        None,         // jump_host_id
                        None,         // notes
                        None,         // icon
                        None,         // block_agent_access
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
                self.btn("host-edit", "Edit", p, false)
                    .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                        if let Some(h) = this.hosts.iter().find(|h| h.id == id_e).cloned() {
                            this.form =
                                Some(HostForm::from_host(&h, &this.groups, &this.credentials));
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

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000099))
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
            .bg(gpui::rgba(0x00000099))
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
            );

        let form_overlay = self
            .form
            .is_some()
            .then(|| self.render_form(&p, cx).into_any_element());
        let cred_overlay = self
            .creds_open
            .then(|| self.render_credentials(&p, cx).into_any_element());

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
                    .children(blocks),
            )
            .children(form_overlay)
            .children(cred_overlay)
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
        let form = HostForm::from_host(&host, &[], &[]);
        assert_eq!(form.name, "Web");
        assert_eq!(form.port, "2222");
        assert_eq!(form.auth, AuthMethod::Key);
        assert_eq!(form.key_path, "/k");
        assert_eq!(form.default_path, "/srv");
        assert_eq!(form.editing_id.as_deref(), Some("h"));
    }
}
