//! `CommandRegistry` — the single, data-driven definition site for every
//! Labonair command (T17-007).
//!
//! Before T17-007 the shell root carried a ~50-entry
//! `.on_action(cx.listener(Self::act_*))` chain plus a parallel
//! `run_palette_command` match: adding a command meant editing four places
//! (a `menu::` action, an `AppShell::act_*` handler, an `.on_action` line, a
//! `run_palette_command` arm). Now every command is one
//! [`CommandRegistry::register`] call in [`register_builtin_commands`]; the
//! native menu bar, the key bindings and the command palette all dispatch the
//! same [`CommandId`] through [`AppShell::dispatch_command`].
//!
//! ## Sanctioned deviation (see `docs/architecture.md` §8)
//!
//! The task text places `CommandRegistry` in `labonair-command-palette` (or a
//! new `labonair-commands` crate). It lives in `labonair-shell` instead: a
//! command body needs `&mut AppShell` to reach the shell-owned panel / feature
//! entities (`panels.ai_chat`, `panels.updater`, `panels.command_palette`,
//! `titlebar`, …) that cannot move onto `Workspace` without a crate cycle —
//! the exact same root cause recorded in §8.4 / §8.9. `AppShell` is only
//! nameable here, so `CommandFn` and the registry are here too. Palette and
//! keymap still *share* the registry: they dispatch through the common
//! [`CommandId`] vocabulary owned by `labonair-command-palette`.

use std::rc::Rc;

use gpui::{Context, Div, InteractiveElement, Window};
use labonair_command_palette::{CommandContext, CommandId, Page as PalettePage};
use labonair_settings_ui::open_settings_window;

use crate::app_shell::AppShell;
use crate::menu;
use crate::pane::SplitDirection;

/// The behaviour half of a command: run against the app root. Boxed so the
/// registry is plain data and a command can be cloned out before it runs
/// (side-stepping the `&mut AppShell` / `&AppShell.command_registry` borrow).
pub(crate) type CommandFn = Rc<dyn Fn(&mut AppShell, &mut Window, &mut Context<AppShell>)>;

/// One registered command.
pub(crate) struct Command {
    pub(crate) id: CommandId,
    /// Contexts the command is offered in (empty = always). Mirrors the
    /// palette's `Command::contexts`; used by [`CommandRegistry::visible_in`].
    pub(crate) contexts: &'static [CommandContext],
    pub(crate) run: CommandFn,
}

/// The app's command table. Populated once by [`register_builtin_commands`].
#[derive(Default)]
pub(crate) struct CommandRegistry {
    commands: Vec<Command>,
}

impl CommandRegistry {
    /// Register a command. One call per command — the whole point of T17-007.
    pub(crate) fn register(
        &mut self,
        id: CommandId,
        contexts: &'static [CommandContext],
        run: impl Fn(&mut AppShell, &mut Window, &mut Context<AppShell>) + 'static,
    ) {
        debug_assert!(
            !self.commands.iter().any(|c| c.id == id),
            "command {id:?} registered twice"
        );
        self.commands.push(Command {
            id,
            contexts,
            run: Rc::new(run),
        });
    }

    /// The `run` closure for `id`, if one is registered.
    pub(crate) fn run_for(&self, id: CommandId) -> Option<CommandFn> {
        self.commands
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.run.clone())
    }

    /// Every registered command. Part of the registry read API (also consumed
    /// by the file-based keymap in T19-008).
    #[allow(dead_code)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter()
    }

    /// Commands available in `ctx` (no-context commands always; context-scoped
    /// ones only when their context is active) — same rule the palette applies.
    #[allow(dead_code)]
    pub(crate) fn visible_in(&self, ctx: Option<CommandContext>) -> Vec<&Command> {
        self.commands
            .iter()
            .filter(|c| match ctx {
                None => c.contexts.is_empty(),
                Some(active) => c.contexts.is_empty() || c.contexts.contains(&active),
            })
            .collect()
    }
}

impl AppShell {
    /// Run the command bound to `id`. No-op for ids with no behaviour of their
    /// own — the palette sub-page navigators (`SwitchTab`, `ConnectSsh`, …) and
    /// the not-yet-wired `ZoomIn` / `OpenShortcuts` / `FormatDocument`
    /// placeholders, all of which were no-op action dispatches before T17-007.
    pub(crate) fn dispatch_command(
        &mut self,
        id: CommandId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(run) = self.command_registry.run_for(id) {
            run(self, window, cx);
        }
    }
}

