//! Hosts-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct HostsCommandProvider;

impl CommandProvider for HostsCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenHostSettings, "Open Hosts", "Connections")
                .with_icon(CommandIcon::Server),
            CommandDescriptor::new(CommandId::NewSshTab, "New SSH Tab", "Connections")
                .with_icon(CommandIcon::Terminal)
                .with_submenu(CommandSubmenu::Hosts),
            CommandDescriptor::new(CommandId::NewSftpTab, "New SFTP Tab", "Connections")
                .with_icon(CommandIcon::Folder)
                .with_submenu(CommandSubmenu::Hosts),
            CommandDescriptor::new(CommandId::NewQuickSsh, "New Quick SSH", "Connections")
                .with_icon(CommandIcon::Terminal)
                .with_submenu(CommandSubmenu::Hosts),
            CommandDescriptor::new(
                CommandId::NewSshConnection,
                "New SSH Connection",
                "Connections",
            )
            .with_icon(CommandIcon::Terminal)
            .with_submenu(CommandSubmenu::Hosts),
            CommandDescriptor::new(CommandId::ConnectSsh, "Connect SSH…", "Connections")
                .with_icon(CommandIcon::Terminal)
                .with_submenu(CommandSubmenu::Hosts),
            CommandDescriptor::new(CommandId::OpenSftp, "Open SFTP…", "Connections")
                .with_icon(CommandIcon::Folder)
                .with_submenu(CommandSubmenu::Hosts),
        ]
    }
}
