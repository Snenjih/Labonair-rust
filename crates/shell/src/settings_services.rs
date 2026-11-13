//! Composition adapters for the small imperative services used by Settings UI.
//!
//! Settings UI owns value rendering and persistence. The composition layer
//! supplies platform discovery services without exposing application state.

use std::sync::Arc;

use gpui::{AnyWindowHandle, Entity};
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
    hosts: Entity<labonair_hosts_ui::HostManagerView>,
    cx: &gpui::App,
) -> SettingsServices {
    let mut surfaces = SettingsSurfaceRegistry::default();
    // The owner surfaces of the Themes page render inline in the Settings
    // window; their fallback entry point simply opens that page.
    let open_themes_page =
        |cx: &mut gpui::App| labonair_settings_ui::open_settings_window(Some("themes"), cx);
    log_surface_registration(
        "theme",
        labonair_theme_ui::register_settings_surfaces(
            &mut surfaces,
            open_themes_page,
            crate::theme_loader::themes_dir(),
        ),
    );
    log_surface_registration(
        "background",
        labonair_background::register_settings_surfaces(
            &mut surfaces,
            open_themes_page,
            |message, cx| {
                labonair_notifications::notification_center(cx).update(cx, |center, cx| {
                    center.push_action_result(
                        labonair_notifications::Notification::error(
                            "Background",
                            "Operation failed",
                        )
                        .details(message),
                        cx,
                    );
                });
            },
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
    log_surface_registration(
        "hosts",
        labonair_hosts_ui::register_settings_surfaces(&mut surfaces, hosts, |cx| {
            labonair_settings_ui::open_settings_window(Some("hosts"), cx)
        }),
    );

    let file_shell = shell.clone();
    let file_window = main_window;
    let services = SettingsServices::new(
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
    );
    // The background session host is unix-only (ADR 0009).
    #[cfg(unix)]
    let services = {
        use labonair_settings::{Settings as _, TerminalSettings};
        let lines = TerminalSettings::try_get(cx).map_or(5_000, |s| s.detached_scrollback_lines());
        services.with_background_host(Arc::new(
            crate::background_host_service::DaemonHostService::new(lines),
        ))
    };
    #[cfg(not(unix))]
    let _ = cx;
    services
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
