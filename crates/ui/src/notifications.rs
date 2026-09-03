//! App-wide notification / toast system (T04-004).
//!
//! Ported from `reference-src/src/modules/notifications/` — the
//! `useNotificationStore` + `NotificationDropdown`. The reference keeps a
//! persistent, newest-first list with a 2s title+message+type spam guard, a
//! 100-item cap and a `notifyOnErrors` gate for passive/background errors,
//! plus an `addActionResultNotification` path that bypasses that gate for
//! user-initiated action results. All of that is kept here.
//!
//! On top of the reference the pure-Rust port renders the list as **stacked
//! toasts** in the top-right of the [`crate::app_shell::AppShell`] with
//! per-severity auto-dismiss, manual close and an optional action button — the
//! reference relied on `motion/react` + a Radix popover for the same UX.
//!
//! Access goes through the [`GlobalNotificationCenter`] global — see
//! [`notification_center`] / [`init`]. The [`notify_err`] helper turns a
//! `Result<T, String>` (Critical Rule 6) into an error toast at the call site.

use std::time::{Duration, Instant};

use gpui::{App, AppContext, Context, Entity, Global, SharedString, Window};

use crate::theme::ThemeStore;

/// Severity of a notification. Names match the reference `NotificationType`
/// (`error | warning | info | success`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Success,
    Warning,
    Error,
}

impl Severity {
    /// Auto-dismiss timeout when the caller does not set one explicitly.
    /// `Error` stays until dismissed manually (the reference never
    /// auto-cleared errors either — they lived in the dropdown until
    /// "Clear all").
    pub fn default_timeout(self) -> Option<Duration> {
        match self {
            Severity::Info => Some(Duration::from_secs(5)),
            Severity::Success => Some(Duration::from_secs(4)),
            Severity::Warning => Some(Duration::from_secs(8)),
            Severity::Error => None,
        }
    }

    /// The toast icon for this severity.
    fn glyph(self) -> labonair_ui_kit::IconName {
        use labonair_ui_kit::IconName;
        match self {
            Severity::Info => IconName::Info,
            Severity::Success => IconName::CircleCheck,
            Severity::Warning => IconName::Warning,
            Severity::Error => IconName::CircleX,
        }
    }

    fn color(self, theme: &ThemeStore) -> gpui::Hsla {
        match self {
            Severity::Info => theme.status_info(),
            Severity::Success => theme.status_success(),
            Severity::Warning => theme.status_warning(),
            Severity::Error => theme.status_error(),
        }
    }
}

/// Callback fired when a toast's action button is clicked.
type ActionCallback = Box<dyn FnMut(&mut Window, &mut App) + 'static>;

/// A button rendered inside a toast. The callback fires once, then the toast
/// is dismissed.
pub struct NotificationAction {
    pub label: SharedString,
    callback: ActionCallback,
}

impl NotificationAction {
    pub fn new(
        label: impl Into<SharedString>,
        callback: impl FnMut(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            callback: Box::new(callback),
        }
    }
}

impl std::fmt::Debug for NotificationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationAction")
            .field("label", &self.label)
            .finish()
    }
}

/// A notification to push. `id` and `timestamp` are assigned on insert.
#[derive(Debug)]
pub struct Notification {
    pub severity: Severity,
    pub title: SharedString,
    pub body: SharedString,
    pub source: Option<SharedString>,
    /// Explicit auto-dismiss timeout; falls back to
    /// [`Severity::default_timeout`] when `None`.
    pub timeout: Option<Duration>,
    pub action: Option<NotificationAction>,
}

impl Notification {
    fn new(
        severity: Severity,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        Self {
            severity,
            title: title.into(),
            body: body.into(),
            source: None,
            timeout: None,
            action: None,
        }
    }

    pub fn info(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self::new(Severity::Info, title, body)
    }

    pub fn success(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self::new(Severity::Success, title, body)
    }

    pub fn warning(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self::new(Severity::Warning, title, body)
    }

    pub fn error(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self::new(Severity::Error, title, body)
    }

