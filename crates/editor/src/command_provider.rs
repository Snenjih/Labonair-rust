//! Editor-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct EditorCommandProvider;

impl CommandProvider for EditorCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::FormatDocument, "Format Document", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(CommandId::GoToSymbol, "Go to Symbol…", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_icon(CommandIcon::FileCode)
                .with_submenu(CommandSubmenu::Outline),
        ]
    }
}
