//! Menu / keyboard-shortcut / command-palette action handlers for [`AppShell`].
//!
//! Split out of `app_shell.rs` in T17-006 so the shell root stays pure
//! composition. Every handler here is bound as a GPUI action on the root
//! element in `AppShell::render`, so the native menu bar and the keyboard
//! shortcuts run identical code. The `.on_action(...)` registration list itself
//! stays in `render`; only the bodies live here. T17-007 replaces this module
//! with a data-driven `CommandRegistry`.

use std::collections::HashMap;

use gpui::{Context, Window};
use labonair_command_palette::{
    CommandId, Page as PalettePage, PaletteChoice, PaletteData, PaletteEvent,
};
use labonair_panel::DockPosition;
use labonair_panel_explorer::BookmarkEvent;
use labonair_settings_ui::{open_settings_window, SettingsTab};

use crate::app_shell::AppShell;
use crate::menu;
use crate::modals::{BookmarksModal, CommandPaletteModal, UpdaterModal};
use crate::pane::SplitDirection;

/// Generate a `menu::SelectTabN` action handler that jumps to the tab at a
/// fixed 0-based index (T13-005 — `Cmd+1..9`).
macro_rules! select_tab_action {
    ($name:ident, $action:ident, $idx:expr) => {
        pub(crate) fn $name(
            &mut self,
            _: &menu::$action,
            window: &mut Window,
            cx: &mut Context<Self>,
        ) {
            self.workspace
                .update(cx, |w, cx| w.select_tab_by_index($idx, window, cx));
        }
    };
}

impl AppShell {
    pub(crate) fn act_open_settings(
        &mut self,
        _: &menu::OpenSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(None, cx);
    }

    pub(crate) fn act_open_ai_settings(
        &mut self,
        _: &menu::AiSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(Some(SettingsTab::Ai), cx);
    }

    pub(crate) fn act_check_for_updates(
        &mut self,
        _: &menu::CheckForUpdates,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels
            .updater
            .update(cx, |u, cx| u.run_check(true, cx));
    }

    /// The primary edge as a [`DockPosition`] (per `sidebarPosition`).
    fn primary_dock(&self, cx: &gpui::App) -> DockPosition {
        self.workspace.read(cx).primary_dock(cx)
    }

