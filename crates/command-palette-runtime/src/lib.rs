//! Runtime command handlers contributed by capability owners.
//!
//! [`labonair_command_palette_core`] owns command metadata. This crate owns
//! the small GPUI-facing bridge for executable contributions, without knowing
//! the application shell or any product module.

use std::rc::Rc;

use gpui::{App, Window};
use labonair_command_palette_core::{CommandId, PaletteAction};

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

/// A callback supplied by one capability owner for its dynamic palette
/// actions. Returning `true` claims the action; returning `false` lets the
/// next owner inspect it.
pub type PaletteActionHandler = Rc<dyn Fn(&PaletteAction, &mut Window, &mut App) -> bool>;

#[derive(Clone)]
struct PaletteActionEntry {
    owner: String,
    handler: PaletteActionHandler,
}

/// Registry for executable dynamic-palette contributions.
///
/// The registry deliberately does not know about the shell or concrete
/// feature entities. Each owner registers one narrow matcher and captures
/// only its own entities/services. The composition root only assembles this
/// registry and dispatches an opaque [`PaletteAction`].
#[derive(Clone, Default)]
pub struct PaletteActionHandlerRegistry {
    entries: Vec<PaletteActionEntry>,
}

impl PaletteActionHandlerRegistry {
    /// Register one owner contribution. One entry per owner keeps the
    /// ownership boundary visible and rejects accidental duplicate assembly.
    pub fn register(
        &mut self,
        owner: impl Into<String>,
        handler: impl Fn(&PaletteAction, &mut Window, &mut App) -> bool + 'static,
    ) -> Result<(), String> {
        let owner = owner.into();
        if self.entries.iter().any(|entry| entry.owner == owner) {
            return Err(owner);
        }
        self.entries.push(PaletteActionEntry {
            owner,
            handler: Rc::new(handler),
        });
        Ok(())
    }

    /// Dispatch an action through owner contributions, returning whether one
    /// owner claimed it.
    pub fn dispatch(&self, action: &PaletteAction, window: &mut Window, cx: &mut App) -> bool {
        self.entries
            .iter()
            .any(|entry| (entry.handler)(action, window, cx))
    }

    /// Owner names are useful for composition diagnostics and tests.
    pub fn owners(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.owner.as_str())
    }
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

    #[test]
    fn palette_action_owners_are_unique_and_ordered() {
        let mut registry = PaletteActionHandlerRegistry::default();
        registry
            .register("unrelated", |_action, _window, _cx| false)
            .unwrap();
        registry
            .register("owner", |_action, _window, _cx| true)
            .unwrap();
        assert_eq!(
            registry.register("owner", |_action, _window, _cx| true),
            Err("owner".to_string())
        );
        assert_eq!(
            registry.owners().collect::<Vec<_>>(),
            ["unrelated", "owner"]
        );
    }
}
