//! Pure helpers shared by the panes: keybind-capture resolution, the
//! preferences -> `ThemeStore` bridge (`apply_prefs_to_theme`), and the user
//! theme-file scan / read / write / delete primitives. Split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move — no logic
//! change).

use std::fs;
use std::path::{Path, PathBuf};

use gpui::{App, Entity};
use serde_json::Value;

use labonair_backend::modules::fs::paths::config_dir;
use labonair_backend::modules::settings::preferences::Preferences;
use labonair_command_palette::{resolve_conflict, Conflict, KeybindMap, ShortcutId};
#[cfg(test)]
use labonair_theme::ThemeFile;
use labonair_theme::{IconThemeRegistry, ThemeRegistry, ThemeStore};

use crate::store::PreferencesStore;
use crate::view::ThemeEntry;

// ─────────────────────────── keybind mutation (pure) ─────────────────────────

/// Result of capturing a keystroke for a shortcut.
pub(crate) enum KbCapture {
    /// The keystroke is free to bind.
    Set,
    /// The keystroke is already used by another shortcut — needs a decision.
    Conflict(ShortcutId),
    /// The keystroke is an OS/menu-reserved accelerator — refused.
    Reserved(&'static str),
}

/// Pure port of `useKeybindsStore.setKeybind` + conflict detection: decide
/// what capturing `binding` for `id` means, given the current effective-
/// binding display `map` (T19-008: the actual persistence target is now
/// `keymap.json` via `crate::keymap_edit`, not this map — `map` here is only
/// used to detect a conflict against the other shortcuts' current bindings).
pub(crate) fn capture_keybind(map: &KeybindMap, id: ShortcutId, binding: &str) -> KbCapture {
    match resolve_conflict(binding, Some(id), map) {
        Some(Conflict::Reserved(label)) => KbCapture::Reserved(label),
        Some(Conflict::Shortcut(other)) => KbCapture::Conflict(other),
        None => KbCapture::Set,
    }
}

/// Build the [`FontOverrides`] snapshot from the typography-relevant
/// preferences. A blank family / zero size means "keep the theme default".
pub(crate) fn font_overrides_from(p: &Preferences) -> labonair_theme::FontOverrides {
    labonair_theme::FontOverrides {
        app_family: p.app_font_family.clone(),
        app_size: p.app_font_size as f32,
        editor_family: p.editor_font_family.clone(),
        editor_size: p.editor_font_size as f32,
        terminal_family: p.terminal_font_family.clone(),
        terminal_size: p.terminal_font_size as f32,
        terminal_line_height: 0.0,
    }
}

/// Push the font + editor-syntax-theme preferences into the [`ThemeStore`], and
/// (re)load + activate the persisted registry theme (T20-005). Used at startup
/// (`AppShell`) and on every settings change.
pub fn apply_prefs_to_theme(p: &Preferences, theme: &Entity<ThemeStore>, cx: &mut App) {
    let overrides = font_overrides_from(p);
    theme.update(cx, |t, cx| t.set_font_overrides(overrides, cx));
    if let Some(id) = labonair_theme::EditorThemeId::from_slug(&p.editor_theme) {
        theme.update(cx, |t, cx| t.set_editor_theme(id, cx));
    }

    // Rescan the user themes directory into the registry, then activate the
    // persisted id (`""` / `"default"` → built-in light/dark).
    theme.update(cx, |t, cx| {
        t.reload_user_themes(&themes_dir(), cx);
    });
    let id = if p.app_theme.is_empty() {
        "default"
    } else {
        p.app_theme.as_str()
    };
    if theme
        .update(cx, |t, cx| t.set_active_theme(id, cx))
        .is_err()
    {
        // Stale id (e.g. a deleted theme) — fall back to the built-in.
        theme.update(cx, |t, cx| {
            let _ = t.set_active_theme("default", cx);
        });
    }
    apply_stored_theme_variant(&p.theme_variant_overrides, theme, cx);

    // Icon theme (T20-006): rescan the user icon-themes directory, then
    // activate the persisted id (`""` / `"default"` → built-in glyph set).
    theme.update(cx, |t, cx| {
        t.reload_user_icon_themes(&icon_themes_dir(), cx);
    });
    let icon_id = if p.icon_theme.is_empty() {
        "default"
    } else {
        p.icon_theme.as_str()
    };
    if theme
        .update(cx, |t, cx| t.set_active_icon_theme(icon_id, cx))
        .is_err()
    {
        theme.update(cx, |t, cx| {
            let _ = t.set_active_icon_theme("default", cx);
        });
    }
}

/// Rescan the user themes directory into the live [`ThemeStore`] registry and
/// re-resolve the active theme (+ its persisted variant). Called on startup by
/// [`apply_prefs_to_theme`] and by `labonair-shell`'s fs-watch on the themes
/// folder (T20-005 live-reload).
pub fn reload_theme_registry(
    prefs: &Entity<PreferencesStore>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) {
    theme.update(cx, |t, cx| {
        t.reload_user_themes(&themes_dir(), cx);
    });
    let overrides = prefs.read(cx).get().theme_variant_overrides.clone();
    apply_stored_theme_variant(&overrides, theme, cx);
}

/// The user themes directory (`<config_dir>/labonair/themes`).
pub fn user_themes_dir() -> PathBuf {
    themes_dir()
}

/// The user icon-themes directory (`<config_dir>/labonair/icon_themes`).
pub fn user_icon_themes_dir() -> PathBuf {
    icon_themes_dir()
}

pub(crate) fn icon_themes_dir() -> PathBuf {
    config_dir().join("icon_themes")
}

/// Rescan the user icon-themes directory into the live [`ThemeStore`] registry
/// and re-resolve the active icon theme. Called on startup by
/// [`apply_prefs_to_theme`] and by `labonair-shell`'s fs-watch on the folder
/// (T20-006 live-reload).
pub fn reload_icon_theme_registry(theme: &Entity<ThemeStore>, cx: &mut App) {
    theme.update(cx, |t, cx| {
        t.reload_user_icon_themes(&icon_themes_dir(), cx);
    });
}

/// `(id, display name)` for every installed icon theme (built-in `"default"`
/// first).
pub fn icon_theme_choices() -> Vec<(String, String)> {
    let mut reg = IconThemeRegistry::builtin();
    reg.load_user_icon_themes(&icon_themes_dir());
    reg.list().into_iter().map(|m| (m.id, m.name)).collect()
}

/// Re-apply the persisted `themeVariantOverrides[family][mode]` selection to the
/// active registry family for the currently-resolved appearance.
pub(crate) fn apply_stored_theme_variant(
    overrides: &std::collections::BTreeMap<String, serde_json::Value>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) {
    let (family, appearance) = {
        let t = theme.read(cx);
        let Some(family) = t.registry().family_of(&t.active_theme_id()) else {
            return;
        };
        let appearance = match t.mode() {
            labonair_theme::ThemeMode::Dark => "dark",
            labonair_theme::ThemeMode::Light => "light",
        };
        (family, appearance)
    };
    let key = overrides
        .get(&family)
        .and_then(|v| v.get(appearance))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    theme.update(cx, |t, cx| t.set_registry_variant(key, cx));
}

pub(crate) fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

/// `(id, display name)` for every installed theme (built-in `"default"` first),
/// for the command palette's "Change App Theme…" sub-page.
pub fn theme_choices() -> Vec<(String, String)> {
    scan_themes(&themes_dir())
        .into_iter()
        .map(|e| (e.id, e.name))
        .collect()
}

/// Live hover-preview of a theme by id (`Some`) or revert (`None`) — no
/// persistence. Used by the command palette's Themes sub-page.
pub fn preview_app_theme(
    id: Option<&str>,
    prefs: &Entity<PreferencesStore>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) {
    let _ = prefs;
    match id {
        None => theme.update(cx, |t, cx| t.cancel_preview(cx)),
        Some(id) => theme.update(cx, |t, cx| t.preview_registry_theme(id, cx)),
    }
}

/// Activate a JSON app theme by id (`"default"` = built-in), persist the
/// selection, and re-apply its stored variant. Used by the palette.
pub fn activate_app_theme(
    id: &str,
    prefs: &Entity<PreferencesStore>,
    theme: &Entity<ThemeStore>,
    cx: &mut App,
) {
    prefs.update(cx, |s, cx| {
        s.set_value("appTheme", Value::String(id.to_string()), cx)
    });
    let p = prefs.read(cx).get().clone();
    apply_prefs_to_theme(&p, theme, cx);
}

/// The selectable theme list (T20-005): the built-in "Labonair" entry (id
/// `"default"`, follows the system light/dark preference) first, then one entry
/// per user registry variant — `id = "<file stem>/<variant name>"`,
/// `name = "<family> — <variant>"`.
pub(crate) fn scan_themes(dir: &Path) -> Vec<ThemeEntry> {
    let mut reg = ThemeRegistry::builtin();
    reg.load_user_themes(dir);
    let mut entries = vec![ThemeEntry {
        id: "default".to_string(),
        name: "Labonair".to_string(),
        builtin: true,
    }];
    for meta in reg.list().into_iter().filter(|m| !m.builtin) {
        entries.push(ThemeEntry {
            id: meta.id(),
            name: format!("{} \u{2014} {}", meta.family, meta.variant_name),
            builtin: false,
        });
    }
    entries
}

#[cfg(test)]
pub(crate) fn read_theme_file_in(dir: &Path, id: &str) -> Result<ThemeFile, String> {
    let raw = fs::read_to_string(dir.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
    ThemeFile::from_json(&raw)
}

pub(crate) fn save_theme_file_in(dir: &Path, id: &str, raw: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{id}.json")), raw).map_err(|e| e.to_string())
}

pub(crate) fn delete_theme_in(dir: &Path, id: &str) -> Result<(), String> {
    if id == "default" {
        return Err("the built-in theme cannot be deleted".to_string());
    }
    fs::remove_file(dir.join(format!("{id}.json"))).map_err(|e| e.to_string())
}

pub(crate) fn slugify(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() {
        "theme".to_string()
    } else {
        s
    }
}

pub(crate) fn char_of(ks: &gpui::Keystroke) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| {
            (ks.key.chars().count() == 1 && !ks.key.chars().any(|c| c.is_control()))
                .then(|| ks.key.clone())
        })
}
