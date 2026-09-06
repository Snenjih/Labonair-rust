//! T19-009: one-time, idempotent migration of the legacy split
//! `preferences`/`editor`/`mcp` top-level keys in `config.json`
//! into the flat, area-based `SettingsContent` layout (T19-001), the
//! `preferences.keybinds` override blob into `keymap.json` (T19-008's file
//! shape), and the SQLite-backed hosts (`backend::modules::hosts`) into
//! `hosts.entries` + the secret store — without losing user data.
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
//! * `hmLayout`/`hmSort`/`hmCardScale` move from the "Sidebar / Host-Manager"
//!   category into the `hosts` area (`hosts.layout`/`hosts.sort`/
//!   `hosts.cardScale`), per T19-001.
//! * `dockLayout`/`sidebar*` (position/open/activePanel/rightOpen/
//!   rightActivePanel/width/rightWidth) move into the `workspace` area.
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
    connections::ConnectionsContent,
    editor::EditorContent,
    file_manager::FileManagerContent,
    general::{self, GeneralContent},
    hosts::{HostAuthMethod, HostEntry, HostTunnel},
    mcp::McpContent,
    personalization::PersonalizationContent,
    terminal::{self, TerminalContent},
    workspace::{self, WorkspaceContent},
    SettingsContent,
};

use super::mcp::McpPrefs;
use super::preferences::{CursorStyle, Preferences, StartupTab, ThemePref};
use super::{editor::EditorPrefs, CONFIG_FILE};
use super::{read_settings_from, write_settings_to};
use crate::modules::hosts::Host;
use crate::modules::secrets::get_password;

const KEY_PREFERENCES: &str = "preferences";
const KEY_EDITOR: &str = "editor";
const KEY_MCP: &str = "mcp";
const KEY_SCHEMA_VERSION: &str = "schemaVersion";
const KEY_MIGRATED_UNKNOWN: &str = "_migratedUnknown";
const KEY_HOSTS_MIGRATED: &str = "hostsMigrated";
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
    "hosts",
    "workspace",
    "ai",
    "mcp",
    "personalization",
    "keymap",
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
// Preferences/editor/mcp -> SettingsContent areas
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
        startup_terminal_count: Some(p.startup_terminal_count),
        autostart: Some(p.autostart),
        credential_encryption: Some(p.credential_encryption),
        notify_on_errors: Some(p.notify_on_errors),
        confirm_quit_with_ssh: Some(p.confirm_quit_with_ssh),
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
        app_corner_radius: Some(p.app_corner_radius),
        background_image: Some(p.background_image.clone()),
        background_opacity: Some(p.background_opacity),
        background_blur: Some(p.background_blur),
        background_tint_color: Some(p.background_tint_color.clone()),
        background_tint_opacity: Some(p.background_tint_opacity),
        tabs_location: Some(p.tabs_location.clone()),
        sidebar_tab_info_line: Some(p.sidebar_tab_info_line.clone()),
        sidebar_group_by_folder: Some(p.sidebar_group_by_folder),
        sidebar_group_single_tabs: Some(p.sidebar_group_single_tabs),
        badges_always_visible: Some(p.badges_always_visible),
        titlebars_icons_position: Some(p.titlebars_icons_position.clone()),
        zen_mode_show_header: Some(p.zen_mode_show_header),
        zen_mode_show_statusbar: Some(p.zen_mode_show_statusbar),
        // T20-007 `theme_settings` fields have no legacy `Preferences` key —
        // they resolve to their `AppearanceContent::defaults()` on read.
        ..AppearanceContent::default()
    }
}

