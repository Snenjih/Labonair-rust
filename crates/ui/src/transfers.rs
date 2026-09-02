//! SFTP transfer queue UI (T08-002).
//!
//! Ported from `reference-src/src/modules/sftp/store/transferStore.ts` +
//! `reference-src/src/modules/header/components/TransferDropdown.tsx`. The React
//! version is a Zustand store fed by four Tauri events (`transfer_progress`,
//! `transfer_step`, `file_conflict`, `file_error`) plus a header popover that
//! lists jobs with a live progress bar, a per-job step log, a cancel button and
//! two modal dialogs (conflict / file-error).
//!
//! Here the backend transfer worker
//! ([`labonair_backend::modules::sftp::worker`]) is unchanged — it already
//! processes the queue, walks folders recursively, verifies checksums and
//! emits the same four events on the in-process [`labonair_backend::EventBus`].
//! [`Workspace`](crate::workspace::Workspace) forwards those events off the bus
//! as [`TransferBusEvent`]s and pumps them into [`TransfersView::apply`]. This
//! module owns the queue display, the cancel action, the conflict/file-error
//! resolution (`resolve_conflict`) and the "overwrite/skip all" sticky policy.
//!
//! Deviations from the reference:
//! * The popover is a fixed bottom-right panel toggled by a pill, not a
//!   configurable status-bar/header bar-item (bar-item placement is a Phase 12
//!   concern).
//! * The conflict modal's "Rename" uses an auto-generated `name_1.ext` seed
//!   (editable) instead of a free-form field with the original name.

use std::collections::{HashMap, HashSet};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use tokio::runtime::Handle as TokioHandle;

use labonair_backend::modules::sftp::commands::{cancel_transfer, resolve_conflict};
use labonair_backend::modules::sftp::{TransferDirection, TransferJob, TransferStatus};
use labonair_backend::App as Backend;

use crate::theme::ThemeStore;

// ── bus events ─────────────────────────────────────────────────────────────

/// A transfer-worker event lifted off the backend broadcast bus. Decoded from
/// the raw `(name, payload)` form by [`TransferBusEvent::from_raw`] — the typed
/// [`labonair_backend::AppEvent`] can't carry these because the worker emits
/// the full [`TransferJob`] for `transfer_progress`, not `AppEvent`'s reduced
/// shape.
#[derive(Clone, Debug)]
pub enum TransferBusEvent {
    Progress(TransferJob),
    Step {
        job_id: String,
        ts: i64,
        message: String,
    },
    Conflict {
        job_id: String,
        src_path: String,
        dest_path: String,
    },
    FileError {
        job_id: String,
        path: String,
        error: String,
    },
}

impl TransferBusEvent {
    pub fn from_raw(name: &str, payload: &serde_json::Value) -> Option<Self> {
        match name {
            "transfer_progress" => serde_json::from_value::<TransferJob>(payload.clone())
                .ok()
                .map(Self::Progress),
            "transfer_step" => Some(Self::Step {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                ts: payload.get("ts").and_then(|v| v.as_i64()).unwrap_or(0),
                message: payload.get("message")?.as_str()?.to_string(),
            }),
            "file_conflict" => Some(Self::Conflict {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                src_path: payload.get("src_path")?.as_str()?.to_string(),
                dest_path: payload.get("dest_path")?.as_str()?.to_string(),
            }),
            "file_error" => Some(Self::FileError {
                job_id: payload.get("job_id")?.as_str()?.to_string(),
                path: payload.get("path")?.as_str()?.to_string(),
                error: payload.get("error")?.as_str()?.to_string(),
            }),
            _ => None,
        }
    }
}

// ── pure helpers (unit-tested) ─────────────────────────────────────────────

/// Percent complete, 0–100. `bytes_total == 0` (folder scan not finished, or
/// an empty file) reads as 0 while running and 100 once complete.
pub fn percent(job: &TransferJob) -> u8 {
    if matches!(job.status, TransferStatus::Completed) {
        return 100;
    }
    if job.bytes_total == 0 {
        return 0;
    }
    ((job.bytes_transferred.min(job.bytes_total) * 100) / job.bytes_total) as u8
}

