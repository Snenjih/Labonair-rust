//! Editor-owned command metadata.

use crate::buffer::Position;
use crate::editing::{EditIntent, SplitIntent};
use labonair_command_palette_core::{
    CommandContext, CommandDescriptor, CommandIcon, CommandId, CommandProvider, CommandSubmenu,
    SubmenuAction, SubmenuDescriptor, SubmenuItem, SubmenuSnapshot,
};

/// Editor actions that require the language-service boundary rather than a
/// direct text transaction. The GPUI adapter supplies the current position or
/// selection and keeps protocol state inside the Editor owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorLanguageCommand {
    TriggerCompletion,
    GoToDefinition,
    GoToDeclaration,
    GoToReferences,
    GoToImplementation,
    PeekDefinition,
    Rename,
    CodeAction,
    FormatDocument,
    FormatSelection,
    OrganizeImports,
    RestartLanguageServer,
    StopLanguageServer,
    ShowDiagnostics,
    NextDiagnostic,
    PreviousDiagnostic,
}

/// Commands for the editor's revision-bound Git projection. Git execution is
/// still delegated to the injected Workspace bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitEditorCommand {
    NextChange,
    PreviousChange,
    OpenProjectDiff,
}

/// Typed route selected by the Editor owner for an executable command ID.
/// Workspace only forwards the resulting intent to the active EditorView.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommandRoute {
    Edit(EditIntent),
    Split(SplitIntent),
    Language(EditorLanguageCommand),
    Git(GitEditorCommand),
}

