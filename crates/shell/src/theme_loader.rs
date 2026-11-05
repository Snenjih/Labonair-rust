//! User theme files: where they live, (re)loading them into the `ThemeStore`
//! and reporting skipped files.
//!
//! The registry and its validation (a family needs a light and a dark variant)
//! belong to `labonair-theme`; this composition adapter owns the folder, the
//! live-reload watch, and turns every rejected file into one notification with
//! an "Open themes folder" action (R08-010 action ids, handled here).

use std::path::PathBuf;

use gpui::{App, Entity};
use labonair_notifications::{notification_center, Notification, NotificationActionEvent};
use labonair_theme::registry::RejectedTheme;
use labonair_theme::ThemeStore;

/// Stable id of the notification action that reveals the themes folder.
const OPEN_THEMES_FOLDER: &str = "themes.open-folder";

/// The folder user theme files are read from.
pub(crate) fn themes_dir() -> PathBuf {
    labonair_filesystem::paths::config_dir().join("themes")
}

/// One notification for one skipped theme file.
fn rejection_notification(rejected: &RejectedTheme) -> Notification {
    let file = rejected
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| rejected.path.display().to_string());
    Notification::warning(
        format!("Theme \u{201C}{file}\u{201D} ignored"),
        rejected.reason.to_string(),
    )
    .source("themes")
    .action(OPEN_THEMES_FOLDER, "Open themes folder")
}

/// Rescan the folder, apply the persisted selection and report skipped files.
fn reload(theme: &Entity<ThemeStore>, cx: &mut App) {
    let rejected = labonair_theme_ui::reload_user_themes(theme, &themes_dir(), cx);
    if rejected.is_empty() {
        return;
    }
    notification_center(cx).update(cx, |center, cx| {
        for file in &rejected {
            center.push(rejection_notification(file), cx);
        }
    });
}

/// Load user themes now, keep them live while the app runs, and handle the
/// "Open themes folder" notification action. Call once at startup.
pub(crate) fn load_and_watch(theme: &Entity<ThemeStore>, cx: &mut App) {
    reload(theme, cx);

    let watched = theme.clone();
    labonair_settings::watch_dir(cx, themes_dir(), move |cx| reload(&watched, cx));

    let center = notification_center(cx);
    cx.subscribe(&center, |_, event: &NotificationActionEvent, cx| {
        if event.action_id == OPEN_THEMES_FOLDER {
            let dir = themes_dir();
            if let Err(error) = std::fs::create_dir_all(&dir) {
                tracing::warn!("could not create themes folder {dir:?}: {error}");
                return;
            }
            cx.reveal_path(&dir);
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_theme::registry::ThemeRejection;

    #[test]
    fn a_rejected_file_becomes_one_actionable_notification() {
        let rejected = RejectedTheme {
            path: PathBuf::from("/themes/solo.json"),
            reason: ThemeRejection::Malformed("expected value".to_owned()),
        };
        let notification = rejection_notification(&rejected);
        assert!(notification.title.contains("solo.json"));
        assert_eq!(notification.actions.len(), 1);
        assert_eq!(notification.actions[0].id, OPEN_THEMES_FOLDER);
        assert_eq!(notification.actions[0].label, "Open themes folder");
    }
}
