//! Field / category / keybind / theme-file unit tests, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007. Ported off the retired
//! `PreferencesStore` onto the layered `labonair_settings::SettingsStore`.

#[cfg(test)]
mod cases {
    use gpui::TestAppContext;
    use serde_json::Value;

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

    #[test]
    fn capability_management_categories_are_not_registered() {
        for removed in ["themes", "hosts", "shortcuts"] {
            assert!(!AREAS.iter().any(|area| area.key == removed));
        }
    }
}
