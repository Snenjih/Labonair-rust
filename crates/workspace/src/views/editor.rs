//! GPUI code-editor view (T06-001).
//!
//! Renders and drives a [`labonair_editor::Document`]: viewport-based line
//! rendering with a line-number gutter, caret + selection, keyboard editing,
//! undo/redo, `Cmd-S` save (atomic write via
//! [`labonair_filesystem::file::save_editor_file_lifecycle_sync`]), a find /
//! replace bar (`Cmd-F`), and external-change detection with a reload banner.
//!
//! The view owns no file IO on the main thread — reads and writes run on
//! `cx.background_executor().spawn`. It emits [`EditorEvent`] so the hosting
//! [`Workspace`](crate::Workspace) can mirror the dirty flag and peek state
//! onto the tab.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, relative, AnyElement, App, AppContext, Bounds, ClickEvent, ClipboardItem,
    Context, Entity, EventEmitter, FocusHandle, Focusable, HighlightStyle, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, Pixels,
    Point, Render, ScrollWheelEvent, SharedString, StatefulInteractiveElement, Styled, StyledText,
    Task, Window,
};
use labonair_filesystem::file::{
    load_editor_file_lifecycle_sync, save_editor_file_lifecycle_sync, stat_editor_file_sync,
    EditorFileError, EditorFileStat, EditorLifecycleLoad, StorageBom, StorageFileCapabilities,
    StorageFileIdentity, StorageFileSnapshot, StorageLineEnding, StorageSaveIntent,
};
use labonair_settings::{EditorSettings, Settings as _, SettingsStore};

use labonair_editor::document_symbols;
use labonair_editor::{
    apply_text_policies,
    command_provider::EditorLanguageCommand,
    document::Motion,
    editing::{EditorCommand, EditorCommandResult, SplitIntent},
    find,
    lifecycle::{FileCapabilities, FileLifecycleEvent, FileState, ReloadDecision, SaveIntent},
    next_match_with_wrap, CancellationToken, DisplayConfig, DisplayMap, DisplayRowKind, Document,
    DocumentVersion, Edit, EditorSessionId, EditorSessionSnapshot, EditorViewSession, FoldRange,
    GitDecorationProvider, GitDecorationRequest, GitDiscardPreview, GitEditorCommand,
    GitGutterError, GitGutterSnapshot, GitHunkAction, GitHunkActionKind, GitHunkStatus,
    HunkActionSink, Language, LanguageId, LanguageServiceEditRequest, LanguageServiceEditResponse,
    LanguageServiceRequest, LanguageServiceResponse, LanguageServiceResult, LanguageServiceRuntime,
    LanguageServiceRuntimeSnapshot, LanguageServiceRuntimeSource, LanguageServiceRuntimeStatus,
    LanguageServiceSnapshot, LanguageServiceState, LocalLanguageServiceRequest, LspPosition,
    LspRange, Match, Position, ProjectSearchHit, ProjectSearchQuery, ProjectSearchRequest,
    ProjectSearchResult, ProjectSearchSession, ProjectSearchSnapshot, Revision, SavePolicy,
    SearchError, SearchQuery, SyntaxHighlighter, TextRange, Viewport, Vim, VimKey, VimMode,
    VimOptions, WhitespaceMode,
};
use labonair_editor::{build_breadcrumb_state, BreadcrumbState};

use crate::syntax_theme::EditorPalette;
use crate::theme::ThemeStore;
use labonair_notifications::{notification_center, Notification};
use labonair_ui_kit::{
    banner, button, context_menu, field_input, text_field, ButtonSize, ButtonVariant, InputEvent,
    InputState, ListItem, MenuItem, Palette, Severity, Tooltip,
};

/// Editor → workspace notifications.
#[derive(Clone, Copy, Debug)]
pub enum EditorEvent {
    /// Dirty flag or title-relevant state changed — re-sync the tab.
    Changed,
    /// The user made their first edit — a peek tab should become permanent.
    Edited,
    /// Vim `:q` / `:wq` — the hosting workspace should close this editor's tab.
    CloseRequested,
}

/// Editor find state, driven entirely by the workspace `Cmd+F` search overlay
/// (T18-002) — the editor no longer paints its own find bar. The active match
/// is shown as the editor selection.
#[derive(Default)]
struct EditorSearch {
    query: SearchQuery,
    matches: Vec<Match>,
    active: Option<usize>,
    error: Option<SearchError>,
}

#[derive(Clone, Debug)]
struct EditorSplitDrag {
    path: labonair_editor::splits::SplitPath,
    orientation: labonair_editor::SplitOrientation,
}

const EDITOR_SCROLLBAR_SIZE: f32 = 10.0;
const EDITOR_SCROLLBAR_MIN_THUMB: f32 = 24.0;
const EDITOR_BREADCRUMB_MAX_PATH_CHARS: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScrollbarAxis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScrollbarGeometry {
    track_length: f32,
    thumb_length: f32,
    thumb_offset: f32,
    max_scroll: usize,
}

#[derive(Clone, Copy, Debug)]
struct EditorScrollbarDrag {
    axis: ScrollbarAxis,
    geometry: ScrollbarGeometry,
}

/// Calculate a compact scrollbar thumb without allowing a zero-sized or
/// out-of-track thumb. The editor uses this for both display rows and columns,
/// so the math remains deterministic and independent of GPUI layout.
fn scrollbar_geometry(
    content_extent: usize,
    viewport_extent: usize,
    scroll: usize,
    track_length: f32,
) -> Option<ScrollbarGeometry> {
    if content_extent <= viewport_extent || track_length <= 0.0 {
        return None;
    }
    let max_scroll = content_extent.saturating_sub(viewport_extent);
    let thumb_length = (track_length * viewport_extent as f32 / content_extent as f32)
        .max(EDITOR_SCROLLBAR_MIN_THUMB)
        .min(track_length);
    let travel = (track_length - thumb_length).max(0.0);
    let thumb_offset = if max_scroll == 0 {
        0.0
    } else {
        travel * scroll.min(max_scroll) as f32 / max_scroll as f32
    };
    Some(ScrollbarGeometry {
        track_length,
        thumb_length,
        thumb_offset,
        max_scroll,
    })
}

fn scrollbar_scroll_from_pointer(
    pointer: f32,
    track_start: f32,
    geometry: ScrollbarGeometry,
) -> usize {
    let travel = (geometry.track_length - geometry.thumb_length).max(0.0);
    if geometry.max_scroll == 0 || travel <= 0.0 {
        return 0;
    }
    let offset = (pointer - track_start - geometry.thumb_length / 2.0).clamp(0.0, travel);
    (offset / travel * geometry.max_scroll as f32).round() as usize
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoldMarker {
    range: FoldRange,
    folded: bool,
}

fn fold_ranges_from_symbols(
    symbols: &[labonair_editor::DocumentSymbol],
    snapshot: &labonair_editor::BufferSnapshot,
) -> Vec<FoldRange> {
    let mut ranges = symbols
        .iter()
        .filter_map(|symbol| {
            let end_line = snapshot
                .byte_to_position(symbol.range.end.min(snapshot.byte_len()))
                .line;
            FoldRange::new(symbol.line, end_line)
        })
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| (range.start_line, range.end_line));
    ranges.dedup();
    ranges
}

fn fold_marker_for_line(
    candidates: &[FoldRange],
    folds: &[FoldRange],
    line: usize,
) -> Option<FoldMarker> {
    candidates
        .iter()
        .copied()
        .filter(|range| range.start_line == line)
        .max_by_key(|range| range.end_line)
        .map(|range| FoldMarker {
            folded: folds.contains(&range),
            range,
        })
}

fn toggle_fold(folds: &mut Vec<FoldRange>, target: FoldRange) -> bool {
    if let Some(index) = folds.iter().position(|range| *range == target) {
        folds.remove(index);
        false
    } else {
        folds.push(target);
        folds.sort_by_key(|range| (range.start_line, range.end_line));
        true
    }
}

impl EditorSearch {
    fn active_match_is(&self, matched: Match) -> bool {
        self.active
            .and_then(|active| self.matches.get(active).copied())
            == Some(matched)
    }
}

const MAX_COMPLETION_ITEMS: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageServiceRequestSlot {
    Other,
    Completion,
    SignatureHelp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SignatureHelpUiState {
    Hidden,
    Loading,
    Ready,
    Empty,
    Unavailable(String),
    Error(String),
}

fn next_completion_selection(current: usize, item_count: usize, delta: isize) -> usize {
    if item_count == 0 {
        return 0;
    }
    let current = current.min(item_count - 1) as isize;
    (current + delta).rem_euclid(item_count as isize) as usize
}

fn signature_help_render_eligible(
    state: &LanguageServiceSnapshot,
    status: &SignatureHelpUiState,
    requested_position: Option<LspPosition>,
    current_position: LspPosition,
    current_version: DocumentVersion,
    current_generation: u64,
) -> bool {
    !matches!(status, SignatureHelpUiState::Hidden)
        && requested_position == Some(current_position)
        && state.version == current_version
        && state.generation == current_generation
}

fn response_error_text(
    response: &Result<LanguageServiceResponse, labonair_editor::LanguageServiceError>,
) -> String {
    response
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_else(|| "language service returned an invalid response".to_string())
}

fn completion_kind_label(kind: labonair_editor::CompletionKind) -> &'static str {
    match kind {
        labonair_editor::CompletionKind::Keyword => "keyword",
        labonair_editor::CompletionKind::Function => "function",
        labonair_editor::CompletionKind::Variable => "variable",
        labonair_editor::CompletionKind::Type => "type",
        labonair_editor::CompletionKind::Field => "field",
        labonair_editor::CompletionKind::Snippet => "snippet",
        labonair_editor::CompletionKind::Text => "text",
    }
}

fn signature_help_message(palette: Palette, message: impl Into<SharedString>) -> impl IntoElement {
    div()
        .px_2()
        .pb_2()
        .whitespace_normal()
        .text_xs()
        .text_color(palette.muted)
        .child(message.into())
}

pub struct EditorView {
    doc: Document,
    theme: Entity<ThemeStore>,
    focus_handle: FocusHandle,
    /// Content-area bounds from the last paint (window-relative).
    bounds: Option<Bounds<Pixels>>,
    /// First visible buffer line.
    scroll_top: usize,
    /// Horizontal display-column offset for long unwrapped lines.
    scroll_left: usize,
    /// Editor-owned folded logical ranges, persisted through the editor session
    /// contract and embedded by Workspace in its outer session snapshot.
    folds: Vec<FoldRange>,
    /// Editor-internal groups. Workspace pane layouts remain a separate
    /// capability; this tree is changed only by Editor commands.
    editor_splits: labonair_editor::EditorSplitTree,
    split_drag_last: Option<f32>,
    /// Cached glyph metrics `(char_width, line_height)` in px.
    metrics: (f32, f32),
    gutter_width: f32,
    search: Option<EditorSearch>,
    project_search: ProjectSearchSession,
    pending_navigation: Option<Position>,
    /// Tree-sitter syntax highlighter for the current document (T06-002).
    syntax: SyntaxHighlighter,
    /// Vim keybinding state machine (T06-003) — `None` when Vim mode is off.
    vim: Option<Vim>,
    /// Live editor settings mirror (font/indent/line-numbers), refreshed from
    /// the layered `SettingsStore` (T13-003).
    prefs: EditorSettings,
    /// Settings-schema hover text (T19-006 Anweisung #5) — `Some((mouse
    /// position, description))` while the mouse is over a key/value in a
    /// `config.json`/`.labonair/settings.json` tab and that key
    /// path has a schema description. `None` everywhere else (including
    /// every non-settings file — [`Self::is_settings_json`] gates this on
    /// every mouse-move so a plain source file never pays the tree-sitter
    /// lookup cost).
    hover: Option<(Point<Pixels>, SharedString)>,
    /// Revision-bound Git projection. Repository state and actions remain in
    /// the Git/Project-Diff capability; this view only renders it.
    git_gutter: GitGutterSnapshot,
    git_provider: Option<Arc<dyn GitDecorationProvider>>,
    git_action_sink: Option<Arc<dyn HunkActionSink>>,
    git_session_id: Option<String>,
    git_refresh_generation: u64,
    git_context_menu: Option<(Point<Pixels>, String, Revision)>,
    text_context_menu: Option<Point<Pixels>>,
    git_discard_confirmation: Option<GitDiscardPreview>,
    /// Editor-owned language-service runtime and its revision-safe result
    /// projection. Workspace only injects the runtime at composition time.
    language_services: Arc<LanguageServiceRuntime>,
    language_service_snapshot: LanguageServiceRuntimeSnapshot,
    language_service_state: LanguageServiceState,
    language_service_uri: Option<String>,
    language_service_generation: u64,
    language_service_open: bool,
    diagnostic_index: usize,
    completion_visible: bool,
    completion_request: Option<Task<()>>,
    completion_selected: usize,
    completion_request_serial: Option<u64>,
    completion_position: Option<LspPosition>,
    hover_request: Option<Task<()>>,
    hover_position: Option<Position>,
    signature_help_state: SignatureHelpUiState,
    signature_help_request_serial: Option<u64>,
    signature_help_position: Option<LspPosition>,
    language_service_request_serial: u64,
    code_actions: Option<Vec<labonair_editor::CodeAction>>,
    rename_requested: bool,
    rename_input: Option<Entity<InputState>>,
    format_on_save_in_flight: bool,
    cursor_blink_on: bool,
    editor_focused: bool,
    /// Restored after the asynchronous file load has established a valid
    /// document length. Dropping it on a path mismatch is corruption-safe.
    pending_session: Option<EditorSessionSnapshot>,
    recovery_prompt: Option<labonair_editor::UnsavedBufferRecovery>,
    autosave: Option<Task<()>>,
    _blink: Task<()>,
}

/// The current editor-settings snapshot (all-defaults before the store exists).
fn current_prefs(cx: &App) -> EditorSettings {
    EditorSettings::try_get(cx).cloned().unwrap_or_else(|| {
        EditorSettings::from_settings(&labonair_settings::SettingsContent::default())
    })
}

/// Apply the editor's Git display policy to the existing inline-diff
/// projection. This policy only controls word-level decorations; line-level
/// gutter markers and Git hunk actions use their existing independent paths.
fn git_inline_decorations_for_view(
    prefs: &EditorSettings,
    git: &GitGutterSnapshot,
    buffer: &labonair_editor::BufferSnapshot,
    display: &labonair_editor::DisplaySnapshot,
) -> Vec<labonair_editor::git::DisplayGitInlineDecoration> {
    if !prefs.git_gutter() || !prefs.git_word_diff() {
        return Vec::new();
    }
    git.display_inline_decorations(buffer, display)
}

fn vim_options(p: &EditorSettings) -> VimOptions {
    VimOptions {
        number: p.line_numbers(),
        relativenumber: p.relative_line_numbers(),
        hlsearch: p.vim_hlsearch(),
        incsearch: p.vim_incsearch(),
        smartcase: p.vim_smartcase(),
        expandtab: !p.indent_with_tabs(),
        tabstop: p.tab_size() as usize,
        shiftwidth: p.tab_size() as usize,
    }
}