/// Resolve an editor-owned command ID without making Workspace understand
/// editor behavior. The current caret is supplied only for Add Cursor.
pub fn route_for_command(id: CommandId, cursor: Position) -> Option<EditorCommandRoute> {
    let route = match id {
        CommandId::EditorAddCursor => EditorCommandRoute::Edit(EditIntent::AddCursor(cursor)),
        CommandId::EditorSelectNextOccurrence => {
            EditorCommandRoute::Edit(EditIntent::SelectNextOccurrence)
        }
        CommandId::EditorSelectAllOccurrences => {
            EditorCommandRoute::Edit(EditIntent::SelectAllOccurrences)
        }
        CommandId::EditorExpandSelection => EditorCommandRoute::Edit(EditIntent::ExpandSelection),
        CommandId::EditorShrinkSelection => EditorCommandRoute::Edit(EditIntent::ShrinkSelection),
        CommandId::EditorDuplicateLines => EditorCommandRoute::Edit(EditIntent::DuplicateLines),
        CommandId::EditorMoveLines => {
            EditorCommandRoute::Edit(EditIntent::MoveLines { down: true })
        }
        CommandId::EditorIndent => {
            EditorCommandRoute::Edit(EditIntent::Indent { unit: "  ".into() })
        }
        CommandId::EditorOutdent => {
            EditorCommandRoute::Edit(EditIntent::Outdent { unit: "  ".into() })
        }
        CommandId::EditorToggleComment => EditorCommandRoute::Edit(EditIntent::ToggleComment),
        CommandId::EditorTranspose => EditorCommandRoute::Edit(EditIntent::Transpose),
        CommandId::EditorSplitRight => EditorCommandRoute::Split(SplitIntent::Right),
        CommandId::EditorSplitDown => EditorCommandRoute::Split(SplitIntent::Down),
        CommandId::EditorFocusNextGroup => EditorCommandRoute::Split(SplitIntent::FocusNext),
        CommandId::EditorFocusPreviousGroup => {
            EditorCommandRoute::Split(SplitIntent::FocusPrevious)
        }
        CommandId::EditorCloseGroup => EditorCommandRoute::Split(SplitIntent::Close),
        CommandId::EditorCloseOtherGroups => EditorCommandRoute::Split(SplitIntent::CloseOthers),
        CommandId::EditorTriggerCompletion => {
            EditorCommandRoute::Language(EditorLanguageCommand::TriggerCompletion)
        }
        CommandId::EditorGoToDefinition => {
            EditorCommandRoute::Language(EditorLanguageCommand::GoToDefinition)
        }
        CommandId::EditorGoToDeclaration => {
            EditorCommandRoute::Language(EditorLanguageCommand::GoToDeclaration)
        }
        CommandId::EditorGoToReferences => {
            EditorCommandRoute::Language(EditorLanguageCommand::GoToReferences)
        }
        CommandId::EditorGoToImplementation => {
            EditorCommandRoute::Language(EditorLanguageCommand::GoToImplementation)
        }
        CommandId::EditorPeekDefinition => {
            EditorCommandRoute::Language(EditorLanguageCommand::PeekDefinition)
        }
        CommandId::EditorRenameSymbol => {
            EditorCommandRoute::Language(EditorLanguageCommand::Rename)
        }
        CommandId::EditorCodeAction => {
            EditorCommandRoute::Language(EditorLanguageCommand::CodeAction)
        }
        CommandId::EditorFormatDocument => {
            EditorCommandRoute::Language(EditorLanguageCommand::FormatDocument)
        }
        CommandId::EditorFormatSelection => {
            EditorCommandRoute::Language(EditorLanguageCommand::FormatSelection)
        }
        CommandId::EditorOrganizeImports => {
            EditorCommandRoute::Language(EditorLanguageCommand::OrganizeImports)
        }
        CommandId::EditorRestartLanguageServer => {
            EditorCommandRoute::Language(EditorLanguageCommand::RestartLanguageServer)
        }
        CommandId::EditorStopLanguageServer => {
            EditorCommandRoute::Language(EditorLanguageCommand::StopLanguageServer)
        }
        CommandId::EditorShowDiagnostics => {
            EditorCommandRoute::Language(EditorLanguageCommand::ShowDiagnostics)
        }
        CommandId::EditorNextDiagnostic => {
            EditorCommandRoute::Language(EditorLanguageCommand::NextDiagnostic)
        }
        CommandId::EditorPreviousDiagnostic => {
            EditorCommandRoute::Language(EditorLanguageCommand::PreviousDiagnostic)
        }
        CommandId::EditorNextGitChange => EditorCommandRoute::Git(GitEditorCommand::NextChange),
        CommandId::EditorPreviousGitChange => {
            EditorCommandRoute::Git(GitEditorCommand::PreviousChange)
        }
        CommandId::EditorOpenProjectDiff => {
            EditorCommandRoute::Git(GitEditorCommand::OpenProjectDiff)
        }
        _ => return None,
    };
    Some(route)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EditorCommandProvider;

/// Editor command IDs that have an executable handler at the Workspace
/// composition boundary. Keeping this list next to the owner metadata avoids
/// a second, drifting command inventory in the shell.
pub const EXECUTABLE_EDITOR_COMMAND_IDS: &[CommandId] = &[
    CommandId::EditorAddCursor,
    CommandId::EditorSelectNextOccurrence,
    CommandId::EditorSelectAllOccurrences,
    CommandId::EditorExpandSelection,
    CommandId::EditorShrinkSelection,
    CommandId::EditorDuplicateLines,
    CommandId::EditorMoveLines,
    CommandId::EditorIndent,
    CommandId::EditorOutdent,
    CommandId::EditorToggleComment,
    CommandId::EditorTranspose,
    CommandId::EditorSplitRight,
    CommandId::EditorSplitDown,
    CommandId::EditorFocusNextGroup,
    CommandId::EditorFocusPreviousGroup,
    CommandId::EditorCloseGroup,
    CommandId::EditorCloseOtherGroups,
    CommandId::EditorTriggerCompletion,
    CommandId::EditorGoToDefinition,
    CommandId::EditorGoToDeclaration,
    CommandId::EditorGoToReferences,
    CommandId::EditorGoToImplementation,
    CommandId::EditorPeekDefinition,
    CommandId::EditorRenameSymbol,
    CommandId::EditorCodeAction,
    CommandId::EditorFormatDocument,
    CommandId::EditorFormatSelection,
    CommandId::EditorOrganizeImports,
    CommandId::EditorRestartLanguageServer,
    CommandId::EditorStopLanguageServer,
    CommandId::EditorShowDiagnostics,
    CommandId::EditorNextDiagnostic,
    CommandId::EditorPreviousDiagnostic,
    CommandId::EditorNextGitChange,
    CommandId::EditorPreviousGitChange,
    CommandId::EditorOpenProjectDiff,
];

pub fn outline_submenu(rows: impl IntoIterator<Item = (usize, String, String)>) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("outline", "Symbols", CommandSubmenu::Outline),
        items: rows
            .into_iter()
            .map(|(line, title, subtitle)| SubmenuItem {
                id: line.to_string(),
                title,
                subtitle: Some(subtitle),
                active: false,
                action: SubmenuAction::GoToLine(line),
                secondary: None,
            })
            .collect(),
    }
}

