//! `CommandDispatcher` — the single, data-driven definition site for every
//! Labonair command (T17-007).
//!
//! Before T17-007 the shell root carried a ~50-entry
//! `.on_action(cx.listener(Self::act_*))` chain plus a parallel
//! `run_palette_command` match: adding a command meant editing four places
//! (a `menu::` action, an `AppShell::act_*` handler, an `.on_action` line, a
//! `run_palette_command` arm). Now every command is one
//! [`CommandDispatcher::register`] call in [`register_builtin_commands`]; the
//! native menu bar, the key bindings and the command palette all dispatch the
//! same [`CommandId`] through [`AppShell::dispatch_command`].
//!
//! ## Sanctioned deviation (see `docs/architecture.md` §8)
//!
//! The UI-free metadata registry lives in `labonair-command-palette-core`.
//! This module adds only the shell-owned execution closures, keeping the
//! palette, menu, and keymap on the same descriptor snapshot without making
//! the core registry depend on GPUI or `AppShell`.

use std::rc::Rc;

use gpui::{Context, Div, InteractiveElement, Window};
use labonair_command_palette::Page as PalettePage;
use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider,
    CommandRegistry as PaletteCommandRegistry, CommandSubmenu,
};
use labonair_settings_ui::open_settings_window;

use crate::app_shell::AppShell;
use crate::menu;
use crate::pane::SplitDirection;

/// The behaviour half of a command: run against the app root. Boxed so the
/// registry is plain data and a command can be cloned out before it runs
/// (side-stepping the `&mut AppShell` / `&AppShell.command_registry` borrow).
pub(crate) type CommandFn = Rc<dyn Fn(&mut AppShell, &mut Window, &mut Context<AppShell>)>;

/// Shell-owned execution half of one registered command.
#[derive(Clone)]
pub(crate) struct Command {
    pub(crate) id: CommandId,
    pub(crate) run: CommandFn,
}

/// Shell dispatcher backed by the UI-free command metadata registry.
#[derive(Clone, Default)]
pub(crate) struct CommandDispatcher {
    metadata: PaletteCommandRegistry,
    commands: Vec<Command>,
}

/// Native-window metadata owned by the shell composition surface. This is
/// intentionally the only command provider that lives in `labonair-shell`:
/// the command targets a GPUI `Window`, not a product capability.
#[derive(Clone, Copy, Debug, Default)]
struct ShellCommandProvider;

impl CommandProvider for ShellCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::ToggleFullScreen, "Toggle Full Screen", "View")
                .with_icon(CommandIcon::Square),
        ]
    }
}

impl CommandDispatcher {
    /// Register metadata and behaviour together. The descriptor is the only
    /// source consumed by palette, menus, and keymap tooling.
    pub(crate) fn register(
        &mut self,
        descriptor: CommandDescriptor,
        run: impl Fn(&mut AppShell, &mut Window, &mut Context<AppShell>) + 'static,
    ) {
        let id = descriptor.id;
        if let Some(registered) = self.metadata.command(id) {
            assert_eq!(
                registered, &descriptor,
                "command metadata differs between its owner provider and shell adapter"
            );
        } else if let Err(error) = self.metadata.register(descriptor) {
            panic!("invalid built-in command registry: {error}");
        }
        self.commands.push(Command {
            id,
            run: Rc::new(run),
        });
    }

