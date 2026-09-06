//! GPUI adapter for the app-wide notification registry.
//!
//! The retained data model and lifecycle live in
//! [`labonair_notifications_core`]. This crate adds only GPUI invalidation and
//! callback-backed compatibility actions. Notifications are consumed by the
//! statusbar dropdown; this crate deliberately has no toast renderer.

use std::{collections::HashMap, time::Instant};

use gpui::{App, AppContext, Context, Entity, Global, SharedString, Window};
use labonair_notifications_core::{NotificationDraft, NotificationKind, NotificationRegistry};

mod status_item;

pub use status_item::NotificationsStatusItem;

/// Severity of a notification. This is the GPUI-facing spelling retained for
/// current feature callers; the UI-free registry uses [`NotificationKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Message,
    Info,
    Success,
    Warning,
    Error,
}

impl From<Severity> for NotificationKind {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Message => Self::Message,
            Severity::Info => Self::Info,
            Severity::Success => Self::Success,
            Severity::Warning => Self::Warning,
            Severity::Error => Self::Error,
        }
    }
}

/// Callback fired when a compatibility action is activated in the dropdown.
type ActionCallback = Box<dyn FnMut(&mut Window, &mut App) + 'static>;

/// A callback-backed action for existing callers.
///
/// New actions should eventually use a stable command/action ID rather than a
/// closure. The registry already stores action metadata separately from this
/// adapter-specific callback.
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

