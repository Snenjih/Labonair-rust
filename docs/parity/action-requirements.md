# Bound macOS Action Requirements Register

**Status:** Inventory IDs assigned; action semantics and runtime acceptance remain open  
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`  
**First acceptance platform:** macOS  
**Owner:** Product architecture with each canonical capability owner

## Purpose and status

This register assigns a stable requirement ID to every distinct action name with
at least one default binding in the pinned macOS profile. Each row retains the
key sequence and active context needed to reproduce the input. It is an
inventory, not evidence that an action exists or behaves correctly in Labonair.
The row count is 585. Unbound actions and non-macOS profiles are separate open
inventories and are not included in this count.

The exact user-visible meaning, availability conditions, canonical Labonair
owner, local action route, settings effects, and runtime parity are still
unclassified for every row unless later evidence is added to that row. Do not
implement from an action name alone. Do not treat an entry in this register as
present, partial, or accepted behavior.

## Shared acceptance contract inherited by every row

- **User goal and workflow:** Identify the observable user operation in the
  pinned runtime, then verify `key sequence → matching active context → owning
  operation → visible/stateful result`. The action-specific operation is
  `Pending` until observed and independently described.
- **States:** Record applicable available/unavailable, active/inactive context,
  normal, empty, loading, error, cancelled, and recovery states. Mark each
  inapplicable state `N/A` with a reason. Exact state sets are `Pending` until
  action classification.
- **Keyboard and mouse:** The row records default key sequences and context.
  Verify each sequence, repeat/timeout behavior, context exclusion, keyboard
  discoverability, and any pointer/menu entry that the runtime provides.
  Pointer routes are `Pending` until observed.
- **Settings and persistence:** Verify the selected keymap profile and any
  user/project override scope, validation, conflict behavior, save/reload,
  reset, and migration. Record domain settings changed by the action only
  after observing the effect. These values are `Pending` per action.
- **Owner and dependencies:** Assign one canonical Labonair feature owner and
  typed command/action contract. The action family below is a routing hint, not
  an accepted owner assignment. Exact owner and dependencies are `Pending`.
- **Acceptance evidence:** Record exact pinned and Labonair builds, OS/window,
  keymap profile, active and inactive contexts, input sequence, visible result,
  failure/cancel path, and artifact/test link. Runtime evidence is `Pending`.

The source-blind separation condition was not met for this work. This register
is part of an independently authored redevelopment record; it makes no claim
of separated-room provenance.

## Requirement rows

| Requirement ID | Action identifier | Pinned key sequence and active context | Workflow family | Semantic behavior | Canonical owner | Local route | Runtime evidence |
|---|---|---|---|---|---|---|---|
| `ACT-MACOS-editor-AcceptEditPrediction` | `editor::AcceptEditPrediction` | ⌥Tab (`Editor && edit_prediction`)<br />Tab (`Editor && edit_prediction && edit_prediction_mode == eager && !showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-AcceptNextLineEditPrediction` | `editor::AcceptNextLineEditPrediction` | ⌃⌘↓ (`Editor && edit_prediction`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-AcceptNextWordEditPrediction` | `editor::AcceptNextWordEditPrediction` | ⌃⌘→ (`Editor && edit_prediction`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-AddSelectionAbove` | `editor::AddSelectionAbove` | ⌘⌃P (`Editor`)<br />⌘⌥↑ (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-AddSelectionBelow` | `editor::AddSelectionBelow` | ⌘⌃N (`Editor`)<br />⌘⌥↓ (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Backspace` | `editor::Backspace` | ⇧⌫ (`Editor`)<br />⌃H (`Editor`)<br />⌫ (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Backtab` | `editor::Backtab` | ⇧Tab (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-BlameHover` | `editor::BlameHover` | ⌘K ⌘B (`Editor`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Cancel` | `editor::Cancel` | Esc (`Editor`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ComposeCompletion` | `editor::ComposeCompletion` | Tab (`Editor && showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ConfirmCodeAction` | `editor::ConfirmCodeAction` | ↩ (`Editor && showing_code_actions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ConfirmCompletion` | `editor::ConfirmCompletion` | ↩ (`Editor && showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ConfirmCompletionReplace` | `editor::ConfirmCompletionReplace` | ⇧↩ (`Editor && showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ConfirmRename` | `editor::ConfirmRename` | ↩ (`Editor && renaming`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ContextMenuFirst` | `editor::ContextMenuFirst` | PageUp (`Editor && (showing_code_actions \|\| showing_completions)`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ContextMenuLast` | `editor::ContextMenuLast` | PageDown (`Editor && (showing_code_actions \|\| showing_completions)`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ContextMenuNext` | `editor::ContextMenuNext` | ↓ (`Editor && (showing_code_actions \|\| showing_completions)`)<br />⌃N (`Editor && (showing_code_actions \|\| showing_completions)`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ContextMenuPrevious` | `editor::ContextMenuPrevious` | ↑ (`Editor && (showing_code_actions \|\| showing_completions)`)<br />⌃P (`Editor && (showing_code_actions \|\| showing_completions)`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Copy` | `editor::Copy` | ⌘C (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-CopyPath` | `editor::CopyPath` | ⌘K P (`Editor`) | Zusätzliche Editor-/OS-Aktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Cut` | `editor::Cut` | ⌘X (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Delete` | `editor::Delete` | ⌃D (`Editor`)<br />⌦ (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteLine` | `editor::DeleteLine` | ⌘⇧K (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToBeginningOfLine` | `editor::DeleteToBeginningOfLine` | ⌘⌫ (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToEndOfLine` | `editor::DeleteToEndOfLine` | ⌘⌦ (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToNextSubwordEnd` | `editor::DeleteToNextSubwordEnd` | ⌃⌥⌦ (`Editor`)<br />⌃⌥D (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToNextWordEnd` | `editor::DeleteToNextWordEnd` | ⌥⌦ (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToPreviousSubwordStart` | `editor::DeleteToPreviousSubwordStart` | ⌃⌥⌫ (`Editor`)<br />⌃⌥H (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DeleteToPreviousWordStart` | `editor::DeleteToPreviousWordStart` | ⌥⌫ (`Editor`)<br />⌃W (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DisplayCursorNames` | `editor::DisplayCursorNames` | ⌃⌘C (`<global>`) | Zusätzliche Editor-/OS-Aktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DuplicateLineDown` | `editor::DuplicateLineDown` | ⌥⇧↓ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-DuplicateLineUp` | `editor::DuplicateLineUp` | ⌥⇧↑ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-EditLogBreakpoint` | `editor::EditLogBreakpoint` | ⇧F9 (`Editor`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ExpandAllDiffHunks` | `editor::ExpandAllDiffHunks` | ⌘" (`Editor`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ExpandExcerpts` | `editor::ExpandExcerpts` | ⇧↩ (`!AcpThread > Editor && mode == full`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FindAllReferences` | `editor::FindAllReferences` | ⌥⇧F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Fold` | `editor::Fold` | ⌥⌘[ (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAll` | `editor::FoldAll` | ⌘K ⌘0 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_1` | `editor::FoldAtLevel_1` | ⌘K ⌘1 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_2` | `editor::FoldAtLevel_2` | ⌘K ⌘2 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_3` | `editor::FoldAtLevel_3` | ⌘K ⌘3 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_4` | `editor::FoldAtLevel_4` | ⌘K ⌘4 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_5` | `editor::FoldAtLevel_5` | ⌘K ⌘5 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_6` | `editor::FoldAtLevel_6` | ⌘K ⌘6 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_7` | `editor::FoldAtLevel_7` | ⌘K ⌘7 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_8` | `editor::FoldAtLevel_8` | ⌘K ⌘8 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldAtLevel_9` | `editor::FoldAtLevel_9` | ⌘K ⌘9 (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-FoldRecursive` | `editor::FoldRecursive` | ⌘K ⌘[ (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Format` | `editor::Format` | ⌘⇧I (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToDeclaration` | `editor::GoToDeclaration` | ⌃F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToDeclarationSplit` | `editor::GoToDeclarationSplit` | ⌥⌃F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToDefinition` | `editor::GoToDefinition` | F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToDefinitionSplit` | `editor::GoToDefinitionSplit` | ⌥F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToDiagnostic` | `editor::GoToDiagnostic` | F8 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToHunk` | `editor::GoToHunk` | ⌘F8 (`!AcpThread > Editor && mode == full`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToImplementation` | `editor::GoToImplementation` | ⇧F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToNextChange` | `editor::GoToNextChange` | ⌘⇧⌥⌫ (`Editor && mode == full`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToPreviousChange` | `editor::GoToPreviousChange` | ⌘⇧⌫ (`Editor && mode == full`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToPreviousDiagnostic` | `editor::GoToPreviousDiagnostic` | ⇧F8 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToPreviousHunk` | `editor::GoToPreviousHunk` | ⌘⇧F8 (`!AcpThread > Editor && mode == full`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToTypeDefinition` | `editor::GoToTypeDefinition` | ⌘F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-GoToTypeDefinitionSplit` | `editor::GoToTypeDefinitionSplit` | ⌥⌘F12 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Hover` | `editor::Hover` | ⌘K ⌘I (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Indent` | `editor::Indent` | ⌘] (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-JoinLines` | `editor::JoinLines` | ⌃J (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-KillRingCut` | `editor::KillRingCut` | ⌃K (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-KillRingYank` | `editor::KillRingYank` | ⌃Y (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-LineDown` | `editor::LineDown` | ⌃PageDown (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-LineUp` | `editor::LineUp` | ⌃PageUp (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveDown` | `editor::MoveDown` | ↓ (`Editor`)<br />⌃N (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveLeft` | `editor::MoveLeft` | ⌃B (`Editor`)<br />← (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveLineDown` | `editor::MoveLineDown` | ⌥↓ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveLineUp` | `editor::MoveLineUp` | ⌥↑ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MovePageDown` | `editor::MovePageDown` | PageDown (`Editor`)<br />⌃V (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MovePageUp` | `editor::MovePageUp` | PageUp (`Editor`)<br />⌃⇧V (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveRight` | `editor::MoveRight` | ⌃F (`Editor`)<br />→ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToBeginning` | `editor::MoveToBeginning` | ⌘↑ (`Editor`)<br />⌘Home (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToBeginningOfLine` | `editor::MoveToBeginningOfLine` | ⌘← (`Editor`)<br />⌃A (`Editor`)<br />Home (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToEnclosingBracket` | `editor::MoveToEnclosingBracket` | ⌘\| (`Editor`)<br />⌃M (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToEnd` | `editor::MoveToEnd` | ⌘↓ (`Editor`)<br />⌘End (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToEndOfLine` | `editor::MoveToEndOfLine` | ⌘→ (`Editor`)<br />⌃E (`Editor`)<br />End (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToEndOfParagraph` | `editor::MoveToEndOfParagraph` | ⌃↓ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToNextSubwordEnd` | `editor::MoveToNextSubwordEnd` | ⌃⌥→ (`Editor`)<br />⌃⌥F (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToNextWordEnd` | `editor::MoveToNextWordEnd` | ⌥→ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToPreviousSubwordStart` | `editor::MoveToPreviousSubwordStart` | ⌃⌥← (`Editor`)<br />⌃⌥B (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToPreviousWordStart` | `editor::MoveToPreviousWordStart` | ⌥← (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToStartOfExcerpt` | `editor::MoveToStartOfExcerpt` | ⌘↑ (`Editor && multibuffer`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToStartOfNextExcerpt` | `editor::MoveToStartOfNextExcerpt` | ⌘↓ (`Editor && multibuffer`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveToStartOfParagraph` | `editor::MoveToStartOfParagraph` | ⌃↑ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-MoveUp` | `editor::MoveUp` | ↑ (`Editor`)<br />⌃P (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Newline` | `editor::Newline` | ⇧↩ (`Editor && mode == full`)<br />↩ (`Editor && mode == full`)<br />⌃↩ (`Editor && mode == auto_height`)<br />⇧↩ (`Editor && mode == auto_height`)<br />⌥↩ (`AgentFeedbackMessageEditor > Editor`)<br />↩ (`AcpThread > Editor && use_modifier_to_send`)<br />⌃↩ (`BufferSearchBar && !in_replace > Editor`)<br />⌃↩ (`BufferSearchBar \|\| ProjectSearchBar`)<br />⌃↩ (`ProjectSearchBar && !in_replace > Editor`)<br />↩ (`CommitEditor > Editor`)<br />↩ (`GitCommit > Editor && mode == auto_height`)<br />↩ (`ConfigureContextServerModal > Editor`)<br />↩ (`NotebookEditor > Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-NewlineAbove` | `editor::NewlineAbove` | ⌘⇧↩ (`Editor && mode == full`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-NewlineBelow` | `editor::NewlineBelow` | ⌘↩ (`Editor && mode == full`)<br />⌃⇧↩ (`Editor && mode == auto_height`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-NextEditPrediction` | `editor::NextEditPrediction` | ⌥Tab (`Editor && mode == full && edit_prediction`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-NextSnippetTabstop` | `editor::NextSnippetTabstop` | Tab (`Editor && in_snippet && has_next_tabstop && !showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-OpenExcerpts` | `editor::OpenExcerpts` | ⌥↩ (`AcpThread > Editor && mode == full`)<br />⌥↩ (`!AcpThread > Editor && mode == full`)<br />⌥↩ (`OutlinePanel && not_editing`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-OpenExcerptsSplit` | `editor::OpenExcerptsSplit` | ⌘⌥↩ (`!AcpThread > Editor && mode == full`)<br />⌘⌥↩ (`OutlinePanel && not_editing`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-OpenSelectionsInMultibuffer` | `editor::OpenSelectionsInMultibuffer` | ⌥↩ (`Editor && mode == full`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-OrganizeImports` | `editor::OrganizeImports` | ⌥⇧O (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Outdent` | `editor::Outdent` | ⌘[ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-PageDown` | `editor::PageDown` | ⌘PageDown (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-PageUp` | `editor::PageUp` | ⌘PageUp (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Paste` | `editor::Paste` | ⌘V (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-PreviousEditPrediction` | `editor::PreviousEditPrediction` | ⌥⇧Tab (`Editor && mode == full && edit_prediction`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-PreviousSnippetTabstop` | `editor::PreviousSnippetTabstop` | ⇧Tab (`Editor && in_snippet && has_previous_tabstop && !showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Redo` | `editor::Redo` | ⌘⇧Z (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-RedoSelection` | `editor::RedoSelection` | ⌘⇧U (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Rename` | `editor::Rename` | F2 (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-RevealInFileManager` | `editor::RevealInFileManager` | ⌘K R (`Editor`)<br />⌘K R (`ImageViewer`) | Zusätzliche Editor-/OS-Aktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Rewrap` | `editor::Rewrap` | ⌘K ⌘Q (`Editor`)<br />⌘K Q (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ScrollCursorCenter` | `editor::ScrollCursorCenter` | ⌃L (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectAll` | `editor::SelectAll` | ⌘A (`Editor`)<br />⌘A (`Terminal`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectAllMatches` | `editor::SelectAllMatches` | ⌘⇧L (`Editor`)<br />⌘F2 (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectDown` | `editor::SelectDown` | ⇧↓ (`Editor`)<br />⌃⇧N (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectEnclosingSymbol` | `editor::SelectEnclosingSymbol` | ⌘⌥E (`Editor && mode == full`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectLargerSyntaxNode` | `editor::SelectLargerSyntaxNode` | ⌘⌃→ (`Editor`)<br />⌃⇧→ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectLeft` | `editor::SelectLeft` | ⇧← (`Editor`)<br />⌃⇧B (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectLine` | `editor::SelectLine` | ⌘L (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectNext` | `editor::SelectNext` | ⌘D (`Editor`)<br />⌘K ⌘D (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectNextSyntaxNode` | `editor::SelectNextSyntaxNode` | ⌘⌃↓ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectPageDown` | `editor::SelectPageDown` | ⇧PageDown (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectPageUp` | `editor::SelectPageUp` | ⇧PageUp (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectPrevious` | `editor::SelectPrevious` | ⌃⌘D (`Editor`)<br />⌘K ⌃⌘D (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectPreviousSyntaxNode` | `editor::SelectPreviousSyntaxNode` | ⌘⌃↑ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectRight` | `editor::SelectRight` | ⇧→ (`Editor`)<br />⌃⇧F (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectSmallerSyntaxNode` | `editor::SelectSmallerSyntaxNode` | ⌘⌃← (`Editor`)<br />⌃⇧← (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToBeginning` | `editor::SelectToBeginning` | ⌘⇧↑ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToBeginningOfLine` | `editor::SelectToBeginningOfLine` | ⌘⇧← (`Editor`)<br />⇧Home (`Editor`)<br />⌃⇧A (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToEnd` | `editor::SelectToEnd` | ⌘⇧↓ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToEndOfLine` | `editor::SelectToEndOfLine` | ⌘⇧→ (`Editor`)<br />⇧End (`Editor`)<br />⌃⇧E (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToEndOfParagraph` | `editor::SelectToEndOfParagraph` | ⌃⇧↓ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToNextSubwordEnd` | `editor::SelectToNextSubwordEnd` | ⌃⌥⇧→ (`Editor`)<br />⌃⌥⇧F (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToNextWordEnd` | `editor::SelectToNextWordEnd` | ⌥⇧→ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToPreviousSubwordStart` | `editor::SelectToPreviousSubwordStart` | ⌃⌥⇧← (`Editor`)<br />⌃⌥⇧B (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToPreviousWordStart` | `editor::SelectToPreviousWordStart` | ⌥⇧← (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToStartOfExcerpt` | `editor::SelectToStartOfExcerpt` | ⌘⇧↑ (`Editor && multibuffer`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToStartOfNextExcerpt` | `editor::SelectToStartOfNextExcerpt` | ⌘⇧↓ (`Editor && multibuffer`) | Excerpts und Multi-Buffers | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectToStartOfParagraph` | `editor::SelectToStartOfParagraph` | ⌃⇧↑ (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SelectUp` | `editor::SelectUp` | ⇧↑ (`Editor`)<br />⌃⇧P (`Editor`) | Bewegen, Auswählen und Löschen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ShowCharacterPalette` | `editor::ShowCharacterPalette` | ⌃⌘Space (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ShowCompletions` | `editor::ShowCompletions` | ⌃Space (`Editor`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ShowEditPrediction` | `editor::ShowEditPrediction` | ⌥Tab (`Editor && !edit_prediction`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ShowSignatureHelp` | `editor::ShowSignatureHelp` | ⌘I (`Editor`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ShowWordCompletions` | `editor::ShowWordCompletions` | ⌃⇧Space (`Editor`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SignatureHelpNext` | `editor::SignatureHelpNext` | ↓ (`Editor && showing_signature_help && !showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-SignatureHelpPrevious` | `editor::SignatureHelpPrevious` | ↑ (`Editor && showing_signature_help && !showing_completions`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Tab` | `editor::Tab` | Tab (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleBlockComments` | `editor::ToggleBlockComments` | ⌘K ⌘/ (`Editor`)<br />⇧⌥A (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleBreakpoint` | `editor::ToggleBreakpoint` | F9 (`Editor`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleCodeActions` | `editor::ToggleCodeActions` | ⌘. (`Editor`) | Sprachaktionen | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleComments` | `editor::ToggleComments` | ⌘/ (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleEditPrediction` | `editor::ToggleEditPrediction` | ⌃⌘E (`Editor`) | Suggestions and modal input | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleFold` | `editor::ToggleFold` | ⌘K ⌘L (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleFoldAll` | `editor::ToggleFoldAll` | ⌘⇧↩ (`BufferSearchBar`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleInlayHints` | `editor::ToggleInlayHints` | ⌃: (`!AcpThread > Editor && mode == full`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleLineNumbers` | `editor::ToggleLineNumbers` | ⌘; (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleSelectedDiffHunks` | `editor::ToggleSelectedDiffHunks` | ⌘' (`Editor`) | Git, diff, and debugger actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-ToggleSoftWrap` | `editor::ToggleSoftWrap` | ⌘K Z (`Editor && mode == full`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Transpose` | `editor::Transpose` | ⌃T (`Editor`) | Struktur- und Zeilenbearbeitung | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-Undo` | `editor::Undo` | ⌘Z (`Editor`) | Text input/output and clipboard | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-UndoSelection` | `editor::UndoSelection` | ⌘U (`Editor`) | Multi-selection | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-UnfoldAll` | `editor::UnfoldAll` | ⌘K ⌘J (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-UnfoldLines` | `editor::UnfoldLines` | ⌥⌘] (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-editor-UnfoldRecursive` | `editor::UnfoldRecursive` | ⌘K ⌘] (`Editor`) | Structure and display actions | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Amend` | `git::Amend` | ⌘⇧↩ (`GitDiff > Editor`)<br />⌘⇧↩ (`CommitEditor > Editor`)<br />⌘⇧↩ (`GitPanel`)<br />⌘⇧↩ (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ApplyCurrentStash` | `git::ApplyCurrentStash` | ⌃Space (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Blame` | `git::Blame` | ⌘⌥G B (`Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Cancel` | `git::Cancel` | Esc (`GitPanel && CommitEditor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Commit` | `git::Commit` | ⌘↩ (`GitDiff > Editor`)<br />⌘↩ (`CommitEditor > Editor`)<br />⌘↩ (`GitPanel`)<br />⌘↩ (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Diff` | `git::Diff` | ⇧⌃D (`AcpThread > Editor`)<br />⌃G D (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-DropCurrentStash` | `git::DropCurrentStash` | ⌃⇧⌫ (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ExpandCommitEditor` | `git::ExpandCommitEditor` | ⇧Esc (`CommitEditor > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Fetch` | `git::Fetch` | ⌃G ⌃G (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ForcePush` | `git::ForcePush` | ⌃G ⇧↑ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-GenerateCommitMessage` | `git::GenerateCommitMessage` | ⌥Tab (`CommitEditor > Editor`)<br />⌥Tab (`GitCommit > Editor && mode == auto_height`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-OpenModifiedFiles` | `git::OpenModifiedFiles` | ⌘⌥G M (`Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-PopCurrentStash` | `git::PopCurrentStash` | ⌃⇧Space (`StashDiff > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Pull` | `git::Pull` | ⌃G ↓ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-PullRebase` | `git::PullRebase` | ⌃G ⇧↓ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Push` | `git::Push` | ⌃G ↑ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Restore` | `git::Restore` | ⌘⌥Z (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-RestoreAndNext` | `git::RestoreAndNext` | ⌘⌥Z (`GitDiff > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-RestoreFile` | `git::RestoreFile` | ⌫ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />⌦ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />⌘⌫ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />⌘⌦ (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-RestoreTrackedFiles` | `git::RestoreTrackedFiles` | ⌃G ⌫ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ReviewDiff` | `git::ReviewDiff` | ⌘⌥G R (`Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-StageAll` | `git::StageAll` | ⌘⌃Y (`GitDiff > Editor`)<br />⌘⌃Y (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-StageAndNext` | `git::StageAndNext` | ⌘Y (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-StageFile` | `git::StageFile` | ⌘Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-StageRange` | `git::StageRange` | ⇧Space (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ToggleFillCommitEditor` | `git::ToggleFillCommitEditor` | ⌥⇧Esc (`CommitEditor > Editor`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-ToggleStaged` | `git::ToggleStaged` | ⌘⌥Y (`Editor && !agent_diff && !AgentPanel`)<br />⌘⌥Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />Space (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-TrashUntrackedFiles` | `git::TrashUntrackedFiles` | ⌃G ⇧⌫ (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-UnstageAll` | `git::UnstageAll` | ⌘⌃⇧Y (`GitDiff > Editor`)<br />⌘⌃⇧Y (`GitPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-UnstageAndNext` | `git::UnstageAndNext` | ⌘⇧Y (`Editor && !agent_diff && !AgentPanel`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-UnstageFile` | `git::UnstageFile` | ⌘⇧Y (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git-Worktree` | `git::Worktree` | ⌘⌃W (`Workspace`) | Git- und Diffaktionen / Git-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-CloseAndReturnToEditor` | `markdown::CloseAndReturnToEditor` | ⌘⇧V (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-Copy` | `markdown::Copy` | ⌘C (`Markdown`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-CopyAsMarkdown` | `markdown::CopyAsMarkdown` | ⌘C (`AgentPanel > Markdown`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-OpenPreview` | `markdown::OpenPreview` | ⌘⇧V (`Editor && extension == md`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-OpenPreviewToTheSide` | `markdown::OpenPreviewToTheSide` | ⌘K V (`Editor && extension == md`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollDown` | `markdown::ScrollDown` | ↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollDownByItem` | `markdown::ScrollDownByItem` | ⌥↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollPageDown` | `markdown::ScrollPageDown` | PageDown (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollPageUp` | `markdown::ScrollPageUp` | PageUp (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollToBottom` | `markdown::ScrollToBottom` | ⌘↓ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollToTop` | `markdown::ScrollToTop` | ⌘↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollUp` | `markdown::ScrollUp` | ↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-markdown-ScrollUpByItem` | `markdown::ScrollUpByItem` | ⌥↑ (`MarkdownPreview`) | Markdown-/SVG-Preview / Markdown-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-FocusSearch` | `search::FocusSearch` | ⌘F (`AcpThreadSearchBar`)<br />⌘F (`BufferSearchBar`)<br />⌘⇧F (`ProjectSearchBar`)<br />⌘F (`KeymapEditor`)<br />⌘F (`SettingsWindow`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-NextHistoryQuery` | `search::NextHistoryQuery` | ↓ (`BufferSearchBar && !in_replace > Editor`)<br />↓ (`ProjectSearchBar > Editor`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-PreviousHistoryQuery` | `search::PreviousHistoryQuery` | ↑ (`BufferSearchBar && !in_replace > Editor`)<br />↑ (`ProjectSearchBar > Editor`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ReplaceAll` | `search::ReplaceAll` | ⌘↩ (`BufferSearchBar && in_replace > Editor`)<br />⌘↩ (`ProjectSearchBar && in_replace > Editor`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ReplaceNext` | `search::ReplaceNext` | ↩ (`BufferSearchBar && in_replace > Editor`)<br />↩ (`ProjectSearchBar && in_replace > Editor`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-SelectAllMatches` | `search::SelectAllMatches` | ⌥↩ (`BufferSearchBar`)<br />⌥↩ (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-SelectNextMatch` | `search::SelectNextMatch` | ↩ (`BufferSearchBar`)<br />⌘G (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-SelectPreviousMatch` | `search::SelectPreviousMatch` | ⇧↩ (`BufferSearchBar`)<br />⇧↩ (`BufferSearchBar && !in_replace > Editor`)<br />⌘⇧G (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleCaseSensitive` | `search::ToggleCaseSensitive` | ⌥⌘C (`AcpThread`)<br />⌥⌘C (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleIncludeIgnored` | `search::ToggleIncludeIgnored` | ⌘⇧I (`FileFinder \|\| (FileFinder > Picker > Editor)`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleRegex` | `search::ToggleRegex` | ⌥⌘X (`AcpThread`)<br />⌥⌘G (`ProjectSearchBar`)<br />⌥⌘X (`ProjectSearchBar`)<br />⌥⌘G (`ProjectSearchView`)<br />⌥⌘X (`ProjectSearchView`)<br />⌥⌘X (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleReplace` | `search::ToggleReplace` | ⌘⇧H (`ProjectSearchBar`)<br />⌘⇧H (`ProjectSearchView`)<br />⌘⇧H (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleSelection` | `search::ToggleSelection` | ⌘⌥L (`BufferSearchBar`)<br />⌘⌥L (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-search-ToggleWholeWord` | `search::ToggleWholeWord` | ⌥⌘W (`AcpThread`)<br />⌥⌘W (`Pane`) | Suchaktionen / Search-Owner | Pending | Pending | Pending | Pending |
| `ACT-MACOS-svg-OpenPreview` | `svg::OpenPreview` | ⌘⇧V (`Editor && extension == svg`) | Markdown-/SVG-Preview / SVG derzeit extern | Pending | Pending | Pending | Pending |
| `ACT-MACOS-svg-OpenPreviewToTheSide` | `svg::OpenPreviewToTheSide` | ⌘K V (`Editor && extension == svg`) | Markdown-/SVG-Preview / SVG derzeit extern | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-ActivateItem` | `pane::ActivateItem` | `ctrl-1 args=0` (`Pane`)<br />`ctrl-2 args=1` (`Pane`)<br />`ctrl-3 args=2` (`Pane`)<br />`ctrl-4 args=3` (`Pane`)<br />`ctrl-5 args=4` (`Pane`)<br />`ctrl-6 args=5` (`Pane`)<br />`ctrl-7 args=6` (`Pane`)<br />`ctrl-8 args=7` (`Pane`)<br />`ctrl-9 args=8` (`Pane`) | Ähnlich sind Labonairs tab::Select1–Select9 (⌘1–⌘9), aber sie wählen globale Workspace-Tabs. Zed wählt ein Item im aktiven Pane (⌃1–⌃9). | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-ActivateLastItem` | `pane::ActivateLastItem` | `ctrl-0` (`Pane`) | Kein entsprechender Last-Item-Befehl in der lokalen Command Registry oder macOS-Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-ActivateNextItem` | `pane::ActivateNextItem` | `alt-cmd-right` (`Pane`)<br />`cmd-}` (`Pane`)<br />`ctrl-tab` (`RunModal`) | Labonair tab::Next nutzt ebenfalls ⌃Tab; es wechselt jedoch Shell-Tabs statt Items innerhalb eines Zed-Panes. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-ActivatePreviousItem` | `pane::ActivatePreviousItem` | `alt-cmd-left` (`Pane`)<br />`cmd-{` (`Pane`)<br />`ctrl-shift-tab` (`RunModal`) | Labonair tab::Prev nutzt ebenfalls ⇧⌃Tab; es wechselt Shell-Tabs statt Items innerhalb eines Zed-Panes. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseActiveItem` | `pane::CloseActiveItem` | `cmd-w args={"close_pinned":false}` (`Pane`) | Labonair tab::Close (⌘W) schließt den aktiven Workspace-Tab; Zeds Pane-Aktion gilt für das aktive Pane-Item und kann andere View-Arten umfassen. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseAllItems` | `pane::CloseAllItems` | `cmd-k w args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen aller Items des aktiven Panes gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseCleanItems` | `pane::CloseCleanItems` | `cmd-k u args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen nur sauberer (unveränderter) Items gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseItemsToTheLeft` | `pane::CloseItemsToTheLeft` | `cmd-k e args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen der Pane-Items links des aktiven Items gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseItemsToTheRight` | `pane::CloseItemsToTheRight` | `cmd-k t args={"close_pinned":false}` (`Pane`) | Kein Befehl zum Schließen der Pane-Items rechts des aktiven Items gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-CloseOtherItems` | `pane::CloseOtherItems` | `alt-cmd-t args={"close_pinned":false}` (`Pane`) | Nächster Treffer ist tab::CloseOthers. Der lokale Befehl hat keinen gepinnten-Tab-Zustand; exakte Schließ- und Dirty-Tab-Semantik bleibt abzugleichen. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-DeploySearch` | `pane::DeploySearch` | `cmd-shift-f` (`Pane`)<br />`cmd-shift-f` (`Workspace`)<br />`cmd-shift-h args={"replace_enabled":true}` (`Workspace`) | Labonairs search::Toggle / Find öffnet die aktuelle Suche mit ⌘F. Zeds ⌘⇧F und replace_enabled-Konfiguration eröffnen einen anderen Scope und Zustand. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-GoBack` | `pane::GoBack` | `ctrl--` (`AcpThread`)<br />`ctrl--` (`ThreadHistory`)<br />`ctrl--` (`Pane`) | Kein Editor-/Pane-Navigationsverlauf-Befehl in der lokalen Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-GoForward` | `pane::GoForward` | `ctrl-_` (`Pane`) | Kein Editor-/Pane-Vorwärtsverlauf-Befehl in der lokalen Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-ReopenClosedItem` | `pane::ReopenClosedItem` | `cmd-shift-t` (`Workspace`) | Kein zuletzt geschlossenes Item erneut öffnen. Sitzungs-Wiederherstellung für Tabs ist kein gleicher Undo-Schließvorgang. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-RevealInProjectPanel` | `pane::RevealInProjectPanel` | `cmd-shift-e` (`!AcpThread > Editor && mode == full`) | sidebar::Toggle (⌘B) zeigt/versteckt den lokalen Datei-Explorer; eine Aktion zum ausgewählten Editorfile im Explorer aufklappen/markieren wurde nicht gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SplitDown` | `pane::SplitDown` | `cmd-k down` (`Pane`)<br />`ctrl-alt-down` (`Terminal`) | Lokales pane::SplitDown (⌘⇧D) ist im Terminal-Kontext verfügbar; EditorSplitDown teilt die interne Editorgruppe. Zed bietet Pane-Splits als Workspace-Aktion. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SplitLeft` | `pane::SplitLeft` | `cmd-k left` (`Pane`)<br />`ctrl-alt-left` (`Terminal`) | Kein linker Workspace-Pane-Split-Befehl gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SplitRight` | `pane::SplitRight` | `cmd-\` (`Editor`)<br />`cmd-k right` (`Pane`)<br />`ctrl-alt-right` (`Terminal`)<br />`cmd-d` (`Terminal`) | Lokales pane::SplitRight (⌘D) ist im Terminal-Kontext verfügbar; EditorSplitRight teilt die interne Editorgruppe (⌘⌥]). Die Oberflächen und Scopes unterscheiden sich. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SplitUp` | `pane::SplitUp` | `cmd-k up` (`Pane`)<br />`ctrl-alt-up` (`Terminal`) | Kein oberer Workspace-Pane-Split-Befehl gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SwapItemLeft` | `pane::SwapItemLeft` | `ctrl-shift-pageup` (`Pane`) | Keine Tastaturaktion zum Umordnen eines Pane-Items links gefunden; visuelle Drag-Interaktion ist hier nicht bewertet. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-SwapItemRight` | `pane::SwapItemRight` | `ctrl-shift-pagedown` (`Pane`) | Keine Tastaturaktion zum Umordnen eines Pane-Items rechts gefunden; visuelle Drag-Interaktion ist hier nicht bewertet. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-pane-TogglePinTab` | `pane::TogglePinTab` | `cmd-k shift-enter` (`Pane`) | Kein Pin-/Unpin-Tab-Zustand oder zugehöriger lokaler Command gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ActivatePane` | `workspace::ActivatePane` | `cmd-1 args=0` (`Workspace`)<br />`cmd-2 args=1` (`Workspace`)<br />`cmd-3 args=2` (`Workspace`)<br />`cmd-4 args=3` (`Workspace`)<br />`cmd-5 args=4` (`Workspace`)<br />`cmd-6 args=5` (`Workspace`)<br />`cmd-7 args=6` (`Workspace`)<br />`cmd-8 args=7` (`Workspace`)<br />`cmd-9 args=8` (`Workspace`) | Labonair cmd-1…cmd-9 wählt Workspace-Tabs (tab::Select1–Select9); Zed cmd-1…cmd-9 aktiviert das jeweilige Pane. Gleiches Shortcutmuster, anderer Container. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ActivatePaneDown` | `workspace::ActivatePaneDown` | `cmd-k cmd-down` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach unten fehlt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ActivatePaneLeft` | `workspace::ActivatePaneLeft` | `cmd-k cmd-left` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach links fehlt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ActivatePaneRight` | `workspace::ActivatePaneRight` | `cmd-k cmd-right` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach rechts fehlt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ActivatePaneUp` | `workspace::ActivatePaneUp` | `cmd-k cmd-up` (`Workspace`) | Lokales pane::FocusNext (⌘]) rotiert durch Panes; eine richtungsbezogene Aktivierung nach oben fehlt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-AddFolderToProject` | `workspace::AddFolderToProject` | `cmd-shift-a` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) | Kein Add-Folder-to-Project-Befehl in der lokalen Command Registry gefunden; Projekt öffnen und Standalone-Modus wechseln sind andere Abläufe. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CloseActiveDock` | `workspace::CloseActiveDock` | `cmd-w` (`Workspace`) | ⌘W ist lokal tab::Close; sidebar::Toggle schaltet nur den Datei-Explorer. Kein allgemeiner Close-Active-Dock-Befehl gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CloseAllItemsAndPanes` | `workspace::CloseAllItemsAndPanes` | `cmd-k cmd-w` (`Pane`) | Kein Workspace-Befehl zum Schließen aller Items und Panes gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CloseInactiveTabsAndPanes` | `workspace::CloseInactiveTabsAndPanes` | `ctrl-alt-cmd-w` (`Pane`) | tab::CloseOthers schließt andere Tabs; ein gemeinsames Schließen inaktiver Tabs und Panes ist nicht belegt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CloseWindow` | `workspace::CloseWindow` | `cmd-shift-w` (`<global>`)<br />`cmd-w` (`SettingsWindow`)<br />`escape` (`SettingsWindow`)<br />`cmd-w` (`SkillCreator`)<br />`cmd-w` (`SkillCreator > Editor`) | Kein entsprechendes lokales Window-Close-Keybinding in der User-Keymap. Native Fensterschließung und ⌘Q (App beenden) sind andere Aktionen. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CopyPath` | `workspace::CopyPath` | `cmd-alt-c` (`OutlinePanel && not_editing`)<br />`cmd-alt-c` (`ProjectPanel`)<br />`cmd-alt-c` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Kein Copy-Path-Command oder Keybinding im lokalen Workspace-/Command-Palette-Owner gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-CopyRelativePath` | `workspace::CopyRelativePath` | `alt-cmd-shift-c` (`OutlinePanel && not_editing`)<br />`alt-cmd-shift-c` (`ProjectPanel`)<br />`alt-cmd-shift-c` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Kein Copy-Relative-Path-Command oder Keybinding im lokalen Workspace-/Command-Palette-Owner gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-DecreaseActiveDockSize` | `workspace::DecreaseActiveDockSize` | `ctrl-alt-- args={"px":0}` (`Workspace`) | Kein Dock-Größenbefehl in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-DecreaseOpenDocksSize` | `workspace::DecreaseOpenDocksSize` | `ctrl-alt-_ args={"px":0}` (`Workspace`) | Kein gemeinsamer Größenbefehl für geöffnete Docks in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-FocusNextPart` | `workspace::FocusNextPart` | `f6` (`Workspace`)<br />`cmd-f6` (`Workspace`) | pane::FocusNext rotiert nur durch Workspace-Panes; Fokusnavigation über Editor, Panels und Statusbar (⌘F6/F6) ist nicht belegt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-FocusPreviousPart` | `workspace::FocusPreviousPart` | `shift-f6` (`Workspace`) | Kein Fokus-zurück-über-Fensterbereiche-Befehl gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-FollowNextCollaborator` | `workspace::FollowNextCollaborator` | `ctrl-alt-cmd-f` (`<global>`) | Kein Collaborator-Follow-Befehl oder kollaborativer Workspace-Command gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-IncreaseActiveDockSize` | `workspace::IncreaseActiveDockSize` | `ctrl-alt-= args={"px":0}` (`Workspace`) | Kein Dock-Größenbefehl in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-IncreaseOpenDocksSize` | `workspace::IncreaseOpenDocksSize` | `ctrl-alt-+ args={"px":0}` (`Workspace`) | Kein gemeinsamer Größenbefehl für geöffnete Docks in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-NewFile` | `workspace::NewFile` | `cmd-n` (`Workspace && !Terminal`)<br />`cmd-n` (`Welcome`) | Der lokale Explorer bietet „New File“ im Kontextmenü (`crates/panel-explorer/src/panel_explorer.rs`); ein Palette-Command oder Default-Keybinding dafür fehlt. Zed bietet zusätzlich den direkten Workspace-Shortcut cmd-n. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-NewTerminal` | `workspace::NewTerminal` | `ctrl-~` (`Workspace`)<br />`cmd-n` (`Terminal`) | NewTerminalTab (⌘T) erstellt lokal einen Terminal-Workspace-Tab; Zed cmd-n ist im Terminal-Kontext und erzeugt ein neues Terminal-Item. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-NewWindow` | `workspace::NewWindow` | `cmd-shift-n` (`Workspace`) | Kein lokaler New-Window-Command oder anpassbares Keybinding gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-Open` | `workspace::Open` | `cmd-o` (`<global>`) | OpenFile (⌘⇧O) bietet lokale Dateiöffnung; Zed workspace::Open ist auf ⌘O gebunden. Gleicher Grundworkflow, anderer Shortcut und Action-Owner. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-OpenWithSystem` | `workspace::OpenWithSystem` | `ctrl-shift-enter` (`ProjectPanel`)<br />`ctrl-shift-enter` (`InvalidBuffer`) | Kein Dateimanager-Open-With-System-Command gefunden. Die Preview-Ansicht kann bestimmte URLs extern öffnen, das ist kein ProjectPanel-Datei-Workflow. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ReopenLastPicker` | `workspace::ReopenLastPicker` | `cmd-k cmd-p` (`Workspace`) | Kein Reopen-Last-Picker-Befehl gefunden; ⌘P toggelt lokal die Command Palette, nicht den zuletzt verwendeten Picker. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ResetActiveDockSize` | `workspace::ResetActiveDockSize` | `ctrl-alt-0` (`Workspace`) | Kein Reset für die Größe eines aktiven Docks gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ResetOpenDocksSize` | `workspace::ResetOpenDocksSize` | `ctrl-alt-)` (`Workspace`) | Kein Reset für die Größen aller offenen Docks gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-Save` | `workspace::Save` | `cmd-s` (`Workspace`) | tab::Save (⌘S) ist der direkte lokale Save-Einstieg; Laufzeit-/Dirty-Buffer-Abdeckung bleibt UI-seitig zu vergleichen. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SaveAll` | `workspace::SaveAll` | `cmd-alt-s` (`Workspace`) | Kein Save-All-Befehl oder Keybinding gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SaveAs` | `workspace::SaveAs` | `cmd-shift-s` (`Workspace`) | Kein Save-As-Befehl oder Keybinding gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SaveWithoutFormat` | `workspace::SaveWithoutFormat` | `cmd-k s` (`Workspace`) | Kein Save-Without-Format-Befehl gefunden; lokales Save hat keinen getrennten Formatting-Bypass-Shortcut. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SwapPaneDown` | `workspace::SwapPaneDown` | `cmd-k shift-down` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach unten gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SwapPaneLeft` | `workspace::SwapPaneLeft` | `cmd-k shift-left` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach links gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SwapPaneRight` | `workspace::SwapPaneRight` | `cmd-k shift-right` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach rechts gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-SwapPaneUp` | `workspace::SwapPaneUp` | `cmd-k shift-up` (`Workspace`) | Keine Tastaturaktion zum Umordnen von Workspace-Panes nach oben gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleAllDocks` | `workspace::ToggleAllDocks` | `alt-cmd-y` (`Workspace`) | Kein Toggle für alle Docks in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleBottomDock` | `workspace::ToggleBottomDock` | `cmd-j` (`Workspace`) | Kein Bottom-Dock-Toggle-Keybinding gefunden; lokale Panels werden über eigene Oberflächen/Statusbar kontrolliert. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleLeftDock` | `workspace::ToggleLeftDock` | `cmd-b` (`Workspace`) | sidebar::Toggle (⌘B) schaltet den Datei-Explorer links; die lokale Sidebar ist nicht deckungsgleich mit Zeds frei belegbarem Left Dock. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleRightDock` | `workspace::ToggleRightDock` | `cmd-alt-b` (`Workspace`)<br />`cmd-r` (`Workspace`) | Kein Right-Dock-Toggle-Befehl oder Keybinding gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleWorktreeSecurity` | `workspace::ToggleWorktreeSecurity` | `ctrl-cmd-s` (`<global>`) | Kein Worktree-Security-Toggle in der lokalen Command Registry oder Keymap gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-ToggleZoom` | `workspace::ToggleZoom` | `shift-escape` (`<global>`) | Ähnlich sind view::ToggleZenMode (⌘⇧Z) und der debug-only Dock-Zoom. Kein regulärer Zoom des aktiven Workspace-Panes ist belegt. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-workspace-Unfollow` | `workspace::Unfollow` | `escape` (`Workspace`) | Kein Collaboration-Unfollow-Command gefunden. | Pending | Pending | Pending | Pending |
| `ACT-MACOS-buffer_search-Deploy` | `buffer_search::Deploy` | `cmd-f` (`Editor && mode == full`)<br />`cmd-alt-l args={"selection_search_enabled":true}` (`Editor && mode == full`)<br />`cmd-f` (`Terminal`)<br />`cmd-f` (`MarkdownPreview`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-buffer_search-Dismiss` | `buffer_search::Dismiss` | `escape` (`BufferSearchBar`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-buffer_search-FocusEditor` | `buffer_search::FocusEditor` | `tab` (`BufferSearchBar`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-buffer_search-UseSelectionForFind` | `buffer_search::UseSelectionForFind` | `cmd-e` (`Editor && mode == full`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-call_hierarchy-ShowIncomingCalls` | `call_hierarchy::ShowIncomingCalls` | `cmd-k cmd-h` (`Editor`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-call_hierarchy-ToggleDirection` | `call_hierarchy::ToggleDirection` | `cmd-k cmd-h` (`CallHierarchyPicker > Picker > Editor`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-diagnostics-Deploy` | `diagnostics::Deploy` | `cmd-shift-m` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-diagnostics-ToggleDiagnosticsRefresh` | `diagnostics::ToggleDiagnosticsRefresh` | `ctrl-r` (`Diagnostics`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-encoding_selector-Toggle` | `encoding_selector::Toggle` | `cmd-k n` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-file_finder-Toggle` | `file_finder::Toggle` | `cmd-p` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-go_to_line-Toggle` | `go_to_line::Toggle` | `ctrl-g` (`Editor && mode == full`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-language_selector-Toggle` | `language_selector::Toggle` | `cmd-k m` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-lsp_tool-ToggleMenu` | `lsp_tool::ToggleMenu` | `ctrl-cmd-l` (`<global>`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-multi_workspace-FocusWorkspaceSidebar` | `multi_workspace::FocusWorkspaceSidebar` | `cmd-alt-;` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-multi_workspace-ToggleWorkspaceSidebar` | `multi_workspace::ToggleWorkspaceSidebar` | `cmd-alt-j` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline-Toggle` | `outline::Toggle` | `cmd-shift-o` (`BufferSearchBar`)<br />`cmd-shift-o` (`Editor && mode == full`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline_panel-CollapseSelectedEntry` | `outline_panel::CollapseSelectedEntry` | `left` (`OutlinePanel && not_editing`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline_panel-ExpandSelectedEntry` | `outline_panel::ExpandSelectedEntry` | `right` (`OutlinePanel && not_editing`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline_panel-OpenSelectedEntry` | `outline_panel::OpenSelectedEntry` | `space` (`OutlinePanel && not_editing`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline_panel-RevealInFileManager` | `outline_panel::RevealInFileManager` | `alt-cmd-r` (`OutlinePanel && not_editing`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-outline_panel-ToggleFocus` | `outline_panel::ToggleFocus` | `cmd-shift-b` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-CollapseAllEntries` | `project_panel::CollapseAllEntries` | `cmd-left` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-CollapseSelectedEntry` | `project_panel::CollapseSelectedEntry` | `left` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-CompareMarkedFiles` | `project_panel::CompareMarkedFiles` | `alt-d` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Copy` | `project_panel::Copy` | `cmd-c` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Cut` | `project_panel::Cut` | `cmd-x` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Delete` | `project_panel::Delete` | `cmd-delete args={"skip_prompt":false}` (`ProjectPanel`)<br />`cmd-alt-backspace args={"skip_prompt":false}` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Duplicate` | `project_panel::Duplicate` | `cmd-d` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-ExpandAllEntries` | `project_panel::ExpandAllEntries` | `cmd-right` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-ExpandSelectedEntry` | `project_panel::ExpandSelectedEntry` | `right` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-NewDirectory` | `project_panel::NewDirectory` | `alt-cmd-n` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-NewFile` | `project_panel::NewFile` | `cmd-n` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-NewSearchInDirectory` | `project_panel::NewSearchInDirectory` | `cmd-alt-shift-f` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Open` | `project_panel::Open` | `space` (`ProjectPanel && not_editing`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Paste` | `project_panel::Paste` | `cmd-v` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Redo` | `project_panel::Redo` | `cmd-shift-z` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Rename` | `project_panel::Rename` | `enter` (`ProjectPanel`)<br />`f2` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-RevealInFileManager` | `project_panel::RevealInFileManager` | `alt-cmd-r` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-ToggleFocus` | `project_panel::ToggleFocus` | `cmd-shift-e` (`AgentPanel`)<br />`cmd-shift-e` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Trash` | `project_panel::Trash` | `backspace args={"skip_prompt":false}` (`ProjectPanel`)<br />`delete args={"skip_prompt":false}` (`ProjectPanel`)<br />`cmd-backspace args={"skip_prompt":true}` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_panel-Undo` | `project_panel::Undo` | `cmd-z` (`ProjectPanel`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_search-OpenTextFinder` | `project_search::OpenTextFinder` | `alt-cmd-f` (`ProjectSearchBar`)<br />`alt-cmd-f` (`ProjectSearchView`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_search-SearchInNew` | `project_search::SearchInNew` | `cmd-enter` (`ProjectSearchBar && !in_replace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_search-ToggleAllSearchResults` | `project_search::ToggleAllSearchResults` | `cmd-shift-enter` (`ProjectSearchBar`)<br />`cmd-shift-enter` (`ProjectSearchView`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_search-ToggleFilters` | `project_search::ToggleFilters` | `cmd-shift-j` (`ProjectSearchBar`)<br />`cmd-shift-j` (`ProjectSearchView`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_search-ToggleFocus` | `project_search::ToggleFocus` | `escape` (`ProjectSearchBar`)<br />`escape` (`ProjectSearchView`)<br />`cmd-f` (`Pane`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-project_symbols-Toggle` | `project_symbols::Toggle` | `cmd-t` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-projects-OpenRecent` | `projects::OpenRecent` | `alt-cmd-o` (`Workspace`)<br />`ctrl-r` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-projects-OpenRemote` | `projects::OpenRemote` | `ctrl-cmd-o args={"from_existing_connection":false}` (`Workspace`)<br />`ctrl-cmd-shift-o args={"from_existing_connection":true}` (`Workspace`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-recent_projects-AddToWorkspace` | `recent_projects::AddToWorkspace` | `cmd-shift-enter` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-recent_projects-RemoveSelected` | `recent_projects::RemoveSelected` | `shift-backspace` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-recent_projects-ToggleActionsMenu` | `recent_projects::ToggleActionsMenu` | `cmd-k` (`RecentProjects \|\| (RecentProjects > Picker > Editor)`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-text_finder-Toggle` | `text_finder::Toggle` | `alt-cmd-f` (`Editor && mode == full`)<br />`alt-cmd-f` (`BufferSearchBar`) | Project, files, search, and language tools | Pending | Pending | Pending | Pending |
| `ACT-MACOS-branch_picker-CycleBranchFilter` | `branch_picker::CycleBranchFilter` | `cmd-shift-i` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-branch_picker-DeleteBranch` | `branch_picker::DeleteBranch` | `cmd-shift-backspace` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-branch_picker-ForceDeleteBranch` | `branch_picker::ForceDeleteBranch` | `cmd-alt-shift-backspace` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-branch_picker-ToggleFilterMenu` | `branch_picker::ToggleFilterMenu` | `cmd-k` (`GitBranchSelector \|\| (GitBranchSelector > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-branches-OpenRecent` | `branches::OpenRecent` | `cmd-ctrl-b` (`Workspace`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_graph-FocusNextTabStop` | `git_graph::FocusNextTabStop` | `tab` (`GitGraph`)<br />`tab` (`GitGraphSearchBar > Editor`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_graph-FocusPreviousTabStop` | `git_graph::FocusPreviousTabStop` | `shift-tab` (`GitGraph`)<br />`shift-tab` (`GitGraphSearchBar > Editor`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-ActivateChangesTab` | `git_panel::ActivateChangesTab` | `cmd-1` (`GitPanel`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-ActivateHistoryTab` | `git_panel::ActivateHistoryTab` | `cmd-2` (`GitPanel`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-CollapseSelectedEntry` | `git_panel::CollapseSelectedEntry` | `left` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-ExpandSelectedEntry` | `git_panel::ExpandSelectedEntry` | `right` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-FirstEntry` | `git_panel::FirstEntry` | `cmd-up` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-FocusChanges` | `git_panel::FocusChanges` | `tab` (`CommitEditor > Editor`)<br />`shift-tab` (`CommitEditor > Editor`)<br />`alt-up` (`CommitEditor > Editor`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-FocusEditor` | `git_panel::FocusEditor` | `alt-down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />`tab` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`)<br />`shift-tab` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-LastEntry` | `git_panel::LastEntry` | `cmd-down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-NextEntry` | `git_panel::NextEntry` | `down` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-PreviousEntry` | `git_panel::PreviousEntry` | `up` (`GitPanel && ChangesList && !GitBranchSelector && !GitRepositorySelector`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_panel-ToggleFocus` | `git_panel::ToggleFocus` | `ctrl-shift-g` (`Workspace`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_picker-ActivateBranchesTab` | `git_picker::ActivateBranchesTab` | `cmd-1` (`GitPicker`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-git_picker-ActivateStashTab` | `git_picker::ActivateStashTab` | `cmd-2` (`GitPicker`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-stash_picker-DropStashItem` | `stash_picker::DropStashItem` | `ctrl-shift-backspace` (`StashList \|\| (StashList > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-stash_picker-ShowStashItem` | `stash_picker::ShowStashItem` | `ctrl-shift-v` (`StashList \|\| (StashList > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-worktree_picker-DeleteWorktree` | `worktree_picker::DeleteWorktree` | `cmd-shift-backspace` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-worktree_picker-ForceDeleteWorktree` | `worktree_picker::ForceDeleteWorktree` | `cmd-alt-shift-backspace` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) | Git and change review | Pending | Pending | Pending | Pending |
| `ACT-MACOS-console-WatchExpression` | `console::WatchExpression` | `alt-enter` (`DebugConsole > Editor`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debug_panel-ToggleFocus` | `debug_panel::ToggleFocus` | `cmd-shift-d` (`Workspace`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-Continue` | `debugger::Continue` | `f5` (`Workspace && debugger_stopped`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-NextBreakpointProperty` | `debugger::NextBreakpointProperty` | `right` (`BreakpointList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-Pause` | `debugger::Pause` | `f6` (`Workspace && debugger_session`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-PreviousBreakpointProperty` | `debugger::PreviousBreakpointProperty` | `left` (`BreakpointList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-Rerun` | `debugger::Rerun` | `f5` (`Workspace`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-RerunSession` | `debugger::RerunSession` | `shift-cmd-f5` (`Workspace && debugger_session`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-Start` | `debugger::Start` | `f4` (`<global>`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-StepInto` | `debugger::StepInto` | `f11` (`Workspace && debugger_stopped`)<br />`ctrl-f11` (`Workspace && debugger_stopped`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-StepOut` | `debugger::StepOut` | `shift-f11` (`Workspace && debugger_stopped`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-StepOver` | `debugger::StepOver` | `f7` (`Workspace && debugger_stopped`)<br />`f10` (`Workspace && debugger_stopped`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-Stop` | `debugger::Stop` | `shift-f5` (`Workspace && debugger_session`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-ToggleEnableBreakpoint` | `debugger::ToggleEnableBreakpoint` | `space` (`BreakpointList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-ToggleExpandItem` | `debugger::ToggleExpandItem` | `shift-alt-escape` (`DebugPanel`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-ToggleSessionPicker` | `debugger::ToggleSessionPicker` | `cmd-i` (`DebugPanel`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-ToggleThreadPicker` | `debugger::ToggleThreadPicker` | `cmd-t` (`DebugPanel`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-debugger-UnsetBreakpoint` | `debugger::UnsetBreakpoint` | `backspace` (`BreakpointList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-dev-EditPredictionContextGoBack` | `dev::EditPredictionContextGoBack` | `alt-left` (`EditPredictionContext > Editor`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-dev-EditPredictionContextGoForward` | `dev::EditPredictionContextGoForward` | `alt-right` (`EditPredictionContext > Editor`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-dev-ResetFrameOverlayStats` | `dev::ResetFrameOverlayStats` | `ctrl-alt-shift-o` (`<global>`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-dev-ToggleFpsOverlay` | `dev::ToggleFpsOverlay` | `ctrl-alt-shift-p` (`<global>`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-dev-ToggleInspector` | `dev::ToggleInspector` | `cmd-alt-i` (`<global>`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-new_process_modal-ActivateAttachTab` | `new_process_modal::ActivateAttachTab` | `cmd-3` (`RunModal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-new_process_modal-ActivateDebugTab` | `new_process_modal::ActivateDebugTab` | `cmd-2` (`RunModal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-new_process_modal-ActivateLaunchTab` | `new_process_modal::ActivateLaunchTab` | `cmd-4` (`RunModal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-new_process_modal-ActivateTaskTab` | `new_process_modal::ActivateTaskTab` | `cmd-1` (`RunModal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-repl-Run` | `repl::Run` | `ctrl-shift-enter` (`Editor && jupyter`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-repl-RunInPlace` | `repl::RunInPlace` | `ctrl-alt-enter` (`Editor && jupyter`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-task-Rerun` | `task::Rerun` | `cmd-alt-r args={"reevaluate_context":false}` (`Workspace && !Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-task-Spawn` | `task::Spawn` | `cmd-shift-r` (`Workspace && !Terminal`)<br />`ctrl-alt-shift-r args={"reveal_target":"center"}` (`Workspace && !Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-Clear` | `terminal::Clear` | `cmd-k` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-Copy` | `terminal::Copy` | `cmd-c` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-Paste` | `terminal::Paste` | `cmd-v` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-PasteText` | `terminal::PasteText` | `ctrl-cmd-v` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-RerunTask` | `terminal::RerunTask` | `cmd-alt-r` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollLineDown` | `terminal::ScrollLineDown` | `shift-down` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollLineUp` | `terminal::ScrollLineUp` | `shift-up` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollPageDown` | `terminal::ScrollPageDown` | `shift-pagedown` (`Terminal`)<br />`cmd-down` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollPageUp` | `terminal::ScrollPageUp` | `shift-pageup` (`Terminal`)<br />`cmd-up` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollToBottom` | `terminal::ScrollToBottom` | `shift-end` (`Terminal`)<br />`cmd-end` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ScrollToTop` | `terminal::ScrollToTop` | `shift-home` (`Terminal`)<br />`cmd-home` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-SendKeystroke` | `terminal::SendKeystroke` | `cmd-backspace args="ctrl-u"` (`Terminal`)<br />`cmd-delete args="ctrl-k"` (`Terminal`)<br />`cmd-right args="ctrl-e"` (`Terminal`)<br />`cmd-left args="ctrl-a"` (`Terminal`)<br />`up args="up"` (`Terminal`)<br />`pageup args="pageup"` (`Terminal`)<br />`down args="down"` (`Terminal`)<br />`pagedown args="pagedown"` (`Terminal`)<br />`escape args="escape"` (`Terminal`)<br />`enter args="enter"` (`Terminal`)<br />`ctrl-c args="ctrl-c"` (`Terminal`)<br />`ctrl-r args="ctrl-r"` (`Terminal`)<br />`ctrl-backspace args="ctrl-w"` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-SendText` | `terminal::SendText` | `alt-delete args="\u001bd"` (`Terminal`)<br />`alt-left args="\u001bb"` (`Terminal`)<br />`alt-right args="\u001bf"` (`Terminal`)<br />`alt-b args="\u001bb"` (`Terminal`)<br />`alt-f args="\u001bf"` (`Terminal`)<br />`ctrl-delete args="\u001b[3;5~"` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ShowCharacterPalette` | `terminal::ShowCharacterPalette` | `ctrl-cmd-space` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal-ToggleViMode` | `terminal::ToggleViMode` | `ctrl-shift-space` (`Terminal`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-terminal_panel-Toggle` | `terminal_panel::Toggle` | `ctrl-`` (`Workspace`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-AddWatch` | `variable_list::AddWatch` | `alt-enter` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-CollapseSelectedEntry` | `variable_list::CollapseSelectedEntry` | `left` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-CopyVariableName` | `variable_list::CopyVariableName` | `cmd-alt-c` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-CopyVariableValue` | `variable_list::CopyVariableValue` | `cmd-c` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-EditVariable` | `variable_list::EditVariable` | `enter` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-ExpandSelectedEntry` | `variable_list::ExpandSelectedEntry` | `right` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-variable_list-RemoveWatch` | `variable_list::RemoveWatch` | `delete` (`VariableList`)<br />`backspace` (`VariableList`) | Terminal, tasks, debugging, and REPL | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-AddSelectionToThread` | `agent::AddSelectionToThread` | `cmd->` (`Editor && mode == full`)<br />`cmd->` (`AcpThread`)<br />`cmd->` (`Terminal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-AllowAlways` | `agent::AllowAlways` | `cmd-alt-y` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-AllowOnce` | `agent::AllowOnce` | `cmd-y` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ArchiveSelectedThread` | `agent::ArchiveSelectedThread` | `backspace` (`ThreadsArchiveView`)<br />`shift-backspace` (`ThreadsSidebar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-Chat` | `agent::Chat` | `enter` (`AcpThread > Editor && !use_modifier_to_send`)<br />`cmd-enter` (`AcpThread > Editor && use_modifier_to_send`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ChatWithFollow` | `agent::ChatWithFollow` | `cmd-enter` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ClearMessageQueue` | `agent::ClearMessageQueue` | `cmd-alt-backspace` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-CycleFavoriteModels` | `agent::CycleFavoriteModels` | `alt-tab` (`AcpThread`)<br />`alt-tab` (`AcpThread > Editor`)<br />`alt-tab` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-CycleModeSelector` | `agent::CycleModeSelector` | `shift-tab` (`AcpThread`)<br />`shift-tab` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-CycleNextInlineAssist` | `agent::CycleNextInlineAssist` | `ctrl-]` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-CyclePreviousInlineAssist` | `agent::CyclePreviousInlineAssist` | `ctrl-[` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-CycleThinkingEffort` | `agent::CycleThinkingEffort` | `ctrl-'` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-DismissThreadSearch` | `agent::DismissThreadSearch` | `escape` (`AcpThreadSearchBar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-EditFirstQueuedMessage` | `agent::EditFirstQueuedMessage` | `cmd-ctrl-e` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ExpandMessageEditor` | `agent::ExpandMessageEditor` | `shift-alt-escape` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-Keep` | `agent::Keep` | `cmd-y` (`AgentDiff`)<br />`cmd-alt-y` (`AgentDiff`)<br />`cmd-y` (`Editor && editor_agent_diff`)<br />`cmd-alt-y` (`Editor && editor_agent_diff`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-KeepAll` | `agent::KeepAll` | `shift-alt-y` (`AgentDiff`)<br />`shift-alt-y` (`Editor && editor_agent_diff`)<br />`shift-alt-y` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ManageProfiles` | `agent::ManageProfiles` | `cmd-alt-p` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ManageSkills` | `agent::ManageSkills` | `cmd-alt-l` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-NewThread` | `agent::NewThread` | `cmd-n` (`AgentPanel`)<br />`cmd-n` (`AcpThread`)<br />`cmd-n` (`AgentPanel > Terminal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-OpenAddContextMenu` | `agent::OpenAddContextMenu` | `ctrl-;` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-OpenAgentDiff` | `agent::OpenAgentDiff` | `shift-ctrl-r` (`Editor && editor_agent_diff`)<br />`shift-ctrl-r` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-OpenPermissionDropdown` | `agent::OpenPermissionDropdown` | `cmd-alt-a` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-OpenSettings` | `agent::OpenSettings` | `cmd-alt-c` (`AgentPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-PasteRaw` | `agent::PasteRaw` | `cmd-shift-v` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-Reject` | `agent::Reject` | `cmd-alt-z` (`AgentDiff`)<br />`cmd-alt-z` (`Editor && editor_agent_diff`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-RejectAll` | `agent::RejectAll` | `shift-alt-z` (`AgentDiff`)<br />`shift-alt-z` (`Editor && editor_agent_diff`)<br />`shift-alt-z` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-RejectOnce` | `agent::RejectOnce` | `cmd-alt-z` (`AcpThread`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-RemoveFirstQueuedMessage` | `agent::RemoveFirstQueuedMessage` | `cmd-shift-backspace` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-RemoveSelectedThread` | `agent::RemoveSelectedThread` | `shift-backspace` (`ThreadHistory > Editor`)<br />`cmd-shift-backspace` (`ThreadsSidebar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-RenameSelectedThread` | `agent::RenameSelectedThread` | `shift-r` (`ThreadsSidebar && not_searching`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputLineDown` | `agent::ScrollOutputLineDown` | `down` (`AcpThread`)<br />`ctrl-alt-down` (`AcpThread`)<br />`ctrl-alt-down` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputLineUp` | `agent::ScrollOutputLineUp` | `up` (`AcpThread`)<br />`ctrl-alt-up` (`AcpThread`)<br />`ctrl-alt-up` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputPageDown` | `agent::ScrollOutputPageDown` | `pagedown` (`AcpThread`)<br />`ctrl-pagedown` (`AcpThread`)<br />`ctrl-pagedown` (`AcpThread > Editor`)<br />`pagedown` (`AcpThread > Editor && end_of_input`)<br />`ctrl-pagedown` (`AcpThread > Editor && end_of_input`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputPageUp` | `agent::ScrollOutputPageUp` | `pageup` (`AcpThread`)<br />`ctrl-pageup` (`AcpThread`)<br />`ctrl-pageup` (`AcpThread > Editor`)<br />`pageup` (`AcpThread > Editor && start_of_input`)<br />`ctrl-pageup` (`AcpThread > Editor && start_of_input`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputToBottom` | `agent::ScrollOutputToBottom` | `end` (`AcpThread`)<br />`ctrl-end` (`AcpThread`)<br />`ctrl-end` (`AcpThread > Editor`)<br />`ctrl-end` (`AcpThread > Editor && end_of_input`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputToNextMessage` | `agent::ScrollOutputToNextMessage` | `shift-pagedown` (`AcpThread`)<br />`ctrl-alt-pagedown` (`AcpThread`)<br />`ctrl-alt-pagedown` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputToPreviousMessage` | `agent::ScrollOutputToPreviousMessage` | `shift-pageup` (`AcpThread`)<br />`ctrl-alt-pageup` (`AcpThread`)<br />`ctrl-alt-pageup` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ScrollOutputToTop` | `agent::ScrollOutputToTop` | `home` (`AcpThread`)<br />`ctrl-home` (`AcpThread`)<br />`ctrl-home` (`AcpThread > Editor`)<br />`ctrl-home` (`AcpThread > Editor && start_of_input`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-SelectNextThreadMatch` | `agent::SelectNextThreadMatch` | `cmd-g` (`AcpThread`)<br />`enter` (`AcpThreadSearchBar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-SelectPreviousThreadMatch` | `agent::SelectPreviousThreadMatch` | `cmd-shift-g` (`AcpThread`)<br />`shift-enter` (`AcpThreadSearchBar`)<br />`shift-enter` (`AcpThreadSearchBar > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-SendImmediately` | `agent::SendImmediately` | `cmd-shift-enter` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-SendNextQueuedMessage` | `agent::SendNextQueuedMessage` | `cmd-shift-alt-enter` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleFastMode` | `agent::ToggleFastMode` | `cmd-alt-.` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleFocus` | `agent::ToggleFocus` | `cmd-?` (`Workspace`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleModelSelector` | `agent::ToggleModelSelector` | `cmd-alt-/` (`AcpThread`)<br />`cmd-alt-/` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleNewThreadMenu` | `agent::ToggleNewThreadMenu` | `cmd-alt-shift-n` (`AgentPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleOptionsMenu` | `agent::ToggleOptionsMenu` | `cmd-alt-m` (`AgentPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleProfileSelector` | `agent::ToggleProfileSelector` | `cmd-i` (`AcpThread`)<br />`cmd-i` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleSearch` | `agent::ToggleSearch` | `cmd-f` (`AcpThread`)<br />`cmd-f` (`AcpThread > Editor`)<br />`cmd-f` (`AgentPanel > Terminal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleSteerFirstQueuedMessage` | `agent::ToggleSteerFirstQueuedMessage` | `cmd-ctrl-s` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleThinkingEffortMenu` | `agent::ToggleThinkingEffortMenu` | `cmd-alt-'` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-ToggleThinkingMode` | `agent::ToggleThinkingMode` | `cmd-alt-k` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agent-UndoLastReject` | `agent::UndoLastReject` | `shift-alt-u` (`AcpThread > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agents_sidebar-FocusSidebarFilter` | `agents_sidebar::FocusSidebarFilter` | `cmd-f` (`ThreadsSidebar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agents_sidebar-NewThreadInGroup` | `agents_sidebar::NewThreadInGroup` | `cmd-n` (`ThreadsSidebar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agents_sidebar-ToggleThreadHistory` | `agents_sidebar::ToggleThreadHistory` | `cmd-g` (`ThreadsSidebar`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-agents_sidebar-ToggleThreadSwitcher` | `agents_sidebar::ToggleThreadSwitcher` | `ctrl-tab` (`AgentPanel`)<br />`ctrl-shift-tab args={"select_last":true}` (`AgentPanel`)<br />`ctrl-tab` (`ThreadsSidebar`)<br />`ctrl-shift-tab args={"select_last":true}` (`ThreadsSidebar`)<br />`ctrl-tab` (`ThreadSwitcher`)<br />`ctrl-shift-tab args={"select_last":true}` (`ThreadSwitcher`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-assistant-InlineAssist` | `assistant::InlineAssist` | `ctrl-enter` (`!AcpThread > Editor && mode == full`)<br />`ctrl-enter` (`Terminal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-channel_modal-ToggleMode` | `channel_modal::ToggleMode` | `tab` (`ChannelModal`)<br />`tab` (`ChannelModal > Picker > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-InsertSpace` | `collab_panel::InsertSpace` | `space` (`(CollabPanel && editing) > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-MoveChannelDown` | `collab_panel::MoveChannelDown` | `alt-down` (`CollabPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-MoveChannelUp` | `collab_panel::MoveChannelUp` | `alt-up` (`CollabPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-OpenSelectedChannelNotes` | `collab_panel::OpenSelectedChannelNotes` | `alt-enter` (`CollabPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-Remove` | `collab_panel::Remove` | `ctrl-backspace` (`CollabPanel && not_editing`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-ToggleFocus` | `collab_panel::ToggleFocus` | `cmd-shift-c` (`<global>`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-collab_panel-ToggleSelectedChannelFavorite` | `collab_panel::ToggleSelectedChannelFavorite` | `shift-enter` (`CollabPanel`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-inline_assistant-ThumbsDownResult` | `inline_assistant::ThumbsDownResult` | `cmd-shift-backspace` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-inline_assistant-ThumbsUpResult` | `inline_assistant::ThumbsUpResult` | `cmd-shift-enter` (`InlineAssistant > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-skill_creator-FocusNextField` | `skill_creator::FocusNextField` | `tab` (`SkillCreator`)<br />`tab` (`SkillCreator > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-skill_creator-FocusPreviousField` | `skill_creator::FocusPreviousField` | `shift-tab` (`SkillCreator`)<br />`shift-tab` (`SkillCreator > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-skill_creator-SaveSkill` | `skill_creator::SaveSkill` | `cmd-enter` (`SkillCreator`)<br />`cmd-enter` (`SkillCreator > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-FocusPredictions` | `zeta::FocusPredictions` | `escape` (`RatePredictionsModal > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-NextEdit` | `zeta::NextEdit` | `shift-down` (`RatePredictionsModal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-PreviewPrediction` | `zeta::PreviewPrediction` | `right` (`RatePredictionsModal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-PreviousEdit` | `zeta::PreviousEdit` | `shift-up` (`RatePredictionsModal`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-ThumbsDownActivePrediction` | `zeta::ThumbsDownActivePrediction` | `cmd-shift-backspace` (`RatePredictionsModal`)<br />`cmd-shift-backspace` (`RatePredictionsModal > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zeta-ThumbsUpActivePrediction` | `zeta::ThumbsUpActivePrediction` | `cmd-shift-enter` (`RatePredictionsModal`)<br />`cmd-shift-enter` (`RatePredictionsModal > Editor`) | AI and collaboration | Pending | Pending | Pending | Pending |
| `ACT-MACOS-edit_prediction-RatePredictions` | `edit_prediction::RatePredictions` | `ctrl-cmd-z` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-edit_prediction-ToggleMenu` | `edit_prediction::ToggleMenu` | `ctrl-cmd-i` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-image_viewer-FitToView` | `image_viewer::FitToView` | `cmd-shift-0` (`ImageViewer`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-image_viewer-ResetZoom` | `image_viewer::ResetZoom` | `cmd-0` (`ImageViewer`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-image_viewer-ZoomIn` | `image_viewer::ZoomIn` | `cmd-=` (`ImageViewer`)<br />`cmd-+` (`ImageViewer`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-image_viewer-ZoomOut` | `image_viewer::ZoomOut` | `cmd--` (`ImageViewer`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-image_viewer-ZoomToActualSize` | `image_viewer::ZoomToActualSize` | `cmd-1` (`ImageViewer`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-CopyAction` | `keymap_editor::CopyAction` | `cmd-c` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-CopyContext` | `keymap_editor::CopyContext` | `cmd-shift-c` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-CreateBinding` | `keymap_editor::CreateBinding` | `alt-enter` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-EditBinding` | `keymap_editor::EditBinding` | `enter` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-OpenCreateKeybindingModal` | `keymap_editor::OpenCreateKeybindingModal` | `cmd-k` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-ShowMatchingKeybinds` | `keymap_editor::ShowMatchingKeybinds` | `cmd-t` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-ToggleConflictFilter` | `keymap_editor::ToggleConflictFilter` | `cmd-alt-c` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keymap_editor-ToggleKeystrokeSearch` | `keymap_editor::ToggleKeystrokeSearch` | `cmd-alt-f` (`KeymapEditor`)<br />`cmd-alt-f` (`KeymapEditor > BufferSearchBar`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keystroke_input-ClearKeystrokes` | `keystroke_input::ClearKeystrokes` | `delete` (`KeystrokeInput`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keystroke_input-StartRecording` | `keystroke_input::StartRecording` | `enter` (`KeystrokeInput`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-keystroke_input-StopRecording` | `keystroke_input::StopRecording` | `escape escape escape` (`KeystrokeInput`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-AddCodeBlock` | `notebook::AddCodeBlock` | `cmd-m` (`NotebookEditor`)<br />`cmd-m` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-AddMarkdownBlock` | `notebook::AddMarkdownBlock` | `cmd-shift-m` (`NotebookEditor`)<br />`cmd-shift-m` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-DeleteCell` | `notebook::DeleteCell` | `d d` (`NotebookEditor && notebook_mode == command`)<br />`backspace` (`NotebookEditor && notebook_mode == command`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-EnterCommandMode` | `notebook::EnterCommandMode` | `escape` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-EnterEditMode` | `notebook::EnterEditMode` | `enter` (`NotebookEditor && notebook_mode == command`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-InterruptKernel` | `notebook::InterruptKernel` | `cmd-c` (`NotebookEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-MoveCellDown` | `notebook::MoveCellDown` | `alt-down` (`NotebookEditor`)<br />`alt-down` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-MoveCellUp` | `notebook::MoveCellUp` | `alt-up` (`NotebookEditor`)<br />`alt-up` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-RestartKernel` | `notebook::RestartKernel` | `cmd-shift-r` (`NotebookEditor`)<br />`cmd-shift-r` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-Run` | `notebook::Run` | `cmd-enter` (`NotebookEditor`)<br />`cmd-enter` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-RunAll` | `notebook::RunAll` | `cmd-shift-enter` (`NotebookEditor`)<br />`cmd-shift-enter` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-notebook-RunAndAdvance` | `notebook::RunAndAdvance` | `shift-enter` (`NotebookEditor`)<br />`shift-enter` (`NotebookEditor > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-picker-ConfirmCompletion` | `picker::ConfirmCompletion` | `tab` (`Picker > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-picker-ConfirmInput` | `picker::ConfirmInput` | `alt-enter args={"secondary":false}` (`Picker > Editor`)<br />`cmd-alt-enter args={"secondary":true}` (`Picker > Editor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-CollapseNavEntry` | `settings_editor::CollapseNavEntry` | `left` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-ExpandNavEntry` | `settings_editor::ExpandNavEntry` | `right` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusFile` | `settings_editor::FocusFile` | `ctrl-1 args=0` (`SettingsWindow`)<br />`ctrl-2 args=1` (`SettingsWindow`)<br />`ctrl-3 args=2` (`SettingsWindow`)<br />`ctrl-4 args=3` (`SettingsWindow`)<br />`ctrl-5 args=4` (`SettingsWindow`)<br />`ctrl-6 args=5` (`SettingsWindow`)<br />`ctrl-7 args=6` (`SettingsWindow`)<br />`ctrl-8 args=7` (`SettingsWindow`)<br />`ctrl-9 args=8` (`SettingsWindow`)<br />`ctrl-0 args=9` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusFirstNavEntry` | `settings_editor::FocusFirstNavEntry` | `home` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusLastNavEntry` | `settings_editor::FocusLastNavEntry` | `end` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusNextFile` | `settings_editor::FocusNextFile` | `cmd-}` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusNextNavEntry` | `settings_editor::FocusNextNavEntry` | `down` (`SettingsWindow > NavigationMenu`)<br />`tab` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusNextRootNavEntry` | `settings_editor::FocusNextRootNavEntry` | `pagedown` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusPreviousFile` | `settings_editor::FocusPreviousFile` | `cmd-{` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusPreviousNavEntry` | `settings_editor::FocusPreviousNavEntry` | `up` (`SettingsWindow > NavigationMenu`)<br />`shift-tab` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-FocusPreviousRootNavEntry` | `settings_editor::FocusPreviousRootNavEntry` | `pageup` (`SettingsWindow > NavigationMenu`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-Minimize` | `settings_editor::Minimize` | `cmd-m` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-OpenCurrentFile` | `settings_editor::OpenCurrentFile` | `cmd-,` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_editor-ToggleFocusNav` | `settings_editor::ToggleFocusNav` | `left` (`SettingsWindow`)<br />`cmd-shift-e` (`SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-settings_profile_selector-Toggle` | `settings_profile_selector::Toggle` | `ctrl-alt-cmd-p` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-tab_switcher-CloseSelectedItem` | `tab_switcher::CloseSelectedItem` | `ctrl-backspace` (`TabSwitcher`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-tab_switcher-Toggle` | `tab_switcher::Toggle` | `ctrl-shift-tab args={"select_last":true}` (`Workspace`)<br />`ctrl-tab` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-tabular_data-OpenPreview` | `tabular_data::OpenPreview` | `cmd-shift-v` (`Editor && (extension == csv \|\| extension == tsv \|\| extension == ssv \|\| extension == psv)`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-tabular_data-OpenPreviewToTheSide` | `tabular_data::OpenPreviewToTheSide` | `cmd-k v` (`Editor && (extension == csv \|\| extension == tsv \|\| extension == ssv \|\| extension == psv)`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-theme-ToggleMode` | `theme::ToggleMode` | `cmd-k cmd-shift-t` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-theme_selector-Toggle` | `theme_selector::Toggle` | `cmd-k cmd-t` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-toolchain-AddToolchain` | `toolchain::AddToolchain` | `cmd-k cmd-m` (`Workspace`)<br />`cmd-shift-a` (`ToolchainSelector`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-DecreaseBufferFontSize` | `zed::DecreaseBufferFontSize` | `cmd-- args={"persist":false}` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-DecreaseUiFontSize` | `zed::DecreaseUiFontSize` | `cmd-- args={"persist":false}` (`Onboarding`)<br />`cmd-- args={"persist":false}` (`Welcome`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-Extensions` | `zed::Extensions` | `cmd-shift-x` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-Hide` | `zed::Hide` | `cmd-h` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-HideOthers` | `zed::HideOthers` | `alt-cmd-h` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-IncreaseBufferFontSize` | `zed::IncreaseBufferFontSize` | `cmd-= args={"persist":false}` (`<global>`)<br />`cmd-+ args={"persist":false}` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-IncreaseUiFontSize` | `zed::IncreaseUiFontSize` | `cmd-= args={"persist":false}` (`Onboarding`)<br />`cmd-+ args={"persist":false}` (`Onboarding`)<br />`cmd-= args={"persist":false}` (`Welcome`)<br />`cmd-+ args={"persist":false}` (`Welcome`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-Minimize` | `zed::Minimize` | `cmd-m` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-OpenKeymap` | `zed::OpenKeymap` | `cmd-k cmd-s` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-OpenKeymapFile` | `zed::OpenKeymapFile` | `cmd-e` (`KeymapEditor`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-OpenSettings` | `zed::OpenSettings` | `cmd-,` (`!SettingsWindow`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-OpenSettingsFile` | `zed::OpenSettingsFile` | `cmd-alt-,` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-OpenWorktreeSetupTasks` | `zed::OpenWorktreeSetupTasks` | `cmd-shift-c` (`WorktreePicker \|\| (WorktreePicker > Picker > Editor)`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-Quit` | `zed::Quit` | `cmd-q` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-ResetBufferFontSize` | `zed::ResetBufferFontSize` | `cmd-0 args={"persist":false}` (`<global>`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-ResetUiFontSize` | `zed::ResetUiFontSize` | `cmd-0 args={"persist":false}` (`Onboarding`)<br />`cmd-0 args={"persist":false}` (`Welcome`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-zed-ToggleFullScreen` | `zed::ToggleFullScreen` | `fn-f` (`Workspace`)<br />`ctrl-cmd-f` (`Workspace`) | Document views, editor configuration, and keymap | Pending | Pending | Pending | Pending |
| `ACT-MACOS-command_palette-Toggle` | `command_palette::Toggle` | `cmd-shift-p` (`Workspace`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-Cancel` | `menu::Cancel` | `cmd-escape` (`<global>`)<br />`ctrl-escape` (`<global>`)<br />`ctrl-c` (`<global>`)<br />`escape` (`<global>`)<br />`escape` (`AgentFeedbackMessageEditor > Editor`)<br />`escape` (`OutlinePanel && not_editing`)<br />`escape` (`ProjectPanel`)<br />`escape` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`escape` (`GitCommit > Editor && mode == auto_height`)<br />`escape` (`Picker > Editor`)<br />`escape` (`ZedPredictModal`)<br />`escape` (`ConfigureContextServerModal > Editor`)<br />`escape` (`OnboardingAiConfigurationModal`)<br />`escape` (`KeybindEditorModal`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-Confirm` | `menu::Confirm` | `enter` (`<global>`)<br />`enter` (`AgentFeedbackMessageEditor > Editor`)<br />`cmd-enter` (`AcpThread > ModeSelector`)<br />`enter` (`ThreadsSidebar`)<br />`space` (`ThreadsSidebar && not_searching`)<br />`enter` (`Editor && inline_input`)<br />`enter` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`space` (`CollabPanel && not_editing`)<br />`cmd-enter` (`ConfigureContextServerModal > Editor`)<br />`enter` (`DebugConsole > Editor`)<br />`cmd-enter` (`KeybindEditorModal`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-Restart` | `menu::Restart` | `alt-shift-enter` (`<global>`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SecondaryConfirm` | `menu::SecondaryConfirm` | `ctrl-enter` (`<global>`)<br />`cmd-enter` (`<global>`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectChild` | `menu::SelectChild` | `right` (`menu`)<br />`right` (`ThreadsSidebar`)<br />`cmd-k right` (`CallHierarchyPicker > Picker > Editor`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectFirst` | `menu::SelectFirst` | `home` (`<global>`)<br />`shift-pageup` (`<global>`)<br />`pageup` (`<global>`)<br />`cmd-up` (`<global>`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectLast` | `menu::SelectLast` | `end` (`<global>`)<br />`shift-pagedown` (`<global>`)<br />`pagedown` (`<global>`)<br />`cmd-down` (`<global>`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectNext` | `menu::SelectNext` | `tab` (`<global>`)<br />`ctrl-n` (`<global>`)<br />`down` (`<global>`)<br />`right` (`Prompt`)<br />`l` (`Prompt`)<br />`shift-down` (`OutlinePanel && not_editing`)<br />`shift-down` (`ProjectPanel`)<br />`down` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`shift-down` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`down` (`Picker > Editor`)<br />`ctrl-down` (`TabSwitcher`)<br />`down` (`KeybindEditorModal > Editor`)<br />`down` (`NotebookEditor && notebook_mode == command`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectParent` | `menu::SelectParent` | `left` (`menu`)<br />`left` (`ThreadsSidebar`)<br />`cmd-k left` (`CallHierarchyPicker > Picker > Editor`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-menu-SelectPrevious` | `menu::SelectPrevious` | `shift-tab` (`<global>`)<br />`ctrl-p` (`<global>`)<br />`up` (`<global>`)<br />`left` (`Prompt`)<br />`h` (`Prompt`)<br />`shift-up` (`OutlinePanel && not_editing`)<br />`shift-up` (`ProjectPanel`)<br />`up` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`shift-up` (`GitPanel && (ChangesList \|\| HistoryList) && !GitBranchSelector && !GitRepositorySelector`)<br />`up` (`Picker > Editor`)<br />`ctrl-shift-tab` (`TabSwitcher`)<br />`ctrl-up` (`TabSwitcher`)<br />`up` (`KeybindEditorModal > Editor`)<br />`up` (`NotebookEditor && notebook_mode == command`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-onboarding-Finish` | `onboarding::Finish` | `cmd-enter` (`Onboarding`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-onboarding-OpenAccount` | `onboarding::OpenAccount` | `alt-shift-a` (`Onboarding`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-onboarding-SignIn` | `onboarding::SignIn` | `alt-tab` (`Onboarding`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-toast-RunAction` | `toast::RunAction` | `alt-shift-enter` (`Workspace`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
| `ACT-MACOS-welcome-OpenRecentProject` | `welcome::OpenRecentProject` | `cmd-1 args=0` (`Welcome`)<br />`cmd-2 args=1` (`Welcome`)<br />`cmd-3 args=2` (`Welcome`)<br />`cmd-4 args=3` (`Welcome`)<br />`cmd-5 args=4` (`Welcome`) | App menu, startup, and transient surfaces | Pending | Pending | Pending | Pending |
