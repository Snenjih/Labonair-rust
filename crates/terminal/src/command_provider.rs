//! Terminal-owned command metadata and executable contributions.

use std::rc::Rc;

use gpui::App;

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider,
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
        ]
    }
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
    let rename_target = target.clone();
    registry
        .register(CommandId::TerminalRename, move |window, cx| {
            rename_target.rename(window, cx)
        })
        .expect("terminal command handler must have a unique id");
}