    /// Register metadata contributed by an owning capability. The shell only
    /// assembles providers; it does not define their palette rows.
    pub(crate) fn register_provider<P: CommandProvider>(&mut self, provider: &P) {
        if let Err(error) = self.metadata.register_provider(provider) {
            panic!("invalid command provider registry: {error}");
        }
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
    pub(crate) fn iter(&self) -> impl Iterator<Item = &CommandDescriptor> {
        self.metadata.iter()
    }

    /// Commands available in `ctx` (no-context commands always; context-scoped
    /// ones only when their context is active) — same rule the palette applies.
    #[allow(dead_code)]
    pub(crate) fn visible_in(&self, ctx: Option<CommandContext>) -> Vec<&CommandDescriptor> {
        self.metadata.available(ctx)
    }

    /// Clone the descriptor snapshot for the palette view.
    pub(crate) fn descriptors(&self) -> Vec<CommandDescriptor> {
        self.metadata.snapshot()
    }

    /// Resolve the default shortcut through the same registry used by the
    /// palette, rather than a second shortcut-to-command table.
    pub(crate) fn command_for_shortcut(
        &self,
        shortcut: labonair_keymap::ShortcutId,
    ) -> Option<CommandId> {
        self.metadata.command_for_shortcut(shortcut)
    }
}

impl AppShell {
    /// Run the command bound to `id`. No-op for ids with no behaviour of their
    /// own — the palette sub-page navigators (`SwitchTab`, `ConnectSsh`, …) and
    /// the not-yet-wired `ZoomIn` / `FormatDocument`
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
    on!(menu::OpenProject => CommandId::OpenProject);
    on!(menu::ReturnToStandalone => CommandId::ReturnToStandalone);
    on!(menu::NewEditorTab => CommandId::NewEditorTab);
    on!(menu::NewPreviewTab => CommandId::NewPreviewTab);
    on!(menu::NewSshTab => CommandId::NewSshTab);
    on!(menu::NewSftpTab => CommandId::NewSftpTab);
    on!(menu::NewSshConnection => CommandId::NewSshConnection);
    on!(menu::NewQuickSsh => CommandId::NewQuickSsh);
    on!(menu::OpenHostSettings => CommandId::OpenHostSettings);
    on!(menu::Save => CommandId::Save);
    on!(menu::CloseTab => CommandId::CloseTab);
    on!(menu::NextTab => CommandId::NextTab);
    on!(menu::PrevTab => CommandId::PrevTab);
    on!(menu::Find => CommandId::Find);
    on!(menu::ToggleSidebar => CommandId::ToggleSidebar);
    on!(menu::ToggleZenMode => CommandId::ToggleZenMode);
    on!(menu::FocusNextPane => CommandId::FocusNextPane);
    on!(menu::OpenSettings => CommandId::OpenSettings);
    on!(menu::OpenKeymapJson => CommandId::OpenKeymapJson);
    on!(menu::CheckForUpdates => CommandId::CheckForUpdates);
    on!(menu::CommandPalette => CommandId::OpenCommandPalette);
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

#[allow(clippy::too_many_arguments)]
fn command_descriptor(
    id: CommandId,
    title: &str,
    section: &str,
    contexts: &[CommandContext],
    shortcut: Option<labonair_keymap::ShortcutId>,
    icon: CommandIcon,
    submenu: Option<CommandSubmenu>,
) -> CommandDescriptor {
    let mut descriptor = CommandDescriptor::new(id, title, section)
        .with_contexts(contexts)
        .with_icon(icon);
    if let Some(shortcut) = shortcut {
        descriptor = descriptor.with_shortcut(shortcut);
    }
    if let Some(submenu) = submenu {
        descriptor = descriptor.with_submenu(submenu);
    }
    descriptor
}

pub(crate) fn register_builtin_commands() -> CommandDispatcher {
    let mut r = CommandDispatcher::default();
    let always = ALWAYS;

    // Capability-owned metadata enters through providers. The existing
    // execution registrations below are transitional shell adapters; their
    // descriptors are checked against the owner snapshot and never become a
    // second palette source.
    r.register_provider(&labonair_workspace::command_provider::WorkspaceCommandProvider);
    r.register_provider(&labonair_terminal::command_provider::TerminalCommandProvider);
    r.register_provider(&labonair_editor::command_provider::EditorCommandProvider);
    r.register_provider(&labonair_hosts::command_provider::HostsCommandProvider);
    r.register_provider(&labonair_theme::command_provider::ThemeCommandProvider);
    r.register_provider(&labonair_settings::command_provider::SettingsCommandProvider);
    r.register_provider(&labonair_keymap::command_provider::KeymapCommandProvider);
    r.register_provider(&labonair_git::command_provider::GitCommandProvider);
    r.register_provider(&labonair_snippets::command_provider::SnippetsCommandProvider);
    r.register_provider(
        &labonair_command_palette_core::command_provider::CommandPaletteCommandProvider,
    );
    r.register_provider(&ShellCommandProvider);

    // ── Tabs / layout ────────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::NewTerminalTab,
            "New Terminal Tab",
            "Layout",
            always,
            Some(labonair_keymap::ShortcutId::TabNew),
            CommandIcon::Terminal,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.new_terminal_tab(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::NewEditorTab,
            "New Editor Tab",
            "Layout",
            always,
            Some(labonair_keymap::ShortcutId::TabNewEditor),
            CommandIcon::File,
            None,
        ),
        |s, window, cx| {
            s.workspace.update(cx, |w, cx| w.new_editor_tab(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::NewPreviewTab,
            "New Preview Tab",
            "Layout",
            always,
            Some(labonair_keymap::ShortcutId::TabNewPreview),
            CommandIcon::File,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.new_preview_tab(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::Save,
            "Save",
            "Tab Actions",
            always,
            None,
            CommandIcon::Edit,
            None,
        ),
        |s, _window, cx| {
            s.workspace.update(cx, |w, cx| w.save_active(cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::CloseTab,
            "Close Current Tab",
            "Tab Actions",
            always,
            Some(labonair_keymap::ShortcutId::TabClose),
            CommandIcon::Close,
            None,
        ),
        |s, window, cx| {
            s.workspace.update(cx, |w, cx| w.close_active(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::DuplicateTab,
            "Duplicate Tab",
            "Layout",
            always,
            None,
            CommandIcon::Copy,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.duplicate_active_tab(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::CloseOtherTabs,
            "Close Other Tabs",
            "Layout",
            always,
            None,
            CommandIcon::Close,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.close_other_tabs(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::NextTab,
            "Next Tab",
            "Tab Actions",
            always,
            Some(labonair_keymap::ShortcutId::TabNext),
            CommandIcon::ChevronRight,
            None,
        ),
        |s, window, cx| {
            s.workspace.update(cx, |w, cx| w.cycle(true, window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::PrevTab,
            "Previous Tab",
            "Tab Actions",
            always,
            Some(labonair_keymap::ShortcutId::TabPrev),
            CommandIcon::ChevronRight,
            None,
        ),
        |s, window, cx| {
            s.workspace.update(cx, |w, cx| w.cycle(false, window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::FocusNextPane,
            "Focus Next Pane",
            "Layout",
            always,
            None,
            CommandIcon::ChevronRight,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.focus_next_pane(window, cx));
        },
    );
    for (id, idx, title, shortcut) in [
        (
            CommandId::SelectTab1,
            0usize,
            "Select Tab 1",
            labonair_keymap::ShortcutId::TabSelect1,
        ),
        (
            CommandId::SelectTab2,
            1,
            "Select Tab 2",
            labonair_keymap::ShortcutId::TabSelect2,
        ),
        (
            CommandId::SelectTab3,
            2,
            "Select Tab 3",
            labonair_keymap::ShortcutId::TabSelect3,
        ),
        (
            CommandId::SelectTab4,
            3,
            "Select Tab 4",
            labonair_keymap::ShortcutId::TabSelect4,
        ),
        (
            CommandId::SelectTab5,
            4,
            "Select Tab 5",
            labonair_keymap::ShortcutId::TabSelect5,
        ),
        (
            CommandId::SelectTab6,
            5,
            "Select Tab 6",
            labonair_keymap::ShortcutId::TabSelect6,
        ),
        (
            CommandId::SelectTab7,
            6,
            "Select Tab 7",
            labonair_keymap::ShortcutId::TabSelect7,
        ),
        (
            CommandId::SelectTab8,
            7,
            "Select Tab 8",
            labonair_keymap::ShortcutId::TabSelect8,
        ),
        (
            CommandId::SelectTab9,
            8,
            "Select Tab 9",
            labonair_keymap::ShortcutId::TabSelect9,
        ),
    ] {
        r.register(
            command_descriptor(
                id,
                title,
                "Tab Actions",
                always,
                Some(shortcut),
                CommandIcon::Terminal,
                None,
            ),
            move |s, window, cx| {
                s.workspace
                    .update(cx, |w, cx| w.select_tab_by_index(idx, window, cx));
            },
        );
    }

    // ── Terminal panes ──────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::SplitRight,
            "Split Pane Right",
            "Layout",
            CTX_TERMINAL,
            Some(labonair_keymap::ShortcutId::PaneSplitRight),
            CommandIcon::ChevronRight,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.split(SplitDirection::Right, window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::SplitDown,
            "Split Pane Down",
            "Layout",
            CTX_TERMINAL,
            Some(labonair_keymap::ShortcutId::PaneSplitDown),
            CommandIcon::ChevronDown,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.split(SplitDirection::Down, window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::ClosePane,
            "Close Active Pane",
            "Layout",
            CTX_TERMINAL,
            Some(labonair_keymap::ShortcutId::PaneClose),
            CommandIcon::Close,
            None,
        ),
        |s, window, cx| {
            s.workspace.update(cx, |w, cx| w.close_pane(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::ClearTerminal,
            "Clear Terminal",
            "Terminal",
            CTX_TERMINALS,
            None,
            CommandIcon::Trash,
            None,
        ),
        |s, _window, cx| {
            s.workspace.update(cx, |w, cx| w.clear_active_terminal(cx));
        },
    );

    // ── Search ──────────────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::Find,
            "Find in Current Pane",
            "Search",
            always,
            Some(labonair_keymap::ShortcutId::SearchFocus),
            CommandIcon::Search,
            None,
        ),
        |s, window, cx| {
            s.toggle_search_overlay(window, cx);
        },
    );

    // ── Connections ─────────────────────────────────────────────────────
    // Connecting is exclusively the command palette's Hosts page
    // (`Enter` = SSH, `Shift+Enter` = SFTP). Host management is no longer a
    // Hosts capability surface; this command opens its canonical selection
    // page and does not create a Settings category.
    r.register(
        command_descriptor(
            CommandId::OpenHostSettings,
            "Open Hosts",
            "Connections",
            always,
            None,
            CommandIcon::Server,
            None,
        ),
        |s, window, cx| {
            s.show_command_palette(Some(PalettePage::Hosts), window, cx);
        },
    );
    for (id, title, icon) in [
        (CommandId::NewSshTab, "New SSH Tab", CommandIcon::Terminal),
        (CommandId::NewSftpTab, "New SFTP Tab", CommandIcon::Folder),
        (
            CommandId::NewQuickSsh,
            "New Quick SSH",
            CommandIcon::Terminal,
        ),
        (
            CommandId::NewSshConnection,
            "New SSH Connection",
            CommandIcon::Terminal,
        ),
    ] {
        r.register(
            command_descriptor(
                id,
                title,
                "Connections",
                always,
                None,
                icon,
                Some(CommandSubmenu::Hosts),
            ),
            |s, window, cx| {
                s.show_command_palette(Some(PalettePage::Hosts), window, cx);
            },
        );
    }

    // ── View / sidebar ─────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::ToggleSidebar,
            "Toggle File Explorer",
            "View",
            always,
            Some(labonair_keymap::ShortcutId::SidebarToggle),
            CommandIcon::PanelLeft,
            None,
        ),
        |s, _window, cx| {
            s.toggle_sidebar(cx);
        },
    );
    r.register(
        command_descriptor(
            CommandId::DebugCyclePanelDock,
            "Debug: Cycle Panel Dock",
            "Application",
            always,
            None,
            CommandIcon::PanelLeft,
            None,
        ),
        |s, _window, cx| {
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
        },
    );
    r.register(
        command_descriptor(
            CommandId::DebugToggleDockZoom,
            "Debug: Toggle Dock Zoom",
            "Application",
            always,
            None,
            CommandIcon::Square,
            None,
        ),
        |s, _window, cx| {
            let pos = s.primary_dock(cx);
            s.workspace.update(cx, |w, cx| {
                let z = w.dock(pos).is_zoomed();
                w.dock_mut(pos).set_zoomed(!z);
                w.persist_docks(cx);
            });
            cx.notify();
        },
    );

    // ── Snippets / source control ──────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::OpenSnippetsPanel,
            "Open Snippets Panel",
            "Snippets",
            always,
            None,
            CommandIcon::Command,
            None,
        ),
        |s, _window, cx| {
            s.open_panel("snippets", cx);
        },
    );
    r.register(
        command_descriptor(
            CommandId::OpenGitGraph,
            "Open Git Graph",
            "Source Control",
            always,
            None,
            CommandIcon::GitBranch,
            None,
        ),
        |s, _window, cx| {
            s.workspace.update(cx, |w, cx| w.open_git_graph_tab(cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::FocusSourceControl,
            "Focus Source Control",
            "Source Control",
            always,
            None,
            CommandIcon::GitBranch,
            None,
        ),
        |s, _window, cx| {
            s.open_panel("source-control", cx);
        },
    );

    // ── Palette ────────────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::OpenCommandPalette,
            "Open Command Palette",
            "Application",
            always,
            Some(labonair_keymap::ShortcutId::CommandPalette),
            CommandIcon::Command,
            None,
        ),
        |s, window, cx| {
            s.toggle_command_palette(window, cx);
        },
    );

    // ── Zen-mode / settings toggles ────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::ToggleZenMode,
            "Toggle: Zen Mode",
            "Settings",
            always,
            Some(labonair_keymap::ShortcutId::ViewZenMode),
            CommandIcon::Eye,
            None,
        ),
        |s, _window, cx| {
            s.toggle_zen_mode(cx);
        },
    );
    for (id, key, ctx, title, icon) in [
        (
            CommandId::ToggleZenModeHeader,
            "zenModeShowHeader",
            ALWAYS,
            "Toggle: Show Header Bar",
            CommandIcon::Eye,
        ),
        (
            CommandId::ToggleZenModeStatusbar,
            "zenModeShowStatusbar",
            ALWAYS,
            "Toggle: Show Status Bar",
            CommandIcon::Eye,
        ),
        (
            CommandId::ToggleEditorWordWrap,
            "editorWordWrap",
            CTX_EDITOR,
            "Toggle: Editor Word Wrap",
            CommandIcon::ChevronDown,
        ),
        (
            CommandId::ToggleLineNumbers,
            "editorLineNumbers",
            CTX_EDITOR,
            "Toggle: Line Numbers",
            CommandIcon::Check,
        ),
        (
            CommandId::ToggleFormatOnSave,
            "editorFormatOnSave",
            CTX_EDITOR,
            "Toggle: Format on Save",
            CommandIcon::Check,
        ),
        (
            CommandId::ToggleCursorBlink,
            "terminalCursorBlink",
            CTX_TERMINAL,
            "Toggle: Terminal Cursor Blink",
            CommandIcon::Eye,
        ),
        (
            CommandId::TogglePaneHeader,
            "terminalShowPaneHeader",
            CTX_TERMINAL,
            "Toggle: Terminal Pane Header",
            CommandIcon::PanelTop,
        ),
        (
            CommandId::TogglePaneFooter,
            "terminalShowPaneFooter",
            CTX_TERMINAL,
            "Toggle: Terminal Pane Footer",
            CommandIcon::PanelBottom,
        ),
        (
            CommandId::ToggleVimMode,
            "vimMode",
            ALWAYS,
            "Toggle: Vim Mode",
            CommandIcon::Check,
        ),
    ] {
        r.register(
            command_descriptor(id, title, "Settings", ctx, None, icon, None),
            move |s, _window, cx| s.toggle_zen_pref(key, cx),
        );
    }

    // ── Application ────────────────────────────────────────────────────
    r.register(
        command_descriptor(
            CommandId::OpenProject,
            "Open Project…",
            "Workspace",
            always,
            None,
            CommandIcon::Folder,
            None,
        ),
        |s, _window, cx| {
            s.workspace.update(cx, |w, cx| w.request_open_project(cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::OpenSettings,
            "Open Settings",
            "Application",
            always,
            None,
            CommandIcon::Edit,
            None,
        ),
        |_s, _window, cx| {
            open_settings_window(None, cx);
        },
    );
    r.register(
        command_descriptor(
            CommandId::ReturnToStandalone,
            "Return to Standalone",
            "Workspace",
            always,
            None,
            CommandIcon::Folder,
            None,
        ),
        |s, _window, cx| {
            s.workspace.update(cx, |w, cx| {
                w.apply_transition(
                    labonair_workspace::context::WorkspaceTransition::ReturnToStandalone,
                    cx,
                );
            });
        },
    );
    r.register(
        command_descriptor(
            CommandId::OpenProjectSettings,
            "Open Project Settings (.labonair/settings.json)",
            "Application",
            always,
            None,
            CommandIcon::Edit,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.open_or_create_project_settings(window, cx));
        },
    );
    r.register(
        command_descriptor(
            CommandId::OpenSettingsJson,
            "Open Settings (JSON)",
            "Application",
            always,
            None,
            CommandIcon::Edit,
            None,
        ),
        |s, window, cx| {
            s.workspace
                .update(cx, |w, cx| w.open_or_create_user_settings_json(window, cx));
        },
    );
    let keymap_descriptor = command_descriptor(
        CommandId::OpenKeymapJson,
        "Open Keymap (JSON)",
        "Keymap",
        always,
        Some(labonair_keymap::ShortcutId::ShortcutsOpen),
        CommandIcon::Edit,
        None,
    )
    .with_default_binding("cmd-shift-/", None);
    r.register(keymap_descriptor, |s, window, cx| {
        s.workspace
            .update(cx, |w, cx| w.open_or_create_user_keymap_json(window, cx));
    });
    r.register(
        command_descriptor(
            CommandId::CheckForUpdates,
            "Check for Updates…",
            "Application",
            always,
            None,
            CommandIcon::Download,
            None,
        ),
        |s, _window, cx| {
            s.panels.updater.update(cx, |u, cx| u.run_check(true, cx));
        },
    );

    // T20-004: debug-only — open the ui-kit component gallery in its own
    // window. The palette row (`Debug: Open Component Gallery`) and the
    // `gallery.rs` view are both compiled out of release builds too.
    #[cfg(debug_assertions)]
    r.register(
        command_descriptor(
            CommandId::OpenComponentGallery,
            "Debug: Open Component Gallery",
            "Application",
            always,
            None,
            CommandIcon::Palette,
            None,
        ),
        |_s, _window, cx| {
            labonair_ui_kit::open_gallery_window(cx);
        },
    );

    // Native-window command: the shell owns both metadata and execution
    // because the action targets GPUI's Window directly.
    r.register(
        command_descriptor(
            CommandId::ToggleFullScreen,
            "Toggle Full Screen",
            "View",
            always,
            None,
            CommandIcon::Square,
            None,
        ),
        |_s, window, _cx| window.toggle_fullscreen(),
    );

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

    #[test]
    fn project_lifecycle_commands_share_the_typed_registry_path() {
        let r = register_builtin_commands();
        let descriptors = r.iter().collect::<Vec<_>>();

        let open = descriptors
            .iter()
            .find(|command| command.id == CommandId::OpenProject)
            .expect("open-project command must be registered");
        let standalone = descriptors
            .iter()
            .find(|command| command.id == CommandId::ReturnToStandalone)
            .expect("return-to-standalone command must be registered");

        assert_eq!(open.id.action_name(), "workspace::OpenProject");
        assert_eq!(standalone.id.action_name(), "workspace::ReturnToStandalone");
        assert!(r.run_for(CommandId::OpenProject).is_some());
        assert!(r.run_for(CommandId::ReturnToStandalone).is_some());
    }
}
