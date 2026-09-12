//! Labonair code editor core (T06-*).
//!
//! Framework-free editing model: a rope-backed [`buffer::EditorBuffer`], a
//! [`document::Document`] wrapping it with caret / selection / undo history /
//! dirty-baseline tracking, literal [`search`] find-replace, and lightweight
//! [`language`] identification. The GPUI view that renders and drives this
//! lives in `labonair-ui` (`editor.rs`). Syntax highlighting (T06-002), vim
//! mode (T06-003) and diff view (T06-004) build on top.

pub mod breadcrumbs;
pub mod buffer;
pub mod command_provider;
pub mod diff;
pub mod display;
pub mod document;
pub mod editing;
pub mod file_finder;
pub mod git;
pub mod history;
pub mod language;
pub mod language_services;
pub mod lifecycle;
pub mod lsp_process;
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
    DisplayConfig, DisplayDiagnostic, DisplayMap, DisplayRow, DisplayRowKind, DisplaySemanticToken,
    DisplaySnapshot, FoldRange, Viewport, WhitespaceMode, WhitespaceToken,
};
pub use document::{Document, Motion};
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
    DisplayGitDecoration, GitDecorationProvider, GitDecorationRequest, GitDiscardPreview,
    GitGutterError, GitGutterSnapshot, GitGutterState, GitHunk, GitHunkAction, GitHunkActionKind,
    GitHunkStatus, GitHunkTarget, GitLineDecoration, GitProviderFuture, HunkActionSink,
};
pub use language::Language;
pub use language_services::{
    default_registry, BackgroundRequest, CancellationToken, CodeAction, CompletionItem,
    CompletionKind, CompletionList, Diagnostic, DiagnosticSeverity, DocumentVersion, FoldingRange,
    HoverContent, LanguageId, LanguageServerId, LanguageServiceCapabilities,
    LanguageServiceCapability, LanguageServiceEditCapabilities, LanguageServiceEditCapability,
    LanguageServiceEditRequest, LanguageServiceEditResponse, LanguageServiceError,
    LanguageServiceRegistry, LanguageServiceRequest, LanguageServiceResponse,
    LanguageServiceResult, LanguageServiceSnapshot, LanguageServiceState, LocalLanguageService,
    LspPosition, LspRange, NavigationTarget, SemanticToken, SignatureHelp, SyntaxLanguageService,
};
pub use lsp_process::{
    ContentLengthFramer, FrameError, LanguageServerTimeoutPolicy, LanguageServiceEvent,
    LocalLanguageServerConfig, LocalLanguageServiceEditRequest, LocalLanguageServiceEditResult,
    LocalLanguageServiceProcess, LocalLanguageServiceRequest, ProcessLifecycleState,
    ProcessServerConfig, ProjectRootPolicy,
};
pub use policy::{apply_text_policies, SavePolicy};
pub use runtime::{
    BackgroundEditRequest, LanguageServiceEditResult, LanguageServiceRuntime,
    LanguageServiceRuntimePolicy, LanguageServiceRuntimeSnapshot, LanguageServiceRuntimeSource,
    LanguageServiceRuntimeStatus,
};
pub use search::{
    find, find_all, next_match, next_match_with_wrap, replace_all, replace_all_checked,
    replace_one, Match, ProjectSearchHit, ProjectSearchOptions, ProjectSearchQuery,
    ProjectSearchRequest, ProjectSearchResult, ProjectSearchSession, ProjectSearchSnapshot,
    ProjectSearchStatus, SearchCaseMode, SearchError, SearchOptions, SearchQuery, SearchResult,
    SearchScope,
};
pub use session::{
    decode_session, encode_session, EditorPersistenceEvent, EditorSessionId, EditorSessionSnapshot,
    EditorSessionStore, EditorViewSession, PersistenceError, UnsavedBufferRecovery,
    EDITOR_SESSION_SCHEMA_VERSION,
};
pub use splits::{EditorGroupId, EditorSplitTree, SplitError, SplitNode, SplitOrientation};
pub use symbols::{document_symbols, DocumentSymbol, SymbolKind};
pub use syntax::{HighlightKind, HighlightSpan, StyledRun, SyntaxHighlighter};
pub use unified::{
    build_hunk_patch, is_whole_file_single_hunk, parse_diff_hunks, DiffHunk, FileDiff,
};
pub use vim::{Vim, VimKey, VimMode, VimOptions, VimResponse};
