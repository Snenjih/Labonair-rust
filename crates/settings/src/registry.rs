//! Compile-time `Settings` registration via `inventory` (T19-002
//! Anweisung #4). `#[derive(RegisterSetting)]` (see `crates/settings-macros`)
//! emits an `inventory::submit! { RegisteredSetting { .. } }` for its type;
//! [`register_all`] walks every submitted entry and calls it once, at
//! `labonair_settings::init` time.

use gpui::App;

/// One inventory-collected `Settings::register` call, `TypeId`-erased so
/// every `#[derive(RegisterSetting)]` type across every crate can submit to
/// the same collection (`inventory::collect!` below) without this crate
/// knowing about any of them ahead of time.
pub struct RegisteredSetting {
    pub register: fn(&mut App),
}

inventory::collect!(RegisteredSetting);

/// Call every `#[derive(RegisterSetting)]` type's `Settings::register(cx)`.
/// Idempotent (registration itself dedups by `TypeId`,
/// `SettingsStore::register_setting`); safe to call more than once.
pub fn register_all(cx: &mut App) {
    for entry in inventory::iter::<RegisteredSetting> {
        (entry.register)(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concrete::{
        EditorSettings, ExplorerSettings, PersonalizationSettings, TerminalSettings, ThemeSettings,
        WorkspaceSettings,
    };
    use crate::settings_trait::Settings;
    use crate::store;
    use gpui::TestAppContext;

    #[gpui::test]
    fn register_all_registers_every_concrete_setting(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let path = std::env::temp_dir().join(format!(
                "labonair-settings-registry-{}.json",
                uuid::Uuid::new_v4()
            ));
            store::init_at(cx, path);
            register_all(cx);

            // Panics (via `SettingsStore::get`'s `unwrap_or_else`) if any of
            // these was never registered — the assertion is just "this
            // doesn't panic", plus a couple of sanity default checks.
            assert_eq!(ThemeSettings::get(cx).app_theme(), "default");
            assert_eq!(TerminalSettings::get(cx).terminal_opacity(), 100);
            let _ = EditorSettings::get(cx);
            let _ = WorkspaceSettings::get(cx);
            let _ = PersonalizationSettings::get(cx);
            assert!(ExplorerSettings::get(cx).sticky_ancestors());
        });
    }
}
