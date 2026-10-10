//! Composition adapters for the small imperative services used by Settings UI.
//!
//! Settings UI owns value rendering and persistence. The composition layer
//! supplies platform discovery services without exposing application state.

use std::sync::Arc;

use gpui::{AnyWindowHandle, Entity};
use labonair_command_palette::Page as PalettePage;
use labonair_settings::SettingsSurfaceRegistry;
use labonair_settings_ui::{
    ServiceFuture, SettingsFileTarget, SettingsServices, SystemFontService,
};

use crate::app_shell::AppShell;

struct ThemeSystemFontService;

impl SystemFontService for ThemeSystemFontService {
    fn list(&self) -> ServiceFuture<Vec<String>> {
        Box::pin(labonair_theme::list_system_fonts())
    }
}

pub(crate) fn settings_services(
    shell: Entity<AppShell>,
    main_window: AnyWindowHandle,
) -> SettingsServices {
    let mut surfaces = SettingsSurfaceRegistry::default();
    log_surface_registration(
        "theme",
        labonair_theme_ui::register_settings_surfaces(
            &mut surfaces,
            surface_opener(
                "theme picker",
                shell.clone(),
                main_window,
                |shell, window, cx| {
                    shell.show_command_palette(Some(PalettePage::Themes), window, cx)
                },
            ),
            surface_opener(
                "icon theme picker",
                shell.clone(),
                main_window,
                |shell, window, cx| {
                    shell.show_command_palette(Some(PalettePage::IconThemes), window, cx)
                },
            ),
        ),
    );
    log_surface_registration(
        "background",
        labonair_background::register_settings_surfaces(
            &mut surfaces,
            surface_opener(
                "background editor",
                shell.clone(),
                main_window,
                |shell, window, cx| shell.show_background_settings(window, cx),
            ),
        ),
    );
    log_surface_registration(
        "keymap",
        labonair_keymap_ui::register_settings_surfaces(
            &mut surfaces,
            surface_opener(
                "keymap editor",
                shell.clone(),
                main_window,
                |shell, window, cx| {
                    let descriptors = shell.command_registry.descriptors();
                    shell.workspace.update(cx, |workspace, cx| {
                        workspace.open_keymap_tab(descriptors, window, cx);
                    });
                },
            ),
        ),
    );

    let file_shell = shell.clone();
    let file_window = main_window;
    SettingsServices::new(
        Arc::new(ThemeSystemFontService),
        move |target, _settings_window, cx| {
            let result = file_window.update(cx, |_, main_window, cx| {
                file_shell.update(cx, |shell, cx| {
                    let opened = shell.workspace.update(cx, |workspace, cx| match target {
                        SettingsFileTarget::User => {
                            workspace.open_or_create_user_settings_json(main_window, cx);
                            true
                        }
                        SettingsFileTarget::Project => {
                            workspace.open_or_create_project_settings(main_window, cx)
                        }
                    });
                    if opened {
                        main_window.activate_window();
                    }
                })
            });
            if let Err(error) = result {
                tracing::warn!("could not open Settings JSON in the main workspace: {error}");
            }
        },
        surfaces,
    )
}

fn surface_opener(
    owner_action: &'static str,
    shell: Entity<AppShell>,
    main_window: AnyWindowHandle,
    open: impl Fn(&mut AppShell, &mut gpui::Window, &mut gpui::Context<AppShell>) + 'static,
) -> impl Fn(&mut gpui::App) + 'static {
    move |cx| {
        let result = main_window.update(cx, |_, window, cx| {
            shell.update(cx, |shell, cx| {
                open(shell, window, cx);
                window.activate_window();
            })
        });
        if let Err(error) = result {
            tracing::warn!("could not open Settings owner action `{owner_action}`: {error}");
        }
    }
}

fn log_surface_registration(owner: &'static str, result: Result<(), String>) {
    if let Err(error) = result {
        tracing::error!("could not register {owner} Settings surfaces: {error}");
    }
}
