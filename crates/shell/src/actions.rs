//! Shell-side glue that the [`CommandRegistry`](crate::commands) and the
//! overlay layers reach through: the palette event handler, the
//! modal-layer mirrors, the shared `AppShell` helper methods that command
//! closures call, and the last three genuine window actions.
//!
//! T17-007 removed the ~50-entry `.on_action(cx.listener(Self::act_*))` chain.
//! Owner modules now contribute executable handlers through the command
//! runtime; remaining shell adapters are dispatched by
//! [`AppShell::dispatch_command`](crate::commands). Only
//! `Minimize` / `Zoom` / `Toggle Full Screen` stay as real window
//! `.on_action`s on the shell root (they only touch `Window`).

use gpui::{App, Context, Window};
use labonair_command_palette::{
    Command as PaletteCommand, Page as PalettePage, PaletteData, PaletteEvent, PaletteWorkspace,
};
use labonair_command_palette_core::{CommandSubmenu, SubmenuRegistry, SubmenuSnapshot};
use labonair_settings::{EditorSettings, GeneralSettings, Settings as _, ThemeSettings};

use labonair_workspace::search_overlay::SearchOverlay;

use crate::app_shell::AppShell;
use crate::menu;
use crate::modals::{CommandPaletteModal, UpdaterModal};

fn register_snapshot(registry: &mut SubmenuRegistry, snapshot: SubmenuSnapshot) {
    registry
        .register(snapshot)
        .expect("built-in submenu ids must be unique");
}

impl AppShell {
    // ── Genuine window actions (kept on the shell root) ────────────────────

    pub(crate) fn act_toggle_fullscreen(
        &mut self,
        _: &menu::ToggleFullScreen,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.toggle_fullscreen();
    }

    pub(crate) fn act_minimize(
        &mut self,
        _: &menu::Minimize,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.minimize_window();
    }

    pub(crate) fn act_zoom_window(
        &mut self,
        _: &menu::ZoomWindow,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.zoom_window();
    }

    // ── Helper methods the command closures call ──────────────────────────

