//! Terminal-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider,
};

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
