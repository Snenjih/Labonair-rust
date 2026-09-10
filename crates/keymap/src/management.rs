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

/// A conflict found against an already-effective binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingConflict {
    pub command: CommandId,
    pub title: String,
    pub keystrokes: String,
    pub context: Option<String>,
}

/// How a command's active binding relates to its shipped default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverrideState {
    /// The command resolves to exactly its shipped default (or has neither a
    /// default nor a user binding).
    Default,
    /// At least one active binding for the command comes from the user layer.
    Customized,
    /// The command ships a default but currently has no active binding, because
    /// the user removed it (or another command took the chord over).
    Unbound,
}

/// One default binding that is currently displaced because the same chord, in
/// the same context, now resolves to a different command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowedBinding {
    /// The displaced default keystrokes.
    pub keystrokes: String,
    /// Portable context label the collision happens in (`None` = global).
    pub context: Option<String>,
    /// The command the chord resolves to now.
    pub taken_by: CommandId,
    /// That command's title, for a ready-to-render message.
    pub taken_by_title: String,
}

/// Which rows the management surface shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RowFilter {
    /// Every command.
    #[default]
    All,
    /// Only commands with at least one user-layer binding.
    Customized,
    /// Only commands whose default is displaced by another command's override.
    Conflicts,
    /// Only commands that ship a default but currently have no active binding.
    Unbound,
}

/// Classify a command row against its shipped default.
pub fn row_override_state(row: &KeymapCommandRow) -> OverrideState {
    if row
        .effective_bindings
        .iter()
        .any(|binding| binding.source == KeybindSource::User)
    {
        return OverrideState::Customized;
    }
    if row.effective_bindings.is_empty() && !row.default_bindings.is_empty() {
        return OverrideState::Unbound;
    }
    OverrideState::Default
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

    /// Find the effective binding using `candidate` in the same portable
    /// context. Bindings for the command currently being edited are ignored,
    /// allowing a command to expose more than one deliberate key chord.
    pub fn conflict(
        &self,
        command: CommandId,
        context: Option<&str>,
        candidate: &str,
    ) -> Option<BindingConflict> {
        let candidate = crate::normalize_keystrokes(candidate);
        self.rows.iter().find_map(|row| {
            if row.command == command {
                return None;
            }
            row.effective_bindings.iter().find_map(|binding| {
                let same_context =
                    binding.context.as_deref().unwrap_or("").trim() == context.unwrap_or("").trim();
                (same_context && crate::normalize_keystrokes(&binding.keystrokes) == candidate)
                    .then(|| BindingConflict {
                        command: row.command,
                        title: row.title.clone(),
                        keystrokes: binding.keystrokes.clone(),
                        context: binding.context.clone(),
                    })
            })
        })
    }

    /// Like [`search`](Self::search), but also narrowed by `filter`. Registry
    /// order is preserved so keyboard navigation stays stable.
    pub fn search_filtered(&self, query: &str, filter: RowFilter) -> Vec<&KeymapCommandRow> {
        let conflicts = if filter == RowFilter::Conflicts {
            Some(self.conflict_commands())
        } else {
            None
        };
        self.search(query)
            .into_iter()
            .filter(|row| match filter {
                RowFilter::All => true,
                RowFilter::Customized => row_override_state(row) == OverrideState::Customized,
                RowFilter::Unbound => row_override_state(row) == OverrideState::Unbound,
                RowFilter::Conflicts => conflicts
                    .as_ref()
                    .is_some_and(|set| set.contains(&row.command)),
            })
            .collect()
    }

    /// Every command involved in a binding conflict — both the command whose
    /// default is displaced and the command that now owns the chord.
    pub fn conflict_commands(&self) -> std::collections::HashSet<CommandId> {
        let index = self.effective_index();
        let mut out = std::collections::HashSet::new();
        for row in &self.rows {
            for shadowed in self.shadowed_for(row, &index) {
                out.insert(row.command);
                out.insert(shadowed.taken_by);
            }
        }
        out
    }

    /// Whether any command's default binding is currently displaced.
    pub fn has_conflicts(&self) -> bool {
        let index = self.effective_index();
        self.rows
            .iter()
            .any(|row| !self.shadowed_for(row, &index).is_empty())
    }

    /// The default bindings of `command` that are currently displaced by a
    /// different command's override, if any.
    pub fn shadowed_defaults(&self, command: CommandId) -> Vec<ShadowedBinding> {
        let Some(row) = self.rows.iter().find(|row| row.command == command) else {
            return Vec::new();
        };
        self.shadowed_for(row, &self.effective_index())
    }

    /// `(normalized context, normalized chord) -> owning command` for every
    /// currently active binding.
    fn effective_index(&self) -> std::collections::HashMap<(String, String), CommandId> {
        let mut index = std::collections::HashMap::new();
        for row in &self.rows {
            for binding in &row.effective_bindings {
                index.insert(
                    (
                        normalized_context(binding.context.as_deref()),
                        crate::normalize_keystrokes(&binding.keystrokes),
                    ),
                    row.command,
                );
            }
        }
        index
    }

    fn shadowed_for(
        &self,
        row: &KeymapCommandRow,
        index: &std::collections::HashMap<(String, String), CommandId>,
    ) -> Vec<ShadowedBinding> {
        row.default_bindings
            .iter()
            .filter_map(|default| {
                let context_name = default.context.map(runtime::context_name);
                let key = (
                    normalized_context(context_name),
                    crate::normalize_keystrokes(&default.keystrokes),
                );
                // This command still owns the chord (directly, or via its own
                // override elsewhere) — not a conflict.
                let mine = row.effective_bindings.iter().any(|binding| {
                    normalized_context(binding.context.as_deref()) == key.0
                        && crate::normalize_keystrokes(&binding.keystrokes) == key.1
                });
                if mine {
                    return None;
                }
                match index.get(&key) {
                    Some(taken_by) if *taken_by != row.command => Some(ShadowedBinding {
                        keystrokes: default.keystrokes.clone(),
                        context: context_name.map(str::to_string),
                        taken_by: *taken_by,
                        taken_by_title: self
                            .rows
                            .iter()
                            .find(|other| other.command == *taken_by)
                            .map(|other| other.title.clone())
                            .unwrap_or_default(),
                    }),
                    _ => None,
                }
            })
            .collect()
    }
}