impl EditorView {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        Self::new_with_language_services(theme, Arc::new(LanguageServiceRuntime::default()), cx)
    }

    pub fn new_with_language_services(
        theme: Entity<ThemeStore>,
        language_services: Arc<LanguageServiceRuntime>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe_global::<SettingsStore>(|this, cx| this.apply_prefs(cx))
            .detach();
        let prefs = current_prefs(cx);
        let vim = prefs.vim_mode().then(|| Vim::new(vim_options(&prefs)));
        language_services.set_presentation_policy(
            prefs.diagnostics(),
            prefs.completion(),
            prefs.hover(),
            prefs.semantic_tokens(),
        );
        let language_service_snapshot =
            language_services.snapshot(Language::PlainText, DocumentVersion::new(0), 0);

        // Drive the blinking caret. The interval/on-off preference is re-read
        // every tick so an `editorCursorBlink*` change takes effect on the
        // next toggle; the repaint is skipped unless this editor is focused,
        // so idle/background editors stay quiet.
        let blink = cx.spawn(async move |view, cx| loop {
            let (ms, blink_pref) = view
                .read_with(cx, |_, cx| current_prefs(cx))
                .map(|p| (p.cursor_blink_interval_ms(), p.cursor_blink()))
                .unwrap_or((530, true));
            cx.background_executor()
                .timer(std::time::Duration::from_millis(ms))
                .await;
            if view
                .update(cx, |this, cx| {
                    this.cursor_blink_on = !this.cursor_blink_on;
                    if blink_pref && this.editor_focused {
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
        });
        Self {
            doc: Document::empty(),
            theme,
            focus_handle: cx.focus_handle(),
            bounds: None,
            scroll_top: 0,
            scroll_left: 0,
            folds: Vec::new(),
            editor_splits: labonair_editor::EditorSplitTree::default(),
            split_drag_last: None,
            metrics: (8.0, 18.0),
            gutter_width: 48.0,
            search: None,
            project_search: ProjectSearchSession::default(),
            pending_navigation: None,
            syntax: SyntaxHighlighter::new(Language::PlainText),
            vim,
            prefs,
            hover: None,
            git_gutter: GitGutterSnapshot::unavailable("", Revision::default()),
            git_provider: None,
            git_action_sink: None,
            git_session_id: None,
            git_refresh_generation: 0,
            git_context_menu: None,
            text_context_menu: None,
            git_discard_confirmation: None,
            language_services,
            language_service_snapshot,
            language_service_state: LanguageServiceState::new(DocumentVersion::new(0)),
            language_service_uri: None,
            language_service_generation: 0,
            language_service_open: false,
            diagnostic_index: 0,
            completion_visible: false,
            completion_request: None,
            completion_selected: 0,
            completion_request_serial: None,
            completion_position: None,
            hover_request: None,
            hover_position: None,
            signature_help_state: SignatureHelpUiState::Hidden,
            signature_help_request_serial: None,
            signature_help_position: None,
            language_service_request_serial: 0,
            code_actions: None,
            rename_requested: false,
            rename_input: None,
            format_on_save_in_flight: false,
            cursor_blink_on: true,
            editor_focused: false,
            pending_session: None,
            recovery_prompt: None,
            autosave: None,
            _blink: blink,
        }
    }

    /// Replace the injected Editor-owned runtime before a document is opened.
    /// This is the narrow app composition seam for a validated local process.
    pub fn set_language_services(&mut self, language_services: Arc<LanguageServiceRuntime>) {
        self.close_language_service_document();
        self.language_services = language_services;
        self.language_service_snapshot =
            self.language_services
                .snapshot(self.doc.language, DocumentVersion::new(0), 0);
        self.language_service_open = false;
        self.language_service_uri = None;
    }

    pub fn language_service_snapshot(&self) -> LanguageServiceRuntimeSnapshot {
        self.language_service_snapshot.clone()
    }

    /// Execute a typed Editor command from the canonical command registry.
    /// Workspace supplies only the active view and never interprets the
    /// document or split-tree state itself.
    pub fn execute_command(
        &mut self,
        command: EditorCommand,
        cx: &mut Context<Self>,
    ) -> EditorCommandResult {
        match command {
            EditorCommand::Edit(intent) => {
                let revision_before = self.editor_revision();
                let dirty_before = self.doc.is_dirty();
                let selections_before = self.doc.selection_set();
                let result = self.doc.execute_edit(intent);
                self.finish_command(
                    revision_before,
                    dirty_before,
                    selections_before,
                    result.changed,
                    cx,
                );
                result
            }
            EditorCommand::Split(intent) => {
                let before = self.editor_splits.clone();
                let result = match intent {
                    SplitIntent::Right => self.editor_splits.split_right().map(|_| ()),
                    SplitIntent::Down => self.editor_splits.split_down().map(|_| ()),
                    SplitIntent::FocusNext => {
                        self.editor_splits.focus_next();
                        Ok(())
                    }
                    SplitIntent::FocusPrevious => {
                        self.editor_splits.focus_previous();
                        Ok(())
                    }
                    SplitIntent::Close => self.editor_splits.close_focused().map(|_| ()),
                    SplitIntent::CloseOthers => self.editor_splits.close_other_groups(),
                    SplitIntent::Resize { delta_milli } => self.editor_splits.resize(
                        self.editor_splits.focused_group(),
                        f32::from(delta_milli) / 1000.0,
                    ),
                };
                if result.is_ok() && self.editor_splits != before {
                    cx.emit(EditorEvent::Changed);
                    cx.notify();
                    EditorCommandResult {
                        changed: true,
                        events: vec![labonair_editor::EditorViewEvent::SplitChanged],
                    }
                } else {
                    EditorCommandResult {
                        changed: false,
                        events: Vec::new(),
                    }
                }
            }
        }
    }

    /// Execute a language-service command using the current immutable
    /// document snapshot. Requests stay on the Editor runtime boundary and
    /// are never interpreted by Workspace or the shell.
    pub fn execute_language_command(
        &mut self,
        command: EditorLanguageCommand,
        cx: &mut Context<Self>,
    ) {
        match command {
            EditorLanguageCommand::TriggerCompletion => {
                self.dismiss_completion();
                self.schedule_completion(cx);
            }
            EditorLanguageCommand::GoToDefinition
            | EditorLanguageCommand::GoToDeclaration
            | EditorLanguageCommand::GoToImplementation
            | EditorLanguageCommand::PeekDefinition => {
                self.request_language_service(
                    LanguageServiceRequest::Definition {
                        position: self.current_lsp_position(),
                    },
                    cx,
                );
            }
            EditorLanguageCommand::GoToReferences => {
                self.request_language_service(
                    LanguageServiceRequest::References {
                        position: self.current_lsp_position(),
                    },
                    cx,
                );
            }
            EditorLanguageCommand::Rename => {
                self.rename_requested = true;
                self.completion_visible = false;
                cx.notify();
            }
            EditorLanguageCommand::CodeAction => {
                self.code_actions = None;
                self.request_language_service(
                    LanguageServiceRequest::CodeActions {
                        range: self.current_lsp_range(),
                    },
                    cx,
                );
            }
            EditorLanguageCommand::FormatDocument => {
                self.request_language_service(LanguageServiceRequest::Formatting, cx);
            }
            EditorLanguageCommand::FormatSelection => {
                let Some((start, end)) = self.doc.selection() else {
                    notify_info(
                        cx,
                        "Format selection",
                        "Select a range before formatting it.",
                    );
                    return;
                };
                let snapshot = self.doc.snapshot();
                self.request_language_edit(
                    LanguageServiceEditRequest::FormatSelection {
                        range: LspRange::from_text_range(
                            &snapshot,
                            TextRange::new(
                                snapshot.position_to_byte(start),
                                snapshot.position_to_byte(end),
                            ),
                        ),
                    },
                    cx,
                );
            }
            EditorLanguageCommand::OrganizeImports => {
                self.request_language_edit(LanguageServiceEditRequest::OrganizeImports, cx);
            }
            EditorLanguageCommand::RestartLanguageServer => {
                self.control_language_service(true, cx);
            }
            EditorLanguageCommand::StopLanguageServer => {
                self.control_language_service(false, cx);
            }
            EditorLanguageCommand::ShowDiagnostics => self.show_diagnostics(cx),
            EditorLanguageCommand::NextDiagnostic => self.navigate_diagnostic(true, cx),
            EditorLanguageCommand::PreviousDiagnostic => self.navigate_diagnostic(false, cx),
        }
    }

    /// Snapshot of the Editor-owned internal split tree for integration and
    /// persistence tests. Workspace does not receive mutable access.
    pub fn editor_split_tree(&self) -> &labonair_editor::EditorSplitTree {
        &self.editor_splits
    }

    fn current_lsp_position(&self) -> LspPosition {
        let snapshot = self.doc.snapshot();
        LspPosition::from_position(&snapshot, self.doc.cursor)
    }

    fn current_lsp_range(&self) -> LspRange {
        let snapshot = self.doc.snapshot();
        let (start, end) = self
            .doc
            .selection()
            .unwrap_or((self.doc.cursor, self.doc.cursor));
        LspRange::from_text_range(
            &snapshot,
            TextRange::new(
                snapshot.position_to_byte(start),
                snapshot.position_to_byte(end),
            ),
        )
    }

    fn finish_command(
        &mut self,
        revision_before: Revision,
        dirty_before: bool,
        selections_before: labonair_editor::SelectionSet,
        changed: bool,
        cx: &mut Context<Self>,
    ) {
        let revision_changed = self.editor_revision() != revision_before;
        let selection_changed = self.doc.selection_set() != selections_before;
        if !(changed || revision_changed || selection_changed) {
            return;
        }
        self.cursor_blink_on = true;
        self.ensure_cursor_visible();
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if !dirty_before && self.doc.is_dirty() {
            cx.emit(EditorEvent::Edited);
        }
        if revision_changed {
            self.sync_language_service_document(cx);
            self.refresh_git(cx);
        }
        self.schedule_autosave(cx);
        cx.notify();
    }

    fn control_language_service(&mut self, restart: bool, cx: &mut Context<Self>) {
        let runtime = self.language_services.clone();
        let language = LanguageId::new(self.doc.language);
        cx.spawn(async move |this, cx| {
            let result = if restart {
                runtime.restart_process(language).await
            } else {
                runtime.stop_process(language).await
            };
            let _ = this.update(cx, |this, cx| {
                if let Err(error) = result {
                    notify(cx, "Language service", &error.to_string());
                }
                let snapshot = this.doc.snapshot();
                this.language_service_snapshot = runtime.snapshot(
                    this.doc.language,
                    DocumentVersion::from_revision(snapshot.revision()),
                    this.language_service_generation,
                );
                cx.notify();
            });
        })
        .detach();
    }

    fn show_diagnostics(&self, cx: &mut Context<Self>) {
        let diagnostics = self.language_service_state.snapshot().diagnostics;
        let body = if diagnostics.is_empty() {
            "No diagnostics for the current document.".to_string()
        } else {
            format!(
                "{} diagnostic(s). Use Next/Previous Diagnostic to navigate.",
                diagnostics.len()
            )
        };
        notify_info(cx, "Editor diagnostics", &body);
    }

    fn navigate_diagnostic(&mut self, forward: bool, cx: &mut Context<Self>) {
        let diagnostics = self.language_service_state.snapshot().diagnostics;
        if diagnostics.is_empty() {
            notify_info(
                cx,
                "Editor diagnostics",
                "No diagnostics for the current document.",
            );
            return;
        }
        if forward {
            self.diagnostic_index = (self.diagnostic_index + 1) % diagnostics.len();
        } else if self.diagnostic_index == 0 {
            self.diagnostic_index = diagnostics.len() - 1;
        } else {
            self.diagnostic_index -= 1;
        }
        let position = diagnostics[self.diagnostic_index]
            .range
            .start
            .to_position(&self.doc.snapshot());
        self.doc.set_caret(position, false);
        self.ensure_cursor_visible();
        cx.emit(EditorEvent::Changed);
        cx.notify();
    }

    fn language_service_uri(&self) -> String {
        self.doc
            .path
            .as_ref()
            .map(|path| format!("file://{}", path.to_string_lossy()))
            .unwrap_or_else(|| "untitled://labonair/editor".to_string())
    }

    fn close_language_service_document(&mut self) {
        let (Some(uri), true) = (self.language_service_uri.take(), self.language_service_open)
        else {
            return;
        };
        self.language_service_open = false;
        let language = LanguageId::new(self.doc.language);
        let _ = self
            .language_services
            .close_document_in_background(uri, language, |_| {});
    }

    fn sync_language_service_document(&mut self, cx: &mut Context<Self>) {
        let uri = self.language_service_uri();
        let was_open = self.language_service_open
            && self.language_service_uri.as_deref() == Some(uri.as_str());
        let snapshot = self.doc.snapshot();
        self.language_service_snapshot = self.language_services.snapshot(
            self.doc.language,
            DocumentVersion::from_revision(snapshot.revision()),
            self.language_service_generation,
        );
        let version = DocumentVersion::from_revision(snapshot.revision());
        let generation = self.language_service_generation.saturating_add(1);
        self.language_service_generation = generation;
        self.language_service_uri = Some(uri.clone());
        self.language_service_open = true;
        self.language_service_state
            .set_document_generation(version, generation);
        self.completion_visible = false;
        self.completion_request = None;
        self.completion_selected = 0;
        self.completion_request_serial = None;
        self.completion_position = None;
        self.signature_help_state = SignatureHelpUiState::Hidden;
        self.signature_help_request_serial = None;
        self.signature_help_position = None;
        self.code_actions = None;
        self.rename_requested = false;
        self.rename_input = None;
        self.format_on_save_in_flight = false;
        let language = LanguageId::new(self.doc.language);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let runtime = self.language_services.clone();
        if was_open {
            let _ = runtime.change_document_in_background(
                uri.clone(),
                language,
                snapshot,
                version,
                generation,
                move |result| {
                    let _ = sender.send(result);
                },
            );
        } else {
            let _ = runtime.open_document_in_background(
                uri.clone(),
                language,
                snapshot,
                version,
                generation,
                move |result| {
                    let _ = sender.send(result);
                },
            );
        }
        cx.spawn(async move |this, cx| {
            let result = receiver.await.unwrap_or_else(|_| {
                Err(labonair_editor::LanguageServiceError::ProviderFailed(
                    "language-service synchronization was cancelled".into(),
                ))
            });
            let _ = this.update(cx, |this, cx| {
                if this.language_service_uri.as_deref() != Some(uri.as_str())
                    || this.language_service_generation != generation
                {
                    return;
                }
                if let Err(error) = result {
                    this.language_service_snapshot =
                        runtime.snapshot(this.doc.language, version, generation);
                    notify(cx, "Language service", &error.to_string());
                } else {
                    this.language_service_snapshot =
                        runtime.snapshot(this.doc.language, version, generation);
                    this.request_language_service(LanguageServiceRequest::Diagnostics, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Dispatch any Editor language-service request without blocking GPUI.
    pub fn request_language_service(
        &mut self,
        request: LanguageServiceRequest,
        cx: &mut Context<Self>,
    ) {
        let (slot, request_position) = match &request {
            LanguageServiceRequest::Completion { position, .. } => {
                (LanguageServiceRequestSlot::Completion, Some(*position))
            }
            LanguageServiceRequest::SignatureHelp { position, .. } => {
                (LanguageServiceRequestSlot::SignatureHelp, Some(*position))
            }
            _ => (LanguageServiceRequestSlot::Other, None),
        };
        self.language_service_request_serial =
            self.language_service_request_serial.saturating_add(1);
        let request_serial = self.language_service_request_serial;
        match slot {
            LanguageServiceRequestSlot::Completion => {
                self.completion_request_serial = Some(request_serial);
                self.completion_position = request_position;
            }
            LanguageServiceRequestSlot::SignatureHelp => {
                self.signature_help_request_serial = Some(request_serial);
                self.signature_help_position = request_position;
            }
            LanguageServiceRequestSlot::Other => {}
        }
        let snapshot = self.doc.snapshot();
        let version = DocumentVersion::from_revision(snapshot.revision());
        let generation = self.language_service_generation;
        let input = LocalLanguageServiceRequest {
            uri: self.language_service_uri(),
            snapshot,
            version,
            generation,
            request,
            cancellation: CancellationToken::default(),
        };
        let (sender, receiver) = tokio::sync::oneshot::channel::<LanguageServiceResult>();
        let runtime = self.language_services.clone();
        let _ = runtime.request_in_background(
            LanguageId::new(self.doc.language),
            input,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let weak = cx.weak_entity();
        cx.spawn(async move |_this, cx| {
            let Ok(result) = receiver.await else { return };
            let _ = weak.update(cx, |this, cx| {
                if this.language_service_generation != result.generation
                    || this.editor_revision().value() != result.version.value()
                    || !this.is_current_language_service_request(
                        slot,
                        request_serial,
                        request_position,
                    )
                {
                    return;
                }
                let response = result.response.clone();
                let accepted = this.language_service_state.apply(result);
                if accepted
                    || (response.is_err()
                        && (matches!(slot, LanguageServiceRequestSlot::Completion)
                            || matches!(slot, LanguageServiceRequestSlot::SignatureHelp)
                            || this.format_on_save_in_flight))
                {
                    this.apply_language_service_response(response, slot, cx);
                    this.language_service_snapshot =
                        runtime.snapshot(this.doc.language, version, generation);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn is_current_language_service_request(
        &self,
        slot: LanguageServiceRequestSlot,
        request_serial: u64,
        request_position: Option<LspPosition>,
    ) -> bool {
        match slot {
            LanguageServiceRequestSlot::Other => true,
            LanguageServiceRequestSlot::Completion => {
                self.completion_request_serial == Some(request_serial)
                    && self.completion_position == request_position
                    && request_position
                        .is_some_and(|position| self.current_lsp_position() == position)
            }
            LanguageServiceRequestSlot::SignatureHelp => {
                self.signature_help_request_serial == Some(request_serial)
                    && self.signature_help_position == request_position
                    && request_position
                        .is_some_and(|position| self.current_lsp_position() == position)
            }
        }
    }

    fn request_language_edit(
        &mut self,
        request: LanguageServiceEditRequest,
        cx: &mut Context<Self>,
    ) {
        let snapshot = self.doc.snapshot();
        let version = DocumentVersion::from_revision(snapshot.revision());
        let generation = self.language_service_generation;
        let runtime = self.language_services.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let _ = runtime.request_edit_in_background(
            labonair_editor::BackgroundEditRequest {
                uri: self.language_service_uri(),
                language: LanguageId::new(self.doc.language),
                snapshot,
                version,
                generation,
                request,
                cancellation: CancellationToken::default(),
            },
            move |result| {
                let _ = sender.send(result);
            },
        );
        let weak = cx.weak_entity();
        cx.spawn(async move |_this, cx| {
            let Ok(result) = receiver.await else { return };
            let _ = weak.update(cx, |this, cx| {
                if this.language_service_generation != result.generation
                    || this.editor_revision().value() != result.version.value()
                {
                    return;
                }
                match result.response {
                    Ok(LanguageServiceEditResponse::Formatting(edits))
                    | Ok(LanguageServiceEditResponse::OrganizeImports(edits)) => {
                        if edits.is_empty() {
                            notify_info(cx, "Editor", "No changes are needed here.");
                        } else {
                            this.apply_service_edits(edits, cx);
                        }
                    }
                    Err(error) => notify(cx, "Editor operation unavailable", &error.to_string()),
                }
            });
        })
        .detach();
    }

    fn apply_language_service_response(
        &mut self,
        response: Result<LanguageServiceResponse, labonair_editor::LanguageServiceError>,
        slot: LanguageServiceRequestSlot,
        cx: &mut Context<Self>,
    ) {
        let Ok(response) = response else {
            if slot == LanguageServiceRequestSlot::Completion {
                self.dismiss_completion();
            }
            if slot == LanguageServiceRequestSlot::SignatureHelp {
                self.language_service_state.clear_signature_help();
                self.signature_help_state = match &response {
                    Err(labonair_editor::LanguageServiceError::UnsupportedCapability(_))
                    | Err(labonair_editor::LanguageServiceError::UnsupportedLanguage(_)) => {
                        SignatureHelpUiState::Unavailable(response_error_text(&response))
                    }
                    Err(_) => SignatureHelpUiState::Error(response_error_text(&response)),
                    Ok(_) => SignatureHelpUiState::Hidden,
                };
            }
            if self.format_on_save_in_flight {
                // Keep the one-shot guard set while re-entering `save`; the
                // fallback write must not start an endless format request.
                notify_info(
                    cx,
                    "Format on save",
                    "The language service could not format this document; saving the current text.",
                );
                self.save(cx);
            }
            return;
        };
        match response {
            LanguageServiceResponse::Definition(targets)
            | LanguageServiceResponse::References(targets) => {
                if let Some(target) = targets.first() {
                    let position = target
                        .selection_range
                        .start
                        .to_position(&self.doc.snapshot());
                    self.doc.set_caret(position, false);
                    self.ensure_cursor_visible();
                    cx.emit(EditorEvent::Changed);
                }
            }
            LanguageServiceResponse::Formatting(edits) => {
                let save_after_format = self.format_on_save_in_flight;
                self.apply_service_edits(edits, cx);
                if save_after_format {
                    // Re-enter save once with the formatting result already
                    // applied. `save` clears this guard before writing.
                    self.save(cx);
                }
            }
            LanguageServiceResponse::CodeActions(actions) if actions.len() == 1 => {
                self.apply_service_edits(actions[0].edits.clone(), cx);
            }
            LanguageServiceResponse::CodeActions(actions) => {
                if actions.is_empty() {
                    notify_info(cx, "Code actions", "No code actions are available here.");
                } else {
                    self.code_actions = Some(actions);
                    cx.notify();
                }
            }
            LanguageServiceResponse::Rename(edits) if edits.is_empty() => {
                notify_info(cx, "Rename", "No rename edits are available here.");
            }
            LanguageServiceResponse::Rename(edits) => {
                self.apply_service_edits(edits, cx);
            }
            LanguageServiceResponse::Completion(list) if list.items.is_empty() => {
                self.completion_visible = false;
                self.completion_selected = 0;
                notify_info(cx, "Completion", "No completion items are available here.");
            }
            LanguageServiceResponse::Completion(list) => {
                self.completion_selected = self
                    .completion_selected
                    .min(list.items.len().min(MAX_COMPLETION_ITEMS).saturating_sub(1));
                self.completion_visible = true;
                cx.notify();
            }
            LanguageServiceResponse::SignatureHelp(Some(_)) => {
                self.signature_help_state = SignatureHelpUiState::Ready;
                cx.notify();
            }
            LanguageServiceResponse::SignatureHelp(None) => {
                self.signature_help_state = SignatureHelpUiState::Empty;
                cx.notify();
            }
            _ => {}
        }
    }

    fn completion_range(&self, snapshot: &labonair_editor::BufferSnapshot) -> TextRange {
        let line = snapshot.line(self.doc.cursor.line);
        let chars = line.chars().collect::<Vec<_>>();
        let end = self.doc.cursor.column.min(chars.len());
        let start = chars[..end]
            .iter()
            .rposition(|ch| !(ch.is_alphanumeric() || *ch == '_'))
            .map_or(0, |index| index + 1);
        TextRange::new(
            snapshot.position_to_byte(Position::new(self.doc.cursor.line, start)),
            snapshot.position_to_byte(Position::new(self.doc.cursor.line, end)),
        )
    }

    fn signature_help_trigger(&self) -> Option<char> {
        let line = self.doc.snapshot().line(self.doc.cursor.line);
        line.chars()
            .nth(self.doc.cursor.column.saturating_sub(1))
            .filter(|character| matches!(character, '(' | ','))
    }

    fn request_signature_help(&mut self, trigger: Option<char>, cx: &mut Context<Self>) {
        if !self.language_service_open {
            self.signature_help_state = SignatureHelpUiState::Unavailable(
                "No language service is active for this document.".to_string(),
            );
            self.signature_help_position = None;
            cx.notify();
            return;
        }
        let position = self.current_lsp_position();
        self.signature_help_state = SignatureHelpUiState::Loading;
        self.signature_help_position = Some(position);
        self.request_language_service(
            LanguageServiceRequest::SignatureHelp { position, trigger },
            cx,
        );
    }

    fn move_completion_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let item_count = self
            .language_service_state
            .snapshot()
            .completion
            .map(|list| list.items.len().min(MAX_COMPLETION_ITEMS))
            .unwrap_or(0);
        if item_count == 0 {
            return;
        }
        self.completion_selected =
            next_completion_selection(self.completion_selected, item_count, delta);
        cx.notify();
    }

    fn dismiss_completion(&mut self) {
        self.completion_visible = false;
        self.completion_selected = 0;
        self.completion_request_serial = None;
        self.completion_position = None;
    }

    fn dismiss_language_popovers(&mut self) {
        self.dismiss_completion();
        self.signature_help_state = SignatureHelpUiState::Hidden;
        self.signature_help_request_serial = None;
        self.signature_help_position = None;
    }

    fn apply_completion_item(&mut self, index: usize, cx: &mut Context<Self>) {
        let state = self.language_service_state.snapshot();
        let current_version = DocumentVersion::from_revision(self.editor_revision());
        if state.version != current_version
            || state.generation != self.language_service_generation
            || self.completion_position != Some(self.current_lsp_position())
        {
            self.dismiss_completion();
            return;
        }
        let Some(item) = state
            .completion
            .and_then(|list| list.items.get(index).cloned())
        else {
            self.dismiss_completion();
            return;
        };
        let snapshot = self.doc.snapshot();
        let range = self.completion_range(&snapshot);
        let before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let selections = self.doc.selection_set();
        if self.doc.apply_edits(
            vec![Edit::new(range, item.insert_text)],
            selections,
            labonair_editor::EditSource::Completion,
            Some("Completion".to_string()),
        ) {
            self.dismiss_completion();
            self.finish_command(before, dirty_before, self.doc.selection_set(), true, cx);
        }
    }

    fn apply_code_action(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(action) = self
            .code_actions
            .as_ref()
            .and_then(|actions| actions.get(index))
            .cloned()
        else {
            self.code_actions = None;
            return;
        };
        self.code_actions = None;
        self.apply_service_edits(action.edits, cx);
    }

    fn schedule_completion(&mut self, cx: &mut Context<Self>) {
        if !self.prefs.completion() {
            return;
        }
        let delay = self.prefs.autocomplete_debounce_ms();
        self.completion_request = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(u64::from(delay)))
                .await;
            let _ = view.update(cx, |this, cx| {
                this.completion_request = None;
                this.request_language_service(
                    LanguageServiceRequest::Completion {
                        position: this.current_lsp_position(),
                        trigger: None,
                    },
                    cx,
                );
            });
        }));
    }

    /// Debounced hover requests keep pointer movement cheap while exposing
    /// the same typed LSP hover result used by the local process and syntax
    /// fallback providers.
    fn schedule_language_hover(&mut self, position: Position, cx: &mut Context<Self>) {
        if !self.prefs.hover() || self.hover_position == Some(position) {
            return;
        }
        self.hover_position = Some(position);
        self.hover_request = None;
        self.language_service_state.clear_hover();
        self.hover_request = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(450))
                .await;
            let _ = view.update(cx, |this, cx| {
                this.hover_request = None;
                if this.hover_position != Some(position) {
                    return;
                }
                let snapshot = this.doc.snapshot();
                this.request_language_service(
                    LanguageServiceRequest::Hover {
                        position: LspPosition::from_position(&snapshot, position),
                    },
                    cx,
                );
            });
        }));
    }

    fn submit_rename(&mut self, new_name: String, cx: &mut Context<Self>) {
        let new_name = new_name.trim().to_string();
        self.rename_requested = false;
        self.rename_input = None;
        if new_name.is_empty() {
            cx.notify();
            return;
        }
        self.request_language_service(
            LanguageServiceRequest::Rename {
                position: self.current_lsp_position(),
                new_name,
            },
            cx,
        );
    }

    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.rename_requested = false;
        self.rename_input = None;
        cx.notify();
    }

    fn ensure_rename_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.rename_requested || self.rename_input.is_some() {
            return;
        }
        let snapshot = self.doc.snapshot();
        let seed = snapshot.text_range(self.completion_range(&snapshot));
        let input = cx.new(|cx| {
            text_field(window, cx)
                .placeholder("New symbol name")
                .default_value(seed)
        });
        let view = cx.entity();
        window
            .subscribe(&input, cx, move |input, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { secondary: false } = event {
                    let value = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| this.submit_rename(value, cx));
                }
            })
            .detach();
        window.focus(&input.read(cx).focus_handle(cx));
        self.rename_input = Some(input);
    }

    fn apply_service_edits(&mut self, edits: Vec<Edit>, cx: &mut Context<Self>) {
        if edits.is_empty() {
            return;
        }
        let revision_before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let selections_before = self.doc.selection_set();
        if self.doc.apply_edits(
            edits.clone(),
            selections_before.clone(),
            labonair_editor::EditSource::Format,
            Some("Language service edit".to_string()),
        ) {
            self.finish_command(revision_before, dirty_before, selections_before, true, cx);
        }
    }

    /// Inject the workspace-owned Git contracts. The editor keeps only the
    /// provider and action sink; repository state and mutations remain outside
    /// this view.
    pub fn set_git_bridge(
        &mut self,
        provider: Arc<dyn GitDecorationProvider>,
        action_sink: Arc<dyn HunkActionSink>,
    ) {
        self.git_provider = Some(provider);
        self.git_action_sink = Some(action_sink);
        self.invalidate_git_requests();
        self.git_gutter = GitGutterSnapshot::unavailable(
            self.current_git_path().unwrap_or_default(),
            self.editor_revision(),
        );
    }

    /// Whether this tab's file is a settings.json the schema-hover helper
    /// applies to: the user file (`~/.config/labonair/config.json`)
    /// or a project file (`<root>/.labonair/settings.json`, T19-003).
    fn is_settings_json(&self) -> bool {
        let Some(path) = &self.doc.path else {
            return false;
        };
        if *path == labonair_settings::user_settings_path() {
            return true;
        }
        path.file_name().and_then(|n| n.to_str()) == Some("settings.json")
            && path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some(".labonair")
    }

    /// Byte offset into the document text for a buffer [`Position`]
    /// (line/column are char-based; this walks prior lines' byte lengths
    /// plus the target line's char-to-byte offset for the column — exact
    /// for the schema-hover lookup's purposes).
    fn byte_offset_for(&self, pos: Position) -> usize {
        let snapshot = self.doc.snapshot();
        let mut offset = 0usize;
        for i in 0..pos.line.min(snapshot.line_count()) {
            offset += snapshot.line(i).len() + 1;
        }
        if pos.line < snapshot.line_count() {
            let line = snapshot.line(pos.line);
            offset += line
                .char_indices()
                .nth(pos.column)
                .map(|(b, _)| b)
                .unwrap_or(line.len());
        }
        offset
    }

    /// Recompute [`Self::hover`] for a mouse position over the text area
    /// (T19-006 Anweisung #5). No-op (clears hover) unless
    /// [`Self::is_settings_json`] and the cursor lands inside a JSON pair
    /// whose key path has a schema description.
    fn update_hover(&mut self, mouse_pos: Point<Pixels>, cx: &mut Context<Self>) {
        let next = if self.is_settings_json() {
            let doc_pos = self.position_at(mouse_pos);
            let offset = self.byte_offset_for(doc_pos);
            let text = self.doc.snapshot().text();
            labonair_settings_json::json_path_at_offset(&text, offset).and_then(|path| {
                let refs: Vec<&str> = path.iter().map(String::as_str).collect();
                labonair_settings::description_for_path(&refs)
                    .map(|desc| (mouse_pos, SharedString::from(desc)))
            })
        } else {
            None
        };
        let changed = next.is_some() || self.hover.is_some();
        self.hover = next;
        if changed {
            cx.notify();
        }
    }

    /// Re-read the live preferences and reconcile the Vim layer without
    /// touching the document buffer (T13-003).
    fn apply_prefs(&mut self, cx: &mut Context<Self>) {
        let p = current_prefs(cx);
        if p == self.prefs {
            return;
        }
        self.prefs = p;
        self.cursor_blink_on = true;
        self.language_services.set_presentation_policy(
            self.prefs.diagnostics(),
            self.prefs.completion(),
            self.prefs.hover(),
            self.prefs.semantic_tokens(),
        );
        match (&mut self.vim, self.prefs.vim_mode()) {
            (Some(vim), true) => vim.options = vim_options(&self.prefs),
            (Some(_), false) => self.vim = None,
            (None, true) => self.vim = Some(Vim::new(vim_options(&self.prefs))),
            (None, false) => {}
        }
        self.language_service_snapshot = self.language_services.snapshot(
            self.doc.language,
            DocumentVersion::from_revision(self.editor_revision()),
            self.language_service_generation,
        );
        self.schedule_autosave(cx);
        cx.notify();
    }

    fn file_limit_bytes(&self) -> u64 {
        u64::from(self.prefs.max_file_size_mb()) * 1024 * 1024
    }

    /// One indentation step for a Tab press outside Vim mode.
    fn indent_unit(&self) -> String {
        if self.prefs.indent_with_tabs() {
            "\t".to_string()
        } else {
            " ".repeat((self.prefs.tab_size().max(1)) as usize)
        }
    }

    /// Soft-wrap width in columns (0 = wrapping off), from the
    /// `editor_word_wrap` preference and the measured content width (T06-005).
    /// Mirrors CodeMirror's `EditorView.lineWrapping`.
    fn wrap_cols(&self) -> usize {
        if !self.prefs.word_wrap() {
            return 0;
        }
        let (cw, _) = self.metrics;
        let width = self.bounds.map(|b| f32::from(b.size.width)).unwrap_or(0.0) - self.gutter_width;
        if cw <= 0.0 || width <= cw * 2.0 {
            return 0;
        }
        (width / cw).floor() as usize
    }

    fn content_width_columns(&self) -> usize {
        let (cw, _) = self.metrics;
        let width = self
            .bounds
            .map(|bounds| f32::from(bounds.size.width))
            .unwrap_or(640.0)
            - self.gutter_width;
        if cw <= 0.0 || width <= 0.0 {
            1
        } else {
            (width / cw).floor() as usize
        }
    }

    fn display_config(&self) -> DisplayConfig {
        let whitespace = match self.prefs.whitespace() {
            "all" => WhitespaceMode::All,
            "boundary" => WhitespaceMode::Boundary,
            _ => WhitespaceMode::None,
        };
        DisplayConfig {
            wrap_columns: (self.wrap_cols() > 0).then_some(self.wrap_cols()),
            show_line_numbers: self
                .vim
                .as_ref()
                .map(|vim| vim.options.number)
                .unwrap_or_else(|| self.prefs.line_numbers()),
            relative_line_numbers: self
                .vim
                .as_ref()
                .map(|vim| vim.options.relativenumber)
                .unwrap_or_else(|| self.prefs.relative_line_numbers()),
            indentation_guides: self.prefs.indentation_guides(),
            whitespace,
            scroll_beyond_last_line: self.prefs.scroll_beyond_last_line(),
        }
    }

    /// Build the visible editor-owned display snapshot. The bootstrap map
    /// contains coordinates only, then the final map retains just the rows
    /// needed by this paint pass.
    fn display_snapshot(
        &self,
        snapshot: &labonair_editor::BufferSnapshot,
    ) -> labonair_editor::DisplaySnapshot {
        let config = self.display_config();
        let bootstrap = DisplayMap::build(
            snapshot,
            Viewport {
                height_rows: snapshot.line_count().saturating_add(1),
                ..Default::default()
            },
            config,
            &self.folds,
        );
        let top_row = bootstrap.logical_line_to_visual_row(self.scroll_top);
        DisplayMap::build(
            snapshot,
            Viewport {
                top_row,
                left_column: self.scroll_left,
                height_rows: self.visible_rows().saturating_add(1),
                width_columns: self.wrap_cols().max(self.content_width_columns()).max(1),
            },
            config,
            &self.folds,
        )
    }

    /// Vertical caret motion across visual (wrapped) rows. Returns `false` when
    /// soft-wrap is off so the caller falls back to logical [`Motion`].
    fn wrap_vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        let cols = self.wrap_cols();
        if cols == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let snapshot = self.doc.snapshot();
        let seg = cur.column / cols;
        let cin = cur.column % cols;
        let last_seg = Wrap { cols }
            .rows(snapshot.line_len(cur.line))
            .saturating_sub(1);
        let last_line = snapshot.line_count().saturating_sub(1);
        let target = if down {
            if seg < last_seg {
                Position::new(cur.line, (seg + 1) * cols + cin)
            } else if cur.line >= last_line {
                return true;
            } else {
                Position::new(cur.line + 1, cin)
            }
        } else if seg > 0 {
            Position::new(cur.line, (seg - 1) * cols + cin)
        } else if cur.line == 0 {
            return true;
        } else {
            let p = cur.line - 1;
            let plast = Wrap { cols }.rows(snapshot.line_len(p)).saturating_sub(1);
            Position::new(p, plast * cols + cin)
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible();
        cx.notify();
        true
    }

    /// Home / End across the current visual row. Returns `false` when soft-wrap
    /// is off.
    fn wrap_horizontal(&mut self, end: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        let cols = self.wrap_cols();
        if cols == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let seg = cur.column / cols;
        let line_len = self.doc.snapshot().line_len(cur.line);
        let target = if end {
            Position::new(cur.line, ((seg + 1) * cols).min(line_len))
        } else {
            Position::new(cur.line, seg * cols)
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible();
        cx.notify();
        true
    }

    /// Point the highlighter at the current document's language and force a
    /// re-parse (called after a load / reload).
    fn resync_syntax(&mut self) {
        self.syntax.set_language(self.doc.language);
        self.syntax.invalidate();
    }

    pub fn focus(&self, window: &mut Window) {
        window.focus(&self.focus_handle);
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.doc.path.clone()
    }

    /// Capture only editor-owned state. Workspace persists the returned value
    /// as an opaque typed child of its tab snapshot.
    pub fn session_snapshot(&self) -> EditorSessionSnapshot {
        let path = self
            .doc
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let id_value = path.clone().unwrap_or_else(|| "untitled".to_string());
        let id = EditorSessionId::new(id_value).unwrap_or_default();
        let mut snapshot = EditorSessionSnapshot::new(id, path);
        snapshot.language = self.doc.language.label().to_string();
        snapshot.split_tree = self.editor_splits.clone();
        snapshot.active_group = self.editor_splits.focused_group().value();
        let selections = self.doc.selection_set();
        snapshot.views = self
            .editor_splits
            .groups()
            .into_iter()
            .map(|group| EditorViewSession {
                group_id: group.value(),
                scroll_top: self.scroll_top,
                scroll_left: self.scroll_left,
                folds: self.folds.clone(),
                selections: selections.clone(),
            })
            .collect();
        snapshot.search = self.search.as_ref().map(|search| search.query.clone());
        if self.doc.is_dirty() {
            snapshot.recovery =
                self.doc
                    .path
                    .as_ref()
                    .map(|path| labonair_editor::UnsavedBufferRecovery {
                        path: path.to_string_lossy().into_owned(),
                        content: self.doc.text(),
                    });
        }
        snapshot
    }

    /// Queue a session payload from Workspace. It is applied after the next
    /// load, when selections and folds can be clamped against real content.
    pub fn set_pending_session(&mut self, snapshot: EditorSessionSnapshot) {
        self.pending_session = Some(snapshot);
    }

    fn apply_pending_session(&mut self) {
        let Some(snapshot) = self.pending_session.take() else {
            return;
        };
        let path = self
            .doc
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        if snapshot.file_path != path {
            return;
        }
        self.recovery_prompt = snapshot.recovery.clone();
        self.editor_splits = if snapshot.split_tree.validate().is_ok() {
            snapshot.split_tree
        } else {
            // Session data is untrusted at this boundary. Keep the document
            // and its single canonical buffer, but discard only an invalid
            // split layout instead of allowing stale IDs into GPUI handlers.
            labonair_editor::EditorSplitTree::default()
        };
        let _ = self
            .editor_splits
            .focus(labonair_editor::EditorGroupId::new(snapshot.active_group));
        if let Some(view) = snapshot
            .views
            .into_iter()
            .find(|view| view.group_id == self.editor_splits.focused_group().value())
        {
            self.scroll_top = view.scroll_top;
            self.scroll_left = view.scroll_left;
            self.folds = view.folds;
            self.doc.set_selection_set(view.selections);
        }
        self.search = snapshot.search.map(|query| EditorSearch {
            query,
            matches: Vec::new(),
            active: None,
            error: None,
        });
        self.clamp_scroll();
        self.refresh_matches();
    }

    /// 1-based `(line, column)` of the caret — for the statusbar
    /// `cursorPosition` bar item.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        (self.doc.cursor.line + 1, self.doc.cursor.column + 1)
    }

    /// Zero-based primary caret position for typed command composition.
    pub fn cursor_position(&self) -> Position {
        self.doc.cursor
    }

    pub fn is_dirty(&self) -> bool {
        self.doc.is_dirty()
    }

    /// Document symbols for the palette "Go to Symbol" / outline page.
    pub fn document_symbols(&self) -> Vec<labonair_editor::DocumentSymbol> {
        let snapshot = self.doc.snapshot();
        labonair_editor::document_symbols(self.syntax.language(), &snapshot)
    }

    /// Move the caret to the start of `line0` (0-based, clamped) and scroll it
    /// into view — palette symbol jump.
    pub fn goto_line(&mut self, line0: usize, cx: &mut Context<Self>) {
        let max = self.doc.snapshot().line_count().saturating_sub(1);
        let line = line0.min(max);
        self.doc.set_caret(Position::new(line, 0), false);
        self.ensure_cursor_visible();
        cx.notify();
    }

    /// Move the caret to a project-search hit. While a newly opened file is
    /// still loading, retain the position and apply it after the load commits.
    pub fn goto_position(&mut self, position: Position, cx: &mut Context<Self>) {
        if matches!(self.doc.file_state(), FileState::Loading) {
            self.pending_navigation = Some(position);
            return;
        }
        let snapshot = self.doc.snapshot();
        let line = position.line.min(snapshot.line_count().saturating_sub(1));
        let column = position.column.min(snapshot.line_len(line));
        self.doc.set_caret(Position::new(line, column), false);
        self.ensure_cursor_visible();
        cx.notify();
    }

    pub fn title(&self) -> String {
        let base = self
            .doc
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| "Untitled".to_string());
        base
    }

    /// The revision used when requesting a Git gutter refresh.
    pub fn editor_revision(&self) -> Revision {
        self.doc.snapshot().revision()
    }

    fn current_git_path(&self) -> Option<String> {
        self.doc
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
    }

    fn invalidate_git_requests(&mut self) {
        self.git_refresh_generation = self.git_refresh_generation.wrapping_add(1);
        self.git_context_menu = None;
        self.git_discard_confirmation = None;
    }

    fn git_request_is_current(
        &self,
        path: &str,
        revision: Revision,
        session_id: &Option<String>,
        generation: u64,
    ) -> bool {
        self.git_refresh_generation == generation
            && self.editor_revision() == revision
            && self.current_git_path().as_deref() == Some(path)
            && self.git_session_id.as_ref() == session_id.as_ref()
    }

    fn git_snapshot_is_current(&self, snapshot: &GitGutterSnapshot) -> bool {
        snapshot.is_current(self.editor_revision())
            && self.current_git_path().as_deref() == Some(snapshot.path.as_str())
    }

    /// Request the current Git projection without doing Git work on the GPUI
    /// foreground thread. Every request carries the document path, revision,
    /// session and view generation so late results are discarded completely.
    fn refresh_git(&mut self, cx: &mut Context<Self>) {
        self.invalidate_git_requests();
        let generation = self.git_refresh_generation;
        let revision = self.editor_revision();
        let session_id = self.git_session_id.clone();
        let path = self.current_git_path();

        let Some(path) = path else {
            self.git_gutter = GitGutterSnapshot::unavailable("", revision);
            cx.notify();
            return;
        };
        let Some(provider) = self.git_provider.clone() else {
            self.git_gutter = GitGutterSnapshot::unavailable(path, revision);
            cx.notify();
            return;
        };

        self.git_gutter = GitGutterSnapshot::loading(path.clone(), revision);
        cx.notify();
        let request = GitDecorationRequest {
            path: path.clone(),
            revision,
            session_id: session_id.clone(),
        };
        let refresh = provider.refresh(request);
        cx.spawn(async move |this, cx| {
            let result = refresh.await;
            let _ = this.update(cx, |this, cx| {
                if !this.git_request_is_current(&path, revision, &session_id, generation) {
                    return;
                }
                match result {
                    Ok(snapshot) if snapshot.path == path && snapshot.revision == revision => {
                        this.git_gutter = snapshot;
                        cx.notify();
                    }
                    Ok(_) => {}
                    Err(error) => {
                        this.git_gutter =
                            GitGutterSnapshot::error(path.clone(), revision, error.to_string());
                        notify(cx, "Git gutter refresh failed", &error.to_string());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    pub fn git_gutter_snapshot(&self) -> &GitGutterSnapshot {
        &self.git_gutter
    }

    pub fn set_git_gutter_loading(&mut self, cx: &mut Context<Self>) {
        self.invalidate_git_requests();
        let path = self.current_git_path().unwrap_or_default();
        self.git_gutter = GitGutterSnapshot::loading(path, self.editor_revision());
        cx.notify();
    }

    /// Apply a provider result only when it still belongs to this document's
    /// current revision. Late Git responses are dropped instead of painting
    /// misleading decorations on a newer buffer.
    pub fn set_git_gutter_snapshot(
        &mut self,
        snapshot: GitGutterSnapshot,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.git_snapshot_is_current(&snapshot) {
            return false;
        }
        self.git_gutter = snapshot;
        cx.notify();
        true
    }

    pub fn set_git_gutter_error(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        let path = self.current_git_path().unwrap_or_default();
        self.git_gutter = GitGutterSnapshot::error(path, self.editor_revision(), message);
        cx.notify();
    }

    pub fn set_git_gutter_no_repository(&mut self, cx: &mut Context<Self>) {
        let path = self.current_git_path().unwrap_or_default();
        self.git_gutter = GitGutterSnapshot::no_repository(path, self.editor_revision());
        cx.notify();
    }

    pub fn git_next_change(&self, forward: bool) -> Option<Position> {
        if !self.git_snapshot_is_current(&self.git_gutter) {
            return None;
        }
        self.git_gutter
            .next_change_line(self.doc.cursor.line, forward, self.editor_revision())
            .map(|line| Position::new(line, 0))
    }

    pub fn git_hunk_action(
        &self,
        hunk_id: &str,
        kind: GitHunkActionKind,
    ) -> Result<GitHunkAction, GitGutterError> {
        if !self.git_snapshot_is_current(&self.git_gutter) {
            return Err(GitGutterError::Provider(
                "Git snapshot belongs to another editor document".to_string(),
            ));
        }
        self.git_gutter
            .action(hunk_id, self.editor_revision(), kind)
    }

    /// Execute the Editor-owned Git navigation commands. Repository state and
    /// Project Diff remain behind the injected Workspace bridge.
    pub fn execute_git_command(&mut self, command: GitEditorCommand, cx: &mut Context<Self>) {
        match command {
            GitEditorCommand::NextChange => self.navigate_git_change(true, cx),
            GitEditorCommand::PreviousChange => self.navigate_git_change(false, cx),
            GitEditorCommand::OpenProjectDiff => {
                let Some(hunk_id) = self.git_gutter.hunks.first().map(|hunk| hunk.id.clone())
                else {
                    notify_info(
                        cx,
                        "Project Diff",
                        "No Git change is available for this file.",
                    );
                    return;
                };
                self.submit_git_hunk_action(hunk_id, GitHunkActionKind::OpenProjectDiff, cx);
            }
        }
    }

    fn navigate_git_change(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(position) = self.git_next_change(forward) else {
            notify_info(
                cx,
                "Git changes",
                "No Git change is available for this file.",
            );
            return;
        };
        self.doc.set_caret(position, false);
        self.ensure_cursor_visible();
        cx.emit(EditorEvent::Changed);
        cx.notify();
    }

    fn submit_git_action(&mut self, action: GitHunkAction, cx: &mut Context<Self>) {
        let Some(sink) = self.git_action_sink.clone() else {
            notify(cx, "Git action failed", "No Git action sink is available.");
            return;
        };
        let Some(path) = self.current_git_path() else {
            notify(cx, "Git action failed", "This document has no file path.");
            return;
        };
        let revision = self.editor_revision();
        let session_id = self.git_session_id.clone();
        let generation = self.git_refresh_generation;
        let refresh_after = matches!(
            &action,
            GitHunkAction::Stage { .. } | GitHunkAction::Unstage { .. }
        );
        let result = sink.submit(action);
        // Workspace drains review intents from the bridge during its render
        // pass. Wake that composition root through the existing editor event
        // channel after the typed sink accepted the intent.
        cx.emit(EditorEvent::Changed);
        cx.spawn(async move |this, cx| {
            let result = result.await;
            let _ = this.update(cx, |this, cx| {
                if !this.git_request_is_current(&path, revision, &session_id, generation) {
                    return;
                }
                match result {
                    Ok(()) if refresh_after => this.refresh_git(cx),
                    Ok(()) => cx.notify(),
                    Err(error) => notify(cx, "Git action failed", &error.to_string()),
                }
            });
        })
        .detach();
    }

    fn submit_git_hunk_action(
        &mut self,
        hunk_id: String,
        kind: GitHunkActionKind,
        cx: &mut Context<Self>,
    ) {
        match self.git_hunk_action(&hunk_id, kind) {
            Ok(action) => self.submit_git_action(action, cx),
            Err(error) => notify(cx, "Git action failed", &error.to_string()),
        }
    }

    // ── Loading ─────────────────────────────────────────────────────────────

    fn editor_identity(identity: StorageFileIdentity) -> labonair_editor::lifecycle::FileIdentity {
        labonair_editor::lifecycle::FileIdentity::new(
            identity.path,
            identity.size,
            identity.modified_ms,
            identity.content_hash,
        )
    }

    fn editor_capabilities(capabilities: StorageFileCapabilities) -> FileCapabilities {
        FileCapabilities {
            can_read: capabilities.can_read,
            can_edit: capabilities.can_edit,
            can_save: capabilities.can_save,
        }
    }

    fn editor_line_ending(ending: StorageLineEnding) -> labonair_editor::lifecycle::LineEnding {
        match ending {
            StorageLineEnding::Lf => labonair_editor::lifecycle::LineEnding::Lf,
            StorageLineEnding::CrLf => labonair_editor::lifecycle::LineEnding::CrLf,
            StorageLineEnding::Cr => labonair_editor::lifecycle::LineEnding::Cr,
            StorageLineEnding::Mixed => labonair_editor::lifecycle::LineEnding::Mixed,
        }
    }

    fn storage_line_ending(ending: labonair_editor::lifecycle::LineEnding) -> StorageLineEnding {
        match ending {
            labonair_editor::lifecycle::LineEnding::Lf => StorageLineEnding::Lf,
            labonair_editor::lifecycle::LineEnding::CrLf => StorageLineEnding::CrLf,
            labonair_editor::lifecycle::LineEnding::Cr => StorageLineEnding::Cr,
            labonair_editor::lifecycle::LineEnding::Mixed => StorageLineEnding::Mixed,
        }
    }

    fn storage_bom(bom: labonair_editor::lifecycle::Bom) -> StorageBom {
        match bom {
            labonair_editor::lifecycle::Bom::None => StorageBom::None,
            labonair_editor::lifecycle::Bom::Utf8 => StorageBom::Utf8,
        }
    }

    fn storage_identity(identity: labonair_editor::lifecycle::FileIdentity) -> StorageFileIdentity {
        StorageFileIdentity {
            path: identity.path,
            size: identity.size,
            modified_ms: identity.modified_ms,
            content_hash: identity.content_hash,
        }
    }

    fn editor_snapshot(snapshot: StorageFileSnapshot) -> labonair_editor::lifecycle::FileSnapshot {
        labonair_editor::lifecycle::FileSnapshot::new(
            Self::editor_identity(snapshot.identity),
            snapshot.text,
            Self::editor_line_ending(snapshot.line_ending),
            labonair_editor::lifecycle::Encoding::Utf8,
            match snapshot.bom {
                StorageBom::None => labonair_editor::lifecycle::Bom::None,
                StorageBom::Utf8 => labonair_editor::lifecycle::Bom::Utf8,
            },
            Self::editor_capabilities(snapshot.capabilities),
        )
    }

    fn document_from_load(
        path: PathBuf,
        result: Result<EditorLifecycleLoad, EditorFileError>,
    ) -> Document {
        match result {
            Ok(EditorLifecycleLoad::Text { snapshot }) => {
                Document::from_file_snapshot(Self::editor_snapshot(snapshot))
            }
            Ok(EditorLifecycleLoad::Binary {
                identity,
                capabilities,
            }) => Document::from_uneditable(
                path,
                FileState::Binary,
                Some(Self::editor_identity(identity)),
                Self::editor_capabilities(capabilities),
            ),
            Ok(EditorLifecycleLoad::TooLarge {
                identity,
                capabilities,
                ..
            }) => Document::from_uneditable(
                path,
                FileState::TooLarge,
                Some(Self::editor_identity(identity)),
                Self::editor_capabilities(capabilities),
            ),
            Ok(EditorLifecycleLoad::UnsupportedEncoding {
                identity,
                encoding,
                capabilities,
            }) => Document::from_uneditable(
                path,
                FileState::Error(format!("Unsupported {encoding} encoding")),
                Some(Self::editor_identity(identity)),
                Self::editor_capabilities(capabilities),
            ),
            Ok(EditorLifecycleLoad::InvalidUtf8 {
                identity,
                capabilities,
            }) => Document::from_uneditable(
                path,
                FileState::Error("The file is not valid UTF-8".to_string()),
                Some(Self::editor_identity(identity)),
                Self::editor_capabilities(capabilities),
            ),
            Err(EditorFileError::Missing { .. }) => Document::from_uneditable(
                path,
                FileState::Missing,
                None,
                FileCapabilities::unavailable(),
            ),
            Err(EditorFileError::Binary { .. }) => Document::from_uneditable(
                path,
                FileState::Binary,
                None,
                FileCapabilities::unavailable(),
            ),
            Err(EditorFileError::TooLarge { .. }) => Document::from_uneditable(
                path,
                FileState::TooLarge,
                None,
                FileCapabilities::unavailable(),
            ),
            Err(error @ EditorFileError::UnsupportedEncoding { .. })
            | Err(error @ EditorFileError::InvalidUtf8 { .. })
            | Err(error @ EditorFileError::Io { .. })
            | Err(error @ EditorFileError::ReadOnly { .. })
            | Err(error @ EditorFileError::ExternalChange { .. }) => Document::from_uneditable(
                path,
                FileState::Error(error.to_string()),
                None,
                FileCapabilities::unavailable(),
            ),
        }
    }

    /// Load `path` into this view, replacing any current document.
    pub fn open_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.close_language_service_document();
        self.pending_navigation = None;
        self.doc = Document::loading(path.clone());
        self.recovery_prompt = None;
        self.invalidate_git_requests();
        self.git_gutter =
            GitGutterSnapshot::loading(path.to_string_lossy().to_string(), self.editor_revision());
        self.scroll_top = 0;
        let path_str = path.to_string_lossy().to_string();
        let max_bytes = self.file_limit_bytes();
        let load = cx
            .background_executor()
            .spawn(async move { load_editor_file_lifecycle_sync(&path_str, Some(max_bytes)) });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |this, cx| {
                let failed = result.is_err();
                this.doc = Self::document_from_load(path, result);
                if !failed {
                    this.apply_pending_session();
                    this.sync_language_service_document(cx);
                    if let Some(position) = this.pending_navigation.take() {
                        this.goto_position(position, cx);
                    }
                }
                if failed {
                    if let FileState::Error(message) = this.doc.file_state() {
                        notify(cx, "Open file failed", message);
                    } else if matches!(this.doc.file_state(), FileState::Missing) {
                        notify(cx, "Open file failed", "The file no longer exists.");
                    }
                }
                this.resync_syntax();
                this.refresh_git(cx);
                cx.emit(EditorEvent::Changed);
                cx.notify();
            });
        })
        .detach();
    }

    /// Re-stat the file; auto-reload if we have no unsaved edits, otherwise flag
    /// the conflict for the reload banner. Called when the tab is (re)activated.
    pub fn check_external(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let stat = cx
            .background_executor()
            .spawn(async move { stat_editor_file_sync(&path_str) });
        cx.spawn(async move |this, cx| {
            let result = stat.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(EditorFileStat { identity, .. }) => {
                    match this.doc.observe_file(Some(Self::editor_identity(identity))) {
                        Ok(Some(FileLifecycleEvent::ReloadNeeded { .. })) => {
                            this.reload_from_disk(cx);
                        }
                        Ok(Some(FileLifecycleEvent::ConflictDetected { .. })) => {
                            this.refresh_git(cx);
                            cx.emit(EditorEvent::Changed);
                            cx.notify();
                        }
                        Ok(_) => {}
                        Err(error) => {
                            notify(cx, "File change detection failed", &error.to_string());
                            this.refresh_git(cx);
                        }
                    }
                }
                Err(EditorFileError::Missing { .. }) => {
                    if let Err(error) = this.doc.observe_file(None) {
                        notify(cx, "File change detection failed", &error.to_string());
                    } else {
                        this.refresh_git(cx);
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
                Err(error) => {
                    let message = error.to_string();
                    if let Err(lifecycle_error) =
                        this.doc.fail_file(FileState::Error(message.clone()))
                    {
                        notify(
                            cx,
                            "File change detection failed",
                            &lifecycle_error.to_string(),
                        );
                    } else {
                        notify(cx, "File change detection failed", &message);
                        this.refresh_git(cx);
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            return;
        };
        self.invalidate_git_requests();
        self.git_gutter =
            GitGutterSnapshot::loading(path.to_string_lossy().to_string(), self.editor_revision());
        let path_str = path.to_string_lossy().to_string();
        let max_bytes = self.file_limit_bytes();
        let load = cx
            .background_executor()
            .spawn(async move { load_editor_file_lifecycle_sync(&path_str, Some(max_bytes)) });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |this, cx| {
                let decision = if matches!(this.doc.file_state(), FileState::Conflict) {
                    ReloadDecision::DiscardBuffer
                } else {
                    ReloadDecision::Automatic
                };
                match result {
                    Ok(EditorLifecycleLoad::Text { snapshot }) => {
                        if let Err(error) = this
                            .doc
                            .reload_snapshot(Self::editor_snapshot(snapshot), decision)
                        {
                            notify(cx, "Reload failed", &error.to_string());
                        } else {
                            this.clamp_scroll();
                            this.resync_syntax();
                            this.sync_language_service_document(cx);
                            this.refresh_git(cx);
                            cx.emit(EditorEvent::Changed);
                            cx.notify();
                        }
                    }
                    other => {
                        this.close_language_service_document();
                        let path = this.doc.path.clone().unwrap_or_default();
                        this.doc = Self::document_from_load(path, other);
                        this.clamp_scroll();
                        this.resync_syntax();
                        this.refresh_git(cx);
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    // ── Saving ──────────────────────────────────────────────────────────────

    pub fn save(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.doc.path.clone() else {
            notify(cx, "Save", "This document has no file path.");
            return;
        };
        let Some(file_snapshot) = self.doc.file_snapshot().cloned() else {
            notify(
                cx,
                "Save failed",
                "This document has no accepted file snapshot.",
            );
            return;
        };
        if let Err(error) = self
            .doc
            .prepare_save(&file_snapshot.identity, SaveIntent::Normal)
        {
            notify(cx, "Save failed", &error.to_string());
            return;
        }
        if self.prefs.format_on_save() && !self.format_on_save_in_flight {
            self.apply_save_policy();
            self.format_on_save_in_flight = true;
            self.request_language_service(LanguageServiceRequest::Formatting, cx);
            return;
        }
        self.format_on_save_in_flight = false;
        let path_str = path.to_string_lossy().to_string();
        let revision_before_policy = self.editor_revision();
        self.apply_save_policy();
        if self.editor_revision() != revision_before_policy {
            self.refresh_git(cx);
        }
        let content = self.doc.text();
        let expected = Self::storage_identity(file_snapshot.identity.clone());
        let line_ending = Self::storage_line_ending(file_snapshot.line_ending);
        let bom = Self::storage_bom(file_snapshot.bom);
        let write = cx.background_executor().spawn(async move {
            let current = stat_editor_file_sync(&path_str)?;
            if current.identity != expected {
                return Err(EditorFileError::ExternalChange {
                    expected,
                    actual: current.identity,
                });
            }
            save_editor_file_lifecycle_sync(
                &path_str,
                &content,
                line_ending,
                bom,
                &current.identity,
                StorageSaveIntent::Normal,
            )
        });
        cx.spawn(async move |this, cx| {
            let result = write.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(snapshot) => match this.doc.accept_saved(Self::editor_snapshot(snapshot)) {
                    Ok(FileLifecycleEvent::Saved { .. }) => {
                        this.refresh_git(cx);
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                    Ok(_) => {}
                    Err(error) => notify(cx, "Save failed", &error.to_string()),
                },
                Err(EditorFileError::ExternalChange { actual, .. }) => {
                    if let Err(error) = this.doc.observe_file(Some(Self::editor_identity(actual))) {
                        notify(cx, "Save failed", &error.to_string());
                    } else {
                        this.refresh_git(cx);
                        notify(
                            cx,
                            "Save failed",
                            "The file changed on disk; review the conflict before saving again.",
                        );
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
                Err(EditorFileError::Missing { .. }) => {
                    if let Err(error) = this.doc.observe_file(None) {
                        notify(cx, "Save failed", &error.to_string());
                    } else {
                        this.refresh_git(cx);
                        notify(cx, "Save failed", "The file no longer exists.");
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
                Err(EditorFileError::ReadOnly { .. }) => {
                    notify(cx, "Save failed", "The file is read-only.");
                }
                Err(error) => {
                    let message = error.to_string();
                    if let Err(lifecycle_error) =
                        this.doc.fail_file(FileState::Error(message.clone()))
                    {
                        notify(cx, "Save failed", &lifecycle_error.to_string());
                    } else {
                        notify(cx, "Save failed", &message);
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn save_policy(&self) -> SavePolicy {
        SavePolicy {
            auto_save: self.prefs.auto_save(),
            auto_save_delay_ms: self.prefs.auto_save_delay_ms(),
            format_on_save: self.prefs.format_on_save(),
            trim_trailing_whitespace: self.prefs.trim_trailing_whitespace(),
            insert_final_newline: self.prefs.insert_final_newline(),
        }
    }

    /// Apply the configured local save normalizer to the canonical buffer so
    /// the accepted disk baseline and the in-memory text stay identical.
    fn apply_save_policy(&mut self) {
        let policy = self.save_policy();
        if !(policy.format_on_save
            || policy.trim_trailing_whitespace
            || policy.insert_final_newline)
        {
            return;
        }
        let current = self.doc.text();
        let next = apply_text_policies(&current, policy);
        if next != current {
            let end = self.doc.snapshot().byte_len();
            let selections = self.doc.selection_set();
            let _ = self.doc.apply_edits(
                vec![Edit::new(TextRange::new(0, end), next)],
                selections,
                labonair_editor::EditSource::Format,
                Some("Apply save policies".to_string()),
            );
        }
    }

    // ── Editing helpers ─────────────────────────────────────────────────────

    fn edit(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Document)) {
        self.cursor_blink_on = true;
        let revision_before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        f(&mut self.doc);
        self.ensure_cursor_visible();
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if !dirty_before && self.doc.is_dirty() {
            cx.emit(EditorEvent::Edited);
        }
        if self.editor_revision() != revision_before {
            self.sync_language_service_document(cx);
            self.refresh_git(cx);
            if let Some(trigger) = self.signature_help_trigger() {
                self.request_signature_help(Some(trigger), cx);
            }
        }
        self.schedule_autosave(cx);
        cx.notify();
    }

    /// Debounce autosave by replacing the previous delayed task. The task
    /// re-reads the setting and dirty state at execution time, so changing a
    /// setting or resolving a conflict cancels the pending write safely.
    fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
        if !self.prefs.auto_save() || !self.doc.is_dirty() || self.doc.path.is_none() {
            self.autosave = None;
            return;
        }
        let delay = self.prefs.auto_save_delay_ms();
        self.autosave = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(delay))
                .await;
            let _ = view.update(cx, |this, cx| {
                if this.prefs.auto_save() && this.doc.is_dirty() {
                    this.save(cx);
                }
            });
        }));
    }

    fn navigate(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        self.cursor_blink_on = true;
        self.doc.move_caret(motion, extend);
        self.dismiss_language_popovers();
        self.ensure_cursor_visible();
        cx.notify();
    }

    /// The current selection text, if any (for "Ask AI about Selection").
    pub fn selected_text(&self) -> Option<String> {
        self.doc.selected_text().filter(|s| !s.is_empty())
    }

    fn copy(&self, cx: &mut Context<Self>) {
        if let Some(text) = self.doc.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.doc.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.edit(cx, |d| d.backspace());
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) {
            if !text.is_empty() {
                self.edit(cx, |d| d.insert(&text));
            }
        }
    }

    // ── Scrolling ───────────────────────────────────────────────────────────

    fn visible_rows(&self) -> usize {
        let h = self
            .bounds
            .map(|b| f32::from(b.size.height))
            .unwrap_or(600.0);
        ((h / self.metrics.1).floor() as usize).max(1)
    }

    fn clamp_scroll(&mut self) {
        let snapshot = self.doc.snapshot();
        self.scroll_top = self.scroll_top.min(self.max_scroll_line());
        let max_left = if self.wrap_cols() == 0 {
            (0..snapshot.line_count())
                .map(|line| snapshot.line_len(line))
                .max()
                .unwrap_or(0)
                .saturating_sub(self.content_width_columns())
        } else {
            0
        };
        self.scroll_left = self.scroll_left.min(max_left);
    }

    fn max_scroll_line(&self) -> usize {
        let snapshot = self.doc.snapshot();
        if self.prefs.scroll_beyond_last_line() {
            return snapshot.line_count().saturating_sub(1);
        }
        let full = DisplayMap::build(
            &snapshot,
            Viewport {
                height_rows: self.visible_rows(),
                width_columns: self.wrap_cols().max(self.content_width_columns()).max(1),
                ..Default::default()
            },
            self.display_config(),
            &self.folds,
        );
        full.display_to_position(full.max_scroll_row(), 0)
            .map(|position| position.line)
            .unwrap_or_else(|| snapshot.line_count().saturating_sub(1))
    }

    fn ensure_cursor_visible(&mut self) {
        let rows = self.visible_rows();
        let line = self.doc.cursor.line;
        if line < self.scroll_top {
            self.scroll_top = line;
        } else if line >= self.scroll_top + rows {
            self.scroll_top = line + 1 - rows;
        }
        self.clamp_scroll();
    }

    fn scroll_by(&mut self, lines: isize, cx: &mut Context<Self>) {
        let max = self.max_scroll_line() as isize;
        let next = (self.scroll_top as isize + lines).clamp(0, max);
        if next as usize != self.scroll_top {
            self.scroll_top = next as usize;
            cx.notify();
        }
    }

    fn scroll_to_visual_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let snapshot = self.doc.snapshot();
        let full = DisplayMap::build(
            &snapshot,
            Viewport {
                height_rows: snapshot.line_count().saturating_add(1),
                ..Default::default()
            },
            self.display_config(),
            &self.folds,
        );
        let next = full
            .display_to_position(row.min(full.total_rows.saturating_sub(1)), 0)
            .map(|position| position.line)
            .unwrap_or_else(|| snapshot.line_count().saturating_sub(1));
        if next != self.scroll_top {
            self.scroll_top = next;
            self.clamp_scroll();
            cx.notify();
        }
    }

    fn set_scrollbar_position(
        &mut self,
        axis: ScrollbarAxis,
        pointer: f32,
        track_start: f32,
        geometry: ScrollbarGeometry,
        cx: &mut Context<Self>,
    ) {
        let next = scrollbar_scroll_from_pointer(pointer, track_start, geometry);
        match axis {
            ScrollbarAxis::Vertical => self.scroll_to_visual_row(next, cx),
            ScrollbarAxis::Horizontal => {
                if next != self.scroll_left {
                    self.scroll_left = next;
                    cx.notify();
                }
            }
        }
    }

    fn toggle_fold_range(&mut self, range: FoldRange, cx: &mut Context<Self>) {
        let _expanded = !toggle_fold(&mut self.folds, range);
        self.clamp_scroll();
        cx.notify();
    }

    // ── Find (driven by the workspace search overlay, T18-002) ──────────────

    /// A one-line pre-fill for the search overlay (current selection).
    pub fn search_seed(&self) -> Option<String> {
        self.doc
            .selected_text()
            .filter(|s| !s.is_empty() && !s.contains('\n'))
    }

    /// Start / update the Editor-owned search contract. Workspace only forwards
    /// this typed query from its overlay; it never constructs match state.
    pub fn search_set(
        &mut self,
        query: SearchQuery,
        cx: &mut Context<Self>,
    ) -> Result<(usize, usize), SearchError> {
        self.project_search.cancel();
        if query.text.is_empty() {
            self.search = None;
            cx.notify();
            return Ok((0, 0));
        }
        let snapshot = self.doc.snapshot();
        let result = match find(&snapshot, &query) {
            Ok(result) => result,
            Err(error) => {
                self.search = Some(EditorSearch {
                    query,
                    matches: Vec::new(),
                    active: None,
                    error: Some(error.clone()),
                });
                cx.notify();
                return Err(error);
            }
        };
        let active = next_match_with_wrap(&result.matches, self.doc.cursor, true, query.wrap);
        self.search = Some(EditorSearch {
            query,
            matches: result.matches,
            active,
            error: None,
        });
        if let Some(active) = active {
            self.select_search_match(active);
        }
        cx.notify();
        Ok(self.search_count())
    }

    /// Step to the next / previous match (wrapping). Returns `(current, total)`.
    pub fn search_step(
        &mut self,
        forward: bool,
        cx: &mut Context<Self>,
    ) -> Result<(usize, usize), SearchError> {
        let Some(s) = &self.search else {
            return Ok((0, 0));
        };
        if let Some(error) = &s.error {
            return Err(error.clone());
        };
        if s.matches.is_empty() {
            return Ok((0, 0));
        }
        let current = s.active.unwrap_or(0);
        let wrap = s.query.wrap;
        let len = s.matches.len();
        let Some(idx) = (if forward {
            (current + 1 < len)
                .then_some(current + 1)
                .or_else(|| wrap.then_some(0))
        } else {
            (current > 0)
                .then_some(current - 1)
                .or_else(|| wrap.then_some(len - 1))
        }) else {
            return Ok(self.search_count());
        };
        self.select_search_match(idx);
        cx.notify();
        Ok(self.search_count())
    }

    /// Close the search / clear the match state.
    pub fn search_close(&mut self, cx: &mut Context<Self>) {
        self.search = None;
        self.project_search.cancel();
        cx.notify();
    }

    /// Begin an asynchronous project search. The Editor allocates the
    /// generation and owns the loading state; Workspace only executes the
    /// returned request through its filesystem adapter.
    pub fn begin_project_search(
        &mut self,
        query: ProjectSearchQuery,
        cx: &mut Context<Self>,
    ) -> ProjectSearchRequest {
        self.search = None;
        let request = self.project_search.begin(query);
        cx.notify();
        request
    }

    pub fn accept_project_search(
        &mut self,
        result: ProjectSearchResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let accepted = self.project_search.accept(result);
        if accepted {
            cx.notify();
        }
        accepted
    }

    pub fn fail_project_search(
        &mut self,
        generation: u64,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> bool {
        let accepted = self.project_search.fail(generation, message);
        if accepted {
            cx.notify();
        }
        accepted
    }

    pub fn project_search_snapshot(&self) -> ProjectSearchSnapshot {
        self.project_search.snapshot()
    }

    pub fn move_project_search_selection(&mut self, delta: isize) -> Option<ProjectSearchHit> {
        self.project_search.move_selection(delta)
    }

    pub fn selected_project_search_hit(&self) -> Option<ProjectSearchHit> {
        self.project_search.selected_hit().cloned()
    }

    /// `(current_1_based, total)` — `0` current means no match / no search.
    pub fn search_count(&self) -> (usize, usize) {
        match &self.search {
            Some(s) if !s.matches.is_empty() => {
                (s.active.map_or(0, |active| active + 1), s.matches.len())
            }
            Some(s) => (0, s.matches.len()),
            None => (0, 0),
        }
    }

    pub fn search_error(&self) -> Option<&str> {
        self.search
            .as_ref()
            .and_then(|search| search.error.as_ref())
            .map(|error| error.message.as_str())
    }

    pub fn search_can_replace(&self) -> bool {
        self.doc.can_edit()
            && self
                .search
                .as_ref()
                .is_some_and(|search| search.error.is_none() && !search.matches.is_empty())
    }

    /// Replace the active Editor match through the canonical Document
    /// transaction. The Workspace overlay cannot mutate the document itself.
    pub fn search_replace_one(
        &mut self,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Result<bool, SearchError> {
        let Some(search) = &self.search else {
            return Ok(false);
        };
        let Some(active) = search.active else {
            return Ok(false);
        };
        let query = search.query.clone();
        let replaced = self.doc.replace_one(&query, replacement, active)?;
        if replaced {
            self.refresh_matches();
            if let Some(active) = self.search.as_ref().and_then(|search| search.active) {
                self.select_search_match(active);
            }
            cx.notify();
        }
        Ok(replaced)
    }

    /// Replace all Editor matches in one canonical undoable transaction.
    pub fn search_replace_all(
        &mut self,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Result<usize, SearchError> {
        let Some(search) = &self.search else {
            return Ok(0);
        };
        let query = search.query.clone();
        let count = self.doc.replace_all_checked(&query, replacement)?;
        if count > 0 {
            self.refresh_matches();
            if let Some(active) = self.search.as_ref().and_then(|search| search.active) {
                self.select_search_match(active);
            }
            cx.notify();
        }
        Ok(count)
    }

    /// Re-run the active search's matcher after a buffer edit, keeping the
    /// active index in range. No-op when no search is active.
    fn refresh_matches(&mut self) {
        if let Some(s) = &mut self.search {
            let snapshot = self.doc.snapshot();
            match find(&snapshot, &s.query) {
                Ok(result) => {
                    s.matches = result.matches;
                    s.active = if s.matches.is_empty() {
                        None
                    } else {
                        Some(s.active.unwrap_or(0).min(s.matches.len() - 1))
                    };
                    s.error = None;
                }
                Err(error) => {
                    s.matches.clear();
                    s.active = None;
                    s.error = Some(error);
                }
            }
        }
    }

    fn select_search_match(&mut self, idx: usize) {
        let Some(s) = &mut self.search else { return };
        let Some(m) = s.matches.get(idx).copied() else {
            return;
        };
        s.active = Some(idx);
        self.doc.set_caret(m.start, false);
        self.doc.set_caret(m.end, true);
        self.ensure_cursor_visible();
    }

    // ── Key handling ────────────────────────────────────────────────────────

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;

        if self.completion_visible {
            let handled = match ks.key.as_str() {
                "escape" if !m.platform && !m.control && !m.alt => {
                    self.dismiss_completion();
                    cx.notify();
                    true
                }
                "up" if !m.platform && !m.control && !m.alt => {
                    self.move_completion_selection(-1, cx);
                    true
                }
                "down" if !m.platform && !m.control && !m.alt => {
                    self.move_completion_selection(1, cx);
                    true
                }
                "p" if m.control && !m.platform && !m.alt => {
                    self.move_completion_selection(-1, cx);
                    true
                }
                "n" if m.control && !m.platform && !m.alt => {
                    self.move_completion_selection(1, cx);
                    true
                }
                "tab" | "enter" if !m.platform && !m.control && !m.alt => {
                    self.apply_completion_item(self.completion_selected, cx);
                    true
                }
                _ => false,
            };
            if handled {
                cx.stop_propagation();
                return;
            }
        }

        if m.platform && !m.control && !m.alt {
            match ks.key.as_str() {
                "s" => self.save(cx),
                "z" if m.shift => self.edit(cx, Document::redo),
                "z" => self.edit(cx, Document::undo),
                "y" => self.edit(cx, Document::redo),
                "a" => {
                    self.doc.select_all();
                    cx.notify();
                }
                "c" => self.copy(cx),
                "x" => self.cut(cx),
                "v" => self.paste(cx),
                "left" => self.navigate(Motion::LineStart, m.shift, cx),
                "right" => self.navigate(Motion::LineEnd, m.shift, cx),
                "up" => self.navigate(Motion::DocStart, m.shift, cx),
                "down" => self.navigate(Motion::DocEnd, m.shift, cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        if m.alt && !m.platform && !m.control {
            match ks.key.as_str() {
                "left" => self.navigate(Motion::WordLeft, m.shift, cx),
                "right" => self.navigate(Motion::WordRight, m.shift, cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        // Vim mode (T06-003): consume the keystroke in the modal state machine.
        // Insert mode lets non-text keys (arrows, page-up/down) fall through to
        // the regular editor navigation below.
        if self.vim.is_some() {
            if let Some(key) = self.vim_key(ks) {
                self.handle_vim(key, cx);
                cx.stop_propagation();
                return;
            }
        }

        if m.control || m.alt {
            return;
        }

        let rows = self.visible_rows().saturating_sub(1).max(1);
        match ks.key.as_str() {
            "left" => self.navigate(Motion::Left, m.shift, cx),
            "right" => self.navigate(Motion::Right, m.shift, cx),
            "up" => {
                if !self.wrap_vertical(false, m.shift, cx) {
                    self.navigate(Motion::Up, m.shift, cx)
                }
            }
            "down" => {
                if !self.wrap_vertical(true, m.shift, cx) {
                    self.navigate(Motion::Down, m.shift, cx)
                }
            }
            "home" => {
                if !self.wrap_horizontal(false, m.shift, cx) {
                    self.navigate(Motion::LineStart, m.shift, cx)
                }
            }
            "end" => {
                if !self.wrap_horizontal(true, m.shift, cx) {
                    self.navigate(Motion::LineEnd, m.shift, cx)
                }
            }
            "pageup" => self.navigate(Motion::PageUp(rows), m.shift, cx),
            "pagedown" => self.navigate(Motion::PageDown(rows), m.shift, cx),
            "enter" => self.edit(cx, |d| d.insert("\n")),
            "tab" => {
                let indent = self.indent_unit();
                self.edit(cx, move |d| d.insert(&indent));
            }
            "backspace" => self.edit(cx, Document::backspace),
            "delete" => self.edit(cx, Document::delete_forward),
            "escape" => {}
            _ => {
                if let Some(text) = printable(ks) {
                    self.edit(cx, |d| d.insert(&text));
                } else {
                    return;
                }
            }
        }
        cx.stop_propagation();
    }

    // ── Vim mode ────────────────────────────────────────────────────────────

    /// Translate a keystroke into a [`VimKey`], or `None` to let the regular
    /// editor navigation handle it (arrow / paging keys in insert mode).
    fn vim_key(&self, ks: &gpui::Keystroke) -> Option<VimKey> {
        let m = &ks.modifiers;
        let insert = self.vim.as_ref().map(Vim::mode) == Some(VimMode::Insert);
        match ks.key.as_str() {
            "escape" => Some(VimKey::Esc),
            "enter" | "return" => Some(VimKey::Enter),
            "backspace" => Some(VimKey::Backspace),
            "tab" => Some(VimKey::Tab),
            "r" if m.control => Some(VimKey::Redo),
            "left" if !insert => Some(VimKey::Char('h')),
            "right" if !insert => Some(VimKey::Char('l')),
            "up" if !insert => Some(VimKey::Char('k')),
            "down" if !insert => Some(VimKey::Char('j')),
            _ => {
                if m.control || m.platform || m.alt {
                    None
                } else {
                    printable(ks)
                        .and_then(|s| s.chars().next())
                        .map(VimKey::Char)
                }
            }
        }
    }

    fn handle_vim(&mut self, key: VimKey, cx: &mut Context<Self>) {
        let revision_before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let resp = {
            let vim = self.vim.as_mut().expect("vim mode active");
            vim.on_key(&mut self.doc, key)
        };
        if resp.handled {
            self.ensure_cursor_visible();
            self.refresh_matches();
            cx.emit(EditorEvent::Changed);
            if !dirty_before && self.doc.is_dirty() {
                cx.emit(EditorEvent::Edited);
            }
            if self.editor_revision() != revision_before {
                self.sync_language_service_document(cx);
                self.refresh_git(cx);
            }
            self.schedule_autosave(cx);
            cx.notify();
        }
        if resp.save {
            self.save(cx);
        }
        if resp.reload {
            self.reload_from_disk(cx);
        }
        if resp.quit {
            cx.emit(EditorEvent::CloseRequested);
        }
    }

    fn position_at(&self, p: Point<Pixels>) -> Position {
        let origin = self.bounds.map(|b| b.origin).unwrap_or_default();
        let (cw, lh) = self.metrics;
        let x = (f32::from(p.x - origin.x) - self.gutter_width).max(0.0);
        let y = f32::from(p.y - origin.y).max(0.0);
        let col_hint = ((x / cw) + 0.5) as usize + self.scroll_left;
        let cols = self.wrap_cols();
        if cols == 0 {
            return Position::new(self.scroll_top + (y / lh) as usize, col_hint);
        }
        // Walk visual rows from the scroll top to resolve the logical line and
        // which wrapped segment the click landed on (T06-005).
        let wrap = Wrap { cols };
        let snapshot = self.doc.snapshot();
        let count = snapshot.line_count();
        let mut cum = 0.0f32;
        let mut line = self.scroll_top;
        while line < count {
            let segs = wrap.rows(snapshot.line_len(line));
            let h = segs as f32 * lh;
            if y < cum + h || line + 1 == count {
                let seg = (((y - cum) / lh).max(0.0) as usize).min(segs.saturating_sub(1));
                return Position::new(line, seg * cols + col_hint);
            }
            cum += h;
            line += 1;
        }
        Position::new(count.saturating_sub(1), col_hint)
    }

    fn render_scrollbars(
        &self,
        display: &labonair_editor::DisplaySnapshot,
        palette: Palette,
        sticky_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (width, height) = self
            .bounds
            .map(|bounds| (f32::from(bounds.size.width), f32::from(bounds.size.height)))
            .unwrap_or((640.0, 480.0));
        let vertical_track_length = (height - sticky_height - EDITOR_SCROLLBAR_SIZE).max(1.0);
        let horizontal_track_length = (width - self.gutter_width - EDITOR_SCROLLBAR_SIZE).max(1.0);
        let vertical_viewport = if self.prefs.scroll_beyond_last_line() {
            1
        } else {
            display.viewport.height_rows.max(1)
        };
        let vertical_scroll = display
            .viewport
            .top_row
            .min(display.total_rows.saturating_sub(vertical_viewport));
        let vertical = scrollbar_geometry(
            display.total_rows,
            vertical_viewport,
            vertical_scroll,
            vertical_track_length,
        );
        let horizontal = if self.wrap_cols() == 0 {
            scrollbar_geometry(
                display.max_line_width,
                display.viewport.width_columns.max(1),
                self.scroll_left,
                horizontal_track_length,
            )
        } else {
            None
        };
        let mut vertical_track = div()
            .id("editor-scrollbar-vertical")
            .absolute()
            .top(px(sticky_height))
            .right_0()
            .bottom(px(EDITOR_SCROLLBAR_SIZE))
            .w(px(EDITOR_SCROLLBAR_SIZE))
            .bg(palette.muted_bg.opacity(0.25));
        if let Some(geometry) = vertical {
            let drag = EditorScrollbarDrag {
                axis: ScrollbarAxis::Vertical,
                geometry,
            };
            let thumb = div()
                .id("editor-scrollbar-vertical-thumb")
                .absolute()
                .top(px(geometry.thumb_offset))
                .left_0()
                .right_0()
                .h(px(geometry.thumb_length))
                .rounded(px(EDITOR_SCROLLBAR_SIZE / 2.0))
                .bg(palette.muted.opacity(0.75))
                .hover(|style| style.bg(palette.accent))
                .tooltip(|window, cx| Tooltip::new("Scroll vertically").build(window, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                    }),
                )
                .on_drag(drag, |_, _, _, cx| cx.new(|_| crate::DragGhost));
            vertical_track = vertical_track.child(thumb);
        }
        let vertical_geometry = vertical;
        vertical_track = vertical_track
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    if let Some(geometry) = vertical_geometry {
                        let start = this
                            .bounds
                            .map(|bounds| f32::from(bounds.origin.y) + sticky_height)
                            .unwrap_or(sticky_height);
                        this.set_scrollbar_position(
                            ScrollbarAxis::Vertical,
                            f32::from(event.position.y),
                            start,
                            geometry,
                            cx,
                        );
                        cx.stop_propagation();
                    }
                }),
            )
            .on_drag_move(cx.listener(
                move |this, event: &gpui::DragMoveEvent<EditorScrollbarDrag>, _window, cx| {
                    let drag = event.drag(cx);
                    if drag.axis != ScrollbarAxis::Vertical {
                        return;
                    }
                    let start = f32::from(event.bounds.origin.y);
                    this.set_scrollbar_position(
                        ScrollbarAxis::Vertical,
                        f32::from(event.event.position.y),
                        start,
                        drag.geometry,
                        cx,
                    );
                },
            ));

        let mut horizontal_track = div()
            .id("editor-scrollbar-horizontal")
            .absolute()
            .left(px(self.gutter_width))
            .right(px(EDITOR_SCROLLBAR_SIZE))
            .bottom_0()
            .h(px(EDITOR_SCROLLBAR_SIZE))
            .bg(palette.muted_bg.opacity(0.25));
        if let Some(geometry) = horizontal {
            let drag = EditorScrollbarDrag {
                axis: ScrollbarAxis::Horizontal,
                geometry,
            };
            let thumb = div()
                .id("editor-scrollbar-horizontal-thumb")
                .absolute()
                .left(px(geometry.thumb_offset))
                .top_0()
                .bottom_0()
                .w(px(geometry.thumb_length))
                .rounded(px(EDITOR_SCROLLBAR_SIZE / 2.0))
                .bg(palette.muted.opacity(0.75))
                .hover(|style| style.bg(palette.accent))
                .tooltip(|window, cx| Tooltip::new("Scroll horizontally").build(window, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _: &MouseDownEvent, _window, cx| {
                        cx.stop_propagation();
                    }),
                )
                .on_drag(drag, |_, _, _, cx| cx.new(|_| crate::DragGhost));
            horizontal_track = horizontal_track.child(thumb);
        }
        let horizontal_geometry = horizontal;
        horizontal_track = horizontal_track
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    if let Some(geometry) = horizontal_geometry {
                        let start = this
                            .bounds
                            .map(|bounds| f32::from(bounds.origin.x) + this.gutter_width)
                            .unwrap_or(this.gutter_width);
                        this.set_scrollbar_position(
                            ScrollbarAxis::Horizontal,
                            f32::from(event.position.x),
                            start,
                            geometry,
                            cx,
                        );
                        cx.stop_propagation();
                    }
                }),
            )
            .on_drag_move(cx.listener(
                move |this, event: &gpui::DragMoveEvent<EditorScrollbarDrag>, _window, cx| {
                    let drag = event.drag(cx);
                    if drag.axis != ScrollbarAxis::Horizontal {
                        return;
                    }
                    let start = f32::from(event.bounds.origin.x);
                    this.set_scrollbar_position(
                        ScrollbarAxis::Horizontal,
                        f32::from(event.event.position.x),
                        start,
                        drag.geometry,
                        cx,
                    );
                },
            ));
        div()
            .absolute()
            .inset_0()
            .child(vertical_track)
            .child(horizontal_track)
            .into_any_element()
    }

    fn render_git_context_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (position, hunk_id, revision) = self.git_context_menu.clone()?;
        if revision != self.editor_revision()
            || !self.git_snapshot_is_current(&self.git_gutter)
            || self.git_gutter.hunk(&hunk_id, revision).is_err()
        {
            return None;
        }
        let hunk = self.git_gutter.hunk(&hunk_id, revision).ok()?;

        let view = cx.entity();
        let dismiss_view = view.clone();
        let dismiss = move |_window: &mut Window, cx: &mut App| {
            dismiss_view.update(cx, |this, cx| {
                this.git_context_menu = None;
                cx.notify();
            });
        };
        let staged = self.git_gutter.staged;
        let (action_kind, action_label) = if staged {
            (GitHunkActionKind::Unstage, "Unstage Hunk")
        } else {
            (GitHunkActionKind::Stage, "Stage Hunk")
        };
        let action_view = view.clone();
        let action_hunk = hunk_id.clone();
        let mut items = vec![
            MenuItem::label(format!(
                "{}  ·  {}",
                hunk.status_label(),
                hunk.line_range_label()
            )),
            MenuItem::label(hunk.summary()),
            MenuItem::label(format!(
                "Patch: {}",
                hunk.patch_preview(3).replace('\n', "  ·  ")
            )),
            MenuItem::separator(),
            MenuItem::new("editor-git-stage-hunk", action_label).on_click(move |_, _, cx| {
                let hunk_id = action_hunk.clone();
                action_view.update(cx, |this, cx| {
                    this.git_context_menu = None;
                    this.submit_git_hunk_action(hunk_id, action_kind, cx);
                });
            }),
            MenuItem::new("editor-git-project-diff", "Open Project Diff").on_click({
                let view = view.clone();
                let hunk_id = hunk_id.clone();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.git_context_menu = None;
                        this.submit_git_hunk_action(
                            hunk_id.clone(),
                            GitHunkActionKind::OpenProjectDiff,
                            cx,
                        );
                    });
                }
            }),
        ];

        if let Ok(GitHunkAction::Discard { preview }) =
            self.git_hunk_action(&hunk_id, GitHunkActionKind::Discard)
        {
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("editor-git-discard-hunk", "Discard Hunk…")
                    .destructive()
                    .on_click({
                        let view = view.clone();
                        move |_, _, cx| {
                            let preview = preview.clone();
                            view.update(cx, |this, cx| {
                                this.git_context_menu = None;
                                this.git_discard_confirmation = Some(preview);
                                cx.notify();
                            });
                        }
                    }),
            );
        }

        Some(context_menu(
            position,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        ))
    }

    fn render_text_context_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let position = self.text_context_menu?;
        let view = cx.entity();
        let dismiss_view = view.clone();
        let dismiss = move |_window: &mut Window, cx: &mut App| {
            dismiss_view.update(cx, |this, cx| {
                this.text_context_menu = None;
                cx.notify();
            });
        };
        let has_selection = self
            .doc
            .selected_text()
            .is_some_and(|text| !text.is_empty());
        let mut items = Vec::new();

        let copy_view = view.clone();
        items.push(
            MenuItem::new("editor-copy", "Copy")
                .keybind(["⌘", "C"])
                .disabled(!has_selection)
                .on_click(move |_, _, cx| {
                    copy_view.update(cx, |this, cx| {
                        this.copy(cx);
                        this.text_context_menu = None;
                    });
                }),
        );
        let cut_view = view.clone();
        items.push(
            MenuItem::new("editor-cut", "Cut")
                .keybind(["⌘", "X"])
                .disabled(!has_selection)
                .on_click(move |_, _, cx| {
                    cut_view.update(cx, |this, cx| {
                        this.cut(cx);
                        this.text_context_menu = None;
                    });
                }),
        );
        let paste_view = view.clone();
        items.push(
            MenuItem::new("editor-paste", "Paste")
                .keybind(["⌘", "V"])
                .on_click(move |_, _, cx| {
                    paste_view.update(cx, |this, cx| {
                        this.paste(cx);
                        this.text_context_menu = None;
                    });
                }),
        );
        items.push(MenuItem::separator());

        let select_all_view = view.clone();
        items.push(
            MenuItem::new("editor-select-all", "Select All")
                .keybind(["⌘", "A"])
                .on_click(move |_, _, cx| {
                    select_all_view.update(cx, |this, cx| {
                        this.doc.select_all();
                        this.text_context_menu = None;
                        cx.notify();
                    });
                }),
        );
        let undo_view = view.clone();
        items.push(
            MenuItem::new("editor-undo", "Undo")
                .keybind(["⌘", "Z"])
                .on_click(move |_, _, cx| {
                    undo_view.update(cx, |this, cx| {
                        this.edit(cx, Document::undo);
                        this.text_context_menu = None;
                    });
                }),
        );
        let redo_view = view.clone();
        items.push(
            MenuItem::new("editor-redo", "Redo")
                .keybind(["⌘", "⇧", "Z"])
                .on_click(move |_, _, cx| {
                    redo_view.update(cx, |this, cx| {
                        this.edit(cx, Document::redo);
                        this.text_context_menu = None;
                    });
                }),
        );

        Some(context_menu(
            position,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        ))
    }

    /// Hunk discard is intentionally not a direct editor mutation. The Git
    /// capability currently exposes whole-file discard only, so the decision
    /// surface explains the limitation and offers the canonical Project Diff
    /// route instead.
    fn render_git_discard_confirmation(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let preview = self.git_discard_confirmation.clone()?;
        if !self.git_snapshot_is_current(&self.git_gutter)
            || preview.target.revision != self.editor_revision()
            || self.current_git_path().as_deref() != Some(preview.target.path.as_str())
        {
            return None;
        }

        let theme = self.theme.read(cx);
        let (card, fg, border, accent, muted) = (
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let palette = Palette::from_theme(theme);
        let view = cx.entity();
        let target = preview.target.clone();
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(crate::theme::modal_scrim())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .max_w(px(480.0))
                        .p_4()
                        .rounded_lg()
                        .bg(card)
                        .border_1()
                        .border_color(border)
                        .text_color(fg)
                        .child("Discard Hunk")
                        .child(SharedString::from(format!(
                            "{} ({})",
                            preview.summary, preview.target.hunk_id
                        )))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(muted)
                                .child(
                                    "Individual hunk discard is not supported. Open Project Diff to review the whole-file operation.",
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .justify_end()
                                .child(
                                    button(
                                        "editor-git-discard-cancel",
                                        palette,
                                        ButtonVariant::Ghost,
                                        ButtonSize::Sm,
                                    )
                                    .text_color(muted)
                                    .hover(|s| s.bg(border).text_color(fg))
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                                        this.git_discard_confirmation = None;
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    button(
                                        "editor-git-discard-project-diff",
                                        palette,
                                        ButtonVariant::Default,
                                        ButtonSize::Sm,
                                    )
                                    .bg(accent)
                                    .text_color(fg)
                                    .hover(|s| s.opacity(0.85))
                                    .child("Open Project Diff")
                                    .on_click(move |_, _, cx| {
                                        let target = target.clone();
                                        view.update(cx, |this, cx| {
                                            this.git_discard_confirmation = None;
                                            this.submit_git_action(
                                                GitHunkAction::OpenProjectDiff { target },
                                                cx,
                                            );
                                        });
                                    }),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_conflict_banner(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let c = Palette::from_theme(theme);
        banner(Severity::Warning, c)
            .child("This file changed on disk since you started editing.")
            .child(
                button("conflict-reload", c, ButtonVariant::Outline, ButtonSize::Xs)
                    .child("Reload (discard my changes)")
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.reload_from_disk(cx)),
                    ),
            )
            .child(
                button("conflict-keep", c, ButtonVariant::Outline, ButtonSize::Xs)
                    .child("Keep mine")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        match this.doc.keep_buffer() {
                            Ok(FileLifecycleEvent::DirtyChanged { .. }) => {
                                cx.emit(EditorEvent::Changed);
                                cx.notify();
                            }
                            Ok(_) => {}
                            Err(error) => notify(cx, "Conflict action failed", &error.to_string()),
                        }
                    })),
            )
    }

    /// Local editor status for an explicitly injected language server. Syntax
    /// fallback remains quiet so opening ordinary files does not add chrome.
    fn render_language_service_status(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let status = &self.language_service_snapshot;
        let (severity, message) = match (status.source, status.status) {
            (
                LanguageServiceRuntimeSource::LocalProcess,
                LanguageServiceRuntimeStatus::Starting,
            ) => (
                Severity::Info,
                "Starting local language service…".to_string(),
            ),
            (LanguageServiceRuntimeSource::LocalProcess, LanguageServiceRuntimeStatus::Running) => {
                (
                    Severity::Info,
                    format!("{} language service active", status.server.as_str()),
                )
            }
            (_, LanguageServiceRuntimeStatus::Crashed) => (
                Severity::Warning,
                format!(
                    "{} unavailable; using syntax fallback{}",
                    status.server.as_str(),
                    status
                        .detail
                        .as_deref()
                        .map(|detail| format!(": {detail}"))
                        .unwrap_or_default()
                ),
            ),
            (_, LanguageServiceRuntimeStatus::Unsupported) => (
                Severity::Info,
                "No language service is registered for this language.".to_string(),
            ),
            _ => return None,
        };
        Some(
            banner(severity, Palette::from_theme(self.theme.read(cx)))
                .child(SharedString::from(message))
                .into_any_element(),
        )
    }

    /// Render the file-lifecycle state without putting non-text content into
    /// the editable buffer. Binary, oversized, missing and failed loads use a
    /// dedicated status surface; read-only text remains visible with editing
    /// disabled by the editor-owned lifecycle.
    fn render_file_status(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = self.doc.file_state();
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let path = self
            .doc
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "this file".to_string());

        let (severity, message, retry) = match state {
            FileState::Loading => (Severity::Info, "Loading file…".to_string(), false),
            FileState::ReadOnly => (
                Severity::Info,
                format!("{path} is read-only. Editing and saving are disabled."),
                false,
            ),
            FileState::ReloadNeeded => (
                Severity::Warning,
                "This file changed on disk and will be reloaded.".to_string(),
                false,
            ),
            FileState::Missing => (
                Severity::Error,
                format!("{path} is missing. No file content was replaced."),
                true,
            ),
            FileState::Binary => (
                Severity::Warning,
                format!("{path} is a binary file and cannot be edited as text."),
                false,
            ),
            FileState::TooLarge => (
                Severity::Warning,
                format!("{path} is too large for the editor."),
                false,
            ),
            FileState::Error(message) => (Severity::Error, message.clone(), true),
            FileState::New | FileState::Clean | FileState::Dirty | FileState::Conflict => {
                return None
            }
        };

        let action = retry.then(|| {
            let button_path = self.doc.path.clone();
            button(
                "file-state-retry",
                palette,
                ButtonVariant::Outline,
                ButtonSize::Xs,
            )
            .child("Retry")
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                if let Some(path) = button_path.clone() {
                    this.open_path(path, cx);
                }
            }))
            .into_any_element()
        });
        let status_banner = banner(severity, palette)
            .child(message)
            .children(action)
            .into_any_element();

        let terminal = matches!(
            state,
            FileState::Loading
                | FileState::Missing
                | FileState::Binary
                | FileState::TooLarge
                | FileState::Error(_)
        );
        Some(if terminal {
            div()
                .flex_1()
                .items_center()
                .justify_center()
                .px_6()
                .child(div().max_w(px(640.0)).w_full().child(status_banner))
                .into_any_element()
        } else {
            status_banner
        })
    }

    fn should_render_text_area(&self) -> bool {
        match self.doc.file_state() {
            FileState::Loading | FileState::Binary | FileState::TooLarge => false,
            FileState::Missing | FileState::Error(_) => self.doc.is_dirty(),
            _ => true,
        }
    }

    fn restore_recovery(&mut self, cx: &mut Context<Self>) {
        let Some(recovery) = self.recovery_prompt.clone() else {
            return;
        };
        let before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let selections = self.doc.selection_set();
        let end = self.doc.snapshot().byte_len();
        if self.doc.apply_edits(
            vec![Edit::new(TextRange::new(0, end), recovery.content)],
            selections,
            labonair_editor::EditSource::External,
            Some("Restore unsaved editor buffer".to_string()),
        ) {
            self.recovery_prompt = None;
            self.finish_command(before, dirty_before, self.doc.selection_set(), true, cx);
        }
    }

    fn discard_recovery(&mut self, cx: &mut Context<Self>) {
        self.recovery_prompt = None;
        cx.notify();
    }

    fn render_recovery_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let recovery = self.recovery_prompt.as_ref()?;
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        Some(
            banner(Severity::Info, palette)
                .child(SharedString::from(format!(
                    "Unsaved changes are available for {}.",
                    recovery.path
                )))
                .child(
                    button(
                        "editor-recovery-restore",
                        palette,
                        ButtonVariant::Default,
                        ButtonSize::Xs,
                    )
                    .child("Restore")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.restore_recovery(cx);
                        },
                    )),
                )
                .child(
                    button(
                        "editor-recovery-discard",
                        palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .child("Discard")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.discard_recovery(cx);
                        },
                    )),
                )
                .into_any_element(),
        )
    }

    /// Bottom status line for Vim mode: the mode indicator plus the live
    /// `:` / `/` command line (T06-003).
    fn render_vim_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, muted, border, accent) = (
            theme.card(),
            theme.foreground(),
            theme.muted_foreground(),
            theme.border(),
            theme.accent(),
        );
        let vim = self.vim.as_ref().unwrap();
        let (label, is_command) = match vim.command_line() {
            Some((prefix, text)) => (format!("{prefix}{text}"), true),
            None => {
                let s = vim.status();
                (
                    if s.is_empty() {
                        "NORMAL".to_string()
                    } else {
                        s
                    },
                    false,
                )
            }
        };
        let cursor = self.doc.cursor;
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_0p5()
            .bg(card)
            .border_t_1()
            .border_color(border)
            .text_xs()
            .font(theme.buffer_font())
            .text_color(if is_command { fg } else { accent })
            .child(SharedString::from(label))
            .child(div().text_color(muted).child(SharedString::from(format!(
                "{}:{}",
                cursor.line + 1,
                cursor.column + 1
            ))))
    }

    /// The schema-hover tooltip (T19-006 Anweisung #5) — a small floating
    /// card glued to the last mouse position, shown only while
    /// [`Self::hover`] is `Some`. A window-relative absolute overlay rather
    /// than GPUI's trigger-based `.tooltip()` widget: that widget attaches
    /// to one fixed element, but the hover target here is a specific
    /// character position inside one big text-row div, which
    /// [`Self::update_hover`] already resolves on every mouse move.
    fn render_hover_tooltip(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (pos, desc) = self.hover.clone()?;
        let theme = self.theme.read(cx);
        let (card, fg, border) = (theme.card(), theme.foreground(), theme.border());
        let origin = self.bounds.map(|b| b.origin).unwrap_or_default();
        Some(
            div()
                .absolute()
                .top(pos.y - origin.y + px(16.0))
                .left(pos.x - origin.x)
                .max_w(px(320.0))
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(border)
                .bg(card)
                .text_size(px(11.0))
                .text_color(fg)
                .shadow_md()
                .child(desc),
        )
    }

    /// Paint one non-focused editor group as a read-only mirror of the active
    /// document. The split tree is editor-owned, while this GPUI adapter is
    /// intentionally the only place that turns its groups into pixels. A
    /// future multi-document group model can replace this mirror without
    /// changing the split contract. Mirrors deliberately show the same
    /// snapshot revision so a focused-group switch cannot present stale text.
    fn render_split_mirror(
        &self,
        group: labonair_editor::EditorGroupId,
        snapshot: &labonair_editor::BufferSnapshot,
        display: &labonair_editor::DisplaySnapshot,
        font: gpui::Font,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let font_px = self.prefs.font_size() as f32;
        let line_h = self.metrics.1;
        let border = palette.border;
        let muted = palette.muted;
        let fg = palette.fg;
        let group_value = group.value();
        let rows = display.rows.iter().take(200).map(|row| {
            div()
                .h(px(line_h))
                .flex()
                .items_center()
                .text_size(px(font_px))
                .font(font.clone())
                .text_color(fg)
                .child(SharedString::from(row.text(snapshot)))
        });
        let close = button(
            ("editor-split-close", group_value),
            palette,
            ButtonVariant::Ghost,
            ButtonSize::IconXs,
        )
        .child("×")
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            if this.editor_splits.close(group).is_ok() {
                cx.emit(EditorEvent::Changed);
                cx.notify();
            }
        }));
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_1()
            .border_color(border)
            .child(
                div()
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .bg(palette.muted_bg)
                    .text_xs()
                    .text_color(muted)
                    .child(SharedString::from(format!(
                        "Editor group {group_value} · mirror · rev {}",
                        snapshot.revision().value()
                    )))
                    .child(close),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .px_2()
                    .children(rows),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                    if this.editor_splits.focus(group).is_ok() {
                        cx.emit(EditorEvent::Changed);
                        cx.notify();
                    }
                    cx.stop_propagation();
                }),
            )
            .into_any_element()
    }

    /// Recursively materialize the Editor-owned split tree. Every split node
    /// gets its own flex axis, local first-child ratio, and local drag
    /// coordinate space; flattening leaves at the root would make nested
    /// dividers resize the wrong node.
    // The recursive renderer carries immutable document/display projections
    // plus the one active widget; keeping those values explicit avoids a
    // second editor-owned context object or workspace split state.
    #[allow(clippy::too_many_arguments)]
    fn render_split_node(
        &self,
        node: &labonair_editor::SplitNode,
        path: &labonair_editor::splits::SplitPath,
        focused_group: labonair_editor::EditorGroupId,
        active_text_area: &mut Option<AnyElement>,
        snapshot: &labonair_editor::BufferSnapshot,
        display: &labonair_editor::DisplaySnapshot,
        font: gpui::Font,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            labonair_editor::SplitNode::Leaf(group) => {
                if *group == focused_group {
                    if let Some(text_area) = active_text_area.take() {
                        let group = *group;
                        let group_value = group.value();
                        let close = button(
                            ("editor-split-close", group_value),
                            palette,
                            ButtonVariant::Ghost,
                            ButtonSize::IconXs,
                        )
                        .child("×")
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                if this.editor_splits.close(group).is_ok() {
                                    cx.emit(EditorEvent::Changed);
                                    cx.notify();
                                }
                            },
                        ));
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .border_1()
                            .border_color(palette.accent)
                            .child(
                                div()
                                    .h(px(24.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px_2()
                                    .bg(palette.muted_bg)
                                    .text_xs()
                                    .text_color(palette.fg)
                                    .child(SharedString::from(format!(
                                        "Editor group {} · active",
                                        group.value()
                                    )))
                                    .child(close),
                            )
                            .child(text_area)
                            .into_any_element()
                    } else {
                        self.render_split_mirror(*group, snapshot, display, font, palette, cx)
                    }
                } else {
                    self.render_split_mirror(*group, snapshot, display, font, palette, cx)
                }
            }
            labonair_editor::SplitNode::Split {
                orientation,
                first_size,
                first,
                second,
            } => {
                let first_path = path.child(labonair_editor::splits::SplitSide::First);
                let second_path = path.child(labonair_editor::splits::SplitSide::Second);
                let first_element = self.render_split_node(
                    first,
                    &first_path,
                    focused_group,
                    active_text_area,
                    snapshot,
                    display,
                    font.clone(),
                    palette,
                    cx,
                );
                let second_element = self.render_split_node(
                    second,
                    &second_path,
                    focused_group,
                    active_text_area,
                    snapshot,
                    display,
                    font,
                    palette,
                    cx,
                );
                let first_size = if first_size.is_finite() {
                    first_size.clamp(0.1, 0.9)
                } else {
                    0.5
                };
                let mut first_slot = div()
                    .min_w_0()
                    .min_h_0()
                    .flex_shrink_0()
                    .child(first_element);
                let mut second_slot = div()
                    .min_w_0()
                    .min_h_0()
                    .flex_shrink_0()
                    .child(second_element);
                match orientation {
                    labonair_editor::SplitOrientation::Horizontal => {
                        first_slot = first_slot.w(relative(first_size));
                        second_slot = second_slot.w(relative(1.0 - first_size));
                    }
                    labonair_editor::SplitOrientation::Vertical => {
                        first_slot = first_slot.h(relative(first_size));
                        second_slot = second_slot.h(relative(1.0 - first_size));
                    }
                };
                let drag = EditorSplitDrag {
                    path: path.clone(),
                    orientation: *orientation,
                };
                let view = cx.entity();
                let mut divider = div()
                    .id(SharedString::from(format!(
                        "editor-split-divider-{}",
                        path.key()
                    )))
                    .flex_shrink_0()
                    .bg(palette.border)
                    .hover(|style| style.bg(palette.accent));
                divider = match orientation {
                    labonair_editor::SplitOrientation::Horizontal => {
                        divider.w(px(3.0)).h_full().cursor_col_resize()
                    }
                    labonair_editor::SplitOrientation::Vertical => {
                        divider.h(px(3.0)).w_full().cursor_row_resize()
                    }
                };
                let divider = divider.on_drag(drag, move |_, _, _, cx| {
                    view.update(cx, |this, _cx| {
                        this.split_drag_last = None;
                    });
                    cx.new(|_| crate::DragGhost)
                });
                let mut surface = div().min_w_0().min_h_0().flex().flex_1();
                surface = match orientation {
                    labonair_editor::SplitOrientation::Horizontal => surface.flex_row(),
                    labonair_editor::SplitOrientation::Vertical => surface.flex_col(),
                };
                let orientation = *orientation;
                surface
                    .child(first_slot)
                    .child(divider)
                    .child(second_slot)
                    .on_drag_move(cx.listener(
                        move |this, event: &gpui::DragMoveEvent<EditorSplitDrag>, _window, cx| {
                            let drag = event.drag(cx);
                            if drag.orientation != orientation {
                                return;
                            }
                            let bounds = event.bounds;
                            let fraction = match orientation {
                                labonair_editor::SplitOrientation::Horizontal => {
                                    f32::from(event.event.position.x - bounds.origin.x)
                                        / f32::from(bounds.size.width).max(1.0)
                                }
                                labonair_editor::SplitOrientation::Vertical => {
                                    f32::from(event.event.position.y - bounds.origin.y)
                                        / f32::from(bounds.size.height).max(1.0)
                                }
                            };
                            if let Some(previous) = this.split_drag_last.replace(fraction) {
                                if this
                                    .editor_splits
                                    .resize_at(&drag.path, fraction - previous)
                                    .is_ok()
                                {
                                    cx.emit(EditorEvent::Changed);
                                    cx.notify();
                                }
                            }
                        },
                    ))
                    .into_any_element()
            }
        }
    }

    /// Compact editor-owned status row. This keeps cursor and selection
    /// feedback close to the buffer while respecting the user's visibility
    /// preferences; the shell statusbar remains a separate global surface.
    fn render_editor_status(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let snapshot = self.doc.snapshot();
        let selection_set = self.doc.selection_set();
        let selected_chars = selection_set
            .ranges(&snapshot)
            .iter()
            .map(|range| snapshot.text_range(*range).chars().count())
            .sum::<usize>();
        let mut left = Vec::new();
        if self.prefs.show_cursor_position() {
            left.push(format!(
                "Ln {}, Col {}",
                self.doc.cursor.line + 1,
                self.doc.cursor.column + 1
            ));
        }
        if self.prefs.show_selection_stats() {
            left.push(format!(
                "{} selection{} · {} chars",
                selection_set.len(),
                if selection_set.len() == 1 { "" } else { "s" },
                selected_chars
            ));
        }
        let right = format!(
            "{} · {} lines",
            self.doc.language.label(),
            snapshot.line_count()
        );
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .flex_shrink_0()
            .h(px(24.0))
            .px_3()
            .bg(palette.bg)
            .border_t_1()
            .border_color(palette.border)
            .text_xs()
            .text_color(palette.muted)
            .child(SharedString::from(left.join("  ·  ")))
            .child(SharedString::from(right))
            .into_any_element()
    }

    /// Render the editor-local path and current symbol context. This is part
    /// of the editor surface, not a new shell chrome zone.
    fn render_breadcrumb(
        &self,
        state: BreadcrumbState,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut row = div()
            .id("editor-breadcrumb")
            .flex()
            .items_center()
            .gap_1()
            .w_full()
            .min_w_0()
            .h(px(30.0))
            .px_2()
            .flex_shrink_0()
            .overflow_hidden()
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.bg)
            .text_xs()
            .text_color(palette.muted);

        match state {
            BreadcrumbState::Empty => {
                row = row.child(SharedString::from("Untitled buffer"));
            }
            BreadcrumbState::Loading => {
                row = row.child(SharedString::from("Loading…"));
            }
            BreadcrumbState::Missing(path) => {
                row = row.child(SharedString::from(match path {
                    Some(path) => format!("Missing · {}", path.label),
                    None => "Missing file".to_string(),
                }));
            }
            BreadcrumbState::Error(path) => {
                row = row.child(SharedString::from(match path {
                    Some(path) => format!("Unable to load · {}", path.label),
                    None => "Unable to load file".to_string(),
                }));
            }
            BreadcrumbState::Ready { path, symbol } => {
                let has_path = path.is_some();
                let has_symbol = symbol.is_some();
                if let Some(path) = path {
                    let full_path = path.full_path.clone();
                    row = row.child(
                        div()
                            .id("editor-breadcrumb-path")
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(palette.muted)
                            .tooltip(move |window, cx| {
                                Tooltip::new(full_path.clone()).build(window, cx)
                            })
                            .child(SharedString::from(path.label)),
                    );
                }

                if let Some(symbol) = symbol {
                    if has_path {
                        row = row.child(
                            div()
                                .flex_shrink_0()
                                .text_color(palette.muted)
                                .child(SharedString::from("/")),
                        );
                    }
                    let line = symbol.line;
                    let label = format!("{}: {}", symbol.kind.label(), symbol.name);
                    let tooltip = format!("Go to {} at line {}", symbol.name, line + 1);
                    let ring = palette.ring;
                    let hover_fill = palette.accent.opacity(0.5);
                    row = row.child(
                        div()
                            .min_w_0()
                            .max_w(px(320.0))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .child(
                                ListItem::new(
                                    ("editor-breadcrumb-symbol", line),
                                    palette.fg,
                                    palette.muted,
                                    palette.selected_fill,
                                )
                                .selected(true)
                                .hover_style(move |style| style.bg(hover_fill))
                                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                    this.focus(window);
                                    this.goto_line(line, cx);
                                }))
                                .extra(move |item| {
                                    item.tab_index(0)
                                        .focus(move |style| style.border_1().border_color(ring))
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(tooltip.clone()).build(window, cx)
                                        })
                                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                            cx.stop_propagation()
                                        })
                                })
                                .child(
                                    div()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .child(SharedString::from(label)),
                                ),
                            ),
                    );
                }

                if !has_path && !has_symbol {
                    row = row.child(SharedString::from("No file or symbol context"));
                }
            }
        }

        row.into_any_element()
    }

    fn render_outline(
        &self,
        symbols: &[labonair_editor::DocumentSymbol],
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rows = symbols.iter().enumerate().take(80).map(|(index, symbol)| {
            let line = symbol.line;
            button(
                ("editor-outline-symbol", index),
                palette,
                ButtonVariant::Ghost,
                ButtonSize::Xs,
            )
            .w_full()
            .justify_start()
            .child(SharedString::from(format!(
                "{}  {}",
                symbol.kind.label(),
                symbol.name
            )))
            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                this.doc.set_caret(Position::new(line, 0), false);
                this.ensure_cursor_visible();
                cx.emit(EditorEvent::Changed);
                cx.notify();
            }))
        });
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .w(px(220.0))
            .min_h_0()
            .border_l_1()
            .border_color(palette.border)
            .bg(palette.sidebar)
            .child(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .border_b_1()
                    .border_color(palette.border)
                    .text_xs()
                    .text_color(palette.fg)
                    .child("Outline"),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .p_1()
                    .children(rows),
            )
            .into_any_element()
    }

    fn render_minimap(
        &self,
        snapshot: &labonair_editor::BufferSnapshot,
        palette: Palette,
    ) -> AnyElement {
        let line_count = snapshot.line_count().max(1);
        let rows = (0..line_count.min(500)).map(|line| {
            let width = (snapshot.line_len(line).min(60) as f32 / 60.0 * 62.0).max(4.0);
            div().h(px(2.0)).w(px(width)).bg(palette.muted.opacity(
                if line == self.doc.cursor.line {
                    0.95
                } else {
                    0.45
                },
            ))
        });
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(px(76.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .overflow_hidden()
            .p_2()
            .bg(palette.sidebar.opacity(0.86))
            .border_l_1()
            .border_color(palette.border)
            .children(rows)
            .into_any_element()
    }

    fn render_language_service_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = self.language_service_state.snapshot();
        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let mut overlay = div()
            .absolute()
            .top(px(12.0))
            .right(px(92.0))
            .w(px(360.0))
            .max_h(px(360.0))
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(palette.border)
            .bg(palette.popover)
            .shadow_md();
        let mut shown = false;
        if self.completion_visible && self.prefs.completion() {
            if let Some(list) = state.completion.clone() {
                if !list.items.is_empty() {
                    shown = true;
                    let visible_count = list.items.len().min(MAX_COMPLETION_ITEMS);
                    let selected = self.completion_selected.min(visible_count - 1);
                    let rows =
                        list.items
                            .iter()
                            .enumerate()
                            .take(visible_count)
                            .map(|(index, item)| {
                                let label = item.label.clone();
                                let detail = item.detail.clone().unwrap_or_default();
                                let kind = completion_kind_label(item.kind);
                                let mut content = div().flex().flex_col().min_w_0().flex_1().child(
                                    div()
                                        .flex()
                                        .w_full()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .child(SharedString::from(label)),
                                        )
                                        .child(
                                            div()
                                                .flex_shrink_0()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child(SharedString::from(kind)),
                                        ),
                                );
                                if !detail.is_empty() {
                                    content = content.child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .overflow_hidden()
                                            .child(SharedString::from(detail)),
                                    );
                                }
                                ListItem::new(
                                    ("editor-completion-item", index),
                                    palette.fg,
                                    palette.muted,
                                    palette.selected_fill,
                                )
                                .selected(index == selected)
                                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                                    this.apply_completion_item(index, cx);
                                }))
                                .child(content)
                            });
                    overlay = overlay
                        .child(labonair_ui_kit::list_header("Completion", palette.muted))
                        .child(
                            div()
                                .id("editor-completion-list")
                                .max_h(px(240.0))
                                .overflow_y_scroll()
                                .children(rows),
                        );
                    if let Some(documentation) = list
                        .items
                        .get(selected)
                        .and_then(|item| item.documentation.clone())
                        .filter(|documentation| !documentation.is_empty())
                    {
                        overlay = overlay.child(
                            div()
                                .id("editor-completion-documentation")
                                .max_h(px(64.0))
                                .overflow_y_scroll()
                                .whitespace_normal()
                                .px_2()
                                .py_1()
                                .border_t_1()
                                .border_color(palette.border)
                                .text_xs()
                                .text_color(palette.muted)
                                .child(SharedString::from(documentation)),
                        );
                    }
                    if list.items.len() > visible_count || list.is_incomplete {
                        overlay = overlay.child(
                            div()
                                .px_2()
                                .py_1()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(SharedString::from(format!(
                                    "Showing {visible_count} of {} completion items",
                                    list.items.len()
                                ))),
                        );
                    }
                }
            }
        }
        if let Some(actions) = self.code_actions.clone() {
            if !actions.is_empty() {
                shown = true;
                let rows = actions.iter().enumerate().take(10).map(|(index, action)| {
                    button(
                        ("editor-code-action-item", index),
                        palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .w_full()
                    .justify_start()
                    .child(SharedString::from(action.title.clone()))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _window, cx| {
                            this.apply_code_action(index, cx);
                        },
                    ))
                });
                overlay = overlay
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .text_xs()
                            .text_color(palette.muted)
                            .child("Code actions"),
                    )
                    .children(rows);
            }
        }
        if let Some(hover) = state.hover {
            if self.prefs.hover() && !hover.contents.is_empty() {
                shown = true;
                overlay = overlay.child(
                    div()
                        .p_2()
                        .text_xs()
                        .text_color(palette.fg)
                        .child(SharedString::from(hover.contents)),
                );
            }
        }
        shown.then(|| overlay.into_any_element())
    }

    fn render_signature_help(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let state = self.language_service_state.snapshot();
        let current_position = self.current_lsp_position();
        let current_version = DocumentVersion::from_revision(self.editor_revision());
        if !signature_help_render_eligible(
            &state,
            &self.signature_help_state,
            self.signature_help_position,
            current_position,
            current_version,
            self.language_service_generation,
        ) {
            return None;
        }

        let theme = self.theme.read(cx);
        let palette = Palette::from_theme(theme);
        let (char_width, line_height) = self.metrics;
        let bounds = self.bounds.map(|bounds| bounds.size);
        let width = bounds.map(|size| f32::from(size.width)).unwrap_or(640.0);
        let height = bounds.map(|size| f32::from(size.height)).unwrap_or(600.0);
        let popover_width = 360.0;
        let popover_height = 180.0;
        let cursor_left = self.gutter_width
            + self.doc.cursor.column.saturating_sub(self.scroll_left) as f32 * char_width;
        let left = cursor_left.clamp(8.0, (width - popover_width - 8.0).max(8.0));
        let cursor_top =
            (self.doc.cursor.line.saturating_sub(self.scroll_top) + 1) as f32 * line_height + 8.0;
        let top = if cursor_top + popover_height > height {
            (cursor_top - line_height - popover_height - 8.0).max(8.0)
        } else {
            cursor_top
        };

        let mut popover = div()
            .absolute()
            .top(px(top))
            .left(px(left))
            .id("editor-signature-help")
            .w(px(popover_width))
            .max_h(px(popover_height))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(palette.border)
            .bg(palette.popover)
            .shadow_md()
            .child(labonair_ui_kit::list_header(
                "Signature help",
                palette.muted,
            ));
        match (&self.signature_help_state, state.signature_help) {
            (SignatureHelpUiState::Loading, _) => {
                popover = popover.child(signature_help_message(palette, "Loading…"));
            }
            (SignatureHelpUiState::Empty, _) => {
                popover = popover.child(signature_help_message(
                    palette,
                    "No signature help is available at this position.",
                ));
            }
            (SignatureHelpUiState::Unavailable(message), _) => {
                popover = popover.child(signature_help_message(
                    palette,
                    format!("Unavailable: {message}"),
                ));
            }
            (SignatureHelpUiState::Error(message), _) => {
                popover =
                    popover.child(signature_help_message(palette, format!("Error: {message}")));
            }
            (SignatureHelpUiState::Ready, Some(help)) => {
                popover = popover.child(
                    div()
                        .px_2()
                        .pb_1()
                        .whitespace_normal()
                        .text_xs()
                        .text_color(palette.fg)
                        .child(SharedString::from(help.label)),
                );
                if let Some(documentation) = help.documentation.filter(|text| !text.is_empty()) {
                    popover = popover.child(
                        div()
                            .px_2()
                            .pb_2()
                            .whitespace_normal()
                            .text_xs()
                            .text_color(palette.muted)
                            .child(SharedString::from(documentation)),
                    );
                }
            }
            (SignatureHelpUiState::Ready, None) => {
                popover = popover.child(signature_help_message(
                    palette,
                    "No signature help is available at this position.",
                ));
            }
            (SignatureHelpUiState::Hidden, _) => return None,
        }
        Some(popover.into_any_element())
    }

    fn render_rename_dialog(
        &self,
        input: &Entity<InputState>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input_for_submit = input.clone();
        div()
            .absolute()
            .top(px(12.0))
            .right(px(92.0))
            .w(px(360.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(palette.border)
            .bg(palette.popover)
            .shadow_md()
            .child(
                div()
                    .text_xs()
                    .text_color(palette.muted)
                    .child("Rename symbol"),
            )
            .child(field_input(input).w_full())
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_1()
                    .mt_1()
                    .child(
                        button(
                            "editor-rename-cancel",
                            palette,
                            ButtonVariant::Ghost,
                            ButtonSize::Xs,
                        )
                        .child("Cancel")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.cancel_rename(cx);
                            },
                        )),
                    )
                    .child(
                        button(
                            "editor-rename-submit",
                            palette,
                            ButtonVariant::Default,
                            ButtonSize::Xs,
                        )
                        .child("Rename")
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                let value = input_for_submit.read(cx).value().to_string();
                                this.submit_rename(value, cx);
                            },
                        )),
                    ),
            )
            .into_any_element()
    }

    fn sticky_context_label(&self, snapshot: &labonair_editor::BufferSnapshot) -> Option<String> {
        if !self.prefs.sticky_context() {
            return None;
        }
        document_symbols(self.doc.language, snapshot)
            .into_iter()
            .filter(|symbol| symbol.line <= self.doc.cursor.line)
            .max_by_key(|symbol| symbol.line)
            .map(|symbol| format!("{}  ·  line {}", symbol.name, symbol.line + 1))
    }
}

