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
use labonair_command_palette::{Page as PalettePage, PaletteChoice, PaletteData, PaletteEvent};
use labonair_panel::DockPosition;
use labonair_settings::{Settings as _, SettingsStore, ThemeSettings};

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

    /// Snapshot the panel-sourced dynamic choices the command palette renders
    /// on its sub-pages. Pref/theme scalars (`color_mode`, `editor_theme`,
    /// `font_size`, toggle bools) are **not** here — the palette reads those
    /// directly via `PalettePrefs` (T17-007). What remains cannot be pulled by
    /// the palette crate itself without a dependency cycle.
    fn build_palette_data(&self, cx: &App) -> PaletteData {
        let hosts = self
            .workspace
            .read(cx)
            .known_hosts(cx)
            .into_iter()
            .map(|(id, name)| PaletteChoice {
                id,
                title: name,
                subtitle: None,
                active: false,
            })
            .collect();

        let recent_hosts = self
            .workspace
            .read(cx)
            .recent_hosts(cx, 5)
            .into_iter()
            .map(|(id, name, _address)| PaletteChoice {
                id,
                title: name,
                subtitle: None,
                active: false,
            })
            .collect();

        let app_theme = ThemeSettings::try_get(cx)
            .map(|s| s.app_theme().to_string())
            .unwrap_or_else(|| "default".to_string());
        let active_theme_id = if app_theme.is_empty() {
            "default"
        } else {
            app_theme.as_str()
        };

        let snippets = self
            .panels
            .snippets
            .read(cx)
            .snippet_choices()
            .into_iter()
            .map(|(id, name, mode)| PaletteChoice {
                id,
                title: name,
                subtitle: Some(mode),
                active: false,
            })
            .collect();

        let git_branches = self
            .panels
            .git_panel
            .read(cx)
            .branch_choices()
            .into_iter()
            .map(|(name, current, remote)| PaletteChoice {
                active: current,
                subtitle: remote.then(|| "remote".to_string()),
                id: name.clone(),
                title: name,
            })
            .collect();

        let symbols = self
            .workspace
            .read(cx)
            .active_editor_symbols(cx)
            .into_iter()
            .map(|s| PaletteChoice {
                id: s.line.to_string(),
                title: s.name,
                subtitle: Some(format!("{}  ·  line {}", s.kind.label(), s.line + 1)),
                active: false,
            })
            .collect();

        let app_themes = labonair_settings_ui::theme_choices()
            .into_iter()
            .map(|(id, name)| PaletteChoice {
                active: id == active_theme_id,
                id,
                title: name,
                subtitle: None,
            })
            .collect();

        let status_bar_hidden = {
            let ws = self.workspace.read(cx);
            let registry = ws.status_item_registry();
            registry
                .iter()
                .filter(|r| registry.is_hidden(r.id))
                .map(|r| PaletteChoice {
                    id: r.id.to_string(),
                    title: crate::status_items::status_item_label(r.id).to_string(),
                    subtitle: None,
                    active: false,
                })
                .collect()
        };

        PaletteData {
            hosts,
            recent_hosts,
            snippets,
            git_branches,
            symbols,
            app_themes,
            status_bar_hidden,
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
                self.workspace.update(cx, |w, cx| {
                    if sftp {
                        w.open_sftp_tab(host_id, window, cx);
                    } else {
                        w.open_ssh_tab(host_id, window, cx);
                    }
                });
            }
            PaletteEvent::SetAppTheme(id) => {
                labonair_settings_ui::activate_app_theme(&id, &self.theme, cx);
            }
            PaletteEvent::PreviewAppTheme(id) => {
                labonair_settings_ui::preview_app_theme(id.as_deref(), &self.theme, cx);
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
