//! T19-009: one-time, idempotent migration of the legacy split
//! `preferences`/`editor`/`mcp` top-level keys in `config.json`
//! into the flat, area-based `SettingsContent` layout (T19-001), the
//! `preferences.keybinds` override blob into `keymap.json` (T19-008's file
//! shape) and the `keymap.json` override layer without losing user data.
//!
//! ## Field-mapping table (`preferences.*` -> `SettingsContent.*`)
//!
//! Every one of the historical `Preferences` struct's ~170 fields is
//! accounted for by [`migrate_settings_v1_to_v2`] one way or another:
//! * the vast majority map 1:1 by (identical) field name into the matching
//!   `SettingsContent` area — see the `general_from`/`appearance_from`/…
//!   builder functions below, one per area, field order mirroring
//!   `Preferences`' own category comments (and `content_bridge.rs`'s reverse
//!   direction, which this migration inverts).
//! * `hmLayout`/`hmSort`/`hmCardScale` remain outside Settings because the
//!   Hosts capability owns its management state.
//! * `dockLayout`/`sidebar*` (position/open/activePanel/rightOpen/
//!   rightActivePanel/width/rightWidth) move into the Workspace-owned
//!   `workspace-layout.json` file before this Settings migration runs.
//! * `mcpBridge*`/`mcpMaxCommandTimeoutSecs`/`mcpAutoRevokeMinutes`/
//!   `mcpNotifyOnActivity` are a documented *mirror* of the separate old
//!   `"mcp"` top-level key (`McpPrefs` — the authoritative source per
//!   `Preferences`' own doc comment); the new `mcp` area is built from the
//!   old `"mcp"` key, not from this mirror, since they carry the same values.
//! * `barItemPlacements` is **not** touched here — it is exclusively
//!   T18-006's job (`migrate_bar_item_placements`, `statusBarItemPlacements`)
//!   and has no `SettingsContent` counterpart (documented in
//!   `content_bridge.rs` too).
//! * `barLayoutMigrated` has no `SettingsContent` counterpart either; when it
//!   is `true` it is preserved losslessly under
//!   `_migratedUnknown.preferences.barLayoutMigrated` rather than silently
//!   dropped (its `false` default needs no entry — absence is unambiguous).
//!
//! The old `"editor"` key (`EditorPrefs` — Vim ex-command settings) maps
//! `hlsearch`/`incsearch`/`smartcase`/`relativeNumber`/`vimMode` onto the new
//! `editor` area's `vimHlsearch`/`vimIncsearch`/`vimSmartcase`/
//! `editorRelativeLineNumbers`/`vimMode` fields (the latter two are also
//! covered by `preferences.editorRelativeLineNumbers`/`editorVimMode` — the
//! `Preferences` value wins on conflict, mirroring `content_bridge.rs`'s
//! "one authoritative flat model" stance). `number`/`expandtab`/`tabstop`/
//! `shiftwidth` have no `SettingsContent` field (Vim `:set` internals never
//! exposed as a setting) and land in `_migratedUnknown.editor.*`.
//!
//! `tests::every_preferences_field_is_accounted_for` proves the table is
//! exhaustive by diffing `serde_json::to_value(Preferences::default())`'s
//! keys against the mapped/unknown/deliberately-skipped lists below — a
//! field silently falling off the table fails the test, per the task's own
//! warning ("Mapping ergänzen, nicht den Test aufweichen").

use serde_json::{Map, Value};
use std::path::Path;

use labonair_settings_content::{
    appearance::AppearanceContent,
    editor::EditorContent,
    file_manager::FileManagerContent,
    general::{self, GeneralContent},
    terminal::{self, TerminalContent},
    workspace::{self, WorkspaceContent},
    SettingsContent,
};

use super::preferences::{CursorStyle, Preferences, StartupTab, ThemePref};
use super::{editor::EditorPrefs, CONFIG_FILE};
use super::{read_settings_from, write_settings_to};

const KEY_PREFERENCES: &str = "preferences";
const KEY_EDITOR: &str = "editor";
const KEY_SCHEMA_VERSION: &str = "schemaVersion";
const KEY_MIGRATED_UNKNOWN: &str = "_migratedUnknown";
/// Marks a `config.json` whose `SettingsContent` area objects have had every
/// leaf that merely restates its built-in default removed — so the file only
/// carries the user's actual overrides (`default.json` stays the full
/// reference). Set both by a fresh v1->v2 migration and by the one-time
/// [`sparsify_v2_settings`] cleanup of files migrated before this existed.
const KEY_SPARSIFIED: &str = "sparsified";
const SCHEMA_VERSION_V2: u64 = 2;

/// `SettingsContent`'s top-level area keys (its `#[serde(rename_all =
/// "camelCase")]` field names) — the only keys [`sparsify_settings_map`]
/// touches. Anything else in `config.json` (`schemaVersion`,
/// `_migratedUnknown`, `statusBarItemPlacements`, …) is left verbatim.
const SETTINGS_CONTENT_AREAS: &[&str] = &[
    "general",
    "appearance",
    "terminal",
    "editor",
    "fileManager",
    "connections",
    "workspace",
];

/// Result of [`migrate_settings_v1_to_v2`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsV2Outcome {
    /// `schemaVersion: 2` (or an unambiguous new-format signal) was already
    /// present; nothing was touched.
    AlreadyMigrated,
    /// No legacy `preferences` blob was present (fresh install); nothing to
    /// migrate.
    NothingToMigrate,
    /// Transformed the legacy blobs into the new area layout.
    Migrated {
        /// Number of `preferences.keybinds` overrides written into
        /// `keymap.json` (0 if there were none / no file was written).
        keybinds_migrated: usize,
    },
}

/// Result of [`sparsify_v2_settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparsifyOutcome {
    /// The file isn't a `schemaVersion: 2` document — nothing to do (a v1
    /// file is handled by [`migrate_settings_v1_to_v2`] instead, which
    /// sparsifies its own output).
    NotV2,
    /// Already carries `sparsified: true` — the cleanup has run before.
    AlreadySparse,
    /// Removed `removed` default-valued / `null` leaves (and any area object
    /// that emptied out entirely) from the file.
    Sparsified { removed: usize },
}

// ─────────────────────────────────────────────────────────────────────────
// Preferences/editor -> SettingsContent areas
// ─────────────────────────────────────────────────────────────────────────

fn theme_pref(v: ThemePref) -> general::ThemePref {
    match v {
        ThemePref::System => general::ThemePref::System,
        ThemePref::Light => general::ThemePref::Light,
        ThemePref::Dark => general::ThemePref::Dark,
    }
}

fn startup_tab(v: StartupTab) -> general::StartupTab {
    match v {
        StartupTab::Terminal => general::StartupTab::Terminal,
        StartupTab::Empty => general::StartupTab::Empty,
    }
}

fn cursor_style(v: CursorStyle) -> terminal::CursorStyle {
    match v {
        CursorStyle::Block => terminal::CursorStyle::Block,
        CursorStyle::Underline => terminal::CursorStyle::Underline,
        CursorStyle::Bar => terminal::CursorStyle::Bar,
    }
}