    /// Toggle the command palette through the modal layer (T17-005).
    pub(crate) fn toggle_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .modal_layer
            .read(cx)
            .active_modal::<CommandPaletteModal>()
            .is_some()
        {
            self.modal_layer.update(cx, |layer, cx| {
                layer.hide_modal(window, cx);
            });
        } else {
            self.show_command_palette(None, window, cx);
        }
    }

    /// Open the command palette as a modal, optionally navigated to `page`.
    pub(crate) fn show_command_palette(
        &mut self,
        page: Option<PalettePage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Panel-sourced dynamic choices (snippets / AI sessions / branches /
        // hosts / symbols / app themes) are snapshotted at open time; every
        // pref/theme-derived value the palette needs it now reads itself
        // through `PalettePrefs` (T17-007 — no per-frame `build_palette_data`).
        let data = self.build_palette_data(cx);
        let palette = self.panels.command_palette.clone();
        palette.update(cx, |p, _| p.set_data(data));
        self.modal_layer.update(cx, |layer, cx| {
            layer.open_modal(window, cx, move |window, cx| {
                palette.update(cx, |p, cx| match page {
                    Some(page) => p.open_to_page(page, window, cx),
                    None => p.open(window, cx),
                });
                CommandPaletteModal::new(palette.clone(), cx)
            });
        });
    }

    /// `Cmd+F` — toggle the workspace's transient search overlay (T18-002).
    pub(crate) fn toggle_search_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let workspace = self.workspace.clone();
        let theme = self.theme.clone();
        self.modal_layer.update(cx, |layer, cx| {
            layer.toggle_modal::<SearchOverlay, _>(window, cx, move |window, cx| {
                SearchOverlay::new(workspace, theme, window, cx)
            });
        });
    }

    // ── Modal-layer mirrors (driven from `render`) ────────────────────────

    /// Mirror the updater dialog's visibility into the modal layer. Driven from
    /// `render` since `dialog_open` is flipped by the async update check.
    pub(crate) fn sync_updater_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let want = self.panels.updater.read(cx).dialog_visible();
        let have = self
            .modal_layer
            .read(cx)
            .active_modal::<UpdaterModal>()
            .is_some();
        if want == have {
            return;
        }
        let updater = self.panels.updater.clone();
        self.modal_layer.update(cx, |layer, cx| {
            if want {
                layer.open_modal(window, cx, move |_, cx| UpdaterModal::new(updater, cx));
            } else {
                layer.hide_modal(window, cx);
            }
        });
    }

    // ── Palette event handler ────────────────────────────────────────────

    /// Compose immutable dynamic submenu snapshots from the owning
    /// capabilities. The palette receives this registry and only renders or
    /// forwards its typed actions; it does not read feature entities.
    fn build_palette_data(&self, cx: &App) -> PaletteData {
        let mut submenus = SubmenuRegistry::default();

        register_snapshot(
            &mut submenus,
            labonair_theme::command_provider::color_mode_submenu(
                GeneralSettings::try_get(cx)
                    .map(|settings| match settings.theme_pref() {
                        labonair_settings::content::general::ThemePref::Light => {
                            labonair_theme::ThemePreference::Light
                        }
                        labonair_settings::content::general::ThemePref::Dark => {
                            labonair_theme::ThemePreference::Dark
                        }
                        labonair_settings::content::general::ThemePref::System => {
                            labonair_theme::ThemePreference::System
                        }
                    })
                    .unwrap_or(labonair_theme::ThemePreference::System),
            ),
        );

        let tabs = labonair_workspace::command_provider::tabs_submenu(
            self.workspace
                .read(cx)
                .palette_tab_rows(cx)
                .into_iter()
                .map(|tab| (tab.id, tab.label, tab.kind_title)),
        );
        register_snapshot(&mut submenus, tabs);

        let hosts = labonair_hosts::command_provider::host_submenu(
            "hosts",
            "Hosts",
            CommandSubmenu::Hosts,
            self.workspace.read(cx).host_picker_rows(cx),
        );
        register_snapshot(&mut submenus, hosts);

        let recent_hosts = labonair_hosts::command_provider::host_submenu(
            "recent-hosts",
            "Recent Hosts",
            CommandSubmenu::RecentHosts,
            self.workspace.read(cx).recent_host_picker_rows(cx, 5),
        );
        register_snapshot(&mut submenus, recent_hosts);

        let app_theme = ThemeSettings::try_get(cx)
            .map(|s| s.app_theme().to_string())
            .unwrap_or_else(|| "default".to_string());
        let active_theme_id = if app_theme.is_empty() {
            "default"
        } else {
            app_theme.as_str()
        };
        let active_editor_theme = EditorSettings::try_get(cx)
            .and_then(|settings| labonair_theme::EditorThemeId::from_slug(settings.editor_theme()))
            .unwrap_or_default();

        let snippets = labonair_snippets::command_provider::snippets_submenu(
            self.panels.snippets.read(cx).snippet_choices(),
        );
        register_snapshot(&mut submenus, snippets);

        let git_branches = labonair_git::command_provider::branches_submenu(
            self.panels.git_panel.read(cx).branch_choices(),
        );
        register_snapshot(&mut submenus, git_branches);

        let symbols = labonair_editor::command_provider::outline_submenu(
            self.workspace
                .read(cx)
                .active_editor_symbols(cx)
                .into_iter()
                .map(|s| {
                    (
                        s.line,
                        s.name,
                        format!("{}  ·  line {}", s.kind.label(), s.line + 1),
                    )
                }),
        );
        register_snapshot(&mut submenus, symbols);

        let app_themes = labonair_theme::command_provider::app_themes_submenu(
            labonair_theme::command_provider::app_theme_choices()
                .into_iter()
                .map(|(id, name)| (id.clone(), name, id == active_theme_id)),
        );
        register_snapshot(&mut submenus, app_themes);

        let active_icon_theme_id = ThemeSettings::try_get(cx)
            .map(|s| s.icon_theme().to_string())
            .unwrap_or_else(|| "default".to_string());
        let active_icon_theme_id = if active_icon_theme_id.is_empty() {
            "default"
        } else {
            active_icon_theme_id.as_str()
        };
        let icon_themes = labonair_theme::command_provider::icon_themes_submenu(
            labonair_theme::command_provider::icon_theme_choices()
                .into_iter()
                .map(|(id, name)| {
                    (
                        id.clone(),
                        name,
                        Some("built-in".to_string()),
                        id == active_icon_theme_id,
                    )
                }),
        );
        register_snapshot(&mut submenus, icon_themes);

        let status_bar_hidden = labonair_workspace::command_provider::hidden_status_items_submenu(
            self.workspace.read(cx).hidden_status_bar_items(),
        );
        register_snapshot(&mut submenus, status_bar_hidden);

        let editor_themes = labonair_theme::command_provider::editor_themes_submenu(
            labonair_theme::EditorThemeId::ALL
                .into_iter()
                .map(|id| (id, active_editor_theme == id)),
        );
        register_snapshot(&mut submenus, editor_themes);

        PaletteData {
            commands: self
                .command_registry
                .descriptors()
                .into_iter()
                .map(PaletteCommand::from_descriptor)
                .collect(),
            submenus,
        }
    }

    /// Forward a single palette pick to the command or dynamic-action
    /// registry. Feature-specific behavior is contributed by the owning
    /// module; the shell does not interpret submenu variants.
    pub(crate) fn handle_palette_event(
        &mut self,
        event: PaletteEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PaletteEvent::Run(id) => self.dispatch_command(id, window, cx),
            PaletteEvent::Action(action) => {
                if !self
                    .command_registry
                    .dispatch_palette_action(&action, window, cx)
                {
                    tracing::warn!(?action, "unhandled dynamic command-palette action");
                }
            }
        }
    }
}
