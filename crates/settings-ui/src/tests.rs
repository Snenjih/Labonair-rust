//! Field / category / keybind / theme-file unit tests, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007 (mechanical move — no logic
//! change).

#[cfg(test)]
mod cases {
    use gpui::{AppContext, TestAppContext};
    use serde_json::Value;

    use labonair_backend::modules::settings::preferences::Preferences;
    use labonair_command_palette::{
        effective_binding, resolve_conflict, Conflict, KeybindMap, ShortcutId,
    };
    use labonair_settings_content::areas::AREAS;

    use crate::apply::*;
    use crate::schema::{all_fields, FieldControl};
    use crate::store::PreferencesStore;

    /// T19-004: every generated field's `json_path` area segment must be one
    /// of `AREAS`' `target_module`s — `schema.rs` has its own, more thorough
    /// version of this check; this one additionally proves `AREAS` and the
    /// field registry agree from the crate's public surface.
    #[test]
    fn every_field_area_matches_an_areas_target_module() {
        let modules: std::collections::HashSet<&str> =
            AREAS.iter().map(|a| a.target_module).collect();
        for f in all_fields() {
            assert!(modules.contains(f.area()), "unknown area `{}`", f.area());
        }
    }

    #[test]
    fn editor_theme_options_are_known_slugs() {
        let opts = all_fields()
            .into_iter()
            .find(|f| f.json_path == "editor.editorTheme")
            .map(|f| match f.control {
                FieldControl::Select(o) => o,
                _ => panic!("editor.editorTheme should be a Select"),
            })
            .unwrap();
        for (slug, _label) in opts {
            assert!(
                labonair_theme::EditorThemeId::from_slug(slug).is_some(),
                "unknown editor theme slug `{slug}`"
            );
        }
    }

    #[test]
    fn font_overrides_snapshot_maps_prefs() {
        let p = Preferences {
            terminal_font_size: 18,
            editor_font_family: "Iosevka".to_string(),
            ..Default::default()
        };
        let o = font_overrides_from(&p);
        assert_eq!(o.terminal_size, 18.0);
        assert_eq!(o.editor_family, "Iosevka");
    }