impl CommandProvider for EditorCommandProvider {
    fn commands(&self) -> Vec<CommandDescriptor> {
        vec![
            CommandDescriptor::new(CommandId::OpenFile, "Open File…", "Editor")
                .with_default_binding("cmd-shift-o", None)
                .with_icon(CommandIcon::File),
            CommandDescriptor::new(CommandId::Find, "Find in Current Pane", "Search")
                .with_default_binding("cmd-f", None)
                .with_icon(CommandIcon::Search),
            CommandDescriptor::new(CommandId::GoToSymbol, "Go to Symbol…", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_icon(CommandIcon::FileCode)
                .with_submenu(CommandSubmenu::Outline),
            CommandDescriptor::new(CommandId::EditorAddCursor, "Add Cursor", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-down", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorSelectNextOccurrence,
                "Select Next Occurrence",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorSelectAllOccurrences,
                "Select All Occurrences",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorExpandSelection,
                "Expand Selection",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-shift-right", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorShrinkSelection,
                "Shrink Selection",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-shift-left", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorDuplicateLines, "Duplicate Lines", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-shift-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorMoveLines, "Move Lines", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorIndent, "Indent", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-]", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorOutdent, "Outdent", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-[", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorToggleComment, "Toggle Comment", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-/", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorTranspose, "Transpose Characters", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorSplitRight, "Split Editor Right", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-]", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorSplitDown, "Split Editor Down", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-shift-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorFocusNextGroup,
                "Focus Next Editor Group",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-alt-right", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorFocusPreviousGroup,
                "Focus Previous Editor Group",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-alt-left", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorCloseGroup, "Close Editor Group", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorCloseOtherGroups,
                "Close Other Editor Groups",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorTriggerCompletion,
                "Trigger Completion",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-space", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorGoToDefinition,
                "Go to Definition",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("f12", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorGoToDeclaration,
                "Go to Declaration",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorGoToReferences,
                "Go to References",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorGoToImplementation,
                "Go to Implementation",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorPeekDefinition, "Peek Definition", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorRenameSymbol, "Rename Symbol", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("f2", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorCodeAction, "Code Action", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-.", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFormatDocument, "Format Document", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("shift-alt-f", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorFormatSelection,
                "Format Selection",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorOrganizeImports,
                "Organize Imports",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorRestartLanguageServer,
                "Restart Language Server",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorStopLanguageServer,
                "Stop Language Server",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorShowDiagnostics,
                "Show Diagnostics",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorNextDiagnostic, "Next Diagnostic", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorPreviousDiagnostic,
                "Previous Diagnostic",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorNextGitChange, "Next Git Change", "Editor")
                .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorPreviousGitChange,
                "Previous Git Change",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorOpenProjectDiff,
                "Open Project Diff",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_editor_ids_have_one_owner_descriptor() {
        let commands = EditorCommandProvider.commands();
        let mut ids = EXECUTABLE_EDITOR_COMMAND_IDS.to_vec();
        ids.sort_by_key(|id| format!("{id:?}"));
        ids.dedup();
        assert_eq!(ids.len(), EXECUTABLE_EDITOR_COMMAND_IDS.len());
        for id in EXECUTABLE_EDITOR_COMMAND_IDS {
            assert!(
                commands.iter().any(|command| command.id == *id),
                "missing editor descriptor for {id:?}"
            );
        }
    }

    #[test]
    fn route_contract_rejects_non_editor_commands_and_preserves_typed_actions() {
        assert!(route_for_command(CommandId::NewEditorTab, Position::new(0, 0)).is_none());
        assert_eq!(
            route_for_command(CommandId::EditorSelectNextOccurrence, Position::new(2, 4)),
            Some(EditorCommandRoute::Edit(EditIntent::SelectNextOccurrence))
        );
        assert_eq!(
            route_for_command(CommandId::EditorNextGitChange, Position::new(0, 0)),
            Some(EditorCommandRoute::Git(GitEditorCommand::NextChange))
        );
    }

    #[test]
    fn open_file_is_editor_owned_and_uses_a_palette_safe_default_binding() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::OpenFile)
            .expect("Open File descriptor");
        assert_eq!(command.id.action_name(), "editor::OpenFile");
        assert_eq!(command.default_bindings[0].keystrokes, "cmd-shift-o");
    }
}
