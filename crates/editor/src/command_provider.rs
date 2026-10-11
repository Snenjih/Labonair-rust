//! Editor-owned command metadata.

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

/// Editor UI actions that change a local navigation surface rather than
/// document content or a language-service request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorUiCommand {
    GoToLineColumn,
    ToggleBreadcrumb,
}

/// Typed route selected by the Editor owner for an executable command ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorCommandRoute {
    Edit(EditIntent),
    Split(SplitIntent),
    Language(EditorLanguageCommand),
    Git(GitEditorCommand),
    Ui(EditorUiCommand),
}

/// Resolve an editor-owned command ID without making Workspace understand
/// editor behavior.
pub fn route_for_command(id: CommandId) -> Option<EditorCommandRoute> {
    let route = match id {
        CommandId::EditorAddCursor => EditorCommandRoute::Edit(EditIntent::AddCursorBelow),
        CommandId::EditorAddCursorAbove => EditorCommandRoute::Edit(EditIntent::AddCursorAbove),
        CommandId::EditorSelectNextOccurrence => {
            EditorCommandRoute::Edit(EditIntent::SelectNextOccurrence)
        }
        CommandId::EditorSelectPreviousOccurrence => {
            EditorCommandRoute::Edit(EditIntent::SelectPreviousOccurrence)
        }
        CommandId::EditorSelectAllOccurrences => {
            EditorCommandRoute::Edit(EditIntent::SelectAllOccurrences)
        }
        CommandId::EditorSelectAll => EditorCommandRoute::Edit(EditIntent::SelectAll),
        CommandId::EditorDuplicateSelection => {
            EditorCommandRoute::Edit(EditIntent::DuplicateSelection)
        }
        CommandId::EditorUndoSelection => EditorCommandRoute::Edit(EditIntent::UndoSelection),
        CommandId::EditorRedoSelection => EditorCommandRoute::Edit(EditIntent::RedoSelection),
        CommandId::EditorGoToLineColumn => EditorCommandRoute::Ui(EditorUiCommand::GoToLineColumn),
        CommandId::EditorToggleBreadcrumb => {
            EditorCommandRoute::Ui(EditorUiCommand::ToggleBreadcrumb)
        }
        CommandId::EditorDeleteLine => EditorCommandRoute::Edit(EditIntent::DeleteLine),
        CommandId::EditorJoinLines => EditorCommandRoute::Edit(EditIntent::JoinLines),
        CommandId::EditorExpandSelection => EditorCommandRoute::Edit(EditIntent::ExpandSelection),
        CommandId::EditorShrinkSelection => EditorCommandRoute::Edit(EditIntent::ShrinkSelection),
        CommandId::EditorDuplicateLines => EditorCommandRoute::Edit(EditIntent::DuplicateLines),
        CommandId::EditorMoveLines => {
            EditorCommandRoute::Edit(EditIntent::MoveLines { down: true })
        }
        CommandId::EditorMoveLinesUp => {
            EditorCommandRoute::Edit(EditIntent::MoveLines { down: false })
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
    CommandId::EditorAddCursorAbove,
    CommandId::EditorSelectNextOccurrence,
    CommandId::EditorSelectPreviousOccurrence,
    CommandId::EditorSelectAllOccurrences,
    CommandId::EditorSelectAll,
    CommandId::EditorUndoSelection,
    CommandId::EditorRedoSelection,
    CommandId::EditorGoToLineColumn,
    CommandId::EditorToggleBreadcrumb,
    CommandId::EditorDeleteLine,
    CommandId::EditorJoinLines,
    CommandId::EditorExpandSelection,
    CommandId::EditorShrinkSelection,
    CommandId::EditorDuplicateLines,
    CommandId::EditorDuplicateSelection,
    CommandId::EditorMoveLines,
    CommandId::EditorMoveLinesUp,
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
            CommandDescriptor::new(CommandId::EditorAddCursor, "Add Cursor Below", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-down", Some(CommandContext::Editor))
                .with_default_binding("cmd-ctrl-n", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorAddCursorAbove,
                "Add Cursor Above",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-alt-up", Some(CommandContext::Editor))
            .with_default_binding("cmd-ctrl-p", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorSelectNextOccurrence,
                "Select Next Occurrence",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorSelectPreviousOccurrence,
                "Select Previous Occurrence",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-cmd-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorSelectAllOccurrences,
                "Select All Occurrences",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-shift-l", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorSelectAll, "Select All", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-a", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorUndoSelection, "Undo Selection", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-u", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorRedoSelection, "Redo Selection", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-shift-u", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorGoToLineColumn,
                "Go to Line/Column…",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorToggleBreadcrumb,
                "Toggle Breadcrumbs",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorDeleteLine, "Delete Line", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-shift-k", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorJoinLines, "Join Lines", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-j", Some(CommandContext::Editor)),
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
            CommandDescriptor::new(
                CommandId::EditorDuplicateSelection,
                "Duplicate Selection",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorMoveLinesUp, "Move Line Up", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("alt-up", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorMoveLines, "Move Line Down", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("alt-down", Some(CommandContext::Editor)),
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
        assert!(route_for_command(CommandId::NewEditorTab).is_none());
        for id in EXECUTABLE_EDITOR_COMMAND_IDS {
            assert!(
                route_for_command(*id).is_some(),
                "executable editor command {id:?} must have an owner route"
            );
        }
        assert_eq!(
            route_for_command(CommandId::EditorSelectNextOccurrence),
            Some(EditorCommandRoute::Edit(EditIntent::SelectNextOccurrence))
        );
        assert_eq!(
            route_for_command(CommandId::EditorAddCursorAbove),
            Some(EditorCommandRoute::Edit(EditIntent::AddCursorAbove))
        );
        assert_eq!(
            route_for_command(CommandId::EditorSelectPreviousOccurrence),
            Some(EditorCommandRoute::Edit(
                EditIntent::SelectPreviousOccurrence
            ))
        );
        assert_eq!(
            route_for_command(CommandId::EditorUndoSelection),
            Some(EditorCommandRoute::Edit(EditIntent::UndoSelection))
        );
        assert_eq!(
            route_for_command(CommandId::EditorNextGitChange),
            Some(EditorCommandRoute::Git(GitEditorCommand::NextChange))
        );
    }

    #[test]
    fn multi_selection_commands_expose_cursor_and_selection_history_bindings() {
        let commands = EditorCommandProvider.commands();
        let bindings = |id| {
            commands
                .iter()
                .find(|command| command.id == id)
                .expect("editor command descriptor")
                .default_bindings
                .iter()
                .map(|binding| binding.keystrokes.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            bindings(CommandId::EditorAddCursorAbove),
            vec!["cmd-alt-up", "cmd-ctrl-p"]
        );
        assert_eq!(
            bindings(CommandId::EditorAddCursor),
            vec!["cmd-alt-down", "cmd-ctrl-n"]
        );
        assert_eq!(
            bindings(CommandId::EditorSelectPreviousOccurrence),
            vec!["ctrl-cmd-d"]
        );
        assert_eq!(bindings(CommandId::EditorUndoSelection), vec!["cmd-u"]);
        assert_eq!(
            bindings(CommandId::EditorRedoSelection),
            vec!["cmd-shift-u"]
        );
    }

    #[test]
    fn delete_line_is_an_editor_owned_command_with_the_macos_default() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorDeleteLine)
            .expect("Delete Line descriptor");
        assert_eq!(command.id.action_name(), "editor.delete-line");
        assert_eq!(command.default_bindings.len(), 1);
        assert_eq!(command.default_bindings[0].keystrokes, "cmd-shift-k");
        assert_eq!(
            route_for_command(CommandId::EditorDeleteLine),
            Some(EditorCommandRoute::Edit(EditIntent::DeleteLine))
        );
    }

    #[test]
    fn join_lines_is_an_editor_owned_command_with_the_default_binding() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorJoinLines)
            .expect("Join Lines descriptor");
        assert_eq!(command.id.action_name(), "editor.join-lines");
        assert_eq!(command.default_bindings.len(), 1);
        assert_eq!(command.default_bindings[0].keystrokes, "ctrl-j");
        assert_eq!(
            route_for_command(CommandId::EditorJoinLines),
            Some(EditorCommandRoute::Edit(EditIntent::JoinLines))
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
