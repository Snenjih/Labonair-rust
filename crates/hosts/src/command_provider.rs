//! Hosts-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu, SubmenuAction,
    SubmenuDescriptor, SubmenuItem, SubmenuSecondary, SubmenuSnapshot,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct HostsCommandProvider;

/// Build a host picker snapshot. SSH is the primary action and SFTP is the
/// typed `Shift+Enter` secondary action for every row.
pub fn host_submenu(
    id: &str,
    title: &str,
    submenu: CommandSubmenu,
    rows: impl IntoIterator<Item = (String, String)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(id, title, submenu),
        items: rows
            .into_iter()
            .map(|(host_id, host_name)| SubmenuItem {
                id: host_id.clone(),
                title: host_name,
                subtitle: None,
                active: false,
                action: SubmenuAction::ConnectHost {
                    host_id: host_id.clone(),
                    sftp: false,
                },
                secondary: Some(SubmenuSecondary {
                    label: "Open SFTP".to_string(),
                    action: SubmenuAction::ConnectHost {
                        host_id,
                        sftp: true,
                    },
                }),
            })
            .collect(),
    }
}

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
