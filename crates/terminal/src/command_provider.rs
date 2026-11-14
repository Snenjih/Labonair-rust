//! Terminal-owned command metadata and executable contributions.

use std::rc::Rc;

use gpui::App;

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSecondary, SubmenuSnapshot,
};
use labonair_command_palette_runtime::CommandHandlerRegistry;

/// The narrow workspace operation required by the terminal command surface.
///
/// The terminal module owns the command ID and its meaning; the workspace
/// supplies the active-pane target because it owns tab/pane orchestration.
pub trait TerminalCommandTarget {
    fn clear_active_terminal(&self, cx: &mut App);
    fn copy_selection(&self, cx: &mut App);
    fn paste(&self, cx: &mut App);
    fn scroll_to_top(&self, cx: &mut App);
    fn scroll_to_bottom(&self, cx: &mut App);
    fn reset(&self, cx: &mut App);
    fn scroll_page_up(&self, cx: &mut App);
    fn scroll_page_down(&self, cx: &mut App);
    fn scroll_line_up(&self, cx: &mut App);
    fn scroll_line_down(&self, cx: &mut App);
    /// Paste without bracketed-paste wrapping.
    fn paste_text(&self, cx: &mut App);
    fn rename(&self, window: &mut gpui::Window, cx: &mut App);
    /// Open every unclaimed session of the detached session host as a tab.
    fn recover_background_sessions(&self, window: &mut gpui::Window, cx: &mut App);
    /// Stop the detached session host with all its sessions, then quit.
    fn quit_and_stop_all_sessions(&self, cx: &mut App);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TerminalCommandProvider;

fn terminal_command(id: CommandId, title: &str, icon: CommandIcon) -> CommandDescriptor {
    CommandDescriptor::new(id, title, "Terminal")
        .with_contexts(&[CommandContext::Terminal, CommandContext::SshTerminal])
        .with_icon(icon)
}

impl CommandProvider for TerminalCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            terminal_command(
                CommandId::ClearTerminal,
                "Clear Terminal",
                CommandIcon::Trash,
            ),
            terminal_command(
                CommandId::TerminalReset,
                "Reset Terminal",
                CommandIcon::Refresh,
            ),
            terminal_command(
                CommandId::TerminalCopySelection,
                "Copy Selection",
                CommandIcon::Copy,
            ),
            terminal_command(CommandId::TerminalPaste, "Paste", CommandIcon::Download),
            terminal_command(
                CommandId::TerminalScrollToTop,
                "Scroll to Top",
                CommandIcon::PanelTop,
            ),
            terminal_command(
                CommandId::TerminalScrollToBottom,
                "Scroll to Bottom",
                CommandIcon::PanelBottom,
            ),
            terminal_command(
                CommandId::TerminalScrollPageUp,
                "Scroll Page Up",
                CommandIcon::PanelTop,
            ),
            terminal_command(
                CommandId::TerminalScrollPageDown,
                "Scroll Page Down",
                CommandIcon::PanelBottom,
            ),
            terminal_command(
                CommandId::TerminalScrollLineUp,
                "Scroll Line Up",
                CommandIcon::PanelTop,
            ),
            terminal_command(
                CommandId::TerminalScrollLineDown,
                "Scroll Line Down",
                CommandIcon::PanelBottom,
            ),
            terminal_command(
                CommandId::TerminalPasteText,
                "Paste Without Bracketed Paste",
                CommandIcon::Download,
            ),
            terminal_command(
                CommandId::TerminalShowCharacterPalette,
                "Show Character Palette",
                CommandIcon::Terminal,
            ),
            terminal_command(
                CommandId::TerminalRename,
                "Rename Terminal",
                CommandIcon::Terminal,
            ),
            // Global (no terminal context): usable with no terminal focused.
            CommandDescriptor::new(
                CommandId::TerminalRecoverBackgroundSessions,
                "Recover Background Sessions",
                "Terminal",
            )
            .with_icon(CommandIcon::Terminal),
            CommandDescriptor::new(
                CommandId::TerminalQuitAndStopAllSessions,
                "Quit and Stop All Sessions",
                "Terminal",
            )
            .with_icon(CommandIcon::Terminal),
            CommandDescriptor::new(
                CommandId::TerminalManage,
                "See / Manage Terminals",
                "Terminal",
            )
            .with_icon(CommandIcon::Terminal)
            .with_submenu(CommandSubmenu::Terminals),
        ]
    }
}

