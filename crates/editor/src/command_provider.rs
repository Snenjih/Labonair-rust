//! Editor-owned command metadata.

use crate::editing::{DeleteTarget, EditIntent, SplitIntent};
use crate::language_services::{Diagnostic, DiagnosticSeverity};
use crate::symbols::DocumentSymbol;
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
    GoToTypeDefinition,
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
    GoBack,
    GoForward,
    ToggleSearchSelection,
    ToggleReadOnly,
    NextSnippetTabstop,
    PreviousSnippetTabstop,
    OpenMarkdownPreview,
    OpenFollowingMarkdownPreview,
    OpenSvgPreview,
    ToggleBreadcrumb,
    ToggleInlayHints,
    Rewrap,
    KillRingCut,
    KillRingYank,
    ShowCharacterPalette,
    Fold,
    FoldAll,
    FoldAtLevel(u8),
    FoldRecursive,
    UnfoldAll,
    UnfoldLines,
    UnfoldRecursive,
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

/// A symbol picker row collected from an open editor buffer in the active
/// workspace. Workspace supplies the tab identity and display label; symbol
/// extraction and row presentation remain owned by Editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSymbolPickerRow {
    pub tab_id: u64,
    pub file_label: String,
    pub symbol: DocumentSymbol,
}

/// One diagnostic result from an open editor buffer in the active workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorDiagnosticPickerRow {
    pub tab_id: u64,
    pub file_label: String,
    pub diagnostic: Diagnostic,
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
        CommandId::EditorSelectLine => EditorCommandRoute::Edit(EditIntent::SelectLine),
        CommandId::EditorSelectAll => EditorCommandRoute::Edit(EditIntent::SelectAll),
        CommandId::EditorDuplicateSelection => {
            EditorCommandRoute::Edit(EditIntent::DuplicateSelection)
        }
        CommandId::EditorUndoSelection => EditorCommandRoute::Edit(EditIntent::UndoSelection),
        CommandId::EditorRedoSelection => EditorCommandRoute::Edit(EditIntent::RedoSelection),
        CommandId::EditorNextSnippetTabstop => {
            EditorCommandRoute::Ui(EditorUiCommand::NextSnippetTabstop)
        }
        CommandId::EditorPreviousSnippetTabstop => {
            EditorCommandRoute::Ui(EditorUiCommand::PreviousSnippetTabstop)
        }
        CommandId::EditorGoToLineColumn => EditorCommandRoute::Ui(EditorUiCommand::GoToLineColumn),
        CommandId::EditorGoBack => EditorCommandRoute::Ui(EditorUiCommand::GoBack),
        CommandId::EditorGoForward => EditorCommandRoute::Ui(EditorUiCommand::GoForward),
        CommandId::EditorToggleSearchSelection => {
            EditorCommandRoute::Ui(EditorUiCommand::ToggleSearchSelection)
        }
        CommandId::EditorToggleReadOnly => EditorCommandRoute::Ui(EditorUiCommand::ToggleReadOnly),
        CommandId::EditorOpenMarkdownPreview => {
            EditorCommandRoute::Ui(EditorUiCommand::OpenMarkdownPreview)
        }
        CommandId::EditorOpenFollowingMarkdownPreview => {
            EditorCommandRoute::Ui(EditorUiCommand::OpenFollowingMarkdownPreview)
        }
        CommandId::EditorOpenSvgPreview => EditorCommandRoute::Ui(EditorUiCommand::OpenSvgPreview),
        CommandId::EditorToggleBreadcrumb => {
            EditorCommandRoute::Ui(EditorUiCommand::ToggleBreadcrumb)
        }
        CommandId::EditorToggleInlayHints => {
            EditorCommandRoute::Ui(EditorUiCommand::ToggleInlayHints)
        }
        CommandId::EditorDeleteLine => EditorCommandRoute::Edit(EditIntent::DeleteLine),
        CommandId::EditorJoinLines => EditorCommandRoute::Edit(EditIntent::JoinLines),
        CommandId::EditorRewrap => EditorCommandRoute::Ui(EditorUiCommand::Rewrap),
        CommandId::EditorDeleteToBeginningOfLine => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::LineStart))
        }
        CommandId::EditorDeleteToEndOfLine => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::LineEnd))
        }
        CommandId::EditorDeleteToNextWordEnd => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::NextWordEnd))
        }
        CommandId::EditorDeleteToPreviousWordStart => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::PreviousWordStart))
        }
        CommandId::EditorDeleteToNextSubwordEnd => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::NextSubwordEnd))
        }
        CommandId::EditorDeleteToPreviousSubwordStart => {
            EditorCommandRoute::Edit(EditIntent::DeleteTo(DeleteTarget::PreviousSubwordStart))
        }
        CommandId::EditorKillRingCut => EditorCommandRoute::Ui(EditorUiCommand::KillRingCut),
        CommandId::EditorKillRingYank => EditorCommandRoute::Ui(EditorUiCommand::KillRingYank),
        CommandId::EditorShowCharacterPalette => {
            EditorCommandRoute::Ui(EditorUiCommand::ShowCharacterPalette)
        }
        CommandId::EditorFold => EditorCommandRoute::Ui(EditorUiCommand::Fold),
        CommandId::EditorFoldAll => EditorCommandRoute::Ui(EditorUiCommand::FoldAll),
        CommandId::EditorFoldAtLevel1 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(1)),
        CommandId::EditorFoldAtLevel2 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(2)),
        CommandId::EditorFoldAtLevel3 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(3)),
        CommandId::EditorFoldAtLevel4 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(4)),
        CommandId::EditorFoldAtLevel5 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(5)),
        CommandId::EditorFoldAtLevel6 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(6)),
        CommandId::EditorFoldAtLevel7 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(7)),
        CommandId::EditorFoldAtLevel8 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(8)),
        CommandId::EditorFoldAtLevel9 => EditorCommandRoute::Ui(EditorUiCommand::FoldAtLevel(9)),
        CommandId::EditorFoldRecursive => EditorCommandRoute::Ui(EditorUiCommand::FoldRecursive),
        CommandId::EditorUnfoldAll => EditorCommandRoute::Ui(EditorUiCommand::UnfoldAll),
        CommandId::EditorUnfoldLines => EditorCommandRoute::Ui(EditorUiCommand::UnfoldLines),
        CommandId::EditorUnfoldRecursive => {
            EditorCommandRoute::Ui(EditorUiCommand::UnfoldRecursive)
        }
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
        CommandId::EditorGoToTypeDefinition => {
            EditorCommandRoute::Language(EditorLanguageCommand::GoToTypeDefinition)
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
    CommandId::EditorSelectLine,
    CommandId::EditorSelectAll,
    CommandId::EditorUndoSelection,
    CommandId::EditorRedoSelection,
    CommandId::EditorNextSnippetTabstop,
    CommandId::EditorPreviousSnippetTabstop,
    CommandId::EditorGoToLineColumn,
    CommandId::EditorGoBack,
    CommandId::EditorGoForward,
    CommandId::EditorToggleSearchSelection,
    CommandId::EditorToggleReadOnly,
    CommandId::EditorOpenMarkdownPreview,
    CommandId::EditorOpenFollowingMarkdownPreview,
    CommandId::EditorOpenSvgPreview,
    CommandId::EditorToggleBreadcrumb,
    CommandId::EditorToggleInlayHints,
    CommandId::EditorDeleteLine,
    CommandId::EditorJoinLines,
    CommandId::EditorRewrap,
    CommandId::EditorDeleteToBeginningOfLine,
    CommandId::EditorDeleteToEndOfLine,
    CommandId::EditorDeleteToNextWordEnd,
    CommandId::EditorDeleteToPreviousWordStart,
    CommandId::EditorDeleteToNextSubwordEnd,
    CommandId::EditorDeleteToPreviousSubwordStart,
    CommandId::EditorKillRingCut,
    CommandId::EditorKillRingYank,
    CommandId::EditorShowCharacterPalette,
    CommandId::EditorFold,
    CommandId::EditorFoldAll,
    CommandId::EditorFoldAtLevel1,
    CommandId::EditorFoldAtLevel2,
    CommandId::EditorFoldAtLevel3,
    CommandId::EditorFoldAtLevel4,
    CommandId::EditorFoldAtLevel5,
    CommandId::EditorFoldAtLevel6,
    CommandId::EditorFoldAtLevel7,
    CommandId::EditorFoldAtLevel8,
    CommandId::EditorFoldAtLevel9,
    CommandId::EditorFoldRecursive,
    CommandId::EditorUnfoldAll,
    CommandId::EditorUnfoldLines,
    CommandId::EditorUnfoldRecursive,
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
    CommandId::EditorGoToTypeDefinition,
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

pub fn outline_submenu(rows: impl IntoIterator<Item = EditorSymbolPickerRow>) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new("outline", "Symbols", CommandSubmenu::Outline),
        items: rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| SubmenuItem {
                id: format!("{}:{index}", row.tab_id),
                title: row.symbol.name,
                subtitle: Some(format!(
                    "{}  ·  {}  ·  line {}",
                    row.file_label,
                    row.symbol.kind.label(),
                    row.symbol.line + 1
                )),
                active: false,
                action: SubmenuAction::GoToEditorLine {
                    tab_id: row.tab_id,
                    line: row.symbol.line,
                },
                secondary: None,
            })
            .collect(),
    }
}

