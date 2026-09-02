//! Command-Snippets system (T12-001).
//!
//! Port of `reference-src/src/modules/snippets/*`: reusable command templates
//! with optional `${VAR_NAME}` prompts, executed either locally (a new terminal
//! tab) or over SSH (a chosen host), plus a run-log drawer for the "silent"
//! execution mode.
//!
//! * Pure helpers ([`extract_snippet_variables`], [`substitute_snippet_variables`],
//!   [`parse_tags`], [`serialize_tags`]) mirror `lib/snippetVariables.ts` /
//!   `lib/snippetUtils.ts` and carry their test suites.
//! * [`SnippetsView`] is the GPUI sidebar panel — grouped list + search, the
//!   create/edit form, the variable-prompt and host-picker modals and the log
//!   drawer. CRUD/groups/reorder persist through
//!   `labonair_backend::modules::snippets::db`; execution is delegated to
//!   [`crate::workspace::Workspace`] (terminal / inject) or
//!   `modules::snippets::exec` (silent).

use std::collections::{HashMap, HashSet};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use labonair_backend::modules::hosts::{self, Host};
use labonair_backend::modules::snippets::db as sdb;
use labonair_backend::modules::snippets::exec::{
    snippet_run_cancel, snippet_run_local, snippet_run_ssh,
};
use labonair_backend::modules::snippets::{CommandSnippet, SnippetGroup, SnippetReorderItem};
use labonair_backend::App as Backend;
use tokio::runtime::Handle as TokioHandle;

use crate::components::IconName;
use crate::notifications::{notification_center, Notification};
use crate::theme::ThemeStore;
use crate::workspace::Workspace;

// ── Pure helpers: variable extraction / substitution ─────────────────────────

/// A `${VAR_NAME}` (or `${VAR_NAME:-default}`) placeholder found in a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetVariable {
    pub name: String,
    pub default_value: Option<String>,
}

/// Shell / POSIX environment variable names excluded from placeholder
/// extraction even though they match the `${UPPER_SNAKE_CASE}` pattern — a user
/// writing `${PATH}` almost always wants the real shell variable, not a prompt.
/// (Verbatim port of `SHELL_RESERVED_VAR_NAMES` in `snippetVariables.ts`.)
const SHELL_RESERVED_VAR_NAMES: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "PWD",
    "OLDPWD",
    "TERM",
    "TMPDIR",
    "TZ",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "DISPLAY",
    "MAIL",
    "EDITOR",
    "VISUAL",
    "CDPATH",
    "PS1",
    "PS2",
    "PS3",
    "PS4",
    "IFS",
    "RANDOM",
    "SECONDS",
    "LINENO",
    "PPID",
    "UID",
    "EUID",
    "SHLVL",
    "HISTFILE",
    "HISTSIZE",
    "HISTCONTROL",
    "BASH",
    "BASH_VERSION",
    "ZSH_VERSION",
    "FUNCNAME",
    "OSTYPE",
    "HOSTTYPE",
    "MACHTYPE",
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
    "SSH_CONNECTION",
    "SSH_CLIENT",
    "SSH_TTY",
];

/// One matched `${…}` occurrence: byte range in the source, name, optional
/// default. Hand-rolled scanner replacing the JS regex
/// `/\$\{([A-Z_][A-Z0-9_]*)(?::-([^}]*))?\}/g` (no `regex` dep in this crate).
struct VarMatch {
    start: usize,
    end: usize,
    name: String,
    default_value: Option<String>,
}

