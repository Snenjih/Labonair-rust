//! The `Onboarding: Open` command (replays every applicable step).

use std::rc::Rc;

use gpui::{App, Window};
use labonair_command_palette_core::{CommandDescriptor, CommandId, CommandProvider};
use labonair_command_palette_runtime::CommandHandlerRegistry;

/// Opens the onboarding card with every applicable step, completed or not.
pub type OpenHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, Default)]
pub struct OnboardingCommandProvider;

impl CommandProvider for OnboardingCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![CommandDescriptor::new(
            CommandId::OpenOnboarding,
            "Onboarding: Open",
            "Application",
        )]
    }
}

pub fn register_handlers(registry: &mut CommandHandlerRegistry, open: OpenHandler) {
    registry
        .register(CommandId::OpenOnboarding, move |window, cx: &mut App| {
            open(window, cx)
        })
        .expect("onboarding command handler must have a unique id");
}
