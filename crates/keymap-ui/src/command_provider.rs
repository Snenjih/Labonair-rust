//! Keymap-UI-owned command execution contributions.

use std::rc::Rc;

use gpui::{App, Window};
use labonair_command_palette_core::{CommandDescriptor, CommandId};
use labonair_command_palette_runtime::CommandHandlerRegistry;

pub type OpenRawHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Register the keymap management entry point.
///
/// The raw JSONC editor remains an injected composition callback because its
/// tab lifecycle belongs to Workspace. Keymap UI owns the management window
/// and the descriptor snapshot it presents.
pub fn register_handlers(
    registry: &mut CommandHandlerRegistry,
    descriptors: Vec<CommandDescriptor>,
    open_raw: OpenRawHandler,
) {
    registry
        .register(CommandId::OpenKeymapJson, move |_window, cx| {
            let descriptors = descriptors.clone();
            let open_raw = open_raw.clone();
            super::open_keymap_window(descriptors, move |window, cx| open_raw(window, cx), cx);
        })
        .expect("keymap UI command handler must have a unique id");
}