fn personalization_from(p: &Preferences) -> PersonalizationContent {
    PersonalizationContent {
        // Owned exclusively by T18-006/T18-007's own top-level-key
        // migrations (`statusBarItemPlacements`/`panelToggleVisibility`);
        // left unset here so this migration never clobbers them.
        status_bar_item_placements: None,
        panel_toggle_visibility: None,
        status_bar_show_explorer_button: Some(p.status_bar_show_explorer_button),
        status_bar_show_snippets_button: Some(p.status_bar_show_snippets_button),
        status_bar_show_source_control_button: Some(p.status_bar_show_source_control_button),
        status_bar_show_tabs_button: Some(p.status_bar_show_tabs_button),
        status_bar_show_cwd_breadcrumb: Some(p.status_bar_show_cwd_breadcrumb),
        status_bar_show_preview_url: Some(p.status_bar_show_preview_url),
        status_bar_show_ai_controls: Some(p.status_bar_show_ai_controls),
    }
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
        dock_layout: Some(p.dock_layout.clone()),
        sidebar_position: Some(p.sidebar_position.clone()),
        sidebar_open: Some(p.sidebar_open),
        sidebar_active_panel: Some(p.sidebar_active_panel.clone()),
        sidebar_right_open: Some(p.sidebar_right_open),
        sidebar_right_active_panel: Some(p.sidebar_right_active_panel.clone()),
        sidebar_width: Some(p.sidebar_width),
        sidebar_right_width: Some(p.sidebar_right_width),
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
        terminal_font_weight: Some(p.terminal_font_weight.clone()),
        terminal_letter_spacing: Some(p.terminal_letter_spacing),
        terminal_line_height: Some(p.terminal_line_height),
        terminal_scrollback: Some(p.terminal_scrollback),
        session_scrollback_lines: Some(p.session_scrollback_lines),
        scrollback_max_size_mb: Some(p.scrollback_max_size_mb),
        scrollback_retention_days: Some(p.scrollback_retention_days),
        terminal_cursor_style: Some(cursor_style(p.terminal_cursor_style)),
        terminal_cursor_blink: Some(p.terminal_cursor_blink),
        terminal_cursor_blink_interval: Some(p.terminal_cursor_blink_interval),
        terminal_copy_on_select: Some(p.terminal_copy_on_select),
        terminal_right_click_pastes: Some(p.terminal_right_click_pastes),
        terminal_word_separator: Some(p.terminal_word_separator.clone()),
        terminal_scroll_sensitivity: Some(p.terminal_scroll_sensitivity),
        terminal_fast_scroll_modifier: Some(p.terminal_fast_scroll_modifier.clone()),
        terminal_show_pane_header: Some(p.terminal_show_pane_header),
        terminal_show_pane_footer: Some(p.terminal_show_pane_footer),
        terminal_use_webgl: Some(p.terminal_use_webgl),
        terminal_composer_enabled: Some(p.terminal_composer_enabled),
        terminal_composer_history_popup: Some(p.terminal_composer_history_popup),
        terminal_composer_argument_completion: Some(p.terminal_composer_argument_completion),
        terminal_blocks_enabled: Some(p.terminal_blocks_enabled),
        terminal_blocks_auto_collapse_on_alt_screen: Some(
            p.terminal_blocks_auto_collapse_on_alt_screen,
        ),
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
        editor_auto_save: Some(p.editor_auto_save.clone()),
        editor_auto_save_delay: Some(p.editor_auto_save_delay),
        editor_autocomplete_debounce_ms: Some(p.editor_autocomplete_debounce_ms),
        editor_max_file_size_mb: Some(p.editor_max_file_size_mb),
        editor_vim_mode: Some(p.editor_vim_mode),
        editor_theme: Some(p.editor_theme.clone()),
        vim_hlsearch: Some(e.hlsearch),
        vim_incsearch: Some(e.incsearch),
        vim_smartcase: Some(e.smartcase),
    }
}

fn file_manager_from(p: &Preferences) -> FileManagerContent {
    FileManagerContent {
        sftp_show_hidden_files: Some(p.sftp_show_hidden_files),
        sftp_show_up_folder: Some(p.sftp_show_up_folder),
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
        sftp_column_size: Some(p.sftp_column_size),
        sftp_column_modified: Some(p.sftp_column_modified),
        sftp_column_permissions: Some(p.sftp_column_permissions),
        sftp_column_type: Some(p.sftp_column_type),
        sftp_remote_edit_show_transfers: Some(p.sftp_remote_edit_show_transfers),
        sftp_max_remote_file_size_mb: Some(p.sftp_max_remote_file_size_mb),
        sftp_font_size: Some(p.sftp_font_size),
        sftp_max_concurrent_transfers: Some(p.sftp_max_concurrent_transfers),
        sftp_default_conflict_resolution: Some(p.sftp_default_conflict_resolution.clone()),
        sftp_chunk_size_kb: Some(p.sftp_chunk_size_kb),
        sftp_on_folder_file_error: Some(p.sftp_on_folder_file_error.clone()),
    }
}

