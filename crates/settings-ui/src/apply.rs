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
use labonair_command_palette::{resolve_conflict, shortcut_slug, Conflict, KeybindMap, ShortcutId};
use labonair_theme::{ThemeFile, ThemeStore};

use crate::store::PreferencesStore;
use crate::view::ThemeEntry;

// ─────────────────────────── keybind mutation (pure) ─────────────────────────

/// Result of capturing a keystroke for a shortcut.
pub(crate) enum KbCapture {
    /// The keystroke is free — here is the new override map to persist.
    Set(KeybindMap),
    /// The keystroke is already used by another shortcut — needs a decision.
    Conflict(ShortcutId),
    /// The keystroke is an OS/menu-reserved accelerator — refused.
    Reserved(&'static str),
}

/// Pure port of `useKeybindsStore.setKeybind` + conflict detection: decide
/// what capturing `binding` for `id` means, given the current `map`.
pub(crate) fn capture_keybind(map: &KeybindMap, id: ShortcutId, binding: &str) -> KbCapture {
    match resolve_conflict(binding, Some(id), map) {
        Some(Conflict::Reserved(label)) => KbCapture::Reserved(label),
        Some(Conflict::Shortcut(other)) => KbCapture::Conflict(other),
        None => {
            let mut m = map.clone();
            m.insert(shortcut_slug(id).to_string(), binding.to_string());
            KbCapture::Set(m)
        }
    }
}

/// Resolve a capture conflict by giving `binding` to `id` and unbinding the
/// previous owner — no silent double-binding.
pub(crate) fn overwrite_keybind(
    map: &KeybindMap,
    id: ShortcutId,
    other: ShortcutId,
    binding: &str,
) -> KeybindMap {
    let mut m = map.clone();
    m.insert(shortcut_slug(other).to_string(), String::new());
    m.insert(shortcut_slug(id).to_string(), binding.to_string());
    m
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

/// Push the font + editor-syntax-theme preferences into the [`ThemeStore`].
/// Used at startup (`AppShell`) and on every settings change.
pub fn apply_prefs_to_theme(p: &Preferences, theme: &Entity<ThemeStore>, cx: &mut App) {
    let overrides = font_overrides_from(p);
    theme.update(cx, |t, cx| t.set_font_overrides(overrides, cx));
    if let Some(id) = labonair_theme::EditorThemeId::from_slug(&p.editor_theme) {
        theme.update(cx, |t, cx| t.set_editor_theme(id, cx));
    }
    // Restore the active JSON app theme (+ persisted variant) on startup.
    if !p.app_theme.is_empty() && p.app_theme != "default" {
        if let Ok(file) = read_theme_file_in(&themes_dir(), &p.app_theme) {
            let dark = matches!(theme.read(cx).mode(), labonair_theme::ThemeMode::Dark);
            let key = p
                .theme_variant_overrides
                .get(&p.app_theme)
                .and_then(|v| v.get(if dark { "dark" } else { "light" }))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let _ = theme.update(cx, |t, cx| t.import_theme_file_variant(file, key, cx));
        }
    } else {
        theme.update(cx, |t, cx| t.clear_custom_theme(cx));
    }
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
    match id {
        None | Some("default") => theme.update(cx, |t, cx| {
            if id.is_none() {
                t.cancel_preview(cx);
            } else {
                t.preview_theme_file(None, None, cx);
            }
        }),
        Some(id) => {
            let Ok(file) = read_theme_file_in(&themes_dir(), id) else {
                return;
            };
            let mode = match theme.read(cx).mode() {
                labonair_theme::ThemeMode::Dark => "dark",
                labonair_theme::ThemeMode::Light => "light",
            };
            let key = prefs
                .read(cx)
                .get()
                .theme_variant_overrides
                .get(id)
                .and_then(|v| v.get(mode))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            theme.update(cx, |t, cx| {
                t.preview_theme_file(Some(&file), key.as_deref(), cx)
            });
        }
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

/// Scans `dir` for valid user theme files. The built-in "Labonair" default is
/// always the first entry; user themes follow, sorted by display name.
pub(crate) fn scan_themes(dir: &Path) -> Vec<ThemeEntry> {
    let mut entries = vec![ThemeEntry {
        id: "default".to_string(),
        name: "Labonair".to_string(),
        builtin: true,
    }];
    if let Ok(rd) = fs::read_dir(dir) {
        let mut users: Vec<ThemeEntry> = rd
            .flatten()
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .filter_map(|e| {
                let path = e.path();
                let id = path.file_stem()?.to_str()?.to_string();
                if id == "default" {
                    return None;
                }
                let file = ThemeFile::from_json(&fs::read_to_string(&path).ok()?).ok()?;
                file.validate().ok()?;
                Some(ThemeEntry {
                    id,
                    name: file.name,
                    builtin: false,
                })
            })
            .collect();
        users.sort_by_key(|a| a.name.to_lowercase());
        entries.extend(users);
    }
    entries
}

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
