//! One-time, idempotent migration of the legacy `barItemPlacements` blob
//! (titlebar-and-statusbar-spanning, T17 and earlier) into the T18-005
//! `statusBarItemPlacements` schema (statusbar-only, `{ side, hidden }`).
//!
//! Old shape: `{ itemId: { itemId, bar: "titlebar"|"statusbar", side, hidden } }`
//! (old ids are camelCase, e.g. `agentAccess`, matching the pre-T18
//! `BarItemId` enum's `serde(rename_all = "camelCase")`).
//! New shape: `{ itemId: { side: "left"|"right", hidden } }` (new ids are
//! kebab-case string keys straight out of `StatusItem::id()` in
//! `crates/shell/src/status_items.rs`).
//!
//! Old-id -> new-id mapping (verified against the live `StatusItemRegistry`
//! registrations in `register_builtin_status_items`, not just against names
//! implied by the task doc — several renamed along the way, e.g.
//! `cwdBreadcrumb` -> `cwd`):
//! * `agentAccess` -> `agent-access`
//! * `jumpHosts` -> `jump-hosts`
//! * `cwdBreadcrumb` -> `cwd`
//! * `previewUrl` -> `preview-url`
//! * `cursorPosition` -> `cursor-position`
//! * `notifications`, `transfers`, `updater` -> unchanged
//! * `ai`, `aiMini`, `aiPanel` -> dropped (AI is a panel toggle now, not
//!   placeable).
//! * `explorerPanel`, `snippetsPanel`, `sourceControlPanel`, `tabsPanel`
//!   (panel-toggle / sidebar-panel ids) -> dropped (panel toggles are
//!   fixed-left, not placeable; Tabs became a sidebar panel, not a status
//!   item).
//! * any other/unrecognised id -> dropped.
//!
//! `bar` no longer matters for the transform: every surviving item lands in
//! the statusbar (titlebar items just move there), so `side`/`hidden` are
//! carried over unconditionally. An entry with no `side` is dropped entirely
//! so the item falls back to its compiled-in `default_side`.

use serde_json::{Map, Value};
use std::path::Path;

use super::{
    read_settings_from, write_settings_to, CONFIG_FILE, KEY_BAR_ITEM_PLACEMENTS,
    KEY_STATUS_BAR_ITEM_PLACEMENTS,
};

const KEY_BAR_ITEM_PLACEMENTS_LEGACY: &str = "barItemPlacements_legacy";

/// Old (camelCase) -> new (kebab-case, `StatusItem::id()`) item-id mapping.
/// Anything not in this table (obsolete AI ids, panel-toggle/sidebar-panel
/// ids, or unrecognised ids) is dropped.
const ID_MAP: &[(&str, &str)] = &[
    ("agentAccess", "agent-access"),
    ("jumpHosts", "jump-hosts"),
    ("cwdBreadcrumb", "cwd"),
    ("previewUrl", "preview-url"),
    ("cursorPosition", "cursor-position"),
    ("notifications", "notifications"),
    ("transfers", "transfers"),
    ("updater", "updater"),
];

/// Result of [`migrate_bar_item_placements`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// `statusBarItemPlacements` already existed; nothing was touched.
    AlreadyMigrated,
    /// No `barItemPlacements` blob was present; nothing to migrate.
    NothingToMigrate,
    /// Transformed `migrated` entries into `statusBarItemPlacements`;
    /// `discarded` obsolete/unknown/side-less entries were dropped.
    Migrated { migrated: usize, discarded: usize },
}

