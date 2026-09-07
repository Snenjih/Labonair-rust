//! Executable command contributions owned by the command-palette module.
//!
//! The palette UI does not know how the application presents its modal. The
//! composition root injects that small host operation while this module keeps
//! ownership of the palette command identity and executable contribution.

use std::rc::Rc;

use gpui::{App, Window};
use labonair_command_palette_core::CommandId;
use labonair_command_palette_runtime::CommandHandlerRegistry;

/// Host operation used to show or hide the command-palette modal.
pub type ToggleHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Register command-palette-owned executable handlers.
pub fn register_handlers(registry: &mut CommandHandlerRegistry, toggle: ToggleHandler) {
    registry
        .register(CommandId::OpenCommandPalette, move |window, cx| {
            toggle(window, cx);
        })
        .expect("command palette handler must have a unique id");
}