/// Human-readable byte size — mirrors the reference `formatBytes`.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

/// Last path segment of a `/`-joined path.
pub fn base_name(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
}

/// Auto-generated rename seed: `report.tar.gz` → `report_1.tar.gz`,
/// `notes` → `notes_1`. Mirrors the reference's "pick a new name" intent
/// (`Datei_1.ext`).
pub fn suggested_rename(name: &str) -> String {
    match name.find('.') {
        Some(0) | None => format!("{name}_1"),
        Some(i) => format!("{}_1{}", &name[..i], &name[i..]),
    }
}

pub fn status_label(status: &TransferStatus) -> &'static str {
    match status {
        TransferStatus::Queued => "queued",
        TransferStatus::Running => "running",
        TransferStatus::Paused => "paused",
        TransferStatus::Cancelled => "cancelled",
        TransferStatus::Completed => "done",
        TransferStatus::Failed(_) => "failed",
    }
}

fn is_active(status: &TransferStatus) -> bool {
    matches!(status, TransferStatus::Queued | TransferStatus::Running)
}

fn is_terminal(status: &TransferStatus) -> bool {
    matches!(
        status,
        TransferStatus::Completed | TransferStatus::Cancelled | TransferStatus::Failed(_)
    )
}

// ── model ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TransferStep {
    pub ts: i64,
    pub message: String,
}

struct JobRow {
    job: TransferJob,
    /// Set while a `file_conflict` for this job awaits the user: `(src, dest)`.
    conflict: Option<(String, String)>,
    /// Set while a per-file `file_error` awaits the user: `(rel_path, error)`.
    file_error: Option<(String, String)>,
}

/// Which resolution dialog is open (only one at a time, matching the
/// reference's conflict-before-file-error precedence).
enum Modal {
    Conflict {
        job_id: String,
        renaming: Option<String>,
    },
    FileError {
        job_id: String,
    },
}

/// Emitted so the workspace can refresh the pane that just received a file.
pub enum TransfersEvent {
    Completed {
        session_id: String,
        direction: TransferDirection,
    },
}

pub struct TransfersView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    /// Newest job first.
    jobs: Vec<JobRow>,
    steps: HashMap<String, Vec<TransferStep>>,
    /// Sticky "overwrite"/"skip" per session id, set by "…All" in the modal.
    sticky: HashMap<String, String>,
    expanded_logs: HashSet<String>,
    modal: Option<Modal>,
    /// Panel open/closed.
    open: bool,
    focus: FocusHandle,
    dialog_focus: FocusHandle,
}

impl EventEmitter<TransfersEvent> for TransfersView {}

