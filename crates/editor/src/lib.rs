//! Labonair code editor core (T06-*).
//!
//! Framework-free editing model: a rope-backed [`buffer::EditorBuffer`], a
//! [`document::Document`] wrapping it with caret / selection / undo history /
//! dirty-baseline tracking, literal [`search`] find-replace, and lightweight
//! [`language`] identification. The GPUI view that renders and drives this
//! lives in `labonair-ui` (`editor.rs`). Syntax highlighting (T06-002), Vim
//! mode (T06-003), Helix mode, and diff view (T06-004) build on top.

pub mod breadcrumbs;
pub mod buffer;
pub mod command_provider;
pub mod diff;
pub mod display;
pub mod document;
pub mod editing;
pub mod file_finder;
pub mod git;
pub mod helix;
pub mod history;
pub mod kill_ring;
pub mod language;
pub mod language_services;
pub mod lifecycle;
pub mod lsp_process;
pub mod navigation;
pub mod policy;
pub mod runtime;
pub mod search;
pub mod session;
pub mod splits;
pub mod symbols;
pub mod syntax;
pub mod unified;
pub mod vim;

pub use breadcrumbs::{
    build_breadcrumb_state, truncate_middle, BreadcrumbPath, BreadcrumbState, BreadcrumbSymbol,
};
pub use buffer::{
    Anchor, AnchorAffinity, AnchorRange, AppliedTransaction, BufferEvent, BufferSnapshot, Edit,
    EditDelta, EditSource, EditorBuffer, EditorError, Position, Revision, Selection, SelectionSet,
    TextRange, Transaction,
};
pub use command_provider::{EditorLanguageCommand, GitEditorCommand};
pub use diff::{side_by_side, ChangeTag, Diff, DiffLine, Hunk, RowKind, SideCell, SideRow};
pub use display::{
    DisplayConfig, DisplayDiagnostic, DisplayDocumentColor, DisplayInlineHint, DisplayMap,
    DisplayRow, DisplayRowKind, DisplaySemanticToken, DisplaySnapshot, DisplayTextProjection,
    FoldRange, Viewport, WhitespaceMode, WhitespaceToken,
};
pub use document::{Document, EditorEditability, Motion};
pub use editing::{
    auto_indent, matching_bracket, EditIntent, EditorCommand, EditorCommandResult,
    EditorCommandSink, EditorSnapshotStream, EditorState, EditorViewEvent, EditorViewSnapshot,
    MultiSelection, SnippetSession, SplitIntent,
};
pub use file_finder::{
    FileFinderCandidate, FileFinderQuery, FileFinderResult, FileFinderSession, FileFinderSnapshot,
    FileFinderStatus, DEFAULT_FILE_FINDER_MAX_RESULTS, MAX_FILE_FINDER_RESULTS,
};
pub use git::{
    DisplayGitDecoration, GitBlameRequest, GitBlameSnapshot, GitDecorationProvider,
    GitDecorationRequest, GitDiscardPreview, GitGutterError, GitGutterSnapshot, GitGutterState,
    GitHunk, GitHunkAction, GitHunkActionKind, GitHunkStatus, GitHunkTarget, GitLineDecoration,
    GitProviderFuture, HunkActionSink,
};
pub use helix::{Helix, HelixKey, HelixMode};
pub use kill_ring::{KillRing, KILL_RING_CAPACITY};
pub use language::Language;
pub use language_services::{
    default_registry, BackgroundRequest, CallHierarchyCall, CallHierarchyItem, CancellationToken,
    CodeAction, CompletionItem, CompletionKind, CompletionList, CompletionTextFormat, Diagnostic,
    DiagnosticSeverity, DocumentColor, DocumentColorValue, DocumentVersion, FoldingRange,
    HoverContent, InlayHint, InlayHintKind, LanguageId, LanguageServerId,
    LanguageServiceCapabilities, LanguageServiceCapability, LanguageServiceEditCapabilities,
    LanguageServiceEditCapability, LanguageServiceEditRequest, LanguageServiceEditResponse,
    LanguageServiceError, LanguageServiceRegistry, LanguageServiceRequest, LanguageServiceResponse,
    LanguageServiceResult, LanguageServiceSnapshot, LanguageServiceState, LocalLanguageService,
    LspPosition, LspRange, NavigationTarget, SemanticToken, SignatureHelp, SyntaxLanguageService,
};
pub use lsp_process::{
    ContentLengthFramer, FrameError, LanguageServerTimeoutPolicy, LanguageServiceEvent,
    LocalLanguageServerConfig, LocalLanguageServiceEditRequest, LocalLanguageServiceEditResult,
    LocalLanguageServiceProcess, LocalLanguageServiceRequest, ProcessLifecycleState,
    ProcessServerConfig, ProjectRootPolicy,
};
pub use navigation::{NavigationHistory, NavigationLocation};
pub use policy::{apply_text_policies, SavePolicy};
pub use runtime::{
    BackgroundEditRequest, LanguageServiceEditResult, LanguageServiceRuntime,
    LanguageServiceRuntimePolicy, LanguageServiceRuntimeSnapshot, LanguageServiceRuntimeSource,
    LanguageServiceRuntimeStatus,
};
pub use search::{
    find, find_all, map_range_through_edits, next_match, next_match_with_wrap,
    plan_project_replace, replace_all, replace_all_checked, replace_one,
    replace_project_all_checked, word_at_cursor, Match, ProjectBufferCandidate, ProjectReplacePlan,
    ProjectReplaceRequest, ProjectReplaceResult, ProjectReplaceStatus, ProjectReplaceTarget,
    ProjectSearchHit, ProjectSearchOptions, ProjectSearchQuery, ProjectSearchRequest,
    ProjectSearchResult, ProjectSearchSession, ProjectSearchSnapshot, ProjectSearchStatus,
    SearchCaseMode, SearchError, SearchOptions, SearchQuery, SearchResult, SearchScope,
};
pub use session::{
    decode_session, encode_session, EditorPersistenceEvent, EditorSessionId, EditorSessionSnapshot,
    EditorSessionStore, EditorViewSession, PersistenceError, UnsavedBufferRecovery,
    EDITOR_SESSION_SCHEMA_VERSION,
};
pub use splits::{EditorGroupId, EditorSplitTree, SplitError, SplitNode, SplitOrientation};
pub use symbols::{document_symbols, document_syntax, DocumentSymbol, DocumentSyntax, SymbolKind};
pub use syntax::{HighlightKind, HighlightSpan, StyledRun, SyntaxHighlighter};
pub use unified::{
    build_hunk_patch, is_whole_file_single_hunk, parse_diff_hunks, DiffHunk, FileDiff,
};
pub use vim::{Vim, VimKey, VimMode, VimOptions, VimResponse};
