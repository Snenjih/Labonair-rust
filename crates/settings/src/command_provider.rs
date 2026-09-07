//! Settings-owned command metadata.

use labonair_command_palette_core::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};

#[derive(Clone, Copy, Debug, Default)]
pub struct SettingsCommandProvider;

impl CommandProvider for SettingsCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenSettings, "Open Settings", "Application")
                .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(
                CommandId::OpenProjectSettings,
                "Open Project Settings (.labonair/settings.json)",
                "Application",
            )
            .with_icon(CommandIcon::Edit),
            CommandDescriptor::new(
                CommandId::OpenSettingsJson,
                "Open Settings (JSON)",
                "Application",
            )
            .with_icon(CommandIcon::Edit),
        ]
    }
}