impl Focusable for TransfersView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TransfersView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            backend,
            tokio,
            theme,
            jobs: Vec::new(),
            steps: HashMap::new(),
            sticky: HashMap::new(),
            expanded_logs: HashSet::new(),
            modal: None,
            open: false,
            focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
        }
    }

    fn row(&mut self, id: &str) -> Option<&mut JobRow> {
        self.jobs.iter_mut().find(|r| r.job.id == id)
    }

    fn session_of(&self, id: &str) -> Option<String> {
        self.jobs
            .iter()
            .find(|r| r.job.id == id)
            .map(|r| r.job.session_id.clone())
    }

    // ── event intake ───────────────────────────────────────────────────────

    pub fn apply(&mut self, ev: TransferBusEvent, cx: &mut Context<Self>) {
        match ev {
            TransferBusEvent::Progress(job) => self.on_progress(job, cx),
            TransferBusEvent::Step {
                job_id,
                ts,
                message,
            } => {
                self.steps
                    .entry(job_id)
                    .or_default()
                    .push(TransferStep { ts, message });
            }
            TransferBusEvent::Conflict {
                job_id,
                src_path,
                dest_path,
            } => self.on_conflict(job_id, src_path, dest_path, cx),
            TransferBusEvent::FileError {
                job_id,
                path,
                error,
            } => {
                if let Some(row) = self.row(&job_id) {
                    row.file_error = Some((path, error));
                    row.job.status = TransferStatus::Paused;
                }
                if self.modal.is_none() {
                    self.modal = Some(Modal::FileError { job_id });
                }
            }
        }
        cx.notify();
    }

    fn on_progress(&mut self, job: TransferJob, cx: &mut Context<Self>) {
        let completed = matches!(job.status, TransferStatus::Completed);
        let (session_id, direction) = (job.session_id.clone(), job.direction.clone());
        // A fresh progress update after a pause means the worker resumed — drop
        // any stale conflict/error dialog state for this job.
        let clear_dialog = !matches!(job.status, TransferStatus::Queued);

        if let Some(row) = self.row(&job.id) {
            row.job = job;
            if clear_dialog {
                row.conflict = None;
                row.file_error = None;
            }
        } else {
            self.jobs.insert(
                0,
                JobRow {
                    job,
                    conflict: None,
                    file_error: None,
                },
            );
        }

        // Close a modal whose job just left the paused state.
        if let Some(m) = &self.modal {
            let mid = match m {
                Modal::Conflict { job_id, .. } | Modal::FileError { job_id } => job_id.clone(),
            };
            if self
                .row(&mid)
                .map(|r| r.conflict.is_none() && r.file_error.is_none())
                .unwrap_or(true)
            {
                self.modal = None;
            }
        }

        if completed {
            cx.emit(TransfersEvent::Completed {
                session_id,
                direction,
            });
        }
    }

    fn on_conflict(
        &mut self,
        job_id: String,
        src_path: String,
        dest_path: String,
        cx: &mut Context<Self>,
    ) {
        // Session-wide "…All" already chosen → resolve without surfacing UI,
        // including for conflicts a big recursive copy reports progressively.
        if let Some(session) = self.session_of(&job_id) {
            if let Some(policy) = self.sticky.get(&session).cloned() {
                self.send_resolution(&job_id, &policy, None, cx);
                return;
            }
        }
        if let Some(row) = self.row(&job_id) {
            row.conflict = Some((src_path, dest_path));
            row.job.status = TransferStatus::Paused;
        }
        if self.modal.is_none() {
            self.modal = Some(Modal::Conflict {
                job_id,
                renaming: None,
            });
        }
    }

    // ── actions ────────────────────────────────────────────────────────────

    /// Open the queue panel (called when a new transfer is enqueued).
    pub fn reveal(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub fn cancel(&mut self, id: String, cx: &mut Context<Self>) {
        let worker = self.backend.transfer.clone();
        self.tokio.spawn(async move {
            let _ = cancel_transfer(id, &worker).await;
        });
        cx.notify();
    }

    pub fn clear_completed(&mut self, cx: &mut Context<Self>) {
        let kept: HashSet<String> = self
            .jobs
            .iter()
            .filter(|r| !is_terminal(&r.job.status))
            .map(|r| r.job.id.clone())
            .collect();
        self.jobs.retain(|r| kept.contains(&r.job.id));
        self.steps.retain(|k, _| kept.contains(k));
        self.expanded_logs.retain(|k| kept.contains(k));
        cx.notify();
    }

    fn send_resolution(
        &mut self,
        id: &str,
        resolution: &str,
        new_name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(row) = self.row(id) {
            row.conflict = None;
            row.file_error = None;
            row.job.status = TransferStatus::Running;
        }
        let worker = self.backend.transfer.clone();
        let (id, resolution) = (id.to_string(), resolution.to_string());
        self.tokio.spawn(async move {
            let _ = resolve_conflict(id, resolution, new_name, &worker).await;
        });
        cx.notify();
    }

    /// Resolve one conflict from the modal. `overwrite_all` / `skip_all` set the
    /// session sticky policy and fan out to every already-paused sibling.
    fn resolve_conflict_choice(
        &mut self,
        job_id: String,
        choice: &str,
        new_name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.modal = None;
        match choice {
            "overwrite_all" | "skip_all" => {
                let base = if choice == "overwrite_all" {
                    "overwrite"
                } else {
                    "skip"
                };
                if let Some(session) = self.session_of(&job_id) {
                    self.sticky.insert(session.clone(), base.to_string());
                    let siblings: Vec<String> = self
                        .jobs
                        .iter()
                        .filter(|r| {
                            r.job.session_id == session
                                && r.job.id != job_id
                                && r.conflict.is_some()
                        })
                        .map(|r| r.job.id.clone())
                        .collect();
                    for sib in siblings {
                        self.send_resolution(&sib, base, None, cx);
                    }
                }
                self.send_resolution(&job_id, base, None, cx);
            }
            other => self.send_resolution(&job_id, other, new_name, cx),
        }
    }

    fn resolve_file_error(&mut self, job_id: String, choice: &str, cx: &mut Context<Self>) {
        self.modal = None;
        self.send_resolution(&job_id, choice, None, cx);
    }
}