/// Migrates the legacy `barItemPlacements` blob into `statusBarItemPlacements`
/// in place, idempotently. Safe to call unconditionally on every startup.
pub fn migrate_bar_item_placements(dir: &Path) -> Result<MigrationOutcome, String> {
    let mut settings = read_settings_from(dir);

    if settings.contains_key(KEY_STATUS_BAR_ITEM_PLACEMENTS) {
        return Ok(MigrationOutcome::AlreadyMigrated);
    }

    let Some(legacy) = settings
        .get(KEY_BAR_ITEM_PLACEMENTS)
        .and_then(|v| v.as_object().cloned())
    else {
        return Ok(MigrationOutcome::NothingToMigrate);
    };

    let path = dir.join(CONFIG_FILE);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }

    let mut migrated_map = Map::new();
    let mut migrated = 0usize;
    let mut discarded = 0usize;

    for (old_id, entry) in &legacy {
        let Some(new_id) = ID_MAP
            .iter()
            .find(|(old, _)| old == old_id)
            .map(|(_, new)| *new)
        else {
            log::debug!("discarding obsolete/unknown bar item placement: {old_id}");
            discarded += 1;
            continue;
        };
        let side = entry
            .as_object()
            .and_then(|o| o.get("side"))
            .and_then(|v| v.as_str())
            .filter(|s| *s == "left" || *s == "right");
        let Some(side) = side else {
            discarded += 1;
            continue;
        };
        let hidden = entry
            .as_object()
            .and_then(|o| o.get("hidden"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut new_entry = Map::new();
        new_entry.insert("side".to_string(), Value::String(side.to_string()));
        new_entry.insert("hidden".to_string(), Value::Bool(hidden));
        migrated_map.insert(new_id.to_string(), Value::Object(new_entry));
        migrated += 1;
    }

    settings.insert(
        KEY_STATUS_BAR_ITEM_PLACEMENTS.to_string(),
        Value::Object(migrated_map),
    );
    let old = settings
        .remove(KEY_BAR_ITEM_PLACEMENTS)
        .unwrap_or(Value::Null);
    settings.insert(KEY_BAR_ITEM_PLACEMENTS_LEGACY.to_string(), old);

    write_settings_to(dir, &settings)?;

    log::info!(
        "migrated barItemPlacements -> statusBarItemPlacements: {migrated} migrated, {discarded} discarded"
    );

    Ok(MigrationOutcome::Migrated {
        migrated,
        discarded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "labonair-bar-migration-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn transforms_titlebar_and_statusbar_entries_and_remaps_ids() {
        let dir = tmp("transform");
        let mut settings = Map::new();
        settings.insert(
            KEY_BAR_ITEM_PLACEMENTS.to_string(),
            json!({
                "cwdBreadcrumb": { "itemId": "cwdBreadcrumb", "bar": "titlebar", "side": "right", "hidden": false },
                "jumpHosts": { "itemId": "jumpHosts", "bar": "statusbar", "side": "left", "hidden": true },
                "agentAccess": { "itemId": "agentAccess", "bar": "titlebar", "side": "right" },
                "updater": { "itemId": "updater", "bar": "statusbar" },
            }),
        );
        write_settings_to(&dir, &settings).unwrap();

        let outcome = migrate_bar_item_placements(&dir).unwrap();
        assert_eq!(
            outcome,
            MigrationOutcome::Migrated {
                migrated: 3,
                discarded: 1
            }
        );

        let new_settings = read_settings_from(&dir);
        let new_blob = new_settings
            .get(KEY_STATUS_BAR_ITEM_PLACEMENTS)
            .unwrap()
            .as_object()
            .unwrap();
        // Old id remapped to the new `StatusItem::id()` string.
        assert_eq!(
            new_blob.get("cwd").unwrap(),
            &json!({ "side": "right", "hidden": false })
        );
        assert_eq!(
            new_blob.get("jump-hosts").unwrap(),
            &json!({ "side": "left", "hidden": true })
        );
        assert_eq!(
            new_blob.get("agent-access").unwrap(),
            &json!({ "side": "right", "hidden": false })
        );
        // `updater` had no `side` -> dropped entirely.
        assert!(!new_blob.contains_key("updater"));

        // Legacy blob preserved under `_legacy`, original key gone.
        assert!(!new_settings.contains_key(KEY_BAR_ITEM_PLACEMENTS));
        assert!(new_settings.contains_key("barItemPlacements_legacy"));

        assert!(dir.join("config.json.bak").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn drops_obsolete_and_unknown_item_ids() {
        let dir = tmp("obsolete");
        let mut settings = Map::new();
        settings.insert(
            KEY_BAR_ITEM_PLACEMENTS.to_string(),
            json!({
                "ai": { "bar": "statusbar", "side": "right" },
                "aiMini": { "bar": "statusbar", "side": "left" },
                "aiPanel": { "bar": "statusbar", "side": "left" },
                "explorerPanel": { "bar": "titlebar", "side": "left" },
                "snippetsPanel": { "bar": "statusbar", "side": "left" },
                "sourceControlPanel": { "bar": "statusbar", "side": "left" },
                "tabsPanel": { "bar": "statusbar", "side": "right" },
                "some-future-id": { "bar": "statusbar", "side": "left" },
                "updater": { "bar": "statusbar", "side": "right", "hidden": true },
            }),
        );
        write_settings_to(&dir, &settings).unwrap();

        let outcome = migrate_bar_item_placements(&dir).unwrap();
        assert_eq!(
            outcome,
            MigrationOutcome::Migrated {
                migrated: 1,
                discarded: 8
            }
        );

        let new_settings = read_settings_from(&dir);
        let new_blob = new_settings
            .get(KEY_STATUS_BAR_ITEM_PLACEMENTS)
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(new_blob.len(), 1);
        assert!(new_blob.contains_key("updater"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_call_is_a_no_op() {
        let dir = tmp("idempotent");
        let mut settings = Map::new();
        settings.insert(
            KEY_BAR_ITEM_PLACEMENTS.to_string(),
            json!({ "jumpHosts": { "bar": "statusbar", "side": "left", "hidden": false } }),
        );
        write_settings_to(&dir, &settings).unwrap();

        migrate_bar_item_placements(&dir).unwrap();
        let after_first = read_settings_from(&dir);

        let outcome = migrate_bar_item_placements(&dir).unwrap();
        assert_eq!(outcome, MigrationOutcome::AlreadyMigrated);

        let after_second = read_settings_from(&dir);
        assert_eq!(after_first, after_second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_case_touches_nothing() {
        let dir = tmp("empty");

        let outcome = migrate_bar_item_placements(&dir).unwrap();
        assert_eq!(outcome, MigrationOutcome::NothingToMigrate);
        assert!(!dir.join(CONFIG_FILE).exists());
        assert!(!dir.join("config.json.bak").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
