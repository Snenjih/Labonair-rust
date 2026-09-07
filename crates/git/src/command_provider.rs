//! Git-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct GitCommandProvider;

impl CommandProvider for GitCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenGitGraph, "Open Git Graph", "Source Control")
                .with_icon(CommandIcon::GitBranch),
            CommandDescriptor::new(
                CommandId::FocusSourceControl,
                "Focus Source Control",
                "Source Control",
            )
            .with_icon(CommandIcon::GitBranch),
            CommandDescriptor::new(
                CommandId::GitSwitchBranch,
                "Git: Switch Branch…",
                "Source Control",
            )
            .with_icon(CommandIcon::GitBranch)
            .with_submenu(CommandSubmenu::GitBranches),
        ]
    }
}