// ── keyboard (dialogs) ─────────────────────────────────────────────────────

impl TransfersView {
    fn on_rename_key(&mut self, job_id: String, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                if let Some(Modal::Conflict { renaming, .. }) = &mut self.modal {
                    *renaming = None;
                }
            }
            "enter" => {
                let name = match &self.modal {
                    Some(Modal::Conflict {
                        renaming: Some(n), ..
                    }) => n.trim().to_string(),
                    _ => String::new(),
                };
                if !name.is_empty() {
                    self.resolve_conflict_choice(job_id, "rename", Some(name), cx);
                }
            }
            "backspace" => {
                if let Some(Modal::Conflict {
                    renaming: Some(n), ..
                }) = &mut self.modal
                {
                    n.pop();
                }
            }
            _ => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks) {
                    if let Some(Modal::Conflict {
                        renaming: Some(n), ..
                    }) = &mut self.modal
                    {
                        n.push_str(&ch);
                    }
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

fn printable(ks: &gpui::Keystroke) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| (ks.key.chars().count() == 1).then(|| ks.key.clone()))
}

// ── rendering ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Colors {
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    bg: gpui::Hsla,
    err: gpui::Hsla,
    warn: gpui::Hsla,
    ok: gpui::Hsla,
    info: gpui::Hsla,
}

impl Render for TransfersView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = {
            let t = self.theme.read(cx);
            Colors {
                fg: t.foreground(),
                muted: t.muted_foreground(),
                accent: t.accent(),
                border: t.border(),
                card: t.card(),
                bg: t.background(),
                err: t.status_error(),
                warn: t.status_warning(),
                ok: t.status_success(),
                info: t.status_info(),
            }
        };

        if let Some(modal) = self.render_modal(c, cx) {
            return modal;
        }

        let active = self
            .jobs
            .iter()
            .filter(|r| is_active(&r.job.status))
            .count();
        if self.jobs.is_empty() {
            return div().into_any_element();
        }

        let mut root = div()
            .absolute()
            .right(px(12.0))
            .bottom(px(12.0))
            .flex()
            .flex_col()
            .items_end()
            .gap_2();

        if self.open {
            root = root.child(self.render_panel(c, cx));
        }

        root.child(
            div()
                .id("transfers-pill")
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded_full()
                .border_1()
                .border_color(c.border)
                .bg(c.card)
                .text_color(c.fg)
                .text_xs()
                .shadow_lg()
                .child(
                    crate::components::IconName::ArrowDownUp
                        .svg(c.fg)
                        .size(px(12.0)),
                )
                .child(SharedString::from(format!(
                    "{active} active \u{00b7} {} total",
                    self.jobs.len()
                )))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.open = !this.open;
                    cx.notify();
                })),
        )
        .into_any_element()
    }
}

