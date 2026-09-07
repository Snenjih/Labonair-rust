//! Snippets-owned command metadata.

use labonair_command_palette_core::{
    CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu, SubmenuAction,
    SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SnippetsCommandProvider;

pub fn snippets_submenu(
    rows: impl IntoIterator<Item = (String, String, String)>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("snippets", "Snippets", CommandSubmenu::Snippets),
        items: rows
            .into_iter()
            .map(|(id, title, mode)| SubmenuItem {
                action: SubmenuAction::RunSnippet(id.clone()),
                id,
                title,
                subtitle: Some(mode),
                active: false,
                secondary: None,
            })
            .collect(),
    }
}

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
