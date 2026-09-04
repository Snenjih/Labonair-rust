//! [`SettingsStore`] — layered [`SettingsContent`] merge (T19-002).
//!
//! Blueprint: `zed-refrence/zed/crates/settings/src/settings_store.rs`,
//! trimmed to what this task needs: a fixed-order layer merge, per-type
//! computed-value cache (`register_setting::<T>()` / `get::<T>()`), and a
//! "dumb" (whole-layer) persist path — the surgical single-key JSON write
//! lands in T19-005.

use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use gpui::{App, Global};
use serde_json::Value;

use labonair_settings_content::{FieldError, MergeFrom, SettingsContent};

use crate::settings_trait::Settings;

/// Placeholder identity for a worktree/project root. `T19-003` (project/
/// folder settings) is the first real producer of `SettingsLayer::Project`
/// entries; until then the variant exists so the enum's shape (and merge
/// order) is fixed now, per the task's normative "merge order is defined in
/// `docs/architecture.md`, do not reorder" warning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorktreeId(pub u64);

/// One settings layer. **Declaration order is the merge order** (lowest
/// precedence first) — `#[derive(Ord)]` on an enum compares by variant
/// declaration index first, so sorting a `BTreeMap<SettingsLayer, _>`'s keys
/// yields exactly the fixed order the task requires: `Default < User < Os <
/// Profile < Project(*) < Language(*)`. Do not reorder these variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SettingsLayer {
    /// `SettingsContent::defaults()` (`assets/settings/default.json`).
    Default,
    /// `~/.config/labonair/labonair-settings.json` (the shared settings
    /// file's top-level `SettingsContent` keys — see `docs/architecture.md`
    /// §8.3 / `content_bridge.rs` for the still-open `editor`/`mcp` key
    /// collision with the legacy `Preferences` bridge, resolved in T19-009).
    User,
    /// Platform-specific overrides (`platform_overrides`-shaped, Zed-style).
    /// No `SettingsContent` field carries this data yet — always empty until
    /// a follow-up task adds the wrapper type; the variant/ordering exists
    /// now so it never needs to be inserted out of order later.
    Os,
    /// Named settings profile overrides. Same status as `Os` — structurally
    /// present, no loader yet.
    Profile,
    /// Per-worktree `.labonair/settings.json` (T19-003). Empty until then.
    Project(WorktreeId),
    /// Per-language editor overrides. Labonair has no LSP-per-language
    /// editor surface yet; placeholder for parity with the Zed model.
    Language(String),
}

/// One computed-value builder, monomorphized per registered `Settings` type.
type Builder = Box<dyn Fn(&SettingsContent, &mut HashMap<TypeId, Box<dyn Any>>)>;

/// The layered settings store. Lives as a GPUI [`Global`]
/// (`cx.global::<SettingsStore>()`); mutation goes through `cx.global_mut`,
/// which GPUI's `Effect::NotifyGlobalObservers` turns into an automatic
/// `cx.observe_global::<SettingsStore>` notification — no bespoke observer
/// list needed here.
pub struct SettingsStore {
    /// Every layer's raw (sparse — every leaf still `Option`) content, keyed
    /// by [`SettingsLayer`]. Iterated in key order (see the `Ord` note above)
    /// by [`Self::recompute`].
    raw: BTreeMap<SettingsLayer, SettingsContent>,
    /// The merged, effective tree — `Default` folded through every other
    /// layer present in `raw`, in order.
    merged: SettingsContent,
    /// Where the `User` layer is read from / persisted to.
    user_path: PathBuf,
    /// Non-fatal per-area parse errors from the last `User` layer (re)load
    /// (`labonair_settings_content::fallible::parse`'s granularity — one
    /// broken area falls back to its default, the rest of the tree is
    /// unaffected).
    parse_errors: Vec<FieldError>,
    /// Per-`Settings`-type computed value, rebuilt by `builders` on every
    /// `recompute`.
    values: HashMap<TypeId, Box<dyn Any>>,
    registered: HashSet<TypeId>,
    builders: Vec<Builder>,
}

impl Global for SettingsStore {}