/// Palette rows for the terminals of the background host. `tab` is the tab of
/// this window that shows the session, if any.
pub fn terminals_submenu(
    rows: impl IntoIterator<Item = (crate::BackgroundSession, Option<u64>)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("terminals", "Terminals", CommandSubmenu::Terminals),
        items: rows
            .into_iter()
            .map(|(session, tab)| {
                let title = session
                    .label
                    .clone()
                    .or_else(|| session.title.clone())
                    .unwrap_or_else(|| format!("Terminal {}", session.id));
                let action = match tab {
                    Some(tab) => SubmenuAction::SwitchToTab(tab),
                    None => SubmenuAction::OpenBackgroundTerminal(session.id),
                };
                SubmenuItem {
                    id: session.id.to_string(),
                    title,
                    subtitle: Some(terminal_subtitle(&session, tab.is_some())),
                    active: false,
                    action,
                    secondary: Some(SubmenuSecondary {
                        label: "Close".to_string(),
                        action: SubmenuAction::KillBackgroundTerminal(session.id),
                    }),
                }
            })
            .collect(),
    }
}

/// `cwd · foreground job · state` of one background terminal.
fn terminal_subtitle(session: &crate::BackgroundSession, open_here: bool) -> String {
    let state = if session.exited {
        "exited"
    } else if open_here {
        "open in a tab"
    } else if session.attached {
        "open in another window"
    } else {
        "detached"
    };
    [
        session.cwd.as_deref(),
        session.foreground.as_deref(),
        Some(state),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" \u{b7} ")
}

/// Register executable handlers owned by the terminal capability.
pub fn register_handlers(
    registry: &mut CommandHandlerRegistry,
    target: Rc<dyn TerminalCommandTarget>,
) {
    macro_rules! register {
        ($id:expr, $method:ident) => {{
            let target = target.clone();
            registry
                .register($id, move |_window, cx| target.$method(cx))
                .expect("terminal command handler must have a unique id");
        }};
    }
    register!(CommandId::ClearTerminal, clear_active_terminal);
    register!(CommandId::TerminalReset, reset);
    register!(CommandId::TerminalCopySelection, copy_selection);
    register!(CommandId::TerminalPaste, paste);
    register!(CommandId::TerminalScrollToTop, scroll_to_top);
    register!(CommandId::TerminalScrollToBottom, scroll_to_bottom);
    register!(CommandId::TerminalScrollPageUp, scroll_page_up);
    register!(CommandId::TerminalScrollPageDown, scroll_page_down);
    register!(CommandId::TerminalScrollLineUp, scroll_line_up);
    register!(CommandId::TerminalScrollLineDown, scroll_line_down);
    register!(CommandId::TerminalPasteText, paste_text);
    registry
        .register(CommandId::TerminalShowCharacterPalette, |window, _cx| {
            window.show_character_palette()
        })
        .expect("terminal command handler must have a unique id");
    let recover_target = target.clone();
    registry
        .register(
            CommandId::TerminalRecoverBackgroundSessions,
            move |window, cx| recover_target.recover_background_sessions(window, cx),
        )
        .expect("terminal command handler must have a unique id");
    let quit_target = target.clone();
    registry
        .register(
            CommandId::TerminalQuitAndStopAllSessions,
            move |_window, cx| quit_target.quit_and_stop_all_sessions(cx),
        )
        .expect("terminal command handler must have a unique id");
    let rename_target = target.clone();
    registry
        .register(CommandId::TerminalRename, move |window, cx| {
            rename_target.rename(window, cx)
        })
        .expect("terminal command handler must have a unique id");
}

#[cfg(test)]
mod terminals_submenu_tests {
    use super::*;
    use crate::BackgroundSession;

    fn session(id: u64, attached: bool, exited: bool) -> BackgroundSession {
        BackgroundSession {
            id,
            pane_tag: None,
            label: Some("Ciel".to_string()),
            cwd: Some("/work/Ciel".to_string()),
            title: None,
            exited,
            attached,
            foreground: Some("sleep".to_string()),
        }
    }

    #[test]
    fn open_here_switches_tab_and_detached_opens_a_tab() {
        let snapshot = terminals_submenu([
            (session(1, true, false), Some(7)),
            (session(2, false, false), None),
        ]);
        assert_eq!(snapshot.items[0].action, SubmenuAction::SwitchToTab(7));
        assert_eq!(
            snapshot.items[0].subtitle.as_deref(),
            Some("/work/Ciel \u{b7} sleep \u{b7} open in a tab")
        );
        assert_eq!(
            snapshot.items[1].action,
            SubmenuAction::OpenBackgroundTerminal(2)
        );
        assert_eq!(
            snapshot.items[1].secondary.as_ref().map(|s| &s.action),
            Some(&SubmenuAction::KillBackgroundTerminal(2))
        );
    }

    #[test]
    fn states_are_distinguishable() {
        let sub = |attached, exited, here| terminal_subtitle(&session(1, attached, exited), here);
        assert!(sub(false, true, false).ends_with("exited"));
        assert!(sub(true, false, false).ends_with("open in another window"));
        assert!(sub(false, false, false).ends_with("detached"));
    }
}
