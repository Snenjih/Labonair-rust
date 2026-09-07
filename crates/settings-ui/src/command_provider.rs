//! Settings-UI-owned command execution contributions.

use gpui::App;
use labonair_command_palette_core::CommandId;
use labonair_command_palette_runtime::CommandHandlerRegistry;

/// Register the settings window entry point owned by this UI sibling.
pub fn register_handlers(registry: &mut CommandHandlerRegistry) {
    registry
        .register(CommandId::OpenSettings, |_window, cx: &mut App| {
            super::open_settings_window(None, cx);
        })
        .expect("settings UI command handler must have a unique id");
}