impl SettingsStore {
    fn new(user_path: PathBuf) -> Self {
        let mut raw = BTreeMap::new();
        raw.insert(SettingsLayer::Default, SettingsContent::defaults());
        raw.insert(SettingsLayer::User, SettingsContent::default());
        let mut store = Self {
            raw,
            merged: SettingsContent::default(),
            user_path,
            parse_errors: Vec::new(),
            values: HashMap::new(),
            registered: HashSet::new(),
            builders: Vec::new(),
        };
        store.recompute();
        store
    }

    /// `merged = defaults(); for layer in order { merged.merge_from(&layer) }`
    /// (Anweisung #3), then every registered `Settings` type's computed value
    /// is rebuilt from the new `merged`. GPUI observer notification is the
    /// caller's responsibility implicitly — every mutator here is only ever
    /// reached through `cx.global_mut::<SettingsStore>()`, which already
    /// queues the `NotifyGlobalObservers` effect on access.
    fn recompute(&mut self) {
        let mut merged = SettingsContent::default();
        for content in self.raw.values() {
            merged.merge_from(content);
        }
        self.merged = merged;
        for builder in &self.builders {
            builder(&self.merged, &mut self.values);
        }
    }

    pub fn merged(&self) -> &SettingsContent {
        &self.merged
    }

    pub fn user_path(&self) -> &Path {
        &self.user_path
    }

    pub fn parse_errors(&self) -> &[FieldError] {
        &self.parse_errors
    }

    /// Replace one layer's content and recompute. `Default` may not be
    /// replaced this way (use is a logic error — the default tree is fixed
    /// at construction from `SettingsContent::defaults()`).
    pub fn set_layer(&mut self, layer: SettingsLayer, content: SettingsContent) {
        debug_assert!(
            !matches!(layer, SettingsLayer::Default),
            "SettingsLayer::Default is fixed, not a settable layer"
        );
        self.raw.insert(layer, content);
        self.recompute();
    }

    pub fn layer(&self, layer: &SettingsLayer) -> Option<&SettingsContent> {
        self.raw.get(layer)
    }

    /// (Re)read the `User` layer from [`Self::user_path`]. A file that isn't
    /// valid JSON/JSONC at all keeps the last-good `User` layer (never
    /// crashes, never silently reverts to all-defaults); a file that parses
    /// but has one broken area falls back to that area's default while every
    /// other area still applies (`labonair_settings_content::parse`'s
    /// existing per-area granularity).
    pub fn reload_user_layer(&mut self) {
        let Ok(raw) = std::fs::read_to_string(&self.user_path) else {
            // No file yet (first run) — an absent file is not a corrupt one;
            // treat it as an empty User layer.
            self.raw
                .insert(SettingsLayer::User, SettingsContent::default());
            self.recompute();
            return;
        };

        if jsonc_parser::parse_to_serde_value(&raw, &Default::default()).is_err() {
            tracing::warn!(
                path = %self.user_path.display(),
                "labonair-settings.json is not valid JSON/JSONC — keeping the last good settings",
            );
            return;
        }

        let (content, errors) = labonair_settings_content::parse(&raw);
        if !errors.is_empty() {
            for e in &errors {
                tracing::warn!(area = e.area, message = %e.message, "settings area failed to parse, using its default");
            }
        }
        self.parse_errors = errors;
        self.raw.insert(SettingsLayer::User, content);
        self.recompute();
    }

    /// Replace the `User` layer and persist it ("dumb" — the whole layer is
    /// re-serialized; the surgical single-key write lands in T19-005). Other
    /// top-level keys already in the file (`preferences`, `dockLayout`, the
    /// legacy `editor`/`mcp` blobs, …) are preserved untouched.
    pub fn set_user_content(&mut self, content: SettingsContent) -> Result<(), String> {
        self.raw.insert(SettingsLayer::User, content);
        self.recompute();
        self.persist_user_layer()
    }

    /// Convenience wrapper: mutate a clone of the current `User` layer, then
    /// commit + persist it in one step.
    pub fn update_user(&mut self, f: impl FnOnce(&mut SettingsContent)) -> Result<(), String> {
        let mut content = self
            .raw
            .get(&SettingsLayer::User)
            .cloned()
            .unwrap_or_default();
        f(&mut content);
        self.set_user_content(content)
    }

