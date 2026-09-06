//! Field / category / keybind / theme-file unit tests, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007. Ported off the retired
//! `PreferencesStore` onto the layered `labonair_settings::SettingsStore`.

#[cfg(test)]
mod cases {
    use gpui::TestAppContext;
    use serde_json::Value;

    use labonair_command_palette::{KeybindMap, ShortcutId};
    use labonair_settings::{
        EditorSettings, SettingsContent, SettingsLayer, SettingsStore, TerminalSettings,
    };
    use labonair_settings_content::areas::AREAS;

    use crate::apply::*;
    use crate::schema::{all_fields, FieldControl};

    /// A `SettingsStore` global rooted at a throwaway temp path, with the
    /// feature slices this crate reads registered.
    fn install_store(cx: &mut gpui::App, content: SettingsContent) {
        let path = std::env::temp_dir().join(format!(
            "labonair-settings-ui-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        let mut store = SettingsStore::new(path);
        store.register_setting::<TerminalSettings>();
        store.register_setting::<EditorSettings>();
        store.register_setting::<labonair_settings::ThemeSettings>();
        store.set_layer(SettingsLayer::User, content);
        cx.set_global(store);
    }

    /// T19-004: every generated field's `json_path` area segment must be one
    /// of `AREAS`' `target_module`s.
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

    #[gpui::test]
    fn font_overrides_snapshot_reads_settings(cx: &mut TestAppContext) {
        let mut content = SettingsContent::default();
        content.terminal.terminal_font_size = Some(18);
        content.editor.editor_font_family = Some("Iosevka".to_string());
        cx.update(|cx| {
            install_store(cx, content);
            let o = font_overrides_from_settings(cx);
            assert_eq!(o.terminal_size, 18.0);
            assert_eq!(o.editor_family, "Iosevka");
        });
    }

    #[test]
    fn settings_store_write_persists_and_reloads() {
        let path = std::env::temp_dir().join(format!(
            "labonair-settings-ui-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        let mut store = SettingsStore::new(path.clone());
        store.register_setting::<TerminalSettings>();
        store
            .update_user_settings(|c| c.terminal.terminal_font_size = Some(19))
            .unwrap();
        assert_eq!(store.merged().terminal.terminal_font_size, Some(19));
        // Idempotent write is a no-op.
        store
            .update_user_settings(|c| c.terminal.terminal_font_size = Some(19))
            .unwrap();

        // A fresh store rooted at the same path reads the value back off disk.
        let mut fresh = SettingsStore::new(path.clone());
        fresh.reload_user_layer();
        assert_eq!(fresh.merged().terminal.terminal_font_size, Some(19));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn app_font_family_write_persists() {
        let path = std::env::temp_dir().join(format!(
            "labonair-settings-ui-test-{}.json",
            uuid::Uuid::new_v4()
        ));
        let mut store = SettingsStore::new(path.clone());
        store
            .update_user_settings(|c| c.appearance.app_font_family = Some("Inter".into()))
            .unwrap();
        let mut fresh = SettingsStore::new(path.clone());
        fresh.reload_user_layer();
        assert_eq!(
            fresh.merged().appearance.app_font_family.as_deref(),
            Some("Inter")
        );
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn wrong_typed_field_write_is_rejected() {
        let mut content = SettingsContent::defaults();
        let field = all_fields()
            .into_iter()
            .find(|f| f.json_path == "terminal.terminalFontSize")
            .unwrap();
        assert!(!(field.set)(&mut content, Value::String("huge".into())));
        assert_eq!(content.terminal.terminal_font_size, Some(15));
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
        std::fs::write(dir.join("default.json"), SAMPLE_THEME).unwrap();

        let list = scan_themes(&dir);
        assert_eq!(list[0].id, "default");
        assert!(list[0].builtin);
        let ids: Vec<&str> = list.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["default", "good/dark", "good/light"]);
        assert_eq!(list[1].name, "Sample \u{2014} dark");
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

    #[test]
    fn capture_free_binding_sets_override() {
        assert!(matches!(
            capture_keybind(&KeybindMap::new(), ShortcutId::TabNew, "cmd-shift-y"),
            KbCapture::Set
        ));
    }

    #[test]
    fn capture_detects_conflict() {
        match capture_keybind(&KeybindMap::new(), ShortcutId::CommandPalette, "cmd-t") {
            KbCapture::Conflict(other) => assert_eq!(other, ShortcutId::TabNew),
            _ => panic!("cmd-t should collide with TabNew"),
        }
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
}