fn connections_from(p: &Preferences) -> ConnectionsContent {
    ConnectionsContent {
        host_ping_interval: Some(p.host_ping_interval),
        ssh_connect_timeout_secs: Some(p.ssh_connect_timeout_secs),
        ssh_auto_reconnect: Some(p.ssh_auto_reconnect),
        ssh_auto_reconnect_delay: Some(p.ssh_auto_reconnect_delay),
        ssh_auto_reconnect_max_attempts: Some(p.ssh_auto_reconnect_max_attempts),
        explorer_remote_poll_interval: Some(p.explorer_remote_poll_interval),
        explorer_auto_reconnect: Some(p.explorer_auto_reconnect),
        explorer_idle_session_timeout_min: Some(p.explorer_idle_session_timeout_min),
        explorer_max_idle_sessions: Some(p.explorer_max_idle_sessions),
        explorer_max_cached_remote_scopes: Some(p.explorer_max_cached_remote_scopes),
    }
}

/// Just the `hm*` (Host-Manager UI) slice of `hosts` — `entries` is left
/// untouched here (that's [`migrate_hosts_to_settings`]'s job, merged in
/// separately so the two migrations can run independently/idempotently).
fn hosts_hm_from(p: &Preferences) -> (Option<String>, Option<String>, Option<u32>) {
    (
        Some(p.hm_layout.clone()),
        Some(p.hm_sort.clone()),
        Some(p.hm_card_scale),
    )
}

/// Builds the `mcp` area from the old separate `"mcp"` key (`McpPrefs`) —
/// the authoritative source, per `Preferences`' own doc comment on its
/// `mcp_bridge_*` mirror fields.
fn mcp_from(m: &McpPrefs) -> McpContent {
    McpContent {
        bridge_enabled: Some(m.bridge_enabled),
        bridge_port: Some(m.bridge_port as u32),
        max_command_timeout_secs: Some(m.max_command_timeout_secs as u32),
        auto_revoke_minutes: Some(m.auto_revoke_minutes),
        notify_on_activity: Some(m.notify_on_activity),
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
];

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
    if settings.get(KEY_SPARSIFIED).and_then(Value::as_bool) == Some(true) {
        return Ok(SparsifyOutcome::AlreadySparse);
    }

    let path = dir.join(CONFIG_FILE);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }

    let removed = sparsify_settings_map(&mut settings);
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
    let raw_mcp = settings.remove(KEY_MCP);

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
    let mcp_prefs: McpPrefs = raw_mcp
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let (hm_layout, hm_sort, hm_card_scale) = hosts_hm_from(&prefs);

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
            "connections",
            serde_json::to_value(connections_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "workspace",
            serde_json::to_value(workspace_from(&prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "mcp",
            serde_json::to_value(mcp_from(&mcp_prefs)).map_err(|e| e.to_string())?,
        ),
        (
            "personalization",
            serde_json::to_value(personalization_from(&prefs)).map_err(|e| e.to_string())?,
        ),
    ];
    for (key, value) in areas {
        settings.insert((*key).to_string(), value.clone());
    }

    // `hosts.layout`/`hosts.sort`/`hosts.cardScale` — merged into whatever
    // `hosts` object already exists (e.g. a prior `migrate_hosts_to_settings`
    // run's `entries`), never overwriting it wholesale.
    let mut hosts_patch = Map::new();
    hosts_patch.insert(
        "layout".to_string(),
        serde_json::to_value(hm_layout).unwrap(),
    );
    hosts_patch.insert("sort".to_string(), serde_json::to_value(hm_sort).unwrap());
    hosts_patch.insert(
        "cardScale".to_string(),
        serde_json::to_value(hm_card_scale).unwrap(),
    );
    merge_object(&mut settings, "hosts", hosts_patch);

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

// ─────────────────────────────────────────────────────────────────────────
// SQLite hosts -> `hosts.entries` + secret store (Thema 2, best-effort per
// T19-009's own scope note: full reconciliation with the live SQLite store
// is T19-010's job; this only hydrates the settings-content projection so
// the new Settings UI has something to show before T19-010 lands).
// ─────────────────────────────────────────────────────────────────────────

/// Result of [`migrate_hosts_to_settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostsV2Outcome {
    /// The `hostsMigrated` marker was already set; nothing was touched.
    AlreadyMigrated,
    /// No SQLite hosts existed; marker set, nothing to write.
    NothingToMigrate,
    Migrated {
        migrated: usize,
    },
}