impl TransfersView {
    fn render_panel(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Transfers"),
            )
            .child(
                div()
                    .id("transfers-clear")
                    .text_xs()
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("Clear completed")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.clear_completed(cx))),
            );

        let mut list = div()
            .id("transfers-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll();
        for row in &self.jobs {
            list = list.child(self.render_job(row, c, cx));
        }

        div()
            .w(px(380.0))
            .max_h(px(460.0))
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .text_color(c.fg)
            .shadow_lg()
            .child(header)
            .child(list)
            .into_any_element()
    }

    fn render_job(&self, row: &JobRow, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let job = &row.job;
        let name = base_name(&job.dest_path).to_string();
        let pct = percent(job);
        let (label, tint) = match &job.status {
            TransferStatus::Running => ("running", c.info),
            TransferStatus::Completed => ("done", c.ok),
            TransferStatus::Paused => ("paused", c.warn),
            TransferStatus::Failed(_) => ("failed", c.err),
            TransferStatus::Cancelled => ("cancelled", c.muted),
            TransferStatus::Queued => ("queued", c.muted),
        };
        let arrow = if matches!(job.direction, TransferDirection::Download) {
            "\u{2193}"
        } else {
            "\u{2191}"
        };
        let id = job.id.clone();
        let id_cancel = job.id.clone();
        let id_log = job.id.clone();
        let has_steps = self
            .steps
            .get(&job.id)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        let log_open = self.expanded_logs.contains(&job.id);
        let active = is_active(&job.status);
        let failed_msg = match &job.status {
            TransferStatus::Failed(e) => Some(e.clone()),
            _ => None,
        };

        let mut header_row =
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(div().text_xs().child(arrow))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .child(SharedString::from(name)),
                )
                .child(div().px_1().rounded_sm().text_xs().text_color(tint).child(
                    SharedString::from(if job.skipped_count > 0 {
                        format!("{label} \u{00b7} {} skipped", job.skipped_count)
                    } else {
                        label.to_string()
                    }),
                ));
        if has_steps {
            header_row = header_row.child(
                div()
                    .id(SharedString::from(format!("transfer-logtoggle-{}", id_log)))
                    .text_xs()
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(if log_open { "\u{25B4}" } else { "\u{25BE}" })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if !this.expanded_logs.remove(&id_log) {
                            this.expanded_logs.insert(id_log.clone());
                        }
                        cx.notify();
                    })),
            );
        }
        if active {
            header_row = header_row.child(
                div()
                    .id(SharedString::from(format!("transfer-cancel-{}", id_cancel)))
                    .text_xs()
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.err))
                    .child("\u{2715}")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.cancel(id_cancel.clone(), cx)
                    })),
            );
        }

        let bar = div().h(px(3.0)).w_full().rounded_full().bg(c.border).child(
            div()
                .h_full()
                .rounded_full()
                .bg(tint)
                .w(gpui::relative(pct as f32 / 100.0)),
        );

        let sub = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .text_xs()
            .text_color(c.muted)
            .child(SharedString::from(format!(
                "{} \u{2192} {}",
                base_name(&job.src_path),
                base_name(&job.dest_path)
            )))
            .child(SharedString::from(match &job.status {
                TransferStatus::Running => format!(
                    "{} / {}  {}/s",
                    format_bytes(job.bytes_transferred),
                    format_bytes(job.bytes_total),
                    format_bytes(job.speed_bps as u64)
                ),
                TransferStatus::Completed => format_bytes(job.bytes_total),
                _ => String::new(),
            }));

        let mut container = div()
            .id(SharedString::from(format!("transfer-row-{id}")))
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(c.border)
            .child(header_row)
            .child(bar)
            .child(sub);

        if let Some(msg) = failed_msg {
            container = container.child(
                div()
                    .text_xs()
                    .font_family("monospace")
                    .text_color(c.err)
                    .child(SharedString::from(msg)),
            );
        }

        if log_open {
            if let Some(steps) = self.steps.get(&row.job.id) {
                let mut log = div()
                    .id(SharedString::from(format!("transfer-log-{}", row.job.id)))
                    .mt_1()
                    .max_h(px(120.0))
                    .overflow_y_scroll()
                    .rounded_sm()
                    .bg(c.bg)
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .gap_0p5();
                for s in steps {
                    log = log.child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(c.muted)
                            .child(SharedString::from(s.message.clone())),
                    );
                }
                container = container.child(log);
            }
        }

        container.into_any_element()
    }

    fn render_modal(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let modal = self.modal.as_ref()?;
        let (job_id, is_file_error) = match modal {
            Modal::Conflict { job_id, .. } => (job_id.clone(), false),
            Modal::FileError { job_id } => (job_id.clone(), true),
        };
        let row = self.jobs.iter().find(|r| r.job.id == job_id)?;

        let body = if is_file_error {
            let (path, error) = row.file_error.clone().unwrap_or_default();
            self.render_file_error_body(&job_id, &path, &error, c, cx)
        } else {
            let dest = row
                .conflict
                .as_ref()
                .map(|(_, d)| d.clone())
                .unwrap_or_else(|| row.job.dest_path.clone());
            self.render_conflict_body(&job_id, &dest, c, cx)
        };

        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(crate::theme::modal_scrim())
                .child(
                    div()
                        .id("transfer-modal")
                        .track_focus(&self.dialog_focus)
                        .w(px(420.0))
                        .flex()
                        .flex_col()
                        .gap_3()
                        .p_4()
                        .rounded_md()
                        .border_1()
                        .border_color(c.border)
                        .bg(c.card)
                        .text_color(c.fg)
                        .shadow_lg()
                        .child(body),
                )
                .into_any_element(),
        )
    }

    fn render_conflict_body(
        &self,
        job_id: &str,
        dest: &str,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let name = base_name(dest).to_string();
        let renaming = match &self.modal {
            Some(Modal::Conflict {
                renaming: Some(n), ..
            }) => Some(n.clone()),
            _ => None,
        };

        let mut root = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Item already exists"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "{name} already exists at {dest}"
                    ))),
            );

        if let Some(buffer) = renaming {
            let id_owned = job_id.to_string();
            let id_key = job_id.to_string();
            root = root
                .child(
                    div()
                        .id("transfer-rename")
                        .track_focus(&self.dialog_focus)
                        .px_2()
                        .py_1()
                        .text_sm()
                        .rounded_sm()
                        .border_1()
                        .border_color(c.accent)
                        .bg(c.bg)
                        .child(SharedString::from(format!("{buffer}\u{2502}")))
                        .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                            this.on_rename_key(id_key.clone(), ev, cx)
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(btn("rn-do", "Rename", c, true).on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| {
                                let name = match &this.modal {
                                    Some(Modal::Conflict {
                                        renaming: Some(n), ..
                                    }) => n.trim().to_string(),
                                    _ => String::new(),
                                };
                                if !name.is_empty() {
                                    this.resolve_conflict_choice(
                                        id_owned.clone(),
                                        "rename",
                                        Some(name),
                                        cx,
                                    );
                                }
                            },
                        )))
                        .child(btn("rn-back", "Back", c, false).on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                if let Some(Modal::Conflict { renaming, .. }) = &mut this.modal {
                                    *renaming = None;
                                }
                                cx.notify();
                            },
                        ))),
                );
            return root.into_any_element();
        }

        let mk = |label: &'static str, choice: &'static str, primary: bool| {
            let id_owned = job_id.to_string();
            btn(label, label, c, primary).on_click(cx.listener(
                move |this, _: &ClickEvent, _w, cx| {
                    this.resolve_conflict_choice(id_owned.clone(), choice, None, cx)
                },
            ))
        };
        let seed = suggested_rename(&name);
        root = root.child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .child(mk("Overwrite", "overwrite", true))
                .child(mk("Skip", "skip", false))
                .child(
                    btn("rn-open", "Rename\u{2026}", c, false).on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            if let Some(Modal::Conflict { renaming, .. }) = &mut this.modal {
                                *renaming = Some(seed.clone());
                            }
                            cx.notify();
                        },
                    )),
                )
                .child(mk("Overwrite all", "overwrite_all", false))
                .child(mk("Skip all", "skip_all", false)),
        );
        root.into_any_element()
    }

    fn render_file_error_body(
        &self,
        job_id: &str,
        path: &str,
        error: &str,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mk = |label: &'static str, choice: &'static str, primary: bool| {
            let id_owned = job_id.to_string();
            btn(label, label, c, primary).on_click(cx.listener(
                move |this, _: &ClickEvent, _w, cx| {
                    this.resolve_file_error(id_owned.clone(), choice, cx)
                },
            ))
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("File failed to transfer"),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(c.muted)
                    .child(SharedString::from(format!("Couldn't transfer {path}"))),
            )
            .child(
                div()
                    .text_xs()
                    .font_family("monospace")
                    .text_color(c.err)
                    .child(SharedString::from(error.to_string())),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap_2()
                    .child(mk("Skip this file", "skip", true))
                    .child(mk("Skip all errors", "skip_all", false))
                    .child(mk("Abort transfer", "abort", false)),
            )
            .into_any_element()
    }
}

