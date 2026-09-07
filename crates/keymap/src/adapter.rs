//! Composition-facing loading contract for the keymap module.
//!
//! This adapter owns the translation from command metadata to a loaded
//! keymap snapshot. It is still UI-free: GPUI installation, display hints, and
//! file watching belong to the application adapter in `labonair-shell`.

use std::collections::BTreeSet;

use labonair_command_palette_core::{
    compatibility_action_names, CommandDescriptor, CommandRegistry,
};

use crate::file::{self, EffectiveBinding, ValidationIssue};

/// Immutable result of one keymap load/reload.
#[derive(Clone, Debug, Default)]
pub struct KeymapSnapshot {
    pub effective_bindings: Vec<EffectiveBinding>,
    pub issues: Vec<ValidationIssue>,
}

/// Load a keymap from the command registry's current descriptor snapshot.
///
/// Feature modules own command descriptors and their optional defaults. The
/// keymap module owns vocabulary validation, default/user layer composition,
/// and recovery from malformed user files.
pub fn load(registry: &CommandRegistry) -> KeymapSnapshot {
    load_descriptors(registry.iter())
}

/// Load a keymap from descriptors supplied by a composition root. Keeping the
/// input as an iterator avoids coupling this contract to a shell dispatcher or
/// another concrete registry wrapper.
pub fn load_descriptors<'a>(
    descriptors: impl IntoIterator<Item = &'a CommandDescriptor>,
) -> KeymapSnapshot {
    let descriptors = descriptors.into_iter().cloned().collect::<Vec<_>>();
    let mut known_actions = descriptors
        .iter()
        .map(|descriptor| descriptor.id.action_name())
        .collect::<BTreeSet<_>>();
    known_actions.extend(compatibility_action_names());

    let owner_defaults = crate::runtime::defaults_from_descriptors(descriptors.iter());
    let effective_bindings =
        file::effective_bindings_with_defaults(&known_actions, &owner_defaults);
    let issues = file::last_issues();

    KeymapSnapshot {
        effective_bindings,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_command_palette_core::{CommandContext, CommandId};

    #[test]
    fn load_uses_descriptor_defaults_and_returns_issues() {
        let descriptor = CommandDescriptor::new(CommandId::Find, "Find", "Search")
            .with_default_binding("cmd-f", Some(CommandContext::Editor));
        let snapshot = load_descriptors([&descriptor]);

        assert!(snapshot.issues.is_empty());
        assert!(snapshot.effective_bindings.iter().any(|binding| {
            binding.action == "search::Toggle"
                && binding.keystrokes == "cmd-f"
                && binding.context.as_deref() == Some("Editor")
        }));
    }
}