fn palette_search_mode(v: super::preferences::PaletteSearchMode) -> workspace::PaletteSearchMode {
    use super::preferences::PaletteSearchMode as P;
    match v {
        P::Contains => workspace::PaletteSearchMode::Contains,
        P::StartsWith => workspace::PaletteSearchMode::StartsWith,
        P::Fuzzy => workspace::PaletteSearchMode::Fuzzy,
    }
}

fn general_from(p: &Preferences) -> GeneralContent {
    GeneralContent {
        theme: Some(theme_pref(p.theme)),
        restore_window_state: Some(p.restore_window_state),
        default_startup_tab: Some(startup_tab(p.default_startup_tab)),
        check_for_updates: Some(p.check_for_updates),
        session_restore: Some(p.session_restore),
    }
}

fn appearance_from(p: &Preferences) -> AppearanceContent {
    AppearanceContent {
        app_theme: Some(p.app_theme.clone()),
        icon_theme: Some(p.icon_theme.clone()),
        theme_variant_overrides: Some(p.theme_variant_overrides.clone()),
        app_font_size: Some(p.app_font_size),
        app_line_height: Some(p.app_line_height),
        app_font_family: Some(p.app_font_family.clone()),
        reduce_motion: Some(p.reduce_motion),
        corner_radius_scale: Some(p.app_corner_radius as f32 / 5.0),
        tabs_location: Some(p.tabs_location.clone()),
        zen_mode_show_header: Some(p.zen_mode_show_header),
        zen_mode_show_statusbar: Some(p.zen_mode_show_statusbar),
        // T20-007 `theme_settings` fields have no legacy `Preferences` key —
        // they resolve to their `AppearanceContent::defaults()` on read.
        ..AppearanceContent::default()
    }
}

const BACKGROUND_KEYS: &[&str] = &[
    "backgroundImage",
    "backgroundOpacity",
    "backgroundBlur",
    "backgroundTintColor",
    "backgroundTintOpacity",
];

/// Convert the pre-T20-007 pixel radius to the current scale value. The
/// explicit modern scale wins when both keys are present.
fn move_legacy_corner_radius(settings: &mut Map<String, Value>) -> usize {
    let Some(appearance) = settings
        .get_mut("appearance")
        .and_then(Value::as_object_mut)
    else {
        return 0;
    };
    let Some(legacy) = appearance.remove("appCornerRadius") else {
        return 0;
    };
    if !appearance.contains_key("cornerRadiusScale") {
        if let Some(px) = legacy.as_f64() {
            appearance.insert("cornerRadiusScale".into(), Value::from(px / 5.0));
        }
    }
    1
}

/// Persist background values in the top-level shape consumed by
/// `labonair-background`. Background rendering and image storage already have
/// their own owner; keeping these keys nested under `appearance` would create
/// a second source of truth.
fn insert_background_values(settings: &mut Map<String, Value>, p: &Preferences) {
    settings.insert(
        "backgroundImage".into(),
        Value::String(p.background_image.clone()),
    );
    settings.insert(
        "backgroundOpacity".into(),
        Value::from(p.background_opacity),
    );
    settings.insert("backgroundBlur".into(), Value::from(p.background_blur));
    settings.insert(
        "backgroundTintColor".into(),
        Value::String(p.background_tint_color.clone()),
    );
    settings.insert(
        "backgroundTintOpacity".into(),
        Value::from(p.background_tint_opacity),
    );
}

/// Move background values emitted by earlier v2 versions out of the Settings
/// appearance object. Existing top-level values win, making this safe to run
/// repeatedly after a user has already edited the Background-owned file.
fn move_background_values_to_owner(settings: &mut Map<String, Value>) -> usize {
    let mut moved = Vec::new();
    if let Some(appearance) = settings
        .get_mut("appearance")
        .and_then(Value::as_object_mut)
    {
        for key in BACKGROUND_KEYS {
            if let Some(value) = appearance.remove(*key) {
                moved.push((*key, value));
            }
        }
    }
    let mut changed = 0;
    for (key, value) in moved {
        if !settings.contains_key(key) {
            settings.insert(key.to_string(), value);
        }
        changed += 1;
    }
    changed
}

fn workspace_from(p: &Preferences) -> WorkspaceContent {
    WorkspaceContent {
        command_palette_search_mode: Some(palette_search_mode(p.command_palette_search_mode)),
        command_palette_show_recent: Some(p.command_palette_show_recent),
        command_palette_blur: Some(p.command_palette_blur),
        command_palette_opacity: Some(p.command_palette_opacity),
        command_palette_position: Some(p.command_palette_position.clone()),
        command_palette_animation: Some(p.command_palette_animation.clone()),
        command_palette_history_size: Some(p.command_palette_history_size),
        command_palette_close_on_overlay_click: Some(p.command_palette_close_on_overlay_click),
        git_status_poll_interval_ms: Some(p.git_status_poll_interval_ms),
    }
}

fn terminal_from(p: &Preferences) -> TerminalContent {
    TerminalContent {
        terminal_shell: Some(p.terminal_shell.clone()),
        terminal_default_path: Some(p.terminal_default_path.clone()),
        new_tab_inherits_cwd: Some(p.new_tab_inherits_cwd),
        confirm_close_terminal_tab: Some(p.confirm_close_terminal_tab),
        terminal_font_family: Some(p.terminal_font_family.clone()),
        terminal_font_size: Some(p.terminal_font_size),
        terminal_line_height: Some(p.terminal_line_height),
        terminal_scrollback: Some(p.terminal_scrollback),
        session_scrollback_lines: Some(p.session_scrollback_lines),
        scrollback_max_size_mb: Some(p.scrollback_max_size_mb),
        scrollback_retention_days: Some(p.scrollback_retention_days),
        terminal_cursor_style: Some(cursor_style(p.terminal_cursor_style)),
        terminal_cursor_blink: Some(p.terminal_cursor_blink),
        terminal_copy_on_select: Some(p.terminal_copy_on_select),
        terminal_right_click_pastes: Some(p.terminal_right_click_pastes),
        terminal_show_pane_header: Some(p.terminal_show_pane_header),
        terminal_show_pane_footer: Some(p.terminal_show_pane_footer),
        terminal_bell: Some(p.terminal_bell),
        terminal_opacity: Some(p.terminal_opacity),
    }
}

