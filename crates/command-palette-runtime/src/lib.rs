//! Runtime command handlers contributed by capability owners.
//!
//! [`labonair_command_palette_core`] owns command metadata. This crate owns
//! the small GPUI-facing bridge for executable contributions, without knowing
//! the application shell or any product module.

use std::rc::Rc;

use gpui::{App, Window};
use labonair_command_palette_core::CommandId;

/// A main-thread command callback. Owner crates capture only the entities and
/// services they need; the shell supplies the active window and application.
pub type CommandHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
struct Entry {
    id: CommandId,
    handler: CommandHandler,
}

/// Registry for executable command contributions.
#[derive(Clone, Default)]
pub struct CommandHandlerRegistry {
    entries: Vec<Entry>,
}

impl CommandHandlerRegistry {
    /// Register one owner-provided handler. Duplicate IDs are rejected so a
    /// command cannot silently acquire two different behaviors.
    pub fn register(
        &mut self,
        id: CommandId,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Result<(), CommandId> {
        if self.entries.iter().any(|entry| entry.id == id) {
            return Err(id);
        }
        self.entries.push(Entry {
            id,
            handler: Rc::new(handler),
        });
        Ok(())
    }

    /// Find an executable handler by its stable command ID.
    pub fn handler(&self, id: CommandId) -> Option<CommandHandler> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.handler.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_handlers_are_rejected() {
        let mut registry = CommandHandlerRegistry::default();
        registry
            .register(CommandId::NewTerminalTab, |_window, _cx| {})
            .unwrap();
        assert_eq!(
            registry
                .register(CommandId::NewTerminalTab, |_window, _cx| {})
                .unwrap_err(),
            CommandId::NewTerminalTab
        );
        assert!(registry.handler(CommandId::NewTerminalTab).is_some());
    }
}
