//! Editor-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};
use labonair_interaction_contracts::ShortcutId;

#[derive(Clone, Copy, Debug, Default)]
pub struct EditorCommandProvider;

pub fn outline_submenu(rows: impl IntoIterator<Item = (usize, String, String)>) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("outline", "Symbols", CommandSubmenu::Outline),
        items: rows
            .into_iter()
            .map(|(line, title, subtitle)| SubmenuItem {
                id: line.to_string(),
                title,
                subtitle: Some(subtitle),
                active: false,
                action: SubmenuAction::GoToLine(line),
                secondary: None,
            })
            .collect(),
    }
}

impl CommandProvider for EditorCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::Find, "Find in Current Pane", "Search")
                .with_shortcut(ShortcutId::SearchFocus)
                .with_default_binding("cmd-f", None)
                .with_icon(CommandIcon::Search),
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
