//! Compatibility adapter for the legacy application event transport.
//!
//! The watcher implementation belongs to `labonair-filesystem`. This module
//! only translates its narrow directory-change callback into `AppEvent` until
//! the application event bus is replaced by typed feature contracts.

use std::sync::Arc;

use crate::events::AppEvent;

pub use labonair_filesystem::watcher::WatcherState;

fn event_handler(app: crate::App) -> labonair_filesystem::watcher::DirectoryChangeHandler {
    Arc::new(move |path| {
        let _ = app.emit_event(AppEvent::DirChanged {
            path: path.path.to_string_lossy().into_owned(),
        });
    })
}

pub async fn fs_watch_dir(
    path: String,
    state: &WatcherState,
    app: crate::App,
) -> Result<(), String> {
    labonair_filesystem::watcher::fs_watch_dir(path, state, event_handler(app)).await
}

pub async fn fs_unwatch_dir(path: String, state: &WatcherState) -> Result<(), String> {
    labonair_filesystem::watcher::fs_unwatch_dir(path, state).await
}

pub async fn fs_sync_watchers(
    paths: Vec<String>,
    state: &WatcherState,
    app: crate::App,
) -> Result<(), String> {
    labonair_filesystem::watcher::fs_sync_watchers(paths, state, event_handler(app)).await
}