/// Builds the `editor` area from both `preferences.editor*`/`vimMode` (the
/// authoritative source on conflict) and the old separate `"editor"` key's
/// `hlsearch`/`incsearch`/`smartcase` (which have no `Preferences`
/// counterpart).
fn editor_from(p: &Preferences, e: &EditorPrefs) -> EditorContent {
    if e.vim_mode != p.editor_vim_mode {
        log::debug!(
            "settings v1->v2: old 'editor'.vimMode ({}) disagrees with preferences.vimMode ({}); keeping preferences",
            e.vim_mode, p.editor_vim_mode
        );
    }
    if e.relative_number != p.editor_relative_line_numbers {
        log::debug!(
            "settings v1->v2: old 'editor'.relativeNumber ({}) disagrees with preferences.editorRelativeLineNumbers ({}); keeping preferences",
            e.relative_number, p.editor_relative_line_numbers
        );
    }
    EditorContent {
        editor_font_family: Some(p.editor_font_family.clone()),
        editor_font_size: Some(p.editor_font_size),
        editor_line_height: Some(p.editor_line_height),
        editor_tab_size: Some(p.editor_tab_size),
        editor_word_wrap: Some(p.editor_word_wrap),
        editor_line_numbers: Some(p.editor_line_numbers),
        editor_relative_line_numbers: Some(p.editor_relative_line_numbers),
        editor_indent_with_tabs: Some(p.editor_indent_with_tabs),
        editor_format_on_save: Some(p.editor_format_on_save),
        editor_trim_trailing_whitespace: Some(p.editor_trim_trailing_whitespace),
        editor_insert_final_newline: Some(p.editor_insert_final_newline),
        editor_bracket_matching: Some(p.editor_bracket_matching),
        editor_show_cursor_position: Some(p.editor_show_cursor_position),
        editor_show_selection_stats: Some(p.editor_show_selection_stats),
        editor_show_outline: Some(p.editor_show_outline),
        editor_indentation_guides: Some(p.editor_indentation_guides),
        editor_vim_mode: Some(p.editor_vim_mode),
        editor_theme: Some(p.editor_theme.clone()),
        vim_hlsearch: Some(e.hlsearch),
        vim_incsearch: Some(e.incsearch),
        vim_smartcase: Some(e.smartcase),
    }
}

fn file_manager_from(p: &Preferences) -> FileManagerContent {
    FileManagerContent {
        explorer_show_hidden_by_default: Some(p.explorer_show_hidden_by_default),
        // Zed-parity Phase 3 tree options — no v1 `Preferences` equivalent, so
        // a migrated file just inherits the shipped defaults.
        explorer_indent_guides: FileManagerContent::defaults().explorer_indent_guides,
        explorer_sticky_ancestors: FileManagerContent::defaults().explorer_sticky_ancestors,
        explorer_auto_reveal_active_file: FileManagerContent::defaults()
            .explorer_auto_reveal_active_file,
        explorer_fold_single_child_dirs: FileManagerContent::defaults()
            .explorer_fold_single_child_dirs,
        explorer_git_decorations: FileManagerContent::defaults().explorer_git_decorations,
        scm_file_tree: FileManagerContent::defaults().scm_file_tree,
    }
}

/// Every field of `Preferences` this migration deliberately leaves alone
/// (owned by another migration / has no `SettingsContent` counterpart), by
/// its `#[serde(rename_all = "camelCase")]` JSON key.
#[cfg_attr(not(test), allow(dead_code))]
const SKIPPED_PREFERENCES_FIELDS: &[&str] = &[
    // T18-006's job (`statusBarItemPlacements`); no `SettingsContent`
    // counterpart (see `content_bridge.rs`).
    "barItemPlacements",
    // Host-manager layout is owned by the Hosts capability and is not a
    // SettingsContent value. Host data migration has its own named step.
    "hmLayout",
    "hmSort",
    "hmCardScale",
];

/// Legacy layout values are consumed by `labonair-workspace` before this
/// migrator runs. They remain in `Preferences` only as a wire-compatibility
/// shape for old config files and must not re-enter `SettingsContent`.
#[cfg_attr(not(test), allow(dead_code))]
const WORKSPACE_LAYOUT_FIELDS: &[&str] = &[
    "sidebarPosition",
    "sidebarOpen",
    "sidebarActivePanel",
    "sidebarRightOpen",
    "sidebarRightActivePanel",
    "sidebarWidth",
    "sidebarRightWidth",
    "dockLayout",
];

/// Legacy terminal preferences that have no native GPUI consumer. They stay
/// in `Preferences` only so old files can still be deserialized and are not
/// emitted into the current typed Settings model.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_TERMINAL_FIELDS: &[&str] = &[
    "terminalFontWeight",
    "terminalLetterSpacing",
    "terminalCursorBlinkInterval",
    "terminalWordSeparator",
    "terminalScrollSensitivity",
    "terminalFastScrollModifier",
    "terminalUseWebgl",
    "terminalComposerEnabled",
    "terminalComposerHistoryPopup",
    "terminalComposerArgumentCompletion",
    "terminalBlocksEnabled",
    "terminalBlocksAutoCollapseOnAltScreen",
];

/// Legacy general preferences with no native runtime consumer. They remain
/// readable in the backend wire shape solely for old configuration files.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_GENERAL_FIELDS: &[&str] = &[
    "startupTerminalCount",
    "autostart",
    "credentialEncryption",
    "confirmQuitWithSsh",
];

/// Legacy appearance preferences with no native GPUI consumer. They remain
/// readable in the backend wire shape solely for old configuration files.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_APPEARANCE_FIELDS: &[&str] = &[
    "sidebarTabInfoLine",
    "sidebarGroupByFolder",
    "sidebarGroupSingleTabs",
    "badgesAlwaysVisible",
    "titlebarsIconsPosition",
];

/// Legacy editor preferences with no native editor consumer. They remain
/// readable in the backend wire shape solely for old configuration files.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_EDITOR_FIELDS: &[&str] = &[
    "editorAutoSave",
    "editorAutoSaveDelay",
    "editorAutocompleteDebounceMs",
    "editorMaxFileSizeMb",
];

/// Legacy SFTP and transfer preferences now owned by the SFTP/transfer
/// runtime, not by the general Settings value store. The current native
/// transfer worker keeps its own validated runtime policy.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_FILE_MANAGER_FIELDS: &[&str] = &[
    "sftpShowHiddenFiles",
    "sftpShowUpFolder",
    "sftpColumnSize",
    "sftpColumnModified",
    "sftpColumnPermissions",
    "sftpColumnType",
    "sftpRemoteEditShowTransfers",
    "sftpMaxRemoteFileSizeMb",
    "sftpFontSize",
    "sftpMaxConcurrentTransfers",
    "sftpDefaultConflictResolution",
    "sftpChunkSizeKb",
    "sftpOnFolderFileError",
];

/// Legacy connection timing preferences had no native Settings consumer.
/// Connection definitions and runtime policy belong to their transport/Hosts
/// owners and remain readable here only for old configuration files.
#[cfg_attr(not(test), allow(dead_code))]
const REMOVED_CONNECTION_FIELDS: &[&str] = &[
    "hostPingInterval",
    "sshConnectTimeoutSecs",
    "sshAutoReconnect",
    "sshAutoReconnectDelay",
    "sshAutoReconnectMaxAttempts",
    "explorerRemotePollInterval",
    "explorerAutoReconnect",
    "explorerIdleSessionTimeoutMin",
    "explorerMaxIdleSessions",
    "explorerMaxCachedRemoteScopes",
];

/// Legacy appearance value converted to the current typed scale field before
/// Settings is written.
#[cfg_attr(not(test), allow(dead_code))]
const MOVED_APPEARANCE_FIELDS: &[&str] = &["appCornerRadius"];