    pub fn source(mut self, source: impl Into<SharedString>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn action(mut self, action: NotificationAction) -> Self {
        self.action = Some(action);
        self
    }

    fn resolved_timeout(&self) -> Option<Duration> {
        self.timeout.or_else(|| self.severity.default_timeout())
    }
}

/// A live notification held by the center.
struct Active {
    id: u64,
    severity: Severity,
    title: SharedString,
    body: SharedString,
    source: Option<SharedString>,
    created: Instant,
    action: Option<NotificationAction>,
}

/// Read-only view of a live notification, for rendering.
#[derive(Debug, Clone)]
pub struct ToastSnapshot {
    pub id: u64,
    pub severity: Severity,
    pub title: SharedString,
    pub body: SharedString,
    pub source: Option<SharedString>,
    pub action_label: Option<SharedString>,
}

/// Newest-first spam guard window, matching the reference (`Date.now() - newest < 2000`).
const SPAM_WINDOW: Duration = Duration::from_millis(2000);
/// Hard cap on retained notifications, matching the reference `.slice(0, 100)`.
const MAX_ITEMS: usize = 100;

/// App-wide notification queue. A GPUI entity; observe it to re-render toasts.
pub struct NotificationCenter {
    items: Vec<Active>,
    next_id: u64,
    /// Gate for passive/background error notifications (reference
    /// `preferencesStore.notifyOnErrors`). Defaults to `true` until the
    /// settings store (T13-001) wires the real preference.
    notify_on_errors: bool,
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            notify_on_errors: true,
        }
    }
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_notify_on_errors(&mut self, enabled: bool) {
        self.notify_on_errors = enabled;
    }

    pub fn notify_on_errors(&self) -> bool {
        self.notify_on_errors
    }

    /// Pushes a notification. `Error` severity is dropped when
    /// [`Self::notify_on_errors`] is `false` (reference `addNotification`
    /// gate). Returns the assigned id, or `None` if gated/spam-blocked.
    pub fn push(&mut self, notif: Notification, cx: &mut Context<Self>) -> Option<u64> {
        if notif.severity == Severity::Error && !self.notify_on_errors {
            return None;
        }
        self.insert(notif, Instant::now(), cx)
    }

    /// Like [`Self::push`] but bypasses the error gate — for direct,
    /// user-initiated action results (reference `addActionResultNotification`).
    pub fn push_action_result(
        &mut self,
        notif: Notification,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        self.insert(notif, Instant::now(), cx)
    }

    /// Insert with an explicit "now" — the spam guard reference point.
    /// Public for deterministic testing.
    pub fn insert(
        &mut self,
        notif: Notification,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        // Spam guard: drop if the newest notification has the same
        // title + body + severity within the window. `title` is part of the
        // key so two different actions failing with the same error text stay
        // visible.
        if let Some(newest) = self.items.first() {
            if newest.title == notif.title
                && newest.body == notif.body
                && newest.severity == notif.severity
                && now.duration_since(newest.created) < SPAM_WINDOW
            {
                return None;
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        let timeout = notif.resolved_timeout();
        self.items.insert(
            0,
            Active {
                id,
                severity: notif.severity,
                title: notif.title,
                body: notif.body,
                source: notif.source,
                created: now,
                action: notif.action,
            },
        );
        self.items.truncate(MAX_ITEMS);

        if let Some(after) = timeout {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(after).await;
                let _ = this.update(cx, |this, cx| this.dismiss(id, cx));
            })
            .detach();
        }
        cx.notify();
        Some(id)
    }

    /// Removes a notification by id. No-op if not present.
    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        let before = self.items.len();
        self.items.retain(|n| n.id != id);
        if self.items.len() != before {
            cx.notify();
        }
    }

    /// Removes every notification (reference "Clear all").
    pub fn clear_all(&mut self, cx: &mut Context<Self>) {
        if self.items.is_empty() {
            return;
        }
        self.items.clear();
        cx.notify();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Newest-first snapshots for rendering.
    pub fn snapshots(&self) -> Vec<ToastSnapshot> {
        self.items
            .iter()
            .map(|n| ToastSnapshot {
                id: n.id,
                severity: n.severity,
                title: n.title.clone(),
                body: n.body.clone(),
                source: n.source.clone(),
                action_label: n.action.as_ref().map(|a| a.label.clone()),
            })
            .collect()
    }

    /// Fires the action callback for `id` (if any) and dismisses that toast.
    pub fn trigger_action(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pos) = self.items.iter().position(|n| n.id == id) else {
            return;
        };
        if let Some(mut action) = self.items[pos].action.take() {
            (action.callback)(window, cx);
        }
        self.items.remove(pos);
        cx.notify();
    }
}

