//! Minimal window position/size persistence (T04-003).
//!
//! The original app uses Tauri's `window-state` plugin to remember the main
//! window's geometry across restarts. GPUI has no equivalent, so we persist a
//! tiny JSON blob (`<data_dir>/labonair/window.json`) ourselves: the windowed
//! bounds are restored on launch and re-saved whenever they change.
//!
//! This is deliberately small — full session persistence (open tabs, layout,
//! panel state) lands in T14-001, which builds on top of this.

use std::path::PathBuf;

use gpui::{point, px, size, Bounds, Pixels};
use serde::{Deserialize, Serialize};

/// Anything smaller than this is treated as corrupt / unusable and ignored.
const MIN_SIZE: f32 = 200.0;

fn state_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("labonair")
        .join("window.json")
}

#[derive(Serialize, Deserialize)]
struct StoredBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl StoredBounds {
    fn from_bounds(b: Bounds<Pixels>) -> Self {
        Self {
            x: f32::from(b.origin.x),
            y: f32::from(b.origin.y),
            width: f32::from(b.size.width),
            height: f32::from(b.size.height),
        }
    }

    fn to_bounds(&self) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(self.x), px(self.y)),
            size: size(px(self.width), px(self.height)),
        }
    }
}

/// The last persisted windowed bounds, if any were saved and look sane.
pub fn load() -> Option<Bounds<Pixels>> {
    let raw = std::fs::read_to_string(state_path()).ok()?;
    let stored: StoredBounds = serde_json::from_str(&raw).ok()?;
    if stored.width < MIN_SIZE || stored.height < MIN_SIZE {
        return None;
    }
    Some(stored.to_bounds())
}

/// Persist the given windowed bounds (best-effort; errors are logged, not
/// propagated — losing window geometry must never break the app).
pub fn save(bounds: Bounds<Pixels>) {
    let path = state_path();
    if let Some(dir) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(dir) {
            tracing::warn!(%err, "failed to create window-state dir");
            return;
        }
    }
    let stored = StoredBounds::from_bounds(bounds);
    match serde_json::to_string(&stored) {
        Ok(raw) => {
            if let Err(err) = std::fs::write(&path, raw) {
                tracing::warn!(%err, "failed to write window state");
            }
        }
        Err(err) => tracing::warn!(%err, "failed to serialize window state"),
    }
}
