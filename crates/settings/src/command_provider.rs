//! Settings-owned command metadata.

use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider,
};
use labonair_interaction_contracts::ShortcutId;

#[derive(Clone, Copy, Debug, Default)]
pub struct SettingsCommandProvider;

impl CommandProvider for SettingsCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        let commands = vec![
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
            CommandDescriptor::new(CommandId::ToggleZenMode, "Toggle: Zen Mode", "Settings")
                .with_shortcut(ShortcutId::ViewZenMode)
                .with_default_binding("cmd-shift-z", None)
                .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleZenModeHeader,
                "Toggle: Show Header Bar",
                "Settings",
            )
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleZenModeStatusbar,
                "Toggle: Show Status Bar",
                "Settings",
            )
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(
                CommandId::ToggleEditorWordWrap,
                "Toggle: Editor Word Wrap",
                "Settings",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_icon(CommandIcon::ChevronDown),
            CommandDescriptor::new(
                CommandId::ToggleLineNumbers,
                "Toggle: Line Numbers",
                "Settings",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_icon(CommandIcon::Check),
            CommandDescriptor::new(
                CommandId::ToggleCursorBlink,
                "Toggle: Terminal Cursor Blink",
                "Settings",
            )
            .with_contexts(&[CommandContext::Terminal])
            .with_icon(CommandIcon::Eye),
            CommandDescriptor::new(CommandId::ToggleVimMode, "Toggle: Vim Mode", "Settings")
                .with_icon(CommandIcon::Check),
        ];

        commands
    }
}
