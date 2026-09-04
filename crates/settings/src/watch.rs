//! Live fs-watch on the `User` settings layer (T19-002 Anweisung #6).
//!
//! Same crates + debounce pattern as
//! `labonair_backend::modules::fs::watcher`: `notify_debouncer_mini` runs its
//! callback on its own background thread — it must never touch GPUI directly
//! (Warnung: "niemals den Store vom Tokio-Thread mutieren"). The callback
//! only flips an `AtomicBool`; a `cx.spawn`ed foreground task polls that flag
//! on a short timer and, when set, brings the reload back onto the main
//! thread via `cx.update`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gpui::App;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};

use crate::store::SettingsStore;

/// How often the foreground poll checks the dirty flag. Matches the task's
/// "~150 ms" debounce note; the debouncer itself also debounces at 150 ms, so
/// a burst of writes (e.g. an editor's atomic-rename save) collapses to one
/// reload either way.
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Start watching `user_path`'s parent directory (not the file itself — a
/// `rename`-based atomic save, which is exactly how this store's own
/// `persist_user_layer` writes, replaces the inode, and a handle on the old
/// inode would silently go stale) and reload the `User` layer whenever it
/// changes. Best-effort: if the watcher can't start (e.g. sandboxed
/// environment with no inotify/FSEvents), settings simply stay
/// load-once — never a hard failure.
pub(crate) fn spawn(cx: &App, user_path: PathBuf) {
    let Some(dir) = user_path.parent().map(|p| p.to_path_buf()) else {
        return;
    };

    let dirty = Arc::new(AtomicBool::new(false));
    let dirty_cb = dirty.clone();
    let watched_path = user_path.clone();

    let debouncer = new_debouncer(
        POLL_INTERVAL,
        move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
            if let Ok(events) = res {
                if events
                    .iter()
                    .any(|e| e.path.file_name() == watched_path.file_name())
                {
                    dirty_cb.store(true, Ordering::SeqCst);
                }
            }
        },
    );

    let Ok(mut debouncer) = debouncer else {
        tracing::warn!("labonair-settings: failed to start the fs-watch debouncer");
        return;
    };
    if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
        tracing::warn!(error = %e, dir = %dir.display(), "labonair-settings: failed to watch settings dir");
        return;
    }

    keep_alive_and_poll(cx, debouncer, dirty);
}

/// Owns the `Debouncer` for as long as the returned `Task` runs (a detached,
/// infinite loop — same idiom `crates/shell/src/bootstrap.rs` uses for its
/// `live_drain` poll), and drains the dirty flag on the foreground thread.
fn keep_alive_and_poll(cx: &App, debouncer: Debouncer<RecommendedWatcher>, dirty: Arc<AtomicBool>) {
    cx.spawn(async move |cx| {
        let _debouncer = debouncer;
        loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            if dirty.swap(false, Ordering::SeqCst) {
                let updated = cx.update(|cx| {
                    cx.global_mut::<SettingsStore>().reload_user_layer();
                });
                if updated.is_err() {
                    // App is shutting down.
                    break;
                }
            }
        }
    })
    .detach();
}

/// Start watching `dir` (a project root's `.labonair` directory — must
/// already exist, see `set_active_project_root`'s doc) for changes to
/// `settings.json` and reload the project layer whenever it changes
/// (T19-003 Anweisung #3). `generation` is the [`SettingsStore::
/// project_watch_generation`] value captured when this watch was started;
/// the poll loop stops itself the moment the store's live generation no
/// longer matches — this is how a root switch "unregisters" the old watch
/// without needing a cancel handle to survive the `cx.spawn` boundary.
/// Best-effort, same as [`spawn`]: a watcher that fails to start just means
/// project settings stay load-once for this root.
pub(crate) fn spawn_project(cx: &App, dir: PathBuf, generation: u64) {
    let dirty = Arc::new(AtomicBool::new(false));
    let dirty_cb = dirty.clone();

    let debouncer = new_debouncer(
        POLL_INTERVAL,
        move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
            if let Ok(events) = res {
                if events
                    .iter()
                    .any(|e| e.path.file_name().and_then(|n| n.to_str()) == Some("settings.json"))
                {
                    dirty_cb.store(true, Ordering::SeqCst);
                }
            }
        },
    );

    let Ok(mut debouncer) = debouncer else {
        tracing::warn!("labonair-settings: failed to start the project fs-watch debouncer");
        return;
    };
    if let Err(e) = debouncer.watcher().watch(&dir, RecursiveMode::NonRecursive) {
        tracing::warn!(error = %e, dir = %dir.display(), "labonair-settings: failed to watch project settings dir");
        return;
    }

    keep_alive_and_poll_project(cx, debouncer, dirty, generation);
}

fn keep_alive_and_poll_project(
    cx: &App,
    debouncer: Debouncer<RecommendedWatcher>,
    dirty: Arc<AtomicBool>,
    generation: u64,
) {
    cx.spawn(async move |cx| {
        let _debouncer = debouncer;
        loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            let still_current = cx
                .update(|cx| cx.global::<SettingsStore>().project_watch_generation() == generation);
            match still_current {
                Ok(true) => {}
                Ok(false) => break, // a newer root/rewatch superseded this task
                Err(_) => break,    // app shutting down
            }
            if dirty.swap(false, Ordering::SeqCst) {
                let updated = cx.update(|cx| {
                    let store = cx.global_mut::<SettingsStore>();
                    if store.project_watch_generation() == generation {
                        store.reload_project_layer();
                    }
                });
                if updated.is_err() {
                    break;
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    //! The GPUI-side poll loop is exercised end-to-end by
    //! `crate::store::tests::recompute_notifies_global_observers` (same
    //! `reload_user_layer` + `cx.observe_global` path this module's poll task
    //! drives). This test instead proves the real notify/debounce half in
    //! isolation — no GPUI test clock involved, since a real OS file-watch
    //! thread racing a deterministic test executor would be flaky by
    //! construction.
    use super::*;

    #[test]
    fn debounced_watcher_flags_dirty_on_real_file_change() {
        let dir =
            std::env::temp_dir().join(format!("labonair-settings-watch-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("labonair-settings.json");
        std::fs::write(&path, "{}").unwrap();

        let dirty = Arc::new(AtomicBool::new(false));
        let dirty_cb = dirty.clone();
        let watched = path.clone();
        let mut debouncer = new_debouncer(
            Duration::from_millis(50),
            move |res: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = res {
                    if events
                        .iter()
                        .any(|e| e.path.file_name() == watched.file_name())
                    {
                        dirty_cb.store(true, Ordering::SeqCst);
                    }
                }
            },
        )
        .unwrap();
        debouncer
            .watcher()
            .watch(&dir, RecursiveMode::NonRecursive)
            .unwrap();

        // Atomic rename — matches how `persist_user_layer` writes.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, r#"{"terminal":{"terminalFontSize":30}}"#).unwrap();
        std::fs::rename(&tmp, &path).unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !dirty.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            dirty.load(Ordering::SeqCst),
            "watcher never observed the rename"
        );
    }
}