fn map_auth_method(s: &str) -> HostAuthMethod {
    match s {
        "key" => HostAuthMethod::PublicKey,
        "agent" => HostAuthMethod::Agent,
        // "password" and any unrecognised legacy value.
        _ => HostAuthMethod::Password,
    }
}

/// Best-effort tag-list parse: the SQLite `tags` column is an opaque string
/// with no backend-enforced shape (written by the not-yet-ported host
/// editor UI). Tries a JSON string array first, falls back to a
/// comma-separated list, then to empty (never fails/panics).
fn parse_tags(raw: &str) -> Vec<String> {
    if let Ok(list) = serde_json::from_str::<Vec<String>>(raw) {
        return list;
    }
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Best-effort tunnel-list parse: same caveat as [`parse_tags`] — the SQLite
/// `tunnels` column's exact JSON shape isn't yet nailed down (T19-010's
/// job); a shape that doesn't parse cleanly into `{localPort, remoteHost,
/// remotePort}` objects is dropped (logged), not fatal, and the SQLite row
/// itself is never touched so no data is actually lost.
fn parse_tunnels(raw: &str) -> Vec<HostTunnel> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RawTunnel {
        local_port: u16,
        remote_host: String,
        remote_port: u16,
    }
    match serde_json::from_str::<Vec<RawTunnel>>(raw) {
        Ok(list) => list
            .into_iter()
            .map(|t| HostTunnel {
                local_port: t.local_port,
                remote_host: t.remote_host,
                remote_port: t.remote_port,
            })
            .collect(),
        Err(e) => {
            log::debug!("settings v1->v2: could not parse host tunnels ({e}), dropping");
            Vec::new()
        }
    }
}

/// Pure host -> `HostEntry` transform (non-secret fields only). `credential_ref`
/// is resolved by the caller (needs the secret store) and passed in.
fn host_to_entry(host: &Host, credential_ref: Option<String>) -> HostEntry {
    HostEntry {
        id: host.id.clone(),
        name: host.name.clone(),
        address: host.host_address.clone(),
        port: host.port.clamp(0, u16::MAX as i64) as u16,
        user: host.username.clone(),
        auth_method: map_auth_method(&host.auth_method),
        jump_host_ref: host.jump_host_id.clone(),
        tunnels: host
            .tunnels
            .as_deref()
            .map(parse_tunnels)
            .unwrap_or_default(),
        last_connected_at: host.last_connected_at,
        group: host.group_id.clone(),
        tags: host.tags.as_deref().map(parse_tags).unwrap_or_default(),
        credential_ref,
    }
}

/// The opaque `credential_ref` string a migrated host entry gets when its
/// secret already lives in the secret store (`backend::modules::secrets`,
/// this codebase's local-file/keychain-equivalent credential store — see
/// that module's doc comment). No secret is copied or moved; this is purely
/// a reference alongside the existing `service::account` key so a future
/// reader (T19-010) knows where to look it up again.
fn credential_ref_for(app: &crate::App, host: &Host) -> Option<String> {
    if get_password(app, &app.secrets, "labonair-app", &host.id)
        .ok()
        .flatten()
        .is_some()
    {
        return Some(format!("secrets:labonair-app:{}", host.id));
    }
    if let Some(cred_id) = &host.credential_id {
        if get_password(app, &app.secrets, "labonair-cred", cred_id)
            .ok()
            .flatten()
            .is_some()
        {
            return Some(format!("secrets:labonair-cred:{cred_id}"));
        }
    }
    None
}

