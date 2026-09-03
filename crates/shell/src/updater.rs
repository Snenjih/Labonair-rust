//! In-app auto-updater (T15-005).
//!
//! Port of `reference-src/src/modules/updater/` (`updaterStore.ts`,
//! `UpdaterDialog.tsx`, `useUpdater.ts`) which drove `tauri-plugin-updater`.
//! Tauri is gone, so the flow is reimplemented on top of
//! [`labonair_backend::modules::updater`]:
//!
//! * check — fetch `latest.json`, compare versions (6 h auto-cadence, or
//!   forced from the menu / settings);
//! * install — download the artifact with a progress bar, verify its minisign
//!   signature, swap the `.app` bundle and relaunch;
//! * every failure is surfaced through the notification system (T04-004).
//!
//! The dialog markup mirrors the reference `UpdaterDialog` states
//! (available / downloading / ready) and its exact button labels.

use std::time::Duration;

use gpui::{
    div, px, ClickEvent, Context, Entity, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::updater as backend;
use labonair_backend::AvailableUpdate;
use tokio::runtime::Handle as TokioHandle;

use crate::theme::ThemeStore;
use labonair_notifications::{notification_center, Notification};

const RESTART_DELAY: Duration = Duration::from_millis(700);

/// Mirrors the reference `UpdaterStatus` union.
#[derive(Debug, Clone)]
pub enum UpdaterStatus {
    Idle,
    Checking,
    UpToDate,
    Available(AvailableUpdate),
    Downloading { downloaded: u64, total: Option<u64> },
    Ready,
    Error(String),
}

impl UpdaterStatus {
    fn is_busy(&self) -> bool {
        matches!(
            self,
            UpdaterStatus::Checking | UpdaterStatus::Downloading { .. } | UpdaterStatus::Ready
        )
    }
}

enum InstallMsg {
    Progress { downloaded: u64, total: Option<u64> },
    Done(Result<(), String>),
}

pub struct UpdaterView {
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    status: UpdaterStatus,
    dialog_open: bool,
    endpoint: String,
    public_key: String,
}

impl UpdaterView {
    pub fn new(tokio: TokioHandle, theme: Entity<ThemeStore>, _cx: &mut Context<Self>) -> Self {
        Self {
            tokio,
            theme,
            status: UpdaterStatus::Idle,
            dialog_open: false,
            endpoint: backend::DEFAULT_UPDATE_ENDPOINT.to_string(),
            public_key: backend::UPDATE_PUBLIC_KEY.to_string(),
        }
    }

    pub fn status(&self) -> &UpdaterStatus {
        &self.status
    }

    pub fn close_dialog(&mut self, cx: &mut Context<Self>) {
        self.dialog_open = false;
        cx.notify();
    }

    /// Check for updates. `manual` (menu / settings) bypasses the 6 h backoff
    /// and always reports the outcome; the automatic startup check stays quiet
    /// unless an update is found.
    pub fn run_check(&mut self, manual: bool, cx: &mut Context<Self>) {
        if self.status.is_busy() {
            // A check / download is already running (reference: bail on
            // `checking | downloading | ready`). Resurface the dialog for a
            // manual trigger so the user sees the in-flight install.
            if manual
                && matches!(
                    self.status,
                    UpdaterStatus::Downloading { .. } | UpdaterStatus::Ready
                )
            {
                self.dialog_open = true;
                cx.notify();
            }
            return;
        }
        if !manual && !backend::should_auto_check() {
            return;
        }

        self.status = UpdaterStatus::Checking;
        cx.notify();

        let endpoint = self.endpoint.clone();
        let task = self
            .tokio
            .spawn(async move { backend::fetch_manifest(&endpoint).await });

        cx.spawn(async move |this, cx| {
            let result = match task.await {
                Ok(r) => r,
                Err(e) => Err(format!("update check crashed: {e}")),
            };
            backend::record_check_now();
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(manifest) => match manifest.available() {
                        Some(update) => {
                            this.status = UpdaterStatus::Available(update);
                            this.dialog_open = true;
                        }
                        None => {
                            this.status = UpdaterStatus::UpToDate;
                            if manual {
                                notification_center(cx).update(cx, |c, cx| {
                                    c.push(
                                        Notification::info(
                                            "You're up to date",
                                            format!(
                                                "Labonair {} is the latest version.",
                                                backend::CURRENT_VERSION
                                            ),
                                        ),
                                        cx,
                                    );
                                });
                            }
                        }
                    },
                    Err(err) => {
                        this.status = UpdaterStatus::Error(err.clone());
                        if manual {
                            notification_center(cx).update(cx, |c, cx| {
                                c.push(Notification::error("Update check failed", err), cx);
                            });
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Download + verify + install the pending update, then relaunch.
    pub fn install(&mut self, cx: &mut Context<Self>) {
        let UpdaterStatus::Available(update) = self.status.clone() else {
            return;
        };
        self.status = UpdaterStatus::Downloading {
            downloaded: 0,
            total: None,
        };
        self.dialog_open = true;
        cx.notify();

        let public_key = self.public_key.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<InstallMsg>();
        let progress_tx = tx.clone();
        self.tokio.spawn(async move {
            let outcome = install_flow(&update, &public_key, &progress_tx).await;
            let _ = tx.send(InstallMsg::Done(outcome));
        });

        cx.spawn(async move |this, cx| {
            while let Some(msg) = rx.recv().await {
                match msg {
                    InstallMsg::Progress { downloaded, total } => {
                        let _ = this.update(cx, |this, cx| {
                            if matches!(this.status, UpdaterStatus::Downloading { .. }) {
                                this.status = UpdaterStatus::Downloading { downloaded, total };
                                cx.notify();
                            }
                        });
                    }
                    InstallMsg::Done(Ok(())) => {
                        let _ = this.update(cx, |this, cx| {
                            this.status = UpdaterStatus::Ready;
                            cx.notify();
                        });
                        cx.background_executor().timer(RESTART_DELAY).await;
                        if let Some(bundle) = backend::current_app_bundle() {
                            backend::relaunch(&bundle);
                        }
                        // No bundle (dev / non-.app run) — just report.
                        let _ = this.update(cx, |this, cx| {
                            notification_center(cx).update(cx, |c, cx| {
                                c.push(
                                    Notification::success(
                                        "Update installed",
                                        "Restart Labonair to finish updating.",
                                    ),
                                    cx,
                                );
                            });
                            this.dialog_open = false;
                            cx.notify();
                        });
                        break;
                    }
                    InstallMsg::Done(Err(err)) => {
                        let _ = this.update(cx, |this, cx| {
                            this.status = UpdaterStatus::Error(err.clone());
                            this.dialog_open = false;
                            notification_center(cx).update(cx, |c, cx| {
                                c.push(Notification::error("Update failed", err), cx);
                            });
                            cx.notify();
                        });
                        break;
                    }
                }
            }
        })
        .detach();
    }
}

async fn install_flow(
    update: &AvailableUpdate,
    public_key: &str,
    tx: &tokio::sync::mpsc::UnboundedSender<InstallMsg>,
) -> Result<(), String> {
    let bytes = backend::download_update(&update.url, |p| {
        let _ = tx.send(InstallMsg::Progress {
            downloaded: p.downloaded,
            total: p.total,
        });
    })
    .await?;

    backend::verify_update(&bytes, &update.signature, public_key)?;

    let bundle = backend::current_app_bundle()
        .ok_or("not running from an installed .app bundle — cannot self-update")?;
    let bytes_moved = bytes;
    tokio::task::spawn_blocking(move || backend::apply_macos_update(&bytes_moved, &bundle))
        .await
        .map_err(|e| format!("install task crashed: {e}"))?
}

/// Release-notes line for the "available" state — trimmed, capped at 120 chars
/// with an ellipsis, matching the reference `UpdaterDialog`.
fn notes_summary(notes: Option<&str>) -> String {
    match notes.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if s.chars().count() > 120 => {
            format!("{}…", s.chars().take(120).collect::<String>().trim_end())
        }
        Some(s) => s.to_string(),
        None => "A new version is ready to install.".to_string(),
    }
}

fn format_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    }
}

impl Render for UpdaterView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.dialog_open
            && matches!(
                self.status,
                UpdaterStatus::Available(_)
                    | UpdaterStatus::Downloading { .. }
                    | UpdaterStatus::Ready
            );
        if !visible {
            return div().into_any_element();
        }

        let t = self.theme.read(cx);
        let (fg, muted, border, card) =
            (t.foreground(), t.muted_foreground(), t.border(), t.card());
        let primary = t.primary();
        let success = t.status_success();

        let ready = matches!(self.status, UpdaterStatus::Ready);
        let downloading = matches!(self.status, UpdaterStatus::Downloading { .. });

        let (title, subtitle): (SharedString, SharedString) = match &self.status {
            UpdaterStatus::Ready => (
                "Update ready to install".into(),
                "Labonair will restart to finish installing.".into(),
            ),
            UpdaterStatus::Downloading { downloaded, total } => {
                let sub = match total {
                    Some(total) if *total > 0 => {
                        let pct = ((*downloaded as f64 / *total as f64) * 100.0).min(100.0);
                        format!("{:.0}% — {}", pct, format_bytes(*downloaded))
                    }
                    _ => format_bytes(*downloaded),
                };
                ("Downloading update…".into(), sub.into())
            }
            UpdaterStatus::Available(update) => (
                format!("Labonair {} is available", update.version).into(),
                notes_summary(update.notes.as_deref()).into(),
            ),
            _ => (SharedString::new_static(""), SharedString::new_static("")),
        };

        let progress_ratio = match &self.status {
            UpdaterStatus::Downloading {
                downloaded,
                total: Some(total),
            } if *total > 0 => Some((*downloaded as f32 / *total as f32).clamp(0.0, 1.0)),
            _ => None,
        };

        let icon_bg = if ready {
            primary.opacity(0.12)
        } else {
            success.opacity(0.12)
        };
        let icon_fg = if ready { primary } else { success };

        let header = div()
            .flex()
            .items_start()
            .gap_3()
            .px_5()
            .py_4()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .mt(px(2.0))
                    .flex_shrink_0()
                    .size_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_lg()
                    .bg(icon_bg)
                    .text_color(icon_fg)
                    .child("\u{2193}"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(fg)
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(px(2.0))
                            .text_xs()
                            .text_color(muted)
                            .child(subtitle),
                    ),
            );

        let progress_bar = downloading.then(|| {
            let filled = progress_ratio.unwrap_or(0.15);
            div().px_5().py_3().border_b_1().border_color(border).child(
                div().h(px(6.0)).w_full().rounded_full().bg(border).child(
                    div()
                        .h_full()
                        .rounded_full()
                        .bg(primary)
                        .w(gpui::relative(filled)),
                ),
            )
        });

        let footer = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .px_5()
            .py_3()
            .children(match &self.status {
                UpdaterStatus::Available(_) => vec![
                    self.btn("updater-later", "Later", false, cx, |this, cx| {
                        this.close_dialog(cx)
                    }),
                    self.btn(
                        "updater-install",
                        "Install & restart",
                        true,
                        cx,
                        |this, cx| this.install(cx),
                    ),
                ],
                UpdaterStatus::Downloading { .. } => {
                    vec![self.btn("updater-installing", "Installing…", false, cx, |_, _| {})]
                }
                UpdaterStatus::Ready => {
                    vec![self.btn("updater-close", "Close", false, cx, |this, cx| {
                        this.close_dialog(cx)
                    })]
                }
                _ => vec![],
            });

        div()
            .id("updater-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(crate::theme::modal_scrim())
            .child(
                div()
                    .id("updater-card")
                    .w(px(400.0))
                    .flex()
                    .flex_col()
                    .rounded_lg()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .overflow_hidden()
                    .on_click(|_, _w, cx| cx.stop_propagation())
                    .child(header)
                    .children(progress_bar)
                    .child(footer),
            )
            .into_any_element()
    }
}

impl UpdaterView {
    #[allow(clippy::type_complexity)]
    fn btn(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> gpui::AnyElement {
        let t = self.theme.read(cx);
        let (fg, muted, accent, border, on_primary) = (
            t.foreground(),
            t.muted_foreground(),
            t.primary(),
            t.border(),
            t.background(),
        );
        let mut el = div()
            .id(id)
            .h(px(28.0))
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .text_xs()
            .cursor_pointer()
            .child(label);
        el = if primary {
            el.bg(accent).text_color(on_primary)
        } else {
            el.text_color(muted)
                .hover(|s| s.text_color(fg))
                .border_1()
                .border_color(border)
        };
        el.on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| on_click(this, cx)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext};
    use labonair_backend::AvailableUpdate;

    fn view(cx: &mut TestAppContext) -> (Entity<UpdaterView>, tokio::runtime::Runtime) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let handle = rt.handle().clone();
        let view = cx.update(|cx| {
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Dark));
            cx.new(|cx| UpdaterView::new(handle, theme, cx))
        });
        (view, rt)
    }

    fn sample() -> AvailableUpdate {
        AvailableUpdate {
            version: "9.9.9".into(),
            notes: Some("  Faster startup.  ".into()),
            pub_date: None,
            url: "https://example.invalid/Labonair.app.tar.gz".into(),
            signature: String::new(),
        }
    }

    #[test]
    fn bytes_format() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn notes_are_trimmed_and_capped() {
        assert_eq!(notes_summary(Some("  hi  ")), "hi");
        assert_eq!(
            notes_summary(Some("   ")),
            "A new version is ready to install."
        );
        assert_eq!(notes_summary(None), "A new version is ready to install.");
        let long = "x".repeat(200);
        let out = notes_summary(Some(&long));
        assert!(out.ends_with('…') && out.chars().count() == 121);
    }

    #[gpui::test]
    fn dialog_visibility_follows_status(cx: &mut TestAppContext) {
        let (view, _rt) = view(cx);
        view.update(cx, |v, _| assert!(!v.dialog_open));
        view.update(cx, |v, cx| {
            v.status = UpdaterStatus::Available(sample());
            v.dialog_open = true;
            cx.notify();
        });
        view.update(cx, |v, cx| v.close_dialog(cx));
        view.read_with(cx, |v, _| assert!(!v.dialog_open));
    }

    #[gpui::test]
    fn install_is_a_noop_without_a_pending_update(cx: &mut TestAppContext) {
        let (view, _rt) = view(cx);
        view.update(cx, |v, cx| v.install(cx));
        view.read_with(cx, |v, _| {
            assert!(matches!(v.status, UpdaterStatus::Idle));
        });
    }

    #[gpui::test]
    fn manual_check_while_downloading_reopens_dialog(cx: &mut TestAppContext) {
        let (view, _rt) = view(cx);
        view.update(cx, |v, cx| {
            v.status = UpdaterStatus::Downloading {
                downloaded: 10,
                total: Some(100),
            };
            v.dialog_open = false;
            v.run_check(true, cx);
        });
        view.read_with(cx, |v, _| {
            assert!(v.dialog_open);
            assert!(matches!(v.status, UpdaterStatus::Downloading { .. }));
        });
    }

    #[gpui::test]
    fn manual_check_enters_checking_state(cx: &mut TestAppContext) {
        let (view, _rt) = view(cx);
        view.update(cx, |v, cx| {
            v.endpoint = "http://127.0.0.1:0/latest.json".into();
            v.run_check(true, cx);
            assert!(matches!(v.status, UpdaterStatus::Checking));
        });
    }
}
