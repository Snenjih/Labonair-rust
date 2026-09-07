//! Workspace-owned command metadata.
//!
//! The workspace capability contributes its discoverable commands without
//! importing the palette UI or the application shell. Execution is still
//! connected by the composition root until command handlers move behind
//! capability-owned action services.

use labonair_command_palette_core::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};

/// Stable command metadata contributed by the workspace capability.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkspaceCommandProvider;

impl CommandProvider for WorkspaceCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenProject, "Open Project…", "Workspace")
                .with_icon(CommandIcon::Folder),
            CommandDescriptor::new(
                CommandId::ReturnToStandalone,
                "Return to Standalone",
                "Workspace",
            )
            .with_icon(CommandIcon::Folder),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_provider_owns_project_lifecycle_commands() {
        let commands = WorkspaceCommandProvider.commands();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].id, CommandId::OpenProject);
        assert_eq!(commands[1].id, CommandId::ReturnToStandalone);
        assert!(commands
            .iter()
            .all(|command| command.section == "Workspace"));
    }
}