/// One-time, idempotent hydration of the SQLite-backed hosts
/// (`backend::modules::hosts`) into `hosts.entries` (non-secret fields) —
/// secrets stay exactly where they already are (the secret store), only a
/// `credential_ref` is added. The SQLite table itself is never modified or
/// deleted (Warnung: "SQLite-Tabelle nicht löschen").
pub fn migrate_hosts_to_settings(
    dir: &Path,
    hosts: &[Host],
    app: &crate::App,
) -> Result<HostsV2Outcome, String> {
    let mut settings = read_settings_from(dir);

    if settings.get(KEY_HOSTS_MIGRATED).and_then(Value::as_bool) == Some(true) {
        return Ok(HostsV2Outcome::AlreadyMigrated);
    }

    if hosts.is_empty() {
        settings.insert(KEY_HOSTS_MIGRATED.to_string(), Value::Bool(true));
        write_settings_to(dir, &settings)?;
        return Ok(HostsV2Outcome::NothingToMigrate);
    }

    let path = dir.join(CONFIG_FILE);
    if path.exists() {
        let _ = std::fs::copy(&path, path.with_extension("json.bak"));
    }

    let entries: Vec<HostEntry> = hosts
        .iter()
        .map(|h| host_to_entry(h, credential_ref_for(app, h)))
        .collect();
    let migrated = entries.len();

    let mut hosts_patch = Map::new();
    hosts_patch.insert(
        "entries".to_string(),
        serde_json::to_value(&entries).map_err(|e| e.to_string())?,
    );
    merge_object(&mut settings, "hosts", hosts_patch);
    settings.insert(KEY_HOSTS_MIGRATED.to_string(), Value::Bool(true));

    write_settings_to(dir, &settings)?;

    log::info!("migrated {migrated} SQLite host(s) into settings.json hosts.entries");

    Ok(HostsV2Outcome::Migrated { migrated })
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
        // Non-default so it survives sparsification into `hosts.layout`.
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
            "notifyOnErrors",
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
            "appCornerRadius",
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
        assert_eq!(after["hosts"]["layout"], Value::from("list"));

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

    #[test]
    fn host_migration_writes_entries_without_secrets_and_is_idempotent() {
        let dir = tmp("hosts");
        let app = crate::App::new(&dir.join("appdata")).unwrap();

        let host = Host {
            id: "h1".to_string(),
            name: "prod".to_string(),
            host_address: "prod.example.com".to_string(),
            port: 22,
            username: "deploy".to_string(),
            auth_method: "key".to_string(),
            private_key_path: None,
            group_id: None,
            tags: Some("[\"prod\",\"web\"]".to_string()),
            created_at: 0,
            last_connected_at: Some(123),
            default_path_ssh: None,
            default_path_sftp: None,
            pin_to_top: false,
            sudo_password_set: false,
            keep_alive_interval: None,
            keep_alive_tries: None,
            sort_order: 0,
            tunnels: None,
            startup_snippet_id: None,
            startup_snippet_mode: None,
            credential_id: None,
            jump_host_id: None,
            notes: None,
            icon: None,
            block_agent_access: false,
        };
        crate::modules::secrets::store_password(&app, &app.secrets, "labonair-app", "h1", "s3cr3t")
            .unwrap();

        let outcome = migrate_hosts_to_settings(&dir, std::slice::from_ref(&host), &app).unwrap();
        assert_eq!(outcome, HostsV2Outcome::Migrated { migrated: 1 });

        let after = read_settings_from(&dir);
        let entries = after["hosts"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["id"], Value::from("h1"));
        assert_eq!(entries[0]["authMethod"], Value::from("publicKey"));
        assert_eq!(
            entries[0]["credentialRef"],
            Value::from("secrets:labonair-app:h1")
        );
        assert_eq!(entries[0]["tags"], serde_json::json!(["prod", "web"]));

        let json_str = serde_json::to_string(&after["hosts"]).unwrap();
        for forbidden in ["s3cr3t", "password", "privateKey"] {
            assert!(
                !json_str.to_lowercase().contains(&forbidden.to_lowercase()),
                "hosts JSON must never contain {forbidden}"
            );
        }

        let second = migrate_hosts_to_settings(&dir, std::slice::from_ref(&host), &app).unwrap();
        assert_eq!(second, HostsV2Outcome::AlreadyMigrated);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_hosts_marks_migrated_without_writing_entries() {
        let dir = tmp("no-hosts");
        let app = crate::App::new(&dir.join("appdata")).unwrap();

        let outcome = migrate_hosts_to_settings(&dir, &[], &app).unwrap();
        assert_eq!(outcome, HostsV2Outcome::NothingToMigrate);

        let after = read_settings_from(&dir);
        assert_eq!(after.get(KEY_HOSTS_MIGRATED).unwrap(), &Value::Bool(true));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