impl EventEmitter<EditorEvent> for EditorView {}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_rename_input(window, cx);
        let theme = self.theme.read(cx);
        let bg = theme.background();
        let fg = theme.foreground();
        let muted = theme.muted_foreground();
        let accent = theme.accent();
        let gutter_bg = theme.card();
        let font = theme.buffer_font();
        let font_px = self.prefs.font_size() as f32;
        let line_h = (font_px * self.prefs.line_height()).ceil().max(1.0);
        let editor_font_family = self.prefs.font_family().to_string();

        let font_id = cx.text_system().resolve_font(&font);
        let char_w = cx
            .text_system()
            .ch_advance(font_id, px(font_px))
            .map(f32::from)
            .unwrap_or(font_px * 0.6)
            .max(1.0);
        self.metrics = (char_w, line_h);

        let snapshot = self.doc.snapshot();
        self.language_service_snapshot = self.language_services.snapshot(
            self.doc.language,
            DocumentVersion::from_revision(snapshot.revision()),
            self.language_service_generation,
        );
        let line_count = snapshot.line_count();
        let digits = line_count.to_string().len().max(3);
        self.gutter_width = digits as f32 * char_w + 16.0;
        self.clamp_scroll();

        let display = self.display_snapshot(&snapshot);
        let wrapping = self.display_config().wrap_columns.is_some();
        let first_visual = display.rows.first().map(|row| row.visual_row).unwrap_or(0);
        let content_w = (display.viewport.width_columns as f32 * char_w).max(char_w);
        let display_rows = display.rows.clone();
        let visible_lines = display_rows
            .iter()
            .map(|row| row.logical_line())
            .collect::<Vec<_>>();
        let visible_start_line = visible_lines.iter().copied().min().unwrap_or(0);
        let visible_end_line = visible_lines.iter().copied().max().unwrap_or(0);
        let git_decorations =
            if self.prefs.git_gutter() && self.git_snapshot_is_current(&self.git_gutter) {
                self.git_gutter.display_decorations(&snapshot, &display)
            } else {
                Default::default()
            };
        let git_inline_decorations =
            git_inline_decorations_for_view(&self.prefs, &self.git_gutter, &snapshot, &display);
        let service_state = self.language_service_state.snapshot();
        let diagnostic_decorations = if self.prefs.diagnostics() {
            display.diagnostic_decorations(&snapshot, &service_state.diagnostics)
        } else {
            Vec::new()
        };
        let semantic_decorations = if self.prefs.semantic_tokens() {
            display.semantic_token_decorations(&snapshot, &service_state.semantic_tokens)
        } else {
            Vec::new()
        };

        let sel = self.doc.selection();
        let cursor = self.doc.cursor;
        let gutter_width = self.gutter_width;

        // Syntax highlighting (T06-002): parse once per revision, keep only the
        // spans covering the visible line range, and repaint with the palette
        // resolved from the active app theme.
        let palette = EditorPalette::resolve(theme.editor_theme(), theme);
        let ui_palette = Palette::from_theme(theme);
        let rename_input = self.rename_input.clone();
        let symbols = document_symbols(self.doc.language, &snapshot);
        let outline_symbols = if self.prefs.show_outline() {
            symbols.clone()
        } else {
            Vec::new()
        };
        let fold_candidates = fold_ranges_from_symbols(&symbols, &snapshot);
        let breadcrumb_state = build_breadcrumb_state(
            self.doc.file_state(),
            self.doc.path.as_deref(),
            &symbols,
            cursor.line,
            EDITOR_BREADCRUMB_MAX_PATH_CHARS,
        );
        let sticky_context = self.sticky_context_label(&snapshot);
        let sticky_height = if sticky_context.is_some() { 24.0 } else { 0.0 };
        let ruler_columns = self
            .prefs
            .rulers()
            .split([',', ' ', ';'])
            .filter_map(|value| value.trim().parse::<usize>().ok())
            .filter(|column| *column > 0)
            .collect::<Vec<_>>();
        let visible_start = snapshot.line_start_byte(visible_start_line);
        let visible_end = snapshot
            .line_end_byte(visible_end_line)
            .saturating_add(1)
            .min(snapshot.byte_len());
        self.syntax.update(&snapshot, visible_start..visible_end);

        // Gutter rows. Line-number visibility follows the editor preferences
        // (T13-003); Vim's `:set number` overrides them while Vim mode is on.
        let gutter = display_rows.iter().map(|display_row| {
            let line = display_row.logical_line();
            let on_cursor = line == cursor.line;
            let git_marker = git_decorations
                .iter()
                .find(|decoration| decoration.visual_row == display_row.visual_row);
            let continuation = matches!(
                display_row.kind,
                DisplayRowKind::Text { start_column, .. } if start_column > 0
            );
            let label = if continuation {
                String::new()
            } else if let Some(number) = display_row.relative_line_number {
                number.to_string()
            } else {
                display_row
                    .line_number
                    .map_or_else(String::new, |n| n.to_string())
            };
            let row_top = (display_row.visual_row.saturating_sub(first_visual) as f32) * line_h;
            let mut row = div()
                .absolute()
                .top(px(row_top))
                .left_0()
                .w(px(gutter_width))
                .h(px(line_h))
                .flex()
                .items_center()
                .justify_end()
                .pr(px(8.0))
                .text_size(px(font_px))
                .font_family(editor_font_family.clone())
                .text_color(if on_cursor { fg } else { muted })
                .child(SharedString::from(
                    if matches!(display_row.kind, DisplayRowKind::FoldPlaceholder { .. }) {
                        format!("› {label}")
                    } else {
                        label
                    },
                ));
            if !continuation {
                if let Some(marker) = fold_marker_for_line(&fold_candidates, &self.folds, line) {
                    let fold_view = cx.entity();
                    let fold_label = if marker.folded { "▸" } else { "▾" };
                    let fold_tooltip = if marker.folded {
                        "Expand folded region"
                    } else {
                        "Fold region"
                    };
                    row = row.child(
                        button(
                            ("editor-fold-marker", line),
                            ui_palette,
                            ButtonVariant::Ghost,
                            ButtonSize::IconXs,
                        )
                        .tab_index(0)
                        .tooltip(move |window, cx| Tooltip::new(fold_tooltip).build(window, cx))
                        .child(fold_label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_, _: &MouseDownEvent, _window, cx| cx.stop_propagation()),
                        )
                        .on_click(move |_, _, cx| {
                            fold_view.update(cx, |this, cx| {
                                this.toggle_fold_range(marker.range, cx);
                            });
                        }),
                    );
                }
            }
            if let Some(decoration) = git_marker {
                let marker_color = match decoration.status {
                    GitHunkStatus::Added | GitHunkStatus::Untracked => accent,
                    GitHunkStatus::Modified => accent.opacity(0.75),
                    GitHunkStatus::Deleted => accent.opacity(0.45),
                };
                row = row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .w(px(3.0))
                        .h(px(line_h))
                        .bg(marker_color),
                );
                let hunk_id = decoration.hunk_id.clone();
                let revision = snapshot.revision();
                row = row.on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                        this.git_context_menu = Some((ev.position, hunk_id.clone(), revision));
                        this.git_discard_confirmation = None;
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
            }
            row
        });

        // Text + selection rows.
        let text_rows = display_rows.iter().map(|display_row| {
            let line = display_row.logical_line();
            let row_top = (display_row.visual_row.saturating_sub(first_visual) as f32) * line_h;
            let content = display_row.text(&snapshot);
            let mut row = div()
                .absolute()
                .top(px(row_top))
                .left(px(0.0))
                .h(px(line_h))
                .flex()
                .items_start()
                .text_size(px(font_px))
                .line_height(px(line_h))
                .font_family(editor_font_family.clone())
                .text_color(fg);
            row = if wrapping {
                row.w(px(content_w))
            } else {
                row.items_center().whitespace_nowrap()
            };
            row = row.on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, ev: &MouseDownEvent, _window, cx| {
                    this.text_context_menu = Some(ev.position);
                    this.git_context_menu = None;
                    this.git_discard_confirmation = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            );

            // Selection and search highlights are painted in display columns,
            // so wrapped rows and horizontal scrolling share one coordinate system.
            if let DisplayRowKind::Text {
                start_column,
                end_column,
                ..
            } = display_row.kind
            {
                if let Some((s, e)) = sel {
                    let start = if line == s.line { s.column } else { 0 };
                    let end = if line == e.line { e.column } else { snapshot.line_len(line) };
                    let lo = start.max(start_column);
                    let hi = end.min(end_column);
                    if hi > lo {
                        row = row.child(
                            div()
                                .absolute()
                                .left(px((lo - start_column - display.viewport.left_column) as f32 * char_w))
                                .top_0()
                                .w(px(((hi - lo) as f32 * char_w).max(2.0)))
                                .h(px(line_h))
                                .bg(accent.opacity(0.3)),
                        );
                    }
                }
                if let Some(search) = &self.search {
                    for matched in &search.matches {
                        if matched.start.line != line || matched.end.line != line {
                            continue;
                        }
                        let lo = matched.start.column.max(start_column);
                        let hi = matched.end.column.min(end_column);
                        if hi > lo {
                            row = row.child(
                                div()
                                    .absolute()
                                    .left(px((lo - start_column - display.viewport.left_column) as f32 * char_w))
                                    .top_0()
                                    .w(px(((hi - lo) as f32 * char_w).max(2.0)))
                                    .h(px(line_h))
                                    .bg(accent.opacity(if search.active_match_is(*matched) { 0.45 } else { 0.18 })),
                            );
                        }
                    }
                }
                for token in display.whitespace_tokens(&snapshot, display_row) {
                    let marker_column = snapshot.byte_to_position(token.range.start).column;
                    if marker_column < start_column || marker_column >= end_column {
                        continue;
                    }
                    row = row.child(
                        div()
                            .absolute()
                            .left(px((marker_column - start_column) as f32 * char_w + 2.0))
                            .top(px(line_h * 0.25))
                            .text_size(px((font_px * 0.55).max(5.0)))
                            .text_color(muted.opacity(0.7))
                            .child(SharedString::from(token.marker.to_string())),
                    );
                }
            }

            // Syntax-highlighted line text.
            let full_line = matches!(
                display_row.kind,
                DisplayRowKind::Text { start_column: 0, end_column, .. } if end_column == snapshot.line_len(line)
            );
            let runs = full_line.then(|| self.syntax.line_runs(&snapshot, line));
            let mut highlights: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
            let mut byte = 0usize;
            for run in runs.as_deref().unwrap_or_default() {
                let len = run.text.len();
                if let Some(kind) = run.kind {
                    highlights.push((
                        byte..byte + len,
                        HighlightStyle {
                            color: Some(palette.color(kind)),
                            ..Default::default()
                        },
                    ));
                }
                byte += len;
            }
            let text = if highlights.is_empty() {
                div().child(SharedString::from(content)).into_any_element()
            } else {
                StyledText::new(SharedString::from(content))
                    .with_highlights(highlights)
                    .into_any_element()
            };
            let text = if wrapping {
                div()
                    .w(px(content_w))
                    .line_height(px(line_h))
                    .child(text)
                    .into_any_element()
            } else {
                text
            };
            let row_start = match display_row.kind {
                DisplayRowKind::Text { start_column, .. } => start_column,
                DisplayRowKind::FoldPlaceholder { .. } => 0,
            };
            for token in semantic_decorations
                .iter()
                .filter(|token| token.visual_row == display_row.visual_row)
            {
                let token_color = if token.token_type.contains("type") {
                    ui_palette.info
                } else if token.token_type.contains("function") {
                    ui_palette.warning
                } else {
                    ui_palette.selected_accent
                };
                row = row.child(
                    div()
                        .absolute()
                        .left(px(token.start_column as f32 * char_w))
                        .bottom_0()
                        .w(px(
                            token
                                .end_column
                                .saturating_sub(token.start_column)
                                .max(1) as f32
                                * char_w,
                        ))
                        .h(px(1.0))
                        .bg(token_color.opacity(0.65)),
                );
            }
            for diagnostic in diagnostic_decorations
                .iter()
                .filter(|diagnostic| diagnostic.visual_row == display_row.visual_row)
            {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(
                            (diagnostic.start_column
                                .saturating_sub(row_start)
                                .saturating_sub(display.viewport.left_column)) as f32
                                * char_w,
                        ))
                        .bottom_0()
                        .w(px(
                            (diagnostic
                                .end_column
                                .saturating_sub(diagnostic.start_column)
                                .max(1) as f32)
                                * char_w,
                        ))
                        .h(px(1.0))
                        .bg(accent.opacity(0.85)),
                );
            }
            for decoration in git_inline_decorations
                .iter()
                .filter(|decoration| decoration.visual_row == display_row.visual_row)
            {
                let color = match decoration.kind {
                    labonair_editor::git::GitInlineChangeKind::Inserted => accent.opacity(0.55),
                    labonair_editor::git::GitInlineChangeKind::Replaced => accent.opacity(0.7),
                    labonair_editor::git::GitInlineChangeKind::Deleted => accent.opacity(0.45),
                };
                let width = if decoration.start_column == decoration.end_column {
                    3.0
                } else {
                    ((decoration.end_column - decoration.start_column) as f32 * char_w).max(3.0)
                };
                row = row.child(
                    div()
                        .absolute()
                        .left(px(decoration.start_column as f32 * char_w))
                        .bottom_0()
                        .w(px(width))
                        .h(px(2.0))
                        .bg(color),
                );
            }
            row.child(text)
        });

        // Caret + current-line band use the same display map as mouse mapping.
        self.editor_focused = self.focus_handle.is_focused(window);
        let caret_point = display.position_to_display(cursor);
        let show_caret =
            !self.prefs.cursor_blink() || !self.editor_focused || self.cursor_blink_on;
        let caret = show_caret
            .then_some(caret_point)
            .flatten()
            .map(|(visual_row, column)| {
                use labonair_settings::content::editor::EditorCursorStyle;
                let (top, width, height, bg_color) = match self.prefs.cursor_style() {
                    EditorCursorStyle::Bar => (0.0, 2.0, line_h, accent),
                    EditorCursorStyle::Underline => {
                        ((line_h - 2.0).max(0.0), char_w.max(2.0), 2.0, accent)
                    }
                    EditorCursorStyle::Block => {
                        (0.0, char_w.max(2.0), line_h, accent.opacity(0.6))
                    }
                };
                div()
                    .absolute()
                    .top(px(
                        (visual_row.saturating_sub(first_visual) as f32) * line_h + top,
                    ))
                    .left(px(column as f32 * char_w))
                    .w(px(width))
                    .h(px(height))
                    .bg(bg_color)
            });

        let current_line = self
            .prefs
            .highlight_current_line()
            .then(|| {
                caret_point.map(|(visual_row, _)| {
                    div()
                        .absolute()
                        .top(px((visual_row.saturating_sub(first_visual) as f32) * line_h))
                        .left(px(0.0))
                        .right(px(0.0))
                        .h(px(line_h))
                        .bg(fg.opacity(0.04))
                })
            })
            .flatten();

        let weak = cx.weak_entity();
        let probe = canvas(
            move |bounds, _window, cx| {
                let _ = weak.update(cx, |this, cx| {
                    if this.bounds != Some(bounds) {
                        this.bounds = Some(bounds);
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let _ = window;

        let mut text_area = div()
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(probe)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(gutter_bg)
                    .w(px(gutter_width))
                    .children(gutter),
            )
            .child(
                div()
                    .absolute()
                    .top(px(sticky_height))
                    .bottom_0()
                    .left(px(gutter_width))
                    .right_0()
                    .overflow_hidden()
                    .children(current_line)
                    .children(text_rows)
                    .children(caret),
            )
            .child(self.render_scrollbars(&display, ui_palette, sticky_height, cx));
        if snapshot.byte_len() == 0 {
            text_area = text_area.child(
                div()
                    .absolute()
                    .top(px(sticky_height + 12.0))
                    .left(px(gutter_width + 12.0))
                    .text_xs()
                    .text_color(ui_palette.muted)
                    .child("Empty file · start typing"),
            );
        }
        for column in &ruler_columns {
            text_area = text_area.child(
                div()
                    .absolute()
                    .top(px(sticky_height))
                    .bottom_0()
                    .left(px(
                        (*column as f32 - display.viewport.left_column as f32) * char_w
                    ))
                    .w(px(1.0))
                    .bg(ui_palette.border.opacity(0.7)),
            );
        }
        if let Some(label) = sticky_context {
            text_area = text_area.child(
                div()
                    .absolute()
                    .top_0()
                    .left(px(gutter_width))
                    .right_0()
                    .h(px(sticky_height))
                    .flex()
                    .items_center()
                    .px_2()
                    .bg(ui_palette.card.opacity(0.96))
                    .border_b_1()
                    .border_color(ui_palette.border)
                    .text_xs()
                    .text_color(ui_palette.muted)
                    .font_family(editor_font_family.clone())
                    .child(SharedString::from(label)),
            );
        }

        let split_count = self.editor_splits.group_count();
        let focused_group = self.editor_splits.focused_group();
        let editor_surface = if split_count == 1 {
            text_area.into_any_element()
        } else {
            let mut active_text_area = Some(text_area.into_any_element());
            self.render_split_node(
                self.editor_splits.root(),
                &labonair_editor::splits::SplitPath::root(),
                focused_group,
                &mut active_text_area,
                &snapshot,
                &display,
                font,
                ui_palette,
                cx,
            )
        };

        let file_status = self.render_file_status(cx);
        let show_text_area = self.should_render_text_area();
        let minimap = self.prefs.minimap();
        let mut buffer_surface = div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0();
        buffer_surface =
            buffer_surface.child(self.render_breadcrumb(breadcrumb_state, ui_palette, cx));
        if show_text_area {
            let mut editor_surface_container = div().relative().flex_1().min_w_0().min_h_0();
            if minimap {
                editor_surface_container = editor_surface_container.pr(px(76.0));
            }
            editor_surface_container = editor_surface_container.child(editor_surface);
            if minimap {
                editor_surface_container =
                    editor_surface_container.child(self.render_minimap(&snapshot, ui_palette));
            }
            buffer_surface = buffer_surface.child(editor_surface_container);
        }
        if self.prefs.show_outline() {
            buffer_surface =
                buffer_surface.child(self.render_outline(&outline_symbols, ui_palette, cx));
        }

        div()
            .id("editor")
            .track_focus(&self.focus_handle)
            .key_context("Editor")
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .bg(bg)
            .when(matches!(self.doc.file_state(), FileState::Conflict), |d| {
                d.child(self.render_conflict_banner(cx))
            })
            .children(self.render_recovery_banner(cx))
            .children(file_status)
            .children(self.render_language_service_status(cx))
            .child(buffer_surface)
            .when(self.vim.is_some(), |d| d.child(self.render_vim_status(cx)))
            .when(
                self.prefs.show_cursor_position() || self.prefs.show_selection_stats(),
                |d| d.child(self.render_editor_status(cx)),
            )
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    window.focus(&this.focus_handle);
                    let pos = this.position_at(ev.position);
                    this.doc.set_caret(pos, ev.modifiers.shift);
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _window, cx| {
                this.update_hover(ev.position, cx);
                this.schedule_language_hover(this.position_at(ev.position), cx);
            }))
            .children(self.render_language_service_overlay(cx))
            .children(self.render_signature_help(cx))
            .children(
                rename_input
                    .as_ref()
                    .map(|input| self.render_rename_dialog(input, ui_palette, cx)),
            )
            .children(self.render_hover_tooltip(cx))
            .children(self.render_git_context_menu(cx))
            .children(self.render_text_context_menu(cx))
            .children(self.render_git_discard_confirmation(cx))
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, window, cx| {
                let dy = match ev.delta {
                    gpui::ScrollDelta::Lines(p) => p.y,
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y) / this.metrics.1,
                };
                let _ = window;
                if dy.abs() >= 0.01 {
                    this.scroll_by(-(dy.round() as isize).clamp(-8, 8), cx);
                }
            }))
    }
}