fn scan_variables(command: &str) -> Vec<VarMatch> {
    let bytes = command.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] != b'$' || bytes[i + 1] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let name_start = i + 2;
        let mut j = name_start;
        let is_head = |b: u8| b.is_ascii_uppercase() || b == b'_';
        let is_tail = |b: u8| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_';
        if j >= bytes.len() || !is_head(bytes[j]) {
            i += 1;
            continue;
        }
        j += 1;
        while j < bytes.len() && is_tail(bytes[j]) {
            j += 1;
        }
        let name = command[name_start..j].to_string();
        if j < bytes.len() && bytes[j] == b'}' {
            out.push(VarMatch {
                start,
                end: j + 1,
                name,
                default_value: None,
            });
            i = j + 1;
            continue;
        }
        if j + 1 < bytes.len() && bytes[j] == b':' && bytes[j + 1] == b'-' {
            let d_start = j + 2;
            let mut k = d_start;
            while k < bytes.len() && bytes[k] != b'}' {
                k += 1;
            }
            if k < bytes.len() {
                out.push(VarMatch {
                    start,
                    end: k + 1,
                    name,
                    default_value: Some(command[d_start..k].to_string()),
                });
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Extracts the unique `${VAR_NAME}` / `${VAR_NAME:-default}` placeholders from
/// a command, in first-occurrence order. Duplicates collapse to their first
/// occurrence (its default wins); reserved shell names are skipped.
pub fn extract_snippet_variables(command: &str) -> Vec<SnippetVariable> {
    let mut out: Vec<SnippetVariable> = Vec::new();
    for m in scan_variables(command) {
        if SHELL_RESERVED_VAR_NAMES.contains(&m.name.as_str()) {
            continue;
        }
        if out.iter().any(|v| v.name == m.name) {
            continue;
        }
        out.push(SnippetVariable {
            name: m.name,
            default_value: m.default_value,
        });
    }
    out
}

/// Substitutes resolved values back into a command. Placeholders whose name is
/// absent from `values` (reserved names, or anything not prompted) are left
/// untouched — matching `substituteSnippetVariables` in the reference, which
/// performs a raw textual replacement (no shell-quoting).
pub fn substitute_snippet_variables(command: &str, values: &HashMap<String, String>) -> String {
    let matches = scan_variables(command);
    if matches.is_empty() {
        return command.to_string();
    }
    let mut result = String::with_capacity(command.len());
    let mut cursor = 0;
    for m in matches {
        result.push_str(&command[cursor..m.start]);
        match values.get(&m.name) {
            Some(v) => result.push_str(v),
            None => result.push_str(&command[m.start..m.end]),
        }
        cursor = m.end;
    }
    result.push_str(&command[cursor..]);
    result
}

/// Parses a snippet's `tags` column (a JSON array string) into a `Vec<String>`.
/// Any non-array / invalid JSON yields an empty vec — port of `parseTags`.
pub fn parse_tags(tags_json: Option<&str>) -> Vec<String> {
    let Some(raw) = tags_json.filter(|s| !s.is_empty()) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

/// Serializes tags back to a JSON array string, or `None` when empty — port of
/// `serializeTags`.
pub fn serialize_tags(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        return None;
    }
    serde_json::to_string(tags).ok()
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn new_run_id() -> String {
    format!("run_{}_{}", now_millis(), uuid::Uuid::new_v4().simple())
}

// ── Run log ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Running,
    Done,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct RunLine {
    pub data: String,
    pub is_err: bool,
}

/// One recorded snippet execution — mirrors `SnippetRunLog` in the reference.
#[derive(Debug, Clone)]
pub struct SnippetRunLog {
    pub run_id: String,
    pub snippet_name: String,
    pub started_at: i64,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub lines: Vec<RunLine>,
}

/// Newest-first, capped at 50 entries (reference `addRunLog` slice(0, 50)).
fn push_run_log(logs: &mut Vec<SnippetRunLog>, log: SnippetRunLog) {
    logs.insert(0, log);
    logs.truncate(50);
}

// ── View ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    Terminal,
    Silent,
    Inject,
}

impl ExecMode {
    fn as_str(self) -> &'static str {
        match self {
            ExecMode::Terminal => "terminal",
            ExecMode::Silent => "silent",
            ExecMode::Inject => "inject",
        }
    }
    fn from_str(s: &str) -> Self {
        match s {
            "silent" => ExecMode::Silent,
            "inject" => ExecMode::Inject,
            _ => ExecMode::Terminal,
        }
    }
    fn label(self) -> &'static str {
        match self {
            ExecMode::Terminal => "Terminal",
            ExecMode::Silent => "Silent",
            ExecMode::Inject => "Inject",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Name,
    Description,
    Command,
    WorkingDir,
    GroupName,
    Search,
    VarValue,
    Host,
}

#[derive(Clone)]
struct FormState {
    id: Option<String>,
    name: String,
    description: String,
    command: String,
    target_ssh: bool,
    host_id: String,
    mode: ExecMode,
    working_dir: String,
    group_id: String,
    sort_order: i64,
}

impl FormState {
    fn empty() -> Self {
        Self {
            id: None,
            name: String::new(),
            description: String::new(),
            command: String::new(),
            target_ssh: false,
            host_id: String::new(),
            mode: ExecMode::Terminal,
            working_dir: String::new(),
            group_id: String::new(),
            sort_order: 0,
        }
    }
    fn from_snippet(s: &CommandSnippet) -> Self {
        Self {
            id: Some(s.id.clone()),
            name: s.name.clone(),
            description: s.description.clone().unwrap_or_default(),
            command: s.command.clone(),
            target_ssh: s.target == "ssh",
            host_id: s.host_id.clone().unwrap_or_default(),
            mode: ExecMode::from_str(&s.default_exec_mode),
            working_dir: s.working_dir.clone().unwrap_or_default(),
            group_id: s.group_id.clone().unwrap_or_default(),
            sort_order: s.sort_order,
        }
    }
}

/// A run held back until the user answers the variable-prompt / host-picker.
struct PendingRun {
    snippet: CommandSnippet,
    mode: ExecMode,
}

struct VarPrompt {
    pending: PendingRun,
    vars: Vec<SnippetVariable>,
    values: Vec<(String, String)>,
    active: usize,
}

struct HostPicker {
    pending: PendingRun,
    /// `command` already has its `${VAR}`s resolved.
    command: String,
    selected: String,
}

#[derive(Clone)]
struct Colors {
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    accent: gpui::Hsla,
    error: gpui::Hsla,
    warning: gpui::Hsla,
}

/// Backend → view snippet-run events, forwarded off the broadcast bus.
enum RunEvent {
    Output {
        run_id: String,
        data: String,
        is_err: bool,
    },
    Done {
        run_id: String,
        exit_code: i32,
        cancelled: bool,
    },
}

fn parse_run_event(name: &str, payload: &serde_json::Value) -> Option<RunEvent> {
    match name {
        "snippet_run_output" => Some(RunEvent::Output {
            run_id: payload.get("runId")?.as_str()?.to_string(),
            data: payload.get("data")?.as_str()?.to_string(),
            is_err: payload.get("stream").and_then(|v| v.as_str()) == Some("stderr"),
        }),
        "snippet_run_done" => Some(RunEvent::Done {
            run_id: payload.get("runId")?.as_str()?.to_string(),
            exit_code: payload
                .get("exitCode")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1) as i32,
            cancelled: payload
                .get("cancelled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }),
        _ => None,
    }
}

pub struct SnippetsView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    workspace: Entity<Workspace>,
    focus: FocusHandle,

    snippets: Vec<CommandSnippet>,
    groups: Vec<SnippetGroup>,
    hosts: Vec<Host>,
    run_logs: Vec<SnippetRunLog>,

    query: String,
    search_open: bool,
    collapsed_groups: HashSet<String>,
    /// `Some` while the create/edit form is on screen.
    form: Option<FormState>,
    adding_group: bool,
    group_name_buf: String,
    active_field: Option<Field>,

    var_prompt: Option<VarPrompt>,
    host_picker: Option<HostPicker>,

    log_open: bool,
    selected_run: Option<String>,

    run_events: std::sync::mpsc::Receiver<RunEvent>,
    _poll: gpui::Task<()>,
}

impl Focusable for SnippetsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SnippetsView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        workspace: Entity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        // Forward snippet-run events off the broadcast bus into a plain channel.
        let (tx, rx) = std::sync::mpsc::channel::<RunEvent>();
        {
            let mut bus = backend.events.subscribe();
            tokio.spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match bus.recv().await {
                        Ok(raw) => {
                            if let Some(ev) = parse_run_event(&raw.name, &raw.payload) {
                                if tx.send(ev).is_err() {
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

        let poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(60))
                .await;
            let ok = this
                .update(cx, |this, cx| {
                    let mut evs = Vec::new();
                    while let Ok(ev) = this.run_events.try_recv() {
                        evs.push(ev);
                    }
                    if !evs.is_empty() {
                        for ev in evs {
                            this.apply_run_event(ev);
                        }
                        cx.notify();
                    }
                })
                .is_ok();
            if !ok {
                break;
            }
        });

        let this = Self {
            backend,
            tokio,
            theme,
            workspace,
            focus: cx.focus_handle(),
            snippets: Vec::new(),
            groups: Vec::new(),
            hosts: Vec::new(),
            run_logs: Vec::new(),
            query: String::new(),
            search_open: false,
            collapsed_groups: HashSet::new(),
            form: None,
            adding_group: false,
            group_name_buf: String::new(),
            active_field: None,
            var_prompt: None,
            host_picker: None,
            log_open: false,
            selected_run: None,
            run_events: rx,
            _poll: poll,
        };
        this.reload(cx);
        this
    }

    // ── data ──────────────────────────────────────────────────────────────

    /// Reload snippets / groups / hosts from the backend.
    pub fn reload(&self, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let jh = self.tokio.spawn(async move {
            let snippets = sdb::snippets_get_all(&app.db).await.unwrap_or_default();
            let groups = sdb::snippet_groups_get_all(&app.db)
                .await
                .unwrap_or_default();
            let hosts = hosts::db::hosts_get_all(&app.db).await.unwrap_or_default();
            (snippets, groups, hosts)
        });
        cx.spawn(async move |this, cx| {
            if let Ok((s, g, h)) = jh.await {
                let _ = this.update(cx, |this, cx| {
                    this.snippets = s;
                    this.groups = g;
                    this.hosts = h;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn host_name(&self, id: &str) -> Option<String> {
        self.hosts
            .iter()
            .find(|h| h.id == id)
            .map(|h| h.name.clone())
    }

    fn apply_run_event(&mut self, ev: RunEvent) {
        match ev {
            RunEvent::Output {
                run_id,
                data,
                is_err,
            } => {
                if let Some(log) = self.run_logs.iter_mut().find(|l| l.run_id == run_id) {
                    log.lines.push(RunLine { data, is_err });
                }
            }
            RunEvent::Done {
                run_id,
                exit_code,
                cancelled,
            } => {
                if let Some(log) = self.run_logs.iter_mut().find(|l| l.run_id == run_id) {
                    log.exit_code = Some(exit_code);
                    log.status = if cancelled {
                        RunStatus::Cancelled
                    } else if exit_code == 0 {
                        RunStatus::Done
                    } else {
                        RunStatus::Error
                    };
                }
            }
        }
    }

    // ── CRUD ──────────────────────────────────────────────────────────────

    fn save_form(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.form.clone() else {
            return;
        };
        if form.name.trim().is_empty() || form.command.trim().is_empty() {
            return;
        }
        let app = self.backend.clone();
        let opt = |s: String| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let name = form.name.trim().to_string();
        let description = opt(form.description.clone());
        let command = form.command.clone();
        let target = if form.target_ssh { "ssh" } else { "local" }.to_string();
        let host_id = (form.target_ssh && !form.host_id.is_empty()).then(|| form.host_id.clone());
        let mode = form.mode.as_str().to_string();
        let working_dir = if form.target_ssh {
            None
        } else {
            opt(form.working_dir.clone())
        };
        let group_id = (!form.group_id.is_empty()).then(|| form.group_id.clone());
        let sort_order = form.sort_order;
        let id = form.id.clone();

        let jh = self.tokio.spawn(async move {
            match id {
                None => sdb::snippets_create(
                    &app.db,
                    name,
                    command,
                    target,
                    description,
                    host_id,
                    Some(mode),
                    working_dir,
                    group_id,
                    None,
                    Some(sort_order),
                )
                .await
                .map(|_| ()),
                Some(id) => sdb::snippets_update(
                    &app.db,
                    id,
                    Some(name),
                    Some(command),
                    Some(target),
                    Some(description.unwrap_or_default()),
                    host_id,
                    Some(mode),
                    working_dir,
                    group_id,
                    None,
                    Some(sort_order),
                )
                .await
                .map(|_| ()),
            }
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| {
                this.form = None;
                this.active_field = None;
                this.reload(cx);
            });
        })
        .detach();
    }

    fn delete_snippet(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let jh = self
            .tokio
            .spawn(async move { sdb::snippets_delete(&app.db, id).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| {
                this.form = None;
                this.reload(cx);
            });
        })
        .detach();
    }

    fn duplicate_snippet(&mut self, s: &CommandSnippet, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let (name, command, target) = (
            format!("{} (copy)", s.name),
            s.command.clone(),
            s.target.clone(),
        );
        let (desc, host, mode, wd, group, tags, order) = (
            s.description.clone(),
            s.host_id.clone(),
            Some(s.default_exec_mode.clone()),
            s.working_dir.clone(),
            s.group_id.clone(),
            s.tags.clone(),
            Some(s.sort_order + 1),
        );
        let jh = self.tokio.spawn(async move {
            sdb::snippets_create(
                &app.db, name, command, target, desc, host, mode, wd, group, tags, order,
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    /// Persist the current on-screen order of a group's snippets (0,1,2,…) —
    /// the "reorder" path. Used by the row up/down buttons.
    fn move_snippet(&mut self, id: &str, delta: i64, cx: &mut Context<Self>) {
        let group_id = self
            .snippets
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.group_id.clone());
        let mut ordered: Vec<CommandSnippet> = self
            .snippets
            .iter()
            .filter(|s| s.group_id == group_id)
            .cloned()
            .collect();
        ordered.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
        let Some(pos) = ordered.iter().position(|s| s.id == id) else {
            return;
        };
        let target = pos as i64 + delta;
        if target < 0 || target as usize >= ordered.len() {
            return;
        }
        ordered.swap(pos, target as usize);
        let items: Vec<SnippetReorderItem> = ordered
            .iter()
            .enumerate()
            .map(|(i, s)| SnippetReorderItem {
                id: s.id.clone(),
                sort_order: i as i64,
            })
            .collect();
        let app = self.backend.clone();
        let jh = self
            .tokio
            .spawn(async move { sdb::snippets_reorder(&app.db, items).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    fn create_group(&mut self, name: String, cx: &mut Context<Self>) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let app = self.backend.clone();
        let jh = self
            .tokio
            .spawn(async move { sdb::snippet_groups_create(&app.db, name, None, None).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| {
                this.adding_group = false;
                this.group_name_buf.clear();
                this.reload(cx);
            });
        })
        .detach();
    }

    fn delete_group(&mut self, id: String, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let jh = self
            .tokio
            .spawn(async move { sdb::snippet_groups_delete(&app.db, id).await });
        cx.spawn(async move |this, cx| {
            let _ = jh.await;
            let _ = this.update(cx, |this, cx| this.reload(cx));
        })
        .detach();
    }

    // ── execution ─────────────────────────────────────────────────────────

    /// `(id, name, default exec mode)` for every loaded snippet — for the
    /// command palette's "Run Snippet…" sub-page.
    pub fn snippet_choices(&self) -> Vec<(String, String, String)> {
        self.snippets
            .iter()
            .map(|s| (s.id.clone(), s.name.clone(), s.default_exec_mode.clone()))
            .collect()
    }

    /// Run a snippet by id with its default execution mode (palette entry).
    pub fn run_by_id(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(snippet) = self.snippets.iter().find(|s| s.id == id).cloned() {
            self.run(snippet, None, window, cx);
        }
    }

    /// Entry point from a Run button / menu item.
    fn run(
        &mut self,
        snippet: CommandSnippet,
        mode: Option<ExecMode>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mode = mode.unwrap_or_else(|| ExecMode::from_str(&snippet.default_exec_mode));
        let vars = extract_snippet_variables(&snippet.command);
        if vars.is_empty() {
            let command = snippet.command.clone();
            self.dispatch(PendingRun { snippet, mode }, command, window, cx);
            return;
        }
        let values = vars
            .iter()
            .map(|v| (v.name.clone(), v.default_value.clone().unwrap_or_default()))
            .collect();
        self.var_prompt = Some(VarPrompt {
            pending: PendingRun { snippet, mode },
            vars,
            values,
            active: 0,
        });
        self.active_field = Some(Field::VarValue);
        cx.notify();
    }

    fn submit_var_prompt(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(prompt) = self.var_prompt.take() else {
            return;
        };
        self.active_field = None;
        let map: HashMap<String, String> = prompt.values.into_iter().collect();
        let command = substitute_snippet_variables(&prompt.pending.snippet.command, &map);
        self.dispatch(prompt.pending, command, window, cx);
    }

    /// Dispatch a run whose `${VAR}`s are already resolved into `command`.
    fn dispatch(
        &mut self,
        pending: PendingRun,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let PendingRun { snippet, mode } = pending;

        if mode == ExecMode::Inject {
            self.workspace
                .update(cx, |w, cx| w.inject_into_active_terminal(&command, cx));
            cx.notify();
            return;
        }

        // Resolve the SSH target host, if any.
        let host_id = if snippet.target == "ssh" {
            match snippet.host_id.as_deref() {
                Some(id) if self.hosts.iter().any(|h| h.id == id) => Some(id.to_string()),
                Some(_) => {
                    self.toast_err(
                        "Snippet host missing",
                        "This snippet's target host no longer exists — edit the snippet to pick a new host.",
                        cx,
                    );
                    return;
                }
                None => {
                    // Ask at runtime.
                    self.host_picker = Some(HostPicker {
                        pending: PendingRun { snippet, mode },
                        command,
                        selected: String::new(),
                    });
                    self.active_field = Some(Field::Host);
                    cx.notify();
                    return;
                }
            }
        } else {
            None
        };

        match mode {
            ExecMode::Inject => unreachable!(),
            ExecMode::Terminal => {
                if let Some(host_id) = host_id {
                    self.workspace.update(cx, |w, cx| {
                        w.run_snippet_ssh_terminal(host_id, command, window, cx)
                    });
                } else {
                    let cwd = (!snippet.working_dir.clone().unwrap_or_default().is_empty())
                        .then(|| snippet.working_dir.clone().unwrap());
                    self.workspace
                        .update(cx, |w, cx| w.run_snippet_local(cwd, command, window, cx));
                }
                cx.notify();
            }
            ExecMode::Silent => self.run_silent(&snippet, host_id, command, cx),
        }
    }

    fn run_silent(
        &mut self,
        snippet: &CommandSnippet,
        host_id: Option<String>,
        command: String,
        cx: &mut Context<Self>,
    ) {
        let run_id = new_run_id();
        push_run_log(
            &mut self.run_logs,
            SnippetRunLog {
                run_id: run_id.clone(),
                snippet_name: snippet.name.clone(),
                started_at: now_millis(),
                status: RunStatus::Running,
                exit_code: None,
                lines: Vec::new(),
            },
        );
        self.log_open = true;
        self.selected_run = Some(run_id.clone());

        let app = self.backend.clone();
        if let Some(host_id) = host_id {
            let Some(session_id) = self.workspace.read(cx).ssh_session_for_host(&host_id) else {
                self.fail_silent(
                    &run_id,
                    "No active SSH session for this host. Open a terminal tab first or use Terminal mode.",
                );
                cx.notify();
                return;
            };
            let state_app = app.clone();
            self.tokio.spawn(async move {
                let _ = snippet_run_ssh(
                    app.clone(),
                    run_id,
                    session_id,
                    command,
                    &state_app.ssh,
                    &state_app.snippet_run,
                )
                .await;
            });
        } else {
            let working_dir = snippet.working_dir.clone().filter(|s| !s.is_empty());
            let state_app = app.clone();
            self.tokio.spawn(async move {
                let _ = snippet_run_local(
                    app.clone(),
                    run_id,
                    command,
                    working_dir,
                    &state_app.snippet_run,
                )
                .await;
            });
        }
        cx.notify();
    }

    fn fail_silent(&mut self, run_id: &str, message: &str) {
        if let Some(log) = self.run_logs.iter_mut().find(|l| l.run_id == run_id) {
            log.status = RunStatus::Error;
            log.lines.push(RunLine {
                data: format!("{message}\n"),
                is_err: true,
            });
        }
    }

    fn cancel_run(&mut self, run_id: String, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let rid = run_id.clone();
        self.tokio.spawn(async move {
            let _ = snippet_run_cancel(rid, &app.snippet_run).await;
        });
        let _ = cx;
    }

    fn toast_err(&self, title: &'static str, message: &'static str, cx: &mut Context<Self>) {
        let center = notification_center(cx);
        center.update(cx, |c, cx| {
            c.push(Notification::error(title, message), cx);
        });
    }

    // ── key handling ──────────────────────────────────────────────────────

    fn field_buf_mut(&mut self, f: Field) -> Option<&mut String> {
        match f {
            Field::Search => Some(&mut self.query),
            Field::GroupName => Some(&mut self.group_name_buf),
            Field::Name => self.form.as_mut().map(|x| &mut x.name),
            Field::Description => self.form.as_mut().map(|x| &mut x.description),
            Field::Command => self.form.as_mut().map(|x| &mut x.command),
            Field::WorkingDir => self.form.as_mut().map(|x| &mut x.working_dir),
            Field::VarValue => self
                .var_prompt
                .as_mut()
                .and_then(|p| p.values.get_mut(p.active).map(|(_, v)| v)),
            Field::Host => None,
        }
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(field) = self.active_field else {
            return;
        };
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                match field {
                    Field::Search => {
                        self.query.clear();
                        self.search_open = false;
                    }
                    Field::GroupName => {
                        self.adding_group = false;
                        self.group_name_buf.clear();
                    }
                    Field::VarValue => self.var_prompt = None,
                    Field::Host => self.host_picker = None,
                    _ => {}
                }
                self.active_field = None;
            }
            "enter" => match field {
                Field::GroupName => {
                    let name = self.group_name_buf.clone();
                    self.create_group(name, cx);
                }
                Field::Command => {
                    if let Some(f) = self.form.as_mut() {
                        f.command.push('\n');
                    }
                }
                Field::VarValue => {
                    let last = self
                        .var_prompt
                        .as_ref()
                        .map(|p| p.active + 1 >= p.vars.len())
                        .unwrap_or(true);
                    if last {
                        self.submit_var_prompt(window, cx);
                    } else if let Some(p) = self.var_prompt.as_mut() {
                        p.active += 1;
                    }
                }
                Field::Name | Field::Description | Field::WorkingDir => self.save_form(cx),
                _ => {}
            },
            "tab" => {
                if field == Field::VarValue {
                    if let Some(p) = self.var_prompt.as_mut() {
                        p.active = (p.active + 1) % p.vars.len().max(1);
                    }
                }
            }
            "backspace" => {
                if let Some(buf) = self.field_buf_mut(field) {
                    buf.pop();
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
                if let Some(ch) = ch {
                    if let Some(buf) = self.field_buf_mut(field) {
                        buf.push_str(&ch);
                    }
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    // ── rendering ─────────────────────────────────────────────────────────

    fn colors(&self, cx: &App) -> Colors {
        let t = self.theme.read(cx);
        Colors {
            bg: t.sidebar_bg(),
            fg: t.sidebar_fg(),
            muted: t.muted_foreground(),
            border: t.sidebar_border(),
            card: t.card(),
            accent: t.accent(),
            error: t.status_error(),
            warning: t.status_warning(),
        }
    }

    fn text_field(
        &self,
        id: &'static str,
        field: Field,
        value: &str,
        placeholder: &'static str,
        c: &Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_field == Some(field);
        let empty = value.is_empty();
        div()
            .id(id)
            .w_full()
            .min_h(px(22.0))
            .px(px(6.0))
            .py(px(3.0))
            .flex()
            .items_center()
            .rounded_sm()
            .border_1()
            .border_color(if active { c.accent } else { c.border })
            .bg(c.bg)
            .text_size(px(11.0))
            .text_color(if empty { c.muted } else { c.fg })
            .whitespace_normal()
            .child(SharedString::from(if empty {
                placeholder.to_string()
            } else {
                value.to_string()
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                cx.stop_propagation();
                this.active_field = Some(field);
                w.focus(&this.focus);
                cx.notify();
            }))
    }

    fn btn(
        &self,
        id: SharedString,
        label: impl Into<SharedString>,
        c: &Colors,
        primary: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> gpui::AnyElement {
        div()
            .id(id)
            .px(px(8.0))
            .h(px(22.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(if primary { c.accent } else { c.border })
            .when(primary, |d| d.bg(c.accent))
            .text_size(px(11.0))
            .text_color(c.fg)
            .hover(|s| s.opacity(0.85))
            .child(label.into())
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                cx.stop_propagation();
                on_click(this, w, cx);
            }))
            .into_any_element()
    }

    fn filtered(&self) -> Vec<CommandSnippet> {
        let q = self.query.trim().to_lowercase();
        let mut v: Vec<CommandSnippet> = if q.is_empty() {
            self.snippets.clone()
        } else {
            self.snippets
                .iter()
                .filter(|s| {
                    s.name.to_lowercase().contains(&q)
                        || s.command.to_lowercase().contains(&q)
                        || s.description
                            .as_deref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                })
                .cloned()
                .collect()
        };
        v.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
        v
    }

    fn render_list(&mut self, c: &Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let filtered = self.filtered();
        let mut groups = self.groups.clone();
        groups.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.name.cmp(&b.name)));
        let has_query = !self.query.trim().is_empty();

        let mut list = div()
            .id("snippets-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(6.0));

        if filtered.is_empty() {
            list = list.child(
                div()
                    .py(px(40.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(6.0))
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(if has_query {
                        "No results"
                    } else {
                        "No snippets yet"
                    }))
                    .child(self.btn(
                        "snip-empty-new".into(),
                        "New snippet",
                        c,
                        true,
                        cx,
                        |this, _w, cx| {
                            this.form = Some(FormState::empty());
                            cx.notify();
                        },
                    )),
            );
            return list.into_any_element();
        }

        for g in &groups {
            let items: Vec<CommandSnippet> = filtered
                .iter()
                .filter(|s| s.group_id.as_deref() == Some(g.id.as_str()))
                .cloned()
                .collect();
            if items.is_empty() && has_query {
                continue;
            }
            let gid = g.id.clone();
            let collapsed = self.collapsed_groups.contains(&gid);
            let gid_toggle = gid.clone();
            let gid_del = gid.clone();
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-grp-{gid}")))
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(9.0))
                            .text_color(c.muted)
                            .child(SharedString::from(if collapsed {
                                "\u{25B6}"
                            } else {
                                "\u{25BC}"
                            }))
                            .child(SharedString::from(g.name.to_uppercase()))
                            .child(SharedString::from(format!("({})", items.len())))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                if !this.collapsed_groups.remove(&gid_toggle) {
                                    this.collapsed_groups.insert(gid_toggle.clone());
                                }
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-grp-del-{gid}")))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.error))
                            .child("\u{2715}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.delete_group(gid_del.clone(), cx);
                            })),
                    ),
            );
            if !collapsed {
                for s in &items {
                    list = list.child(self.render_row(s, c, cx));
                }
            }
        }

        // Ungrouped.
        let ungrouped: Vec<CommandSnippet> = filtered
            .iter()
            .filter(|s| s.group_id.is_none())
            .cloned()
            .collect();
        if !ungrouped.is_empty() {
            if !groups.is_empty() {
                list = list.child(
                    div()
                        .text_size(px(9.0))
                        .text_color(c.muted)
                        .child(SharedString::from("OTHER")),
                );
            }
            for s in &ungrouped {
                list = list.child(self.render_row(s, c, cx));
            }
        }

        // Add-group footer.
        if !has_query {
            if self.adding_group {
                list = list.child(self.text_field(
                    "snip-new-group",
                    Field::GroupName,
                    &self.group_name_buf.clone(),
                    "Group name\u{2026}",
                    c,
                    cx,
                ));
            } else {
                list = list.child(self.btn(
                    "snip-add-group".into(),
                    "+ Add group",
                    c,
                    false,
                    cx,
                    |this, w, cx| {
                        this.adding_group = true;
                        this.active_field = Some(Field::GroupName);
                        w.focus(&this.focus);
                        cx.notify();
                    },
                ));
            }
        }

        list.into_any_element()
    }

    fn render_row(
        &self,
        s: &CommandSnippet,
        c: &Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let is_ssh = s.target == "ssh";
        let host_label = if is_ssh {
            match s.host_id.as_deref() {
                Some(id) => self
                    .host_name(id)
                    .unwrap_or_else(|| "Host missing".to_string()),
                None => "Ask at runtime".to_string(),
            }
        } else {
            String::new()
        };
        let preview = s
            .description
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| s.command.lines().next().unwrap_or("").to_string());
        let mode = ExecMode::from_str(&s.default_exec_mode);

        let s_run = s.clone();
        let s_run_silent = s.clone();
        let s_edit = s.clone();
        let s_dup = s.clone();
        let s_del_id = s.id.clone();
        let s_up = s.id.clone();
        let s_down = s.id.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(6.0))
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.0))
                            .text_color(c.fg)
                            .child(SharedString::from(s.name.clone())),
                    )
                    .when(is_ssh, |d| {
                        d.child(
                            div()
                                .text_size(px(9.0))
                                .text_color(c.muted)
                                .child(SharedString::from(host_label.clone())),
                        )
                    }),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .child(SharedString::from(preview)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(self.btn(
                        SharedString::from(format!("snip-run-{}", s.id)),
                        format!("\u{25B6} RUN ({})", mode.label()),
                        c,
                        true,
                        cx,
                        move |this, w, cx| this.run(s_run.clone(), None, w, cx),
                    ))
                    .child(self.btn(
                        SharedString::from(format!("snip-run-silent-{}", s.id)),
                        "log",
                        c,
                        false,
                        cx,
                        move |this, w, cx| {
                            this.run(s_run_silent.clone(), Some(ExecMode::Silent), w, cx)
                        },
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-up-{}", s.id)))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|st| st.text_color(c.fg))
                            .child("\u{25B2}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.move_snippet(&s_up, -1, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-down-{}", s.id)))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|st| st.text_color(c.fg))
                            .child("\u{25BC}")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.move_snippet(&s_down, 1, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-edit-{}", s.id)))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|st| st.text_color(c.fg))
                            .child(IconName::Pencil.svg(c.muted).size(px(11.0)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.form = Some(FormState::from_snippet(&s_edit));
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-dup-{}", s.id)))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|st| st.text_color(c.fg))
                            .child(IconName::Copy.svg(c.muted).size(px(11.0)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.duplicate_snippet(&s_dup, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-del-{}", s.id)))
                            .px(px(3.0))
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .hover(|st| st.text_color(c.error))
                            .child(IconName::Trash.svg(c.muted).size(px(11.0)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.delete_snippet(s_del_id.clone(), cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_form(&mut self, c: &Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(form) = self.form.clone() else {
            return div().into_any_element();
        };
        let is_new = form.id.is_none();
        let mut groups = self.groups.clone();
        groups.sort_by(|a, b| a.name.cmp(&b.name));

        let mut body = div()
            .id("snip-form-body")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(8.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.fg)
                    .child(SharedString::from(if is_new {
                        "New Snippet"
                    } else {
                        "Edit Snippet"
                    })),
            )
            .child(label(c, "Name"))
            .child(self.text_field("snip-f-name", Field::Name, &form.name, "e.g. Deploy", c, cx))
            .child(label(c, "Description"))
            .child(self.text_field(
                "snip-f-desc",
                Field::Description,
                &form.description,
                "Optional",
                c,
                cx,
            ))
            .child(label(c, "Command"))
            .child(self.text_field(
                "snip-f-cmd",
                Field::Command,
                &form.command,
                "Enter command\u{2026}",
                c,
                cx,
            ));

        // Group picker (chips).
        body = body.child(label(c, "Group")).child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.0))
                .child(self.group_chip(
                    "snip-f-grp-none",
                    "No group",
                    form.group_id.is_empty(),
                    c,
                    cx,
                    String::new(),
                ))
                .children(groups.iter().map(|g| {
                    self.group_chip(
                        SharedString::from(format!("snip-f-grp-{}", g.id)),
                        &g.name,
                        form.group_id == g.id,
                        c,
                        cx,
                        g.id.clone(),
                    )
                })),
        );

        // Target toggle.
        body = body.child(label(c, "Target")).child(
            div()
                .flex()
                .gap(px(4.0))
                .child(self.toggle_chip(
                    "snip-f-tgt-local",
                    "Local",
                    !form.target_ssh,
                    c,
                    cx,
                    |f| f.target_ssh = false,
                ))
                .child(
                    self.toggle_chip("snip-f-tgt-ssh", "SSH", form.target_ssh, c, cx, |f| {
                        f.target_ssh = true
                    }),
                ),
        );

        if form.target_ssh {
            let mut host_row = div().flex().flex_wrap().gap(px(4.0)).child(self.group_chip(
                "snip-f-host-ask",
                "Ask at runtime",
                form.host_id.is_empty(),
                c,
                cx,
                String::new(),
            ));
            for h in self.hosts.clone() {
                let hid = h.id.clone();
                let selected = form.host_id == h.id;
                host_row = host_row.child(
                    div()
                        .id(SharedString::from(format!("snip-f-host-{}", h.id)))
                        .px(px(6.0))
                        .h(px(20.0))
                        .flex()
                        .items_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(if selected { c.accent } else { c.border })
                        .when(selected, |d| d.bg(c.accent))
                        .text_size(px(10.0))
                        .text_color(c.fg)
                        .child(SharedString::from(h.name.clone()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            if let Some(f) = this.form.as_mut() {
                                f.host_id = hid.clone();
                            }
                            cx.notify();
                        })),
                );
            }
            body = body.child(label(c, "Host")).child(host_row);
        }

        // Exec mode.
        body = body
            .child(label(c, "Default Mode"))
            .child(div().flex().gap(px(4.0)).children(
                [ExecMode::Terminal, ExecMode::Silent, ExecMode::Inject].map(|m| {
                    self.toggle_chip(
                        SharedString::from(format!("snip-f-mode-{}", m.as_str())),
                        m.label(),
                        form.mode == m,
                        c,
                        cx,
                        move |f| f.mode = m,
                    )
                }),
            ));

        if !form.target_ssh {
            body = body.child(label(c, "Working Dir")).child(self.text_field(
                "snip-f-wd",
                Field::WorkingDir,
                &form.working_dir,
                "Inherit from terminal",
                c,
                cx,
            ));
        }

        let footer = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(6.0))
            .p(px(8.0))
            .border_t_1()
            .border_color(c.border)
            .child(self.btn(
                "snip-f-cancel".into(),
                "Cancel",
                c,
                false,
                cx,
                |this, _w, cx| {
                    this.form = None;
                    this.active_field = None;
                    cx.notify();
                },
            ))
            .child(self.btn(
                "snip-f-save".into(),
                if is_new { "Create" } else { "Save" },
                c,
                true,
                cx,
                |this, _w, cx| this.save_form(cx),
            ));

        div()
            .flex_1()
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .child(body)
            .child(footer)
            .into_any_element()
    }

    fn group_chip(
        &self,
        id: impl Into<gpui::ElementId>,
        text: &str,
        selected: bool,
        c: &Colors,
        cx: &mut Context<Self>,
        group_id: String,
    ) -> gpui::AnyElement {
        div()
            .id(id)
            .px(px(6.0))
            .h(px(20.0))
            .flex()
            .items_center()
            .rounded_sm()
            .border_1()
            .border_color(if selected { c.accent } else { c.border })
            .when(selected, |d| d.bg(c.accent))
            .text_size(px(10.0))
            .text_color(c.fg)
            .child(SharedString::from(text.to_string()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if let Some(f) = this.form.as_mut() {
                    f.group_id = group_id.clone();
                }
                cx.notify();
            }))
            .into_any_element()
    }

    fn toggle_chip(
        &self,
        id: impl Into<gpui::ElementId>,
        text: &'static str,
        selected: bool,
        c: &Colors,
        cx: &mut Context<Self>,
        apply: impl Fn(&mut FormState) + 'static,
    ) -> gpui::AnyElement {
        div()
            .id(id)
            .flex_1()
            .h(px(22.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .border_1()
            .border_color(if selected { c.accent } else { c.border })
            .when(selected, |d| d.bg(c.accent))
            .text_size(px(10.0))
            .text_color(c.fg)
            .child(text)
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if let Some(f) = this.form.as_mut() {
                    apply(f);
                }
                cx.notify();
            }))
            .into_any_element()
    }

    fn render_var_prompt(&self, c: &Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let p = self.var_prompt.as_ref()?;
        let name = p.pending.snippet.name.clone();
        let mut rows = div().flex().flex_col().gap(px(6.0));
        for (i, v) in p.vars.iter().enumerate() {
            let val = p.values.get(i).map(|(_, x)| x.clone()).unwrap_or_default();
            let active = self.active_field == Some(Field::VarValue) && p.active == i;
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .child(SharedString::from(v.name.clone())),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("snip-var-{}", v.name)))
                            .w_full()
                            .min_h(px(22.0))
                            .px(px(6.0))
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(if active { c.accent } else { c.border })
                            .bg(c.bg)
                            .text_size(px(11.0))
                            .text_color(if val.is_empty() { c.muted } else { c.fg })
                            .child(SharedString::from(if val.is_empty() {
                                v.default_value.clone().unwrap_or_default()
                            } else {
                                val
                            }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                                cx.stop_propagation();
                                if let Some(p) = this.var_prompt.as_mut() {
                                    p.active = i;
                                }
                                this.active_field = Some(Field::VarValue);
                                w.focus(&this.focus);
                                cx.notify();
                            })),
                    ),
            );
        }
        Some(self.modal(
            c,
            "Fill in variables",
            &format!("\"{name}\" uses variables — fill in a value for each."),
            rows.into_any_element(),
            "Run",
            cx,
            |this, w, cx| this.submit_var_prompt(w, cx),
            |this, _w, cx| {
                this.var_prompt = None;
                this.active_field = None;
                cx.notify();
            },
        ))
    }

    fn render_host_picker(&self, c: &Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let hp = self.host_picker.as_ref()?;
        let name = hp.pending.snippet.name.clone();
        let selected = hp.selected.clone();
        let mut rows = div().flex().flex_col().gap(px(3.0));
        if self.hosts.is_empty() {
            rows = rows.child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .child("No hosts configured yet. Add one in the Host Manager first."),
            );
        }
        for h in &self.hosts {
            let hid = h.id.clone();
            let is_sel = selected == h.id;
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("snip-hp-{}", h.id)))
                    .px(px(6.0))
                    .h(px(22.0))
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_sel { c.accent } else { c.border })
                    .when(is_sel, |d| d.bg(c.accent))
                    .text_size(px(11.0))
                    .text_color(c.fg)
                    .child(SharedString::from(format!(
                        "{} ({})",
                        h.name, h.host_address
                    )))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if let Some(hp) = this.host_picker.as_mut() {
                            hp.selected = hid.clone();
                        }
                        cx.notify();
                    })),
            );
        }
        Some(self.modal(
            c,
            "Select a host",
            &format!("\"{name}\" asks for a host at runtime. Choose which host to run it on."),
            rows.into_any_element(),
            "Run",
            cx,
            |this, w, cx| {
                let Some(hp) = this.host_picker.take() else {
                    return;
                };
                if hp.selected.is_empty() {
                    this.host_picker = Some(hp);
                    return;
                }
                this.active_field = None;
                let mut snippet = hp.pending.snippet;
                snippet.host_id = Some(hp.selected);
                this.dispatch(
                    PendingRun {
                        snippet,
                        mode: hp.pending.mode,
                    },
                    hp.command,
                    w,
                    cx,
                );
            },
            |this, _w, cx| {
                this.host_picker = None;
                this.active_field = None;
                cx.notify();
            },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn modal(
        &self,
        c: &Colors,
        title: &str,
        desc: &str,
        body: gpui::AnyElement,
        confirm_label: &'static str,
        cx: &mut Context<Self>,
        on_confirm: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        on_cancel: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> gpui::AnyElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                div()
                    .w(px(280.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .p(px(12.0))
                    .rounded_md()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.card)
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(c.fg)
                            .child(SharedString::from(title.to_string())),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .child(SharedString::from(desc.to_string())),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(6.0))
                            .child(self.btn(
                                "snip-modal-cancel".into(),
                                "Cancel",
                                c,
                                false,
                                cx,
                                on_cancel,
                            ))
                            .child(self.btn(
                                "snip-modal-ok".into(),
                                confirm_label,
                                c,
                                true,
                                cx,
                                on_confirm,
                            )),
                    ),
            )
            .into_any_element()
    }

    fn render_log_drawer(&self, c: &Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.log_open {
            return div().into_any_element();
        }
        let selected = self
            .selected_run
            .as_ref()
            .and_then(|id| self.run_logs.iter().find(|l| &l.run_id == id))
            .or_else(|| self.run_logs.first());

        let mut tabs = div()
            .id("snip-log-tabs")
            .w(px(120.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(1.0))
            .p(px(3.0))
            .border_r_1()
            .border_color(c.border)
            .overflow_y_scroll();
        for l in &self.run_logs {
            let rid = l.run_id.clone();
            let active = selected.map(|s| s.run_id == l.run_id).unwrap_or(false);
            let glyph = match l.status {
                RunStatus::Running => "\u{25CC}",
                RunStatus::Done => "\u{2713}",
                RunStatus::Cancelled => "\u{25A0}",
                RunStatus::Error => "\u{25B2}",
            };
            tabs = tabs.child(
                div()
                    .id(SharedString::from(format!("snip-log-tab-{}", l.run_id)))
                    .flex()
                    .gap(px(3.0))
                    .px(px(4.0))
                    .py(px(2.0))
                    .rounded_sm()
                    .when(active, |d| d.bg(c.accent))
                    .text_size(px(10.0))
                    .text_color(c.fg)
                    .child(SharedString::from(glyph))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(l.snippet_name.clone())),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.selected_run = Some(rid.clone());
                        cx.notify();
                    })),
            );
        }

        let output = match selected {
            None => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.0))
                .text_color(c.muted)
                .child("No runs yet")
                .into_any_element(),
            Some(l) => {
                let mut pre = div()
                    .id("snip-log-output")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .p(px(6.0))
                    .overflow_y_scroll()
                    .text_size(px(10.0));
                if l.lines.is_empty() && l.status == RunStatus::Running {
                    pre = pre.child(div().text_color(c.muted).child("Running\u{2026}"));
                }
                for line in &l.lines {
                    pre = pre.child(
                        div()
                            .whitespace_normal()
                            .text_color(if line.is_err { c.error } else { c.fg })
                            .child(SharedString::from(line.data.clone())),
                    );
                }
                if let Some(code) = l.exit_code {
                    if l.status != RunStatus::Running {
                        pre = pre.child(
                            div()
                                .text_color(if code == 0 { c.muted } else { c.error })
                                .child(SharedString::from(format!("[exit {code}]"))),
                        );
                    }
                }
                pre.into_any_element()
            }
        };

        let run_id_for_cancel = selected
            .filter(|l| l.status == RunStatus::Running)
            .map(|l| l.run_id.clone());

        div()
            .h(px(200.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(c.border)
            .bg(c.card)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(6.0))
                    .py(px(3.0))
                    .border_b_1()
                    .border_color(c.border)
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .child("Snippet Logs"),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .when_some(run_id_for_cancel, |d, rid| {
                                d.child(
                                    div()
                                        .id("snip-log-cancel")
                                        .px(px(4.0))
                                        .text_size(px(10.0))
                                        .text_color(c.warning)
                                        .child("Cancel")
                                        .on_click(cx.listener(
                                            move |this, _: &ClickEvent, _w, cx| {
                                                this.cancel_run(rid.clone(), cx);
                                            },
                                        )),
                                )
                            })
                            .child(
                                div()
                                    .id("snip-log-clear")
                                    .px(px(4.0))
                                    .text_size(px(10.0))
                                    .text_color(c.muted)
                                    .hover(|s| s.text_color(c.fg))
                                    .child("Clear")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.run_logs.clear();
                                        this.selected_run = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("snip-log-close")
                                    .px(px(4.0))
                                    .text_size(px(10.0))
                                    .text_color(c.muted)
                                    .hover(|s| s.text_color(c.fg))
                                    .child("\u{2715}")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.log_open = false;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .when(self.run_logs.is_empty(), |d| {
                        d.items_center().justify_center().child(
                            div()
                                .text_size(px(10.0))
                                .text_color(c.muted)
                                .child("No runs yet"),
                        )
                    })
                    .when(!self.run_logs.is_empty(), |d| d.child(tabs).child(output)),
            )
            .into_any_element()
    }
}

