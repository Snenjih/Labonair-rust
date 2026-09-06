use notify::RecommendedWatcher;
use notify_debouncer_mini::{new_debouncer, Debouncer};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Callback invoked after a debounced change in a watched directory.
///
/// The filesystem service knows nothing about the application event bus or
/// GPUI. The composition layer decides how to translate this signal.
pub type DirectoryChangeHandler = Arc<dyn Fn(DirectoryChanged) + Send + Sync>;

/// Typed domain signal emitted by the filesystem watcher.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryChanged {
    pub path: PathBuf,
}

pub struct WatcherState(pub Arc<Mutex<HashMap<String, Debouncer<RecommendedWatcher>>>>);

impl Default for WatcherState {
    fn default() -> Self {
        WatcherState(Arc::new(Mutex::new(HashMap::new())))
    }
}

fn normalize_path(path: &str) -> String {
    super::paths::expand_home(path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn create_watcher(
    path: &str,
    on_change: DirectoryChangeHandler,
) -> Result<Debouncer<RecommendedWatcher>, String> {
    let emit_path = path.to_string();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            if res.is_ok() {
                on_change(DirectoryChanged {
                    path: PathBuf::from(&emit_path),
                });
            }
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(Path::new(path), notify::RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    Ok(debouncer)
}

/// Starts a non-recursive watcher for `path`. No-op if already watched.
pub async fn fs_watch_dir(
    path: String,
    state: &WatcherState,
    on_change: DirectoryChangeHandler,
) -> Result<(), String> {
    let key = normalize_path(&path);

    {
        let map = state.0.lock().map_err(|e| e.to_string())?;
        if map.contains_key(&key) {
            return Ok(());
        }
    }

    if !Path::new(&key).is_dir() {
        return Ok(());
    }

    let debouncer = create_watcher(&key, on_change)?;
    state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .insert(key, debouncer);

    Ok(())
}

/// Stops the watcher for `path`. No-op if not currently watched.
pub async fn fs_unwatch_dir(path: String, state: &WatcherState) -> Result<(), String> {
    let key = normalize_path(&path);
    state.0.lock().map_err(|e| e.to_string())?.remove(&key);
    Ok(())
}

/// Synchronises the active watcher set to exactly `paths`.
/// Removes stale watchers and starts new ones. Call with an empty
/// vec to stop all watchers (e.g. on explorer unmount).
pub async fn fs_sync_watchers(
    paths: Vec<String>,
    state: &WatcherState,
    on_change: DirectoryChangeHandler,
) -> Result<(), String> {
    let normalized: Vec<String> = paths.iter().map(|p| normalize_path(p)).collect();
    let target_set: HashSet<&str> = normalized.iter().map(|s| s.as_str()).collect();

    // Collect paths not yet watched — outside the lock to avoid holding it during watcher creation.
    let to_add: Vec<String> = {
        let map = state.0.lock().map_err(|e| e.to_string())?;
        normalized
            .iter()
            .filter(|p| !map.contains_key(p.as_str()))
            .cloned()
            .collect()
    };

    // Create new watchers (potentially slow I/O) without holding the lock.
    let mut new_watchers: Vec<(String, Debouncer<RecommendedWatcher>)> = Vec::new();
    for path in to_add {
        if Path::new(&path).is_dir() {
            if let Ok(d) = create_watcher(&path, on_change.clone()) {
                new_watchers.push((path, d));
            }
        }
    }

    // One final lock: prune stale entries, insert new ones.
    let mut map = state.0.lock().map_err(|e| e.to_string())?;
    map.retain(|k, _| target_set.contains(k.as_str()));
    for (path, debouncer) in new_watchers {
        map.entry(path).or_insert(debouncer);
    }

    Ok(())
}
