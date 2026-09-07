//! Command-palette-owned command metadata.

use crate::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};
use labonair_interaction_contracts::ShortcutId;

#[derive(Clone, Copy, Debug, Default)]
pub struct CommandPaletteCommandProvider;

impl CommandProvider for CommandPaletteCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![CommandDescriptor::new(
            CommandId::OpenCommandPalette,
            "Open Command Palette",
            "Application",
        )
        .with_shortcut(ShortcutId::CommandPalette)
        .with_default_binding("cmd-p", None)
        .with_icon(CommandIcon::Command)]
    }
}