/// Bridge every native-menu / key-binding [`menu`] action to its [`CommandId`]
/// on the shell root element. Lives here, not in `app_shell.rs`, so the shell
/// root stays pure composition (≤ 8 genuine window `.on_action`s). The menu
/// actions keep their stable types (macOS accelerator display) — this is the
/// task's "Action → Command bridge".
pub(crate) fn attach_action_handlers(
    mut el: Div,
    can_split: bool,
    has_split: bool,
    cx: &mut Context<AppShell>,
) -> Div {
    macro_rules! on {
        ($act:path => $id:expr) => {
            el = el.on_action(cx.listener(|this, _: &$act, window, cx| {
                this.dispatch_command($id, window, cx);
            }));
        };
    }

    on!(menu::NewTerminalTab => CommandId::NewTerminalTab);
    on!(menu::NewEditorTab => CommandId::NewEditorTab);
    on!(menu::NewPreviewTab => CommandId::NewPreviewTab);
    on!(menu::NewSshTab => CommandId::NewSshTab);
    on!(menu::NewSftpTab => CommandId::NewSftpTab);
    on!(menu::NewSshConnection => CommandId::NewSshConnection);
    on!(menu::NewQuickSsh => CommandId::NewQuickSsh);
    on!(menu::OpenHostManager => CommandId::OpenHostManager);
    on!(menu::Save => CommandId::Save);
    on!(menu::CloseTab => CommandId::CloseTab);
    on!(menu::NextTab => CommandId::NextTab);
    on!(menu::PrevTab => CommandId::PrevTab);
    on!(menu::Find => CommandId::Find);
    on!(menu::ToggleSidebar => CommandId::ToggleSidebar);
    on!(menu::ToggleAiPanel => CommandId::ToggleAiPanel);
    on!(menu::ToggleZenMode => CommandId::ToggleZenMode);
    on!(menu::FocusNextPane => CommandId::FocusNextPane);
    on!(menu::AskAboutSelection => CommandId::AskSelection);
    on!(menu::NewAiSession => CommandId::NewAiSession);
    on!(menu::ClearChat => CommandId::ClearChat);
    on!(menu::AiSettings => CommandId::OpenAiSettings);
    on!(menu::OpenSettings => CommandId::OpenSettings);
    on!(menu::CheckForUpdates => CommandId::CheckForUpdates);
    on!(menu::CommandPalette => CommandId::OpenCommandPalette);
    on!(menu::OpenPathBookmarks => CommandId::OpenPathBookmarks);
    on!(menu::DebugCyclePanelDock => CommandId::DebugCyclePanelDock);
    on!(menu::DebugToggleDockZoom => CommandId::DebugToggleDockZoom);
    on!(menu::SelectTab1 => CommandId::SelectTab1);
    on!(menu::SelectTab2 => CommandId::SelectTab2);
    on!(menu::SelectTab3 => CommandId::SelectTab3);
    on!(menu::SelectTab4 => CommandId::SelectTab4);
    on!(menu::SelectTab5 => CommandId::SelectTab5);
    on!(menu::SelectTab6 => CommandId::SelectTab6);
    on!(menu::SelectTab7 => CommandId::SelectTab7);
    on!(menu::SelectTab8 => CommandId::SelectTab8);
    on!(menu::SelectTab9 => CommandId::SelectTab9);

    // Context-gated: the handler only exists while the active tab supports it,
    // so the matching menu item greys out on its own (parity with the former
    // `.when(can_split)` / `.when(has_split)` guards).
    if can_split {
        on!(menu::SplitPaneRight => CommandId::SplitRight);
        on!(menu::SplitPaneDown => CommandId::SplitDown);
    }
    if has_split {
        on!(menu::ClosePane => CommandId::ClosePane);
    }

    el
}

/// The one place every command is defined. Adding a command = one
/// `r.register(...)` line here (plus, only if it needs a keystroke / menu item,
/// a `menu::` action + a line in [`attach_action_handlers`]).
const ALWAYS: &[CommandContext] = &[];
const CTX_EDITOR: &[CommandContext] = &[CommandContext::Editor];
const CTX_TERMINAL: &[CommandContext] = &[CommandContext::Terminal];
const CTX_TERMINALS: &[CommandContext] = &[CommandContext::Terminal, CommandContext::SshTerminal];