fn btn(
    id: &'static str,
    label: &'static str,
    c: Colors,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .when(primary, |d| d.bg(c.accent).text_color(c.bg))
        .when(!primary, |d| d.bg(c.border).text_color(c.fg))
        .hover(|s| s.opacity(0.85))
        .child(label)
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn job(total: u64, done: u64, status: TransferStatus) -> TransferJob {
        TransferJob {
            id: "j".into(),
            session_id: "s".into(),
            src_path: "/a/b.txt".into(),
            dest_path: "/c/b.txt".into(),
            direction: TransferDirection::Upload,
            status,
            bytes_total: total,
            bytes_transferred: done,
            speed_bps: 0.0,
            skipped_count: 0,
        }
    }

    #[test]
    fn percent_clamps_and_completes() {
        assert_eq!(percent(&job(0, 0, TransferStatus::Running)), 0);
        assert_eq!(percent(&job(100, 50, TransferStatus::Running)), 50);
        assert_eq!(percent(&job(100, 999, TransferStatus::Running)), 100);
        assert_eq!(percent(&job(0, 0, TransferStatus::Completed)), 100);
    }

    #[test]
    fn format_bytes_scales() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn base_name_last_segment() {
        assert_eq!(base_name("/a/b/c.txt"), "c.txt");
        assert_eq!(base_name("/a/b"), "b");
        assert_eq!(base_name("plain"), "plain");
    }

    #[test]
    fn suggested_rename_inserts_before_first_dot() {
        assert_eq!(suggested_rename("report.tar.gz"), "report_1.tar.gz");
        assert_eq!(suggested_rename("notes"), "notes_1");
        assert_eq!(suggested_rename(".bashrc"), ".bashrc_1");
    }

    #[test]
    fn bus_event_decodes_progress_and_step() {
        let j = serde_json::json!({
            "id": "1", "session_id": "s", "src_path": "/a", "dest_path": "/b",
            "direction": "download", "status": "running", "bytes_total": 10,
            "bytes_transferred": 5, "speed_bps": 0.0, "skipped_count": 0
        });
        assert!(matches!(
            TransferBusEvent::from_raw("transfer_progress", &j),
            Some(TransferBusEvent::Progress(_))
        ));
        let s = serde_json::json!({ "job_id": "1", "ts": 42, "message": "hi" });
        assert!(matches!(
            TransferBusEvent::from_raw("transfer_step", &s),
            Some(TransferBusEvent::Step { ts: 42, .. })
        ));
        assert!(TransferBusEvent::from_raw("unrelated", &s).is_none());
    }

    #[test]
    fn status_label_covers_all() {
        assert_eq!(status_label(&TransferStatus::Queued), "queued");
        assert_eq!(status_label(&TransferStatus::Failed("x".into())), "failed");
        assert_eq!(status_label(&TransferStatus::Completed), "done");
    }
}
