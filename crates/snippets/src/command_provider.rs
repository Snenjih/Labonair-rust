//! Snippets-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SnippetsCommandProvider;

impl CommandProvider for SnippetsCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(
                CommandId::OpenSnippetsPanel,
                "Open Snippets Panel",
                "Snippets",
            )
            .with_icon(CommandIcon::Command),
            CommandDescriptor::new(CommandId::RunSnippet, "Run Snippet…", "Snippets")
                .with_icon(CommandIcon::Command)
                .with_submenu(CommandSubmenu::Snippets),
        ]
    }
}
