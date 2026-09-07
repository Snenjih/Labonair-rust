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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TerminalCommandProvider;

impl CommandProvider for TerminalCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::ClearTerminal, "Clear Terminal", "Terminal")
                .with_contexts(&[CommandContext::Terminal, CommandContext::SshTerminal])
                .with_icon(CommandIcon::Trash),
        ]
    }
}

/// Register executable handlers owned by the terminal capability.
pub fn register_handlers(
    registry: &mut CommandHandlerRegistry,
    target: Rc<dyn TerminalCommandTarget>,
) {
    registry
        .register(CommandId::ClearTerminal, move |_window, cx| {
            target.clear_active_terminal(cx);
        })
        .expect("terminal command handler must have a unique id");
}
