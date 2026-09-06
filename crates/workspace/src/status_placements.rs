//! The `statusBarItemPlacements` blob — T18-005.
//!
//! Replaces the old, titlebar-and-statusbar-spanning `barItemPlacements` /
//! `BarItemId` / `BarLoc` mechanism (a fixed 15-variant enum from before the
//! T17 layout rework). That system had no live consumer: the titlebar
//! (T18-001) and the statusbar (T17-003+) both render from their own
//! registries now, keyed by an arbitrary `&'static str` id rather than a
//! closed enum, and the titlebar has no "moveable item" concept at all. This
//! module is its statusbar-only replacement: it converts between the
//! persisted JSON blob (`{ itemId: { side, hidden } }`) and
//! [`labonair_panel::StatusPlacement`], for whatever ids the running
//! [`labonair_panel::StatusItemRegistry`] happens to have registered.
//!
//! Persistence itself (the atomic read-merge-write + lock) lives in
//! `labonair_backend::modules::settings` (`status_bar_item_placements_load` /
//! `settings_set_status_bar_placement`, mirroring the old bar-item
//! functions); this module is the pure JSON <-> struct half.

use std::collections::HashMap;

use gpui::Global;
use serde_json::{json, Map, Value};

use labonair_panel::{StatusPlacement, StatusSide};

/// Bumped whenever any window persists a status-bar placement change, so
/// every other window's [`crate::status_bar::StatusBar`] re-reads the blob
/// from disk (`cx.observe_global`) and stays in sync (T18-005 point 8, "two
/// windows").
#[derive(Default)]
pub struct StatusBarLayoutTick(pub u64);

impl Global for StatusBarLayoutTick {}

fn side_as_str(side: StatusSide) -> &'static str {
    match side {
        StatusSide::Left => "left",
        StatusSide::Right => "right",
    }
}

fn side_from_str(s: &str) -> Option<StatusSide> {
    match s {
        "left" => Some(StatusSide::Left),
        "right" => Some(StatusSide::Right),
        _ => None,
    }
}

/// Parse the persisted `statusBarItemPlacements` blob into an override table
/// keyed by item id. Entries for ids nobody registered are kept (harmless —
/// [`labonair_panel::StatusItemRegistry::resolve_side`] simply never looks
/// them up) so a placement set while a plugin/panel was temporarily absent
/// isn't lost.
pub fn overrides_from_blob(blob: &Map<String, Value>) -> HashMap<String, StatusPlacement> {
    let mut out = HashMap::new();
    for (id, raw) in blob {
        let side = raw
            .get("side")
            .and_then(Value::as_str)
            .and_then(side_from_str)
            .unwrap_or(StatusSide::Right);
        let hidden = raw.get("hidden").and_then(Value::as_bool).unwrap_or(false);
        out.insert(id.clone(), StatusPlacement { side, hidden });
    }
    out
}

/// JSON patch for one `settings_set_status_bar_placement` call — only the
/// keys that changed, so an unrelated concurrent write to the same item's
/// other field is never clobbered (the backend does a read-merge-write).
pub fn placement_patch(side: Option<StatusSide>, hidden: Option<bool>) -> Value {
    let mut patch = json!({});
    let obj = patch.as_object_mut().unwrap();
    if let Some(s) = side {
        obj.insert(
            "side".to_string(),
            Value::String(side_as_str(s).to_string()),
        );
    }
    if let Some(h) = hidden {
        obj.insert("hidden".to_string(), Value::Bool(h));
    }
    patch
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn overrides_from_blob_parses_known_keys() {
        let mut blob = Map::new();
        blob.insert("cwd".to_string(), json!({ "side": "left", "hidden": true }));
        blob.insert("jump-hosts".to_string(), json!({ "side": "right" }));
        let overrides = overrides_from_blob(&blob);

        assert_eq!(
            overrides.get("cwd"),
            Some(&StatusPlacement {
                side: StatusSide::Left,
                hidden: true,
            })
        );
        // Missing `hidden` defaults to visible.
        assert_eq!(
            overrides.get("jump-hosts"),
            Some(&StatusPlacement {
                side: StatusSide::Right,
                hidden: false,
            })
        );
        assert!(!overrides.contains_key("missing"));
    }

    #[test]
    fn overrides_from_blob_ignores_garbage_side() {
        let mut blob = Map::new();
        blob.insert("weird".to_string(), json!({ "side": "up" }));
        let overrides = overrides_from_blob(&blob);
        // Unparseable side falls back to the right cluster rather than
        // panicking or dropping the entry.
        assert_eq!(
            overrides.get("weird"),
            Some(&StatusPlacement {
                side: StatusSide::Right,
                hidden: false,
            })
        );
    }

    #[test]
    fn placement_patch_only_includes_set_fields() {
        let p = placement_patch(Some(StatusSide::Left), None);
        assert_eq!(p, json!({ "side": "left" }));
        let p = placement_patch(None, Some(true));
        assert_eq!(p, json!({ "hidden": true }));
        let p = placement_patch(Some(StatusSide::Right), Some(false));
        assert_eq!(p, json!({ "side": "right", "hidden": false }));
    }
}