/// Preferences fields with no `SettingsContent` destination, preserved
/// losslessly under `_migratedUnknown.preferences.*` instead of a mapped
/// area field.
#[cfg_attr(not(test), allow(dead_code))]
const UNKNOWN_PREFERENCES_FIELDS: &[&str] = &["barLayoutMigrated"];

/// Old `"editor"` key (`EditorPrefs`) fields with no `SettingsContent`
/// destination (Vim `:set` internals never exposed as a setting row),
/// preserved under `_migratedUnknown.editor.*`. Documented alongside
/// [`SKIPPED_PREFERENCES_FIELDS`]/[`UNKNOWN_PREFERENCES_FIELDS`] even though
/// the actual writes in [`migrate_settings_v1_to_v2`] are hand-rolled
/// per-field (the four fields have three different JSON value types).
#[allow(dead_code)]
const UNKNOWN_EDITOR_FIELDS: &[&str] = &["number", "expandtab", "tabstop", "shiftwidth"];

fn merge_object(dst: &mut Map<String, Value>, key: &str, patch: Map<String, Value>) {
    let mut obj = dst
        .get(key)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    for (k, v) in patch {
        obj.insert(k, v);
    }
    dst.insert(key.to_string(), Value::Object(obj));
}

/// Recursively drop, from `value`, every object key that either holds JSON
/// `null` or `deep_eq`s the same key in `default`. Keys absent from `default`
/// are kept as-is; a nested object that empties out is dropped too. `removed`
/// is incremented once per dropped key. In this settings model a leaf is an
/// `Option<T>`, so `null` == "unset at this layer" == absent — dropping it
/// loses nothing.
fn strip_defaults(value: &mut Value, default: &Value, removed: &mut usize) {
    let Value::Object(def) = default else { return };
    let Value::Object(obj) = value else { return };
    let before = obj.len();
    obj.retain(|_k, v| {
        if v.is_null() {
            return false;
        }
        let Some(dv) = def.get(_k) else {
            return true;
        };
        // Numbers: tolerant compare. `f32` settings leaves (line heights,
        // letter spacing, …) widen to `f64` when serialized, and serde_json's
        // default parser doesn't round-trip `f64` exactly, so a value read
        // back from `config.json` can differ from a freshly-computed default
        // by ~1e-16. Any *real* override differs by orders of magnitude more.
        if let (Some(a), Some(b)) = (v.as_f64(), dv.as_f64()) {
            let tol = 1e-9 * a.abs().max(b.abs()).max(1.0);
            return (a - b).abs() > tol;
        }
        if *v == *dv {
            return false;
        }
        if v.is_object() && dv.is_object() {
            strip_defaults(v, dv, removed);
            return !v.as_object().is_some_and(|o| o.is_empty());
        }
        true
    });
    *removed += before - obj.len();
}

/// Apply [`strip_defaults`] to every `SettingsContent` area object present in
/// `settings`, using `SettingsContent::defaults()` as the reference, and drop
/// any area object that ends up empty. Returns the number of leaves/areas
/// removed. Shared by the fresh v1->v2 migration and the standalone
/// [`sparsify_v2_settings`] cleanup.
fn sparsify_settings_map(settings: &mut Map<String, Value>) -> usize {
    let defaults = serde_json::to_value(SettingsContent::defaults())
        .expect("SettingsContent::defaults() always serializes");
    let Value::Object(def_obj) = defaults else {
        return 0;
    };
    let mut removed = 0usize;
    for area in SETTINGS_CONTENT_AREAS {
        let Some(def_area) = def_obj.get(*area) else {
            continue;
        };
        let empty = {
            let Some(v) = settings.get_mut(*area) else {
                continue;
            };
            strip_defaults(v, def_area, &mut removed);
            v.as_object().is_some_and(|o| o.is_empty())
        };
        if empty {
            settings.remove(*area);
            removed += 1;
        }
    }
    removed
}

/// One-time, idempotent cleanup of a `schemaVersion: 2` `config.json` that
/// was written *before* the migrator learned to emit only overrides — i.e. a
/// file where every `SettingsContent` area was spelled out in full, all
/// values equal to their defaults. Strips it back to just the user's real
/// overrides (`default.json` remains the full reference) and stamps
/// `sparsified: true` so it never runs twice. Best-effort: safe to call
/// unconditionally on every startup, right after [`migrate_settings_v1_to_v2`]
/// (a fresh v1->v2 migration already stamps `sparsified: true` itself, so
/// this then no-ops).
pub fn sparsify_v2_settings(dir: &Path) -> Result<SparsifyOutcome, String> {
    let mut settings = read_settings_from(dir);

    if settings.get(KEY_SCHEMA_VERSION).and_then(Value::as_u64) != Some(SCHEMA_VERSION_V2) {
        return Ok(SparsifyOutcome::NotV2);
    }
    let background_migration_needed = settings
        .get("appearance")
        .and_then(Value::as_object)
        .is_some_and(|appearance| {
            BACKGROUND_KEYS
                .iter()
                .any(|key| appearance.contains_key(*key))
        });
    let corner_radius_migration_needed = settings
        .get("appearance")
        .and_then(Value::as_object)
        .is_some_and(|appearance| appearance.contains_key("appCornerRadius"));
    if settings.get(KEY_SPARSIFIED).and_then(Value::as_bool) == Some(true)
        && !background_migration_needed
        && !corner_radius_migration_needed
    {
        return Ok(SparsifyOutcome::AlreadySparse);
    }

    let path = dir.join(CONFIG_FILE);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }

    let moved_background = move_background_values_to_owner(&mut settings);
    let moved_corner_radius = move_legacy_corner_radius(&mut settings);
    let removed = moved_background + moved_corner_radius + sparsify_settings_map(&mut settings);
    settings.insert(KEY_SPARSIFIED.to_string(), Value::Bool(true));
    write_settings_to(dir, &settings)?;

    log::info!("sparsified config.json (removed {removed} default-valued key(s))");
    Ok(SparsifyOutcome::Sparsified { removed })
}

