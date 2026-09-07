//! Keymap-owned command metadata.

use labonair_command_palette_core::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};

use crate::ShortcutId;

/// The keymap capability owns the discoverable entry point for editing the
/// user's keymap. The JSON editor itself remains an execution concern of the
/// host until its UI boundary is extracted.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeymapCommandProvider;

impl CommandProvider for KeymapCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenKeymapJson, "Open Keymap (JSON)", "Keymap")
                .with_shortcut(ShortcutId::ShortcutsOpen)
                .with_default_binding("cmd-shift-/", None)
                .with_icon(CommandIcon::Edit),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_provider_owns_keymap_editor_entrypoint() {
        let command = KeymapCommandProvider.commands()[0].clone();
        assert_eq!(command.id, CommandId::OpenKeymapJson);
        assert_eq!(command.shortcut, Some(ShortcutId::ShortcutsOpen));
    }
}