fn normalized_context(context: Option<&str>) -> String {
    context.unwrap_or("").trim().to_string()
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
    fn override_state_and_conflicts_track_the_user_layer() {
        let find = CommandDescriptor::new(CommandId::Find, "Find", "Search")
            .with_default_binding("cmd-f", Some(CommandContext::Editor));
        let settings = CommandDescriptor::new(CommandId::OpenSettings, "Settings", "Application");
        // The user rebound Find's default `cmd-f` to Settings.
        let runtime = KeymapSnapshot {
            effective_bindings: vec![EffectiveBinding {
                keystrokes: "cmd-f".into(),
                action: CommandId::OpenSettings.action_name().into(),
                context: Some("Editor".into()),
                source: KeybindSource::User,
            }],
            issues: Vec::new(),
        };
        let snap = snapshot([&find, &settings], &runtime, document());

        let find_row = snap
            .rows
            .iter()
            .find(|row| row.command == CommandId::Find)
            .unwrap();
        let settings_row = snap
            .rows
            .iter()
            .find(|row| row.command == CommandId::OpenSettings)
            .unwrap();
        assert_eq!(row_override_state(find_row), OverrideState::Unbound);
        assert_eq!(row_override_state(settings_row), OverrideState::Customized);

        assert!(snap.has_conflicts());
        let conflicts = snap.conflict_commands();
        assert!(conflicts.contains(&CommandId::Find));
        assert!(conflicts.contains(&CommandId::OpenSettings));

        let shadowed = snap.shadowed_defaults(CommandId::Find);
        assert_eq!(shadowed.len(), 1);
        assert_eq!(shadowed[0].taken_by, CommandId::OpenSettings);
        assert_eq!(shadowed[0].taken_by_title, "Settings");
        assert_eq!(shadowed[0].keystrokes, "cmd-f");

        assert_eq!(
            snap.search_filtered("", RowFilter::Customized)
                .iter()
                .map(|row| row.command)
                .collect::<Vec<_>>(),
            vec![CommandId::OpenSettings]
        );
        assert_eq!(snap.search_filtered("", RowFilter::Conflicts).len(), 2);
        assert_eq!(
            snap.search_filtered("", RowFilter::Unbound)
                .iter()
                .map(|row| row.command)
                .collect::<Vec<_>>(),
            vec![CommandId::Find]
        );
    }

    #[test]
    fn global_and_context_binding_for_one_chord_is_not_a_conflict() {
        let palette = CommandDescriptor::new(CommandId::OpenCommandPalette, "Palette", "Nav")
            .with_default_binding("cmd-p", None);
        let find = CommandDescriptor::new(CommandId::Find, "Find", "Search")
            .with_default_binding("cmd-p", Some(CommandContext::Editor));
        let runtime = KeymapSnapshot {
            effective_bindings: vec![
                EffectiveBinding {
                    keystrokes: "cmd-p".into(),
                    action: CommandId::OpenCommandPalette.action_name().into(),
                    context: None,
                    source: KeybindSource::Default,
                },
                EffectiveBinding {
                    keystrokes: "cmd-p".into(),
                    action: CommandId::Find.action_name().into(),
                    context: Some("Editor".into()),
                    source: KeybindSource::Default,
                },
            ],
            issues: Vec::new(),
        };
        let snap = snapshot([&palette, &find], &runtime, document());
        assert!(!snap.has_conflicts());
        assert!(snap.conflict_commands().is_empty());
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
        assert_eq!(
            snapshot.conflict(CommandId::OpenSettings, Some("Editor"), "shift-cmd-f"),
            None
        );
    }

    #[test]
    fn conflict_finds_another_command_in_the_same_context() {
        let first = CommandDescriptor::new(CommandId::Find, "Find", "Search")
            .with_default_binding("cmd-f", Some(CommandContext::Editor));
        let second = CommandDescriptor::new(CommandId::OpenSettings, "Settings", "Application");
        let runtime = KeymapSnapshot {
            effective_bindings: vec![EffectiveBinding {
                keystrokes: "cmd-f".into(),
                action: CommandId::Find.action_name().into(),
                context: Some("Editor".into()),
                source: KeybindSource::Default,
            }],
            issues: Vec::new(),
        };
        let snapshot = snapshot([&first, &second], &runtime, document());

        let conflict = snapshot
            .conflict(CommandId::OpenSettings, Some("Editor"), "cmd-f")
            .expect("same-context binding must conflict");
        assert_eq!(conflict.command, CommandId::Find);
        assert_eq!(conflict.title, "Find");
        assert_eq!(conflict.context.as_deref(), Some("Editor"));
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