pub(crate) fn register_builtin_commands() -> CommandRegistry {
    let mut r = CommandRegistry::default();
    let always = ALWAYS;

    // ── Tabs / layout ────────────────────────────────────────────────────
    r.register(CommandId::NewTerminalTab, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.new_terminal_tab(window, cx));
    });
    r.register(CommandId::NewEditorTab, always, |s, window, cx| {
        s.workspace.update(cx, |w, cx| w.new_editor_tab(window, cx));
    });
    r.register(CommandId::NewPreviewTab, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.new_preview_tab(window, cx));
    });
    r.register(CommandId::Save, always, |s, _window, cx| {
        s.workspace.update(cx, |w, cx| w.save_active(cx));
    });
    r.register(CommandId::CloseTab, always, |s, window, cx| {
        s.workspace.update(cx, |w, cx| w.close_active(window, cx));
    });
    r.register(CommandId::DuplicateTab, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.duplicate_active_tab(window, cx));
    });
    r.register(CommandId::CloseOtherTabs, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.close_other_tabs(window, cx));
    });
    r.register(CommandId::NextTab, always, |s, window, cx| {
        s.workspace.update(cx, |w, cx| w.cycle(true, window, cx));
    });
    r.register(CommandId::PrevTab, always, |s, window, cx| {
        s.workspace.update(cx, |w, cx| w.cycle(false, window, cx));
    });
    r.register(CommandId::FocusNextPane, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.focus_next_pane(window, cx));
    });
    for (id, idx) in [
        (CommandId::SelectTab1, 0usize),
        (CommandId::SelectTab2, 1),
        (CommandId::SelectTab3, 2),
        (CommandId::SelectTab4, 3),
        (CommandId::SelectTab5, 4),
        (CommandId::SelectTab6, 5),
        (CommandId::SelectTab7, 6),
        (CommandId::SelectTab8, 7),
        (CommandId::SelectTab9, 8),
    ] {
        r.register(id, always, move |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.select_tab_by_index(idx, window, cx));
        });
    }

    // ── Terminal panes ──────────────────────────────────────────────────
    r.register(CommandId::SplitRight, CTX_TERMINAL, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.split(SplitDirection::Right, window, cx));
    });
    r.register(CommandId::SplitDown, CTX_TERMINAL, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.split(SplitDirection::Down, window, cx));
    });
    r.register(CommandId::ClosePane, CTX_TERMINAL, |s, window, cx| {
        s.workspace.update(cx, |w, cx| w.close_pane(window, cx));
    });
    r.register(CommandId::ClearTerminal, CTX_TERMINALS, |s, _window, cx| {
        s.workspace.update(cx, |w, cx| w.clear_active_terminal(cx));
    });

    // ── Search ──────────────────────────────────────────────────────────
    r.register(CommandId::Find, always, |s, window, cx| {
        s.toggle_search_overlay(window, cx);
    });

    // ── Connections ─────────────────────────────────────────────────────
    for id in [
        CommandId::OpenHostManager,
        CommandId::NewSshTab,
        CommandId::NewSftpTab,
        CommandId::NewQuickSsh,
    ] {
        r.register(id, always, |s, _window, cx| {
            s.workspace.update(cx, |w, cx| w.open_host_manager(cx));
        });
    }
    r.register(CommandId::NewSshConnection, always, |s, window, cx| {
        // `Cmd+Shift+N` — open the palette straight to the Hosts page
        // (`Enter` = SSH, `Shift+Enter` = SFTP).
        s.show_command_palette(Some(PalettePage::Hosts), window, cx);
    });

    // ── View / sidebar ─────────────────────────────────────────────────
    r.register(CommandId::ToggleSidebar, always, |s, _window, cx| {
        s.toggle_sidebar(cx);
    });
    r.register(CommandId::DebugCyclePanelDock, always, |s, _window, cx| {
        let pos = s.primary_dock(cx);
        let Some(name) = s
            .workspace
            .read(cx)
            .dock(pos)
            .active_name()
            .map(str::to_owned)
        else {
            return;
        };
        s.move_panel(&name, pos.next(), cx);
    });
    r.register(CommandId::DebugToggleDockZoom, always, |s, _window, cx| {
        let pos = s.primary_dock(cx);
        s.workspace.update(cx, |w, cx| {
            let z = w.dock(pos).is_zoomed();
            w.dock_mut(pos).set_zoomed(!z);
            w.persist_docks(cx);
        });
        cx.notify();
    });

    // ── AI ─────────────────────────────────────────────────────────────
    r.register(CommandId::ToggleAiPanel, always, |s, _window, cx| {
        s.select_panel("ai", cx);
    });
    r.register(CommandId::AskSelection, always, |s, _window, cx| {
        let Some((label, text)) = s.workspace.read(cx).active_selection(cx) else {
            return;
        };
        s.panels
            .ai_chat
            .update(cx, |v, cx| v.attach_selection(label, text, cx));
        s.open_panel("ai", cx);
    });
    r.register(CommandId::NewAiSession, always, |s, _window, cx| {
        s.panels.ai_chat.update(cx, |v, cx| v.new_session(cx));
        s.open_panel("ai", cx);
    });
    r.register(CommandId::ClearChat, always, |s, _window, cx| {
        s.panels.ai_chat.update(cx, |v, cx| v.clear_active_chat(cx));
    });

    // ── Snippets / source control ──────────────────────────────────────
    r.register(CommandId::OpenSnippetsPanel, always, |s, _window, cx| {
        s.open_panel("snippets", cx);
    });
    r.register(CommandId::OpenGitGraph, always, |s, _window, cx| {
        s.workspace.update(cx, |w, cx| w.open_git_graph_tab(cx));
    });
    r.register(CommandId::FocusSourceControl, always, |s, _window, cx| {
        s.open_panel("source-control", cx);
    });

    // ── Bookmarks / palette ────────────────────────────────────────────
    r.register(CommandId::OpenPathBookmarks, always, |s, window, cx| {
        s.toggle_bookmarks(window, cx);
    });
    r.register(CommandId::OpenCommandPalette, always, |s, window, cx| {
        s.toggle_command_palette(window, cx);
    });

    // ── Zen-mode / settings toggles ────────────────────────────────────
    r.register(CommandId::ToggleZenMode, always, |s, _window, cx| {
        s.toggle_zen_mode(cx);
    });
    for (id, key, ctx) in [
        (CommandId::ToggleZenModeHeader, "zenModeShowHeader", ALWAYS),
        (
            CommandId::ToggleZenModeStatusbar,
            "zenModeShowStatusbar",
            ALWAYS,
        ),
        (
            CommandId::ToggleEditorWordWrap,
            "editorWordWrap",
            CTX_EDITOR,
        ),
        (
            CommandId::ToggleLineNumbers,
            "editorLineNumbers",
            CTX_EDITOR,
        ),
        (
            CommandId::ToggleFormatOnSave,
            "editorFormatOnSave",
            CTX_EDITOR,
        ),
        (
            CommandId::ToggleCursorBlink,
            "terminalCursorBlink",
            CTX_TERMINAL,
        ),
        (
            CommandId::TogglePaneHeader,
            "terminalShowPaneHeader",
            CTX_TERMINAL,
        ),
        (
            CommandId::TogglePaneFooter,
            "terminalShowPaneFooter",
            CTX_TERMINAL,
        ),
        (CommandId::ToggleVimMode, "vimMode", ALWAYS),
    ] {
        r.register(id, ctx, move |s, _window, cx| s.toggle_zen_pref(key, cx));
    }

    // ── Application ────────────────────────────────────────────────────
    r.register(CommandId::OpenSettings, always, |_s, _window, cx| {
        open_settings_window(None, cx);
    });
    r.register(CommandId::OpenAiSettings, always, |_s, _window, cx| {
        open_settings_window(Some("ai"), cx);
    });
    r.register(CommandId::OpenProjectSettings, always, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.open_or_create_project_settings(window, cx));
    });
    r.register(CommandId::CheckForUpdates, always, |s, _window, cx| {
        s.panels.updater.update(cx, |u, cx| u.run_check(true, cx));
    });

    // Not registered on purpose (no behaviour of their own, unchanged from
    // pre-T17-007 no-op action dispatches): `ZoomIn` / `ZoomOut` / `ZoomReset`
    // / `OpenShortcuts` / `FormatDocument`, and every sub-page navigator id
    // (`SwitchTab`, `AdjustFontSize`, `ConnectSsh`, `OpenSftp`,
    // `ChangeAppTheme`, `ChangeColorMode`, `ChangeEditorTheme`,
    // `SwitchAiSession`, `RunSnippet`, `GitSwitchBranch`, `GoToSymbol`) which
    // the palette resolves internally and never emits as `Run`.

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_id_is_unique_and_context_rules_hold() {
        let r = register_builtin_commands();
        let ids: Vec<_> = r.iter().map(|c| c.id).collect();
        let mut deduped = ids.clone();
        deduped.sort_by_key(|id| format!("{id:?}"));
        deduped.dedup();
        assert_eq!(ids.len(), deduped.len(), "duplicate command registration");

        // Split/close-pane are terminal-only; new-tab is always available.
        let always = r.visible_in(None);
        assert!(always.iter().any(|c| c.id == CommandId::NewTerminalTab));
        assert!(!always.iter().any(|c| c.id == CommandId::SplitRight));
        assert!(!always.iter().any(|c| c.id == CommandId::ClosePane));

        let term = r.visible_in(Some(CommandContext::Terminal));
        assert!(term.iter().any(|c| c.id == CommandId::SplitRight));
        assert!(term.iter().any(|c| c.id == CommandId::ClosePane));
    }

    #[test]
    fn unregistered_ids_resolve_to_no_run() {
        let r = register_builtin_commands();
        assert!(r.run_for(CommandId::ZoomIn).is_none());
        assert!(r.run_for(CommandId::SwitchTab).is_none());
        assert!(r.run_for(CommandId::OpenShortcuts).is_none());
    }
}
