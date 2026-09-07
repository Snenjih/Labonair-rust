//! Deterministic keymap resolution, independent of GPUI and file formats.

use std::collections::HashMap;

use labonair_command_palette_core::{CommandContext, CommandId};

use crate::normalize;

/// One validated binding supplied by a keymap layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapBinding {
    pub keystrokes: String,
    pub command: CommandId,
    pub context: Option<CommandContext>,
}

impl KeymapBinding {
    pub fn global(keystrokes: impl Into<String>, command: CommandId) -> Self {
        Self {
            keystrokes: keystrokes.into(),
            command,
            context: None,
        }
    }

    pub fn in_context(
        keystrokes: impl Into<String>,
        command: CommandId,
        context: CommandContext,
    ) -> Self {
        Self {
            keystrokes: keystrokes.into(),
            command,
            context: Some(context),
        }
    }
}

/// A binding after context precedence has been resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBinding {
    pub keystrokes: String,
    pub command: CommandId,
    pub context: Option<CommandContext>,
}

/// Resolve bindings in declaration order.
///
/// Global bindings apply everywhere. A binding matching the active context
/// takes precedence over a global binding for the same normalized keystroke;
/// later declarations at the same precedence replace earlier declarations.
/// The returned order follows the first appearance of each keystroke, which
/// keeps installation and diagnostics stable.
pub fn resolve(
    bindings: impl IntoIterator<Item = KeymapBinding>,
    active_context: Option<CommandContext>,
) -> Vec<ResolvedBinding> {
    let mut positions = HashMap::<String, usize>::new();
    let mut selected = Vec::<(u8, ResolvedBinding)>::new();

    for binding in bindings {
        let Some(priority) = (match (binding.context, active_context) {
            (None, _) => Some(0),
            (Some(context), Some(active)) if context == active => Some(1),
            _ => None,
        }) else {
            continue;
        };

        let key = normalize(&binding.keystrokes);
        let resolved = ResolvedBinding {
            keystrokes: binding.keystrokes,
            command: binding.command,
            context: binding.context,
        };
        match positions.get(&key).copied() {
            Some(index) if priority >= selected[index].0 => selected[index] = (priority, resolved),
            Some(_) => {}
            None => {
                positions.insert(key, selected.len());
                selected.push((priority, resolved));
            }
        }
    }

    selected.into_iter().map(|(_, binding)| binding).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_specific_binding_wins_over_global() {
        let resolved = resolve(
            [
                KeymapBinding::global("cmd-p", CommandId::OpenCommandPalette),
                KeymapBinding::in_context("cmd-p", CommandId::Find, CommandContext::Editor),
            ],
            Some(CommandContext::Editor),
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].command, CommandId::Find);
        assert_eq!(resolved[0].context, Some(CommandContext::Editor));
    }

    #[test]
    fn unrelated_context_is_ignored_and_later_global_replaces_global() {
        let resolved = resolve(
            [
                KeymapBinding::global("cmd-p", CommandId::OpenCommandPalette),
                KeymapBinding::in_context("cmd-p", CommandId::Find, CommandContext::Editor),
                KeymapBinding::global("shift-cmd-p", CommandId::Find),
                KeymapBinding::global("shift-cmd-p", CommandId::OpenCommandPalette),
            ],
            Some(CommandContext::Terminal),
        );

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].command, CommandId::OpenCommandPalette);
        assert_eq!(resolved[1].command, CommandId::OpenCommandPalette);
    }

    #[test]
    fn normalized_keystrokes_share_one_stable_slot() {
        let resolved = resolve(
            [
                KeymapBinding::global("shift-cmd-d", CommandId::Find),
                KeymapBinding::global("cmd-shift-d", CommandId::OpenCommandPalette),
            ],
            None,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].keystrokes, "cmd-shift-d");
        assert_eq!(resolved[0].command, CommandId::OpenCommandPalette);
    }
}
