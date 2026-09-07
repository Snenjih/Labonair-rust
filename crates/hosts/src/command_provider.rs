//! Hosts-owned command metadata.

use crate::HostPickerRow;

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
    rows: impl IntoIterator<Item = HostPickerRow>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(id, title, submenu),
        items: rows
            .into_iter()
            .map(|row| SubmenuItem {
                id: row.id.clone(),
                title: row.name,
                subtitle: Some(row.subtitle),
                active: false,
                action: SubmenuAction::ConnectHost {
                    host_id: row.id.clone(),
                    sftp: false,
                },
                secondary: Some(SubmenuSecondary {
                    label: "Open SFTP".to_string(),
                    action: SubmenuAction::ConnectHost {
                        host_id: row.id,
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
            CommandDescriptor::new(CommandId::OpenHosts, "Open Hosts", "Connections")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_rows_keep_canonical_identity_and_sftp_secondary_action() {
        let snapshot = host_submenu(
            "hosts",
            "Hosts",
            CommandSubmenu::Hosts,
            [HostPickerRow {
                id: "prod".to_string(),
                name: "Production".to_string(),
                subtitle: "deploy@prod.example.com:22".to_string(),
            }],
        );
        let row = &snapshot.items[0];

        assert_eq!(row.title, "Production");
        assert_eq!(row.subtitle.as_deref(), Some("deploy@prod.example.com:22"));
        assert_eq!(
            row.action,
            SubmenuAction::ConnectHost {
                host_id: "prod".to_string(),
                sftp: false,
            }
        );
        assert_eq!(
            row.secondary.as_ref().map(|secondary| &secondary.action),
            Some(&SubmenuAction::ConnectHost {
                host_id: "prod".to_string(),
                sftp: true,
            })
        );
    }
}
