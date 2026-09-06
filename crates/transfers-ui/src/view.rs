//! Transfer queue UI (T08-002).
//!
//! Ported from `reference-src/src/modules/sftp/store/transferStore.ts` +
//! `reference-src/src/modules/header/components/TransferDropdown.tsx`. The React
//! version is a Zustand store fed by four Tauri events (`transfer_progress`,
//! `transfer_step`, `file_conflict`, `file_error`) plus a header popover that
//! lists jobs with a live progress bar, a per-job step log, a cancel button and
//! two modal dialogs (conflict / file-error).
//!
//! The UI receives a typed event source and sends actions through an injected
//! [`labonair_transfers::TransferService`]. Lifecycle state is owned by the
//! UI-free [`labonair_transfers::TransferRegistry`], not by Workspace.
//!
//! Deviations from the reference:
//! * The popover is anchored to the status-bar item; it is not rendered as a
//!   workspace overlay.
//! * The conflict modal's "Rename" uses an auto-generated `name_1.ext` seed
//!   (editable) instead of a free-form field with the original name.

use std::collections::HashSet;
use std::sync::Arc;

use gpui::{
    div, px, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use tokio::runtime::Handle as TokioHandle;

use labonair_theme::store::ThemeStore;
use labonair_transfers::{
    RegistryUpdate, ResolveRequest, TransferDirection, TransferEvent, TransferEventError,
    TransferEventSource, TransferJob, TransferRegistry, TransferResolution, TransferService,
    TransferSnapshot, TransferStatus,
};
use labonair_ui_kit::{
    button, indicator, ButtonSize, ButtonVariant, IndicatorSize, ListItem, Palette,
};

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
pub enum TransferUiEvent {
    Completed {
        session_id: String,
        direction: TransferDirection,
    },
}

pub struct TransfersView {
    service: Arc<dyn TransferService>,
    registry: TransferRegistry,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    expanded_logs: HashSet<String>,
    modal: Option<Modal>,
    /// Panel open/closed.
    open: bool,
    focus: FocusHandle,
    dialog_focus: FocusHandle,
}

impl EventEmitter<TransferUiEvent> for TransfersView {}

impl Focusable for TransfersView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TransfersView {
    pub fn new(
        service: Arc<dyn TransferService>,
        events: Arc<dyn TransferEventSource>,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        let this = Self {
            service,
            registry: TransferRegistry::default(),
            tokio,
            theme,
            expanded_logs: HashSet::new(),
            modal: None,
            open: false,
            focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
        };
        let view = cx.entity().downgrade();
        let service = this.service.clone();
        let tokio = this.tokio.clone();
        let mut receiver = events.subscribe();
        cx.spawn(async move |_, cx| loop {
            let event = match receiver.recv().await {
                Ok(event) => event,
                Err(TransferEventError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "transfer event stream lagged");
                    continue;
                }
                Err(TransferEventError::Closed) => break,
            };
            let update = match view.update(cx, |this, cx| this.apply(event, cx)) {
                Ok(update) => update,
                Err(_) => break,
            };
            if let RegistryUpdate::Resolve(requests) = update {
                for request in requests {
                    let service = service.clone();
                    tokio.spawn(async move {
                        let _ = service.resolve(request.job_id, request.resolution).await;
                    });
                }
            }
        })
        .detach();
        this
    }

    // ── event intake ───────────────────────────────────────────────────────

    pub fn apply(&mut self, event: TransferEvent, cx: &mut Context<Self>) -> RegistryUpdate {
        let modal = match &event {
            TransferEvent::Conflict { job_id, .. } => Some((job_id.clone(), false)),
            TransferEvent::FileError { job_id, .. } => Some((job_id.clone(), true)),
            _ => None,
        };
        let was_empty = self.registry.snapshot().is_empty();
        let update = self.registry.apply(event);
        if was_empty && !self.registry.snapshot().is_empty() {
            // A newly queued transfer should be visible immediately. The
            // transfer module owns this presentation policy; Workspace only
            // submits the request.
            self.open = true;
        }
        if let Some((job_id, is_file_error)) = modal {
            if self.modal.is_none()
                && self
                    .registry
                    .snapshot()
                    .iter()
                    .find(|record| record.job.id == job_id)
                    .map(|record| {
                        if is_file_error {
                            record.file_error.is_some()
                        } else {
                            record.conflict.is_some()
                        }
                    })
                    .unwrap_or(false)
            {
                self.modal = Some(if is_file_error {
                    Modal::FileError { job_id }
                } else {
                    Modal::Conflict {
                        job_id,
                        renaming: None,
                    }
                });
            }
        }
        if let Some(modal) = &self.modal {
            let job_id = match modal {
                Modal::Conflict { job_id, .. } | Modal::FileError { job_id } => job_id,
            };
            if self
                .registry
                .snapshot()
                .iter()
                .find(|record| &record.job.id == job_id)
                .map(|record| record.conflict.is_none() && record.file_error.is_none())
                .unwrap_or(true)
            {
                self.modal = None;
            }
        }
        cx.notify();
        if let RegistryUpdate::Completed {
            session_id,
            direction,
        } = &update
        {
            cx.emit(TransferUiEvent::Completed {
                session_id: session_id.clone(),
                direction: *direction,
            });
        }
        update
    }

    // ── actions ────────────────────────────────────────────────────────────

    /// Open the queue panel (called when a new transfer is enqueued).
    pub fn reveal(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn total_count(&self) -> usize {
        self.registry.snapshot().len()
    }

    /// Number of jobs still queued or running — the `transfers` statusbar
    /// item (T18-004) is only visible/active while this is non-zero.
    pub fn active_count(&self) -> usize {
        self.registry.active_count()
    }

    pub fn cancel(&mut self, id: String, cx: &mut Context<Self>) {
        let service = self.service.clone();
        self.tokio.spawn(async move {
            let _ = service.cancel(id).await;
        });
        cx.notify();
    }

    pub fn clear_completed(&mut self, cx: &mut Context<Self>) {
        self.registry.clear_completed();
        let kept: HashSet<String> = self
            .registry
            .snapshot()
            .into_iter()
            .map(|record| record.job.id)
            .collect();
        self.expanded_logs.retain(|k| kept.contains(k));
        cx.notify();
    }

    fn send_resolution(
        &mut self,
        id: &str,
        resolution: TransferResolution,
        cx: &mut Context<Self>,
    ) {
        let update = self.registry.resolve(id, resolution);
        let RegistryUpdate::Resolve(requests) = update else {
            return;
        };
        self.dispatch_resolutions(requests);
        cx.notify();
    }

    fn dispatch_resolutions(&self, requests: Vec<ResolveRequest>) {
        let service = self.service.clone();
        let tokio = self.tokio.clone();
        tokio.spawn(async move {
            for request in requests {
                let _ = service.resolve(request.job_id, request.resolution).await;
            }
        });
    }

    /// Resolve one conflict from the modal. `overwrite_all` / `skip_all` set the
    /// session sticky policy and fan out to every already-paused sibling.
    fn resolve_conflict_choice(
        &mut self,
        job_id: String,
        choice: &str,
        _new_name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.modal = None;
        let resolution = match choice {
            "overwrite" | "overwrite_all" => TransferResolution::Overwrite,
            "skip" | "skip_all" => TransferResolution::Skip,
            "rename" => TransferResolution::Rename(_new_name.unwrap_or_default()),
            _ => return,
        };
        self.send_resolution(&job_id, resolution, cx);
    }

    fn resolve_file_error(&mut self, job_id: String, choice: &str, cx: &mut Context<Self>) {
        self.modal = None;
        let resolution = match choice {
            "skip" => TransferResolution::Skip,
            "skip_all" => TransferResolution::SkipAll,
            "abort" => TransferResolution::Abort,
            _ => return,
        };
        self.send_resolution(&job_id, resolution, cx);
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
    /// The full token snapshot the ui-kit primitives (`button`, `ListItem`, …)
    /// are styled from.
    palette: Palette,
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
                palette: Palette::from_theme(t),
            }
        };

        if let Some(modal) = self.render_modal(c, cx) {
            return modal;
        }

        let records = self.registry.snapshot();
        if records.is_empty() || !self.open {
            return div().into_any_element();
        }

        div()
            .absolute()
            .bottom(px(24.0))
            .right(px(0.0))
            .child(self.render_panel(c, cx))
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
                button(
                    "transfers-clear",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::Xs,
                )
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
        for record in self.registry.snapshot() {
            list = list.child(self.render_job(&record, c, cx));
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

    fn render_job(
        &self,
        record: &TransferSnapshot,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let job = &record.job;
        let name = base_name(&job.dest_path).to_string();
        let pct = percent(job);
        let label = status_label(&job.status);
        let tint = match &job.status {
            TransferStatus::Running => c.info,
            TransferStatus::Completed => c.ok,
            TransferStatus::Paused => c.warn,
            TransferStatus::Failed(_) => c.err,
            TransferStatus::Cancelled | TransferStatus::Queued => c.muted,
        };
        let arrow = if matches!(job.direction, TransferDirection::Download) {
            "\u{2193}"
        } else {
            "\u{2191}"
        };
        let id = job.id.clone();
        let id_cancel = job.id.clone();
        let id_log = job.id.clone();
        let has_steps = !record.steps.is_empty();
        let log_open = self.expanded_logs.contains(&job.id);
        let active = is_active(&job.status);
        // T20-003: migrated to the shared `ListItem` primitive — the status
        // dot is the shared `Indicator`, and the log-toggle/cancel actions
        // are shared `button()`s collected into `ListItem::trailing`. The
        // row itself has no click handler (only the trailing buttons do), so
        // `ListItem`'s default hover/cursor-pointer chrome is turned off via
        // its `.extra()` escape hatch to avoid implying the whole row is
        // clickable.
        let mut trailing = div().flex().flex_row().items_center().gap_1();
        if has_steps {
            trailing = trailing.child(
                button(
                    SharedString::from(format!("transfer-logtoggle-{}", id_log)),
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
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
            trailing = trailing.child(
                button(
                    SharedString::from(format!("transfer-cancel-{}", id_cancel)),
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child("\u{2715}")
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.cancel(id_cancel.clone(), cx)
                })),
            );
        }

        let header_row = ListItem::new(
            SharedString::from(format!("transfer-header-{id}")),
            c.fg,
            c.muted,
            c.border,
        )
        .child(indicator(IndicatorSize::Sm, tint))
        .child(div().text_xs().child(arrow))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .child(SharedString::from(name)),
        )
        .child(
            div()
                .px_1()
                .rounded_sm()
                .text_xs()
                .text_color(tint)
                .child(SharedString::from(if job.skipped_count > 0 {
                    format!("{label} \u{00b7} {} skipped", job.skipped_count)
                } else {
                    label.to_string()
                })),
        )
        .trailing(trailing)
        .extra(|row| row.cursor_default());

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

        // Failure details stay in the actionable file-error modal, where the
        // user can choose Skip, Skip all, or Abort. The list row intentionally
        // exposes only the stable status label so it does not become a second
        // feature-local error surface next to Notifications.
        if log_open && !record.steps.is_empty() {
            let mut log = div()
                .id(SharedString::from(format!(
                    "transfer-log-{}",
                    record.job.id
                )))
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
            for s in &record.steps {
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

        container.into_any_element()
    }

    fn render_modal(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let modal = self.modal.as_ref()?;
        let (job_id, is_file_error) = match modal {
            Modal::Conflict { job_id, .. } => (job_id.clone(), false),
            Modal::FileError { job_id } => (job_id.clone(), true),
        };
        let row = self
            .registry
            .snapshot()
            .into_iter()
            .find(|record| record.job.id == job_id)?;

        let body = if is_file_error {
            let file_error = row.file_error.as_ref()?;
            let path = file_error.path.clone();
            let error = file_error.error.clone();
            self.render_file_error_body(&job_id, &path, &error, c, cx)
        } else {
            let dest = row
                .conflict
                .as_ref()
                .map(|conflict| conflict.destination_path.clone())
                .unwrap_or_else(|| row.job.dest_path.clone());
            self.render_conflict_body(&job_id, &dest, c, cx)
        };

        Some(
            div()
                .absolute()
                .bottom(px(24.0))
                .right(px(0.0))
                .w(px(420.0))
                .bg(labonair_theme::store::modal_scrim())
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

/// Thin wrapper over the shared `button()` primitive — mirrors
/// `panel-snippets`'s `btn()` (T20-003). `primary` maps to `Default`, every
/// other modal action to `Outline`.
fn btn(
    id: &'static str,
    label: &'static str,
    c: Colors,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if primary {
        ButtonVariant::Default
    } else {
        ButtonVariant::Outline
    };
    button(id, c.palette, variant, ButtonSize::Xs).child(label)
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
    fn typed_event_decoder_is_available_in_the_core_contract() {
        let j = serde_json::json!({
            "id": "1", "session_id": "s", "src_path": "/a", "dest_path": "/b",
            "direction": "download", "status": "running", "bytes_total": 10,
            "bytes_transferred": 5, "speed_bps": 0.0, "skipped_count": 0
        });
        assert!(matches!(
            TransferEvent::from_raw("transfer_progress", &j),
            Some(TransferEvent::Progress(_))
        ));
        let s = serde_json::json!({ "job_id": "1", "ts": 42, "message": "hi" });
        assert!(matches!(
            TransferEvent::from_raw("transfer_step", &s),
            Some(TransferEvent::Step { timestamp: 42, .. })
        ));
        assert!(TransferEvent::from_raw("unrelated", &s).is_none());
    }

    #[test]
    fn status_label_covers_all() {
        assert_eq!(status_label(&TransferStatus::Queued), "queued");
        assert_eq!(status_label(&TransferStatus::Failed("x".into())), "failed");
        assert_eq!(status_label(&TransferStatus::Completed), "done");
    }
}
