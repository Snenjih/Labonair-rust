//! Keymap-owned command metadata.

use labonair_command_palette_core::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};

/// The keymap capability owns the discoverable entry point for editing the
/// user's keymap. Opening the surface (a workspace tab) is an execution
/// concern of the tab owner (`labonair-workspace`); this crate only publishes
/// the command metadata. `CommandId::OpenKeymapJson` keeps its historical name
/// (and `zed::OpenKeymap` alias) for user-keymap-file compatibility.
#[derive(Clone, Copy, Debug, Default)]
pub struct KeymapCommandProvider;

impl CommandProvider for KeymapCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenKeymapJson, "Open Keymap", "Keymap")
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
        assert_eq!(
            command
                .default_bindings
                .first()
                .map(|b| b.keystrokes.as_str()),
            Some("cmd-shift-/")
        );
    }
}