fn label(c: &Colors, text: &'static str) -> gpui::AnyElement {
    div()
        .text_size(px(9.0))
        .text_color(c.muted)
        .child(text)
        .into_any_element()
}

impl Render for SnippetsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = self.colors(cx);

        let header = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(28.0))
            .px(px(6.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex_1()
                    .text_size(px(11.0))
                    .text_color(c.fg)
                    .child("Snippets"),
            )
            .child(
                div()
                    .id("snip-search-toggle")
                    .px(px(4.0))
                    .text_size(px(11.0))
                    .text_color(if self.search_open { c.fg } else { c.muted })
                    .child(
                        IconName::Search
                            .svg(if self.search_open { c.fg } else { c.muted })
                            .size(px(12.0)),
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.search_open = !this.search_open;
                        if this.search_open {
                            this.active_field = Some(Field::Search);
                            w.focus(&this.focus);
                        } else {
                            this.query.clear();
                            this.active_field = None;
                        }
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("snip-log-toggle")
                    .px(px(4.0))
                    .text_size(px(11.0))
                    .text_color(if self.log_open { c.fg } else { c.muted })
                    .child("\u{2261}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.log_open = !this.log_open;
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("snip-new")
                    .px(px(4.0))
                    .text_size(px(12.0))
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("+")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.form = Some(FormState::empty());
                        cx.notify();
                    })),
            );

        let search_bar = self.search_open.then(|| {
            self.text_field(
                "snip-search",
                Field::Search,
                &self.query.clone(),
                "Search snippets\u{2026}",
                &c,
                cx,
            )
        });

        let main: gpui::AnyElement = if self.form.is_some() {
            self.render_form(&c, cx)
        } else {
            self.render_list(&c, cx)
        };

        let var_prompt = self.render_var_prompt(&c, cx);
        let host_picker = self.render_host_picker(&c, cx);
        let log_drawer = self.render_log_drawer(&c, cx);

        div()
            .key_context("SnippetsPanel")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .child(header)
            .when_some(search_bar, |d, sb| d.child(div().p(px(6.0)).child(sb)))
            .child(main)
            .child(log_drawer)
            .children(var_prompt)
            .children(host_picker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(vars: &[SnippetVariable]) -> Vec<&str> {
        vars.iter().map(|v| v.name.as_str()).collect()
    }

    // ── extract_snippet_variables (ported from snippetVariables.test.ts) ──

    #[test]
    fn extract_empty_for_no_variables() {
        assert!(extract_snippet_variables("docker compose up -d").is_empty());
    }

    #[test]
    fn extract_single_no_default() {
        assert_eq!(
            extract_snippet_variables("echo ${NAME}"),
            vec![SnippetVariable {
                name: "NAME".into(),
                default_value: None
            }]
        );
    }

    #[test]
    fn extract_with_default() {
        assert_eq!(
            extract_snippet_variables("deploy ${ENVIRONMENT:-staging}"),
            vec![SnippetVariable {
                name: "ENVIRONMENT".into(),
                default_value: Some("staging".into())
            }]
        );
    }

    #[test]
    fn extract_dedupes_keeping_first() {
        assert_eq!(
            extract_snippet_variables("echo ${NAME} > ${NAME}.txt"),
            vec![SnippetVariable {
                name: "NAME".into(),
                default_value: None
            }]
        );
        assert_eq!(
            extract_snippet_variables("echo ${NAME:-world} && echo ${NAME}"),
            vec![SnippetVariable {
                name: "NAME".into(),
                default_value: Some("world".into())
            }]
        );
    }

    #[test]
    fn extract_substring_names_do_not_collide() {
        assert_eq!(
            names(&extract_snippet_variables("echo ${VAR} ${VAR_2}")),
            vec!["VAR", "VAR_2"]
        );
    }

    #[test]
    fn extract_preserves_order() {
        assert_eq!(
            names(&extract_snippet_variables(
                "scp ${SRC} user@${HOST_NAME}:${DEST}"
            )),
            vec!["SRC", "HOST_NAME", "DEST"]
        );
    }

    #[test]
    fn extract_excludes_reserved_names() {
        assert!(extract_snippet_variables("echo $PATH && echo ${PATH}").is_empty());
        assert_eq!(
            names(&extract_snippet_variables(
                "HOME=${HOME} ${TARGET_DIR:-/tmp}"
            )),
            vec!["TARGET_DIR"]
        );
    }

    #[test]
    fn extract_ignores_positional_and_lowercase() {
        assert!(extract_snippet_variables("echo ${1} ${@} ${#}").is_empty());
        assert!(extract_snippet_variables("echo ${name} ${Name}").is_empty());
    }

    // ── substitute_snippet_variables ────────────────────────────────────

    fn vals(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn substitute_single() {
        assert_eq!(
            substitute_snippet_variables("echo ${NAME}", &vals(&[("NAME", "world")])),
            "echo world"
        );
    }

    #[test]
    fn substitute_duplicates_same_value() {
        assert_eq!(
            substitute_snippet_variables("echo ${NAME} > ${NAME}.txt", &vals(&[("NAME", "log")])),
            "echo log > log.txt"
        );
    }

    #[test]
    fn substitute_default_form() {
        assert_eq!(
            substitute_snippet_variables(
                "deploy ${ENVIRONMENT:-staging}",
                &vals(&[("ENVIRONMENT", "prod")])
            ),
            "deploy prod"
        );
    }

    #[test]
    fn substitute_leaves_reserved_and_unsupplied_untouched() {
        assert_eq!(
            substitute_snippet_variables("echo ${PATH}", &HashMap::new()),
            "echo ${PATH}"
        );
        assert_eq!(
            substitute_snippet_variables("echo ${NAME}", &HashMap::new()),
            "echo ${NAME}"
        );
        assert_eq!(
            substitute_snippet_variables("docker compose up -d", &HashMap::new()),
            "docker compose up -d"
        );
    }

    // ── tags (ported from snippetUtils.test.ts) ─────────────────────────

    #[test]
    fn parse_tags_valid_and_invalid() {
        assert_eq!(
            parse_tags(Some(r#"["foo","bar","baz"]"#)),
            vec!["foo", "bar", "baz"]
        );
        assert!(parse_tags(None).is_empty());
        assert!(parse_tags(Some("")).is_empty());
        assert!(parse_tags(Some("not-json")).is_empty());
        assert!(parse_tags(Some(r#"{"key":"value"}"#)).is_empty());
        assert!(parse_tags(Some("null")).is_empty());
        assert_eq!(
            parse_tags(Some(r#"["hello world","foo-bar"]"#)),
            vec!["hello world", "foo-bar"]
        );
    }

    #[test]
    fn serialize_tags_and_roundtrip() {
        assert_eq!(serialize_tags(&[]), None);
        assert_eq!(
            serialize_tags(&["foo".into()]).as_deref(),
            Some(r#"["foo"]"#)
        );
        let tags = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        assert_eq!(parse_tags(serialize_tags(&tags).as_deref()), tags);
    }

    // ── run-log ring buffer ────────────────────────────────────────────

    #[test]
    fn run_log_is_newest_first_capped_at_50() {
        let mut logs = Vec::new();
        for i in 0..60 {
            push_run_log(
                &mut logs,
                SnippetRunLog {
                    run_id: format!("r{i}"),
                    snippet_name: format!("s{i}"),
                    started_at: 0,
                    status: RunStatus::Running,
                    exit_code: None,
                    lines: Vec::new(),
                },
            );
        }
        assert_eq!(logs.len(), 50);
        assert_eq!(logs[0].run_id, "r59");
        assert_eq!(logs[49].run_id, "r10");
    }
}