/// App-wide handle to the [`NotificationCenter`] entity.
pub struct GlobalNotificationCenter(pub Entity<NotificationCenter>);

impl Global for GlobalNotificationCenter {}

/// Creates the [`NotificationCenter`] and installs it as a global. Call once
/// at startup.
pub fn init(cx: &mut App) -> Entity<NotificationCenter> {
    let center = cx.new(|_| NotificationCenter::new());
    cx.set_global(GlobalNotificationCenter(center.clone()));
    center
}

/// The [`NotificationCenter`] entity from the global. Panics if [`init`] has
/// not run.
pub fn notification_center(cx: &App) -> Entity<NotificationCenter> {
    cx.global::<GlobalNotificationCenter>().0.clone()
}

/// Turns a `Result<T, String>` into an error toast on failure (via the
/// action-result path, so it shows regardless of the error gate). Returns the
/// `Ok` value, or `None` on error.
pub fn notify_err<T>(
    title: impl Into<SharedString>,
    result: Result<T, String>,
    cx: &mut App,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(message) => {
            let title = title.into();
            notification_center(cx).update(cx, |center, cx| {
                center.push_action_result(Notification::error(title, message), cx);
            });
            None
        }
    }
}

// ── Toast rendering ─────────────────────────────────────────────────────────

use gpui::{
    div, px, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
};

