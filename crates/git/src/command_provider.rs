//! Git-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu, SubmenuAction,
    SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct GitCommandProvider;

pub fn branches_submenu(rows: impl IntoIterator<Item = (String, bool, bool)>) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("git-branches", "Branches", CommandSubmenu::GitBranches),
        items: rows
            .into_iter()
            .map(|(name, current, remote)| SubmenuItem {
                id: name.clone(),
                title: name.clone(),
                subtitle: remote.then(|| "remote".to_string()),
                active: current,
                action: SubmenuAction::SwitchBranch(name),
                secondary: None,
            })
            .collect(),
    }
}

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