    #[gpui::test]
    fn set_value_persists_and_notifies(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let d2 = dir.clone();
        let store = cx.new(|_| PreferencesStore::with_dir(d2));
        let count = std::rc::Rc::new(std::cell::RefCell::new(0));
        let c2 = count.clone();
        cx.update(|cx| {
            cx.observe(&store, move |_, _| *c2.borrow_mut() += 1)
                .detach();
        });
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::from(19), cx);
        });
        cx.run_until_parked();
        assert_eq!(store.read_with(cx, |s, _| s.get().terminal_font_size), 19);
        assert_eq!(*count.borrow(), 1);
        // Persisted to disk — a fresh store reads it back.
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .terminal_font_size,
            19
        );
        // Idempotent set does not notify again.
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::from(19), cx);
        });
        cx.run_until_parked();
        assert_eq!(*count.borrow(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    const SAMPLE_THEME: &str = r##"{
        "name": "Sample",
        "variants": {
            "dark":  { "mode": "dark",  "colors": { "primary": "#ff0000" } },
            "light": { "mode": "light", "colors": { "primary": "#0000ff" } }
        }
    }"##;

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("labonair-themes-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn slugify_makes_filesystem_safe_names() {
        assert_eq!(slugify("Tokyo Night!!"), "tokyo-night");
        assert_eq!(slugify("  "), "theme");
        assert_eq!(slugify("Ayu_Mirage"), "ayu-mirage");
    }

    #[test]
    fn scan_themes_lists_valid_user_themes_and_skips_junk() {
        let dir = tmp();
        std::fs::write(dir.join("good.json"), SAMPLE_THEME).unwrap();
        std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignore me").unwrap();
        // A file literally named default.json must never shadow the built-in.
        std::fs::write(dir.join("default.json"), SAMPLE_THEME).unwrap();

        let list = scan_themes(&dir);
        assert_eq!(list[0].id, "default");
        assert!(list[0].builtin);
        let ids: Vec<&str> = list.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["default", "good"]);
        assert_eq!(list[1].name, "Sample");
        assert!(!list[1].builtin);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_read_and_delete_theme_roundtrip() {
        let dir = tmp();
        save_theme_file_in(&dir, "mine", SAMPLE_THEME).unwrap();
        let file = read_theme_file_in(&dir, "mine").unwrap();
        assert_eq!(file.name, "Sample");

        assert!(delete_theme_in(&dir, "default").is_err());
        delete_theme_in(&dir, "mine").unwrap();
        assert!(read_theme_file_in(&dir, "mine").is_err());
        assert_eq!(scan_themes(&dir).len(), 1);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[gpui::test]
    fn app_font_family_preference_persists(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        store.update(cx, |s, cx| {
            s.set_value("appFontFamily", Value::String("Inter".into()), cx);
        });
        cx.run_until_parked();
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .app_font_family,
            "Inter"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn capture_free_binding_sets_override() {
        match capture_keybind(&KeybindMap::new(), ShortcutId::TabNew, "cmd-shift-y") {
            KbCapture::Set(m) => {
                assert_eq!(m.get("tab.new").map(String::as_str), Some("cmd-shift-y"))
            }
            _ => panic!("expected a free binding"),
        }
    }

    #[test]
    fn capture_detects_conflict_then_overwrite_unbinds_loser() {
        let map = KeybindMap::new();
        match capture_keybind(&map, ShortcutId::CommandPalette, "cmd-t") {
            KbCapture::Conflict(other) => assert_eq!(other, ShortcutId::TabNew),
            _ => panic!("cmd-t should collide with TabNew"),
        }
        let next = overwrite_keybind(
            &map,
            ShortcutId::CommandPalette,
            ShortcutId::TabNew,
            "cmd-t",
        );
        assert_eq!(
            next.get("command.palette").map(String::as_str),
            Some("cmd-t")
        );
        assert_eq!(next.get("tab.new").map(String::as_str), Some(""));
        assert_eq!(effective_binding(ShortcutId::TabNew, &next), None);
        // No silent double-binding — cmd-t has exactly one owner now.
        assert_eq!(
            resolve_conflict("cmd-t", None, &next),
            Some(Conflict::Shortcut(ShortcutId::CommandPalette))
        );
    }

    #[test]
    fn capture_refuses_reserved_accelerator() {
        assert!(matches!(
            capture_keybind(&KeybindMap::new(), ShortcutId::TabNew, "cmd-,"),
            KbCapture::Reserved("Settings")
        ));
    }

    #[test]
    fn shortcuts_category_is_registered() {
        assert!(AREAS.iter().any(|a| a.key == "shortcuts"));
    }

    #[gpui::test]
    fn keybinds_persist_and_reset(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        let mut m = KeybindMap::new();
        m.insert("tab.new".into(), "cmd-shift-t".into());
        store.update(cx, |s, cx| {
            s.set_value("keybinds", serde_json::to_value(&m).unwrap(), cx);
        });
        cx.run_until_parked();
        assert_eq!(
            PreferencesStore::with_dir(dir.clone())
                .get()
                .keybinds
                .get("tab.new")
                .map(String::as_str),
            Some("cmd-shift-t")
        );
        // Reset all → empty map persists across a reload.
        store.update(cx, |s, cx| {
            s.set_value("keybinds", serde_json::json!({}), cx);
        });
        cx.run_until_parked();
        assert!(PreferencesStore::with_dir(dir.clone())
            .get()
            .keybinds
            .is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[gpui::test]
    fn bad_type_is_rejected(cx: &mut TestAppContext) {
        let dir = std::env::temp_dir().join(format!("labonair-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = cx.new(|_| PreferencesStore::with_dir(dir.clone()));
        store.update(cx, |s, cx| {
            s.set_value("terminalFontSize", Value::String("huge".into()), cx);
        });
        assert_eq!(
            store.read_with(cx, |s, _| s.get().terminal_font_size),
            Preferences::default().terminal_font_size
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