/// Soft-wrap geometry (T06-005). `cols == 0` disables wrapping; otherwise a
/// logical line is broken every `cols` characters (character grid, like a
/// monospace terminal — no word-boundary breaking, matching CodeMirror's
/// default `lineWrapping` for code).
#[derive(Clone, Copy)]
struct Wrap {
    cols: usize,
}

impl Wrap {
    /// Visual-row count for a logical line of `len` characters (at least 1).
    fn rows(&self, len: usize) -> usize {
        if self.cols == 0 {
            return 1;
        }
        len.max(1).div_ceil(self.cols)
    }
}

/// A GPUI keystroke that produces a single printable character, if any.
fn printable(ks: &gpui::Keystroke) -> Option<String> {
    let text = ks.key_char.clone()?;
    if text.chars().any(|c| c.is_control()) || text.is_empty() {
        return None;
    }
    Some(text)
}

fn notify(cx: &mut App, title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    notification_center(cx).update(cx, |center, cx| {
        center.push(
            Notification::error(title.clone(), "The editor operation failed.")
                .source("editor")
                .details(body.clone())
                .dedupe_key(format!("editor:{title}:{body}")),
            cx,
        );
    });
}

fn notify_info(cx: &mut App, title: &str, body: &str) {
    let (title, body) = (title.to_string(), body.to_string());
    notification_center(cx).update(cx, |center, cx| {
        center.push(
            Notification::info(title.clone(), body.clone())
                .source("editor")
                .details(body.clone())
                .dedupe_key(format!("editor:info:{title}:{body}")),
            cx,
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext};
    use labonair_editor::SearchOptions;
    use labonair_settings::content::editor::EditorContent;
    use labonair_settings::{SettingsContent, SettingsLayer, SettingsStore};

    /// Build an [`EditorSettings`] with `f` applied to a default `editor` area.
    fn editor_prefs(f: impl FnOnce(&mut EditorContent)) -> EditorSettings {
        let mut c = SettingsContent::default();
        f(&mut c.editor);
        EditorSettings::from_settings(&c)
    }

    /// Install a `SettingsStore` global whose `User` layer carries `f`'s edits,
    /// so `EditorView::apply_prefs` (which reads the global) sees them.
    fn install_editor_prefs(cx: &mut App, f: impl FnOnce(&mut EditorContent)) {
        let mut c = SettingsContent::default();
        f(&mut c.editor);
        let mut store = SettingsStore::new(std::path::PathBuf::from(
            "/tmp/labonair-editor-tests-unused/config.json",
        ));
        store.register_setting::<EditorSettings>();
        store.set_layer(SettingsLayer::User, c);
        cx.set_global(store);
    }

    fn setup(cx: &mut TestAppContext) -> Entity<EditorView> {
        cx.update(|cx| {
            let theme = cx.new(|_| crate::theme::ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| EditorView::new(theme, cx))
        })
    }

    #[test]
    fn completion_selection_moves_and_wraps() {
        assert_eq!(next_completion_selection(0, 3, -1), 2);
        assert_eq!(next_completion_selection(2, 3, 1), 0);
        assert_eq!(next_completion_selection(99, 3, -1), 1);
        assert_eq!(next_completion_selection(4, 0, 1), 0);
    }

    #[test]
    fn signature_help_render_requires_current_revision_generation_and_cursor() {
        let mut state = LanguageServiceState::new(DocumentVersion::new(3));
        state.set_document_generation(DocumentVersion::new(3), 7);
        let snapshot = state.snapshot();
        let position = LspPosition {
            line: 2,
            character: 4,
        };
        assert!(signature_help_render_eligible(
            &snapshot,
            &SignatureHelpUiState::Loading,
            Some(position),
            position,
            DocumentVersion::new(3),
            7,
        ));
        assert!(!signature_help_render_eligible(
            &snapshot,
            &SignatureHelpUiState::Hidden,
            Some(position),
            position,
            DocumentVersion::new(3),
            7,
        ));
        assert!(!signature_help_render_eligible(
            &snapshot,
            &SignatureHelpUiState::Ready,
            Some(position),
            LspPosition {
                line: 2,
                character: 5,
            },
            DocumentVersion::new(3),
            7,
        ));
        assert!(!signature_help_render_eligible(
            &snapshot,
            &SignatureHelpUiState::Ready,
            Some(position),
            position,
            DocumentVersion::new(2),
            7,
        ));
    }

    #[gpui::test]
    fn completion_accepts_selected_item_and_cancel_clears_selection(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("completion.txt".into(), "pr", 1);
                v.doc.move_caret(Motion::DocEnd, false);
                v.language_service_generation = 1;
                let version = DocumentVersion::from_revision(v.editor_revision());
                v.language_service_state.set_document_generation(version, 1);
                assert!(v.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test"),
                    version,
                    generation: 1,
                    response: Ok(LanguageServiceResponse::Completion(
                        labonair_editor::CompletionList {
                            items: vec![
                                labonair_editor::CompletionItem {
                                    label: "print".into(),
                                    kind: labonair_editor::CompletionKind::Function,
                                    detail: None,
                                    insert_text: "print".into(),
                                    filter_text: None,
                                    sort_text: None,
                                    documentation: None,
                                },
                                labonair_editor::CompletionItem {
                                    label: "println".into(),
                                    kind: labonair_editor::CompletionKind::Function,
                                    detail: None,
                                    insert_text: "println".into(),
                                    filter_text: None,
                                    sort_text: None,
                                    documentation: None,
                                },
                            ],
                            is_incomplete: false,
                        },
                    )),
                }));
                v.completion_visible = true;
                v.completion_position = Some(v.current_lsp_position());
                v.completion_selected = 1;
                v.apply_completion_item(v.completion_selected, cx);
                assert_eq!(v.doc.text(), "println");

                v.completion_visible = true;
                v.completion_selected = 1;
                v.dismiss_completion();
                assert!(!v.completion_visible);
                assert_eq!(v.completion_selected, 0);
                assert!(v.completion_position.is_none());
            });
        });
    }

    #[gpui::test]
    fn edits_set_dirty_and_emit(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "abc", 1);
                v.doc.move_caret(Motion::DocEnd, false);
                v.edit(cx, |d| d.insert("d"));
                assert!(v.is_dirty());
                assert_eq!(v.doc.text(), "abcd");
            });
        });
    }

    #[test]
    fn unsupported_loads_never_create_editable_placeholder_content() {
        let path = PathBuf::from("asset.bin");
        let binary = EditorLifecycleLoad::Binary {
            identity: StorageFileIdentity::from_bytes(path.clone(), 1, b"binary"),
            capabilities: StorageFileCapabilities {
                can_read: true,
                can_edit: true,
                can_save: true,
            },
        };
        let document = EditorView::document_from_load(path.clone(), Ok(binary));
        assert_eq!(document.file_state(), &FileState::Binary);
        assert!(document.text().is_empty());
        assert!(!document.is_dirty());

        let oversized = EditorLifecycleLoad::TooLarge {
            identity: StorageFileIdentity::from_bytes(path.clone(), 1, b"large"),
            size: 11,
            limit: 10,
            capabilities: StorageFileCapabilities {
                can_read: true,
                can_edit: true,
                can_save: true,
            },
        };
        let document = EditorView::document_from_load(path, Ok(oversized));
        assert_eq!(document.file_state(), &FileState::TooLarge);
        assert!(document.text().is_empty());
        assert!(!document.is_dirty());
    }

    #[test]
    fn read_only_text_keeps_content_but_cannot_be_edited_or_saved() {
        let path = PathBuf::from("README.txt");
        let snapshot = StorageFileSnapshot {
            identity: StorageFileIdentity::from_bytes(path, 1, b"protected"),
            text: "protected".to_string(),
            line_ending: StorageLineEnding::Lf,
            encoding: labonair_filesystem::file::StorageEncoding::Utf8,
            bom: StorageBom::None,
            capabilities: StorageFileCapabilities {
                can_read: true,
                can_edit: false,
                can_save: false,
            },
        };
        let document = EditorView::document_from_load(
            PathBuf::from("README.txt"),
            Ok(EditorLifecycleLoad::Text { snapshot }),
        );
        assert_eq!(document.file_state(), &FileState::ReadOnly);
        assert_eq!(document.text(), "protected");
        assert!(!document.is_dirty());
        assert!(!document.file_capabilities().unwrap().can_edit);
    }

    #[test]
    fn missing_load_has_no_placeholder_text() {
        let document = EditorView::document_from_load(
            PathBuf::from("missing.txt"),
            Err(EditorFileError::Missing {
                path: PathBuf::from("missing.txt"),
            }),
        );
        assert_eq!(document.file_state(), &FileState::Missing);
        assert!(document.text().is_empty());
        assert!(!document.is_dirty());
    }

    #[test]
    fn clean_external_change_reloads_through_typed_lifecycle() {
        let mut document = Document::from_file(PathBuf::from("notes.txt"), "before", 1);
        let changed_identity =
            labonair_editor::lifecycle::FileIdentity::from_bytes("notes.txt", 2, b"after");
        assert!(matches!(
            document.observe_file(Some(changed_identity)),
            Ok(Some(FileLifecycleEvent::ReloadNeeded { .. }))
        ));
        assert_eq!(document.file_state(), &FileState::ReloadNeeded);

        let snapshot = labonair_editor::lifecycle::FileSnapshot::new(
            labonair_editor::lifecycle::FileIdentity::from_bytes("notes.txt", 2, b"after"),
            "after".to_string(),
            labonair_editor::lifecycle::LineEnding::Lf,
            labonair_editor::lifecycle::Encoding::Utf8,
            labonair_editor::lifecycle::Bom::None,
            FileCapabilities::editable(),
        );
        document
            .reload_snapshot(snapshot, ReloadDecision::Automatic)
            .unwrap();
        assert_eq!(document.file_state(), &FileState::Clean);
        assert_eq!(document.text(), "after");
    }

    #[test]
    fn dirty_external_change_enters_conflict_and_keep_preserves_buffer() {
        let mut document = Document::from_file(PathBuf::from("notes.txt"), "before", 1);
        document.set_caret(Position::new(0, 6), false);
        document.insert(" locally");
        let local_text = document.text();
        let changed_identity =
            labonair_editor::lifecycle::FileIdentity::from_bytes("notes.txt", 2, b"on disk");
        assert!(matches!(
            document.observe_file(Some(changed_identity)),
            Ok(Some(FileLifecycleEvent::ConflictDetected { .. }))
        ));
        assert_eq!(document.file_state(), &FileState::Conflict);
        document.keep_buffer().unwrap();
        assert_eq!(document.file_state(), &FileState::Dirty);
        assert_eq!(document.text(), local_text);
    }

    #[test]
    fn normal_save_preflight_refuses_an_external_identity() {
        let mut document = Document::from_file(PathBuf::from("notes.txt"), "before", 1);
        document.set_caret(Position::new(0, 6), false);
        document.insert(" locally");
        let current_identity =
            labonair_editor::lifecycle::FileIdentity::from_bytes("notes.txt", 2, b"on disk");
        assert!(matches!(
            document.prepare_save(&current_identity, SaveIntent::Normal),
            Err(labonair_editor::lifecycle::FileLifecycleError::ExternalChange { .. })
        ));
        document
            .prepare_save(&current_identity, SaveIntent::OverwriteExternal)
            .unwrap();
    }

    #[gpui::test]
    fn find_navigates_matches(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "x y x y x", 1);
                let (cur, total) = v
                    .search_set(SearchQuery::new("x", Default::default()), cx)
                    .unwrap();
                assert_eq!(total, 3);
                assert!(cur >= 1);
                let (cur2, _) = v.search_step(true, cx).unwrap();
                assert_ne!(cur, cur2);
                assert!(v.doc.selection().is_some());
            });
        });
    }

    #[gpui::test]
    fn search_replace_actions_use_typed_state_and_one_undo_step(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "one two\none three", 1);
                let query = SearchQuery::new(
                    "(one) (\\w+)",
                    SearchOptions {
                        regex: true,
                        multiline: true,
                        ..Default::default()
                    },
                );
                assert_eq!(v.search_set(query, cx).unwrap(), (1, 2));
                assert!(v.search_replace_one("$2-$1", cx).unwrap());
                assert_eq!(v.doc.text(), "two-one\none three");
                assert_eq!(v.search_replace_all("$2/$1", cx).unwrap(), 1);
                assert_eq!(v.doc.text(), "two-one\nthree/one");
                v.doc.undo();
                assert_eq!(v.doc.text(), "two-one\none three");
            });
        });
    }

    #[gpui::test]
    fn invalid_editor_regex_is_an_inline_error_and_disables_actions(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "text", 1);
                let query = SearchQuery::new(
                    "[",
                    SearchOptions {
                        regex: true,
                        ..Default::default()
                    },
                );
                assert!(v.search_set(query, cx).is_err());
                assert!(v.search_error().is_some());
                assert_eq!(v.search_count(), (0, 0));
                assert!(!v.search_can_replace());
            });
        });
    }

    #[gpui::test]
    fn live_prefs_toggle_vim_and_indent(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            install_editor_prefs(cx, |e| {
                e.editor_indent_with_tabs = Some(false);
                e.editor_tab_size = Some(3);
                e.editor_vim_mode = Some(true);
            });
        });
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.apply_prefs(cx);
                assert!(v.vim.is_some(), "vim enabled via prefs");
                assert_eq!(v.indent_unit(), "   ");
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |e| {
                e.editor_vim_mode = Some(false);
                e.editor_indent_with_tabs = Some(true);
            });
        });
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.apply_prefs(cx);
                assert!(v.vim.is_none(), "vim disabled via prefs");
                assert_eq!(v.indent_unit(), "\t");
            });
        });
    }

    #[test]
    fn wrap_rows_math() {
        let w = Wrap { cols: 10 };
        assert_eq!(w.rows(0), 1);
        assert_eq!(w.rows(1), 1);
        assert_eq!(w.rows(10), 1);
        assert_eq!(w.rows(11), 2);
        assert_eq!(w.rows(25), 3);
        assert_eq!(Wrap { cols: 0 }.rows(999), 1, "wrap off = one row");
    }

    #[test]
    fn scrollbar_thumb_math_clamps_to_track_and_scroll_range() {
        let geometry = scrollbar_geometry(100, 10, usize::MAX, 100.0).expect("overflow");
        assert_eq!(geometry.max_scroll, 90);
        assert_eq!(geometry.thumb_length, EDITOR_SCROLLBAR_MIN_THUMB);
        assert_eq!(geometry.thumb_offset, 76.0);
        assert_eq!(scrollbar_scroll_from_pointer(-20.0, 0.0, geometry), 0);
        assert_eq!(scrollbar_scroll_from_pointer(100.0, 0.0, geometry), 90);
        assert_eq!(scrollbar_scroll_from_pointer(50.0, 0.0, geometry), 45);
        assert!(scrollbar_geometry(10, 10, 0, 100.0).is_none());
    }

    #[test]
    fn fold_toggle_is_idempotent_and_keeps_ranges_ordered() {
        let first = FoldRange::new(8, 12).expect("valid fold");
        let second = FoldRange::new(2, 5).expect("valid fold");
        let mut folds = Vec::new();
        assert!(toggle_fold(&mut folds, first));
        assert!(toggle_fold(&mut folds, second));
        assert_eq!(folds, vec![second, first]);
        assert!(!toggle_fold(&mut folds, first));
        assert_eq!(folds, vec![second]);
        assert!(toggle_fold(&mut folds, first));
        assert_eq!(folds, vec![second, first]);
    }

    #[test]
    fn git_word_diff_setting_only_controls_inline_decorations() {
        let buffer = labonair_editor::EditorBuffer::from_text("let value = 2;\n");
        let buffer_snapshot = buffer.snapshot();
        let display = DisplayMap::build(
            &buffer_snapshot,
            Viewport {
                height_rows: 8,
                ..Default::default()
            },
            DisplayConfig::default(),
            &[],
        );
        let mut git = GitGutterSnapshot::empty("src/lib.rs", buffer_snapshot.revision());
        git.inline_decorations
            .push(labonair_editor::git::GitInlineDecoration {
                line: 0,
                start_column: 12,
                end_column: 13,
                kind: labonair_editor::git::GitInlineChangeKind::Replaced,
                source_text: "2".into(),
                hunk_id: "hunk-1".into(),
            });
        git.decorations.push(labonair_editor::GitLineDecoration {
            line: 0,
            status: labonair_editor::GitHunkStatus::Modified,
            hunk_id: "hunk-1".into(),
        });

        let enabled = editor_prefs(|editor| editor.editor_git_word_diff = Some(true));
        assert_eq!(
            git_inline_decorations_for_view(&enabled, &git, &buffer_snapshot, &display).len(),
            1
        );

        let disabled = editor_prefs(|editor| editor.editor_git_word_diff = Some(false));
        assert!(
            git_inline_decorations_for_view(&disabled, &git, &buffer_snapshot, &display).is_empty()
        );
        assert_eq!(
            git.decorations_for_revision(buffer_snapshot.revision())
                .len(),
            1,
            "the line-level Git projection remains available independently"
        );
    }

    #[gpui::test]
    fn soft_wrap_navigation_crosses_visual_rows(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                // 45 visible columns: (408 - 48 gutter) / 8.
                v.metrics = (8.0, 18.0);
                v.gutter_width = 48.0;
                v.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(408.0),
                        height: px(360.0),
                    },
                });
                v.prefs = editor_prefs(|e| e.editor_word_wrap = Some(true));
                assert_eq!(v.wrap_cols(), 45);

                let long: String = "x".repeat(120);
                v.doc = Document::from_file("t.txt".into(), &long, 1);
                v.doc.set_caret(Position::new(0, 10), false);

                assert!(v.wrap_vertical(true, false, cx));
                assert_eq!(v.doc.cursor.column, 55, "down one visual row = +cols");
                assert!(v.wrap_vertical(true, false, cx));
                assert_eq!(v.doc.cursor.column, 100);
                assert!(v.wrap_vertical(false, false, cx));
                assert_eq!(v.doc.cursor.column, 55, "up one visual row = -cols");

                assert!(v.wrap_horizontal(false, false, cx));
                assert_eq!(v.doc.cursor.column, 45, "home = start of visual row");
                assert!(v.wrap_horizontal(true, false, cx));
                assert_eq!(v.doc.cursor.column, 90, "end = end of visual row");
            });
        });
    }

    #[gpui::test]
    fn soft_wrap_disabled_falls_back_to_logical(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.prefs = editor_prefs(|e| e.editor_word_wrap = Some(false));
                assert_eq!(v.wrap_cols(), 0);
                assert!(!v.wrap_vertical(true, false, cx));
                assert!(!v.wrap_horizontal(true, false, cx));
            });
        });
    }

    #[gpui::test]
    fn vim_mode_routes_keys_and_edits(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.doc = Document::from_file("t.txt".into(), "hello world", 1);
                v.vim = Some(Vim::default());
                // `dw` deletes the first word.
                v.handle_vim(VimKey::Char('d'), cx);
                v.handle_vim(VimKey::Char('w'), cx);
                assert_eq!(v.doc.text(), "world");
                assert_eq!(v.vim.as_ref().unwrap().mode(), VimMode::Normal);
                // `:q` asks the workspace to close the tab.
                for k in [VimKey::Char(':'), VimKey::Char('q'), VimKey::Enter] {
                    v.handle_vim(k, cx);
                }
            });
        });
    }
}