/// Build the editor-owned searchable diagnostics page from current buffer
/// snapshots. Workspace contributes rows; the picker only renders and routes.
pub fn diagnostics_submenu(
    rows: impl IntoIterator<Item = EditorDiagnosticPickerRow>,
) -> SubmenuSnapshot {
    SubmenuSnapshot {
        descriptor: SubmenuDescriptor::new(
            "diagnostics",
            "Diagnostics",
            CommandSubmenu::Diagnostics,
        ),
        items: rows
            .into_iter()
            .enumerate()
            .map(|(index, row)| {
                let line = row.diagnostic.range.start.line;
                let severity = match row.diagnostic.severity {
                    DiagnosticSeverity::Error => "error",
                    DiagnosticSeverity::Warning => "warning",
                    DiagnosticSeverity::Information => "information",
                    DiagnosticSeverity::Hint => "hint",
                };
                SubmenuItem {
                    id: format!("{}:{index}", row.tab_id),
                    title: row.diagnostic.message,
                    subtitle: Some(format!(
                        "{}  ·  {}  ·  line {}",
                        row.file_label,
                        severity,
                        line + 1
                    )),
                    active: false,
                    action: SubmenuAction::GoToEditorLine {
                        tab_id: row.tab_id,
                        line,
                    },
                    secondary: None,
                }
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
            CommandDescriptor::new(
                CommandId::EditorToggleSearchSelection,
                "Toggle Search Selection",
                "Search",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-alt-l", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorToggleReadOnly,
                "Toggle Read Only",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorOpenMarkdownPreview,
                "Open Markdown Preview",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-shift-v", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorOpenFollowingMarkdownPreview,
                "Open Following Markdown Preview",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorOpenSvgPreview,
                "Open SVG Preview",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
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
            CommandDescriptor::new(CommandId::EditorSelectLine, "Select Line", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-l", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorUndoSelection, "Undo Selection", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-u", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorRedoSelection, "Redo Selection", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-shift-u", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorNextSnippetTabstop,
                "Next Snippet Tabstop",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("tab", Some(CommandContext::EditorSnippetNext)),
            CommandDescriptor::new(
                CommandId::EditorPreviousSnippetTabstop,
                "Previous Snippet Tabstop",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("shift-tab", Some(CommandContext::EditorSnippetPrevious)),
            CommandDescriptor::new(
                CommandId::EditorGoToLineColumn,
                "Go to Line/Column…",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(CommandId::EditorGoBack, "Go Back", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-alt-left", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorGoForward, "Go Forward", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-alt-right", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorToggleBreadcrumb,
                "Toggle Breadcrumbs",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor]),
            CommandDescriptor::new(
                CommandId::EditorToggleInlayHints,
                "Toggle Inlay Hints",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-:", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorDeleteLine, "Delete Line", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-shift-k", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToBeginningOfLine,
                "Delete to Beginning of Line",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-backspace", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToEndOfLine,
                "Delete to End of Line",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-delete", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToNextWordEnd,
                "Delete to Next Word End",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("alt-delete", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToPreviousWordStart,
                "Delete to Previous Word Start",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("alt-backspace", Some(CommandContext::Editor))
            .with_default_binding("ctrl-w", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToNextSubwordEnd,
                "Delete to Next Subword End",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-alt-delete", Some(CommandContext::Editor))
            .with_default_binding("ctrl-alt-d", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorDeleteToPreviousSubwordStart,
                "Delete to Previous Subword Start",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-alt-backspace", Some(CommandContext::Editor))
            .with_default_binding("ctrl-alt-h", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorKillRingCut, "Kill Ring Cut", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-k", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorKillRingYank, "Kill Ring Yank", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-y", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorShowCharacterPalette,
                "Show Character Palette",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("ctrl-cmd-space", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorJoinLines, "Join Lines", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("ctrl-j", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorRewrap, "Rewrap Paragraphs", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-q", Some(CommandContext::Editor))
                .with_default_binding("cmd-k q", Some(CommandContext::Editor)),
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
            CommandDescriptor::new(
                CommandId::EditorGoToTypeDefinition,
                "Go to Type Definition",
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
            .with_contexts(&[CommandContext::Editor])
            .with_submenu(CommandSubmenu::Diagnostics),
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
            CommandDescriptor::new(CommandId::EditorFold, "Fold Region", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-[", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAll, "Fold All", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-0", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel1, "Fold at Level 1", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-1", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel2, "Fold at Level 2", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-2", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel3, "Fold at Level 3", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-3", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel4, "Fold at Level 4", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-4", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel5, "Fold at Level 5", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-5", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel6, "Fold at Level 6", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-6", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel7, "Fold at Level 7", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-7", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel8, "Fold at Level 8", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-8", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldAtLevel9, "Fold at Level 9", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-9", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorFoldRecursive, "Fold Recursively", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-[", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorUnfoldAll, "Unfold All", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-k cmd-j", Some(CommandContext::Editor)),
            CommandDescriptor::new(CommandId::EditorUnfoldLines, "Unfold Lines", "Editor")
                .with_contexts(&[CommandContext::Editor])
                .with_default_binding("cmd-alt-]", Some(CommandContext::Editor)),
            CommandDescriptor::new(
                CommandId::EditorUnfoldRecursive,
                "Unfold Recursively",
                "Editor",
            )
            .with_contexts(&[CommandContext::Editor])
            .with_default_binding("cmd-k cmd-]", Some(CommandContext::Editor)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_submenu_targets_the_source_editor_and_line() {
        let snapshot = outline_submenu([EditorSymbolPickerRow {
            tab_id: 17,
            file_label: "src/main.rs".to_string(),
            symbol: DocumentSymbol {
                name: "main".to_string(),
                kind: crate::SymbolKind::Function,
                line: 8,
                range: crate::TextRange::new(0, 12),
                name_range: crate::TextRange::new(3, 7),
            },
        }]);

        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].title, "main");
        assert_eq!(
            snapshot.items[0].subtitle.as_deref(),
            Some("src/main.rs  ·  function  ·  line 9")
        );
        assert_eq!(
            snapshot.items[0].action,
            SubmenuAction::GoToEditorLine {
                tab_id: 17,
                line: 8,
            }
        );
    }

    #[test]
    fn diagnostics_submenu_targets_the_diagnostic_buffer_and_line() {
        let snapshot = diagnostics_submenu([EditorDiagnosticPickerRow {
            tab_id: 23,
            file_label: "src/lib.rs".to_string(),
            diagnostic: Diagnostic {
                range: crate::LspRange {
                    start: crate::LspPosition {
                        line: 11,
                        character: 2,
                    },
                    end: crate::LspPosition {
                        line: 11,
                        character: 8,
                    },
                },
                severity: DiagnosticSeverity::Warning,
                message: "unused value".to_string(),
                source: crate::LanguageServerId::new("rust-analyzer"),
                code: None,
            },
        }]);

        assert_eq!(snapshot.descriptor.id, "diagnostics");
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].title, "unused value");
        assert_eq!(
            snapshot.items[0].subtitle.as_deref(),
            Some("src/lib.rs  ·  warning  ·  line 12")
        );
        assert_eq!(
            snapshot.items[0].action,
            SubmenuAction::GoToEditorLine {
                tab_id: 23,
                line: 11,
            }
        );
    }

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
            route_for_command(CommandId::EditorToggleSearchSelection),
            Some(EditorCommandRoute::Ui(
                EditorUiCommand::ToggleSearchSelection
            ))
        );
        assert_eq!(
            route_for_command(CommandId::EditorGoBack),
            Some(EditorCommandRoute::Ui(EditorUiCommand::GoBack))
        );
        assert_eq!(
            route_for_command(CommandId::EditorGoForward),
            Some(EditorCommandRoute::Ui(EditorUiCommand::GoForward))
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
        assert_eq!(
            route_for_command(CommandId::EditorGoToTypeDefinition),
            Some(EditorCommandRoute::Language(
                EditorLanguageCommand::GoToTypeDefinition
            ))
        );
        assert_eq!(
            route_for_command(CommandId::EditorToggleInlayHints),
            Some(EditorCommandRoute::Ui(EditorUiCommand::ToggleInlayHints))
        );
        let inlay_command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorToggleInlayHints)
            .expect("Toggle Inlay Hints descriptor");
        assert_eq!(inlay_command.default_bindings[0].keystrokes, "ctrl-:");
        assert!(EditorCommandProvider
            .commands()
            .iter()
            .any(|command| command.id == CommandId::EditorGoToTypeDefinition
                && command.title == "Go to Type Definition"));
    }

    #[test]
    fn selection_search_toggle_is_editor_owned_with_its_pinned_binding() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorToggleSearchSelection)
            .expect("selection search command");

        assert_eq!(command.title, "Toggle Search Selection");
        assert_eq!(command.section, "Search");
        assert_eq!(command.id.action_name(), "search::ToggleSelection");
        assert_eq!(command.default_bindings[0].keystrokes, "cmd-alt-l");
        assert_eq!(
            command.default_bindings[0].context,
            Some(CommandContext::Editor)
        );
    }

    #[test]
    fn markdown_preview_is_editor_owned_and_uses_its_pinned_binding() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorOpenMarkdownPreview)
            .expect("Markdown Preview descriptor");

        assert_eq!(command.title, "Open Markdown Preview");
        assert_eq!(command.id.action_name(), "markdown::OpenPreview");
        assert_eq!(command.default_bindings[0].keystrokes, "cmd-shift-v");
        assert_eq!(
            command.default_bindings[0].context,
            Some(CommandContext::Editor)
        );
        assert_eq!(
            route_for_command(CommandId::EditorOpenMarkdownPreview),
            Some(EditorCommandRoute::Ui(EditorUiCommand::OpenMarkdownPreview))
        );
    }

    #[test]
    fn svg_preview_is_an_editor_owned_command() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorOpenSvgPreview)
            .expect("SVG Preview descriptor");

        assert_eq!(command.title, "Open SVG Preview");
        assert_eq!(command.id.action_name(), "svg::OpenPreview");
        assert!(command.default_bindings.is_empty());
        assert_eq!(
            route_for_command(CommandId::EditorOpenSvgPreview),
            Some(EditorCommandRoute::Ui(EditorUiCommand::OpenSvgPreview))
        );
    }

    #[test]
    fn following_markdown_preview_is_an_editor_owned_command() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorOpenFollowingMarkdownPreview)
            .expect("Following Markdown Preview descriptor");

        assert_eq!(command.title, "Open Following Markdown Preview");
        assert_eq!(command.id.action_name(), "markdown::OpenFollowingPreview");
        assert!(command.default_bindings.is_empty());
        assert_eq!(
            route_for_command(CommandId::EditorOpenFollowingMarkdownPreview),
            Some(EditorCommandRoute::Ui(
                EditorUiCommand::OpenFollowingMarkdownPreview
            ))
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
    fn snippet_tabstop_commands_are_bound_only_in_their_live_contexts() {
        let commands = EditorCommandProvider.commands();
        let next = commands
            .iter()
            .find(|command| command.id == CommandId::EditorNextSnippetTabstop)
            .expect("next snippet tabstop command");
        let previous = commands
            .iter()
            .find(|command| command.id == CommandId::EditorPreviousSnippetTabstop)
            .expect("previous snippet tabstop command");

        assert_eq!(next.default_bindings[0].keystrokes, "tab");
        assert_eq!(
            next.default_bindings[0].context,
            Some(CommandContext::EditorSnippetNext)
        );
        assert_eq!(previous.default_bindings[0].keystrokes, "shift-tab");
        assert_eq!(
            previous.default_bindings[0].context,
            Some(CommandContext::EditorSnippetPrevious)
        );
        assert_eq!(
            route_for_command(CommandId::EditorNextSnippetTabstop),
            Some(EditorCommandRoute::Ui(EditorUiCommand::NextSnippetTabstop))
        );
    }

    #[test]
    fn navigation_history_commands_have_editor_scoped_shortcuts() {
        let commands = EditorCommandProvider.commands();
        for (id, binding) in [
            (CommandId::EditorGoBack, "ctrl-alt-left"),
            (CommandId::EditorGoForward, "ctrl-alt-right"),
        ] {
            let command = commands.iter().find(|command| command.id == id).unwrap();
            assert_eq!(command.default_bindings.len(), 1);
            assert_eq!(command.default_bindings[0].keystrokes, binding);
            assert_eq!(
                command.default_bindings[0].context,
                Some(CommandContext::Editor)
            );
        }
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
    fn rewrap_is_an_editor_owned_command_with_both_chord_bindings() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorRewrap)
            .expect("Rewrap descriptor");
        assert_eq!(command.id.action_name(), "editor.rewrap");
        assert_eq!(
            command
                .default_bindings
                .iter()
                .map(|binding| binding.keystrokes.as_str())
                .collect::<Vec<_>>(),
            vec!["cmd-k cmd-q", "cmd-k q"]
        );
        assert_eq!(
            route_for_command(CommandId::EditorRewrap),
            Some(EditorCommandRoute::Ui(EditorUiCommand::Rewrap))
        );
    }

    #[test]
    fn delete_to_boundary_commands_are_editor_owned_and_have_mac_defaults() {
        let commands = EditorCommandProvider.commands();
        let cases = [
            (
                CommandId::EditorDeleteToBeginningOfLine,
                "cmd-backspace",
                DeleteTarget::LineStart,
            ),
            (
                CommandId::EditorDeleteToEndOfLine,
                "cmd-delete",
                DeleteTarget::LineEnd,
            ),
            (
                CommandId::EditorDeleteToNextWordEnd,
                "alt-delete",
                DeleteTarget::NextWordEnd,
            ),
            (
                CommandId::EditorDeleteToPreviousWordStart,
                "alt-backspace",
                DeleteTarget::PreviousWordStart,
            ),
            (
                CommandId::EditorDeleteToNextSubwordEnd,
                "ctrl-alt-delete",
                DeleteTarget::NextSubwordEnd,
            ),
            (
                CommandId::EditorDeleteToPreviousSubwordStart,
                "ctrl-alt-backspace",
                DeleteTarget::PreviousSubwordStart,
            ),
        ];
        for (id, binding, target) in cases {
            let command = commands
                .iter()
                .find(|command| command.id == id)
                .expect("delete to boundary descriptor");
            assert_eq!(command.default_bindings[0].keystrokes, binding);
            assert_eq!(
                route_for_command(id),
                Some(EditorCommandRoute::Edit(EditIntent::DeleteTo(target)))
            );
        }
    }

    #[test]
    fn select_line_is_an_editor_owned_command_with_its_mac_default() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorSelectLine)
            .expect("Select Line descriptor");
        assert_eq!(command.id.action_name(), "editor.select-line");
        assert_eq!(command.default_bindings[0].keystrokes, "cmd-l");
        assert_eq!(
            route_for_command(CommandId::EditorSelectLine),
            Some(EditorCommandRoute::Edit(EditIntent::SelectLine))
        );
    }

    #[test]
    fn kill_ring_commands_are_editor_owned_with_spec_bindings() {
        let commands = EditorCommandProvider.commands();
        for (id, binding, route) in [
            (
                CommandId::EditorKillRingCut,
                "ctrl-k",
                EditorUiCommand::KillRingCut,
            ),
            (
                CommandId::EditorKillRingYank,
                "ctrl-y",
                EditorUiCommand::KillRingYank,
            ),
        ] {
            let command = commands
                .iter()
                .find(|command| command.id == id)
                .expect("Kill Ring descriptor");
            assert_eq!(command.default_bindings[0].keystrokes, binding);
            assert_eq!(route_for_command(id), Some(EditorCommandRoute::Ui(route)));
        }
    }

    #[test]
    fn character_palette_is_an_editor_owned_command_with_its_spec_binding() {
        let command = EditorCommandProvider
            .commands()
            .into_iter()
            .find(|command| command.id == CommandId::EditorShowCharacterPalette)
            .expect("Character Palette descriptor");
        assert_eq!(command.id.action_name(), "editor.show-character-palette");
        assert_eq!(command.default_bindings[0].keystrokes, "ctrl-cmd-space");
        assert_eq!(
            command.default_bindings[0].context,
            Some(CommandContext::Editor)
        );
        assert_eq!(
            route_for_command(CommandId::EditorShowCharacterPalette),
            Some(EditorCommandRoute::Ui(
                EditorUiCommand::ShowCharacterPalette
            ))
        );
        assert!(EXECUTABLE_EDITOR_COMMAND_IDS.contains(&CommandId::EditorShowCharacterPalette));
    }

    #[test]
    fn fold_and_unfold_commands_are_editor_owned_with_spec_bindings() {
        let commands = EditorCommandProvider.commands();
        let cases = [
            (CommandId::EditorFold, "cmd-alt-["),
            (CommandId::EditorFoldAll, "cmd-k cmd-0"),
            (CommandId::EditorFoldAtLevel1, "cmd-k cmd-1"),
            (CommandId::EditorFoldAtLevel2, "cmd-k cmd-2"),
            (CommandId::EditorFoldAtLevel3, "cmd-k cmd-3"),
            (CommandId::EditorFoldAtLevel4, "cmd-k cmd-4"),
            (CommandId::EditorFoldAtLevel5, "cmd-k cmd-5"),
            (CommandId::EditorFoldAtLevel6, "cmd-k cmd-6"),
            (CommandId::EditorFoldAtLevel7, "cmd-k cmd-7"),
            (CommandId::EditorFoldAtLevel8, "cmd-k cmd-8"),
            (CommandId::EditorFoldAtLevel9, "cmd-k cmd-9"),
            (CommandId::EditorFoldRecursive, "cmd-k cmd-["),
            (CommandId::EditorUnfoldAll, "cmd-k cmd-j"),
            (CommandId::EditorUnfoldLines, "cmd-alt-]"),
            (CommandId::EditorUnfoldRecursive, "cmd-k cmd-]"),
        ];
        for (id, binding) in cases {
            let command = commands
                .iter()
                .find(|command| command.id == id)
                .expect("fold command descriptor");
            assert_eq!(command.default_bindings[0].keystrokes, binding);
            assert!(matches!(
                route_for_command(id),
                Some(EditorCommandRoute::Ui(_))
            ));
        }
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
