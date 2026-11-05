//! Field / category / keybind / theme-file unit tests, split out of the old
//! `crates/ui/src/settings.rs` monolith in T16-007. Ported off the retired
//! `PreferencesStore` onto the layered `labonair_settings::SettingsStore`.

#[cfg(test)]
mod cases {
    use serde_json::Value;

    use labonair_settings::{SettingsContent, SettingsStore, TerminalSettings};
    use labonair_settings_content::areas::AREAS;

    use crate::schema::{all_fields, FieldControl};

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
        assert_eq!(content.terminal.terminal_font_size, Some(14));
    }

    #[test]
    fn capability_management_categories_are_not_registered() {
        for removed in ["themes", "hosts", "shortcuts"] {
            assert!(!AREAS.iter().any(|area| area.key == removed));
        }

        let fields = all_fields();
        for removed in ["appearance.appTheme", "appearance.themeVariantOverrides"] {
            assert!(
                !fields.iter().any(|field| field.json_path == removed),
                "management value `{removed}` must not be an editable Settings field"
            );
        }
    }
}

/// Layout + prepaint coverage: every page, a search and the page history build
/// and lay out (including the virtualized section cards and the inline theme
/// surfaces) without panicking.
#[cfg(test)]
mod render {
    use std::sync::Arc;

    use gpui::{point, px, size, AnyView, TestAppContext, VisualTestContext, WindowAppearance};

    use crate::services::{
        ServiceFuture, SettingsServices, SettingsSurfaceRegistry, SystemFontService,
    };
    use crate::view::SettingsView;

    struct NoFonts;

    impl SystemFontService for NoFonts {
        fn list(&self) -> ServiceFuture<Vec<String>> {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    #[gpui::test]
    fn every_page_search_and_history_render(cx: &mut TestAppContext) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio runtime");
        cx.update(|cx| {
            gpui_component::init(cx);
            labonair_theme::init_theme(WindowAppearance::Dark, cx);
            labonair_notifications::init(cx);
        });
        let mut surfaces = SettingsSurfaceRegistry::default();
        labonair_theme_ui::register_settings_surfaces(
            &mut surfaces,
            |_| {},
            std::path::PathBuf::new(),
        )
        .expect("theme surfaces register");
        let services = SettingsServices::new(Arc::new(NoFonts), |_, _, _| {}, surfaces);
        let theme = cx.update(|cx| labonair_theme::theme_store(cx));
        let handle = runtime.handle().clone();
        let (view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_window, cx| {
            let mut view = SettingsView::new(theme, services, handle, cx);
            view.windowed = true;
            view.open = true;
            view
        });
        let page_count = view.read_with(cx, |view, _| view.pages.len());
        let any: AnyView = view.clone().into();
        for page in 0..page_count {
            view.update(cx, |view, cx| view.go_to_area(page, cx));
            cx.draw(
                point(px(0.0), px(0.0)),
                size(px(900.0), px(700.0)),
                |_, _| any.clone(),
            );
        }
        // Back/forward walk the visited pages.
        view.update(cx, |view, cx| {
            assert!(view.can_go_back());
            view.step_visited(-1, cx);
            assert!(view.can_go_forward());
        });
        view.update(cx, |view, cx| {
            view.search = "font".to_owned();
            view.refresh_search_results();
            cx.notify();
        });
        cx.draw(
            point(px(0.0), px(0.0)),
            size(px(900.0), px(700.0)),
            |_, _| any.clone(),
        );
    }
}