/// One-time, idempotent migration of the legacy `preferences`/`editor`/`mcp`
/// top-level keys into the flat `SettingsContent` area layout, and of
/// `preferences.keybinds` into `keymap.json`. Safe to call unconditionally
/// on every startup, **before** `labonair_settings::init` (T19-002) reads
/// the same file — see `crates/app/src/main.rs`.
pub fn migrate_settings_v1_to_v2(dir: &Path) -> Result<SettingsV2Outcome, String> {
    let mut settings = read_settings_from(dir);

    if settings.get(KEY_SCHEMA_VERSION).and_then(Value::as_u64) == Some(SCHEMA_VERSION_V2) {
        return Ok(SettingsV2Outcome::AlreadyMigrated);
    }

    if !settings.contains_key(KEY_PREFERENCES) {
        return Ok(if settings.is_empty() {
            SettingsV2Outcome::NothingToMigrate
        } else {
            // No legacy `preferences` blob but the file has *something* —
            // either already new-shaped (has `general`/…) or an unrelated
            // shape; either way there is nothing this step needs to do.
            SettingsV2Outcome::AlreadyMigrated
        });
    }

    let path = dir.join(CONFIG_FILE);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }

    let raw_preferences = settings.remove(KEY_PREFERENCES).expect("checked above");
    let raw_editor = settings.remove(KEY_EDITOR);
    // Keep the standalone `mcp` object untouched. It is owned and loaded by
    // the MCP capability, not by SettingsContent.

    // `preferences.keybinds` was removed from the `Preferences` struct by
    // T19-008 (keybinds now live in their own `keymap.json`), so a pre-T19-008
    // file's overrides must be read straight off the raw JSON — deserializing
    // into today's `Preferences` would silently drop the unknown field.
    let keybinds: std::collections::BTreeMap<String, String> = raw_preferences
        .get("keybinds")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let prefs: Preferences = serde_json::from_value(raw_preferences).unwrap_or_default();
    let editor_prefs: EditorPrefs = raw_editor
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let areas: &[(&str, Value)] = &[
        (
            "general",
            serde_json::to_value(general_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "appearance",
            serde_json::to_value(appearance_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "terminal",
            serde_json::to_value(terminal_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "editor",
            serde_json::to_value(editor_from(&prefs, &editor_prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "fileManager",
            serde_json::to_value(file_manager_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "workspace",
            serde_json::to_value(workspace_from(&prefs)).map_err(|e| e.to_string())?,
        ),
    ];
    for (key, value) in areas {
        settings.insert((*key).to_string(), value.clone());
    }
    insert_background_values(&mut settings, &prefs);

    // `_migratedUnknown` — fields with no `SettingsContent` destination,
    // preserved losslessly rather than dropped. Only written when it has real
    // content: `barLayoutMigrated` defaults to `false` (absence is
    // unambiguous), and the old `"editor"` Vim `:set` internals only exist if
    // there was an `"editor"` key at all.
    let mut unknown_prefs = Map::new();
    if prefs.bar_layout_migrated {
        unknown_prefs.insert("barLayoutMigrated".to_string(), Value::Bool(true));
    }
    let mut unknown_editor = Map::new();
    if raw_editor.is_some() {
        unknown_editor.insert("number".to_string(), Value::Bool(editor_prefs.number));
        unknown_editor.insert("expandtab".to_string(), Value::Bool(editor_prefs.expandtab));
        unknown_editor.insert(
            "tabstop".to_string(),
            Value::from(editor_prefs.tabstop as u64),
        );
        unknown_editor.insert(
            "shiftwidth".to_string(),
            Value::from(editor_prefs.shiftwidth as u64),
        );
    }
    let mut unknown = Map::new();
    if !unknown_prefs.is_empty() {
        unknown.insert("preferences".to_string(), Value::Object(unknown_prefs));
    }
    if !unknown_editor.is_empty() {
        unknown.insert("editor".to_string(), Value::Object(unknown_editor));
    }
    if !unknown.is_empty() {
        merge_object(&mut settings, KEY_MIGRATED_UNKNOWN, unknown);
    }

    // The pre-migration file is preserved as `config.json.bak` (written
    // above) — no need to also carry verbatim `*_legacy` blobs inside the
    // live file.

    settings.insert(
        KEY_SCHEMA_VERSION.to_string(),
        Value::from(SCHEMA_VERSION_V2),
    );

    let keybinds_migrated = write_keymap_overrides(dir, &keybinds)?;

    // Strip every area leaf that merely restates its default, so the file
    // holds only the user's real overrides (`default.json` is the full
    // reference). Stamp `sparsified` so the standalone cleanup no-ops.
    sparsify_settings_map(&mut settings);
    settings.insert(KEY_SPARSIFIED.to_string(), Value::Bool(true));

    write_settings_to(dir, &settings)?;

    log::info!(
        "migrated config.json to schemaVersion 2 ({} keybind override(s))",
        keybinds_migrated
    );

    Ok(SettingsV2Outcome::Migrated { keybinds_migrated })
}

// ─────────────────────────────────────────────────────────────────────────
// Keybinds -> keymap.json
// ─────────────────────────────────────────────────────────────────────────

/// Old `ShortcutId` slug -> new `CommandId::action_name()` string. Kept as a
/// self-contained string table (rather than importing `CommandId` itself)
/// because `labonair-backend` must not depend on `labonair-command-palette`
/// (that edge already runs the other way — see `scripts/check_crate_deps.py`).
/// Verified 1:1 against `crates/command-palette/src/keybind.rs`'s
/// `shortcut_slug` and `crates/command-palette/src/palette.rs`'s
/// `ACTION_NAMES` (T17-007/T19-008).
const SLUG_TO_ACTION: &[(&str, &str)] = &[
    ("command.palette", "command_palette::Toggle"),
    ("shortcuts.open", "settings::OpenShortcuts"),
    ("tab.new", "tab::NewTerminal"),
    ("tab.newPreview", "tab::NewPreview"),
    ("tab.newEditor", "tab::NewEditor"),
    ("tab.close", "tab::Close"),
    ("tab.next", "tab::Next"),
    ("tab.prev", "tab::Prev"),
    ("tab.selectTab1", "tab::Select1"),
    ("tab.selectTab2", "tab::Select2"),
    ("tab.selectTab3", "tab::Select3"),
    ("tab.selectTab4", "tab::Select4"),
    ("tab.selectTab5", "tab::Select5"),
    ("tab.selectTab6", "tab::Select6"),
    ("tab.selectTab7", "tab::Select7"),
    ("tab.selectTab8", "tab::Select8"),
    ("tab.selectTab9", "tab::Select9"),
    ("pane.splitRight", "pane::SplitRight"),
    ("pane.splitDown", "pane::SplitDown"),
    ("pane.close", "pane::Close"),
    ("pane.focusNext", "pane::FocusNext"),
    ("search.focus", "search::Toggle"),
    ("sidebar.toggle", "sidebar::Toggle"),
    ("view.zenMode", "view::ToggleZenMode"),
    ("view.zoomIn", "view::ZoomIn"),
    ("view.zoomOut", "view::ZoomOut"),
    ("view.zoomReset", "view::ZoomReset"),
];

/// Writes `keymap.json` from `preferences.keybinds` overrides
/// (`{ slug: "cmd-x" }`, `""` = unbind), one flat `{ "context": null,
/// "bindings": {...} }` block (no per-command default context table exists
/// pre-T19-008, so every migrated binding is global — matches the shipped
/// `default-{macos,linux}.json` assets' own predominant pattern). Returns
/// the number of overrides migrated. Does **not** create a file when there
/// are no overrides (Anweisung #3).
fn write_keymap_overrides(
    dir: &Path,
    keybinds: &std::collections::BTreeMap<String, String>,
) -> Result<usize, String> {
    if keybinds.is_empty() {
        return Ok(0);
    }

    let mut bindings = Map::new();
    let mut migrated = 0usize;
    for (slug, keystroke) in keybinds {
        let Some((_, action)) = SLUG_TO_ACTION.iter().find(|(s, _)| s == slug) else {
            log::debug!("settings v1->v2: unknown keybind slug '{slug}', dropping override");
            continue;
        };
        let value = if keystroke.is_empty() {
            Value::Null
        } else {
            Value::String(action.to_string())
        };
        bindings.insert(keystroke_key(keystroke, action), value);
        migrated += 1;
    }
    if bindings.is_empty() {
        return Ok(0);
    }

    let mut block = Map::new();
    block.insert("context".to_string(), Value::Null);
    block.insert("bindings".to_string(), Value::Object(bindings));
    let doc = Value::Array(vec![Value::Object(block)]);
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;

    let path = dir.join("keymap.json");
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;

    Ok(migrated)
}

/// `keymap.json`'s object key for one binding is the keystroke itself; an
/// unbind (`""` in the old model) has no keystroke to key by, so it keys by
/// the action's own default keystroke instead — irrelevant in practice since
/// the value is `null` either way, but keeps every entry keyed by a real
/// keystroke string.
fn keystroke_key(keystroke: &str, action: &str) -> String {
    if keystroke.is_empty() {
        format!("unbind:{action}")
    } else {
        keystroke.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_settings_content::MergeFrom;
    use std::collections::BTreeSet;

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "labonair-settings-v2-migration-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `SettingsContent::defaults()` folded through a `config.json` map's
    /// area keys, exactly as `labonair-settings`' `SettingsStore` would merge
    /// the User layer — lets a test assert "sparsifying changed nothing an
    /// app would observe".
    fn merged_from_map(map: &Map<String, Value>) -> SettingsContent {
        let user: SettingsContent =
            serde_json::from_value(Value::Object(map.clone())).unwrap_or_default();
        let mut merged = SettingsContent::defaults();
        merged.merge_from(&user);
        merged
    }

    fn full_legacy_settings() -> Value {
        let mut prefs = serde_json::to_value(Preferences::default()).unwrap();
        prefs["terminalFontSize"] = Value::from(42);
        // Host-manager layout is intentionally not migrated into SettingsContent.
        prefs["hmLayout"] = Value::from("list");
        // A genuinely unrepresentable field set to a non-default value — must
        // be preserved verbatim under `_migratedUnknown`.
        prefs["barLayoutMigrated"] = Value::from(true);
        prefs["keybinds"] = serde_json::json!({
            "tab.new": "cmd-t",
            "view.zoomIn": "",
            "unknown.slug": "cmd-9",
        });
        serde_json::json!({
            "preferences": prefs,
            "editor": { "vimMode": true, "hlsearch": false, "number": true, "tabstop": 8 },
            "mcp": { "bridgeEnabled": true, "bridgePort": 51000, "maxCommandTimeoutSecs": 60, "autoRevokeMinutes": 5, "notifyOnActivity": true },
            "statusBarItemPlacements": { "cwd": { "side": "left", "hidden": false } },
        })
    }

    #[test]
    fn every_preferences_field_is_accounted_for() {
        let value = serde_json::to_value(Preferences::default()).unwrap();
        let obj = value.as_object().unwrap();
        // Every field this migration maps into a `SettingsContent` area, by
        // its `Preferences` JSON key (camelCase, with the struct's own
        // `#[serde(rename = ..)]` overrides applied).
        let mapped: &[&str] = &[
            "theme",
            "restoreWindowState",
            "defaultStartupTab",
            "startupTerminalCount",
            "autostart",
            "credentialEncryption",
            "confirmQuitWithSsh",
            "checkForUpdates",
            "sessionRestore",
            "appTheme",
            "iconTheme",
            "themeVariantOverrides",
            "appFontSize",
            "appLineHeight",
            "appFontFamily",
            "reduceMotion",
            "backgroundImage",
            "backgroundOpacity",
            "backgroundBlur",
            "backgroundTintColor",
            "backgroundTintOpacity",
            "tabsLocation",
            "sidebarTabInfoLine",
            "sidebarGroupByFolder",
            "sidebarGroupSingleTabs",
            "badgesAlwaysVisible",
            "titlebarsIconsPosition",
            "zenModeShowHeader",
            "zenModeShowStatusbar",
            "statusBarShowExplorerButton",
            "statusBarShowSnippetsButton",
            "statusBarShowSourceControlButton",
            "statusBarShowTabsButton",
            "statusBarShowCwdBreadcrumb",
            "statusBarShowPreviewUrl",
            "statusBarShowAiControls",
            "sidebarPosition",
            "sidebarOpen",
            "sidebarActivePanel",
            "sidebarRightOpen",
            "sidebarRightActivePanel",
            "sidebarWidth",
            "sidebarRightWidth",
            "dockLayout",
            "hmLayout",
            "hmSort",
            "hmCardScale",
            "terminalShell",
            "terminalDefaultPath",
            "newTabInheritsCwd",
            "confirmCloseTerminalTab",
            "terminalFontFamily",
            "terminalFontSize",
            "terminalFontWeight",
            "terminalLetterSpacing",
            "terminalLineHeight",
            "terminalScrollback",
            "sessionScrollbackLines",
            "scrollbackMaxSizeMb",
            "scrollbackRetentionDays",
            "terminalCursorStyle",
            "terminalCursorBlink",
            "terminalCursorBlinkInterval",
            "terminalCopyOnSelect",
            "terminalRightClickPastes",
            "terminalWordSeparator",
            "terminalScrollSensitivity",
            "terminalFastScrollModifier",
            "terminalShowPaneHeader",
            "terminalShowPaneFooter",
            "terminalUseWebgl",
            "terminalComposerEnabled",
            "terminalComposerHistoryPopup",
            "terminalComposerArgumentCompletion",
            "terminalBlocksEnabled",
            "terminalBlocksAutoCollapseOnAltScreen",
            "terminalBell",
            "terminalOpacity",
            "editorFontFamily",
            "editorFontSize",
            "editorLineHeight",
            "editorTabSize",
            "editorWordWrap",
            "editorLineNumbers",
            "editorRelativeLineNumbers",
            "editorIndentWithTabs",
            "editorFormatOnSave",
            "editorTrimTrailingWhitespace",
            "editorInsertFinalNewline",
            "editorBracketMatching",
            "editorShowCursorPosition",
            "editorShowSelectionStats",
            "editorShowOutline",
            "editorIndentationGuides",
            "editorAutoSave",
            "editorAutoSaveDelay",
            "editorAutocompleteDebounceMs",
            "editorMaxFileSizeMb",
            "vimMode",
            "editorTheme",
            "sftpShowHiddenFiles",
            "sftpShowUpFolder",
            "explorerShowHiddenByDefault",
            "sftpColumnSize",
            "sftpColumnModified",
            "sftpColumnPermissions",
            "sftpColumnType",
            "sftpRemoteEditShowTransfers",
            "sftpMaxRemoteFileSizeMb",
            "sftpFontSize",
            "sftpMaxConcurrentTransfers",
            "sftpDefaultConflictResolution",
            "sftpChunkSizeKb",
            "sftpOnFolderFileError",
            "hostPingInterval",
            "sshConnectTimeoutSecs",
            "sshAutoReconnect",
            "sshAutoReconnectDelay",
            "sshAutoReconnectMaxAttempts",
            "explorerRemotePollInterval",
            "explorerAutoReconnect",
            "explorerIdleSessionTimeoutMin",
            "explorerMaxIdleSessions",
            "explorerMaxCachedRemoteScopes",
            "commandPaletteSearchMode",
            "commandPaletteShowRecent",
            "commandPaletteBlur",
            "commandPaletteOpacity",
            "commandPalettePosition",
            "commandPaletteAnimation",
            "commandPaletteHistorySize",
            "commandPaletteCloseOnOverlayClick",
            "gitStatusPollIntervalMs",
            "mcpBridgeEnabled",
            "mcpBridgePort",
            "mcpMaxCommandTimeoutSecs",
            "mcpAutoRevokeMinutes",
            "mcpNotifyOnActivity",
        ];
        let mut accounted: BTreeSet<&str> = mapped.iter().copied().collect();
        accounted.extend(SKIPPED_PREFERENCES_FIELDS.iter().copied());
        accounted.extend(WORKSPACE_LAYOUT_FIELDS.iter().copied());
        accounted.extend(REMOVED_TERMINAL_FIELDS.iter().copied());
        accounted.extend(REMOVED_GENERAL_FIELDS.iter().copied());
        accounted.extend(REMOVED_APPEARANCE_FIELDS.iter().copied());
        accounted.extend(REMOVED_EDITOR_FIELDS.iter().copied());
        accounted.extend(REMOVED_FILE_MANAGER_FIELDS.iter().copied());
        accounted.extend(REMOVED_CONNECTION_FIELDS.iter().copied());
        accounted.extend(MOVED_APPEARANCE_FIELDS.iter().copied());
        accounted.extend(UNKNOWN_PREFERENCES_FIELDS.iter().copied());

        let all_keys: BTreeSet<&str> = obj.keys().map(|s| s.as_str()).collect();
        let missing: Vec<&&str> = mapped.iter().filter(|k| !all_keys.contains(**k)).collect();
        assert!(
            missing.is_empty(),
            "mapped field(s) not present on Preferences (typo?): {missing:?}"
        );
        assert_eq!(
            all_keys, accounted,
            "every Preferences field must be mapped, skipped, or listed as unknown"
        );
    }

    #[test]
    fn v1_background_values_are_migrated_to_the_background_owner() {
        let dir = tmp("background-v1");
        let mut prefs = serde_json::to_value(Preferences::default()).unwrap();
        prefs["backgroundImage"] = Value::from("wallpaper.png");
        prefs["backgroundOpacity"] = Value::from(45);
        prefs["appCornerRadius"] = Value::from(10);
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&serde_json::json!({ "preferences": prefs })).unwrap(),
        )
        .unwrap();

        migrate_settings_v1_to_v2(&dir).unwrap();
        let after = read_settings_from(&dir);
        assert_eq!(after["backgroundImage"], Value::from("wallpaper.png"));
        assert_eq!(after["backgroundOpacity"], Value::from(45));
        assert_eq!(after["appearance"]["cornerRadiusScale"], Value::from(2.0));
        assert!(after
            .get("appearance")
            .and_then(Value::as_object)
            .is_none_or(|appearance| !appearance.contains_key("backgroundImage")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn v2_background_values_move_even_when_already_sparsified() {
        let dir = tmp("background-v2");
        let mut appearance = Map::new();
        appearance.insert("backgroundImage".into(), Value::from("wallpaper.png"));
        appearance.insert("backgroundOpacity".into(), Value::from(45));
        let doc = serde_json::json!({
            "schemaVersion": 2,
            "sparsified": true,
            "appearance": appearance,
        });
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            sparsify_v2_settings(&dir).unwrap(),
            SparsifyOutcome::Sparsified { .. }
        ));
        let after = read_settings_from(&dir);
        assert_eq!(after["backgroundImage"], Value::from("wallpaper.png"));
        assert_eq!(after["backgroundOpacity"], Value::from(45));
        assert!(after
            .get("appearance")
            .and_then(Value::as_object)
            .is_none_or(|appearance| !appearance.contains_key("backgroundImage")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_corner_radius_becomes_the_current_scale() {
        let dir = tmp("corner-radius-v2");
        let doc = serde_json::json!({
            "schemaVersion": 2,
            "sparsified": true,
            "appearance": { "appCornerRadius": 10 }
        });
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            sparsify_v2_settings(&dir).unwrap(),
            SparsifyOutcome::Sparsified { .. }
        ));
        let after = read_settings_from(&dir);
        assert_eq!(after["appearance"]["cornerRadiusScale"], Value::from(2.0));
        assert!(!after["appearance"]
            .as_object()
            .unwrap()
            .contains_key("appCornerRadius"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn full_migration_moves_every_field_and_counts_match() {
        let dir = tmp("full");
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&full_legacy_settings()).unwrap(),
        )
        .unwrap();

        let outcome = migrate_settings_v1_to_v2(&dir).unwrap();
        assert_eq!(
            outcome,
            SettingsV2Outcome::Migrated {
                keybinds_migrated: 2
            }
        );

        let after = read_settings_from(&dir);
        assert_eq!(after.get(KEY_SCHEMA_VERSION).unwrap(), &Value::from(2));
        assert_eq!(after.get(KEY_SPARSIFIED).unwrap(), &Value::from(true));

        // Non-default values survive sparsification, in the right area.
        assert_eq!(after["terminal"]["terminalFontSize"], Value::from(42));
        assert_eq!(after["editor"]["vimHlsearch"], Value::from(false));
        assert_eq!(after["mcp"]["bridgePort"], Value::from(51000));
        assert!(!after.contains_key("hosts"));

        // Unrelated top-level key (T18-006) survives untouched.
        assert_eq!(
            after["statusBarItemPlacements"]["cwd"]["side"],
            Value::from("left")
        );

        // The pre-migration file is preserved only as `.bak`; no verbatim
        // `*_legacy` blobs are carried inside the live file any more.
        assert!(dir.join("config.json.bak").exists());
        assert!(!after.contains_key("preferences"));
        assert!(!after.contains_key("preferences_legacy"));
        assert!(!after.contains_key("editor_legacy"));
        assert!(!after.contains_key("mcp_legacy"));

        // Genuinely unrepresentable fields are still preserved losslessly.
        assert_eq!(
            after["_migratedUnknown"]["preferences"]["barLayoutMigrated"],
            Value::from(true)
        );
        assert_eq!(
            after["_migratedUnknown"]["editor"]["tabstop"],
            Value::from(8)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keybind_overrides_migrate_including_unbind() {
        let dir = tmp("keybinds");
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&full_legacy_settings()).unwrap(),
        )
        .unwrap();

        migrate_settings_v1_to_v2(&dir).unwrap();

        let keymap_text = std::fs::read_to_string(dir.join("keymap.json")).unwrap();
        let keymap: Value = serde_json::from_str(&keymap_text).unwrap();
        let block = &keymap[0];
        assert_eq!(block["context"], Value::Null);
        assert_eq!(block["bindings"]["cmd-t"], Value::from("tab::NewTerminal"));
        // Unbind (old empty-string override) -> null, keyed by a placeholder
        // since there is no keystroke to key it by.
        assert_eq!(block["bindings"]["unbind:view::ZoomIn"], Value::Null);
        // Unknown slug silently dropped, not migrated.
        assert!(!keymap_text.contains("unknown.slug"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_keybind_overrides_writes_no_keymap_file() {
        let dir = tmp("no-keybinds");
        let mut legacy = full_legacy_settings();
        legacy["preferences"]["keybinds"] = serde_json::json!({});
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        migrate_settings_v1_to_v2(&dir).unwrap();
        assert!(!dir.join("keymap.json").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_call_is_a_no_op() {
        let dir = tmp("idempotent");
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&full_legacy_settings()).unwrap(),
        )
        .unwrap();

        migrate_settings_v1_to_v2(&dir).unwrap();
        let after_first = read_settings_from(&dir);

        let outcome = migrate_settings_v1_to_v2(&dir).unwrap();
        assert_eq!(outcome, SettingsV2Outcome::AlreadyMigrated);

        let after_second = read_settings_from(&dir);
        assert_eq!(after_first, after_second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn already_v2_file_is_untouched() {
        let dir = tmp("already-v2");
        let doc = serde_json::json!({ "schemaVersion": 2, "general": { "theme": "dark" } });
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();

        let outcome = migrate_settings_v1_to_v2(&dir).unwrap();
        assert_eq!(outcome, SettingsV2Outcome::AlreadyMigrated);
        assert_eq!(read_settings_from(&dir), doc.as_object().unwrap().clone());
        assert!(!dir.join("config.json.bak").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_case_is_nothing_to_migrate() {
        let dir = tmp("empty");
        let outcome = migrate_settings_v1_to_v2(&dir).unwrap();
        assert_eq!(outcome, SettingsV2Outcome::NothingToMigrate);
        assert!(!dir.join(CONFIG_FILE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_file_with_only_preferences_still_migrates() {
        let dir = tmp("partial");
        let doc = serde_json::json!({ "preferences": serde_json::to_value(Preferences::default()).unwrap() });
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&doc).unwrap(),
        )
        .unwrap();

        let outcome = migrate_settings_v1_to_v2(&dir).unwrap();
        assert_eq!(
            outcome,
            SettingsV2Outcome::Migrated {
                keybinds_migrated: 0
            }
        );

        // Input was `Preferences::default()` end to end, so every migrated
        // area leaf equalled its `SettingsContent` default — nothing is left
        // to persist. `default.json` stays the full reference; `config.json`
        // is just the schema stamp.
        let after = read_settings_from(&dir);
        assert_eq!(after.get(KEY_SCHEMA_VERSION), Some(&Value::from(2)));
        assert_eq!(after.get(KEY_SPARSIFIED), Some(&Value::from(true)));
        assert!(!after.contains_key("preferences"));
        assert!(!after.contains_key("preferences_legacy"));
        assert!(!after.contains_key("editor_legacy"));
        assert!(!after.contains_key("mcp_legacy"));
        assert!(!after.contains_key(KEY_MIGRATED_UNKNOWN));
        for area in SETTINGS_CONTENT_AREAS {
            assert!(
                !after.contains_key(*area),
                "all-default area `{area}` should have been stripped, found {:?}",
                after.get(*area)
            );
        }

        // And an app reading this file sees exactly the shipped defaults.
        assert_eq!(merged_from_map(&after), SettingsContent::defaults());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrator_output_only_carries_real_overrides() {
        let dir = tmp("overrides-only");
        let mut prefs = serde_json::to_value(Preferences::default()).unwrap();
        prefs["terminalFontSize"] = Value::from(20);
        prefs["editorTabSize"] = Value::from(8);
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&serde_json::json!({ "preferences": prefs })).unwrap(),
        )
        .unwrap();

        migrate_settings_v1_to_v2(&dir).unwrap();
        let after = read_settings_from(&dir);

        // The two overridden leaves are present with their new values...
        assert_eq!(after["terminal"]["terminalFontSize"], Value::from(20));
        assert_eq!(after["editor"]["editorTabSize"], Value::from(8));
        // ...and no `null`s leaked in from `appearance_from`'s `..Default`.
        for (_area, v) in after.iter() {
            if let Some(obj) = v.as_object() {
                assert!(
                    obj.values().all(|leaf| !leaf.is_null()),
                    "sparsified config should carry no null leaves: {v}"
                );
            }
        }
        // Untouched, plain-scalar areas are gone entirely (a survivor here
        // means `Preferences::default()` and `<Area>Content::defaults()`
        // have drifted — a real bug, not test brittleness).
        assert!(!after.contains_key("preferences"));
        assert!(!after.contains_key("connections"));
        assert!(!after.contains_key("personalization"));
        assert_eq!(after.get(KEY_SPARSIFIED), Some(&Value::from(true)));

        let mut expected = SettingsContent::defaults();
        expected.terminal.terminal_font_size = Some(20);
        expected.editor.editor_tab_size = Some(8);
        assert_eq!(merged_from_map(&after), expected);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sparsify_v2_settings_cleans_pre_existing_full_file_and_is_idempotent() {
        let dir = tmp("sparsify-standalone");

        // A v2 file written the buggy way: every area spelled out in full,
        // all at their defaults, plus one genuine override and one unrelated
        // key that must be preserved.
        let mut full = serde_json::to_value(SettingsContent::defaults()).unwrap();
        full["terminal"]["terminalFontSize"] = Value::from(20);
        {
            let obj = full.as_object_mut().unwrap();
            obj.insert(KEY_SCHEMA_VERSION.to_string(), Value::from(2));
            obj.insert(
                "statusBarItemPlacements".to_string(),
                serde_json::json!({ "cwd": { "side": "left" } }),
            );
        }
        std::fs::write(
            dir.join(CONFIG_FILE),
            serde_json::to_string_pretty(&full).unwrap(),
        )
        .unwrap();

        let merged_before = merged_from_map(&read_settings_from(&dir));

        let outcome = sparsify_v2_settings(&dir).unwrap();
        assert!(matches!(outcome, SparsifyOutcome::Sparsified { removed } if removed > 0));

        let after = read_settings_from(&dir);
        assert_eq!(
            after["terminal"],
            serde_json::json!({ "terminalFontSize": 20 })
        );
        assert!(!after.contains_key("general"));
        assert!(!after.contains_key("appearance"));
        assert_eq!(
            after["statusBarItemPlacements"]["cwd"]["side"],
            Value::from("left")
        );
        assert_eq!(after.get(KEY_SPARSIFIED), Some(&Value::from(true)));
        assert_eq!(merged_from_map(&after), merged_before);
        assert!(dir.join("config.json.bak").exists());

        // Second run: flag already set, nothing rewritten.
        assert_eq!(
            sparsify_v2_settings(&dir).unwrap(),
            SparsifyOutcome::AlreadySparse
        );
        assert_eq!(read_settings_from(&dir), after);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sparsify_v2_settings_skips_non_v2_files() {
        let dir = tmp("sparsify-non-v2");
        std::fs::write(
            dir.join(CONFIG_FILE),
            r#"{"preferences":{"terminalFontSize":20}}"#,
        )
        .unwrap();
        assert_eq!(sparsify_v2_settings(&dir).unwrap(), SparsifyOutcome::NotV2);
        assert!(!dir.join("config.json.bak").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
