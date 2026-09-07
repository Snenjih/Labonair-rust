//! Shell-side glue that the [`CommandRegistry`](crate::commands) and the
//! overlay layers reach through: the palette event handler, the
//! modal-layer mirrors, the shared `AppShell` helper methods that command
//! closures call, and the last three genuine window actions.
//!
//! T17-007 removed the ~50-entry `.on_action(cx.listener(Self::act_*))` chain:
//! every former `act_*` body is now a closure in
//! [`register_builtin_commands`](crate::commands::register_builtin_commands),
//! dispatched by [`AppShell::dispatch_command`](crate::commands). Only
//! `Minimize` / `Zoom` / `Toggle Full Screen` stay as real window
//! `.on_action`s on the shell root (they only touch `Window`).

use gpui::{App, Context, Window};
use labonair_command_palette::{
    Command as PaletteCommand, Page as PalettePage, PaletteData, PaletteEvent, PaletteWorkspace,
};
use labonair_command_palette_core::{CommandSubmenu, SubmenuRegistry, SubmenuSnapshot};
use labonair_panel::DockPosition;
use labonair_settings::{
    EditorSettings, GeneralSettings, Settings as _, SettingsStore, ThemeSettings,
};

use labonair_workspace::search_overlay::SearchOverlay;

use crate::app_shell::AppShell;
use crate::menu;
use crate::modals::{CommandPaletteModal, UpdaterModal};

