//! Theme-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct ThemeCommandProvider;

impl CommandProvider for ThemeCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::ChangeAppTheme, "Change App Theme…", "View")
                .with_icon(CommandIcon::Sparkles)
                .with_submenu(CommandSubmenu::Themes),
            CommandDescriptor::new(CommandId::ChangeIconTheme, "Change Icon Theme…", "View")
                .with_icon(CommandIcon::Palette)
                .with_submenu(CommandSubmenu::IconThemes),
            CommandDescriptor::new(CommandId::ChangeColorMode, "Change Color Mode…", "View")
                .with_icon(CommandIcon::ChevronDown)
                .with_submenu(CommandSubmenu::ColorMode),
            CommandDescriptor::new(CommandId::ChangeEditorTheme, "Change Editor Theme…", "View")
                .with_contexts(&[CommandContext::Editor])
                .with_icon(CommandIcon::Sparkles)
                .with_submenu(CommandSubmenu::EditorTheme),
        ]
    }
}