/// A notification draft to publish. Identity and read state are assigned by
/// the UI-free registry on insert.
#[derive(Debug)]
pub struct Notification {
    pub severity: Severity,
    pub title: SharedString,
    pub body: SharedString,
    pub source: Option<SharedString>,
    pub details: Option<SharedString>,
    pub dedupe_key: Option<SharedString>,
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
            details: None,
            dedupe_key: None,
            action: None,
        }
    }

    pub fn message(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self::new(Severity::Message, title, body)
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

    pub fn details(mut self, details: impl Into<SharedString>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn dedupe_key(mut self, key: impl Into<SharedString>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    pub fn action(mut self, action: NotificationAction) -> Self {
        self.action = Some(action);
        self
    }
}

/// Read-only view of a retained notification for the statusbar dropdown.
#[derive(Debug, Clone)]
pub struct NotificationSnapshot {
    pub id: u64,
    pub severity: Severity,
    pub title: SharedString,
    pub body: SharedString,
    pub details: Option<SharedString>,
    pub source: Option<SharedString>,
    pub action_label: Option<SharedString>,
    pub read: bool,
}

/// GPUI-facing adapter around [`NotificationRegistry`].
pub struct NotificationCenter {
    registry: NotificationRegistry,
    actions: HashMap<u64, NotificationAction>,
}

impl Default for NotificationCenter {
    fn default() -> Self {
        Self {
            registry: NotificationRegistry::new(),
            actions: HashMap::new(),
        }
    }
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish a retained notification. There is no error gate or timeout:
    /// every message reaches the notification dropdown until dismissed.
    pub fn push(&mut self, notif: Notification, cx: &mut Context<Self>) -> Option<u64> {
        self.insert(notif, Instant::now(), cx)
    }

    /// Compatibility alias for callers that distinguish action results. All
    /// user-visible messages now use the same retained registry.
    pub fn push_action_result(
        &mut self,
        notif: Notification,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        self.push(notif, cx)
    }

    /// Insert with an explicit clock value for deterministic adapter tests.
    pub fn insert(
        &mut self,
        notif: Notification,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> Option<u64> {
        let action = notif.action;
        let default_key = format!(
            "{:?}|{}|{}|{}",
            notif.severity,
            notif.title,
            notif.body,
            notif
                .source
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or_default()
        );
        let mut draft = NotificationDraft::new(
            notif.severity.into(),
            notif.title.to_string(),
            notif.body.to_string(),
        )
        .dedupe_key(
            notif
                .dedupe_key
                .as_ref()
                .map(|value| value.to_string())
                .unwrap_or(default_key),
        );
        if let Some(source) = notif.source {
            draft = draft.source(source.to_string());
        }
        if let Some(details) = notif.details {
            draft = draft.details(details.to_string());
        }
        if let Some(action) = action {
            draft = draft.action(labonair_notifications_core::NotificationAction::new(
                "default",
                action.label.to_string(),
            ));
            let id = self.registry.insert(draft, now)?;
            self.actions.insert(id, action);
            cx.notify();
            return Some(id);
        }
        let id = self.registry.insert(draft, now)?;
        cx.notify();
        Some(id)
    }

    pub fn dismiss(&mut self, id: u64, cx: &mut Context<Self>) {
        self.actions.remove(&id);
        if self.registry.dismiss(id) {
            cx.notify();
        }
    }

    pub fn mark_read(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.registry.mark_read(id) {
            cx.notify();
        }
    }

    pub fn clear_all(&mut self, cx: &mut Context<Self>) {
        self.actions.clear();
        if self.registry.clear() {
            cx.notify();
        }
    }

    pub fn len(&self) -> usize {
        self.registry.len()
    }

    pub fn unread_count(&self) -> usize {
        self.registry.unread_count()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    pub fn snapshots(&self) -> Vec<NotificationSnapshot> {
        self.registry
            .list()
            .iter()
            .map(|item| NotificationSnapshot {
                id: item.id,
                severity: match item.kind {
                    NotificationKind::Message => Severity::Message,
                    NotificationKind::Info => Severity::Info,
                    NotificationKind::Success => Severity::Success,
                    NotificationKind::Warning => Severity::Warning,
                    NotificationKind::Error => Severity::Error,
                },
                title: SharedString::from(item.title.clone()),
                body: SharedString::from(item.summary.clone()),
                details: item.details.clone().map(SharedString::from),
                source: item.source.clone().map(SharedString::from),
                action_label: item
                    .actions
                    .first()
                    .map(|action| SharedString::from(action.label.clone())),
                read: item.read,
            })
            .collect()
    }

    pub fn trigger_action(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mut action) = self.actions.remove(&id) else {
            return;
        };
        (action.callback)(window, cx);
        self.registry.dismiss(id);
        cx.notify();
    }
}

/// App-wide handle to the [`NotificationCenter`] entity.
pub struct GlobalNotificationCenter(pub Entity<NotificationCenter>);

impl Global for GlobalNotificationCenter {}

/// Creates the [`NotificationCenter`] and installs it as a global.
pub fn init(cx: &mut App) -> Entity<NotificationCenter> {
    let center = cx.new(|_| NotificationCenter::new());
    cx.set_global(GlobalNotificationCenter(center.clone()));
    center
}

/// The [`NotificationCenter`] entity from the global.
pub fn notification_center(cx: &App) -> Entity<NotificationCenter> {
    cx.global::<GlobalNotificationCenter>().0.clone()
}

/// Turns a `Result<T, String>` into a retained error notification.
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn base() -> Notification {
        Notification::info("Test", "Hello")
    }

    #[gpui::test]
    fn push_adds_newest_first_and_tracks_unread(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let center = cx.new(|_| NotificationCenter::new());
            center.update(cx, |center, cx| {
                let a = center.push(Notification::info("A", "first"), cx).unwrap();
                let b = center.push(Notification::info("B", "second"), cx).unwrap();
                assert_ne!(a, b);
                assert_eq!(center.snapshots()[0].title, "B");
                assert_eq!(center.unread_count(), 2);
                center.mark_read(b, cx);
                assert_eq!(center.unread_count(), 1);
            });
        });
    }

    #[gpui::test]
    fn equivalent_recent_messages_are_deduplicated(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let center = cx.new(|_| NotificationCenter::new());
            center.update(cx, |center, cx| {
                let now = Instant::now();
                assert!(center.insert(base(), now, cx).is_some());
                assert!(center
                    .insert(base(), now + std::time::Duration::from_millis(500), cx)
                    .is_none());
                assert!(center
                    .insert(
                        Notification::info("Test", "Other"),
                        now + std::time::Duration::from_millis(500),
                        cx,
                    )
                    .is_some());
            });
        });
    }

    #[gpui::test]
    fn details_and_clear_are_available_to_dropdown(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let center = cx.new(|_| NotificationCenter::new());
            center.update(cx, |center, cx| {
                center.push(
                    Notification::error("Failed", "Could not connect")
                        .details("The server rejected the connection."),
                    cx,
                );
                assert_eq!(
                    center.snapshots()[0]
                        .details
                        .as_ref()
                        .map(|value| value.to_string()),
                    Some("The server rejected the connection.".to_string())
                );
                center.clear_all(cx);
                assert!(center.is_empty());
            });
        });
    }
}