    fn persist_user_layer(&self) -> Result<(), String> {
        let content = self
            .raw
            .get(&SettingsLayer::User)
            .cloned()
            .unwrap_or_default();

        let mut map = std::fs::read_to_string(&self.user_path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        let content_value = serde_json::to_value(&content).map_err(|e| e.to_string())?;
        if let Value::Object(content_map) = content_value {
            for (k, v) in content_map {
                map.insert(k, v);
            }
        }

        let dir = self
            .user_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        // `.bak` safety net before the atomic rename, same convention as
        // `labonair_backend::modules::settings::preferences`.
        if self.user_path.exists() {
            let _ = std::fs::copy(&self.user_path, self.user_path.with_extension("json.bak"));
        }

        let tmp = self.user_path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string())?;
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.user_path).map_err(|e| e.to_string())
    }

    /// Register a `Settings` type: compute its initial value from the
    /// current `merged` tree and remember how to recompute it on every
    /// future `recompute()`. Idempotent — re-registering the same type is a
    /// no-op (mirrors Zed's `register_setting_internal` dedup).
    pub fn register_setting<T: Settings>(&mut self) {
        let id = TypeId::of::<T>();
        if !self.registered.insert(id) {
            return;
        }
        self.values
            .insert(id, Box::new(T::from_settings(&self.merged)));
        self.builders.push(Box::new(|merged, values| {
            values.insert(TypeId::of::<T>(), Box::new(T::from_settings(merged)));
        }));
    }

    pub fn is_registered<T: Settings>(&self) -> bool {
        self.registered.contains(&TypeId::of::<T>())
    }

    /// The current computed value for `T`. Panics if `T` was never
    /// registered (`Settings::register` / `#[derive(RegisterSetting)]` +
    /// `labonair_settings::init` not having run for it) — same contract as
    /// Zed's `Settings::get`.
    #[track_caller]
    pub fn get<T: Settings>(&self) -> &T {
        self.values
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
            .unwrap_or_else(|| {
                panic!(
                    "labonair-settings: {} was never registered — call its `register(cx)` (or add \
                     `#[derive(RegisterSetting)]` and run `labonair_settings::register_all(cx)`) \
                     before reading it",
                    std::any::type_name::<T>()
                )
            })
    }
}

fn default_user_path() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    let base = dirs::home_dir()
        .expect("cannot resolve home dir")
        .join(".config");
    #[cfg(target_os = "windows")]
    let base = dirs::config_dir().expect("cannot resolve config dir");

    let dir = base.join("labonair");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("labonair-settings.json")
}

/// Build the store, load the `User` layer once, and publish it as the
/// [`SettingsStore`] global. Registration of concrete `Settings` types and
/// the fs-watch task are the caller's job (`labonair_settings::init`) —
/// split out so tests can construct a store against an explicit path without
/// touching the real `~/.config/labonair` directory.
pub(crate) fn init(cx: &mut App) {
    let mut store = SettingsStore::new(default_user_path());
    store.reload_user_layer();
    cx.set_global(store);
}

