//! UI-facing data contract for keymap management.
//!
//! The management surface is a presentation adapter, not a second keymap
//! implementation. This module joins command-owner metadata with the
//! keymap-owned effective bindings and lossless user document so a UI can
//! render searchable rows, diagnostics, and an escape hatch to raw JSONC
//! without owning persistence or command discovery.

use std::collections::BTreeSet;

use labonair_command_palette_core::{
    compatibility_action_names, CommandContext, CommandDescriptor, CommandId,
};

use crate::{
    adapter::KeymapSnapshot,
    file::{EffectiveBinding, KeybindSource, KeymapDocument, ValidationIssue},
    runtime::{self, KeymapBinding},
};

/// One command row in the keymap management surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapCommandRow {
    pub command: CommandId,
    pub title: String,
    pub section: String,
    pub contexts: Vec<CommandContext>,
    pub default_bindings: Vec<KeymapBinding>,
    pub effective_bindings: Vec<EffectiveBinding>,
}

/// Immutable view model assembled by the keymap owner for its UI adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapManagementSnapshot {
    pub rows: Vec<KeymapCommandRow>,
    pub document: KeymapDocument,
    pub issues: Vec<ValidationIssue>,
}

impl KeymapManagementSnapshot {
    /// Return rows whose command title, section, action name, or binding
    /// contains `query`, preserving registry order for stable keyboard use.
    pub fn search(&self, query: &str) -> Vec<&KeymapCommandRow> {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return self.rows.iter().collect();
        }

        self.rows
            .iter()
            .filter(|row| {
                let action = row.command.action_name().to_lowercase();
                let metadata = row
                    .effective_bindings
                    .iter()
                    .map(|binding| format!("{} {}", binding.keystrokes, binding.action))
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                row.title.to_lowercase().contains(&query)
                    || row.section.to_lowercase().contains(&query)
                    || action.contains(&query)
                    || metadata.contains(&query)
            })
            .collect()
    }
}

/// Build the management snapshot from one command registry snapshot, the
/// already-loaded runtime result, and the lossless user document.
pub fn snapshot<'a>(
    descriptors: impl IntoIterator<Item = &'a CommandDescriptor>,
    runtime: &KeymapSnapshot,
    document: KeymapDocument,
) -> KeymapManagementSnapshot {
    let descriptors = descriptors.into_iter().collect::<Vec<_>>();
    let rows = descriptors
        .iter()
        .map(|descriptor| KeymapCommandRow {
            command: descriptor.id,
            title: descriptor.title.clone(),
            section: descriptor.section.clone(),
            contexts: descriptor.contexts.clone(),
            default_bindings: descriptor
                .default_bindings
                .iter()
                .map(|binding| KeymapBinding {
                    keystrokes: binding.keystrokes.clone(),
                    command: descriptor.id,
                    context: binding.context,
                })
                .collect(),
            effective_bindings: runtime
                .effective_bindings
                .iter()
                .filter(|binding| {
                    runtime::command_for_action(&binding.action) == Some(descriptor.id)
                })
                .cloned()
                .collect(),
        })
        .collect();

    KeymapManagementSnapshot {
        rows,
        document,
        issues: runtime.issues.clone(),
    }
}

/// Build the known-action vocabulary used by the lossless document parser.
/// Compatibility aliases remain accepted for migration but are not added as
/// separate management rows.
pub fn known_actions<'a>(
    descriptors: impl IntoIterator<Item = &'a CommandDescriptor>,
) -> BTreeSet<&'a str> {
    let mut actions = descriptors
        .into_iter()
        .map(|descriptor| descriptor.id.action_name())
        .collect::<BTreeSet<_>>();
    actions.extend(compatibility_action_names());
    actions
}

/// Whether a binding is supplied by the user layer. Kept as a helper so the
/// UI does not compare enum values or duplicate source semantics.
pub fn is_user_binding(binding: &EffectiveBinding) -> bool {
    binding.source == KeybindSource::User
}

#[cfg(test)]
mod tests {
    use super::*;
    use labonair_command_palette_core::CommandContext;

    fn document() -> KeymapDocument {
        KeymapDocument::from_source("[]", &BTreeSet::new())
    }

    #[test]
    fn snapshot_groups_effective_bindings_by_typed_command() {
        let descriptor = CommandDescriptor::new(CommandId::Find, "Find", "Search")
            .with_default_binding("cmd-f", Some(CommandContext::Editor));
        let runtime = KeymapSnapshot {
            effective_bindings: vec![EffectiveBinding {
                keystrokes: "cmd-f".into(),
                action: CommandId::Find.action_name().into(),
                context: Some("Editor".into()),
                source: KeybindSource::Default,
            }],
            issues: Vec::new(),
        };

        let snapshot = snapshot([&descriptor], &runtime, document());

        assert_eq!(snapshot.rows.len(), 1);
        assert_eq!(snapshot.rows[0].command, CommandId::Find);
        assert_eq!(snapshot.rows[0].effective_bindings.len(), 1);
        assert_eq!(snapshot.search("cmd-f").len(), 1);
    }

    #[test]
    fn search_is_case_insensitive_and_empty_query_preserves_order() {
        let first = CommandDescriptor::new(CommandId::Find, "Find", "Search");
        let second = CommandDescriptor::new(CommandId::OpenSettings, "Settings", "Application");
        let runtime = KeymapSnapshot::default();
        let snapshot = snapshot([&first, &second], &runtime, document());

        assert_eq!(
            snapshot.search("SETTINGS")[0].command,
            CommandId::OpenSettings
        );
        assert_eq!(
            snapshot
                .search("")
                .iter()
                .map(|row| row.command)
                .collect::<Vec<_>>(),
            vec![CommandId::Find, CommandId::OpenSettings]
        );
    }
}
