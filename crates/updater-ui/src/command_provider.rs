//! Updater-owned command metadata and executable contributions.

use gpui::{App, Entity};
use labonair_command_palette_core::{CommandDescriptor, CommandIcon, CommandId, CommandProvider};
use labonair_command_palette_runtime::CommandHandlerRegistry;

use crate::UpdaterView;

#[derive(Clone, Copy, Debug, Default)]
pub struct UpdaterCommandProvider;

impl CommandProvider for UpdaterCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![CommandDescriptor::new(
            CommandId::CheckForUpdates,
            "Check for Updates…",
            "Application",
        )
        .with_icon(CommandIcon::Download)]
    }
}

/// Register the updater action against the owner entity supplied by startup.
pub fn register_handlers(registry: &mut CommandHandlerRegistry, updater: &Entity<UpdaterView>) {
    let updater = updater.clone();
    registry
        .register(CommandId::CheckForUpdates, move |_window, cx: &mut App| {
            updater.update(cx, |updater, cx| updater.run_check(true, cx));
        })
        .expect("updater command handler must have a unique id");
}