/// Flip one boolean leaf across the layered [`SettingsStore`], keyed by its
/// local key — the vocabulary `crate::commands::register_builtin_commands`'
/// zen-mode/settings toggle table uses. Covers exactly the settings wired
/// through [`AppShell::toggle_zen_pref`]; a key outside this list is a no-op.
fn toggle_setting_bool(key: &str, cx: &mut App) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let _ = cx
        .global_mut::<SettingsStore>()
        .update_user_settings(|c| match key {
            "zenModeShowHeader" => {
                c.appearance.zen_mode_show_header =
                    Some(!c.appearance.zen_mode_show_header.unwrap_or(true));
            }
            "zenModeShowStatusbar" => {
                c.appearance.zen_mode_show_statusbar =
                    Some(!c.appearance.zen_mode_show_statusbar.unwrap_or(true));
            }
            "editorWordWrap" => {
                c.editor.editor_word_wrap = Some(!c.editor.editor_word_wrap.unwrap_or(false));
            }
            "editorLineNumbers" => {
                c.editor.editor_line_numbers = Some(!c.editor.editor_line_numbers.unwrap_or(true));
            }
            "editorFormatOnSave" => {
                c.editor.editor_format_on_save =
                    Some(!c.editor.editor_format_on_save.unwrap_or(false));
            }
            "terminalCursorBlink" => {
                c.terminal.terminal_cursor_blink =
                    Some(!c.terminal.terminal_cursor_blink.unwrap_or(true));
            }
            "terminalShowPaneHeader" => {
                c.terminal.terminal_show_pane_header =
                    Some(!c.terminal.terminal_show_pane_header.unwrap_or(false));
            }
            "terminalShowPaneFooter" => {
                c.terminal.terminal_show_pane_footer =
                    Some(!c.terminal.terminal_show_pane_footer.unwrap_or(false));
            }
            "vimMode" => {
                c.editor.editor_vim_mode = Some(!c.editor.editor_vim_mode.unwrap_or(false));
            }
            _ => {}
        });
}

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

    /// The primary edge as a [`DockPosition`] (per `sidebarPosition`).
    pub(crate) fn primary_dock(&self, cx: &App) -> DockPosition {
        self.workspace.read(cx).primary_dock(cx)
    }

    /// `Cmd+B` — toggle the primary dock open/closed.
    pub(crate) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        let pos = self.primary_dock(cx);
        self.workspace.update(cx, |w, cx| {
            w.dock_mut(pos).toggle_open();
            w.persist_docks(cx);
        });
        cx.notify();
    }

    /// "show me X" — never closes the dock (palette / menu intent).
    pub(crate) fn open_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_panel(name, cx));
        cx.notify();
    }

    /// Move a panel to another dock (T17-002 API).
    pub(crate) fn move_panel(&mut self, name: &str, to: DockPosition, cx: &mut Context<Self>) {
        let moved = self.workspace.update(cx, |w, cx| {
            let moved = w.move_panel(name, to, cx);
            if moved {
                w.persist_docks(cx);
            }
            moved
        });
        if moved {
            cx.notify();
        }
    }

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

    /// `view.zenMode`: both bars visible → hide both, otherwise show both.
    pub(crate) fn toggle_zen_mode(&mut self, cx: &mut Context<Self>) {
        let (show_header, show_statusbar) = ThemeSettings::try_get(cx)
            .map(|s| (s.zen_mode_show_header(), s.zen_mode_show_statusbar()))
            .unwrap_or((true, true));
        let next = !(show_header || show_statusbar);
        if cx.has_global::<SettingsStore>() {
            let _ = cx.global_mut::<SettingsStore>().update_user_settings(|c| {
                c.appearance.zen_mode_show_header = Some(next);
                c.appearance.zen_mode_show_statusbar = Some(next);
            });
        }
    }

    pub(crate) fn toggle_zen_pref(&mut self, key: &str, cx: &mut Context<Self>) {
        toggle_setting_bool(key, cx);
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
            labonair_workspace::command_provider::zoom_submenu(),
        );
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

    /// Service a single palette pick straight from the `PaletteEvent`
    /// subscription (T17-005 — no `pending_commands` buffer / `drain`).
    pub(crate) fn handle_palette_event(
        &mut self,
        event: PaletteEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PaletteEvent::SwitchToTab(id) => {
                self.workspace
                    .update(cx, |w, cx| w.reveal_tab(id, window, cx));
            }
            // Runnable commands go through the shared registry (T17-007).
            PaletteEvent::Run(id) => self.dispatch_command(id, window, cx),
            PaletteEvent::ConnectHost { host_id, sftp } => {
                let request = if sftp {
                    labonair_hosts::HostOpenRequest::sftp(host_id)
                } else {
                    labonair_hosts::HostOpenRequest::ssh(host_id)
                };
                self.workspace.update(cx, |w, cx| {
                    w.open_host_request(request, window, cx);
                });
            }
            PaletteEvent::SetAppTheme(id) => {
                labonair_settings_ui::activate_app_theme(&id, &self.theme, cx);
            }
            PaletteEvent::SetIconTheme(id) => {
                if cx.has_global::<SettingsStore>() {
                    let persisted_id = id.clone();
                    let _ = cx
                        .global_mut::<SettingsStore>()
                        .update_user_settings(move |c| {
                            c.appearance.icon_theme = Some(persisted_id);
                        });
                }
                let _ = self
                    .theme
                    .update(cx, |theme, cx| theme.set_active_icon_theme(id, cx));
            }
            PaletteEvent::PreviewAppTheme(id) => {
                labonair_settings_ui::preview_app_theme(id.as_deref(), &self.theme, cx);
            }
            PaletteEvent::PreviewIconTheme(id) => {
                let _ = self
                    .theme
                    .update(cx, |theme, cx| theme.preview_icon_theme(id.as_deref(), cx));
            }
            PaletteEvent::RunSnippet(id) => {
                self.panels
                    .snippets
                    .update(cx, |s, cx| s.run_by_id(&id, window, cx));
            }
            PaletteEvent::SwitchBranch(name) => {
                self.panels
                    .git_panel
                    .update(cx, |g, cx| g.checkout(name, cx));
            }
            PaletteEvent::GoToLine(line) => {
                self.workspace
                    .update(cx, |w, cx| w.active_editor_goto_line(line, cx));
            }
            PaletteEvent::ShowStatusBarItem(id) => {
                self.workspace.update(cx, |w, cx| {
                    if let Some(sid) = w.status_item_registry().get(&id).map(|r| r.id) {
                        w.set_status_bar_placement(sid, None, Some(false), cx);
                    }
                });
            }
            PaletteEvent::SetColorMode(pref) => {
                use labonair_settings::content::general::ThemePref;
                let value = match pref {
                    crate::theme::ThemePreference::System => ThemePref::System,
                    crate::theme::ThemePreference::Light => ThemePref::Light,
                    crate::theme::ThemePreference::Dark => ThemePref::Dark,
                };
                if cx.has_global::<SettingsStore>() {
                    let _ = cx
                        .global_mut::<SettingsStore>()
                        .update_user_settings(|c| c.general.theme = Some(value));
                }
                labonair_settings_ui::apply_prefs_to_theme(&self.theme, cx);
            }
            PaletteEvent::SetEditorTheme(id) => {
                if cx.has_global::<SettingsStore>() {
                    let _ = cx.global_mut::<SettingsStore>().update_user_settings(|c| {
                        c.editor.editor_theme = Some(id.slug().to_string())
                    });
                }
                labonair_settings_ui::apply_prefs_to_theme(&self.theme, cx);
            }
        }
    }
}