    /// `Cmd+B` — toggle the primary dock open/closed.
    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        let pos = self.primary_dock(cx);
        self.workspace.update(cx, |w, cx| {
            w.dock_mut(pos).toggle_open();
            w.persist_docks(cx);
        });
        cx.notify();
    }

    /// Status-bar-toggle intent: open + activate `name`, or close its dock if
    /// it is already the active panel there.
    pub(crate) fn select_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.select_panel(name, cx));
        cx.notify();
    }

    /// "show me X" — never closes the dock (palette / menu intent).
    pub(crate) fn open_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.open_panel(name, cx));
        cx.notify();
    }

    /// Move a panel to another dock (T17-002 API).
    fn move_panel(&mut self, name: &str, to: DockPosition, cx: &mut Context<Self>) {
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

    // ── Menu / shortcut action handlers (T04-005) ──────────────────────────

    pub(crate) fn act_new_terminal_tab(
        &mut self,
        _: &menu::NewTerminalTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_terminal_tab(window, cx));
    }

    pub(crate) fn act_close_tab(
        &mut self,
        _: &menu::CloseTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.close_active(window, cx));
    }

    pub(crate) fn act_close_pane(
        &mut self,
        _: &menu::ClosePane,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.close_pane(window, cx));
    }

    pub(crate) fn act_split_right(
        &mut self,
        _: &menu::SplitPaneRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.split(SplitDirection::Right, window, cx));
    }

    pub(crate) fn act_split_down(
        &mut self,
        _: &menu::SplitPaneDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.split(SplitDirection::Down, window, cx));
    }

    pub(crate) fn act_find(&mut self, _: &menu::Find, window: &mut Window, cx: &mut Context<Self>) {
        let handled = self
            .workspace
            .update(cx, |w, cx| w.find_in_active_editor(cx));
        if !handled {
            self.titlebar.update(cx, |t, cx| t.open_search(window, cx));
        }
    }

    pub(crate) fn act_save(&mut self, _: &menu::Save, _: &mut Window, cx: &mut Context<Self>) {
        self.workspace.update(cx, |w, cx| w.save_active(cx));
    }

    pub(crate) fn act_new_editor_tab(
        &mut self,
        _: &menu::NewEditorTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_editor_tab(window, cx));
    }

    pub(crate) fn act_new_preview_tab(
        &mut self,
        _: &menu::NewPreviewTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.new_preview_tab(window, cx));
    }

    pub(crate) fn act_toggle_sidebar(
        &mut self,
        _: &menu::ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar(cx);
    }

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

    pub(crate) fn act_next_tab(
        &mut self,
        _: &menu::NextTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.cycle(true, window, cx));
    }

    pub(crate) fn act_prev_tab(
        &mut self,
        _: &menu::PrevTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.cycle(false, window, cx));
    }

    pub(crate) fn act_toggle_ai_panel(
        &mut self,
        _: &menu::ToggleAiPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_panel("ai", cx);
    }

    /// Temporary T17-002 debug shortcut (`Cmd+Alt+Shift+M`): move the active
    /// panel of the primary dock to the next dock position.
    pub(crate) fn act_debug_cycle_panel_dock(
        &mut self,
        _: &menu::DebugCyclePanelDock,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = self.primary_dock(cx);
        let Some(name) = self
            .workspace
            .read(cx)
            .dock(pos)
            .active_name()
            .map(str::to_owned)
        else {
            return;
        };
        self.move_panel(&name, pos.next(), cx);
    }

    /// Temporary T17-002 debug shortcut (`Cmd+Alt+Shift+Z`): toggle the primary
    /// dock's zoom state.
    pub(crate) fn act_debug_toggle_dock_zoom(
        &mut self,
        _: &menu::DebugToggleDockZoom,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = self.primary_dock(cx);
        self.workspace.update(cx, |w, cx| {
            let z = w.dock(pos).is_zoomed();
            w.dock_mut(pos).set_zoomed(!z);
            w.persist_docks(cx);
        });
        cx.notify();
    }

    /// "Ask AI about Selection" — capture the active editor/terminal selection
    /// into the AI composer and reveal the panel.
    pub(crate) fn act_ask_about_selection(
        &mut self,
        _: &menu::AskAboutSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((label, text)) = self.workspace.read(cx).active_selection(cx) else {
            return;
        };
        self.panels
            .ai_chat
            .update(cx, |v, cx| v.attach_selection(label, text, cx));
        self.open_panel("ai", cx);
    }

    pub(crate) fn act_new_ai_session(
        &mut self,
        _: &menu::NewAiSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels.ai_chat.update(cx, |v, cx| v.new_session(cx));
        self.open_panel("ai", cx);
    }

    pub(crate) fn act_clear_chat(
        &mut self,
        _: &menu::ClearChat,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.panels
            .ai_chat
            .update(cx, |v, cx| v.clear_active_chat(cx));
    }

    pub(crate) fn act_open_host_manager(
        &mut self,
        _: &menu::OpenHostManager,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    pub(crate) fn act_new_ssh_tab(
        &mut self,
        _: &menu::NewSshTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    pub(crate) fn act_new_sftp_tab(
        &mut self,
        _: &menu::NewSftpTab,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    pub(crate) fn act_new_ssh_connection(
        &mut self,
        _: &menu::NewSshConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `Cmd+Shift+N` opens the command palette straight to the Hosts page
        // (`Enter` = SSH, `Shift+Enter` = SFTP).
        self.show_command_palette(Some(PalettePage::Hosts), window, cx);
    }

    pub(crate) fn act_new_quick_ssh(
        &mut self,
        _: &menu::NewQuickSsh,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace.update(cx, |w, cx| w.open_host_manager(cx));
    }

    pub(crate) fn act_command_palette(
        &mut self,
        _: &menu::CommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_command_palette(window, cx);
    }

    pub(crate) fn act_open_path_bookmarks(
        &mut self,
        _: &menu::OpenPathBookmarks,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_bookmarks(window, cx);
    }

    /// Toggle the command palette through the modal layer (T17-005).
    fn toggle_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    fn show_command_palette(
        &mut self,
        page: Option<PalettePage>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The palette's data used to be rebuilt every frame in `render`; now it
        // is snapshotted at open time (T17-006 — no per-frame `build_palette_data`).
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

    /// Toggle the path-bookmarks popover. Its `open` flag is mirrored into the
    /// modal layer by [`Self::sync_bookmarks_modal`] on the next tick.
    fn toggle_bookmarks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.panels
            .bookmarks
            .update(cx, |b, cx| b.toggle(window, cx));
    }

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

    /// Mirror the path-bookmarks popover's `open` flag into the modal layer.
    pub(crate) fn sync_bookmarks_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let want = self.panels.bookmarks.read(cx).is_open();
        let have = self
            .modal_layer
            .read(cx)
            .active_modal::<BookmarksModal>()
            .is_some();
        if want == have {
            return;
        }
        let bookmarks = self.panels.bookmarks.clone();
        self.modal_layer.update(cx, |layer, cx| {
            if want {
                layer.open_modal(window, cx, move |_, cx| BookmarksModal::new(bookmarks, cx));
            } else {
                layer.hide_modal(window, cx);
            }
        });
    }

    pub(crate) fn act_focus_next_pane(
        &mut self,
        _: &menu::FocusNextPane,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.workspace
            .update(cx, |w, cx| w.focus_next_pane(window, cx));
    }

    pub(crate) fn act_toggle_zen_mode(
        &mut self,
        _: &menu::ToggleZenMode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_zen_mode(cx);
    }

    /// `view.zenMode`: both bars visible → hide both, otherwise show both.
    fn toggle_zen_mode(&mut self, cx: &mut Context<Self>) {
        let p = self.prefs.read(cx).get();
        let next = !(p.zen_mode_show_header || p.zen_mode_show_statusbar);
        self.prefs.update(cx, |s, cx| {
            s.set_value("zenModeShowHeader", serde_json::Value::Bool(next), cx);
            s.set_value("zenModeShowStatusbar", serde_json::Value::Bool(next), cx);
        });
    }

    fn toggle_zen_pref(&mut self, key: &str, cx: &mut Context<Self>) {
        let cur = self
            .prefs
            .read(cx)
            .value(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        self.prefs.update(cx, |s, cx| {
            s.set_value(key, serde_json::Value::Bool(!cur), cx)
        });
    }

    select_tab_action!(act_select_tab_1, SelectTab1, 0);
    select_tab_action!(act_select_tab_2, SelectTab2, 1);
    select_tab_action!(act_select_tab_3, SelectTab3, 2);
    select_tab_action!(act_select_tab_4, SelectTab4, 3);
    select_tab_action!(act_select_tab_5, SelectTab5, 4);
    select_tab_action!(act_select_tab_6, SelectTab6, 5);
    select_tab_action!(act_select_tab_7, SelectTab7, 6);
    select_tab_action!(act_select_tab_8, SelectTab8, 7);
    select_tab_action!(act_select_tab_9, SelectTab9, 8);

    /// Snapshot the live state the command palette needs for its dynamic
    /// sub-pages and `rightLabel` states.
    fn build_palette_data(&self, cx: &gpui::App) -> PaletteData {
        let ts = self.workspace.read(cx).tab_store();
        let ts = ts.read(cx);
        let active = ts.active_id();
        let tabs = ts
            .tabs()
            .iter()
            .map(|t| PaletteChoice {
                id: t.id.to_string(),
                title: t.label(),
                subtitle: Some(t.kind.default_title().to_string()),
                active: t.id == active,
            })
            .collect();

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
            .map(|(id, name)| PaletteChoice {
                id,
                title: name,
                subtitle: None,
                active: false,
            })
            .collect();

        let theme = self.theme.read(cx);
        let p = self.prefs.read(cx).get();

        let active_theme_id = if p.app_theme.is_empty() {
            "default"
        } else {
            p.app_theme.as_str()
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

        let ai_sessions = self
            .panels
            .ai_chat
            .read(cx)
            .session_choices(cx)
            .into_iter()
            .map(|(id, title, active)| PaletteChoice {
                id,
                title,
                subtitle: None,
                active,
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
        let mut toggles = HashMap::new();
        toggles.insert("zenModeShowHeader", p.zen_mode_show_header);
        toggles.insert("zenModeShowStatusbar", p.zen_mode_show_statusbar);
        toggles.insert("editorWordWrap", p.editor_word_wrap);
        toggles.insert("editorLineNumbers", p.editor_line_numbers);
        toggles.insert("editorFormatOnSave", p.editor_format_on_save);
        toggles.insert("terminalCursorBlink", p.terminal_cursor_blink);
        toggles.insert("terminalShowPaneHeader", p.terminal_show_pane_header);
        toggles.insert("terminalShowPaneFooter", p.terminal_show_pane_footer);
        toggles.insert("vimMode", p.editor_vim_mode);

        PaletteData {
            tabs,
            hosts,
            recent_hosts,
            snippets,
            ai_sessions,
            git_branches,
            symbols,
            app_themes,
            color_mode: theme.preference(),
            editor_theme: theme.editor_theme(),
            font_size: Some(p.terminal_font_size),
            toggles,
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
            PaletteEvent::Run(id) => self.run_palette_command(id, window, cx),
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
                labonair_settings_ui::activate_app_theme(&id, &self.prefs, &self.theme, cx);
            }
            PaletteEvent::PreviewAppTheme(id) => {
                labonair_settings_ui::preview_app_theme(
                    id.as_deref(),
                    &self.prefs,
                    &self.theme,
                    cx,
                );
            }
            PaletteEvent::RunSnippet(id) => {
                self.panels
                    .snippets
                    .update(cx, |s, cx| s.run_by_id(&id, window, cx));
            }
            PaletteEvent::SwitchAiSession(id) => {
                self.panels
                    .ai_chat
                    .update(cx, |v, cx| v.switch_to_session(&id, cx));
                self.open_panel("ai", cx);
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
            PaletteEvent::SetColorMode(pref) => {
                let key = match pref {
                    crate::theme::ThemePreference::System => "system",
                    crate::theme::ThemePreference::Light => "light",
                    crate::theme::ThemePreference::Dark => "dark",
                };
                self.prefs.update(cx, |s, cx| {
                    s.set_value("theme", serde_json::Value::String(key.into()), cx)
                });
                let p = self.prefs.read(cx).get().clone();
                labonair_settings_ui::apply_prefs_to_theme(&p, &self.theme, cx);
            }
            PaletteEvent::SetEditorTheme(id) => {
                self.prefs.update(cx, |s, cx| {
                    s.set_value(
                        "editorTheme",
                        serde_json::Value::String(id.slug().into()),
                        cx,
                    )
                });
                let p = self.prefs.read(cx).get().clone();
                labonair_settings_ui::apply_prefs_to_theme(&p, &self.theme, cx);
            }
        }
    }

    /// Service a single bookmark pick straight from the `BookmarkEvent`
    /// subscription (T17-005 — no `pending_bookmarks` buffer / `drain`).
    pub(crate) fn handle_bookmark_event(
        &mut self,
        event: BookmarkEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            BookmarkEvent::OpenLocal(path) => {
                self.panels
                    .explorer
                    .update(cx, |e, cx| e.set_root_str(Some(path), cx));
                self.select_panel("explorer", cx);
            }
            BookmarkEvent::OpenRemote { host_id, .. } => {
                self.workspace
                    .update(cx, |w, cx| w.open_sftp_tab(host_id, window, cx));
            }
        }
    }

    fn run_palette_command(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        macro_rules! dispatch {
            ($action:expr) => {
                window.dispatch_action(Box::new($action), cx)
            };
        }
        match id {
            CommandId::NewTerminalTab => dispatch!(menu::NewTerminalTab),
            CommandId::NewEditorTab => dispatch!(menu::NewEditorTab),
            CommandId::CloseTab => dispatch!(menu::CloseTab),
            CommandId::ClosePane => dispatch!(menu::ClosePane),
            CommandId::SplitRight => dispatch!(menu::SplitPaneRight),
            CommandId::SplitDown => dispatch!(menu::SplitPaneDown),
            CommandId::NextTab => dispatch!(menu::NextTab),
            CommandId::PrevTab => dispatch!(menu::PrevTab),
            CommandId::Find => dispatch!(menu::Find),
            CommandId::ToggleSidebar => dispatch!(menu::ToggleSidebar),
            CommandId::ToggleFullScreen => dispatch!(menu::ToggleFullScreen),
            CommandId::ZoomIn => dispatch!(menu::ZoomIn),
            CommandId::ZoomOut => dispatch!(menu::ZoomOut),
            CommandId::ZoomReset => dispatch!(menu::ResetZoom),
            CommandId::AskSelection => dispatch!(menu::AskAboutSelection),
            CommandId::NewAiSession => dispatch!(menu::NewAiSession),
            CommandId::OpenHostManager => dispatch!(menu::OpenHostManager),
            CommandId::OpenShortcuts => dispatch!(menu::OpenShortcuts),
            CommandId::OpenSettings => dispatch!(menu::OpenSettings),
            CommandId::CheckForUpdates => dispatch!(menu::CheckForUpdates),
            CommandId::DuplicateTab => self
                .workspace
                .update(cx, |w, cx| w.duplicate_active_tab(window, cx)),
            CommandId::CloseOtherTabs => self
                .workspace
                .update(cx, |w, cx| w.close_other_tabs(window, cx)),
            CommandId::ClearTerminal => self
                .workspace
                .update(cx, |w, cx| w.clear_active_terminal(cx)),
            CommandId::ToggleAiPanel => self.select_panel("ai", cx),
            CommandId::OpenSnippetsPanel => self.open_panel("snippets", cx),
            CommandId::OpenPathBookmarks => self.toggle_bookmarks(window, cx),
            CommandId::OpenGitGraph => self.workspace.update(cx, |w, cx| w.open_git_graph_tab(cx)),
            CommandId::FocusSourceControl => self.open_panel("source-control", cx),
            CommandId::ToggleZenMode => self.toggle_zen_mode(cx),
            CommandId::ToggleZenModeHeader => self.toggle_zen_pref("zenModeShowHeader", cx),
            CommandId::ToggleZenModeStatusbar => self.toggle_zen_pref("zenModeShowStatusbar", cx),
            CommandId::OpenAiSettings => dispatch!(menu::AiSettings),
            CommandId::ToggleEditorWordWrap => self.toggle_zen_pref("editorWordWrap", cx),
            CommandId::ToggleLineNumbers => self.toggle_zen_pref("editorLineNumbers", cx),
            CommandId::ToggleFormatOnSave => self.toggle_zen_pref("editorFormatOnSave", cx),
            CommandId::ToggleCursorBlink => self.toggle_zen_pref("terminalCursorBlink", cx),
            CommandId::TogglePaneHeader => self.toggle_zen_pref("terminalShowPaneHeader", cx),
            CommandId::TogglePaneFooter => self.toggle_zen_pref("terminalShowPaneFooter", cx),
            CommandId::ToggleVimMode => self.toggle_zen_pref("vimMode", cx),
            // Sub-page navigators — resolved inside the palette, never emitted
            // as `Run`. Listed so the match stays exhaustive.
            CommandId::SwitchTab
            | CommandId::AdjustFontSize
            | CommandId::ConnectSsh
            | CommandId::OpenSftp
            | CommandId::ChangeAppTheme
            | CommandId::ChangeColorMode
            | CommandId::ChangeEditorTheme
            | CommandId::SwitchAiSession
            | CommandId::RunSnippet
            | CommandId::GitSwitchBranch
            | CommandId::GoToSymbol => {}
            CommandId::FormatDocument => {}
        }
    }
}