#[cfg(test)]
pub(crate) fn init_at(cx: &mut App, user_path: PathBuf) {
    let mut store = SettingsStore::new(user_path);
    store.reload_user_layer();
    cx.set_global(store);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_path() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("labonair-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("labonair-settings.json")
    }

    #[test]
    fn layer_ord_matches_the_normative_merge_order() {
        let mut layers = vec![
            SettingsLayer::Language("rust".into()),
            SettingsLayer::Project(WorktreeId(1)),
            SettingsLayer::Profile,
            SettingsLayer::Os,
            SettingsLayer::User,
            SettingsLayer::Default,
        ];
        layers.sort();
        assert_eq!(
            layers,
            vec![
                SettingsLayer::Default,
                SettingsLayer::User,
                SettingsLayer::Os,
                SettingsLayer::Profile,
                SettingsLayer::Project(WorktreeId(1)),
                SettingsLayer::Language("rust".into()),
            ]
        );
    }

    #[test]
    fn recompute_merges_default_user_project_in_order() {
        let mut store = SettingsStore::new(tmp_path());
        assert_eq!(store.merged().terminal.terminal_font_size, Some(14));

        let mut user = SettingsContent::default();
        user.terminal.terminal_font_size = Some(18);
        store.set_layer(SettingsLayer::User, user);
        assert_eq!(store.merged().terminal.terminal_font_size, Some(18));

        let mut project = SettingsContent::default();
        project.terminal.terminal_font_size = Some(22);
        store.set_layer(SettingsLayer::Project(WorktreeId(1)), project);
        assert_eq!(store.merged().terminal.terminal_font_size, Some(22));

        // User still wins over an unrelated leaf the project layer never set.
        assert_eq!(store.merged().terminal.terminal_scrollback, Some(5_000));
    }

    #[test]
    fn reload_user_layer_round_trips_from_disk() {
        let path = tmp_path();
        std::fs::write(&path, r#"{"terminal":{"terminalFontSize":20}}"#).unwrap();
        let mut store = SettingsStore::new(path);
        store.reload_user_layer();
        assert_eq!(store.merged().terminal.terminal_font_size, Some(20));
        assert!(store.parse_errors().is_empty());
    }

    #[test]
    fn reload_user_layer_keeps_last_good_value_on_corrupt_file() {
        let path = tmp_path();
        std::fs::write(&path, r#"{"terminal":{"terminalFontSize":20}}"#).unwrap();
        let mut store = SettingsStore::new(path.clone());
        store.reload_user_layer();
        assert_eq!(store.merged().terminal.terminal_font_size, Some(20));

        std::fs::write(&path, "not json at all {{{").unwrap();
        store.reload_user_layer();
        // Corrupt file → last-good User layer is kept, not wiped.
        assert_eq!(store.merged().terminal.terminal_font_size, Some(20));
    }

    #[test]
    fn reload_user_layer_defaults_only_the_broken_area() {
        let path = tmp_path();
        std::fs::write(
            &path,
            r#"{"terminal":{"terminalFontSize":"nope"},"general":{"startupTerminalCount":2}}"#,
        )
        .unwrap();
        let mut store = SettingsStore::new(path);
        store.reload_user_layer();
        assert_eq!(store.parse_errors().len(), 1);
        assert_eq!(store.parse_errors()[0].area, "terminal");
        assert_eq!(store.merged().terminal.terminal_font_size, Some(14)); // default
        assert_eq!(store.merged().general.startup_terminal_count, Some(2));
    }

    #[test]
    fn set_user_content_persists_and_preserves_unrelated_keys() {
        let path = tmp_path();
        std::fs::write(&path, r#"{"preferences":{"theme":"dark"}}"#).unwrap();
        let mut store = SettingsStore::new(path.clone());
        store.reload_user_layer();

        let mut user = SettingsContent::default();
        user.terminal.terminal_font_size = Some(30);
        store.set_user_content(user).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(value["preferences"]["theme"], "dark");
        assert_eq!(value["terminal"]["terminalFontSize"], 30);
        assert!(path.with_extension("json.bak").exists());
    }

    #[derive(Debug, PartialEq)]
    struct FakeFontSize(u32);

    impl Settings for FakeFontSize {
        fn from_settings(content: &SettingsContent) -> Self {
            Self(content.terminal.terminal_font_size.unwrap_or(0))
        }
    }

    #[test]
    fn register_setting_computes_and_recomputes_on_change() {
        let mut store = SettingsStore::new(tmp_path());
        store.register_setting::<FakeFontSize>();
        assert_eq!(store.get::<FakeFontSize>(), &FakeFontSize(14));

        let mut user = SettingsContent::default();
        user.terminal.terminal_font_size = Some(21);
        store.set_layer(SettingsLayer::User, user);
        assert_eq!(store.get::<FakeFontSize>(), &FakeFontSize(21));
    }

    #[test]
    fn register_setting_is_idempotent() {
        let mut store = SettingsStore::new(tmp_path());
        store.register_setting::<FakeFontSize>();
        store.register_setting::<FakeFontSize>();
        assert_eq!(store.builders.len(), 1);
    }

    #[gpui::test]
    fn recompute_notifies_global_observers(cx: &mut gpui::TestAppContext) {
        let notified = std::rc::Rc::new(std::cell::Cell::new(false));
        cx.update(|cx| {
            init_at(cx, tmp_path());
            let flag = notified.clone();
            cx.observe_global::<SettingsStore>(move |_| flag.set(true))
                .detach();
        });
        // `observe_global` activates its subscription via `cx.defer`, so let
        // that first effect cycle flush before triggering the real mutation.
        cx.background_executor.run_until_parked();
        assert!(!notified.get());

        cx.update(|cx| {
            cx.global_mut::<SettingsStore>().reload_user_layer();
        });
        cx.background_executor.run_until_parked();
        assert!(
            notified.get(),
            "reload_user_layer must notify SettingsStore's global observers"
        );
    }
}