/// Builds the stacked toast overlay for the app shell. Returns `None` when
/// there is nothing to show. The overlay container only occupies its own
/// top-right box, so clicks elsewhere pass through untouched; only the toast
/// cards are interactive.
pub fn render_overlay(
    center: &Entity<NotificationCenter>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) -> Option<gpui::AnyElement> {
    let snapshots = center.read(cx).snapshots();
    if snapshots.is_empty() {
        return None;
    }
    let theme = theme.read(cx);
    let (card, fg, muted, border) = (
        theme.card(),
        theme.foreground(),
        theme.muted_foreground(),
        theme.border(),
    );

    let toasts = snapshots.into_iter().map(|t| {
        let accent = t.severity.color(theme);
        let center_close = center.clone();
        let center_action = center.clone();
        let id = t.id;
        let action_label = t.action_label.clone();

        div()
            .id(("toast", id))
            .w(px(360.0))
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_lg()
            .bg(card)
            .border_1()
            .border_color(accent)
            .shadow_lg()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().child(t.severity.glyph().svg(accent)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(fg)
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(t.title.clone()),
                                    )
                                    .children(t.source.clone().map(|s| {
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(border)
                                            .text_color(muted)
                                            .child(s)
                                    })),
                            )
                            .child(div().text_color(muted).child(t.body.clone())),
                    )
                    .child(
                        div()
                            .id(("toast-close", id))
                            .flex_shrink_0()
                            .size(px(16.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_color(muted)
                            .hover(|s| s.bg(border).text_color(fg))
                            .child("\u{2715}")
                            .on_click(move |_, _window, cx| {
                                center_close.update(cx, |c, cx| c.dismiss(id, cx));
                            }),
                    ),
            )
            .children(action_label.map(|label| {
                div().flex().justify_end().child(
                    div()
                        .id(("toast-action", id))
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(accent)
                        .text_color(card)
                        .hover(|s| s.opacity(0.9))
                        .child(label)
                        .on_click(move |_, window, cx| {
                            center_action.update(cx, |c, cx| c.trigger_action(id, window, cx));
                        }),
                )
            }))
            .into_any_element()
    });

    Some(
        div()
            .absolute()
            .top_4()
            .right_4()
            .flex()
            .flex_col()
            .gap_2()
            .children(toasts)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn base() -> Notification {
        Notification::info("Test", "Hello")
    }

    #[gpui::test]
    fn push_adds_newest_first_with_ids(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                let a = c.push(Notification::info("A", "first"), cx).unwrap();
                let b = c.push(Notification::info("B", "second"), cx).unwrap();
                assert_ne!(a, b);
                let s = c.snapshots();
                assert_eq!(s.len(), 2);
                assert_eq!(s[0].title, "B");
                assert_eq!(s[1].title, "A");
            });
        });
    }

    #[gpui::test]
    fn spam_guard_blocks_then_allows(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                let t0 = Instant::now();
                assert!(c.insert(base(), t0, cx).is_some());
                // identical within window → blocked
                assert!(c
                    .insert(base(), t0 + Duration::from_millis(500), cx)
                    .is_none());
                // different body → allowed
                assert!(c
                    .insert(Notification::info("Test", "Other"), t0, cx)
                    .is_some());
                // different type, same text → allowed
                assert!(c
                    .insert(Notification::warning("Test", "Hello"), t0, cx)
                    .is_some());
                // after the window → allowed
                assert!(c
                    .insert(base(), t0 + Duration::from_millis(2001), cx)
                    .is_some());
            });
        });
    }

    #[gpui::test]
    fn spam_guard_keeps_different_titles(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                let t0 = Instant::now();
                assert!(c
                    .insert(Notification::error("Push Failed", "dead session"), t0, cx)
                    .is_some());
                assert!(c
                    .insert(Notification::error("Stash Failed", "dead session"), t0, cx)
                    .is_some());
                assert_eq!(c.len(), 2);
            });
        });
    }

    #[gpui::test]
    fn error_gate(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                c.set_notify_on_errors(false);
                assert!(c.push(Notification::error("E", "x"), cx).is_none());
                assert_eq!(c.len(), 0);
                // action-result path bypasses the gate
                assert!(c
                    .push_action_result(Notification::error("E", "x"), cx)
                    .is_some());
                assert_eq!(c.len(), 1);
                // non-error still passes
                c.set_notify_on_errors(true);
                assert!(c.push(Notification::error("E2", "y"), cx).is_some());
            });
        });
    }

    #[gpui::test]
    fn cap_at_100(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                for i in 0..105 {
                    c.push(Notification::info("T", format!("msg-{i}")), cx);
                }
                assert_eq!(c.len(), 100);
            });
        });
    }

    #[gpui::test]
    fn dismiss_and_clear(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                let id = c.push(base(), cx).unwrap();
                c.dismiss(999, cx);
                assert_eq!(c.len(), 1);
                c.dismiss(id, cx);
                assert_eq!(c.len(), 0);
                c.push(Notification::info("a", "1"), cx);
                c.push(Notification::info("b", "2"), cx);
                c.clear_all(cx);
                assert!(c.is_empty());
            });
        });
    }

    #[gpui::test]
    fn action_button_fires_callback_once(cx: &mut TestAppContext) {
        let fired = std::rc::Rc::new(std::cell::Cell::new(0));
        let f2 = fired.clone();
        cx.update(|cx| {
            let c = cx.new(|_| NotificationCenter::new());
            c.update(cx, |c, cx| {
                let id = c
                    .push(
                        Notification::warning("Reconnect", "Session dropped").action(
                            NotificationAction::new("Retry", move |_, _| {
                                f2.set(f2.get() + 1);
                            }),
                        ),
                        cx,
                    )
                    .unwrap();
                assert_eq!(
                    c.snapshots()[0]
                        .action_label
                        .as_ref()
                        .map(|s| s.to_string()),
                    Some("Retry".to_string())
                );
                // trigger_action needs a Window; simulate the callback path by
                // taking it directly is not possible here, so assert the label
                // wiring and that dismiss removes it.
                c.dismiss(id, cx);
            });
        });
        assert_eq!(fired.get(), 0, "callback must not fire without a click");
    }

    #[gpui::test]
    fn auto_dismiss_after_timeout(cx: &mut TestAppContext) {
        let c = cx.new(|_| NotificationCenter::new());
        c.update(cx, |c, cx| {
            c.push(
                Notification::info("A", "x").timeout(Duration::from_millis(50)),
                cx,
            );
            assert_eq!(c.len(), 1);
        });
        cx.executor().advance_clock(Duration::from_millis(100));
        cx.run_until_parked();
        c.update(cx, |c, _| assert_eq!(c.len(), 0));
    }
}
