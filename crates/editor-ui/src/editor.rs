//! GPUI code-editor view (T06-001).
//!
//! Renders and drives a [`labonair_editor::Document`]: viewport-based line
//! rendering with a line-number gutter, caret + selection, keyboard editing,
//! undo/redo, `Cmd-S` save (atomic write via
//! [`labonair_filesystem::file::save_editor_file_lifecycle_sync`]), a find /
//! replace bar (`Cmd-F`), and external-change detection with a reload banner.
//!
//! The view owns no file IO on the main thread — reads and writes run on
//! `cx.background_executor().spawn`. It emits [`EditorEvent`] for the hosting
//! Workspace to mirror the dirty flag and peek state onto its tab.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, px, relative, AnyElement, App, AppContext, Bounds, ClickEvent, ClipboardItem,
    Context, Entity, EventEmitter, FocusHandle, Focusable, HighlightStyle, InteractiveElement,
    IntoElement, KeyContext, KeyDownEvent, ListAlignment, ListState, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollWheelEvent,
    SharedString, StatefulInteractiveElement, Styled, StyledText, Task, Window,
};
use labonair_command_palette_core::CommandId;
use labonair_filesystem::file::{
    load_editor_file_lifecycle_with_encoding_sync, save_editor_file_lifecycle_sync,
    stat_editor_file_sync, EditorFileError, EditorFileStat, EditorLifecycleLoad, StorageBom,
    StorageEncoding, StorageFileCapabilities, StorageFileIdentity, StorageFileSnapshot,
    StorageLineEnding, StorageSaveIntent,
};
use labonair_settings::content::editor::{
    EditorDiagnosticMaxSeverity, EditorDocumentColorMode, EditorMinimapCurrentLineHighlight,
    EditorMinimapDisplayIn, EditorMinimapThumb, EditorMinimapThumbBorder, EditorMinimapVisibility,
    EditorModalMode, EditorScrollbarVisibility, SearchSeedBehavior,
};
use labonair_settings::{EditorSettings, Settings as _, SettingsStore};

use labonair_editor::{
    apply_text_policies,
    command_provider::{
        route_for_command, EditorCommandRoute, EditorLanguageCommand, EditorUiCommand,
    },
    document::Motion,
    document_syntax,
    editing::{EditIntent, EditorCommand, EditorCommandResult, SplitIntent},
    find,
    lifecycle::{
        FileCapabilities, FileLifecycleEvent, FileState, LineEnding, ReloadDecision, SaveIntent,
    },
    map_range_through_edits, next_match_with_wrap, replace_all_checked, replace_one,
    word_at_cursor, Anchor, AnchorAffinity, BufferSnapshot, CallHierarchyCall, CallHierarchyItem,
    CancellationToken, CodeLens, DisplayConfig, DisplayMap, DisplayRowKind, DisplaySnapshot,
    Document, DocumentColor, DocumentColorValue, DocumentLink, DocumentVersion, Edit,
    EditorEditability, EditorSessionId, EditorSessionSnapshot, EditorViewSession, FoldRange,
    FoldingRange, GitBlameRequest, GitBlameSnapshot, GitDecorationProvider, GitDecorationRequest,
    GitDiscardPreview, GitEditorCommand, GitGutterError, GitGutterSnapshot, GitHunkAction,
    GitHunkActionKind, GitHunkStatus, Helix, HelixKey, HelixMode, HunkActionSink, InlayHint,
    InlayHintKind, KillRing, Language, LanguageId, LanguageServiceCapabilities,
    LanguageServiceCommand, LanguageServiceEditRequest, LanguageServiceEditResponse,
    LanguageServiceRequest, LanguageServiceResponse, LanguageServiceResult, LanguageServiceRuntime,
    LanguageServiceRuntimeSnapshot, LanguageServiceRuntimeSource, LanguageServiceRuntimeStatus,
    LanguageServiceSnapshot, LanguageServiceState, LocalLanguageServiceRequest, LspPosition,
    LspRange, Match, NavigationHistory, NavigationLocation, Position, ProjectBufferCandidate,
    ProjectReplacePlan, ProjectReplaceRequest, ProjectReplaceResult, ProjectReplaceTarget,
    ProjectSearchHit, ProjectSearchQuery, ProjectSearchRequest, ProjectSearchResult,
    ProjectSearchSession, ProjectSearchSnapshot, Revision, SavePolicy, SearchError, SearchOptions,
    SearchQuery, Selection, SelectionSet, SnippetSession, SyntaxHighlighter, TextRange, Viewport,
    Vim, VimKey, VimMode, VimOptions, WhitespaceMode,
};
use labonair_editor::{build_breadcrumb_state, BreadcrumbState};

use crate::syntax_theme::EditorPalette;
use labonair_notifications::{notification_center, Notification};
use labonair_theme::{modal_scrim, ThemeStore};
use labonair_ui_kit::{
    banner, button, button_disabled, context_menu, icon_button_builder, popover_menu,
    select_popover, select_trigger, text_field, text_field_surface, text_input, ButtonSize,
    ButtonVariant, IconButtonShape, IconName, InputEvent, InputState, ListItem, MenuItem, Palette,
    SelectOption, SelectPopoverAnchor, Severity, SubmenuHoverSource, TextFieldState, Tooltip,
};

type CallHierarchyNavigator = Rc<dyn Fn(String, LspPosition, &mut Window, &mut App)>;
type DocumentLinkNavigator = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// Editor → workspace notifications.
#[derive(Clone, Debug)]
pub enum EditorEvent {
    /// Dirty flag or title-relevant state changed — re-sync the tab.
    Changed,
    /// The user made their first edit — a peek tab should become permanent.
    Edited,
    /// A Markdown preview should open for this exact immutable source revision.
    MarkdownPreviewRequested(EditorBufferSnapshot),
    /// A following Markdown preview should bind to this Editor tab and revision.
    MarkdownFollowingPreviewRequested(EditorBufferSnapshot),
    /// An SVG preview should open for this file path in the Workspace preview host.
    SvgPreviewRequested(PathBuf),
    /// The content or file identity of an editor buffer changed.
    BufferChanged(EditorBufferSnapshot),
    /// Vim `:q` / `:wq` — the hosting workspace should close this editor's tab.
    CloseRequested,
}

/// Immutable source data crossing from the Editor owner into its Workspace
/// preview host. The rope snapshot is cheap to clone and remains revisioned.
#[derive(Clone, Debug)]
pub struct EditorBufferSnapshot {
    pub path: Option<PathBuf>,
    pub language: Language,
    pub snapshot: BufferSnapshot,
}

/// Editor-owned query and match state shared by the workspace search overlay
/// and the editor toolbar's local buffer-search row. The active match is
/// shown as the editor selection.
#[derive(Default)]
struct EditorSearch {
    query: SearchQuery,
    selection_seed: Option<TextRange>,
    matches: Vec<Match>,
    active: Option<usize>,
    error: Option<SearchError>,
}

#[derive(Clone, Debug)]
struct EditorSplitDrag {
    path: labonair_editor::splits::SplitPath,
    orientation: labonair_editor::SplitOrientation,
}

/// A pointer gesture keeps its original selection state so a drag is one
/// selection-history step regardless of how many mouse-move events it emits.
#[derive(Clone, Debug)]
struct EditorSelectionDrag {
    previous: SelectionSet,
    preserved: Vec<Selection>,
    anchor: Anchor,
    columnar: bool,
    origin_visual_row: usize,
    origin_display_column: usize,
}

const EDITOR_SCROLLBAR_SIZE: f32 = 10.0;
const EDITOR_SCROLLBAR_MIN_THUMB: f32 = 24.0;
const EDITOR_BREADCRUMB_MAX_PATH_CHARS: usize = 96;
type CharacterPaletteEntry = (&'static str, &'static str);
type CharacterPaletteGroup = (&'static str, &'static str, &'static [CharacterPaletteEntry]);

const CHARACTER_PALETTE_GROUPS: &[CharacterPaletteGroup] = &[
    (
        "common",
        "Common Symbols",
        &[
            ("©", "Copyright sign"),
            ("®", "Registered sign"),
            ("™", "Trademark sign"),
            ("§", "Section sign"),
            ("¶", "Paragraph sign"),
            ("†", "Dagger"),
            ("‡", "Double dagger"),
            ("•", "Bullet"),
            ("…", "Ellipsis"),
            ("—", "Em dash"),
            ("–", "En dash"),
            ("·", "Middle dot"),
        ],
    ),
    (
        "math",
        "Mathematics",
        &[
            ("±", "Plus-minus"),
            ("×", "Multiplication sign"),
            ("÷", "Division sign"),
            ("≠", "Not equal to"),
            ("≤", "Less than or equal to"),
            ("≥", "Greater than or equal to"),
            ("∞", "Infinity"),
            ("√", "Square root"),
            ("∑", "Summation"),
            ("∏", "Product"),
        ],
    ),
    (
        "arrows",
        "Arrows",
        &[
            ("←", "Left arrow"),
            ("→", "Right arrow"),
            ("↑", "Up arrow"),
            ("↓", "Down arrow"),
            ("↔", "Left-right arrow"),
            ("⇒", "Right double arrow"),
            ("⇐", "Left double arrow"),
            ("↦", "Maps to"),
        ],
    ),
    (
        "greek",
        "Greek",
        &[
            ("α", "Alpha"),
            ("β", "Beta"),
            ("γ", "Gamma"),
            ("δ", "Delta"),
            ("λ", "Lambda"),
            ("π", "Pi"),
            ("σ", "Sigma"),
            ("Ω", "Omega"),
        ],
    ),
];

fn editor_toolbar_has_contribution(
    breadcrumbs: bool,
    quick_actions: bool,
    selections_menu: bool,
    code_actions: bool,
    markdown_preview: bool,
) -> bool {
    breadcrumbs || quick_actions || selections_menu || code_actions || markdown_preview
}

/// The divider's cursor communicates the resize operation; the drag payload
/// itself intentionally renders no visual content.
struct EditorDragGhost;

impl Render for EditorDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

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

#[derive(Clone, Copy, Debug)]
struct EditorMinimapDrag {
    total_rows: usize,
    viewport_rows: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct MinimapLineMarkers {
    cursor: bool,
    selection: bool,
    search: bool,
    selected_text_occurrence: bool,
    selected_symbol_occurrence: bool,
    diagnostic: Option<labonair_editor::language_services::DiagnosticSeverity>,
    git: Option<GitHunkStatus>,
}

#[derive(Clone, Debug)]
struct MinimapOccurrenceCache {
    revision: Revision,
    line_count: usize,
    slot_count: usize,
    selected_text: Option<String>,
    symbol: Option<String>,
    selected_text_slots: Vec<usize>,
    symbol_slots: Vec<usize>,
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

fn centered_search_scroll_line(
    layout: &DisplaySnapshot,
    position: Position,
    visible_rows: usize,
) -> Option<usize> {
    let (match_row, _) = layout.position_to_display(position)?;
    let visible_rows = visible_rows.max(1);
    let max_top_row = layout.total_rows.saturating_sub(visible_rows);
    let top_row = match_row.saturating_sub(visible_rows / 2).min(max_top_row);
    layout
        .display_to_position(top_row, 0)
        .map(|position| position.line)
}

fn minimap_slot(line: usize, line_count: usize, slot_count: usize) -> Option<usize> {
    if line_count == 0 || slot_count == 0 || line >= line_count {
        return None;
    }
    Some((line.saturating_mul(slot_count) / line_count).min(slot_count - 1))
}

fn mark_minimap_line_range(
    markers: &mut [MinimapLineMarkers],
    line_count: usize,
    start_line: usize,
    end_line: usize,
    mut mark: impl FnMut(&mut MinimapLineMarkers),
) {
    if markers.is_empty() || line_count == 0 || start_line >= line_count {
        return;
    }
    let start = minimap_slot(start_line, line_count, markers.len());
    let end = minimap_slot(end_line.min(line_count - 1), line_count, markers.len());
    if let (Some(start), Some(end)) = (start, end) {
        for marker in &mut markers[start..=end] {
            mark(marker);
        }
    }
}

fn minimap_scroll_from_pointer(
    pointer: f32,
    track_start: f32,
    track_length: f32,
    total_rows: usize,
    viewport_rows: usize,
) -> usize {
    if track_length <= 0.0 || total_rows <= viewport_rows {
        return 0;
    }
    let fraction = ((pointer - track_start) / track_length).clamp(0.0, 1.0);
    let center_row = (fraction * total_rows as f32).round() as usize;
    center_row
        .saturating_sub(viewport_rows / 2)
        .min(total_rows.saturating_sub(viewport_rows))
}

fn scrollbar_marker_offset(visual_row: usize, total_rows: usize, track_length: f32) -> Option<f32> {
    if total_rows == 0 || track_length <= 0.0 {
        return None;
    }
    let marker_height = 2.0_f32.min(track_length);
    let row = visual_row.min(total_rows - 1);
    Some(
        (row as f32 / total_rows as f32 * track_length)
            .min((track_length - marker_height).max(0.0)),
    )
}

fn symbol_at_position(
    snapshot: &labonair_editor::BufferSnapshot,
    position: Position,
) -> Option<String> {
    let line = snapshot.line(position.line);
    let chars = line.chars().collect::<Vec<_>>();
    let is_symbol = |ch: char| ch.is_alphanumeric() || ch == '_';
    let column = if chars.get(position.column).is_some_and(|ch| is_symbol(*ch)) {
        position.column
    } else if position.column > 0
        && chars
            .get(position.column - 1)
            .is_some_and(|ch| is_symbol(*ch))
    {
        position.column - 1
    } else {
        return None;
    };
    let mut start = column;
    let mut end = column + 1;
    while start > 0 && is_symbol(chars[start - 1]) {
        start -= 1;
    }
    while end < chars.len() && is_symbol(chars[end]) {
        end += 1;
    }
    let line_start = snapshot.line_start_byte(position.line);
    let text_start = line_start
        + line
            .char_indices()
            .nth(start)
            .map_or(line.len(), |(byte, _)| byte);
    let text_end = line_start
        + line
            .char_indices()
            .nth(end)
            .map_or(line.len(), |(byte, _)| byte);
    Some(snapshot.text_range(TextRange::new(text_start, text_end)))
}

fn find_scrollbar_marker_offsets(
    snapshot: &labonair_editor::BufferSnapshot,
    display: &labonair_editor::DisplaySnapshot,
    track_length: f32,
    text: &str,
    whole_word: bool,
) -> Vec<f32> {
    if text.is_empty() || text.len() > 256 {
        return Vec::new();
    }
    let query = SearchQuery::new(
        text,
        SearchOptions {
            case_sensitive: true,
            smart_case: false,
            whole_word,
            ..Default::default()
        },
    );
    let Ok(found) = find(snapshot, &query) else {
        return Vec::new();
    };
    let mut seen_pixels = HashSet::new();
    found
        .matches
        .into_iter()
        .filter_map(|matched| {
            let visual_row = display.logical_line_to_visual_row(matched.start.line);
            let offset = scrollbar_marker_offset(visual_row, display.total_rows, track_length)?;
            seen_pixels
                .insert(offset.round() as usize)
                .then_some(offset)
        })
        .collect()
}

fn find_minimap_occurrence_slots(
    snapshot: &labonair_editor::BufferSnapshot,
    line_count: usize,
    slot_count: usize,
    text: &str,
    whole_word: bool,
) -> Vec<usize> {
    if text.is_empty() || text.len() > 256 {
        return Vec::new();
    }
    let query = SearchQuery::new(
        text,
        SearchOptions {
            case_sensitive: true,
            smart_case: false,
            whole_word,
            ..Default::default()
        },
    );
    let Ok(found) = find(snapshot, &query) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    found
        .matches
        .into_iter()
        .filter_map(|matched| minimap_slot(matched.start.line, line_count, slot_count))
        .filter(|slot| seen.insert(*slot))
        .collect()
}

fn scrollbar_shows_diagnostic(
    severity: labonair_editor::language_services::DiagnosticSeverity,
    max_severity: EditorDiagnosticMaxSeverity,
) -> bool {
    use labonair_editor::language_services::DiagnosticSeverity as Severity;

    match max_severity {
        EditorDiagnosticMaxSeverity::All => true,
        EditorDiagnosticMaxSeverity::Information => severity != Severity::Hint,
        EditorDiagnosticMaxSeverity::Warning => {
            matches!(severity, Severity::Error | Severity::Warning)
        }
        EditorDiagnosticMaxSeverity::Error => severity == Severity::Error,
    }
}

fn scrollbar_policy_visible(visibility: EditorScrollbarVisibility, overflowing: bool) -> bool {
    match visibility {
        EditorScrollbarVisibility::Always => true,
        EditorScrollbarVisibility::Never => false,
        // GPUI does not expose the native scrollbar preference, so `system`
        // currently follows the app's auto-hide behavior.
        EditorScrollbarVisibility::Auto | EditorScrollbarVisibility::System => overflowing,
    }
}

fn minimap_policy_visible(
    visibility: EditorMinimapVisibility,
    display_in: EditorMinimapDisplayIn,
    editor_focused: bool,
    overflowing: bool,
) -> bool {
    let visibility_allows = match visibility {
        EditorMinimapVisibility::Never => false,
        EditorMinimapVisibility::Auto => overflowing,
        EditorMinimapVisibility::Always => true,
    };
    visibility_allows && (display_in == EditorMinimapDisplayIn::All || editor_focused)
}

fn minimap_width_pixels(max_width_columns: u32) -> f32 {
    (max_width_columns.clamp(20, 160) as f32 * 3.0).clamp(76.0, 320.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FoldMarker {
    range: FoldRange,
    folded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorControl {
    Diagnostics,
    InlineDiagnostics,
    GitBlame,
    LineNumbers,
    Minimap,
    SelectionMenu,
    InlayHints,
    CodeLens,
    SemanticHighlights,
    VimMode,
    HelixMode,
}

impl EditorControl {
    fn id(self) -> &'static str {
        match self {
            Self::Diagnostics => "editor-control-diagnostics",
            Self::InlineDiagnostics => "editor-control-inline-diagnostics",
            Self::GitBlame => "editor-control-git-blame",
            Self::LineNumbers => "editor-control-line-numbers",
            Self::Minimap => "editor-control-minimap",
            Self::SelectionMenu => "editor-control-selections-menu",
            Self::InlayHints => "editor-control-inlay-hints",
            Self::CodeLens => "editor-control-code-lens",
            Self::SemanticHighlights => "editor-control-semantic-highlights",
            Self::VimMode => "editor-control-vim-mode",
            Self::HelixMode => "editor-control-helix-mode",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Diagnostics => "Diagnostics",
            Self::InlineDiagnostics => "Inline Diagnostics",
            Self::GitBlame => "Git Blame in Status Bar",
            Self::LineNumbers => "Line Numbers",
            Self::Minimap => "Minimap",
            Self::SelectionMenu => "Selections Menu",
            Self::InlayHints => "Inlay Hints",
            Self::CodeLens => "Code Lens",
            Self::SemanticHighlights => "Semantic Highlights",
            Self::VimMode => "Vim Mode",
            Self::HelixMode => "Helix Mode",
        }
    }
}

fn available_editor_controls(capabilities: LanguageServiceCapabilities) -> Vec<EditorControl> {
    let mut controls = vec![
        EditorControl::Diagnostics,
        EditorControl::InlineDiagnostics,
        EditorControl::GitBlame,
        EditorControl::LineNumbers,
        EditorControl::Minimap,
        EditorControl::SelectionMenu,
    ];
    if capabilities.inlay_hints {
        controls.push(EditorControl::InlayHints);
    }
    if capabilities.code_lens && capabilities.execute_command {
        controls.push(EditorControl::CodeLens);
    }
    if capabilities.semantic_tokens {
        controls.push(EditorControl::SemanticHighlights);
    }
    controls.extend([EditorControl::VimMode, EditorControl::HelixMode]);
    controls
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

fn fold_ranges_from_lsp(
    ranges: &[FoldingRange],
    snapshot: &labonair_editor::BufferSnapshot,
) -> Vec<FoldRange> {
    let line_count = snapshot.line_count();
    let mut ranges = ranges
        .iter()
        .filter(|range| range.start_line < line_count && range.end_line < line_count)
        .filter_map(|range| FoldRange::new(range.start_line, range.end_line))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| (range.start_line, range.end_line));
    ranges.dedup();
    ranges
}

fn fold_candidates(
    server_ranges: &[FoldRange],
    symbols: &[labonair_editor::DocumentSymbol],
    syntax_ranges: &[FoldRange],
    snapshot: &labonair_editor::BufferSnapshot,
) -> Vec<FoldRange> {
    if server_ranges.is_empty() {
        let mut ranges = fold_ranges_from_symbols(symbols, snapshot);
        ranges.extend_from_slice(syntax_ranges);
        ranges.sort_by_key(|range| (range.start_line, range.end_line));
        ranges.dedup();
        ranges
    } else {
        server_ranges.to_vec()
    }
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

fn fold_range_for_cursor(candidates: &[FoldRange], line: usize) -> Option<FoldRange> {
    candidates
        .iter()
        .copied()
        .filter(|range| range.start_line <= line && line <= range.end_line)
        .min_by_key(|range| range.end_line.saturating_sub(range.start_line))
}

fn fold_contains(outer: FoldRange, inner: FoldRange) -> bool {
    outer != inner && outer.start_line <= inner.start_line && outer.end_line >= inner.end_line
}

fn fold_ranges_within(candidates: &[FoldRange], outer: FoldRange) -> Vec<FoldRange> {
    candidates
        .iter()
        .copied()
        .filter(|range| *range == outer || fold_contains(outer, *range))
        .collect()
}

fn fold_ranges_at_level(candidates: &[FoldRange], level: u8) -> Vec<FoldRange> {
    candidates
        .iter()
        .copied()
        .filter(|range| {
            let depth = candidates
                .iter()
                .copied()
                .filter(|outer| fold_contains(*outer, *range))
                .count();
            depth + 1 == usize::from(level)
        })
        .collect()
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

struct DisplayLayoutCache {
    revision: Revision,
    config: DisplayConfig,
    folds: Vec<FoldRange>,
    inlay_hints: Vec<InlayHint>,
    layout: DisplaySnapshot,
}

struct SymbolCache {
    revision: Revision,
    language: Language,
    symbols: Arc<[labonair_editor::DocumentSymbol]>,
    folding_ranges: Arc<[FoldRange]>,
}

#[derive(Clone)]
struct ScrollbarOccurrenceCache {
    revision: Revision,
    total_rows: usize,
    track_height: usize,
    selected_text: Option<String>,
    symbol: Option<String>,
    selected_text_offsets: Vec<f32>,
    symbol_offsets: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LanguageServiceRequestSlot {
    Other,
    Completion,
    SignatureHelp,
    Navigation,
    TypeDefinition,
    CallHierarchy,
    CodeLens,
    DocumentLinks,
    InlayHints,
    FoldingRanges,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CallHierarchyDirection {
    #[default]
    Incoming,
    Outgoing,
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

fn language_service_controls_available(snapshot: &LanguageServiceRuntimeSnapshot) -> bool {
    snapshot.source == LanguageServiceRuntimeSource::LocalProcess
        && !matches!(
            snapshot.status,
            LanguageServiceRuntimeStatus::Disabled | LanguageServiceRuntimeStatus::Unsupported
        )
}

fn language_service_status_label(status: LanguageServiceRuntimeStatus) -> &'static str {
    match status {
        LanguageServiceRuntimeStatus::Disabled => "Disabled",
        LanguageServiceRuntimeStatus::Starting => "Starting",
        LanguageServiceRuntimeStatus::Running => "Running",
        LanguageServiceRuntimeStatus::Crashed => "Crashed",
        LanguageServiceRuntimeStatus::Fallback => "Stopped",
        LanguageServiceRuntimeStatus::Unsupported => "Unsupported",
    }
}

fn git_blame_status_label(blame: &GitBlameSnapshot) -> String {
    let compact_summary = blame
        .summary
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let was_truncated = compact_summary.chars().count() > 56;
    let summary = compact_summary.chars().take(56).collect::<String>();
    if was_truncated {
        format!("{} · {summary}…", blame.author)
    } else {
        format!("{} · {summary}", blame.author)
    }
}

fn git_blame_age_label(author_time: i64, now: i64) -> String {
    let elapsed = now.saturating_sub(author_time).max(0) as u64;
    if elapsed < 60 {
        "just now".into()
    } else if elapsed < 60 * 60 {
        format!("{} min ago", elapsed / 60)
    } else if elapsed < 24 * 60 * 60 {
        format!("{} hr ago", elapsed / (60 * 60))
    } else if elapsed < 30 * 24 * 60 * 60 {
        format!("{} days ago", elapsed / (24 * 60 * 60))
    } else if elapsed < 365 * 24 * 60 * 60 {
        format!("{} mo ago", elapsed / (30 * 24 * 60 * 60))
    } else {
        format!("{} yr ago", elapsed / (365 * 24 * 60 * 60))
    }
}

fn diagnostic_at_position<'a>(
    diagnostics: &'a [labonair_editor::Diagnostic],
    snapshot: &labonair_editor::BufferSnapshot,
    position: Position,
    maximum: EditorDiagnosticMaxSeverity,
) -> Option<&'a labonair_editor::Diagnostic> {
    let offset = snapshot.position_to_byte(position);
    diagnostics
        .iter()
        .filter(|diagnostic| scrollbar_shows_diagnostic(diagnostic.severity, maximum))
        .filter(|diagnostic| {
            let range = diagnostic.range.to_text_range(snapshot);
            range.start <= offset && offset <= range.end
        })
        .min_by_key(|diagnostic| {
            let range = diagnostic.range.to_text_range(snapshot);
            (diagnostic.severity, range.end.saturating_sub(range.start))
        })
}

fn document_link_at_position<'a>(
    links: &'a [DocumentLink],
    snapshot: &BufferSnapshot,
    position: Position,
) -> Option<&'a DocumentLink> {
    let offset = snapshot.position_to_byte(position);
    links.iter().find(|link| {
        let range = link.range.to_text_range(snapshot);
        range.start <= offset && offset < range.end
    })
}

fn error_lens_for_line(
    diagnostics: &[labonair_editor::Diagnostic],
    line: usize,
    maximum: EditorDiagnosticMaxSeverity,
) -> Option<(
    labonair_editor::language_services::DiagnosticSeverity,
    String,
)> {
    let diagnostic = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.range.start.line == line
                && scrollbar_shows_diagnostic(diagnostic.severity, maximum)
        })
        .min_by_key(|diagnostic| diagnostic.severity)?;
    let message = diagnostic
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!message.is_empty()).then_some((diagnostic.severity, message))
}

fn diagnostic_hover_text(diagnostic: &labonair_editor::Diagnostic) -> String {
    let severity = match diagnostic.severity {
        labonair_editor::language_services::DiagnosticSeverity::Error => "Error",
        labonair_editor::language_services::DiagnosticSeverity::Warning => "Warning",
        labonair_editor::language_services::DiagnosticSeverity::Information => "Information",
        labonair_editor::language_services::DiagnosticSeverity::Hint => "Hint",
    };
    let code = diagnostic
        .code
        .as_deref()
        .map(|code| format!(" ({code})"))
        .unwrap_or_default();
    format!(
        "{severity}{code}: {}\n{}",
        diagnostic.message,
        diagnostic.source.as_str()
    )
}

fn document_color_at_position(
    colors: &[DocumentColor],
    snapshot: &labonair_editor::BufferSnapshot,
    position: Position,
) -> Option<DocumentColorValue> {
    let offset = snapshot.position_to_byte(position);
    colors.iter().find_map(|color| {
        let range = color.range.to_text_range(snapshot);
        (range.start <= offset && offset <= range.end).then_some(color.color)
    })
}

fn document_color_hover_text(color: DocumentColorValue) -> String {
    if color.alpha == u8::MAX {
        format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue)
    } else {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            color.red, color.green, color.blue, color.alpha
        )
    }
}

fn diagnostic_error_warning_counts(diagnostics: &[labonair_editor::Diagnostic]) -> (usize, usize) {
    use labonair_editor::language_services::DiagnosticSeverity;

    diagnostics
        .iter()
        .fold((0, 0), |(errors, warnings), diagnostic| {
            match diagnostic.severity {
                DiagnosticSeverity::Error => (errors + 1, warnings),
                DiagnosticSeverity::Warning => (errors, warnings + 1),
                DiagnosticSeverity::Information | DiagnosticSeverity::Hint => (errors, warnings),
            }
        })
}

fn diagnostic_menu_labels(
    diagnostic: &labonair_editor::Diagnostic,
    snapshot: &BufferSnapshot,
) -> (String, String) {
    use labonair_editor::language_services::DiagnosticSeverity;

    let position = diagnostic.range.start.to_position(snapshot);
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => "Error",
        DiagnosticSeverity::Warning => "Warning",
        DiagnosticSeverity::Information => "Information",
        DiagnosticSeverity::Hint => "Hint",
    };
    let label = format!("{}:{} · {severity}", position.line + 1, position.column + 1);
    let detail = diagnostic
        .code
        .as_deref()
        .map(|code| format!("{code}: {}", diagnostic.message))
        .unwrap_or_else(|| diagnostic.message.clone())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (label, detail)
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
    /// Server fold ranges are refreshed for each document generation. Empty
    /// or unavailable results fall back to the local symbol ranges.
    server_fold_ranges: Vec<FoldRange>,
    selection_menu_anchor: Option<Point<Pixels>>,
    editor_controls_anchor: Option<Point<Pixels>>,
    language_menu_anchor: Option<Point<Pixels>>,
    diagnostics_menu_anchor: Option<Point<Pixels>>,
    language_service_menu_anchor: Option<Point<Pixels>>,
    language_list_state: ListState,
    line_ending_menu_anchor: Option<Point<Pixels>>,
    encoding_menu_anchor: Option<Point<Pixels>>,
    selection_drag: Option<EditorSelectionDrag>,
    outline_open: bool,
    breadcrumb_visible_override: Option<bool>,
    diagnostics_enabled_override: Option<bool>,
    inline_diagnostics_enabled_override: Option<bool>,
    line_numbers_enabled_override: Option<bool>,
    minimap_enabled_override: Option<bool>,
    semantic_tokens_enabled_override: Option<bool>,
    selections_menu_enabled_override: Option<bool>,
    modal_mode_override: Option<EditorModalMode>,
    /// Editor-internal groups. Workspace pane layouts remain a separate
    /// capability; this tree is changed only by Editor commands.
    editor_splits: labonair_editor::EditorSplitTree,
    /// Independent selection, fold, and viewport state for each split group.
    editor_group_views: HashMap<labonair_editor::EditorGroupId, EditorViewSession>,
    split_drag_last: Option<f32>,
    /// Cached glyph metrics `(char_width, line_height)` in px.
    metrics: (f32, f32),
    gutter_width: f32,
    /// Document-wide display coordinates are reused across paints until the
    /// buffer revision, display settings, or folds change.
    display_layout_cache: Option<DisplayLayoutCache>,
    /// Tree-sitter symbols are likewise stable for one buffer revision and
    /// language. The render path uses this for outline, folds, breadcrumbs,
    /// and sticky context instead of parsing the document repeatedly.
    symbol_cache: Option<SymbolCache>,
    scrollbar_occurrence_cache: Option<ScrollbarOccurrenceCache>,
    minimap_occurrence_cache: Option<MinimapOccurrenceCache>,
    search: Option<EditorSearch>,
    kill_ring: KillRing,
    buffer_search_open: bool,
    buffer_search_input: Option<Entity<InputState>>,
    project_search: ProjectSearchSession,
    pending_navigation: Option<Position>,
    pending_lsp_navigation: Option<LspPosition>,
    navigation_history: NavigationHistory,
    /// Tree-sitter syntax highlighter for the current document (T06-002).
    syntax: SyntaxHighlighter,
    /// Vim keybinding state machine (T06-003) — `None` when Vim mode is off.
    vim: Option<Vim>,
    helix: Option<Helix>,
    /// Live editor settings mirror (font/indent/line-numbers), refreshed from
    /// the layered `SettingsStore` (T13-003).
    prefs: EditorSettings,
    inlay_hints_enabled_override: Option<bool>,
    code_lens_enabled_override: Option<bool>,
    /// Editor-owned hover text and its window-relative pointer anchor. It is
    /// populated by diagnostics, inlay hints, or settings-schema descriptions.
    hover: Option<(Point<Pixels>, SharedString)>,
    error_lens_task: Option<Task<()>>,
    error_lens_revision: Option<Revision>,
    /// Revision-bound Git projection. Repository state and actions remain in
    /// the Git/Project-Diff capability; this view only renders it.
    git_gutter: GitGutterSnapshot,
    git_provider: Option<Arc<dyn GitDecorationProvider>>,
    git_action_sink: Option<Arc<dyn HunkActionSink>>,
    git_session_id: Option<String>,
    git_refresh_generation: u64,
    git_blame: Option<GitBlameSnapshot>,
    git_blame_request: Option<(String, usize, Revision)>,
    git_blame_generation: u64,
    git_blame_task: Option<Task<()>>,
    git_blame_enabled: bool,
    git_context_menu: Option<(Point<Pixels>, String, Revision)>,
    text_context_menu: Option<Point<Pixels>>,
    character_palette_open: bool,
    character_palette_group_open: Option<&'static str>,
    git_discard_confirmation: Option<GitDiscardPreview>,
    /// Editor-owned language-service runtime and its revision-safe result
    /// projection. Workspace only injects the runtime at composition time.
    language_services: Arc<LanguageServiceRuntime>,
    call_hierarchy_navigator: Option<CallHierarchyNavigator>,
    document_link_navigator: Option<DocumentLinkNavigator>,
    language_service_snapshot: LanguageServiceRuntimeSnapshot,
    language_service_state: LanguageServiceState,
    language_service_uri: Option<String>,
    language_service_generation: u64,
    language_service_open: bool,
    diagnostic_index: usize,
    diagnostics_revision: Option<Revision>,
    completion_visible: bool,
    completion_request: Option<Task<()>>,
    completion_selected: usize,
    completion_request_serial: Option<u64>,
    completion_position: Option<LspPosition>,
    hover_request: Option<Task<()>>,
    hover_position: Option<Position>,
    inlay_hints_request: Option<Task<()>>,
    inlay_hints_request_serial: Option<u64>,
    inlay_hints_request_range: Option<LspRange>,
    code_lenses: Vec<CodeLens>,
    code_lens_request_serial: Option<u64>,
    document_links: Vec<DocumentLink>,
    document_link_request_serial: Option<u64>,
    signature_help_state: SignatureHelpUiState,
    signature_help_request_serial: Option<u64>,
    signature_help_position: Option<LspPosition>,
    navigation_request_serial: Option<u64>,
    navigation_position: Option<LspPosition>,
    folding_ranges_request_serial: Option<u64>,
    language_service_request_serial: u64,
    code_actions: Option<Vec<labonair_editor::CodeAction>>,
    call_hierarchy_root: Option<CallHierarchyItem>,
    call_hierarchy_calls: Vec<CallHierarchyCall>,
    call_hierarchy_direction: CallHierarchyDirection,
    call_hierarchy_open: bool,
    call_hierarchy_loading: bool,
    call_hierarchy_request_serial: Option<u64>,
    rename_requested: bool,
    rename_input: Option<Entity<InputState>>,
    go_to_line_requested: bool,
    go_to_line_input: Option<Entity<InputState>>,
    format_on_save_in_flight: bool,
    save_intent_after_format: Option<SaveIntent>,
    cursor_blink_on: bool,
    editor_focused: bool,
    minimap_hovered: bool,
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

fn inlay_hint_tooltip_at_position<'a>(
    hints: &'a [InlayHint],
    snapshot: &labonair_editor::BufferSnapshot,
    position: Position,
    kind_visible: impl Fn(InlayHintKind) -> bool,
) -> Option<&'a str> {
    hints
        .iter()
        .filter(|hint| kind_visible(hint.kind))
        .find_map(|hint| {
            (hint.position.to_position(snapshot) == position)
                .then_some(hint.tooltip.as_deref())
                .flatten()
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
    /// Publish the active modal state to GPUI's keymap context tree. User
    /// bindings can scope actions with predicates such as
    /// `Editor && editor_mode == helix && modal_mode == select`.
    fn keymap_context(&self) -> KeyContext {
        let mut context = KeyContext::default();
        context.add("Editor");
        if self.call_hierarchy_open {
            context.add("CallHierarchyPicker");
        }

        let (editor_mode, modal_mode) = if let Some(vim) = &self.vim {
            context.add("EditorVim");
            let modal_mode = match vim.mode() {
                VimMode::Normal => "normal",
                VimMode::Insert => "insert",
                VimMode::Visual => "visual",
                VimMode::VisualLine => "visual_line",
                VimMode::Command => "command",
            };
            ("vim", modal_mode)
        } else if let Some(helix) = &self.helix {
            context.add("EditorHelix");
            let modal_mode = match helix.mode() {
                HelixMode::Select => "select",
                HelixMode::Insert => "insert",
            };
            ("helix", modal_mode)
        } else {
            ("normal", "normal")
        };
        context.set("editor_mode", editor_mode);
        context.set("modal_mode", modal_mode);
        if !self.completion_visible {
            if self.doc.has_next_snippet_tabstop() {
                context.add("EditorSnippetNext");
            }
            if self.doc.has_previous_snippet_tabstop() {
                context.add("EditorSnippetPrevious");
            }
        }
        context
    }

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
        let helix = (!prefs.vim_mode() && prefs.helix_mode()).then(Helix::new);
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
            server_fold_ranges: Vec::new(),
            selection_menu_anchor: None,
            editor_controls_anchor: None,
            language_menu_anchor: None,
            diagnostics_menu_anchor: None,
            language_service_menu_anchor: None,
            language_list_state: ListState::new(Language::ALL.len(), ListAlignment::Top, px(32.0)),
            line_ending_menu_anchor: None,
            encoding_menu_anchor: None,
            selection_drag: None,
            outline_open: false,
            breadcrumb_visible_override: None,
            diagnostics_enabled_override: None,
            inline_diagnostics_enabled_override: None,
            line_numbers_enabled_override: None,
            minimap_enabled_override: None,
            semantic_tokens_enabled_override: None,
            selections_menu_enabled_override: None,
            modal_mode_override: None,
            editor_splits: labonair_editor::EditorSplitTree::default(),
            editor_group_views: HashMap::new(),
            split_drag_last: None,
            metrics: (8.0, 18.0),
            gutter_width: 48.0,
            display_layout_cache: None,
            symbol_cache: None,
            scrollbar_occurrence_cache: None,
            minimap_occurrence_cache: None,
            search: None,
            kill_ring: KillRing::default(),
            buffer_search_open: false,
            buffer_search_input: None,
            project_search: ProjectSearchSession::default(),
            pending_navigation: None,
            pending_lsp_navigation: None,
            navigation_history: NavigationHistory::default(),
            syntax: SyntaxHighlighter::new(Language::PlainText),
            vim,
            helix,
            prefs,
            inlay_hints_enabled_override: None,
            code_lens_enabled_override: None,
            hover: None,
            error_lens_task: None,
            error_lens_revision: None,
            git_gutter: GitGutterSnapshot::unavailable("", Revision::default()),
            git_provider: None,
            git_action_sink: None,
            git_session_id: None,
            git_refresh_generation: 0,
            git_blame: None,
            git_blame_request: None,
            git_blame_generation: 0,
            git_blame_task: None,
            git_blame_enabled: false,
            git_context_menu: None,
            text_context_menu: None,
            character_palette_open: false,
            character_palette_group_open: None,
            git_discard_confirmation: None,
            language_services,
            call_hierarchy_navigator: None,
            document_link_navigator: None,
            language_service_snapshot,
            language_service_state: LanguageServiceState::new(DocumentVersion::new(0)),
            language_service_uri: None,
            language_service_generation: 0,
            language_service_open: false,
            diagnostic_index: 0,
            diagnostics_revision: None,
            completion_visible: false,
            completion_request: None,
            completion_selected: 0,
            completion_request_serial: None,
            completion_position: None,
            hover_request: None,
            hover_position: None,
            inlay_hints_request: None,
            inlay_hints_request_serial: None,
            inlay_hints_request_range: None,
            code_lenses: Vec::new(),
            code_lens_request_serial: None,
            document_links: Vec::new(),
            document_link_request_serial: None,
            signature_help_state: SignatureHelpUiState::Hidden,
            signature_help_request_serial: None,
            signature_help_position: None,
            navigation_request_serial: None,
            navigation_position: None,
            folding_ranges_request_serial: None,
            language_service_request_serial: 0,
            code_actions: None,
            call_hierarchy_root: None,
            call_hierarchy_calls: Vec::new(),
            call_hierarchy_direction: CallHierarchyDirection::Incoming,
            call_hierarchy_open: false,
            call_hierarchy_loading: false,
            call_hierarchy_request_serial: None,
            rename_requested: false,
            rename_input: None,
            go_to_line_requested: false,
            go_to_line_input: None,
            format_on_save_in_flight: false,
            save_intent_after_format: None,
            cursor_blink_on: true,
            editor_focused: false,
            minimap_hovered: false,
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

    /// Inject the Workspace-owned file-opening path for call-hierarchy rows.
    pub fn set_call_hierarchy_navigator(
        &mut self,
        navigator: impl Fn(String, LspPosition, &mut Window, &mut App) + 'static,
    ) {
        self.call_hierarchy_navigator = Some(Rc::new(navigator));
    }

    /// Inject the Workspace-owned safe navigation path for LSP document links.
    pub fn set_document_link_navigator(
        &mut self,
        navigator: impl Fn(String, &mut Window, &mut App) + 'static,
    ) {
        self.document_link_navigator = Some(Rc::new(navigator));
    }

    pub fn language_service_snapshot(&self) -> LanguageServiceRuntimeSnapshot {
        self.language_service_snapshot.clone()
    }

    /// Editor-owned editability state consumed by the Workspace tab menu.
    pub fn editability(&self) -> Option<EditorEditability> {
        self.doc.editability()
    }

    fn capture_active_editor_group_view(&mut self) -> EditorViewSession {
        let group = self.editor_splits.focused_group();
        let view = self.editor_group_view_snapshot(group);
        self.editor_group_views.insert(group, view.clone());
        view
    }

    fn editor_group_view_snapshot(
        &self,
        group: labonair_editor::EditorGroupId,
    ) -> EditorViewSession {
        if group == self.editor_splits.focused_group() {
            return EditorViewSession {
                group_id: group.value(),
                scroll_top: self.scroll_top,
                scroll_left: self.scroll_left,
                folds: self.folds.clone(),
                selections: self.doc.selection_set(),
            };
        }
        let mut view = self
            .editor_group_views
            .get(&group)
            .cloned()
            .unwrap_or_else(|| EditorViewSession {
                group_id: group.value(),
                scroll_top: 0,
                scroll_left: 0,
                folds: Vec::new(),
                selections: SelectionSet::default(),
            });
        view.group_id = group.value();
        view.selections = self
            .doc
            .editor_group_selection(group)
            .unwrap_or(view.selections);
        view
    }

    fn restore_editor_group_view(&mut self, group: labonair_editor::EditorGroupId) {
        let view = self
            .editor_group_views
            .get(&group)
            .cloned()
            .unwrap_or_else(|| EditorViewSession {
                group_id: group.value(),
                scroll_top: 0,
                scroll_left: 0,
                folds: Vec::new(),
                selections: self.doc.editor_group_selection(group).unwrap_or_default(),
            });
        self.scroll_top = view.scroll_top;
        self.scroll_left = view.scroll_left;
        self.folds = view.folds;
        self.display_layout_cache = None;
        self.clamp_scroll();
    }

    fn reset_editor_group_views(&mut self) {
        self.scroll_top = 0;
        self.scroll_left = 0;
        self.folds.clear();
        self.editor_group_views.clear();
        let groups = self.editor_splits.groups();
        let focused = self.editor_splits.focused_group();
        self.doc.activate_editor_group_view(focused);
        self.doc.retain_editor_group_views(&groups);
        let selection = self.doc.selection_set();
        for group in groups {
            if group != focused {
                self.doc.create_editor_group_view(group);
                self.doc
                    .set_editor_group_selection(group, selection.clone());
            }
            self.editor_group_views.insert(
                group,
                EditorViewSession {
                    group_id: group.value(),
                    scroll_top: 0,
                    scroll_left: 0,
                    folds: Vec::new(),
                    selections: selection.clone(),
                },
            );
        }
        self.display_layout_cache = None;
    }

    fn focus_editor_group(&mut self, group: labonair_editor::EditorGroupId) -> bool {
        if self.editor_splits.focused_group() == group {
            return false;
        }
        self.capture_active_editor_group_view();
        if self.editor_splits.focus(group).is_err() {
            return false;
        }
        self.doc.activate_editor_group_view(group);
        self.restore_editor_group_view(group);
        true
    }

    fn close_editor_group(&mut self, group: labonair_editor::EditorGroupId) -> bool {
        let previous_focus = self.editor_splits.focused_group();
        if self.editor_splits.close(group).is_err() {
            return false;
        }
        let focused = self.editor_splits.focused_group();
        if focused != previous_focus {
            self.doc.activate_editor_group_view(focused);
            self.restore_editor_group_view(focused);
        }
        let groups = self.editor_splits.groups();
        self.doc.retain_editor_group_views(&groups);
        self.editor_group_views
            .retain(|view_group, _| groups.contains(view_group));
        true
    }

    fn synchronize_editor_group_views(
        &mut self,
        previous_focus: labonair_editor::EditorGroupId,
        previous_group_count: usize,
    ) {
        let focused = self.editor_splits.focused_group();
        if self.editor_splits.group_count() > previous_group_count {
            let new_view = self
                .editor_group_views
                .get(&previous_focus)
                .cloned()
                .unwrap_or_else(|| self.capture_active_editor_group_view());
            let mut new_view = new_view;
            new_view.group_id = focused.value();
            self.editor_group_views.insert(focused, new_view);
            self.doc.create_editor_group_view(focused);
        }
        self.doc.activate_editor_group_view(focused);
        let groups = self.editor_splits.groups();
        self.doc.retain_editor_group_views(&groups);
        self.editor_group_views
            .retain(|group, _| groups.contains(group));
        if focused != previous_focus {
            self.restore_editor_group_view(focused);
        }
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
                let previous_group_count = before.group_count();
                let previous_focus = before.focused_group();
                self.capture_active_editor_group_view();
                let result = match intent {
                    SplitIntent::Left => self.editor_splits.split_left().map(|_| ()),
                    SplitIntent::Right => self.editor_splits.split_right().map(|_| ()),
                    SplitIntent::Up => self.editor_splits.split_up().map(|_| ()),
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
                    SplitIntent::ResetSizes => {
                        self.editor_splits.reset_sizes();
                        Ok(())
                    }
                    SplitIntent::Resize { delta_milli } => self.editor_splits.resize(
                        self.editor_splits.focused_group(),
                        f32::from(delta_milli) / 1000.0,
                    ),
                };
                if result.is_ok() && self.editor_splits != before {
                    self.synchronize_editor_group_views(previous_focus, previous_group_count);
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

    /// Dispatch a command registered by the Editor capability.
    ///
    /// Workspace resolves the active view and forwards the stable command ID;
    /// route selection and the associated edit, split, language, or Git
    /// behavior stay inside the Editor owner.
    pub fn dispatch_registered_command(&mut self, id: CommandId, cx: &mut Context<Self>) -> bool {
        let Some(route) = route_for_command(id) else {
            return false;
        };
        match route {
            EditorCommandRoute::Edit(intent) => {
                let intent = match (id, intent) {
                    (CommandId::EditorIndent, EditIntent::Indent { .. }) => EditIntent::Indent {
                        unit: self.indent_unit(),
                    },
                    (CommandId::EditorOutdent, EditIntent::Outdent { .. }) => EditIntent::Outdent {
                        unit: self.indent_unit(),
                    },
                    (_, intent) => intent,
                };
                let _ = self.execute_command(EditorCommand::Edit(intent), cx);
            }
            EditorCommandRoute::Split(intent) => {
                let _ = self.execute_command(EditorCommand::Split(intent), cx);
            }
            EditorCommandRoute::Language(command) => {
                self.execute_language_command(command, cx);
            }
            EditorCommandRoute::Git(command) => self.execute_git_command(command, cx),
            EditorCommandRoute::Ui(EditorUiCommand::GoToLineColumn) => {
                self.go_to_line_requested = true;
                cx.notify();
            }
            EditorCommandRoute::Ui(EditorUiCommand::GoBack) => self.navigate_history_back(cx),
            EditorCommandRoute::Ui(EditorUiCommand::GoForward) => self.navigate_history_forward(cx),
            EditorCommandRoute::Ui(EditorUiCommand::ToggleSearchSelection) => {
                self.toggle_search_selection_scope(cx);
            }
            EditorCommandRoute::Ui(EditorUiCommand::ToggleReadOnly) => {
                if self.doc.toggle_temporary_read_only() {
                    cx.emit(EditorEvent::Changed);
                    cx.notify();
                }
            }
            EditorCommandRoute::Ui(EditorUiCommand::NextSnippetTabstop) => {
                if !self.doc.next_snippet_tabstop() {
                    return false;
                }
                self.ensure_cursor_visible(cx);
                cx.notify();
            }
            EditorCommandRoute::Ui(EditorUiCommand::PreviousSnippetTabstop) => {
                if !self.doc.previous_snippet_tabstop() {
                    return false;
                }
                self.ensure_cursor_visible(cx);
                cx.notify();
            }
            EditorCommandRoute::Ui(EditorUiCommand::OpenMarkdownPreview) => {
                if let Some(source) = self.markdown_source_snapshot() {
                    cx.emit(EditorEvent::MarkdownPreviewRequested(source));
                } else {
                    notify_info(
                        cx,
                        "Markdown Preview",
                        "Open a Markdown file before opening its preview.",
                    );
                }
            }
            EditorCommandRoute::Ui(EditorUiCommand::OpenSvgPreview) => {
                let svg_path = self.doc.path.clone().filter(|path| {
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
                });
                if let Some(path) = svg_path {
                    cx.emit(EditorEvent::SvgPreviewRequested(path));
                } else {
                    notify_info(
                        cx,
                        "SVG Preview",
                        "Open an SVG file before opening its preview.",
                    );
                }
            }
            EditorCommandRoute::Ui(EditorUiCommand::OpenFollowingMarkdownPreview) => {
                if let Some(source) = self.markdown_source_snapshot() {
                    cx.emit(EditorEvent::MarkdownFollowingPreviewRequested(source));
                } else {
                    notify_info(
                        cx,
                        "Markdown Preview",
                        "Open a Markdown file before opening a following preview.",
                    );
                }
            }
            EditorCommandRoute::Ui(EditorUiCommand::ToggleBreadcrumb) => {
                self.breadcrumb_visible_override = Some(!self.breadcrumbs_visible());
                cx.notify();
            }
            EditorCommandRoute::Ui(EditorUiCommand::ToggleInlayHints) => {
                self.toggle_inlay_hints(cx);
            }
            EditorCommandRoute::Ui(EditorUiCommand::Rewrap) => {
                let columns = self.content_width_columns();
                let columns = if columns <= 1 { 80 } else { columns };
                let _ =
                    self.execute_command(EditorCommand::Edit(EditIntent::Rewrap { columns }), cx);
            }
            EditorCommandRoute::Ui(EditorUiCommand::KillRingCut) => self.kill_ring_cut(cx),
            EditorCommandRoute::Ui(EditorUiCommand::KillRingYank) => self.kill_ring_yank(cx),
            EditorCommandRoute::Ui(EditorUiCommand::ShowCharacterPalette) => {
                self.character_palette_open = true;
                self.character_palette_group_open = None;
                cx.notify();
            }
            EditorCommandRoute::Ui(
                command @ (EditorUiCommand::Fold
                | EditorUiCommand::FoldAll
                | EditorUiCommand::FoldAtLevel(_)
                | EditorUiCommand::FoldRecursive
                | EditorUiCommand::UnfoldAll
                | EditorUiCommand::UnfoldLines
                | EditorUiCommand::UnfoldRecursive),
            ) => self.execute_fold_command(command, cx),
        }
        true
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
            EditorLanguageCommand::ShowIncomingCalls => {
                self.start_call_hierarchy(cx);
            }
            EditorLanguageCommand::ToggleCallHierarchyDirection => {
                self.toggle_call_hierarchy_direction(cx);
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
            EditorLanguageCommand::GoToTypeDefinition => {
                self.request_language_service(
                    LanguageServiceRequest::TypeDefinition {
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

    fn start_call_hierarchy(&mut self, cx: &mut Context<Self>) {
        self.call_hierarchy_open = true;
        self.call_hierarchy_loading = true;
        self.call_hierarchy_root = None;
        self.call_hierarchy_calls.clear();
        self.call_hierarchy_direction = CallHierarchyDirection::Incoming;
        self.request_language_service(
            LanguageServiceRequest::PrepareCallHierarchy {
                position: self.current_lsp_position(),
            },
            cx,
        );
        cx.notify();
    }

    fn execute_code_lens(&mut self, command: LanguageServiceCommand, cx: &mut Context<Self>) {
        self.request_language_service(
            LanguageServiceRequest::ExecuteCommand {
                command: command.command,
                arguments: command.arguments,
            },
            cx,
        );
    }

    fn toggle_call_hierarchy_direction(&mut self, cx: &mut Context<Self>) {
        let Some(item) = self.call_hierarchy_root.clone() else {
            self.start_call_hierarchy(cx);
            return;
        };
        self.call_hierarchy_direction = match self.call_hierarchy_direction {
            CallHierarchyDirection::Incoming => CallHierarchyDirection::Outgoing,
            CallHierarchyDirection::Outgoing => CallHierarchyDirection::Incoming,
        };
        self.call_hierarchy_open = true;
        self.call_hierarchy_loading = true;
        self.call_hierarchy_calls.clear();
        let request = match self.call_hierarchy_direction {
            CallHierarchyDirection::Incoming => LanguageServiceRequest::IncomingCalls { item },
            CallHierarchyDirection::Outgoing => LanguageServiceRequest::OutgoingCalls { item },
        };
        self.request_language_service(request, cx);
        cx.notify();
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

    /// Return an immutable typed snapshot for the Workspace preview adapter.
    pub fn buffer_snapshot(&self) -> EditorBufferSnapshot {
        EditorBufferSnapshot {
            path: self.doc.path.clone(),
            language: self.doc.language,
            snapshot: self.doc.snapshot(),
        }
    }

    fn markdown_source_snapshot(&self) -> Option<EditorBufferSnapshot> {
        (self.doc.language == Language::Markdown).then(|| self.buffer_snapshot())
    }

    fn emit_buffer_changed(&self, cx: &mut Context<Self>) {
        cx.emit(EditorEvent::BufferChanged(self.buffer_snapshot()));
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
        self.ensure_cursor_visible(cx);
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if revision_changed {
            self.emit_buffer_changed(cx);
        }
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
        let (errors, warnings) = diagnostic_error_warning_counts(&diagnostics);
        let body = if diagnostics.is_empty() {
            "No diagnostics for the current document.".to_string()
        } else {
            format!(
                "{} diagnostic(s): {} error(s), {} warning(s), and {} information/hint(s). Use Next/Previous Diagnostic to navigate.",
                diagnostics.len(),
                errors,
                warnings,
                diagnostics.len().saturating_sub(errors + warnings)
            )
        };
        notify_info(cx, "Editor diagnostics", &body);
    }

    fn navigate_diagnostic(&mut self, forward: bool, cx: &mut Context<Self>) {
        let diagnostics = self
            .language_service_state
            .snapshot()
            .diagnostics
            .into_iter()
            .filter(|diagnostic| {
                scrollbar_shows_diagnostic(
                    diagnostic.severity,
                    self.prefs.diagnostics_max_severity(),
                )
            })
            .collect::<Vec<_>>();
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
        self.ensure_cursor_visible(cx);
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
        self.server_fold_ranges.clear();
        self.folding_ranges_request_serial = None;
        self.hover = None;
        self.hover_request = None;
        self.hover_position = None;
        self.error_lens_task = None;
        self.error_lens_revision = None;
        self.inlay_hints_request = None;
        self.inlay_hints_request_serial = None;
        self.inlay_hints_request_range = None;
        self.code_lenses.clear();
        self.code_lens_request_serial = None;
        self.document_links.clear();
        self.document_link_request_serial = None;
        self.language_service_uri = Some(uri.clone());
        self.language_service_open = true;
        self.language_service_state
            .set_document_generation(version, generation);
        self.diagnostics_revision = None;
        self.diagnostics_menu_anchor = None;
        self.completion_visible = false;
        self.completion_request = None;
        self.completion_selected = 0;
        self.completion_request_serial = None;
        self.completion_position = None;
        self.signature_help_state = SignatureHelpUiState::Hidden;
        self.signature_help_request_serial = None;
        self.signature_help_position = None;
        self.code_actions = None;
        self.call_hierarchy_root = None;
        self.call_hierarchy_calls.clear();
        self.call_hierarchy_open = false;
        self.call_hierarchy_loading = false;
        self.call_hierarchy_request_serial = None;
        self.rename_requested = false;
        self.rename_input = None;
        self.format_on_save_in_flight = false;
        self.save_intent_after_format = None;
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
                    if this.language_service_snapshot.source
                        == LanguageServiceRuntimeSource::LocalProcess
                        && this.language_service_snapshot.capabilities.folding_ranges
                    {
                        this.request_language_service(LanguageServiceRequest::FoldingRanges, cx);
                    }
                    if this.language_service_snapshot.source
                        == LanguageServiceRuntimeSource::LocalProcess
                        && this.language_service_snapshot.capabilities.document_symbols
                    {
                        this.request_language_service(LanguageServiceRequest::DocumentSymbols, cx);
                    }
                    if this.code_lens_enabled()
                        && this.language_service_snapshot.capabilities.code_lens
                        && this.language_service_snapshot.capabilities.execute_command
                    {
                        this.request_code_lenses(cx);
                    }
                    if this.language_service_snapshot.capabilities.document_links {
                        this.request_document_links(cx);
                    }
                    if this.prefs.lsp_document_colors() != EditorDocumentColorMode::Off
                        && this.language_service_snapshot.capabilities.document_colors
                    {
                        this.request_language_service(LanguageServiceRequest::DocumentColors, cx);
                    }
                    this.schedule_inlay_hints(this.prefs.inlay_edit_debounce_ms(), cx);
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
        let request_range = match &request {
            LanguageServiceRequest::InlayHints { range } => Some(*range),
            _ => None,
        };
        let (slot, request_position) = match &request {
            LanguageServiceRequest::Completion { position, .. } => {
                (LanguageServiceRequestSlot::Completion, Some(*position))
            }
            LanguageServiceRequest::SignatureHelp { position, .. } => {
                (LanguageServiceRequestSlot::SignatureHelp, Some(*position))
            }
            LanguageServiceRequest::Definition { position }
            | LanguageServiceRequest::References { position } => {
                (LanguageServiceRequestSlot::Navigation, Some(*position))
            }
            LanguageServiceRequest::TypeDefinition { position } => {
                (LanguageServiceRequestSlot::TypeDefinition, Some(*position))
            }
            LanguageServiceRequest::PrepareCallHierarchy { position } => {
                (LanguageServiceRequestSlot::CallHierarchy, Some(*position))
            }
            LanguageServiceRequest::IncomingCalls { .. }
            | LanguageServiceRequest::OutgoingCalls { .. } => {
                (LanguageServiceRequestSlot::CallHierarchy, None)
            }
            LanguageServiceRequest::CodeLenses
            | LanguageServiceRequest::ResolveCodeLens { .. }
            | LanguageServiceRequest::ExecuteCommand { .. } => {
                (LanguageServiceRequestSlot::CodeLens, None)
            }
            LanguageServiceRequest::DocumentLinks
            | LanguageServiceRequest::ResolveDocumentLink { .. } => {
                (LanguageServiceRequestSlot::DocumentLinks, None)
            }
            LanguageServiceRequest::InlayHints { .. } => {
                (LanguageServiceRequestSlot::InlayHints, None)
            }
            LanguageServiceRequest::FoldingRanges => {
                (LanguageServiceRequestSlot::FoldingRanges, None)
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
            LanguageServiceRequestSlot::Navigation | LanguageServiceRequestSlot::TypeDefinition => {
                self.navigation_request_serial = Some(request_serial);
                self.navigation_position = request_position;
            }
            LanguageServiceRequestSlot::InlayHints => {
                self.inlay_hints_request_serial = Some(request_serial);
                self.inlay_hints_request_range = request_range;
            }
            LanguageServiceRequestSlot::FoldingRanges => {
                self.folding_ranges_request_serial = Some(request_serial);
            }
            LanguageServiceRequestSlot::CallHierarchy => {
                self.call_hierarchy_request_serial = Some(request_serial);
            }
            LanguageServiceRequestSlot::CodeLens => {
                self.code_lens_request_serial = Some(request_serial);
            }
            LanguageServiceRequestSlot::DocumentLinks => {
                self.document_link_request_serial = Some(request_serial);
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
                        && (matches!(
                            slot,
                            LanguageServiceRequestSlot::Completion
                                | LanguageServiceRequestSlot::SignatureHelp
                                | LanguageServiceRequestSlot::TypeDefinition
                                | LanguageServiceRequestSlot::CallHierarchy
                                | LanguageServiceRequestSlot::CodeLens
                                | LanguageServiceRequestSlot::DocumentLinks
                        ) || this.format_on_save_in_flight))
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
            LanguageServiceRequestSlot::Navigation | LanguageServiceRequestSlot::TypeDefinition => {
                self.navigation_request_serial == Some(request_serial)
                    && self.navigation_position == request_position
                    && request_position
                        .is_some_and(|position| self.current_lsp_position() == position)
            }
            LanguageServiceRequestSlot::CallHierarchy => {
                self.call_hierarchy_request_serial == Some(request_serial)
                    && request_position
                        .is_none_or(|position| self.current_lsp_position() == position)
            }
            LanguageServiceRequestSlot::CodeLens => {
                self.code_lens_request_serial == Some(request_serial)
            }
            LanguageServiceRequestSlot::DocumentLinks => {
                self.document_link_request_serial == Some(request_serial)
            }
            LanguageServiceRequestSlot::InlayHints => {
                self.inlay_hints_request_serial == Some(request_serial)
                    && self.inlay_hints_request_range == Some(self.visible_inlay_range())
            }
            LanguageServiceRequestSlot::FoldingRanges => {
                self.folding_ranges_request_serial == Some(request_serial)
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
                let intent = self.save_intent_after_format.unwrap_or(SaveIntent::Normal);
                notify_info(
                    cx,
                    "Format on save",
                    "The language service could not format this document; saving the current text.",
                );
                self.save_with_intent(intent, cx);
            } else if slot == LanguageServiceRequestSlot::TypeDefinition {
                notify(
                    cx,
                    "Type Definition unavailable",
                    &response_error_text(&response),
                );
            } else if slot == LanguageServiceRequestSlot::FoldingRanges {
                self.server_fold_ranges.clear();
                cx.notify();
            } else if slot == LanguageServiceRequestSlot::CallHierarchy {
                self.call_hierarchy_loading = false;
                notify(
                    cx,
                    "Call hierarchy unavailable",
                    &response_error_text(&response),
                );
                cx.notify();
            } else if slot == LanguageServiceRequestSlot::CodeLens {
                self.code_lenses.clear();
                notify(cx, "Code Lens unavailable", &response_error_text(&response));
                cx.notify();
            } else if slot == LanguageServiceRequestSlot::DocumentLinks {
                self.document_links.clear();
                cx.notify();
            }
            return;
        };
        match response {
            LanguageServiceResponse::Diagnostics(_) => {
                self.diagnostics_revision = Some(self.editor_revision());
                self.schedule_error_lens(cx);
            }
            LanguageServiceResponse::Definition(targets)
            | LanguageServiceResponse::TypeDefinition(targets)
            | LanguageServiceResponse::References(targets) => {
                if let Some(target) = targets.first() {
                    let position = target
                        .selection_range
                        .start
                        .to_position(&self.doc.snapshot());
                    self.goto_position(position, cx);
                }
            }
            LanguageServiceResponse::CallHierarchyItems(items) => {
                let Some(item) = items.into_iter().next() else {
                    self.call_hierarchy_root = None;
                    self.call_hierarchy_loading = false;
                    notify_info(cx, "Call hierarchy", "No callable symbol was found here.");
                    cx.notify();
                    return;
                };
                self.call_hierarchy_root = Some(item.clone());
                self.call_hierarchy_direction = CallHierarchyDirection::Incoming;
                self.call_hierarchy_loading = true;
                self.request_language_service(LanguageServiceRequest::IncomingCalls { item }, cx);
            }
            LanguageServiceResponse::CallHierarchyCalls(calls) => {
                self.call_hierarchy_calls = calls;
                self.call_hierarchy_loading = false;
                cx.notify();
            }
            LanguageServiceResponse::CodeLenses(lenses) => {
                self.code_lenses = if self.code_lens_enabled() {
                    lenses
                        .into_iter()
                        .filter(|lens| lens.command.is_some())
                        .collect()
                } else {
                    Vec::new()
                };
                cx.notify();
            }
            LanguageServiceResponse::DocumentLinks(links) => {
                self.document_links = links
                    .into_iter()
                    .filter(|link| link.target.is_some())
                    .collect();
                cx.notify();
            }
            LanguageServiceResponse::Formatting(edits) => {
                let save_after_format = self.format_on_save_in_flight;
                let intent = self.save_intent_after_format.unwrap_or(SaveIntent::Normal);
                self.apply_service_edits(edits, cx);
                if save_after_format {
                    // Re-enter save once with the formatting result already
                    // applied. `save` clears this guard before writing.
                    self.save_with_intent(intent, cx);
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
            LanguageServiceResponse::FoldingRanges(ranges) => {
                self.server_fold_ranges = fold_ranges_from_lsp(&ranges, &self.doc.snapshot());
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

    fn parse_completion_snippet(&self, template: &str) -> SnippetSession {
        let snapshot = self.doc.snapshot();
        let cursor = self.doc.cursor;
        let line = snapshot.line(cursor.line).to_owned();
        let current_word = word_at_cursor(&line, cursor.column).unwrap_or_default();
        let selected_text = self.doc.selected_text().unwrap_or_default();
        let file_path = self.doc.file_path().map(std::path::Path::to_path_buf);
        SnippetSession::parse_with_variables(template, |name| match name {
            "TM_FILENAME" => file_path.as_deref().and_then(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().into_owned())
            }),
            "TM_FILENAME_BASE" => file_path.as_deref().and_then(|path| {
                path.file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
            }),
            "TM_FILEPATH" => file_path
                .as_deref()
                .map(|path| path.to_string_lossy().into_owned()),
            "TM_DIRECTORY" => file_path.as_deref().and_then(|path| {
                path.parent()
                    .map(|value| value.to_string_lossy().into_owned())
            }),
            "TM_CURRENT_LINE" => Some(line.clone()),
            "TM_CURRENT_WORD" => Some(current_word.clone()),
            "TM_SELECTED_TEXT" => Some(selected_text.clone()),
            "TM_LINE_INDEX" => Some(cursor.line.to_string()),
            "TM_LINE_NUMBER" => Some(cursor.line.saturating_add(1).to_string()),
            _ => None,
        })
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
        let range = if let Some(lsp_range) = item.text_edit_range {
            let range = lsp_range.to_text_range(&snapshot);
            if labonair_editor::LspRange::from_text_range(&snapshot, range) != lsp_range {
                self.dismiss_completion();
                return;
            }
            range
        } else {
            self.completion_range(&snapshot)
        };
        if range.start > range.end || range.end > snapshot.byte_len() {
            self.dismiss_completion();
            return;
        }
        let before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let selections = self.doc.selection_set();
        let applied = if item.text_format == labonair_editor::CompletionTextFormat::Snippet {
            let snippet = self.parse_completion_snippet(&item.insert_text);
            self.doc.apply_snippet(range, snippet)
        } else {
            self.doc.apply_edits(
                vec![Edit::new(range, item.insert_text)],
                selections.clone(),
                labonair_editor::EditSource::Completion,
                Some("Completion".to_string()),
            )
        };
        if applied {
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

    fn schedule_inlay_hints(&mut self, delay_ms: u64, cx: &mut Context<Self>) {
        if !self.inlay_hints_enabled()
            || !self.language_service_open
            || !self.language_service_snapshot.capabilities.inlay_hints
        {
            return;
        }
        self.inlay_hints_request_serial = None;
        self.inlay_hints_request_range = None;
        self.inlay_hints_request = Some(cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(delay_ms))
                .await;
            let _ = view.update(cx, |this, cx| {
                this.inlay_hints_request = None;
                this.request_visible_inlay_hints(cx);
            });
        }));
    }

    fn request_visible_inlay_hints(&mut self, cx: &mut Context<Self>) {
        if !self.inlay_hints_enabled()
            || !self.language_service_open
            || !self.language_service_snapshot.capabilities.inlay_hints
        {
            return;
        }
        let range = self.visible_inlay_range();
        self.request_language_service(LanguageServiceRequest::InlayHints { range }, cx);
    }

    fn visible_inlay_range(&self) -> LspRange {
        let snapshot = self.doc.snapshot();
        let start_line = self.scroll_top.min(snapshot.line_count().saturating_sub(1));
        let end_line = start_line
            .saturating_add(self.visible_rows().max(1))
            .min(snapshot.line_count().saturating_sub(1));
        LspRange {
            start: LspPosition::from_position(&snapshot, Position::new(start_line, 0)),
            end: LspPosition::from_position(
                &snapshot,
                Position::new(end_line, snapshot.line_len(end_line)),
            ),
        }
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

    fn schedule_error_lens(&mut self, cx: &mut Context<Self>) {
        self.error_lens_task = None;
        self.error_lens_revision = None;
        if !self.inline_diagnostics_enabled() || !self.diagnostics_enabled() {
            cx.notify();
            return;
        }
        if self
            .language_service_state
            .snapshot()
            .diagnostics
            .is_empty()
        {
            cx.notify();
            return;
        }

        let revision = self.editor_revision();
        let delay = std::time::Duration::from_millis(self.prefs.error_lens_delay_ms());
        self.error_lens_task = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(delay).await;
            let _ = view.update(cx, |this, cx| {
                if this.editor_revision() != revision
                    || !this.inline_diagnostics_enabled()
                    || !this.diagnostics_enabled()
                {
                    return;
                }
                this.error_lens_revision = Some(revision);
                this.error_lens_task = None;
                cx.notify();
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

    fn submit_go_to_line_column(&mut self, value: &str, cx: &mut Context<Self>) {
        self.go_to_line_requested = false;
        self.go_to_line_input = None;
        if let Some(position) = parse_line_column(value) {
            self.goto_position(position, cx);
        } else {
            notify_info(
                cx,
                "Go to Line/Column",
                "Enter a positive line number, optionally followed by :column.",
            );
            cx.notify();
        }
    }

    fn cancel_go_to_line_column(&mut self, cx: &mut Context<Self>) {
        self.go_to_line_requested = false;
        self.go_to_line_input = None;
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

    fn ensure_go_to_line_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.go_to_line_requested || self.go_to_line_input.is_some() {
            return;
        }
        let input = cx.new(|cx| {
            text_field(window, cx)
                .placeholder("Line or line:column")
                .default_value("")
        });
        let view = cx.entity();
        window
            .subscribe(&input, cx, move |input, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { secondary: false } = event {
                    let value = input.read(cx).value().to_string();
                    view.update(cx, |this, cx| this.submit_go_to_line_column(&value, cx));
                }
            })
            .detach();
        window.focus(&input.read(cx).focus_handle(cx));
        self.go_to_line_input = Some(input);
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

    /// Recompute [`Self::hover`] for a mouse position over the text area.
    fn update_hover(&mut self, mouse_pos: Point<Pixels>, cx: &mut Context<Self>) {
        let snapshot = self.doc.snapshot();
        let doc_pos = self.position_at(mouse_pos);
        let language_state = self.language_service_state.snapshot();
        let diagnostic_hover = if self.diagnostics_enabled() {
            diagnostic_at_position(
                &language_state.diagnostics,
                &snapshot,
                doc_pos,
                self.prefs.diagnostics_max_severity(),
            )
            .map(diagnostic_hover_text)
        } else {
            None
        };
        let inlay_hover = self
            .inlay_hints_enabled()
            .then(|| {
                inlay_hint_tooltip_at_position(
                    &language_state.inlay_hints,
                    &snapshot,
                    doc_pos,
                    |kind| self.inlay_hint_kind_visible(kind),
                )
                .map(str::to_string)
            })
            .flatten();
        let color_hover = (self.prefs.lsp_document_colors() != EditorDocumentColorMode::Off)
            .then(|| {
                document_color_at_position(&language_state.document_colors, &snapshot, doc_pos)
                    .map(document_color_hover_text)
            })
            .flatten();
        let document_link_hover = self.document_link_at_position(doc_pos).and_then(|link| {
            link.target.as_ref().map(|target| {
                let description = link
                    .tooltip
                    .clone()
                    .unwrap_or_else(|| format!("Open {target}"));
                format!("{description} · Ctrl/Cmd-click to open")
            })
        });
        let next = diagnostic_hover
            .map(|text| (mouse_pos, SharedString::from(text)))
            .or_else(|| inlay_hover.map(|text| (mouse_pos, SharedString::from(text))))
            .or_else(|| color_hover.map(|text| (mouse_pos, SharedString::from(text))))
            .or_else(|| document_link_hover.map(|text| (mouse_pos, SharedString::from(text))))
            .or_else(|| {
                if self.is_settings_json() {
                    let offset = snapshot.position_to_byte(doc_pos);
                    let text = snapshot.text();
                    labonair_settings_json::json_path_at_offset(&text, offset).and_then(|path| {
                        let refs: Vec<&str> = path.iter().map(String::as_str).collect();
                        labonair_settings::description_for_path(&refs)
                            .map(|desc| (mouse_pos, SharedString::from(desc)))
                    })
                } else {
                    None
                }
            });
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
        let inlay_hints_were_enabled = self.inlay_hints_enabled();
        let inlay_hints_setting_changed = p.inlay_hints() != self.prefs.inlay_hints();
        let code_lens_setting_changed = p.code_lens() != self.prefs.code_lens();
        let document_color_mode_changed =
            p.lsp_document_colors() != self.prefs.lsp_document_colors();
        let modal_mode_setting_changed =
            p.vim_mode() != self.prefs.vim_mode() || p.helix_mode() != self.prefs.helix_mode();
        let error_lens_was_enabled =
            self.inline_diagnostics_enabled() && self.diagnostics_enabled();
        let error_lens_delay_changed = p.error_lens_delay_ms() != self.prefs.error_lens_delay_ms();
        if p.diagnostics() != self.prefs.diagnostics() {
            self.diagnostics_enabled_override = None;
        }
        if p.error_lens() != self.prefs.error_lens() {
            self.inline_diagnostics_enabled_override = None;
        }
        if p.line_numbers() != self.prefs.line_numbers() {
            self.line_numbers_enabled_override = None;
        }
        if p.minimap_visibility() != self.prefs.minimap_visibility() {
            self.minimap_enabled_override = None;
        }
        if p.semantic_tokens() != self.prefs.semantic_tokens() {
            self.semantic_tokens_enabled_override = None;
        }
        if p.selections_menu() != self.prefs.selections_menu() {
            self.selections_menu_enabled_override = None;
        }
        if code_lens_setting_changed {
            self.code_lens_enabled_override = None;
        }
        if modal_mode_setting_changed {
            self.modal_mode_override = None;
        }
        self.prefs = p;
        if inlay_hints_setting_changed {
            self.inlay_hints_enabled_override = None;
        }
        if !self.prefs.toolbar_quick_actions() && self.buffer_search_open {
            self.buffer_search_open = false;
            self.search_close(cx);
        }
        if !self.prefs.toolbar_quick_actions() {
            self.editor_controls_anchor = None;
        }
        if !self.selections_menu_enabled() {
            self.selection_menu_anchor = None;
        }
        if !self.inline_diagnostics_enabled() || !self.diagnostics_enabled() {
            self.error_lens_task = None;
            self.error_lens_revision = None;
        } else if !error_lens_was_enabled || error_lens_delay_changed {
            self.schedule_error_lens(cx);
        }
        self.cursor_blink_on = true;
        self.language_services.set_presentation_policy(
            self.prefs.diagnostics(),
            self.prefs.completion(),
            self.prefs.hover(),
            self.prefs.semantic_tokens(),
        );
        match self.active_modal_mode() {
            EditorModalMode::Normal => {
                self.vim = None;
                self.helix = None;
            }
            EditorModalMode::Vim => {
                self.helix = None;
                match &mut self.vim {
                    Some(vim) => vim.options = vim_options(&self.prefs),
                    None => self.vim = Some(Vim::new(vim_options(&self.prefs))),
                }
            }
            EditorModalMode::Helix => {
                self.vim = None;
                self.helix.get_or_insert_with(Helix::new);
            }
        }
        let inlay_hints_are_enabled = self.inlay_hints_enabled();
        if inlay_hints_were_enabled != inlay_hints_are_enabled {
            if inlay_hints_are_enabled {
                self.schedule_inlay_hints(0, cx);
            } else {
                self.inlay_hints_request = None;
                self.inlay_hints_request_serial = None;
                self.inlay_hints_request_range = None;
                self.language_service_state.clear_inlay_hints();
            }
        }
        if code_lens_setting_changed {
            if self.code_lens_enabled()
                && self.language_service_snapshot.capabilities.code_lens
                && self.language_service_snapshot.capabilities.execute_command
            {
                self.request_code_lenses(cx);
            } else {
                self.code_lenses.clear();
                self.code_lens_request_serial = None;
            }
        }
        if document_color_mode_changed {
            if self.prefs.lsp_document_colors() == EditorDocumentColorMode::Off {
                self.language_service_state.clear_document_colors();
            } else if self.language_service_snapshot.capabilities.document_colors {
                self.request_language_service(LanguageServiceRequest::DocumentColors, cx);
            }
        }
        self.language_service_snapshot = self.language_services.snapshot(
            self.doc.language,
            DocumentVersion::from_revision(self.editor_revision()),
            self.language_service_generation,
        );
        self.schedule_autosave(cx);
        cx.notify();
    }

    fn breadcrumbs_visible(&self) -> bool {
        self.breadcrumb_visible_override
            .unwrap_or_else(|| self.prefs.toolbar_breadcrumbs())
    }

    fn diagnostics_enabled(&self) -> bool {
        self.diagnostics_enabled_override
            .unwrap_or_else(|| self.prefs.diagnostics())
    }

    fn inline_diagnostics_enabled(&self) -> bool {
        self.inline_diagnostics_enabled_override
            .unwrap_or_else(|| self.prefs.error_lens())
    }

    fn line_numbers_enabled(&self) -> bool {
        self.line_numbers_enabled_override.unwrap_or_else(|| {
            self.vim
                .as_ref()
                .map(|vim| vim.options.number)
                .unwrap_or_else(|| self.prefs.line_numbers())
        })
    }

    fn minimap_visibility(&self) -> EditorMinimapVisibility {
        self.minimap_enabled_override
            .map(|enabled| {
                if enabled {
                    EditorMinimapVisibility::Always
                } else {
                    EditorMinimapVisibility::Never
                }
            })
            .unwrap_or_else(|| self.prefs.minimap_visibility())
    }

    fn minimap_enabled(&self) -> bool {
        self.minimap_enabled_override
            .unwrap_or_else(|| self.prefs.minimap())
    }

    fn semantic_tokens_enabled(&self) -> bool {
        self.semantic_tokens_enabled_override
            .unwrap_or_else(|| self.prefs.semantic_tokens())
    }

    fn selections_menu_enabled(&self) -> bool {
        self.selections_menu_enabled_override
            .unwrap_or_else(|| self.prefs.selections_menu())
    }

    fn editor_control_enabled(&self, control: EditorControl) -> bool {
        match control {
            EditorControl::Diagnostics => self.diagnostics_enabled(),
            EditorControl::InlineDiagnostics => self.inline_diagnostics_enabled(),
            EditorControl::GitBlame => self.git_blame_enabled,
            EditorControl::LineNumbers => self.line_numbers_enabled(),
            EditorControl::Minimap => self.minimap_enabled(),
            EditorControl::SelectionMenu => self.selections_menu_enabled(),
            EditorControl::InlayHints => self.inlay_hints_enabled(),
            EditorControl::CodeLens => self.code_lens_enabled(),
            EditorControl::SemanticHighlights => self.semantic_tokens_enabled(),
            EditorControl::VimMode => self.vim.is_some(),
            EditorControl::HelixMode => self.helix.is_some(),
        }
    }

    fn active_modal_mode(&self) -> EditorModalMode {
        self.modal_mode_override.unwrap_or_else(|| {
            if self.prefs.vim_mode() {
                EditorModalMode::Vim
            } else if self.prefs.helix_mode() {
                EditorModalMode::Helix
            } else {
                EditorModalMode::Normal
            }
        })
    }

    fn toggle_editor_control(&mut self, control: EditorControl, cx: &mut Context<Self>) {
        match control {
            EditorControl::Diagnostics => {
                let enabled = !self.diagnostics_enabled();
                self.diagnostics_enabled_override = Some(enabled);
                if !enabled {
                    self.error_lens_task = None;
                    self.error_lens_revision = None;
                } else if self.inline_diagnostics_enabled() {
                    self.schedule_error_lens(cx);
                }
            }
            EditorControl::InlineDiagnostics => {
                let enabled = !self.inline_diagnostics_enabled();
                self.inline_diagnostics_enabled_override = Some(enabled);
                if !enabled || !self.diagnostics_enabled() {
                    self.error_lens_task = None;
                    self.error_lens_revision = None;
                } else {
                    self.schedule_error_lens(cx);
                }
            }
            EditorControl::GitBlame => {
                self.git_blame_enabled = !self.git_blame_enabled;
                if self.git_blame_enabled {
                    self.schedule_git_blame(cx);
                } else {
                    self.git_blame_generation = self.git_blame_generation.wrapping_add(1);
                    self.git_blame_task = None;
                    self.git_blame_request = None;
                    self.git_blame = None;
                }
            }
            EditorControl::LineNumbers => {
                self.line_numbers_enabled_override = Some(!self.line_numbers_enabled());
            }
            EditorControl::Minimap => {
                self.minimap_enabled_override = Some(!self.minimap_enabled());
            }
            EditorControl::SelectionMenu => {
                let enabled = !self.selections_menu_enabled();
                self.selections_menu_enabled_override = Some(enabled);
                if !enabled {
                    self.selection_menu_anchor = None;
                }
            }
            EditorControl::InlayHints => self.toggle_inlay_hints(cx),
            EditorControl::CodeLens => {
                let enabled = !self.code_lens_enabled();
                self.code_lens_enabled_override = Some(enabled);
                if enabled
                    && self.language_service_snapshot.capabilities.code_lens
                    && self.language_service_snapshot.capabilities.execute_command
                {
                    self.request_code_lenses(cx);
                } else {
                    self.code_lenses.clear();
                    self.code_lens_request_serial = None;
                }
            }
            EditorControl::SemanticHighlights => {
                self.semantic_tokens_enabled_override = Some(!self.semantic_tokens_enabled());
            }
            EditorControl::VimMode => {
                self.set_modal_mode(if self.active_modal_mode() == EditorModalMode::Vim {
                    EditorModalMode::Normal
                } else {
                    EditorModalMode::Vim
                });
            }
            EditorControl::HelixMode => {
                self.set_modal_mode(if self.active_modal_mode() == EditorModalMode::Helix {
                    EditorModalMode::Normal
                } else {
                    EditorModalMode::Helix
                });
            }
        }
        cx.notify();
    }

    fn set_modal_mode(&mut self, mode: EditorModalMode) {
        self.modal_mode_override = Some(mode);
        self.vim = (mode == EditorModalMode::Vim).then(|| Vim::new(vim_options(&self.prefs)));
        self.helix = (mode == EditorModalMode::Helix).then(Helix::new);
    }

    fn inlay_hints_enabled(&self) -> bool {
        self.inlay_hints_enabled_override
            .unwrap_or_else(|| self.prefs.inlay_hints())
    }

    fn toggle_inlay_hints(&mut self, cx: &mut Context<Self>) {
        let enabled = !self.inlay_hints_enabled();
        self.inlay_hints_enabled_override = Some(enabled);
        if enabled {
            self.schedule_inlay_hints(0, cx);
        } else {
            self.inlay_hints_request = None;
            self.inlay_hints_request_serial = None;
            self.inlay_hints_request_range = None;
            self.language_service_state.clear_inlay_hints();
            cx.notify();
        }
    }

    fn code_lens_enabled(&self) -> bool {
        self.code_lens_enabled_override
            .unwrap_or_else(|| self.prefs.code_lens())
    }

    fn request_code_lenses(&mut self, cx: &mut Context<Self>) {
        if !self.code_lens_enabled()
            || !self.language_service_snapshot.capabilities.code_lens
            || !self.language_service_snapshot.capabilities.execute_command
        {
            return;
        }
        self.request_language_service(LanguageServiceRequest::CodeLenses, cx);
    }

    fn request_document_links(&mut self, cx: &mut Context<Self>) {
        if !self.language_service_snapshot.capabilities.document_links {
            return;
        }
        self.request_language_service(LanguageServiceRequest::DocumentLinks, cx);
    }

    fn document_link_at_position(&self, position: Position) -> Option<&DocumentLink> {
        let snapshot = self.doc.snapshot();
        document_link_at_position(&self.document_links, &snapshot, position)
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
            "selection" => WhitespaceMode::Selection,
            "boundary" => WhitespaceMode::Boundary,
            _ => WhitespaceMode::None,
        };
        DisplayConfig {
            wrap_columns: (self.wrap_cols() > 0).then_some(self.wrap_cols()),
            show_line_numbers: self.line_numbers_enabled(),
            relative_line_numbers: self
                .vim
                .as_ref()
                .map(|vim| vim.options.relativenumber)
                .unwrap_or_else(|| self.prefs.relative_line_numbers()),
            indentation_guides: self.prefs.indentation_guides(),
            whitespace,
            whitespace_space_marker: self.prefs.whitespace_space_marker(),
            whitespace_tab_marker: self.prefs.whitespace_tab_marker(),
            scroll_beyond_last_line: self.prefs.scroll_beyond_last_line(),
        }
    }

    fn display_layout(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        config: DisplayConfig,
    ) -> &DisplaySnapshot {
        let service_state = self.language_service_state.snapshot();
        let mut inlay_hints = if self.inlay_hints_enabled() {
            service_state
                .inlay_hints
                .into_iter()
                .filter(|hint| self.inlay_hint_kind_visible(hint.kind))
                .collect()
        } else {
            Vec::new()
        };
        if self.prefs.lsp_document_colors() == EditorDocumentColorMode::Inlay
            && service_state.version == DocumentVersion::from_revision(snapshot.revision())
        {
            inlay_hints.extend(
                service_state
                    .document_colors
                    .into_iter()
                    .map(|color| InlayHint {
                        position: color.range.end,
                        label: " ".into(),
                        kind: InlayHintKind::Other,
                        tooltip: None,
                        padding_left: false,
                        padding_right: false,
                    }),
            );
        }
        let needs_rebuild = self.display_layout_cache.as_ref().is_none_or(|cache| {
            cache.revision != snapshot.revision()
                || cache.config != config
                || cache.folds != self.folds
                || cache.inlay_hints != inlay_hints
        });
        if needs_rebuild {
            let hinted_columns = inlay_hints
                .iter()
                .map(|hint| {
                    hint.label.chars().count()
                        + usize::from(hint.padding_left)
                        + usize::from(hint.padding_right)
                })
                .sum::<usize>();
            let layout_height = snapshot
                .byte_len()
                .saturating_add(hinted_columns)
                .saturating_add(snapshot.line_count())
                .saturating_add(1);
            let layout = DisplayMap::build_with_inlay_hints(
                snapshot,
                Viewport {
                    height_rows: layout_height,
                    ..Default::default()
                },
                config,
                &self.folds,
                &inlay_hints,
            );
            self.display_layout_cache = Some(DisplayLayoutCache {
                revision: snapshot.revision(),
                config,
                folds: self.folds.clone(),
                inlay_hints,
                layout,
            });
        }
        &self
            .display_layout_cache
            .as_ref()
            .expect("display layout cache is populated")
            .layout
    }

    fn inlay_hint_kind_visible(&self, kind: InlayHintKind) -> bool {
        match kind {
            InlayHintKind::Type => self.prefs.inlay_type_hints(),
            InlayHintKind::Parameter => self.prefs.inlay_parameter_hints(),
            InlayHintKind::Other => self.prefs.inlay_other_hints(),
        }
    }

    /// Build the visible editor-owned display snapshot from the cached
    /// document-wide coordinate layout. Only the visible rows are cloned for
    /// this paint pass.
    fn display_snapshot(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
    ) -> labonair_editor::DisplaySnapshot {
        let config = self.display_config();
        let visible_rows = self.visible_rows().saturating_add(1);
        let width_columns = self.wrap_cols().max(self.content_width_columns()).max(1);
        let scroll_left = self.scroll_left;
        let scroll_top = self.scroll_top;
        let layout = self.display_layout(snapshot, config);
        let top_row = layout.logical_line_to_visual_row(scroll_top);
        layout.for_viewport(Viewport {
            top_row,
            left_column: scroll_left,
            height_rows: visible_rows,
            width_columns,
        })
    }

    /// Vertical caret motion across visual (wrapped) rows. Returns `false` when
    /// soft-wrap is off so the caller falls back to logical [`Motion`].
    fn wrap_vertical(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        if self.wrap_cols() == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let snapshot = self.doc.snapshot();
        let config = self.display_config();
        let layout = self.display_layout(&snapshot, config);
        let Some((visual_row, display_column)) = layout.position_to_display(cur) else {
            return false;
        };
        let target_row = if down {
            visual_row.saturating_add(1)
        } else {
            let Some(previous) = visual_row.checked_sub(1) else {
                return true;
            };
            previous
        };
        let Some(target) = layout.display_to_position(target_row, display_column) else {
            return true;
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible(cx);
        cx.notify();
        true
    }

    /// Home / End across the current visual row. Returns `false` when soft-wrap
    /// is off.
    fn wrap_horizontal(&mut self, end: bool, extend: bool, cx: &mut Context<Self>) -> bool {
        if self.wrap_cols() == 0 {
            return false;
        }
        let cur = self.doc.cursor;
        let snapshot = self.doc.snapshot();
        let config = self.display_config();
        let layout = self.display_layout(&snapshot, config);
        let Some((visual_row, _)) = layout.position_to_display(cur) else {
            return false;
        };
        let target_column = if end { layout.max_line_width } else { 0 };
        let Some(target) = layout.display_to_position(visual_row, target_column) else {
            return false;
        };
        self.doc.set_caret(target, extend);
        self.ensure_cursor_visible(cx);
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
        snapshot.views = self
            .editor_splits
            .groups()
            .into_iter()
            .map(|group| self.editor_group_view_snapshot(group))
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
        if let Some(language) = Language::from_label(&snapshot.language) {
            self.doc.set_language(language);
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
        let mut saved_views = snapshot
            .views
            .into_iter()
            .map(|view| (labonair_editor::EditorGroupId::new(view.group_id), view))
            .collect::<HashMap<_, _>>();
        self.editor_group_views.clear();
        let groups = self.editor_splits.groups();
        for group in &groups {
            self.doc.create_editor_group_view(*group);
            let view = saved_views
                .remove(group)
                .unwrap_or_else(|| EditorViewSession {
                    group_id: group.value(),
                    scroll_top: 0,
                    scroll_left: 0,
                    folds: Vec::new(),
                    selections: self.doc.editor_group_selection(*group).unwrap_or_default(),
                });
            self.doc
                .set_editor_group_selection(*group, view.selections.clone());
            self.editor_group_views.insert(*group, view);
        }
        let focused_group = self.editor_splits.focused_group();
        self.doc.activate_editor_group_view(focused_group);
        self.doc.retain_editor_group_views(&groups);
        self.restore_editor_group_view(focused_group);
        self.search = snapshot.search.map(|query| {
            let selection_seed = query
                .selection_range
                .or_else(|| self.search_selection_range());
            EditorSearch {
                query,
                selection_seed,
                matches: Vec::new(),
                active: None,
                error: None,
            }
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
        if let Some(symbols) = self.language_service_state.snapshot().document_symbols {
            return symbols;
        }
        let snapshot = self.doc.snapshot();
        labonair_editor::document_symbols(self.syntax.language(), &snapshot)
    }

    /// Current revision-bound diagnostics for the Editor-owned diagnostics
    /// palette page. Display severity settings do not filter this result list.
    pub fn diagnostics(&self) -> Vec<labonair_editor::Diagnostic> {
        self.language_service_state.snapshot().diagnostics
    }

    /// Move the caret to the start of `line0` (0-based, clamped) and scroll it
    /// into view — palette symbol jump.
    pub fn goto_line(&mut self, line0: usize, cx: &mut Context<Self>) {
        self.navigate_to_position(Position::new(line0, 0), cx);
    }

    /// Move the caret to a project-search hit. While a newly opened file is
    /// still loading, retain the position and apply it after the load commits.
    pub fn goto_position(&mut self, position: Position, cx: &mut Context<Self>) {
        if matches!(self.doc.file_state(), FileState::Loading) {
            self.pending_navigation = Some(position);
            return;
        }
        self.pending_lsp_navigation = None;
        self.navigate_to_position(position, cx);
    }

    /// Move to an LSP UTF-16 position, retaining it until the target document
    /// finishes loading when Workspace has just opened the result file.
    pub fn goto_lsp_position(&mut self, position: LspPosition, cx: &mut Context<Self>) {
        if matches!(self.doc.file_state(), FileState::Loading) {
            self.pending_lsp_navigation = Some(position);
            return;
        }
        self.pending_lsp_navigation = None;
        let buffer_position = position.to_position(&self.doc.snapshot());
        self.navigate_to_position(buffer_position, cx);
    }

    fn navigation_location(&self) -> NavigationLocation {
        NavigationLocation::new(self.doc.cursor, self.scroll_top, self.scroll_left)
    }

    fn navigate_to_position(&mut self, position: Position, cx: &mut Context<Self>) {
        if matches!(self.doc.file_state(), FileState::Loading) {
            self.pending_navigation = Some(position);
            return;
        }
        self.pending_navigation = None;
        let from = self.navigation_location();
        let snapshot = self.doc.snapshot();
        let line = position.line.min(snapshot.line_count().saturating_sub(1));
        let column = position.column.min(snapshot.line_len(line));
        self.doc.set_caret(Position::new(line, column), false);
        self.ensure_cursor_visible(cx);
        self.navigation_history
            .record_transition(from, self.navigation_location());
        self.cursor_blink_on = true;
        cx.emit(EditorEvent::Changed);
        cx.notify();
    }

    fn navigate_history_back(&mut self, cx: &mut Context<Self>) {
        if let Some(location) = self.navigation_history.back() {
            self.restore_navigation_location(location, cx);
        }
    }

    fn navigate_history_forward(&mut self, cx: &mut Context<Self>) {
        if let Some(location) = self.navigation_history.forward() {
            self.restore_navigation_location(location, cx);
        }
    }

    fn restore_navigation_location(
        &mut self,
        location: NavigationLocation,
        cx: &mut Context<Self>,
    ) {
        let snapshot = self.doc.snapshot();
        let line = location
            .position
            .line
            .min(snapshot.line_count().saturating_sub(1));
        let column = location.position.column.min(snapshot.line_len(line));
        self.doc.set_caret(Position::new(line, column), false);
        self.scroll_top = location.scroll_top;
        self.scroll_left = location.scroll_left;
        self.ensure_cursor_visible(cx);
        self.cursor_blink_on = true;
        cx.emit(EditorEvent::Changed);
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
        self.git_blame_generation = self.git_blame_generation.wrapping_add(1);
        self.git_blame_task = None;
        self.git_blame_request = None;
        self.git_blame = None;
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

    fn schedule_git_blame(&mut self, cx: &mut Context<Self>) {
        if !self.git_blame_enabled {
            self.git_blame = None;
            self.git_blame_request = None;
            return;
        }
        let Some(provider) = self.git_provider.clone() else {
            self.git_blame = None;
            self.git_blame_request = None;
            return;
        };
        let Some(path) = self.current_git_path() else {
            self.git_blame = None;
            self.git_blame_request = None;
            return;
        };
        let line = self.doc.cursor.line;
        let revision = self.editor_revision();
        let key = (path.clone(), line, revision);
        if self.git_blame_request.as_ref() == Some(&key) {
            return;
        }

        self.git_blame_generation = self.git_blame_generation.wrapping_add(1);
        let generation = self.git_blame_generation;
        self.git_blame_request = Some(key);
        self.git_blame = None;
        let session_id = self.git_session_id.clone();
        let request = GitBlameRequest {
            path: path.clone(),
            line,
            revision,
            session_id: session_id.clone(),
        };
        let blame = provider.blame(request);
        self.git_blame_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(450))
                .await;
            let result = blame.await;
            let _ = this.update(cx, |this, cx| {
                if this.git_blame_generation != generation
                    || this.editor_revision() != revision
                    || this.doc.cursor.line != line
                    || this.current_git_path().as_deref() != Some(path.as_str())
                    || this.git_session_id != session_id
                {
                    return;
                }
                this.git_blame_task = None;
                this.git_blame = result.ok().filter(|snapshot| {
                    snapshot.path == path && snapshot.line == line && snapshot.revision == revision
                });
                cx.notify();
            });
        }));
    }

    /// Request the current Git projection without doing Git work on the GPUI
    /// foreground thread. Every request carries the document path, revision,
    /// session and view generation so late results are discarded completely.
    fn refresh_git(&mut self, cx: &mut Context<Self>) {
        self.invalidate_git_requests();
        self.schedule_git_blame(cx);
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
        self.ensure_cursor_visible(cx);
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
            labonair_editor::lifecycle::Bom::Utf16Le => StorageBom::Utf16Le,
            labonair_editor::lifecycle::Bom::Utf16Be => StorageBom::Utf16Be,
        }
    }

    fn editor_bom(bom: StorageBom) -> labonair_editor::lifecycle::Bom {
        match bom {
            StorageBom::None => labonair_editor::lifecycle::Bom::None,
            StorageBom::Utf8 => labonair_editor::lifecycle::Bom::Utf8,
            StorageBom::Utf16Le => labonair_editor::lifecycle::Bom::Utf16Le,
            StorageBom::Utf16Be => labonair_editor::lifecycle::Bom::Utf16Be,
        }
    }

    fn editor_encoding(encoding: StorageEncoding) -> labonair_editor::lifecycle::Encoding {
        match encoding {
            StorageEncoding::Utf8 => labonair_editor::lifecycle::Encoding::Utf8,
            StorageEncoding::Utf16Le => labonair_editor::lifecycle::Encoding::Utf16Le,
            StorageEncoding::Utf16Be => labonair_editor::lifecycle::Encoding::Utf16Be,
            StorageEncoding::Windows1252 => labonair_editor::lifecycle::Encoding::Windows1252,
            StorageEncoding::ShiftJis => labonair_editor::lifecycle::Encoding::ShiftJis,
        }
    }

    fn storage_encoding(encoding: labonair_editor::lifecycle::Encoding) -> StorageEncoding {
        match encoding {
            labonair_editor::lifecycle::Encoding::Utf8 => StorageEncoding::Utf8,
            labonair_editor::lifecycle::Encoding::Utf16Le => StorageEncoding::Utf16Le,
            labonair_editor::lifecycle::Encoding::Utf16Be => StorageEncoding::Utf16Be,
            labonair_editor::lifecycle::Encoding::Windows1252 => StorageEncoding::Windows1252,
            labonair_editor::lifecycle::Encoding::ShiftJis => StorageEncoding::ShiftJis,
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
            Self::editor_encoding(snapshot.encoding),
            Self::editor_bom(snapshot.bom),
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
            | Err(error @ EditorFileError::InvalidEncodingData { .. })
            | Err(error @ EditorFileError::UnrepresentableText { .. })
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
        self.open_path_with_encoding(path, None, cx);
    }

    fn open_path_with_encoding(
        &mut self,
        path: PathBuf,
        encoding: Option<StorageEncoding>,
        cx: &mut Context<Self>,
    ) {
        self.close_language_service_document();
        self.line_ending_menu_anchor = None;
        self.encoding_menu_anchor = None;
        self.editor_controls_anchor = None;
        self.diagnostics_menu_anchor = None;
        self.language_service_menu_anchor = None;
        self.format_on_save_in_flight = false;
        self.save_intent_after_format = None;
        self.pending_navigation = None;
        self.pending_lsp_navigation = None;
        self.navigation_history.clear();
        self.doc = Document::loading(path.clone());
        self.reset_editor_group_views();
        self.recovery_prompt = None;
        self.invalidate_git_requests();
        self.git_gutter =
            GitGutterSnapshot::loading(path.to_string_lossy().to_string(), self.editor_revision());
        self.scroll_top = 0;
        let path_str = path.to_string_lossy().to_string();
        let max_bytes = self.file_limit_bytes();
        let load = cx.background_executor().spawn(async move {
            load_editor_file_lifecycle_with_encoding_sync(&path_str, Some(max_bytes), encoding)
        });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            let _ = this.update(cx, |this, cx| {
                let failed = result.is_err();
                this.doc = Self::document_from_load(path, result);
                this.reset_editor_group_views();
                if !failed {
                    this.apply_pending_session();
                    this.sync_language_service_document(cx);
                    if let Some(position) = this.pending_lsp_navigation.take() {
                        this.goto_lsp_position(position, cx);
                    } else if let Some(position) = this.pending_navigation.take() {
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
                this.emit_buffer_changed(cx);
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
        self.line_ending_menu_anchor = None;
        self.encoding_menu_anchor = None;
        self.editor_controls_anchor = None;
        self.diagnostics_menu_anchor = None;
        self.language_service_menu_anchor = None;
        self.format_on_save_in_flight = false;
        self.save_intent_after_format = None;
        self.invalidate_git_requests();
        self.git_gutter =
            GitGutterSnapshot::loading(path.to_string_lossy().to_string(), self.editor_revision());
        let path_str = path.to_string_lossy().to_string();
        let max_bytes = self.file_limit_bytes();
        let encoding = self
            .doc
            .file_format()
            .map(|format| Self::storage_encoding(format.encoding));
        let load = cx.background_executor().spawn(async move {
            load_editor_file_lifecycle_with_encoding_sync(&path_str, Some(max_bytes), encoding)
        });
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
                            this.emit_buffer_changed(cx);
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
                        this.emit_buffer_changed(cx);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    // ── Saving ──────────────────────────────────────────────────────────────

    pub fn save(&mut self, cx: &mut Context<Self>) {
        self.save_with_intent(SaveIntent::Normal, cx);
    }

    fn overwrite_external(&mut self, cx: &mut Context<Self>) {
        self.save_with_intent(SaveIntent::OverwriteExternal, cx);
    }

    fn save_with_intent(&mut self, intent: SaveIntent, cx: &mut Context<Self>) {
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
        if let Err(error) = self.doc.prepare_save(&file_snapshot.identity, intent) {
            notify(cx, "Save failed", &error.to_string());
            return;
        }
        if self.prefs.format_on_save() && !self.format_on_save_in_flight {
            self.apply_save_policy();
            self.format_on_save_in_flight = true;
            self.save_intent_after_format = Some(intent);
            self.request_language_service(LanguageServiceRequest::Formatting, cx);
            return;
        }
        let intent = if self.format_on_save_in_flight {
            self.save_intent_after_format.take().unwrap_or(intent)
        } else {
            intent
        };
        self.format_on_save_in_flight = false;
        let path_str = path.to_string_lossy().to_string();
        let revision_before_policy = self.editor_revision();
        self.apply_save_policy();
        if self.editor_revision() != revision_before_policy {
            self.refresh_git(cx);
        }
        let content = self.doc.text();
        let expected = Self::storage_identity(file_snapshot.identity.clone());
        let format = self.doc.file_format();
        let encoding = Self::storage_encoding(
            format
                .map(|format| format.encoding)
                .unwrap_or(file_snapshot.encoding),
        );
        let line_ending = Self::storage_line_ending(
            format
                .map(|format| format.line_ending)
                .unwrap_or(file_snapshot.line_ending),
        );
        let bom = Self::storage_bom(format.map(|format| format.bom).unwrap_or(file_snapshot.bom));
        let save_intent = match intent {
            SaveIntent::Normal => StorageSaveIntent::Normal,
            SaveIntent::OverwriteExternal => StorageSaveIntent::OverwriteExternal,
        };
        let write = cx.background_executor().spawn(async move {
            let current = stat_editor_file_sync(&path_str)?;
            if intent == SaveIntent::Normal && current.identity != expected {
                return Err(EditorFileError::ExternalChange {
                    expected,
                    actual: current.identity,
                });
            }
            save_editor_file_lifecycle_sync(
                &path_str,
                &content,
                encoding,
                line_ending,
                bom,
                &current.identity,
                save_intent,
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
        self.ensure_cursor_visible(cx);
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if !dirty_before && self.doc.is_dirty() {
            cx.emit(EditorEvent::Edited);
        }
        if self.editor_revision() != revision_before {
            self.emit_buffer_changed(cx);
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
        self.execute_command(EditorCommand::Edit(EditIntent::Move { motion, extend }), cx);
        self.dismiss_language_popovers();
    }

    /// The current selection text, if any (for "Ask AI about Selection").
    pub fn selected_text(&self) -> Option<String> {
        self.doc.selected_text().filter(|s| !s.is_empty())
    }

    /// Byte range of the current primary selection for buffer-search scope.
    pub fn search_selection_range(&self) -> Option<TextRange> {
        let (start, end) = self.doc.selection()?;
        let snapshot = self.doc.snapshot();
        let range = TextRange::new(
            snapshot.position_to_byte(start),
            snapshot.position_to_byte(end),
        );
        (!range.is_empty()).then_some(range)
    }

    /// Toggle selection-limited matching for the active buffer-search query.
    pub fn toggle_search_selection_scope(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(search) = &self.search else {
            return false;
        };
        let selection_range = if search.query.selection_range.is_some() {
            None
        } else {
            let Some(selection_range) = search.selection_seed else {
                return false;
            };
            Some(selection_range)
        };
        let query = search.query.clone().with_selection_range(selection_range);
        let _ = self.search_set(query, cx);
        true
    }

    fn copy(&self, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard_selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.clipboard_selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
            self.execute_command(EditorCommand::Edit(EditIntent::DeleteSelections), cx);
        }
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) {
            if !text.is_empty() {
                self.paste_text(&text, cx);
            }
        }
    }

    fn kill_ring_cut(&mut self, cx: &mut Context<Self>) {
        if !self.doc.can_edit() {
            return;
        }
        let snapshot = self.doc.snapshot();
        let text = self
            .doc
            .kill_to_line_end_ranges()
            .into_iter()
            .map(|range| snapshot.text_range(range))
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            return;
        }
        self.kill_ring.push(text);
        self.execute_command(EditorCommand::Edit(EditIntent::KillToLineEnd), cx);
    }

    fn kill_ring_yank(&mut self, cx: &mut Context<Self>) {
        if !self.doc.can_edit() {
            return;
        }
        let Some(text) = self.kill_ring.yank() else {
            return;
        };
        let replacements = vec![text; self.doc.selection_set().len()];
        self.execute_command(
            EditorCommand::Edit(EditIntent::InsertMany(replacements)),
            cx,
        );
    }

    fn clipboard_selection_text(&self) -> Option<String> {
        let snapshot = self.doc.snapshot();
        let selected = self
            .doc
            .selection_set()
            .ranges(&snapshot)
            .into_iter()
            .filter(|range| !range.is_empty())
            .map(|range| snapshot.text_range(range))
            .collect::<Vec<_>>();
        (!selected.is_empty()).then(|| selected.join("\n"))
    }

    fn paste_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let selection_count = self.doc.selection_set().len();
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let lines = normalized
            .split('\n')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let replacements = if selection_count > 1 && lines.len() == selection_count {
            lines
        } else {
            vec![normalized; selection_count]
        };
        self.execute_command(
            EditorCommand::Edit(EditIntent::InsertMany(replacements)),
            cx,
        );
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
        let config = self.display_config();
        let max_line_width = self.display_layout(&snapshot, config).max_line_width;
        let max_scroll_line = self.max_scroll_line_for_snapshot(&snapshot, config);
        self.scroll_top = self.scroll_top.min(max_scroll_line);
        let max_left = if config.wrap_columns.is_none() {
            max_line_width.saturating_sub(self.content_width_columns())
        } else {
            0
        };
        self.scroll_left = self.scroll_left.min(max_left);
    }

    fn max_scroll_line(&mut self) -> usize {
        let snapshot = self.doc.snapshot();
        self.max_scroll_line_for_snapshot(&snapshot, self.display_config())
    }

    fn max_scroll_line_for_snapshot(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        config: DisplayConfig,
    ) -> usize {
        if config.scroll_beyond_last_line {
            return snapshot.line_count().saturating_sub(1);
        }
        let visible_rows = self.visible_rows().max(1);
        let layout = self.display_layout(snapshot, config);
        let max_row = layout.total_rows.saturating_sub(visible_rows);
        layout
            .display_to_position(max_row, 0)
            .map(|position| position.line)
            .unwrap_or_else(|| snapshot.line_count().saturating_sub(1))
    }

    fn ensure_cursor_visible(&mut self, cx: &mut Context<Self>) {
        let old_scroll_top = self.scroll_top;
        let rows = self.visible_rows();
        let line = self.doc.cursor.line;
        if line < self.scroll_top {
            self.scroll_top = line;
        } else if line >= self.scroll_top + rows {
            self.scroll_top = line + 1 - rows;
        }
        self.clamp_scroll();
        if self.scroll_top != old_scroll_top {
            self.schedule_inlay_hints(self.prefs.inlay_scroll_debounce_ms(), cx);
        }
        self.schedule_git_blame(cx);
    }

    fn scroll_by(&mut self, lines: isize, cx: &mut Context<Self>) {
        let max = self.max_scroll_line() as isize;
        let next = (self.scroll_top as isize + lines).clamp(0, max);
        if next as usize != self.scroll_top {
            self.scroll_top = next as usize;
            self.schedule_inlay_hints(self.prefs.inlay_scroll_debounce_ms(), cx);
            cx.notify();
        }
    }

    fn scroll_to_visual_row(&mut self, row: usize, cx: &mut Context<Self>) {
        let snapshot = self.doc.snapshot();
        let config = self.display_config();
        let layout = self.display_layout(&snapshot, config);
        let next = layout
            .display_to_position(row.min(layout.total_rows.saturating_sub(1)), 0)
            .map(|position| position.line)
            .unwrap_or_else(|| snapshot.line_count().saturating_sub(1));
        if next != self.scroll_top {
            self.scroll_top = next;
            self.clamp_scroll();
            self.schedule_inlay_hints(self.prefs.inlay_scroll_debounce_ms(), cx);
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

    fn execute_fold_command(&mut self, command: EditorUiCommand, cx: &mut Context<Self>) {
        let snapshot = self.doc.snapshot();
        let symbols = self.symbols_for_snapshot(&snapshot);
        let syntax_ranges = self.syntax_folding_ranges_for_snapshot(&snapshot);
        let candidates = fold_candidates(
            &self.server_fold_ranges,
            &symbols,
            &syntax_ranges,
            &snapshot,
        );
        let before = self.folds.clone();
        match command {
            EditorUiCommand::Fold => {
                let Some(range) = fold_range_for_cursor(&candidates, self.doc.cursor.line) else {
                    return;
                };
                self.toggle_fold_range(range, cx);
                return;
            }
            EditorUiCommand::FoldAll => self.folds = candidates,
            EditorUiCommand::FoldAtLevel(level) => {
                self.folds = fold_ranges_at_level(&candidates, level)
            }
            EditorUiCommand::FoldRecursive => {
                let Some(outer) = fold_range_for_cursor(&candidates, self.doc.cursor.line) else {
                    return;
                };
                self.folds.extend(fold_ranges_within(&candidates, outer));
                self.folds
                    .sort_by_key(|range| (range.start_line, range.end_line));
                self.folds.dedup();
            }
            EditorUiCommand::UnfoldAll => self.folds.clear(),
            EditorUiCommand::UnfoldLines => {
                let line_ranges = self
                    .doc
                    .selection_set()
                    .ranges(&snapshot)
                    .into_iter()
                    .map(|range| {
                        let start = snapshot.byte_to_position(range.start).line;
                        let mut end = snapshot.byte_to_position(range.end).line;
                        let end_position = snapshot.byte_to_position(range.end);
                        if !range.is_empty() && end > start && end_position.column == 0 {
                            end -= 1;
                        }
                        (start, end)
                    })
                    .collect::<Vec<_>>();
                self.folds.retain(|fold| {
                    !line_ranges
                        .iter()
                        .any(|(start, end)| fold.start_line <= *end && fold.end_line >= *start)
                });
            }
            EditorUiCommand::UnfoldRecursive => {
                let line = self.doc.cursor.line;
                if let Some(outer) = fold_range_for_cursor(&candidates, line) {
                    self.folds
                        .retain(|fold| !(*fold == outer || fold_contains(outer, *fold)));
                } else {
                    self.folds
                        .retain(|fold| !(fold.start_line <= line && line <= fold.end_line));
                }
            }
            _ => return,
        }
        if self.folds != before {
            self.clamp_scroll();
            cx.notify();
        }
    }

    // ── Find (driven by the workspace search overlay, T18-002) ──────────────

    /// A one-line pre-fill for the search overlay (current selection).
    pub fn search_seed(&self) -> Option<String> {
        let selection = self
            .doc
            .selected_text()
            .filter(|s| !s.is_empty() && !s.contains('\n'));
        match self.prefs.search_seed_from_cursor() {
            SearchSeedBehavior::Always => selection.or_else(|| {
                let cursor = self.doc.cursor;
                let line = self.doc.snapshot().line(cursor.line);
                word_at_cursor(&line, cursor.column)
            }),
            SearchSeedBehavior::Selection => selection,
            SearchSeedBehavior::Never => None,
        }
    }

    fn ensure_buffer_search_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.buffer_search_input.is_some() {
            return;
        }
        let seed = self.search_seed().unwrap_or_default();
        let input = cx.new(|cx| {
            text_field(window, cx)
                .placeholder("Find in buffer…")
                .default_value(seed)
        });
        let editor = cx.entity();
        window
            .subscribe(
                &input,
                cx,
                move |input, event: &InputEvent, _window, cx| match event {
                    InputEvent::Change => {
                        let query = input.read(cx).value().to_string();
                        editor.update(cx, |this, cx| {
                            let _ = this
                                .search_set(SearchQuery::new(query, SearchOptions::default()), cx);
                        });
                    }
                    InputEvent::PressEnter { secondary } => {
                        editor.update(cx, |this, cx| {
                            let _ = this.search_step(!*secondary, cx);
                        });
                    }
                    _ => {}
                },
            )
            .detach();
        self.buffer_search_input = Some(input);
    }

    fn toggle_buffer_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.buffer_search_open {
            self.buffer_search_open = false;
            self.search_close(cx);
            self.focus(window);
        } else {
            self.ensure_buffer_search_input(window, cx);
            self.buffer_search_open = true;
            if let Some(input) = self.buffer_search_input.clone() {
                let query = input.read(cx).value().to_string();
                let focus_handle = input.read(cx).focus_handle(cx);
                if !query.is_empty() {
                    let _ = self.search_set(SearchQuery::new(query, SearchOptions::default()), cx);
                }
                window.focus(&focus_handle);
            }
        }
        cx.notify();
    }

    /// Start / update the Editor-owned search contract. Workspace only forwards
    /// this typed query from its overlay; it never constructs match state.
    pub fn search_set(
        &mut self,
        query: SearchQuery,
        cx: &mut Context<Self>,
    ) -> Result<(usize, usize), SearchError> {
        self.project_search.cancel();
        let selection_seed = self
            .search
            .as_ref()
            .and_then(|search| search.selection_seed)
            .or_else(|| self.search_selection_range());
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
                    selection_seed,
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
            selection_seed,
            matches: result.matches,
            active,
            error: None,
        });
        if let Some(active) = active {
            self.select_search_match(active, cx);
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
        self.select_search_match(idx, cx);
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

    pub fn begin_project_replace(
        &mut self,
        replacement: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Result<ProjectReplaceRequest, SearchError> {
        let request = self.project_search.begin_replace(replacement)?;
        cx.notify();
        Ok(request)
    }

    pub fn complete_project_replace(
        &mut self,
        plan: &ProjectReplacePlan,
        replacements: usize,
        cx: &mut Context<Self>,
    ) -> Result<ProjectReplaceResult, SearchError> {
        if plan.replacements != replacements {
            let error = SearchError {
                message: "project replacement count changed while applying the plan".to_string(),
            };
            self.project_search
                .fail_replace(plan.generation, error.message.clone());
            cx.notify();
            return Err(error);
        }
        let result = ProjectReplaceResult {
            generation: plan.generation,
            matched_paths: plan.matched_paths.clone(),
            files_changed: plan.targets.len(),
            replacements,
        };
        if !self.project_search.accept_replace(result.clone()) {
            return Err(SearchError {
                message: "project replacement request is stale".to_string(),
            });
        }
        cx.notify();
        Ok(result)
    }

    pub fn fail_project_replace(
        &mut self,
        generation: u64,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> bool {
        let accepted = self.project_search.fail_replace(generation, message);
        if accepted {
            cx.notify();
        }
        accepted
    }

    pub fn project_buffer_candidate(&self) -> Option<ProjectBufferCandidate> {
        Some(ProjectBufferCandidate {
            path: self.path()?.to_string_lossy().into_owned(),
            snapshot: self.doc.snapshot(),
            can_edit: self.doc.can_edit(),
        })
    }

    pub fn plan_project_replace(
        &self,
        request: &ProjectReplaceRequest,
        buffers: &[ProjectBufferCandidate],
    ) -> Result<ProjectReplacePlan, SearchError> {
        let snapshot = self.project_search.snapshot();
        if snapshot.generation != request.generation
            || snapshot.replacement_status != labonair_editor::ProjectReplaceStatus::Applying
        {
            return Err(SearchError {
                message: "project replacement request is stale".to_string(),
            });
        }
        labonair_editor::plan_project_replace(request, &snapshot.hits, buffers)
    }

    pub fn apply_project_replace_target(
        &mut self,
        request: &ProjectReplaceRequest,
        target: &ProjectReplaceTarget,
        cx: &mut Context<Self>,
    ) -> Result<usize, SearchError> {
        if self
            .path()
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned())
            != Some(target.path.clone())
            || self.editor_revision() != target.revision
        {
            return Err(SearchError {
                message: format!(
                    "{} changed before replacement could be applied",
                    target.path
                ),
            });
        }
        if !self.doc.can_edit() {
            return Err(SearchError {
                message: format!("{} is read-only", target.path),
            });
        }
        let count = self
            .doc
            .replace_project_all_checked(&request.query, &request.replacement)?;
        if count != target.replacements {
            return Err(SearchError {
                message: format!(
                    "{} changed before replacement could be applied",
                    target.path
                ),
            });
        }
        if count > 0 {
            self.search = None;
            self.clamp_scroll();
            self.resync_syntax();
            self.sync_language_service_document(cx);
            self.refresh_git(cx);
            cx.emit(EditorEvent::Changed);
            cx.notify();
        }
        Ok(count)
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

    /// Current byte range used by selection-scoped buffer search.
    pub fn search_selection_scope(&self) -> Option<TextRange> {
        self.search
            .as_ref()
            .and_then(|search| search.query.selection_range)
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
        let snapshot = self.doc.snapshot();
        let edits = replace_one(&snapshot, &query, replacement, active)?
            .into_iter()
            .collect::<Vec<_>>();
        if edits.is_empty() {
            return Ok(false);
        }
        let replaced = self.doc.replace_one(&query, replacement, active)?;
        if replaced {
            if let Some(search) = &mut self.search {
                search.query.map_selection_range_through_edits(&edits);
                search.selection_seed = search
                    .selection_seed
                    .map(|range| map_range_through_edits(range, &edits));
            }
            self.refresh_matches();
            if let Some(active) = self.search.as_ref().and_then(|search| search.active) {
                self.select_search_match(active, cx);
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
        let snapshot = self.doc.snapshot();
        let edits = replace_all_checked(&snapshot, &query, replacement)?;
        if edits.is_empty() {
            return Ok(0);
        }
        let count = self.doc.replace_all_checked(&query, replacement)?;
        if count > 0 {
            if let Some(search) = &mut self.search {
                search.query.map_selection_range_through_edits(&edits);
                search.selection_seed = search
                    .selection_seed
                    .map(|range| map_range_through_edits(range, &edits));
            }
            self.refresh_matches();
            if let Some(active) = self.search.as_ref().and_then(|search| search.active) {
                self.select_search_match(active, cx);
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

    fn select_search_match(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(s) = &mut self.search else { return };
        let Some(m) = s.matches.get(idx).copied() else {
            return;
        };
        s.active = Some(idx);
        self.doc.set_caret(m.start, false);
        self.doc.set_caret(m.end, true);
        if self.prefs.search_center_on_match() {
            let scroll_line = {
                let snapshot = self.doc.snapshot();
                let config = self.display_config();
                let visible_rows = self.visible_rows();
                let layout = self.display_layout(&snapshot, config);
                centered_search_scroll_line(layout, m.start, visible_rows)
            };
            if let Some(scroll_line) = scroll_line {
                let old_scroll_top = self.scroll_top;
                self.scroll_top = scroll_line;
                self.clamp_scroll();
                if self.scroll_top != old_scroll_top {
                    self.schedule_inlay_hints(self.prefs.inlay_scroll_debounce_ms(), cx);
                }
            } else {
                self.ensure_cursor_visible(cx);
            }
        } else {
            self.ensure_cursor_visible(cx);
        }
    }

    // ── Key handling ────────────────────────────────────────────────────────

    fn insert_printable(&mut self, text: &str, cx: &mut Context<Self>) {
        let mut characters = text.chars();
        let Some(character) = characters.next() else {
            return;
        };
        let single_character = characters.next().is_none();
        if single_character
            && character == '>'
            && matches!(self.doc.language, Language::Html | Language::Xml)
        {
            self.execute_command(EditorCommand::Edit(EditIntent::TypeTagClose), cx);
        } else if single_character
            && matches!(character, '(' | '[' | '{' | ')' | ']' | '}' | '\'' | '"')
        {
            self.execute_command(EditorCommand::Edit(EditIntent::TypeBracket(character)), cx);
        } else {
            self.execute_command(EditorCommand::Edit(EditIntent::Insert(text.to_owned())), cx);
        }
    }

    fn insert_newline(&mut self, cx: &mut Context<Self>) {
        let indent = self.indent_unit();
        self.execute_command(
            EditorCommand::Edit(EditIntent::AutoIndent { unit: indent }),
            cx,
        );
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;

        if self.go_to_line_requested
            && ks.key.as_str() == "escape"
            && !m.platform
            && !m.control
            && !m.alt
        {
            self.cancel_go_to_line_column(cx);
            cx.stop_propagation();
            return;
        }

        if self.call_hierarchy_open
            && ks.key.as_str() == "escape"
            && !m.platform
            && !m.control
            && !m.alt
        {
            self.call_hierarchy_open = false;
            cx.notify();
            cx.stop_propagation();
            return;
        }

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

        if self.helix.is_some() {
            if let Some(key) = self.helix_key(ks) {
                if self.handle_helix(key, cx) {
                    cx.stop_propagation();
                    return;
                }
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
                if self.doc.selection_set().len() > 1 || !self.wrap_vertical(false, m.shift, cx) {
                    self.navigate(Motion::Up, m.shift, cx)
                }
            }
            "down" => {
                if self.doc.selection_set().len() > 1 || !self.wrap_vertical(true, m.shift, cx) {
                    self.navigate(Motion::Down, m.shift, cx)
                }
            }
            "home" => {
                if self.doc.selection_set().len() > 1 || !self.wrap_horizontal(false, m.shift, cx) {
                    self.navigate(Motion::LineStart, m.shift, cx)
                }
            }
            "end" => {
                if self.doc.selection_set().len() > 1 || !self.wrap_horizontal(true, m.shift, cx) {
                    self.navigate(Motion::LineEnd, m.shift, cx)
                }
            }
            "pageup" => self.navigate(Motion::PageUp(rows), m.shift, cx),
            "pagedown" => self.navigate(Motion::PageDown(rows), m.shift, cx),
            "enter" => self.insert_newline(cx),
            "tab" if m.shift => {
                let indent = self.indent_unit();
                self.execute_command(
                    EditorCommand::Edit(EditIntent::Outdent { unit: indent }),
                    cx,
                );
            }
            "tab" => {
                let indent = self.indent_unit();
                self.execute_command(EditorCommand::Edit(EditIntent::Insert(indent)), cx);
            }
            "backspace" => {
                self.execute_command(EditorCommand::Edit(EditIntent::DeleteBackward), cx);
            }
            "delete" => {
                self.execute_command(EditorCommand::Edit(EditIntent::DeleteForward), cx);
            }
            "escape" => {}
            _ => {
                if let Some(text) = printable(ks) {
                    self.insert_printable(&text, cx);
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
            self.ensure_cursor_visible(cx);
            self.refresh_matches();
            cx.emit(EditorEvent::Changed);
            if !dirty_before && self.doc.is_dirty() {
                cx.emit(EditorEvent::Edited);
            }
            if self.editor_revision() != revision_before {
                self.emit_buffer_changed(cx);
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

    fn helix_key(&self, ks: &gpui::Keystroke) -> Option<HelixKey> {
        let helix = self.helix.as_ref()?;
        let modifiers = &ks.modifiers;
        let insert = helix.mode() == HelixMode::Insert;
        if modifiers.control && !modifiers.platform && !modifiers.alt && ks.key == "r" {
            return Some(HelixKey::Redo);
        }
        if modifiers.control || modifiers.platform || modifiers.alt {
            return None;
        }
        match ks.key.as_str() {
            "escape" => Some(HelixKey::Escape),
            "enter" | "return" if !insert => Some(HelixKey::Enter),
            "backspace" if !insert => Some(HelixKey::Backspace),
            "delete" if !insert => Some(HelixKey::Delete),
            "tab" if !insert => Some(HelixKey::Tab),
            "left" if !insert => Some(HelixKey::Char('h')),
            "right" if !insert => Some(HelixKey::Char('l')),
            "up" if !insert => Some(HelixKey::Char('k')),
            "down" if !insert => Some(HelixKey::Char('j')),
            _ if !insert => printable(ks)
                .and_then(|text| text.chars().next())
                .map(HelixKey::Char),
            _ => None,
        }
    }

    fn handle_helix(&mut self, key: HelixKey, cx: &mut Context<Self>) -> bool {
        let revision_before = self.editor_revision();
        let dirty_before = self.doc.is_dirty();
        let handled = self
            .helix
            .as_mut()
            .is_some_and(|helix| helix.on_key(&mut self.doc, key));
        if !handled {
            return false;
        }

        self.ensure_cursor_visible(cx);
        self.refresh_matches();
        cx.emit(EditorEvent::Changed);
        if !dirty_before && self.doc.is_dirty() {
            cx.emit(EditorEvent::Edited);
        }
        if self.editor_revision() != revision_before {
            self.emit_buffer_changed(cx);
            self.sync_language_service_document(cx);
            self.refresh_git(cx);
        }
        self.schedule_autosave(cx);
        cx.notify();
        true
    }

    fn display_coordinates_at(&mut self, p: Point<Pixels>) -> Option<(usize, usize)> {
        let origin = self.bounds.map(|b| b.origin).unwrap_or_default();
        let (cw, lh) = self.metrics;
        if cw <= 0.0 || lh <= 0.0 {
            return None;
        }
        let x = (f32::from(p.x - origin.x) - self.gutter_width).max(0.0);
        let y = f32::from(p.y - origin.y).max(0.0);
        let snapshot = self.doc.snapshot();
        let config = self.display_config();
        let scroll_top = self.scroll_top;
        let scroll_left = self.scroll_left;
        let layout = self.display_layout(&snapshot, config);
        let first_row = layout.logical_line_to_visual_row(scroll_top);
        let visual_row = first_row.saturating_add((y / lh) as usize);
        let display_column = ((x / cw) + 0.5) as usize + scroll_left;
        Some((visual_row, display_column))
    }

    fn position_at(&mut self, p: Point<Pixels>) -> Position {
        let snapshot = self.doc.snapshot();
        let config = self.display_config();
        let Some((visual_row, display_column)) = self.display_coordinates_at(p) else {
            return self.doc.cursor;
        };
        let layout = self.display_layout(&snapshot, config);
        layout
            .display_to_position(visual_row, display_column)
            .unwrap_or_else(|| Position::new(snapshot.line_count().saturating_sub(1), 0))
    }

    fn begin_selection_drag(&mut self, point: Point<Pixels>, shift: bool, alt: bool) {
        let previous = self.doc.selection_set();
        let snapshot = self.doc.snapshot();
        let coordinates = self.display_coordinates_at(point).unwrap_or((0, 0));
        let position = self.position_at(point);
        let head = Anchor::new(snapshot.position_to_byte(position), AnchorAffinity::After);

        let (preserved, anchor) = if shift && !alt {
            let primary_index = previous.primary_index();
            let anchor = previous.primary().anchor;
            let preserved = previous
                .selections()
                .iter()
                .enumerate()
                .filter_map(|(index, selection)| (index != primary_index).then_some(*selection))
                .collect();
            (preserved, anchor)
        } else if alt {
            (
                previous.selections().to_vec(),
                Anchor::new(head.offset(), AnchorAffinity::Before),
            )
        } else {
            (
                Vec::new(),
                Anchor::new(head.offset(), AnchorAffinity::Before),
            )
        };

        let mut selections = preserved.clone();
        selections.push(Selection { anchor, head });
        let selection_set = SelectionSet::from_selections(selections, preserved.len());
        self.doc.set_selection_set_transient(selection_set);
        self.selection_drag = Some(EditorSelectionDrag {
            previous,
            preserved,
            anchor,
            columnar: alt,
            origin_visual_row: coordinates.0,
            origin_display_column: coordinates.1,
        });
    }

    fn update_selection_drag(&mut self, point: Point<Pixels>) {
        let Some(drag) = self.selection_drag.clone() else {
            return;
        };
        let snapshot = self.doc.snapshot();
        if drag.columnar {
            let (current_visual_row, current_display_column) = self
                .display_coordinates_at(point)
                .unwrap_or((drag.origin_visual_row, drag.origin_display_column));
            let config = self.display_config();
            let (columnar_selections, columnar_primary) = {
                let layout = self.display_layout(&snapshot, config);
                let last_row = layout.total_rows.saturating_sub(1);
                let origin_row = drag.origin_visual_row.min(last_row);
                let current_row = current_visual_row.min(last_row);
                let start_row = origin_row.min(current_row);
                let end_row = origin_row.max(current_row);
                let mut selections = Vec::new();
                let mut primary = 0;
                for visual_row in start_row..=end_row {
                    let (Some(anchor_position), Some(head_position)) = (
                        layout.display_to_position(visual_row, drag.origin_display_column),
                        layout.display_to_position(visual_row, current_display_column),
                    ) else {
                        continue;
                    };
                    if visual_row == current_row {
                        primary = selections.len();
                    }
                    selections.push(Selection {
                        anchor: Anchor::new(
                            snapshot.position_to_byte(anchor_position),
                            AnchorAffinity::Before,
                        ),
                        head: Anchor::new(
                            snapshot.position_to_byte(head_position),
                            AnchorAffinity::After,
                        ),
                    });
                }
                (selections, primary)
            };
            let mut selections = drag.preserved;
            let primary = selections.len() + columnar_primary;
            selections.extend(columnar_selections);
            let selection_set = SelectionSet::from_selections(selections, primary);
            self.doc.set_selection_set_transient(selection_set);
            return;
        }
        let position = self.position_at(point);
        let head = Anchor::new(snapshot.position_to_byte(position), AnchorAffinity::After);
        let mut selections = drag.preserved;
        let primary = selections.len();
        selections.push(Selection {
            anchor: drag.anchor,
            head,
        });
        self.doc
            .set_selection_set_transient(SelectionSet::from_selections(selections, primary));
    }

    fn finish_selection_drag(&mut self) {
        if let Some(drag) = self.selection_drag.take() {
            self.doc.commit_selection_change(drag.previous);
        }
    }

    fn scrollbar_occurrence_offsets(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        display: &labonair_editor::DisplaySnapshot,
        track_length: f32,
        selected_text: Option<String>,
        symbol: Option<String>,
    ) -> (Vec<f32>, Vec<f32>) {
        let track_height = track_length.round() as usize;
        let cached = self
            .scrollbar_occurrence_cache
            .as_ref()
            .is_some_and(|cache| {
                cache.revision == snapshot.revision()
                    && cache.total_rows == display.total_rows
                    && cache.track_height == track_height
                    && cache.selected_text == selected_text
                    && cache.symbol == symbol
            });
        if !cached {
            let selected_text_offsets = selected_text.as_deref().map_or_else(Vec::new, |text| {
                find_scrollbar_marker_offsets(snapshot, display, track_length, text, false)
            });
            let symbol_offsets = symbol.as_deref().map_or_else(Vec::new, |text| {
                find_scrollbar_marker_offsets(snapshot, display, track_length, text, true)
            });
            self.scrollbar_occurrence_cache = Some(ScrollbarOccurrenceCache {
                revision: snapshot.revision(),
                total_rows: display.total_rows,
                track_height,
                selected_text,
                symbol,
                selected_text_offsets,
                symbol_offsets,
            });
        }
        self.scrollbar_occurrence_cache
            .as_ref()
            .map(|cache| {
                (
                    cache.selected_text_offsets.clone(),
                    cache.symbol_offsets.clone(),
                )
            })
            .unwrap_or_default()
    }

    fn render_scrollbar_markers(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        display: &labonair_editor::DisplaySnapshot,
        track_length: f32,
        palette: Palette,
    ) -> Vec<AnyElement> {
        let selection_ranges = self.doc.selection_set().ranges(snapshot);
        let selected_text = self
            .prefs
            .scrollbar_selected_text()
            .then(|| {
                selection_ranges
                    .iter()
                    .find(|range| !range.is_empty())
                    .map(|range| snapshot.text_range(*range))
            })
            .flatten()
            .filter(|text| !text.is_empty() && text.len() <= 256);
        let symbol = self
            .prefs
            .scrollbar_selected_symbol()
            .then(|| symbol_at_position(snapshot, self.doc.cursor))
            .flatten();
        let (selected_text_offsets, symbol_offsets) = self.scrollbar_occurrence_offsets(
            snapshot,
            display,
            track_length,
            selected_text,
            symbol,
        );
        let mut elements = Vec::new();
        let mut seen = HashSet::new();
        for (offsets, color, kind) in [
            (&selected_text_offsets, palette.selected_accent, 2_u8),
            (&symbol_offsets, palette.accent, 3_u8),
        ] {
            for offset in offsets {
                if seen.insert((offset.round() as usize, kind)) {
                    elements.push(
                        div()
                            .absolute()
                            .top(px(*offset))
                            .left_0()
                            .right_0()
                            .h(px(2.0_f32.min(track_length)))
                            .bg(color)
                            .into_any_element(),
                    );
                }
            }
        }
        let mut add_line = |line: usize, color, kind: u8| {
            if line >= snapshot.line_count() {
                return;
            }
            let visual_row = display.logical_line_to_visual_row(line);
            let Some(offset) =
                scrollbar_marker_offset(visual_row, display.total_rows, track_length)
            else {
                return;
            };
            if seen.insert((offset.round() as usize, kind)) {
                elements.push(
                    div()
                        .absolute()
                        .top(px(offset))
                        .left_0()
                        .right_0()
                        .h(px(2.0_f32.min(track_length)))
                        .bg(color)
                        .into_any_element(),
                );
            }
        };

        if self.prefs.scrollbar_cursors() {
            for selection in self.doc.selection_set().selections() {
                add_line(selection.head.resolve(snapshot).line, palette.primary, 0);
            }
        }
        if self.prefs.scrollbar_selected_text() {
            for range in &selection_ranges {
                if range.is_empty() {
                    continue;
                }
                let start = snapshot.byte_to_position(range.start).line;
                let end = snapshot.byte_to_position(range.end).line;
                add_line(start, palette.selected_accent, 1);
                if end != start {
                    add_line(end, palette.selected_accent, 1);
                }
            }
        }
        if self.prefs.scrollbar_search_results() {
            if let Some(search) = &self.search {
                for matched in &search.matches {
                    add_line(matched.start.line, palette.info, 4);
                }
            }
        }

        let language_state = self.language_service_state.snapshot();
        if self.diagnostics_enabled()
            && language_state.version == DocumentVersion::from_revision(self.editor_revision())
        {
            for diagnostic in language_state.diagnostics {
                if !scrollbar_shows_diagnostic(
                    diagnostic.severity,
                    self.prefs.scrollbar_diagnostics(),
                ) {
                    continue;
                }
                let color = match diagnostic.severity {
                    labonair_editor::language_services::DiagnosticSeverity::Error => palette.error,
                    labonair_editor::language_services::DiagnosticSeverity::Warning => {
                        palette.warning
                    }
                    labonair_editor::language_services::DiagnosticSeverity::Information
                    | labonair_editor::language_services::DiagnosticSeverity::Hint => palette.info,
                };
                add_line(diagnostic.range.start.line, color, 5);
            }
        }

        if self.prefs.scrollbar_git_diff()
            && self.prefs.git_gutter()
            && self.git_snapshot_is_current(&self.git_gutter)
        {
            for hunk in &self.git_gutter.hunks {
                let color = match hunk.status {
                    GitHunkStatus::Added | GitHunkStatus::Untracked => palette.success,
                    GitHunkStatus::Modified => palette.warning,
                    GitHunkStatus::Deleted => palette.error,
                };
                if hunk.changed_lines.is_empty() {
                    add_line(hunk.new_start.saturating_sub(1) as usize, color, 6);
                } else {
                    for line in &hunk.changed_lines {
                        add_line(*line, color, 6);
                    }
                }
            }
        }
        elements
    }

    fn render_scrollbars(
        &mut self,
        display: &labonair_editor::DisplaySnapshot,
        palette: Palette,
        sticky_height: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (width, height) = self
            .bounds
            .map(|bounds| (f32::from(bounds.size.width), f32::from(bounds.size.height)))
            .unwrap_or((640.0, 480.0));
        let vertical_viewport = if self.prefs.scroll_beyond_last_line() {
            1
        } else {
            display.viewport.height_rows.max(1)
        };
        let visibility = self.prefs.scrollbar_visibility();
        let wrap_enabled = self.wrap_cols() != 0;
        let vertical_overflow = display.total_rows > vertical_viewport;
        let horizontal_overflow =
            !wrap_enabled && display.max_line_width > display.viewport.width_columns.max(1);
        let show_vertical = self.prefs.vertical_scrollbar()
            && scrollbar_policy_visible(visibility, vertical_overflow);
        let show_horizontal = self.prefs.horizontal_scrollbar()
            && !wrap_enabled
            && scrollbar_policy_visible(visibility, horizontal_overflow);
        let vertical_track_length = (height
            - sticky_height
            - if show_horizontal {
                EDITOR_SCROLLBAR_SIZE
            } else {
                0.0
            })
        .max(1.0);
        let horizontal_track_length = (width
            - self.gutter_width
            - if show_vertical {
                EDITOR_SCROLLBAR_SIZE
            } else {
                0.0
            })
        .max(1.0);
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
        let horizontal = if !wrap_enabled {
            scrollbar_geometry(
                display.max_line_width,
                display.viewport.width_columns.max(1),
                self.scroll_left,
                horizontal_track_length,
            )
        } else {
            None
        };
        let scrollbar_markers = if show_vertical {
            let marker_snapshot = self.doc.snapshot();
            self.render_scrollbar_markers(&marker_snapshot, display, vertical_track_length, palette)
        } else {
            Vec::new()
        };
        let mut vertical_track = div()
            .id("editor-scrollbar-vertical")
            .absolute()
            .top(px(sticky_height))
            .right_0()
            .bottom(px(if show_horizontal {
                EDITOR_SCROLLBAR_SIZE
            } else {
                0.0
            }))
            .w(px(EDITOR_SCROLLBAR_SIZE))
            .bg(palette.muted_bg.opacity(0.25))
            .children(scrollbar_markers);
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
                .on_drag(drag, |_, _, _, cx| cx.new(|_| EditorDragGhost));
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
                    }
                    cx.stop_propagation();
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
            .right(px(if show_vertical {
                EDITOR_SCROLLBAR_SIZE
            } else {
                0.0
            }))
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
                .on_drag(drag, |_, _, _, cx| cx.new(|_| EditorDragGhost));
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
                    }
                    cx.stop_propagation();
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
        let mut overlay = div().absolute().inset_0();
        if show_vertical {
            overlay = overlay.child(vertical_track);
        }
        if show_horizontal {
            overlay = overlay.child(horizontal_track);
        }
        overlay.into_any_element()
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

    fn insert_character_from_palette(&mut self, character: &'static str, cx: &mut Context<Self>) {
        self.character_palette_open = false;
        self.character_palette_group_open = None;
        if self.doc.can_edit() {
            let _ = self.execute_command(
                EditorCommand::Edit(EditIntent::Insert(character.to_owned())),
                cx,
            );
        }
        cx.notify();
    }

    fn render_character_palette(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.character_palette_open {
            return None;
        }

        let view = cx.entity();
        let can_edit = self.doc.can_edit();
        let open_group = self.character_palette_group_open;
        let mut items = vec![MenuItem::label("Character Palette")];
        for (group_id, label, entries) in CHARACTER_PALETTE_GROUPS.iter().copied() {
            let choices = entries
                .iter()
                .enumerate()
                .map(|(index, (character, description))| {
                    let character = *character;
                    let view = view.clone();
                    MenuItem::new(format!("editor-character-{group_id}-{index}"), *description)
                        .detail(character)
                        .disabled(!can_edit)
                        .on_click(move |_, window, cx| {
                            view.update(cx, |this, cx| {
                                this.insert_character_from_palette(character, cx);
                                this.focus(window);
                            });
                        })
                })
                .collect();
            let group_view = view.clone();
            let on_hover = move |source: SubmenuHoverSource,
                                 hovered: bool,
                                 _window: &mut Window,
                                 cx: &mut App| {
                group_view.update(cx, |this, cx| {
                    let changed = if hovered {
                        let was = this.character_palette_group_open;
                        this.character_palette_group_open = Some(group_id);
                        was != Some(group_id)
                    } else if source == SubmenuHoverSource::Flyout
                        && this.character_palette_group_open == Some(group_id)
                    {
                        this.character_palette_group_open = None;
                        true
                    } else {
                        false
                    };
                    if changed {
                        cx.notify();
                    }
                });
            };
            items.push(
                MenuItem::submenu(format!("editor-character-group-{group_id}"), label, choices)
                    .submenu_control(open_group == Some(group_id), on_hover),
            );
        }

        let dismiss_view = view;
        let dismiss = move |_window: &mut Window, cx: &mut App| {
            dismiss_view.update(cx, |this, cx| {
                this.character_palette_open = false;
                this.character_palette_group_open = None;
                cx.notify();
            });
        };
        let viewport = window.viewport_size();
        let anchor = gpui::point(
            px(((f32::from(viewport.width) - 220.0) / 2.0).max(8.0)),
            px(((f32::from(viewport.height) - 150.0) / 2.0).max(8.0)),
        );
        Some(context_menu(
            anchor,
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
                .bg(modal_scrim())
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
                                        ButtonVariant::Subtle,
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
                                        ButtonVariant::Subtle,
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
        let banner = banner(Severity::Warning, c)
            .child("This file changed on disk since you started editing.");
        let banner = if self.format_on_save_in_flight {
            banner.child(
                button_disabled(
                    "conflict-overwrite",
                    c,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child("Overwrite"),
            )
        } else {
            banner.child(
                button(
                    "conflict-overwrite",
                    c,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child("Overwrite")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.overwrite_external(cx))),
            )
        };
        banner
            .child(
                button(
                    "conflict-reload",
                    c,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child("Reload (discard my changes)")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.reload_from_disk(cx))),
            )
            .child(
                button("conflict-keep", c, ButtonVariant::Outlined, ButtonSize::Xs)
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

    fn render_language_service_menu(
        &self,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let anchor = self.language_service_menu_anchor?;
        let status = self.language_service_snapshot.clone();
        if !language_service_controls_available(&status) {
            return None;
        }
        let editor = cx.entity();
        let mut items = vec![MenuItem::label(format!(
            "{} · {}",
            status.server.as_str(),
            language_service_status_label(status.status)
        ))];
        if let Some(detail) = status.detail {
            items.push(MenuItem::label(detail));
        }
        if matches!(
            status.status,
            LanguageServiceRuntimeStatus::Starting | LanguageServiceRuntimeStatus::Running
        ) {
            let stop_editor = editor.clone();
            items.push(
                MenuItem::new("editor-language-service-stop", "Stop Language Server").on_click(
                    move |_, _window, app| {
                        stop_editor.update(app, |this, cx| {
                            this.language_service_menu_anchor = None;
                            let _ = this.dispatch_registered_command(
                                CommandId::EditorStopLanguageServer,
                                cx,
                            );
                        });
                    },
                ),
            );
        }
        let restart_label = if status.status == LanguageServiceRuntimeStatus::Fallback {
            "Start Language Server"
        } else {
            "Restart Language Server"
        };
        let restart_editor = editor.clone();
        items.push(
            MenuItem::new("editor-language-service-restart", restart_label).on_click(
                move |_, _window, app| {
                    restart_editor.update(app, |this, cx| {
                        this.language_service_menu_anchor = None;
                        let _ = this.dispatch_registered_command(
                            CommandId::EditorRestartLanguageServer,
                            cx,
                        );
                    });
                },
            ),
        );
        let dismiss_editor = editor;
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.language_service_menu_anchor = None;
                cx.notify();
            });
        };
        Some(popover_menu(anchor, palette, dismiss, items))
    }

    fn render_diagnostics_menu(
        &self,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let anchor = self.diagnostics_menu_anchor?;
        if !self.diagnostics_enabled() {
            return None;
        }
        let snapshot = self.doc.snapshot();
        if self.diagnostics_revision != Some(snapshot.revision()) {
            return None;
        }

        let mut diagnostics = self.language_service_state.snapshot().diagnostics;
        diagnostics.sort_by_key(|diagnostic| {
            let position = diagnostic.range.start.to_position(&snapshot);
            (position.line, position.column, diagnostic.severity)
        });
        let total = diagnostics.len();
        let editor = cx.entity();
        let mut items = vec![MenuItem::label(format!("{total} document diagnostics"))];
        if total == 0 {
            items.push(MenuItem::label("No diagnostics for the current document."));
        } else {
            for (index, diagnostic) in diagnostics.into_iter().take(100).enumerate() {
                let (label, detail) = diagnostic_menu_labels(&diagnostic, &snapshot);
                let item_editor = editor.clone();
                items.push(
                    MenuItem::new(format!("editor-diagnostic-{index}"), label)
                        .detail(detail)
                        .on_click(move |_, _window, app| {
                            item_editor.update(app, |this, cx| {
                                this.diagnostics_menu_anchor = None;
                                let snapshot = this.doc.snapshot();
                                if this.diagnostics_revision != Some(snapshot.revision()) {
                                    return;
                                }
                                let position = diagnostic.range.start.to_position(&snapshot);
                                this.doc.set_caret(position, false);
                                this.ensure_cursor_visible(cx);
                                cx.emit(EditorEvent::Changed);
                                cx.notify();
                            });
                        }),
                );
            }
            if total > 100 {
                items.push(MenuItem::label(format!("Showing 100 of {total}")));
            }
        }

        let dismiss_editor = editor;
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.diagnostics_menu_anchor = None;
                cx.notify();
            });
        };
        Some(popover_menu(anchor, palette, dismiss, items))
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
                ButtonVariant::Outlined,
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
                        ButtonVariant::Filled,
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
                        ButtonVariant::Subtle,
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

    fn render_helix_status(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let helix_status = self.helix.as_ref().map(Helix::status).unwrap_or("HELIX");
        let cursor = self.doc.cursor;
        div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .flex_shrink_0()
            .px_3()
            .py_0p5()
            .bg(theme.card())
            .border_t_1()
            .border_color(theme.border())
            .text_xs()
            .font(theme.buffer_font())
            .text_color(theme.accent())
            .child(SharedString::from(helix_status))
            .child(
                div()
                    .text_color(theme.muted_foreground())
                    .child(SharedString::from(format!(
                        "{}:{}",
                        cursor.line + 1,
                        cursor.column + 1
                    ))),
            )
    }

    /// The schema or diagnostic hover tooltip — a small floating card
    /// glued to the last mouse position, shown only while
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
        let first_line = self
            .editor_group_views
            .get(&group)
            .map_or(0, |view| view.scroll_top);
        let rows = display
            .rows
            .iter()
            .filter(|row| row.logical_line() >= first_line)
            .take(200)
            .map(|row| {
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
            ButtonVariant::Subtle,
            ButtonSize::IconXs,
        )
        .child("×")
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
            if this.close_editor_group(group) {
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
                    if this.focus_editor_group(group) {
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
                            ButtonVariant::Subtle,
                            ButtonSize::IconXs,
                        )
                        .child("×")
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                if this.close_editor_group(group) {
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
                    cx.new(|_| EditorDragGhost)
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
        let mut left = div().flex().items_center().gap_2();
        if self.prefs.show_cursor_position() {
            left = left.child(
                button(
                    "editor-cursor-position",
                    palette,
                    ButtonVariant::Transparent,
                    ButtonSize::Xs,
                )
                .child(SharedString::from(format!(
                    "Ln {}, Col {}",
                    self.doc.cursor.line + 1,
                    self.doc.cursor.column + 1
                )))
                .tooltip(|window, cx| Tooltip::new("Go to Line/Column").build(window, cx))
                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                    let _ = this.dispatch_registered_command(CommandId::EditorGoToLineColumn, cx);
                })),
            );
        }
        if self.prefs.show_selection_stats() {
            if self.prefs.show_cursor_position() {
                left = left.child(SharedString::from("·"));
            }
            left = left.child(SharedString::from(format!(
                "{} selection{} · {} chars",
                selection_set.len(),
                if selection_set.len() == 1 { "" } else { "s" },
                selected_chars
            )));
        }
        let language = self.doc.language;
        let language_trigger = select_trigger(
            "editor-language-menu",
            palette,
            language.label(),
            self.language_menu_anchor.is_some(),
        )
        .tooltip(|window, cx| Tooltip::new("Select language mode").build(window, cx))
        .on_click(cx.listener(|this, event: &ClickEvent, _window, cx| {
            this.language_menu_anchor = Some(Point {
                x: event.position().x,
                y: event.position().y + px(4.0),
            });
            cx.stop_propagation();
            cx.notify();
        }));
        let mut right = div()
            .flex()
            .items_center()
            .gap_2()
            .child(language_trigger)
            .child(SharedString::from(format!(
                "{} lines",
                snapshot.line_count()
            )));
        if language_service_controls_available(&self.language_service_snapshot) {
            let service_label =
                language_service_status_label(self.language_service_snapshot.status);
            let trigger = button(
                "editor-language-service-status",
                palette,
                ButtonVariant::Transparent,
                ButtonSize::Xs,
            )
            .child(SharedString::from(format!("LSP · {service_label}")))
            .tooltip(|window, cx| {
                Tooltip::new("Language service status and controls").build(window, cx)
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                    this.language_service_menu_anchor = Some(Point {
                        x: event.position.x,
                        y: event.position.y + px(4.0),
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            );
            right = right.child(trigger);
        }
        if self.diagnostics_enabled() && self.diagnostics_revision == Some(snapshot.revision()) {
            let diagnostics = self.language_service_state.snapshot().diagnostics;
            let (errors, warnings) = diagnostic_error_warning_counts(&diagnostics);
            let label = if errors == 0 && warnings == 0 {
                "✓".to_string()
            } else {
                format!("{errors} errors · {warnings} warnings")
            };
            let tooltip = if errors == 0 && warnings == 0 {
                "No errors or warnings · Open diagnostics"
            } else {
                "Open diagnostics"
            };
            right = right.child(
                button(
                    "editor-diagnostics-status",
                    palette,
                    ButtonVariant::Transparent,
                    ButtonSize::Xs,
                )
                .child(SharedString::from(label))
                .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                        this.diagnostics_menu_anchor = Some(Point {
                            x: event.position.x,
                            y: event.position.y + px(4.0),
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ),
            );
        }
        if let Some(blame) = self.git_blame.as_ref().filter(|blame| {
            self.git_blame_enabled
                && blame.revision == snapshot.revision()
                && blame.line == self.doc.cursor.line
                && self.current_git_path().as_deref() == Some(blame.path.as_str())
        }) {
            let label = git_blame_status_label(blame);
            let tooltip = format!(
                "{} · {}\nCommit {}\n{}",
                blame.author,
                git_blame_age_label(
                    blame.author_time,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |duration| duration.as_secs() as i64),
                ),
                blame.commit,
                blame.summary
            );
            right = right.child(
                div()
                    .id("editor-git-blame-status")
                    .text_xs()
                    .text_color(theme.muted_foreground())
                    .child(SharedString::from(label))
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx)),
            );
        }
        let file_format = match self.doc.file_state() {
            FileState::Clean
            | FileState::Dirty
            | FileState::Conflict
            | FileState::ReadOnly
            | FileState::Missing => self.doc.file_format(),
            _ => None,
        };
        if let Some(format) = file_format {
            let ending_label = format.line_ending.as_str();
            let can_change_line_ending = self.doc.can_edit()
                && matches!(
                    self.doc.file_state(),
                    FileState::Clean | FileState::Dirty | FileState::Conflict
                );
            if can_change_line_ending {
                let trigger = button(
                    "editor-line-ending-menu",
                    palette,
                    ButtonVariant::Transparent,
                    ButtonSize::Xs,
                )
                .child(SharedString::from(ending_label))
                .tooltip(|window, cx| Tooltip::new("Change line ending").build(window, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                        this.line_ending_menu_anchor = Some(Point {
                            x: event.position.x,
                            y: event.position.y + px(4.0),
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
                right = right.child(trigger);
            } else {
                right = right.child(SharedString::from(ending_label));
            }
            let can_reopen_encoding = self.doc.path.is_some()
                && matches!(
                    self.doc.file_state(),
                    FileState::Clean | FileState::ReadOnly
                );
            if can_reopen_encoding {
                let trigger = button(
                    "editor-encoding-menu",
                    palette,
                    ButtonVariant::Transparent,
                    ButtonSize::Xs,
                )
                .child(SharedString::from(format.encoding.as_str()))
                .tooltip(|window, cx| Tooltip::new("Reopen with encoding").build(window, cx))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                        this.encoding_menu_anchor = Some(Point {
                            x: event.position.x,
                            y: event.position.y + px(4.0),
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
                right = right.child(trigger);
            } else {
                right = right.child(SharedString::from(format.encoding.as_str()));
            }
            if format.bom != labonair_editor::lifecycle::Bom::None {
                right = right.child("BOM");
            }
        }
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
            .child(left)
            .child(right)
            .into_any_element()
    }

    fn render_language_menu(&self, palette: Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let anchor = self.language_menu_anchor?;
        let options: Vec<SelectOption> = Language::ALL
            .iter()
            .map(|language| (language.id().into(), language.label().into()))
            .collect();
        let editor = cx.entity();
        let dismiss_editor = editor.clone();
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.language_menu_anchor = None;
                cx.notify();
            });
        };
        let on_select = move |token: &SharedString, _window: &mut Window, app: &mut App| {
            if let Some(language) = Language::from_id(token.as_ref()) {
                editor.update(app, |this, cx| this.set_language_mode(language, cx));
            }
        };
        Some(select_popover(
            SelectPopoverAnchor::new("editor-language", anchor, self.language_list_state.clone()),
            palette,
            &options,
            self.doc.language.id(),
            dismiss,
            on_select,
        ))
    }

    fn set_language_mode(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language_menu_anchor = None;
        if !self.doc.set_language(language) {
            cx.notify();
            return;
        }
        self.close_language_service_document();
        self.resync_syntax();
        self.symbol_cache = None;
        self.display_layout_cache = None;
        self.folds.clear();
        self.server_fold_ranges.clear();
        self.sync_language_service_document(cx);
        cx.emit(EditorEvent::Changed);
        cx.notify();
    }

    fn render_line_ending_menu(
        &self,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let anchor = self.line_ending_menu_anchor?;
        if !matches!(
            self.doc.file_state(),
            FileState::Clean | FileState::Dirty | FileState::Conflict
        ) {
            return None;
        }
        let current = self.doc.file_format()?.line_ending;
        let can_edit = self.doc.can_edit()
            && matches!(
                self.doc.file_state(),
                FileState::Clean | FileState::Dirty | FileState::Conflict
            );
        let editor = cx.entity();
        let item_editor = editor.clone();
        let item = move |line_ending: LineEnding, id: &'static str| {
            let editor = item_editor.clone();
            MenuItem::new(id, line_ending.as_str())
                .checked(current == line_ending)
                .disabled(!can_edit)
                .on_click(move |_, _window, app| {
                    editor.update(app, |this, cx| {
                        this.line_ending_menu_anchor = None;
                        match this.doc.set_line_ending(line_ending) {
                            Ok(Some(_)) => cx.emit(EditorEvent::Changed),
                            Ok(None) => {}
                            Err(error) => {
                                notify(cx, "Line ending change failed", &error.to_string())
                            }
                        }
                        cx.notify();
                    });
                })
        };
        let dismiss_editor = editor;
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.line_ending_menu_anchor = None;
                cx.notify();
            });
        };
        Some(popover_menu(
            anchor,
            palette,
            dismiss,
            vec![
                MenuItem::label("Line endings"),
                item(LineEnding::Lf, "editor-line-ending-lf"),
                item(LineEnding::CrLf, "editor-line-ending-crlf"),
                item(LineEnding::Cr, "editor-line-ending-cr"),
            ],
        ))
    }

    fn render_encoding_menu(&self, palette: Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let anchor = self.encoding_menu_anchor?;
        let current = self.doc.file_format()?.encoding;
        let path = self.doc.path.clone()?;
        let can_reopen = matches!(
            self.doc.file_state(),
            FileState::Clean | FileState::ReadOnly
        );
        let editor = cx.entity();
        let item_editor = editor.clone();
        let item = move |encoding: StorageEncoding, id: &'static str| {
            let editor = item_editor.clone();
            let path = path.clone();
            MenuItem::new(id, encoding.as_str())
                .checked(current == EditorView::editor_encoding(encoding))
                .disabled(!can_reopen)
                .on_click(move |_, _window, app| {
                    let selected_path = path.clone();
                    editor.update(app, move |this, cx| {
                        this.encoding_menu_anchor = None;
                        if !matches!(
                            this.doc.file_state(),
                            FileState::Clean | FileState::ReadOnly
                        ) {
                            cx.notify();
                            return;
                        }
                        if this.doc.file_format().is_some_and(|format| {
                            format.encoding == Self::editor_encoding(encoding)
                        }) {
                            cx.notify();
                            return;
                        }
                        this.open_path_with_encoding(selected_path, Some(encoding), cx);
                    });
                })
        };
        let dismiss_editor = editor;
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.encoding_menu_anchor = None;
                cx.notify();
            });
        };
        Some(popover_menu(
            anchor,
            palette,
            dismiss,
            vec![
                MenuItem::label("Reopen with encoding"),
                item(StorageEncoding::Utf8, "editor-encoding-utf8"),
                item(StorageEncoding::Utf16Le, "editor-encoding-utf16le"),
                item(StorageEncoding::Utf16Be, "editor-encoding-utf16be"),
                item(StorageEncoding::Windows1252, "editor-encoding-windows1252"),
                item(StorageEncoding::ShiftJis, "editor-encoding-shiftjis"),
            ],
        ))
    }

    fn render_editor_toolbar(
        &mut self,
        breadcrumb_state: BreadcrumbState,
        palette: Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let breadcrumbs = self.breadcrumbs_visible();
        let quick_actions = self.prefs.toolbar_quick_actions();
        let selections_menu = self.selections_menu_enabled();
        let code_actions = self.prefs.toolbar_code_actions()
            && self.language_service_snapshot.capabilities.code_actions;
        let markdown_preview = self.doc.language == Language::Markdown;
        let svg_preview = self.doc.path.as_ref().is_some_and(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
        });
        let temporary_read_only = self.doc.temporary_read_only();
        if !editor_toolbar_has_contribution(
            breadcrumbs,
            quick_actions,
            selections_menu,
            code_actions,
            markdown_preview || svg_preview,
        ) && !temporary_read_only
        {
            return None;
        }
        let mut row = div()
            .id("editor-toolbar")
            .flex()
            .items_center()
            .gap_1()
            .w_full()
            .h(px(30.0))
            .px_2()
            .flex_shrink_0()
            .bg(palette.bg)
            .border_b_1()
            .border_color(palette.border);
        if breadcrumbs {
            row = row.child(self.render_breadcrumb(breadcrumb_state, palette, cx));
        } else {
            row = row.child(div().flex_1().min_w_0());
        }

        let mut actions = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .flex_shrink_0();
        if markdown_preview {
            let preview =
                icon_button_builder("editor-markdown-preview-toggle", palette, IconName::Eye)
                    .size(ButtonSize::IconSm)
                    .shape(IconButtonShape::Square)
                    .tooltip("Open Markdown Preview")
                    .render()
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        let _ = this
                            .dispatch_registered_command(CommandId::EditorOpenMarkdownPreview, cx);
                    }));
            actions = actions.child(preview);
        }
        if svg_preview {
            let preview = icon_button_builder("editor-svg-preview-toggle", palette, IconName::Eye)
                .size(ButtonSize::IconSm)
                .shape(IconButtonShape::Square)
                .tooltip("Open SVG Preview")
                .render()
                .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                    let _ = this.dispatch_registered_command(CommandId::EditorOpenSvgPreview, cx);
                }));
            actions = actions.child(preview);
        }
        if temporary_read_only {
            let toggle = button(
                "editor-read-only-toggle",
                palette,
                ButtonVariant::Outlined,
                ButtonSize::Xs,
            )
            .child("Read only")
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                let _ = this.dispatch_registered_command(CommandId::EditorToggleReadOnly, cx);
            }));
            actions = actions.child(toggle);
        }
        if quick_actions {
            let search =
                icon_button_builder("editor-buffer-search-toggle", palette, IconName::Search)
                    .size(ButtonSize::IconSm)
                    .shape(IconButtonShape::Square)
                    .selected(self.buffer_search_open)
                    .tooltip("Find in Buffer")
                    .render()
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.toggle_buffer_search(window, cx);
                    }));
            actions = actions.child(search);
            let controls = icon_button_builder("editor-controls-toggle", palette, IconName::Filter)
                .size(ButtonSize::IconSm)
                .shape(IconButtonShape::Square)
                .selected(self.editor_controls_anchor.is_some())
                .tooltip("Editor Controls")
                .render()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                        this.editor_controls_anchor = if this.editor_controls_anchor.is_some() {
                            None
                        } else {
                            Some(Point {
                                x: event.position.x,
                                y: event.position.y + px(4.0),
                            })
                        };
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
            actions = actions.child(controls);
        }
        if selections_menu {
            let trigger =
                icon_button_builder("editor-selections-menu", palette, IconName::ListTree)
                    .size(ButtonSize::IconSm)
                    .shape(IconButtonShape::Square)
                    .selected(self.selection_menu_anchor.is_some())
                    .tooltip("Selections")
                    .render()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                            this.selection_menu_anchor = if this.selection_menu_anchor.is_some() {
                                None
                            } else {
                                Some(Point {
                                    x: event.position.x,
                                    y: event.position.y + px(4.0),
                                })
                            };
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    );
            actions = actions.child(trigger);
        }
        if code_actions {
            let trigger =
                icon_button_builder("editor-code-actions-toggle", palette, IconName::Code)
                    .size(ButtonSize::IconSm)
                    .shape(IconButtonShape::Square)
                    .selected(self.code_actions.is_some())
                    .tooltip("Code Actions")
                    .render()
                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                        let _ = this.dispatch_registered_command(CommandId::EditorCodeAction, cx);
                    }));
            actions = actions.child(trigger);
        }

        row = row.child(actions);
        let mut toolbar = div().flex().flex_col().w_full().flex_shrink_0().child(row);
        if quick_actions && self.buffer_search_open {
            toolbar = toolbar.children(self.render_buffer_search_bar(palette, window, cx));
        }
        Some(toolbar.into_any_element())
    }

    fn render_editor_controls_menu(
        &self,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let anchor = self.editor_controls_anchor?;
        let editor = cx.entity();
        let mut items = vec![MenuItem::label("Editor Controls")];
        for control in available_editor_controls(self.language_service_snapshot.capabilities) {
            let editor = editor.clone();
            items.push(
                MenuItem::new(control.id(), control.label())
                    .checked(self.editor_control_enabled(control))
                    .on_click(move |_, _window, app| {
                        editor.update(app, |this, cx| {
                            this.editor_controls_anchor = None;
                            this.toggle_editor_control(control, cx);
                        });
                    }),
            );
        }
        let dismiss_editor = editor;
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.editor_controls_anchor = None;
                cx.notify();
            });
        };
        Some(popover_menu(anchor, palette, dismiss, items))
    }

    fn render_buffer_search_bar(
        &self,
        palette: Palette,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let input = self.buffer_search_input.as_ref()?;
        let field_state = if input.read(cx).focus_handle(cx).is_focused(window) {
            TextFieldState::Focused
        } else {
            TextFieldState::Normal
        };
        let (current, total) = self.search_count();
        let count_text = if let Some(error) = self.search_error() {
            SharedString::from(error.to_string())
        } else {
            SharedString::from(format!("{current}/{total}"))
        };
        let count_color = if self.search_error().is_some() {
            palette.error
        } else {
            palette.muted
        };
        let previous = icon_button_builder(
            "editor-buffer-search-previous",
            palette,
            IconName::ChevronUp,
        )
        .size(ButtonSize::IconSm)
        .shape(IconButtonShape::Square)
        .tooltip("Previous Match")
        .render()
        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            let _ = this.search_step(false, cx);
        }));
        let next = icon_button_builder("editor-buffer-search-next", palette, IconName::ChevronDown)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip("Next Match")
            .render()
            .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                let _ = this.search_step(true, cx);
            }));
        let close = icon_button_builder("editor-buffer-search-close", palette, IconName::Close)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip("Close Search")
            .render()
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_buffer_search(window, cx);
            }));
        Some(
            div()
                .id("editor-buffer-search")
                .flex()
                .items_center()
                .gap_1()
                .w_full()
                .h(px(36.0))
                .px_2()
                .flex_shrink_0()
                .bg(palette.bg)
                .border_b_1()
                .border_color(palette.border)
                .child(div().flex_1().min_w_0().child(text_field_surface(
                    "editor-buffer-search-input",
                    palette,
                    field_state,
                    text_input(input, palette),
                )))
                .child(
                    div()
                        .min_w(px(64.0))
                        .text_xs()
                        .text_color(count_color)
                        .child(count_text),
                )
                .child(previous)
                .child(next)
                .child(close)
                .into_any_element(),
        )
    }

    fn render_selection_menu(
        &self,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.selections_menu_enabled() {
            return None;
        }
        let anchor = self.selection_menu_anchor?;
        let editor = cx.entity();
        let command_item = |id: CommandId, label: &'static str, disabled: bool| {
            let editor = editor.clone();
            MenuItem::new(id.action_name(), label)
                .disabled(disabled)
                .on_click(move |_, _window, app| {
                    editor.update(app, |this, cx| {
                        this.selection_menu_anchor = None;
                        let _ = this.dispatch_registered_command(id, cx);
                        cx.notify();
                    });
                })
        };
        let has_git_hunks =
            !self.git_gutter.hunks.is_empty() && self.git_snapshot_is_current(&self.git_gutter);
        let show_outline = editor.clone();
        let outline_item = MenuItem::new("editor-open-symbol-outline", "Go to Symbol").on_click(
            move |_, _window, app| {
                show_outline.update(app, |this, cx| {
                    this.selection_menu_anchor = None;
                    this.outline_open = true;
                    cx.notify();
                });
            },
        );
        let dismiss_editor = editor.clone();
        let dismiss = move |_window: &mut Window, app: &mut App| {
            dismiss_editor.update(app, |this, cx| {
                this.selection_menu_anchor = None;
                cx.notify();
            });
        };
        let items = vec![
            MenuItem::label("Selections"),
            command_item(CommandId::EditorSelectAll, "Select All", false),
            command_item(
                CommandId::EditorSelectNextOccurrence,
                "Select Next Occurrence",
                false,
            ),
            command_item(
                CommandId::EditorSelectPreviousOccurrence,
                "Select Previous Occurrence",
                false,
            ),
            command_item(
                CommandId::EditorSelectAllOccurrences,
                "Select All Occurrences",
                false,
            ),
            command_item(CommandId::EditorExpandSelection, "Expand Selection", false),
            command_item(CommandId::EditorShrinkSelection, "Shrink Selection", false),
            command_item(CommandId::EditorAddCursorAbove, "Add Cursor Above", false),
            command_item(CommandId::EditorAddCursor, "Add Cursor Below", false),
            MenuItem::separator(),
            MenuItem::label("Navigation and Lines"),
            outline_item,
            command_item(CommandId::EditorGoToLineColumn, "Go to Line/Column…", false),
            command_item(CommandId::EditorNextDiagnostic, "Next Problem", false),
            command_item(
                CommandId::EditorPreviousDiagnostic,
                "Previous Problem",
                false,
            ),
            command_item(CommandId::EditorNextGitChange, "Next Hunk", !has_git_hunks),
            command_item(
                CommandId::EditorPreviousGitChange,
                "Previous Hunk",
                !has_git_hunks,
            ),
            command_item(CommandId::EditorMoveLinesUp, "Move Line Up", false),
            command_item(CommandId::EditorMoveLines, "Move Line Down", false),
            command_item(
                CommandId::EditorDuplicateSelection,
                "Duplicate Selection",
                false,
            ),
        ];
        Some(popover_menu(anchor, palette, dismiss, items))
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
            .flex_1()
            .min_w_0()
            .h(px(30.0))
            .overflow_hidden()
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
                    let copy_path = full_path.clone();
                    row = row.child(
                        div()
                            .id("editor-breadcrumb-path")
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(palette.muted)
                            .tooltip(move |window, cx| {
                                Tooltip::new(format!(
                                    "{} · right-click to copy absolute path",
                                    full_path
                                ))
                                .build(window, cx)
                            })
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |_this, _event: &MouseDownEvent, _window, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copy_path.clone(),
                                    ));
                                    cx.stop_propagation();
                                }),
                            )
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

        row.on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            this.outline_open = true;
            cx.notify();
        }))
        .tooltip(|window, cx| Tooltip::new("Open Symbol Outline").build(window, cx))
        .into_any_element()
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
                ButtonVariant::Subtle,
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
                this.outline_open = false;
                this.ensure_cursor_visible(cx);
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

    fn minimap_occurrence_slots(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        line_count: usize,
        slot_count: usize,
        selected_text: Option<String>,
        symbol: Option<String>,
    ) -> (Vec<usize>, Vec<usize>) {
        let cached = self.minimap_occurrence_cache.as_ref().is_some_and(|cache| {
            cache.revision == snapshot.revision()
                && cache.line_count == line_count
                && cache.slot_count == slot_count
                && cache.selected_text == selected_text
                && cache.symbol == symbol
        });
        if !cached {
            let selected_text_slots = selected_text.as_deref().map_or_else(Vec::new, |text| {
                find_minimap_occurrence_slots(snapshot, line_count, slot_count, text, false)
            });
            let symbol_slots = symbol.as_deref().map_or_else(Vec::new, |text| {
                find_minimap_occurrence_slots(snapshot, line_count, slot_count, text, true)
            });
            self.minimap_occurrence_cache = Some(MinimapOccurrenceCache {
                revision: snapshot.revision(),
                line_count,
                slot_count,
                selected_text,
                symbol,
                selected_text_slots,
                symbol_slots,
            });
        }
        self.minimap_occurrence_cache
            .as_ref()
            .map(|cache| {
                (
                    cache.selected_text_slots.clone(),
                    cache.symbol_slots.clone(),
                )
            })
            .unwrap_or_default()
    }

    fn render_minimap(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
        display: &labonair_editor::DisplaySnapshot,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let line_count = snapshot.line_count().max(1);
        let height = self
            .bounds
            .map(|bounds| f32::from(bounds.size.height))
            .unwrap_or(480.0);
        let track_length = (height - 16.0).max(1.0);
        let slot_count = line_count
            .min(500)
            .min(track_length.floor().max(1.0) as usize)
            .max(1);
        let slot_height = track_length / slot_count as f32;
        let selection_ranges = self.doc.selection_set().ranges(snapshot);
        let selected_text = selection_ranges
            .iter()
            .find(|range| !range.is_empty())
            .map(|range| snapshot.text_range(*range))
            .filter(|text| !text.is_empty() && text.len() <= 256);
        let symbol = symbol_at_position(snapshot, self.doc.cursor);
        let (selected_text_slots, symbol_slots) =
            self.minimap_occurrence_slots(snapshot, line_count, slot_count, selected_text, symbol);
        let mut markers = vec![MinimapLineMarkers::default(); slot_count];
        mark_minimap_line_range(
            &mut markers,
            line_count,
            self.doc.cursor.line,
            self.doc.cursor.line,
            |marker| marker.cursor = true,
        );
        for range in selection_ranges {
            if range.is_empty() {
                continue;
            }
            mark_minimap_line_range(
                &mut markers,
                line_count,
                snapshot.byte_to_position(range.start).line,
                snapshot.byte_to_position(range.end).line,
                |marker| marker.selection = true,
            );
        }
        for slot in selected_text_slots {
            markers[slot].selected_text_occurrence = true;
        }
        for slot in symbol_slots {
            markers[slot].selected_symbol_occurrence = true;
        }
        if let Some(search) = &self.search {
            for matched in &search.matches {
                mark_minimap_line_range(
                    &mut markers,
                    line_count,
                    matched.start.line,
                    matched.end.line,
                    |marker| marker.search = true,
                );
            }
        }
        let language_state = self.language_service_state.snapshot();
        if self.diagnostics_enabled()
            && language_state.version == DocumentVersion::from_revision(self.editor_revision())
        {
            for diagnostic in language_state.diagnostics {
                mark_minimap_line_range(
                    &mut markers,
                    line_count,
                    diagnostic.range.start.line,
                    diagnostic.range.end.line,
                    |marker| {
                        marker.diagnostic =
                            Some(marker.diagnostic.map_or(diagnostic.severity, |current| {
                                current.min(diagnostic.severity)
                            }));
                    },
                );
            }
        }
        if self.git_snapshot_is_current(&self.git_gutter) {
            for hunk in &self.git_gutter.hunks {
                if hunk.changed_lines.is_empty() {
                    let line = hunk.new_start.saturating_sub(1) as usize;
                    mark_minimap_line_range(&mut markers, line_count, line, line, |marker| {
                        marker.git = Some(hunk.status)
                    });
                } else {
                    for line in &hunk.changed_lines {
                        mark_minimap_line_range(&mut markers, line_count, *line, *line, |marker| {
                            marker.git = Some(hunk.status)
                        });
                    }
                }
            }
        }
        let show_current_line = match self.prefs.minimap_current_line_highlight() {
            EditorMinimapCurrentLineHighlight::Inherit => self.prefs.highlight_current_line(),
            EditorMinimapCurrentLineHighlight::Always => true,
            EditorMinimapCurrentLineHighlight::Never => false,
        };
        let minimap_width = minimap_width_pixels(self.prefs.minimap_max_width_columns());
        let content_width = (minimap_width - 20.0).max(12.0);
        let rows = (0..slot_count).map(|slot| {
            let line = (slot.saturating_mul(line_count) / slot_count).min(line_count - 1);
            let width = (snapshot.line_len(line).min(60) as f32 / 60.0 * content_width).max(4.0);
            let marker = markers[slot];
            let mut row = div()
                .relative()
                .w_full()
                .h(px(slot_height))
                .when(marker.cursor && show_current_line, |row| {
                    row.bg(palette.accent.opacity(0.12))
                })
                .child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top_0()
                        .h_full()
                        .w(px(width))
                        .bg(palette
                            .muted
                            .opacity(if marker.cursor { 0.9 } else { 0.42 })),
                );
            if marker.selection {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top_0()
                        .h_full()
                        .w(px((width * 0.72).max(3.0)))
                        .bg(palette.selected_accent.opacity(0.85)),
                );
            }
            if marker.search {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top_0()
                        .h_full()
                        .w(px((width * 0.45).max(3.0)))
                        .bg(palette.info.opacity(0.9)),
                );
            }
            if marker.selected_text_occurrence {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(10.0))
                        .top_0()
                        .h_full()
                        .w(px((width * 0.55).max(3.0)))
                        .bg(palette.selected_accent.opacity(0.75)),
                );
            }
            if marker.selected_symbol_occurrence {
                row = row.child(
                    div()
                        .absolute()
                        .left(px(8.0))
                        .top_0()
                        .h_full()
                        .w(px(2.0))
                        .bg(palette.accent),
                );
            }
            if marker.cursor {
                row = row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .w(px(2.0))
                        .bg(palette.primary),
                );
            }
            if let Some(status) = marker.git {
                let color = match status {
                    GitHunkStatus::Added | GitHunkStatus::Untracked => palette.success,
                    GitHunkStatus::Modified => palette.warning,
                    GitHunkStatus::Deleted => palette.error,
                };
                row = row.child(
                    div()
                        .absolute()
                        .left(px(3.0))
                        .top_0()
                        .h_full()
                        .w(px(2.0))
                        .bg(color),
                );
            }
            if let Some(severity) = marker.diagnostic {
                let color = match severity {
                    labonair_editor::language_services::DiagnosticSeverity::Error => palette.error,
                    labonair_editor::language_services::DiagnosticSeverity::Warning => {
                        palette.warning
                    }
                    labonair_editor::language_services::DiagnosticSeverity::Information
                    | labonair_editor::language_services::DiagnosticSeverity::Hint => palette.info,
                };
                row = row.child(
                    div()
                        .absolute()
                        .left(px(6.0))
                        .top_0()
                        .h_full()
                        .w(px(2.0))
                        .bg(color),
                );
            }
            row
        });
        let show_thumb =
            self.prefs.minimap_thumb() == EditorMinimapThumb::Always || self.minimap_hovered;
        let thumb = show_thumb
            .then(|| {
                scrollbar_geometry(
                    display.total_rows,
                    display.viewport.height_rows,
                    display.viewport.top_row,
                    track_length,
                )
            })
            .flatten()
            .map(|geometry| {
                let thumb = div()
                    .absolute()
                    .top(px(8.0 + geometry.thumb_offset))
                    .right_0()
                    .w(px(3.0))
                    .h(px(geometry.thumb_length))
                    .rounded(px(2.0))
                    .bg(palette.muted.opacity(0.78));
                match self.prefs.minimap_thumb_border() {
                    EditorMinimapThumbBorder::All => thumb.border_1().border_color(palette.border),
                    EditorMinimapThumbBorder::LeftOpen => {
                        thumb.border_l_1().border_color(palette.border)
                    }
                    EditorMinimapThumbBorder::None => thumb,
                }
            });
        let drag = EditorMinimapDrag {
            total_rows: display.total_rows,
            viewport_rows: display.viewport.height_rows,
        };
        let minimap_total_rows = drag.total_rows;
        let minimap_viewport_rows = drag.viewport_rows;
        let minimap = div()
            .id("editor-minimap")
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(px(minimap_width))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .overflow_hidden()
            .p_2()
            .bg(palette.sidebar.opacity(0.86))
            .border_l_1()
            .border_color(palette.border)
            .children(rows)
            .children(thumb)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    let (track_start, length) = this
                        .bounds
                        .map(|bounds| {
                            (
                                f32::from(bounds.origin.y) + 8.0,
                                (f32::from(bounds.size.height) - 16.0).max(1.0),
                            )
                        })
                        .unwrap_or((8.0, 1.0));
                    let row = minimap_scroll_from_pointer(
                        f32::from(event.position.y),
                        track_start,
                        length,
                        minimap_total_rows,
                        minimap_viewport_rows,
                    );
                    this.scroll_to_visual_row(row, cx);
                    cx.stop_propagation();
                }),
            )
            .on_drag(drag, |_, _, _, cx| cx.new(|_| EditorDragGhost))
            .on_drag_move(cx.listener(
                move |this, event: &gpui::DragMoveEvent<EditorMinimapDrag>, _window, cx| {
                    let drag = event.drag(cx);
                    let track_start = f32::from(event.bounds.origin.y) + 8.0;
                    let track_length = (f32::from(event.bounds.size.height) - 16.0).max(1.0);
                    let row = minimap_scroll_from_pointer(
                        f32::from(event.event.position.y),
                        track_start,
                        track_length,
                        drag.total_rows,
                        drag.viewport_rows,
                    );
                    this.scroll_to_visual_row(row, cx);
                },
            ))
            .on_hover({
                let view = cx.entity();
                move |hovered, _window, cx| {
                    view.update(cx, |this, cx| {
                        if this.minimap_hovered != *hovered {
                            this.minimap_hovered = *hovered;
                            cx.notify();
                        }
                    });
                }
            });
        minimap.into_any_element()
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
        if self.call_hierarchy_open {
            shown = true;
            let direction_label = match self.call_hierarchy_direction {
                CallHierarchyDirection::Incoming => "Incoming calls",
                CallHierarchyDirection::Outgoing => "Outgoing calls",
            };
            let toggle_label = match self.call_hierarchy_direction {
                CallHierarchyDirection::Incoming => "Show outgoing",
                CallHierarchyDirection::Outgoing => "Show incoming",
            };
            let root_label = self
                .call_hierarchy_root
                .as_ref()
                .map(|item| item.name.as_str())
                .unwrap_or("Call hierarchy");
            let header = div()
                .flex()
                .items_center()
                .gap_1()
                .child(labonair_ui_kit::list_header(
                    format!("{direction_label} · {root_label}"),
                    palette.muted,
                ))
                .child(
                    button(
                        "editor-call-hierarchy-toggle-direction",
                        palette,
                        ButtonVariant::Subtle,
                        ButtonSize::Xs,
                    )
                    .child(toggle_label)
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.toggle_call_hierarchy_direction(cx);
                        },
                    )),
                )
                .child(
                    button(
                        "editor-call-hierarchy-close",
                        palette,
                        ButtonVariant::Subtle,
                        ButtonSize::Xs,
                    )
                    .child("Close")
                    .on_click(cx.listener(
                        |this, _: &ClickEvent, _window, cx| {
                            this.call_hierarchy_open = false;
                            cx.notify();
                        },
                    )),
                );
            overlay = overlay.child(header);
            if self.call_hierarchy_loading {
                overlay = overlay.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("Loading call hierarchy…"),
                );
            } else if self.call_hierarchy_calls.is_empty() {
                overlay = overlay.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(palette.muted)
                        .child(match self.call_hierarchy_direction {
                            CallHierarchyDirection::Incoming => "No incoming calls.",
                            CallHierarchyDirection::Outgoing => "No outgoing calls.",
                        }),
                );
            } else {
                let rows =
                    self.call_hierarchy_calls
                        .iter()
                        .enumerate()
                        .take(100)
                        .map(|(index, call)| {
                            let line = call.item.selection_range.start.line.saturating_add(1);
                            let file = call
                                .item
                                .uri
                                .rsplit('/')
                                .next()
                                .unwrap_or(call.item.uri.as_str());
                            let detail = call.item.detail.as_deref().unwrap_or(file);
                            let label = format!("{} · {detail}:{line}", call.item.name);
                            let uri = call.item.uri.clone();
                            let position = call.item.selection_range.start;
                            ListItem::new(
                                ("editor-call-hierarchy-item", index),
                                palette.fg,
                                palette.muted,
                                palette.selected_fill,
                            )
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.call_hierarchy_open = false;
                                if this.language_service_uri.as_deref() == Some(uri.as_str()) {
                                    this.goto_lsp_position(position, cx);
                                } else if let Some(navigate) = this.call_hierarchy_navigator.clone()
                                {
                                    navigate(uri.clone(), position, window, cx);
                                }
                                cx.notify();
                            }))
                            .child(SharedString::from(label))
                        });
                overlay = overlay.child(
                    div()
                        .id("editor-call-hierarchy-results")
                        .max_h(px(240.0))
                        .overflow_y_scroll()
                        .children(rows),
                );
            }
        }
        if let Some(actions) = self.code_actions.clone() {
            if !actions.is_empty() {
                shown = true;
                let rows = actions.iter().enumerate().take(10).map(|(index, action)| {
                    button(
                        ("editor-code-action-item", index),
                        palette,
                        ButtonVariant::Subtle,
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
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input_for_submit = input.clone();
        let field_state = if input.read(cx).focus_handle(cx).is_focused(window) {
            TextFieldState::Focused
        } else {
            TextFieldState::Normal
        };
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
            .child(text_field_surface(
                "editor-rename-input",
                palette,
                field_state,
                text_input(input, palette),
            ))
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
                            ButtonVariant::Subtle,
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
                            ButtonVariant::Filled,
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

    fn render_go_to_line_dialog(
        &self,
        input: &Entity<InputState>,
        palette: Palette,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let input_for_submit = input.clone();
        let field_state = if input.read(cx).focus_handle(cx).is_focused(window) {
            TextFieldState::Focused
        } else {
            TextFieldState::Normal
        };
        div()
            .absolute()
            .top(px(12.0))
            .right(px(92.0))
            .w(px(320.0))
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
                    .child("Go to line or line:column"),
            )
            .child(text_field_surface(
                "editor-go-to-line-input",
                palette,
                field_state,
                text_input(input, palette),
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_1()
                    .mt_1()
                    .child(
                        button(
                            "editor-go-to-line-cancel",
                            palette,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .child("Cancel")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _window, cx| {
                                this.cancel_go_to_line_column(cx);
                            },
                        )),
                    )
                    .child(
                        button(
                            "editor-go-to-line-submit",
                            palette,
                            ButtonVariant::Filled,
                            ButtonSize::Xs,
                        )
                        .child("Go")
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window, cx| {
                                let value = input_for_submit.read(cx).value().to_string();
                                this.submit_go_to_line_column(&value, cx);
                            },
                        )),
                    ),
            )
            .into_any_element()
    }

    fn symbols_for_snapshot(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
    ) -> Arc<[labonair_editor::DocumentSymbol]> {
        let language = self.doc.language;
        let needs_rebuild = self.symbol_cache.as_ref().is_none_or(|cache| {
            cache.revision != snapshot.revision() || cache.language != language
        });
        if needs_rebuild {
            let syntax = document_syntax(language, snapshot);
            self.symbol_cache = Some(SymbolCache {
                revision: snapshot.revision(),
                language,
                symbols: Arc::from(syntax.symbols),
                folding_ranges: Arc::from(syntax.folding_ranges),
            });
        }
        Arc::clone(
            &self
                .symbol_cache
                .as_ref()
                .expect("symbol cache is populated")
                .symbols,
        )
    }

    fn syntax_folding_ranges_for_snapshot(
        &mut self,
        snapshot: &labonair_editor::BufferSnapshot,
    ) -> Arc<[FoldRange]> {
        let _ = self.symbols_for_snapshot(snapshot);
        Arc::clone(
            &self
                .symbol_cache
                .as_ref()
                .expect("syntax cache is populated")
                .folding_ranges,
        )
    }

    fn sticky_context_label(&self, symbols: &[labonair_editor::DocumentSymbol]) -> Option<String> {
        if !self.prefs.sticky_context() {
            return None;
        }
        symbols
            .iter()
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
        self.ensure_go_to_line_input(window, cx);
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
        let visible_diagnostics = service_state
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                scrollbar_shows_diagnostic(
                    diagnostic.severity,
                    self.prefs.diagnostics_max_severity(),
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        let diagnostic_decorations = if self.diagnostics_enabled() {
            display.diagnostic_decorations(&snapshot, &visible_diagnostics)
        } else {
            Vec::new()
        };
        let semantic_decorations = if self.semantic_tokens_enabled() {
            display.semantic_token_decorations(&snapshot, &service_state.semantic_tokens)
        } else {
            Vec::new()
        };
        let document_color_decorations = if self.prefs.lsp_document_colors()
            != EditorDocumentColorMode::Off
            && service_state.version == DocumentVersion::from_revision(snapshot.revision())
        {
            display.document_color_decorations(&snapshot, &service_state.document_colors)
        } else {
            Vec::new()
        };

        let sel = self.doc.selection();
        let selections = self.doc.selection_set();
        let cursor = self.doc.cursor;
        let gutter_width = self.gutter_width;

        // Syntax highlighting (T06-002): parse once per revision, keep only the
        // spans covering the visible line range, and repaint with the palette
        // resolved from the active app theme.
        let palette = EditorPalette::resolve(theme.editor_theme(), theme);
        let ui_palette = Palette::from_theme(theme);
        let rename_input = self.rename_input.clone();
        let go_to_line_input = self.go_to_line_input.clone();
        let symbols = self.symbols_for_snapshot(&snapshot);
        let syntax_ranges = self.syntax_folding_ranges_for_snapshot(&snapshot);
        let fold_candidates = fold_candidates(
            &self.server_fold_ranges,
            &symbols,
            &syntax_ranges,
            &snapshot,
        );
        let breadcrumb_state = build_breadcrumb_state(
            self.doc.file_state(),
            self.doc.path.as_deref(),
            &symbols,
            cursor.line,
            EDITOR_BREADCRUMB_MAX_PATH_CHARS,
        );
        let sticky_context = self.sticky_context_label(&symbols);
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
            let continuation = display_row.continuation;
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
                            ButtonVariant::Subtle,
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
        let diagnostics_enabled = self.diagnostics_enabled();
        let inline_diagnostics_enabled = self.inline_diagnostics_enabled();
        let code_lens_enabled = self.code_lens_enabled();
        let text_rows = display_rows.iter().map(|display_row| {
            let line = display_row.logical_line();
            let row_top = (display_row.visual_row.saturating_sub(first_visual) as f32) * line_h;
            let projection = display_row.text_projection(&snapshot);
            let source_content = display_row.text(&snapshot);
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
                let first_visible_column = start_column.saturating_add(display.viewport.left_column);
                if self.prefs.inlay_show_background() {
                    for hint in &display_row.inline_hints {
                        let hint_width = hint.label.chars().count()
                            + usize::from(hint.padding_left)
                            + usize::from(hint.padding_right);
                        let hint_column = hint
                            .display_column
                            .saturating_sub(display.viewport.left_column);
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(hint_column as f32 * char_w))
                                .top(px(2.0))
                                .w(px((hint_width as f32 * char_w).max(2.0)))
                                .h(px((line_h - 4.0).max(1.0)))
                                .rounded(px(2.0))
                                .bg(accent.opacity(0.1)),
                        );
                    }
                }
                for column in display_row
                    .indent_guides
                    .iter()
                    .copied()
                    .filter(|column| *column >= first_visible_column)
                {
                    row = row.child(
                        div()
                            .absolute()
                            .left(px(
                                column.saturating_sub(first_visible_column) as f32 * char_w,
                            ))
                            .top_0()
                            .w(px(1.0))
                            .h(px(line_h))
                            .bg(muted.opacity(0.25)),
                    );
                }
                if let Some((s, e)) = sel {
                    let start = if line == s.line { s.column } else { 0 };
                    let end = if line == e.line { e.column } else { snapshot.line_len(line) };
                    let lo = start.max(start_column);
                    let hi = end.min(end_column);
                    if hi > lo {
                        let display_start = display_row
                            .display_column_for_source_position(Position::new(line, lo))
                            .unwrap_or_else(|| lo.saturating_sub(start_column))
                            .saturating_sub(display.viewport.left_column);
                        let display_end = display_row
                            .display_column_for_source_position(Position::new(line, hi))
                            .unwrap_or_else(|| hi.saturating_sub(start_column))
                            .saturating_sub(display.viewport.left_column);
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(display_start as f32 * char_w))
                                .top_0()
                                .w(px(((display_end.saturating_sub(display_start)) as f32 * char_w).max(2.0)))
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
                            let display_start = display_row
                                .display_column_for_source_position(Position::new(line, lo))
                                .unwrap_or_else(|| lo.saturating_sub(start_column))
                                .saturating_sub(display.viewport.left_column);
                            let display_end = display_row
                                .display_column_for_source_position(Position::new(line, hi))
                                .unwrap_or_else(|| hi.saturating_sub(start_column))
                                .saturating_sub(display.viewport.left_column);
                            row = row.child(
                                div()
                                    .absolute()
                                    .left(px(display_start as f32 * char_w))
                                    .top_0()
                                    .w(px(((display_end.saturating_sub(display_start)) as f32 * char_w).max(2.0)))
                                    .h(px(line_h))
                                    .bg(accent.opacity(if search.active_match_is(*matched) { 0.45 } else { 0.18 })),
                            );
                        }
                    }
                }
                for token in display.whitespace_tokens(&snapshot, display_row, &selections) {
                    let marker_column = snapshot.byte_to_position(token.range.start).column;
                    if marker_column < start_column || marker_column >= end_column {
                        continue;
                    }
                    let display_column = display_row
                        .display_column_for_source_position(Position::new(line, marker_column))
                        .unwrap_or_else(|| marker_column.saturating_sub(start_column))
                        .saturating_sub(display.viewport.left_column);
                    row = row.child(
                        div()
                            .absolute()
                            .left(px(display_column as f32 * char_w + 2.0))
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
                    let start_column = source_content[..byte].chars().count();
                    let end_column = source_content[..byte + len].chars().count();
                    let range = projection.project_source_range(start_column, end_column);
                    if let Some(range) = range {
                    highlights.push((
                        range,
                        HighlightStyle {
                            color: Some(palette.color(kind)),
                            ..Default::default()
                        },
                    ));
                    }
                }
                byte += len;
            }
            highlights.extend(projection.inlay_hint_ranges.iter().cloned().map(|range| {
                (
                    range,
                    HighlightStyle {
                        color: Some(muted),
                        ..Default::default()
                    },
                )
            }));
            let text = if highlights.is_empty() {
                div()
                    .child(SharedString::from(projection.text.clone()))
                    .into_any_element()
            } else {
                StyledText::new(SharedString::from(projection.text.clone()))
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
            for decoration in document_color_decorations
                .iter()
                .filter(|color| color.visual_row == display_row.visual_row)
            {
                let color_value = decoration.color;
                let packed_rgb = (u32::from(color_value.red) << 16)
                    | (u32::from(color_value.green) << 8)
                    | u32::from(color_value.blue);
                let color: gpui::Hsla = gpui::rgb(packed_rgb).into();
                let color = color.opacity(f32::from(color_value.alpha) / 255.0);
                let start = decoration.start_column as f32 * char_w;
                let width = (decoration.end_column - decoration.start_column) as f32 * char_w;
                match self.prefs.lsp_document_colors() {
                    EditorDocumentColorMode::Off => {}
                    EditorDocumentColorMode::Inline => {
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(start))
                                .top_0()
                                .w(px(width))
                                .h(px(line_h))
                                .bg(color.opacity(0.2)),
                        );
                    }
                    EditorDocumentColorMode::Inlay => {
                        let size = char_w.min(line_h);
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(
                                    decoration.end_column as f32 * char_w + (char_w - size) * 0.5,
                                ))
                                .top(px((line_h - size) * 0.5))
                                .w(px(size))
                                .h(px(size))
                                .rounded(px(2.0))
                                .border_1()
                                .border_color(ui_palette.border)
                                .bg(color),
                        );
                    }
                }
            }
            for diagnostic in diagnostic_decorations
                .iter()
                .filter(|diagnostic| diagnostic.visual_row == display_row.visual_row)
            {
                let (diagnostic_color, underline_height) = match diagnostic.severity {
                    labonair_editor::language_services::DiagnosticSeverity::Error => {
                        (ui_palette.error, 2.0)
                    }
                    labonair_editor::language_services::DiagnosticSeverity::Warning => {
                        (ui_palette.warning, 1.5)
                    }
                    labonair_editor::language_services::DiagnosticSeverity::Information
                    | labonair_editor::language_services::DiagnosticSeverity::Hint => {
                        (ui_palette.info, 1.0)
                    }
                };
                row = row.child(
                    div()
                        .absolute()
                        .left(px(diagnostic.start_column as f32 * char_w))
                        .bottom_0()
                        .w(px(
                            (diagnostic
                                .end_column
                                .saturating_sub(diagnostic.start_column)
                                .max(1) as f32)
                                * char_w,
                        ))
                        .h(px(underline_height))
                        .bg(diagnostic_color.opacity(0.9)),
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
            row = row.child(text);
            for link in self
                .document_links
                .iter()
                .filter(|link| link.range.start.line == line && link.range.end.line == line)
            {
                let start = link.range.start.to_position(&snapshot);
                let end = link.range.end.to_position(&snapshot);
                let start_column = display_row
                    .display_column_for_source_position(start)
                    .unwrap_or(start.column)
                    .saturating_sub(display.viewport.left_column);
                let end_column = display_row
                    .display_column_for_source_position(end)
                    .unwrap_or(end.column)
                    .saturating_sub(display.viewport.left_column);
                let width = end_column.saturating_sub(start_column) as f32 * char_w;
                if width > 0.0 {
                    row = row.child(
                        div()
                            .absolute()
                            .left(px(start_column as f32 * char_w))
                            .bottom_0()
                            .w(px(width))
                            .h(px(1.0))
                            .bg(accent.opacity(0.85)),
                    );
                }
            }
            let is_final_text_segment = match display_row.kind {
                DisplayRowKind::Text { end_column, .. } => end_column >= snapshot.line_len(line),
                DisplayRowKind::FoldPlaceholder { .. } => false,
            };
            if is_final_text_segment && code_lens_enabled {
                let display_end = display_row
                    .display_column_for_source_position(Position::new(
                        line,
                        snapshot.line_len(line),
                    ))
                    .unwrap_or(snapshot.line_len(line))
                    .saturating_sub(display.viewport.left_column);
                let mut left = (display_end as f32 * char_w + 8.0).max(0.0);
                for (index, lens) in self
                    .code_lenses
                    .iter()
                    .enumerate()
                    .filter(|(_, lens)| lens.range.start.line == line)
                {
                    let Some(command) = lens.command.clone() else {
                        continue;
                    };
                    let title = command.title.clone();
                    let tooltip = format!("Run Code Lens: {title}");
                    let width = (title.chars().count() as f32 * char_w * 0.7 + 18.0).max(32.0);
                    row = row.child(
                        button(
                            ("editor-code-lens", index),
                            ui_palette,
                            ButtonVariant::Subtle,
                            ButtonSize::Xs,
                        )
                        .child(title)
                        .tooltip(move |window, cx| {
                            Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                            this.execute_code_lens(command.clone(), cx);
                        }))
                        .absolute()
                        .left(px(left))
                        .top_0()
                        .h(px(line_h)),
                    );
                    left += width;
                }
            }
            if inline_diagnostics_enabled
                && diagnostics_enabled
                && self.error_lens_revision == Some(snapshot.revision())
                && is_final_text_segment
            {
                if let Some((severity, message)) = error_lens_for_line(
                    &visible_diagnostics,
                    line,
                    self.prefs.diagnostics_max_severity(),
                ) {
                    let line_length = snapshot.line_len(line);
                    let (row_start, _) = match display_row.kind {
                        DisplayRowKind::Text {
                            start_column,
                            end_column,
                            ..
                        } => (start_column, end_column),
                        DisplayRowKind::FoldPlaceholder { .. } => (0, 0),
                    };
                    let source_end = display_row
                        .display_column_for_source_position(Position::new(line, line_length))
                        .unwrap_or_else(|| line_length.saturating_sub(row_start));
                    let trailing_hint_width = display_row
                        .inline_hints
                        .iter()
                        .filter(|hint| hint.source_column == line_length)
                        .map(|hint| {
                            hint.label.chars().count()
                                + usize::from(hint.padding_left)
                                + usize::from(hint.padding_right)
                        })
                        .sum::<usize>();
                    let minimum_column = (self.prefs.error_lens_minimum_column() as usize)
                        .saturating_sub(row_start);
                    let lens_column = source_end
                        .saturating_add(trailing_hint_width)
                        .max(minimum_column);
                    let left = (lens_column as f32 * char_w
                        - display.viewport.left_column as f32 * char_w
                        + self.prefs.error_lens_spacing() as f32 * font_px)
                        .max(0.0);
                    let available = (content_w - left).max(0.0);
                    if available > 0.0 {
                        let lens_color = match severity {
                            labonair_editor::language_services::DiagnosticSeverity::Error => {
                                ui_palette.error
                            }
                            labonair_editor::language_services::DiagnosticSeverity::Warning => {
                                ui_palette.warning
                            }
                            labonair_editor::language_services::DiagnosticSeverity::Information
                            | labonair_editor::language_services::DiagnosticSeverity::Hint => {
                                ui_palette.info
                            }
                        };
                        row = row.child(
                            div()
                                .absolute()
                                .left(px(left))
                                .top_0()
                                .h(px(line_h))
                                .max_w(px(available))
                                .flex()
                                .items_center()
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_size(px((font_px * 0.85).max(8.0)))
                                .text_color(lens_color.opacity(0.9))
                                .child(SharedString::from(message)),
                        );
                    }
                }
            }
            row
        });

        // Caret + current-line band use the same display map as mouse mapping.
        self.editor_focused = self.focus_handle.is_focused(window);
        let caret_point = display.position_to_display(cursor);
        let show_caret = !self.prefs.cursor_blink() || !self.editor_focused || self.cursor_blink_on;
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
                    EditorCursorStyle::Block => (0.0, char_w.max(2.0), line_h, accent.opacity(0.6)),
                };
                div()
                    .absolute()
                    .top(px((visual_row.saturating_sub(first_visual) as f32)
                        * line_h
                        + top))
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
        let minimap = minimap_policy_visible(
            self.minimap_visibility(),
            self.prefs.minimap_display_in(),
            self.editor_focused,
            display.total_rows > display.viewport.height_rows,
        );
        let minimap_width = minimap_width_pixels(self.prefs.minimap_max_width_columns());
        let mut buffer_surface = div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0();
        buffer_surface = buffer_surface.children(self.render_editor_toolbar(
            breadcrumb_state,
            ui_palette,
            window,
            cx,
        ));
        if show_text_area {
            let mut editor_surface_container = div().relative().flex_1().min_w_0().min_h_0();
            if minimap {
                editor_surface_container = editor_surface_container.pr(px(minimap_width));
            }
            editor_surface_container = editor_surface_container.child(editor_surface);
            if minimap {
                editor_surface_container = editor_surface_container
                    .child(self.render_minimap(&snapshot, &display, ui_palette, cx));
            }
            buffer_surface = buffer_surface.child(editor_surface_container);
        }
        if self.prefs.show_outline() || self.outline_open {
            buffer_surface = buffer_surface.child(self.render_outline(&symbols, ui_palette, cx));
        }

        div()
            .id("editor")
            .track_focus(&self.focus_handle)
            .key_context(self.keymap_context())
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
            .children(self.render_language_service_menu(ui_palette, cx))
            .children(self.render_diagnostics_menu(ui_palette, cx))
            .child(buffer_surface)
            .children(self.render_selection_menu(ui_palette, cx))
            .children(self.render_editor_controls_menu(ui_palette, cx))
            .children(self.render_language_menu(ui_palette, cx))
            .children(self.render_line_ending_menu(ui_palette, cx))
            .children(self.render_encoding_menu(ui_palette, cx))
            .when(self.vim.is_some(), |d| d.child(self.render_vim_status(cx)))
            .when(self.helix.is_some(), |d| {
                d.child(self.render_helix_status(cx))
            })
            .child(self.render_editor_status(cx))
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    if !this
                        .bounds
                        .is_some_and(|bounds| bounds.contains(&ev.position))
                    {
                        return;
                    }
                    if ev.modifiers.platform || ev.modifiers.control {
                        let position = this.position_at(ev.position);
                        let target = this
                            .document_link_at_position(position)
                            .and_then(|link| link.target.clone());
                        if let (Some(target), Some(navigate)) =
                            (target, this.document_link_navigator.clone())
                        {
                            navigate(target, window, cx);
                            cx.stop_propagation();
                            cx.notify();
                            return;
                        }
                    }
                    window.focus(&this.focus_handle);
                    this.begin_selection_drag(ev.position, ev.modifiers.shift, ev.modifiers.alt);
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _window, cx| {
                if ev.pressed_button == Some(MouseButton::Left) && this.selection_drag.is_some() {
                    this.update_selection_drag(ev.position);
                    cx.notify();
                } else if this.selection_drag.is_none() {
                    this.update_hover(ev.position, cx);
                    let position = this.position_at(ev.position);
                    this.schedule_language_hover(position, cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _ev: &MouseUpEvent, _window, cx| {
                    this.finish_selection_drag();
                    cx.notify();
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _ev: &MouseUpEvent, _window, cx| {
                    this.finish_selection_drag();
                    cx.notify();
                }),
            )
            .children(self.render_language_service_overlay(cx))
            .children(self.render_signature_help(cx))
            .children(
                rename_input
                    .as_ref()
                    .map(|input| self.render_rename_dialog(input, ui_palette, window, cx)),
            )
            .children(
                go_to_line_input
                    .as_ref()
                    .map(|input| self.render_go_to_line_dialog(input, ui_palette, window, cx)),
            )
            .children(self.render_hover_tooltip(cx))
            .children(self.render_git_context_menu(cx))
            .children(self.render_text_context_menu(cx))
            .children(self.render_character_palette(window, cx))
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

fn parse_line_column(value: &str) -> Option<Position> {
    let (line, column) = value
        .trim()
        .split_once(':')
        .map_or((value, None), |(line, column)| (line, Some(column)));
    let line = line.trim().parse::<usize>().ok()?;
    let column = match column {
        Some(column) => column.trim().parse::<usize>().ok()?,
        None => 1,
    };
    if line == 0 || column == 0 {
        return None;
    }
    Some(Position::new(line - 1, column - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, KeyBindingContextPredicate, TestAppContext};
    use labonair_editor::SearchOptions;
    use labonair_settings::content::editor::EditorContent;
    use labonair_settings::{SettingsContent, SettingsLayer, SettingsStore};

    #[test]
    fn line_column_parser_uses_one_based_user_coordinates() {
        assert_eq!(parse_line_column("12"), Some(Position::new(11, 0)));
        assert_eq!(parse_line_column(" 12 : 5 "), Some(Position::new(11, 4)));
        assert_eq!(parse_line_column("0"), None);
        assert_eq!(parse_line_column("1:0"), None);
        assert_eq!(parse_line_column("one"), None);
        assert_eq!(parse_line_column("1:2:3"), None);
    }

    #[gpui::test]
    fn go_to_line_column_command_opens_the_editor_input(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                assert!(!this.go_to_line_requested);
                assert!(this.dispatch_registered_command(CommandId::EditorGoToLineColumn, cx));
                assert!(this.go_to_line_requested);
            });
        });
    }

    #[gpui::test]
    fn following_markdown_preview_command_emits_typed_editor_source(cx: &mut TestAppContext) {
        let view = setup(cx);
        let captured = Arc::new(std::sync::Mutex::new(None));
        let captured_event = captured.clone();
        let _subscription = cx.update(|cx| {
            cx.subscribe(&view, move |_, event: &EditorEvent, _| {
                if let EditorEvent::MarkdownFollowingPreviewRequested(source) = event {
                    if let Ok(mut captured) = captured_event.lock() {
                        *captured = Some(source.clone());
                    }
                }
            })
        });

        cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.doc = Document::from_file("guide.md".into(), "# Guide", 1);
                assert!(view.dispatch_registered_command(
                    CommandId::EditorOpenFollowingMarkdownPreview,
                    cx
                ));
            });
        });

        let source = captured
            .lock()
            .expect("event capture lock")
            .clone()
            .expect("following preview event");
        assert_eq!(
            source.path.as_deref(),
            Some(std::path::Path::new("guide.md"))
        );
        assert_eq!(source.language, Language::Markdown);
        assert_eq!(source.snapshot.text(), "# Guide");
    }

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
            let theme = cx.new(|_| ThemeStore::new(gpui::WindowAppearance::Light));
            cx.new(|cx| EditorView::new(theme, cx))
        })
    }

    #[gpui::test]
    fn call_hierarchy_commands_open_a_contextual_picker(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("main.rs".into(), "fn main() {}", 1);
                assert!(this.dispatch_registered_command(CommandId::EditorShowIncomingCalls, cx));
                assert!(this.call_hierarchy_open);
                assert_eq!(
                    this.call_hierarchy_direction,
                    CallHierarchyDirection::Incoming
                );
                assert!(key_context_matches(
                    &this.keymap_context(),
                    "CallHierarchyPicker"
                ));
            });
        });
    }

    #[gpui::test]
    fn call_hierarchy_rows_keep_typed_targets_and_utf16_navigation(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("main.rs".into(), "a😀bc\n", 1);
                let position = LspPosition {
                    line: 0,
                    character: 3,
                };
                this.goto_lsp_position(position, cx);
                assert_eq!(this.cursor_position(), Position::new(0, 2));

                let item = CallHierarchyItem {
                    name: "caller".into(),
                    kind: 12,
                    uri: "file:///repo/caller.rs".into(),
                    range: LspRange {
                        start: position,
                        end: position,
                    },
                    selection_range: LspRange {
                        start: position,
                        end: position,
                    },
                    detail: Some("crate::caller".into()),
                    tags: Vec::new(),
                    data: None,
                };
                this.apply_language_service_response(
                    Ok(LanguageServiceResponse::CallHierarchyCalls(vec![
                        CallHierarchyCall {
                            item: item.clone(),
                            ranges: vec![item.selection_range],
                        },
                    ])),
                    LanguageServiceRequestSlot::CallHierarchy,
                    cx,
                );
                assert!(!this.call_hierarchy_loading);
                assert_eq!(this.call_hierarchy_calls[0].item.name, "caller");
                assert_eq!(
                    this.call_hierarchy_calls[0].item.uri,
                    "file:///repo/caller.rs"
                );
            });
        });
    }

    fn key_context_matches(context: &KeyContext, expression: &str) -> bool {
        let Ok(predicate) = KeyBindingContextPredicate::parse(expression) else {
            return false;
        };
        let contexts = std::slice::from_ref(context);
        predicate.eval_inner(contexts, contexts)
    }

    #[gpui::test]
    fn html_greater_than_input_routes_through_editor_tag_completion(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("index.html".into(), "<section", 1);
                this.doc.set_caret(Position::new(0, 8), false);

                this.insert_printable(">", cx);

                assert_eq!(this.doc.text(), "<section></section>");
                assert_eq!(this.doc.cursor, Position::new(0, 9));
            });
        });
    }

    #[gpui::test]
    fn alt_pointer_drag_adds_rectangular_selections_and_undoes_as_one_step(
        cx: &mut TestAppContext,
    ) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.metrics = (10.0, 18.0);
                this.gutter_width = 0.0;
                this.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(120.0),
                        height: px(80.0),
                    },
                });
                this.doc =
                    Document::from_file("test.txt".into(), "abcdef\nuvwxyz\n012345\nlast", 1);
                this.doc.cursor = Position::new(3, 1);

                this.begin_selection_drag(gpui::point(px(30.0), px(0.0)), false, true);
                this.update_selection_drag(gpui::point(px(50.0), px(36.0)));
                this.finish_selection_drag();

                let selections = this.doc.selection_set();
                assert_eq!(selections.len(), 4);
                let snapshot = this.doc.snapshot();
                let selected_rows = selections
                    .ranges(&snapshot)
                    .into_iter()
                    .filter(|range| range.start < range.end)
                    .map(|range| snapshot.text_range(range))
                    .collect::<Vec<_>>();
                assert_eq!(selected_rows, ["de", "xy", "34"]);
                assert!(this.doc.undo_selection());
                assert_eq!(this.doc.selection_set().len(), 1);
                assert_eq!(this.doc.cursor, Position::new(3, 1));
                assert!(!this.doc.undo_selection());
            });
        });
    }

    #[test]
    fn completion_selection_moves_and_wraps() {
        assert_eq!(next_completion_selection(0, 3, -1), 2);
        assert_eq!(next_completion_selection(2, 3, 1), 0);
        assert_eq!(next_completion_selection(99, 3, -1), 1);
        assert_eq!(next_completion_selection(4, 0, 1), 0);
    }

    #[test]
    fn editor_controls_hide_language_features_without_provider_support() {
        let mut capabilities = LanguageServiceCapabilities::SYNTAX_DERIVED;
        capabilities.inlay_hints = false;
        capabilities.code_lens = false;
        capabilities.semantic_tokens = false;
        let controls = available_editor_controls(capabilities);

        assert!(controls.contains(&EditorControl::Diagnostics));
        assert!(controls.contains(&EditorControl::GitBlame));
        assert!(controls.contains(&EditorControl::LineNumbers));
        assert!(controls.contains(&EditorControl::Minimap));
        assert!(!controls.contains(&EditorControl::InlayHints));
        assert!(!controls.contains(&EditorControl::CodeLens));
        assert!(!controls.contains(&EditorControl::SemanticHighlights));

        capabilities.inlay_hints = true;
        capabilities.code_lens = true;
        capabilities.execute_command = false;
        capabilities.semantic_tokens = true;
        let controls = available_editor_controls(capabilities);
        assert!(controls.contains(&EditorControl::InlayHints));
        assert!(!controls.contains(&EditorControl::CodeLens));
        capabilities.execute_command = true;
        let controls = available_editor_controls(capabilities);
        assert!(controls.contains(&EditorControl::CodeLens));
        assert!(controls.contains(&EditorControl::SemanticHighlights));
    }

    #[test]
    fn language_service_controls_require_an_injected_process() {
        let local = LanguageServiceRuntimeSnapshot {
            language: LanguageId::new(Language::Rust),
            server: labonair_editor::LanguageServerId::new("rust-analyzer"),
            source: LanguageServiceRuntimeSource::LocalProcess,
            status: LanguageServiceRuntimeStatus::Running,
            capabilities: LanguageServiceCapabilities::SYNTAX_DERIVED,
            version: DocumentVersion::new(1),
            generation: 2,
            detail: None,
        };
        assert!(language_service_controls_available(&local));
        assert_eq!(language_service_status_label(local.status), "Running");

        let mut stopped = local.clone();
        stopped.status = LanguageServiceRuntimeStatus::Fallback;
        assert!(language_service_controls_available(&stopped));
        assert_eq!(language_service_status_label(stopped.status), "Stopped");

        let mut fallback = local;
        fallback.source = LanguageServiceRuntimeSource::SyntaxFallback;
        fallback.status = LanguageServiceRuntimeStatus::Fallback;
        assert!(!language_service_controls_available(&fallback));
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
                                    text_format: labonair_editor::CompletionTextFormat::PlainText,
                                    detail: None,
                                    insert_text: "print".into(),
                                    text_edit_range: None,
                                    filter_text: None,
                                    sort_text: None,
                                    documentation: None,
                                },
                                labonair_editor::CompletionItem {
                                    label: "println".into(),
                                    kind: labonair_editor::CompletionKind::Function,
                                    text_format: labonair_editor::CompletionTextFormat::PlainText,
                                    detail: None,
                                    insert_text: "println".into(),
                                    text_edit_range: None,
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
    fn completion_snippets_replace_text_and_navigate_tabstops(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.doc = Document::from_file("completion.rs".into(), "pr", 1);
                view.doc.move_caret(Motion::DocEnd, false);
                view.language_service_generation = 1;
                let version = DocumentVersion::from_revision(view.editor_revision());
                view.language_service_state
                    .set_document_generation(version, 1);
                assert!(view.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test"),
                    version,
                    generation: 1,
                    response: Ok(LanguageServiceResponse::Completion(
                        labonair_editor::CompletionList {
                            items: vec![labonair_editor::CompletionItem {
                                label: "print".into(),
                                kind: labonair_editor::CompletionKind::Snippet,
                                text_format: labonair_editor::CompletionTextFormat::Snippet,
                                detail: None,
                                insert_text: "print_${1:arg}(${2:value})$0".into(),
                                text_edit_range: None,
                                filter_text: None,
                                sort_text: None,
                                documentation: None,
                            }],
                            is_incomplete: false,
                        },
                    )),
                }));
                view.completion_visible = true;
                view.completion_position = Some(view.current_lsp_position());
                view.apply_completion_item(0, cx);

                assert_eq!(view.doc.text(), "print_arg(value)");
                assert_eq!(view.doc.selected_text().as_deref(), Some("arg"));
                let context = view.keymap_context();
                assert!(key_context_matches(&context, "EditorSnippetNext"));
                assert!(!key_context_matches(&context, "EditorSnippetPrevious"));

                assert!(view.dispatch_registered_command(CommandId::EditorNextSnippetTabstop, cx,));
                assert_eq!(view.doc.selected_text().as_deref(), Some("value"));
                assert!(key_context_matches(
                    &view.keymap_context(),
                    "EditorSnippetNext"
                ));
                assert!(key_context_matches(
                    &view.keymap_context(),
                    "EditorSnippetPrevious"
                ));

                assert!(view.dispatch_registered_command(CommandId::EditorNextSnippetTabstop, cx,));
                assert_eq!(view.doc.selected_text(), None);
                let context = view.keymap_context();
                assert!(!key_context_matches(&context, "EditorSnippetNext"));
                assert!(key_context_matches(&context, "EditorSnippetPrevious"));

                assert!(
                    view.dispatch_registered_command(CommandId::EditorPreviousSnippetTabstop, cx,)
                );
                assert_eq!(view.doc.selected_text().as_deref(), Some("value"));
            });
        });
    }

    #[gpui::test]
    fn completion_uses_lsp_edit_range_and_respects_plain_text_format(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.doc = Document::from_file("completion.rs".into(), "object.field", 1);
                view.doc.move_caret(Motion::DocEnd, false);
                view.language_service_generation = 1;
                let version = DocumentVersion::from_revision(view.editor_revision());
                view.language_service_state
                    .set_document_generation(version, 1);
                assert!(view.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test"),
                    version,
                    generation: 1,
                    response: Ok(LanguageServiceResponse::Completion(
                        labonair_editor::CompletionList {
                            items: vec![labonair_editor::CompletionItem {
                                label: "literal".into(),
                                kind: labonair_editor::CompletionKind::Text,
                                text_format: labonair_editor::CompletionTextFormat::PlainText,
                                detail: None,
                                insert_text: "${1:name}".into(),
                                text_edit_range: Some(labonair_editor::LspRange {
                                    start: LspPosition {
                                        line: 0,
                                        character: 0,
                                    },
                                    end: LspPosition {
                                        line: 0,
                                        character: 12,
                                    },
                                }),
                                filter_text: None,
                                sort_text: None,
                                documentation: None,
                            }],
                            is_incomplete: false,
                        },
                    )),
                }));
                view.completion_visible = true;
                view.completion_position = Some(view.current_lsp_position());

                view.apply_completion_item(0, cx);

                assert_eq!(view.doc.text(), "${1:name}");
                assert_eq!(view.doc.selected_text(), None);
            });
        });
    }

    #[gpui::test]
    fn completion_snippets_expand_file_and_cursor_variables(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.doc = Document::from_file("src/main.rs".into(), "let value = word", 1);
                view.doc.move_caret(Motion::DocEnd, false);
                view.language_service_generation = 1;
                let version = DocumentVersion::from_revision(view.editor_revision());
                view.language_service_state
                    .set_document_generation(version, 1);
                assert!(view.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test"),
                    version,
                    generation: 1,
                    response: Ok(LanguageServiceResponse::Completion(
                        labonair_editor::CompletionList {
                            items: vec![labonair_editor::CompletionItem {
                                label: "contextual snippet".into(),
                                kind: labonair_editor::CompletionKind::Snippet,
                                text_format: labonair_editor::CompletionTextFormat::Snippet,
                                detail: None,
                                insert_text: "${TM_FILENAME_BASE}:${TM_CURRENT_WORD}:${TM_LINE_NUMBER}:${1:arg}".into(),
                                text_edit_range: None,
                                filter_text: None,
                                sort_text: None,
                                documentation: None,
                            }],
                            is_incomplete: false,
                        },
                    )),
                }));
                view.completion_visible = true;
                view.completion_position = Some(view.current_lsp_position());

                view.apply_completion_item(0, cx);

                assert_eq!(view.doc.text(), "let value = main:word:1:arg");
                assert_eq!(view.doc.selected_text().as_deref(), Some("arg"));
            });
        });
    }

    #[gpui::test]
    fn manual_language_mode_refreshes_editor_owned_language_state(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |view, cx| {
                view.doc = Document::from_file("notes.txt".into(), "fn main() {}", 1);
                assert_eq!(view.doc.language, Language::PlainText);

                view.set_language_mode(Language::Rust, cx);

                assert_eq!(view.doc.language, Language::Rust);
                assert_eq!(view.syntax.language(), Language::Rust);
                assert_eq!(view.session_snapshot().language, "Rust");
                assert!(view.language_service_open);
            });
        });
    }

    #[gpui::test]
    fn manual_language_mode_restores_from_editor_session(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |view, _cx| {
                view.doc = Document::from_file("notes.txt".into(), "fn main() {}", 1);
                let mut session = view.session_snapshot();
                session.language = "Rust".to_owned();
                view.set_pending_session(session);

                view.apply_pending_session();
                view.resync_syntax();

                assert_eq!(view.doc.language, Language::Rust);
                assert_eq!(view.syntax.language(), Language::Rust);
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

    #[gpui::test]
    fn registered_read_only_command_locks_and_unlocks_the_document(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("lock.txt".into(), "text", 1);

                assert!(this.dispatch_registered_command(CommandId::EditorToggleReadOnly, cx));
                assert!(this.doc.temporary_read_only());
                assert!(!this.doc.can_edit());

                assert!(this.dispatch_registered_command(CommandId::EditorToggleReadOnly, cx));
                assert!(!this.doc.temporary_read_only());
                assert!(this.doc.can_edit());
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

    #[gpui::test]
    fn registered_navigation_commands_restore_and_branch_location_history(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file(
                    "test.rs".into(),
                    "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight",
                    1,
                );
                this.goto_line(2, cx);
                this.goto_line(5, cx);
                assert_eq!(this.cursor_position(), Position::new(5, 0));

                assert!(this.dispatch_registered_command(CommandId::EditorGoBack, cx));
                assert_eq!(this.cursor_position(), Position::new(2, 0));
                assert!(this.dispatch_registered_command(CommandId::EditorGoForward, cx));
                assert_eq!(this.cursor_position(), Position::new(5, 0));

                assert!(this.dispatch_registered_command(CommandId::EditorGoBack, cx));
                this.goto_line(7, cx);
                assert!(this.dispatch_registered_command(CommandId::EditorGoBack, cx));
                assert_eq!(this.cursor_position(), Position::new(2, 0));
                assert!(this.dispatch_registered_command(CommandId::EditorGoForward, cx));
                assert_eq!(this.cursor_position(), Position::new(7, 0));
            });
        });
    }

    #[gpui::test]
    fn conflict_overwrite_action_saves_the_local_buffer(cx: &mut TestAppContext) {
        let path = std::env::temp_dir().join(format!(
            "labonair-editor-overwrite-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::write(&path, "before").unwrap();

        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| this.open_path(path.clone(), cx));
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                assert_eq!(this.doc.file_state(), &FileState::Clean);
                this.doc.move_caret(Motion::DocEnd, false);
                this.doc.insert(" local");
            });
        });

        std::fs::write(&path, "external").unwrap();
        let external_identity = stat_editor_file_sync(&path.to_string_lossy())
            .expect("stat external edit")
            .identity;
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.doc
                    .observe_file(Some(EditorView::editor_identity(external_identity)))
                    .unwrap();
                assert_eq!(this.doc.file_state(), &FileState::Conflict);
            });
        });

        cx.update(|cx| {
            view.update(cx, |this, cx| this.overwrite_external(cx));
        });
        cx.background_executor.run_until_parked();
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                assert_eq!(this.doc.file_state(), &FileState::Clean);
                assert_eq!(this.doc.text(), "before local");
            });
        });
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "before local");
        std::fs::remove_file(path).unwrap();
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
    fn center_on_match_preference_centers_the_active_buffer_search_result(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| editor.search_center_on_match = Some(true));
        });
        cx.update(|cx| {
            view.update(cx, |editor, cx| {
                let text = (0..40)
                    .map(|line| if line == 20 { "target" } else { "line" })
                    .collect::<Vec<_>>()
                    .join("\n");
                editor.doc = Document::from_file("many.rs".into(), &text, 1);
                editor.metrics = (10.0, 18.0);
                editor.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(500.0),
                        height: px(180.0),
                    },
                });
                editor.apply_prefs(cx);
                assert_eq!(
                    editor
                        .search_set(SearchQuery::new("target", SearchOptions::default()), cx)
                        .unwrap(),
                    (1, 1)
                );
                assert_eq!(editor.scroll_top, 15);
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
                e.editor_modal_mode = Some(EditorModalMode::Vim);
            });
        });
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.apply_prefs(cx);
                assert!(v.vim.is_some(), "vim enabled via prefs");
                assert!(v.helix.is_none(), "Vim takes precedence over Helix");
                assert_eq!(v.indent_unit(), "   ");
                let key_context = v.keymap_context();
                assert!(key_context_matches(
                    &key_context,
                    "EditorVim && editor_mode == vim && modal_mode == normal",
                ));
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |e| {
                e.editor_modal_mode = Some(EditorModalMode::Helix);
                e.editor_indent_with_tabs = Some(true);
            });
        });
        cx.update(|cx| {
            view.update(cx, |v, cx| {
                v.apply_prefs(cx);
                assert!(v.vim.is_none(), "vim disabled via prefs");
                assert!(v.helix.is_some(), "Helix enabled via prefs");
                assert_eq!(v.indent_unit(), "\t");
            });
        });
    }

    #[gpui::test]
    fn toggle_inlay_hints_is_editor_owned_and_overrides_the_default_off_setting(
        cx: &mut TestAppContext,
    ) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                assert!(!this.inlay_hints_enabled());
                assert!(this.dispatch_registered_command(CommandId::EditorToggleInlayHints, cx));
                assert!(this.inlay_hints_enabled());
                assert!(this.dispatch_registered_command(CommandId::EditorToggleInlayHints, cx));
                assert!(!this.inlay_hints_enabled());
                assert!(this
                    .language_service_state
                    .snapshot()
                    .inlay_hints
                    .is_empty());
            });
        });
    }

    #[gpui::test]
    fn inlay_hint_results_are_discarded_after_viewport_changes(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.doc = Document::from_file("test.rs".into(), "a\nb\nc", 1);
                this.inlay_hints_request_serial = Some(4);
                this.inlay_hints_request_range = Some(this.visible_inlay_range());
                let range = this.inlay_hints_request_range;
                assert!(this.is_current_language_service_request(
                    LanguageServiceRequestSlot::InlayHints,
                    4,
                    None,
                ));

                this.scroll_top = 1;
                assert_ne!(this.visible_inlay_range(), range.expect("requested range"));
                assert!(!this.is_current_language_service_request(
                    LanguageServiceRequestSlot::InlayHints,
                    4,
                    None,
                ));
            });
        });
    }

    #[gpui::test]
    fn inlay_hints_project_as_virtual_text_and_respect_kind_settings(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.doc = Document::from_file("test.rs".into(), "let x=1;", 1);
                this.inlay_hints_enabled_override = Some(true);
                let version = DocumentVersion::from_revision(this.editor_revision());
                let generation = this.language_service_generation;
                this.language_service_state
                    .set_document_generation(version, generation);
                assert!(this.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test"),
                    version,
                    generation,
                    response: Ok(LanguageServiceResponse::InlayHints(vec![InlayHint {
                        position: LspPosition {
                            line: 0,
                            character: 5,
                        },
                        label: ": i32".into(),
                        kind: InlayHintKind::Type,
                        tooltip: None,
                        padding_left: true,
                        padding_right: false,
                    }])),
                }));

                let snapshot = this.doc.snapshot();
                let config = this.display_config();
                let layout = this.display_layout(&snapshot, config);
                assert_eq!(
                    layout.rows[0].text_projection(&snapshot).text,
                    "let x : i32=1;"
                );
                assert_eq!(this.doc.text(), "let x=1;");

                this.prefs =
                    editor_prefs(|settings| settings.editor_inlay_type_hints = Some(false));
                this.display_layout_cache = None;
                let config = this.display_config();
                let layout = this.display_layout(&snapshot, config);
                assert_eq!(layout.rows[0].text_projection(&snapshot).text, "let x=1;");
            });
        });
    }

    #[gpui::test]
    fn inlay_hint_tooltip_appears_over_virtual_text(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.rs".into(), "abc", 1);
                this.metrics = (10.0, 18.0);
                this.gutter_width = 0.0;
                this.inlay_hints_enabled_override = Some(true);
                this.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(100.0),
                        height: px(40.0),
                    },
                });
                let version = DocumentVersion::from_revision(this.editor_revision());
                let generation = this.language_service_generation;
                this.language_service_state
                    .set_document_generation(version, generation);
                assert!(this.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test-lsp"),
                    version,
                    generation,
                    response: Ok(LanguageServiceResponse::InlayHints(vec![InlayHint {
                        position: LspPosition {
                            line: 0,
                            character: 1,
                        },
                        label: ": String".into(),
                        kind: InlayHintKind::Type,
                        tooltip: Some("The inferred type of this value".into()),
                        padding_left: true,
                        padding_right: false,
                    }])),
                }));

                this.update_hover(gpui::point(px(15.0), px(0.0)), cx);
                assert_eq!(
                    this.hover.as_ref().map(|(_, text)| text.as_ref()),
                    Some("The inferred type of this value")
                );
            });
        });
    }

    #[gpui::test]
    fn document_color_hover_shows_the_server_color_value(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.css".into(), "color: #ff00aa;", 1);
                this.metrics = (10.0, 18.0);
                this.gutter_width = 0.0;
                this.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(200.0),
                        height: px(40.0),
                    },
                });
                let version = DocumentVersion::from_revision(this.editor_revision());
                let generation = this.language_service_generation;
                this.language_service_state
                    .set_document_generation(version, generation);
                let color = DocumentColor {
                    range: LspRange {
                        start: LspPosition {
                            line: 0,
                            character: 7,
                        },
                        end: LspPosition {
                            line: 0,
                            character: 14,
                        },
                    },
                    color: DocumentColorValue {
                        red: 255,
                        green: 0,
                        blue: 170,
                        alpha: 255,
                    },
                };
                assert!(this.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test-lsp"),
                    version,
                    generation,
                    response: Ok(LanguageServiceResponse::DocumentColors(vec![color])),
                }));

                let snapshot = this.doc.snapshot();
                let display = this
                    .display_layout(&snapshot, this.display_config())
                    .clone();
                assert_eq!(
                    display.rows[0].text_projection(&snapshot).text,
                    "color: #ff00aa ;"
                );
                this.update_hover(gpui::point(px(80.0), px(0.0)), cx);
                assert_eq!(
                    this.hover.as_ref().map(|(_, text)| text.as_ref()),
                    Some("#FF00AA")
                );
            });
        });
    }

    #[gpui::test]
    fn helix_mode_routes_selection_actions_and_insert_input(cx: &mut TestAppContext) {
        cx.update(|cx| {
            install_editor_prefs(cx, |settings| {
                settings.editor_modal_mode = Some(EditorModalMode::Helix);
            });
        });
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.txt".into(), "abc", 1);
                this.apply_prefs(cx);
                assert!(this.helix.is_some());
                assert!(this.vim.is_none());
                let key_context = this.keymap_context();
                assert!(key_context_matches(
                    &key_context,
                    "EditorHelix && editor_mode == helix && modal_mode == select",
                ));

                assert!(this.handle_helix(HelixKey::Char('d'), cx));
                assert_eq!(this.doc.text(), "bc");
                assert_eq!(this.helix.as_ref().map(Helix::status), Some("HELIX SELECT"));

                assert!(this.handle_helix(HelixKey::Char('i'), cx));
                let key_context = this.keymap_context();
                assert!(key_context_matches(
                    &key_context,
                    "EditorHelix && editor_mode == helix && modal_mode == insert",
                ));
                this.insert_printable("x", cx);
                assert_eq!(this.doc.text(), "xbc");
                assert!(this.handle_helix(HelixKey::Escape, cx));
                assert_eq!(this.helix.as_ref().map(Helix::status), Some("HELIX SELECT"));
                assert_eq!(this.doc.selected_text().as_deref(), Some("x"));
            });
        });
    }

    #[gpui::test]
    fn type_definition_response_moves_to_the_server_target(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.rs".into(), "struct Widget;\n", 1);
                let position = LspPosition {
                    line: 0,
                    character: 7,
                };
                let target = labonair_editor::NavigationTarget {
                    name: "Widget".into(),
                    range: LspRange {
                        start: position,
                        end: LspPosition {
                            line: 0,
                            character: 13,
                        },
                    },
                    selection_range: LspRange {
                        start: position,
                        end: LspPosition {
                            line: 0,
                            character: 13,
                        },
                    },
                };
                this.apply_language_service_response(
                    Ok(LanguageServiceResponse::TypeDefinition(vec![target])),
                    LanguageServiceRequestSlot::TypeDefinition,
                    cx,
                );
                assert_eq!(this.doc.cursor, Position::new(0, 7));
            });
        });
    }

    #[gpui::test]
    fn navigation_response_is_discarded_after_the_caret_moves(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.doc = Document::from_file("test.rs".into(), "struct Widget;", 1);
                let position = this.current_lsp_position();
                this.navigation_request_serial = Some(4);
                this.navigation_position = Some(position);
                assert!(this.is_current_language_service_request(
                    LanguageServiceRequestSlot::Navigation,
                    4,
                    Some(position),
                ));

                this.doc.set_caret(Position::new(0, 1), false);
                assert!(!this.is_current_language_service_request(
                    LanguageServiceRequestSlot::Navigation,
                    4,
                    Some(position),
                ));
            });
        });
    }

    #[gpui::test]
    fn registered_indent_commands_use_live_tab_settings(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_indent_with_tabs = Some(false);
                editor.editor_tab_size = Some(3);
            });
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                this.doc = Document::from_file("test.rs".into(), "line", 1);
                this.doc.cursor = Position::default();

                assert!(this.dispatch_registered_command(CommandId::EditorIndent, cx));
                assert_eq!(this.doc.text(), "   line");
                assert!(this.dispatch_registered_command(CommandId::EditorOutdent, cx));
                assert_eq!(this.doc.text(), "line");
            });
        });

        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_indent_with_tabs = Some(true);
            });
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                assert!(this.dispatch_registered_command(CommandId::EditorIndent, cx));
                assert_eq!(this.doc.text(), "\tline");
                assert!(this.dispatch_registered_command(CommandId::EditorOutdent, cx));
                assert_eq!(this.doc.text(), "line");
            });
        });
    }

    #[gpui::test]
    fn selections_menu_preference_hides_its_contribution_and_closes_the_popover(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_toolbar_breadcrumbs = Some(false);
                editor.editor_toolbar_quick_actions = Some(false);
                editor.editor_selections_menu = Some(true);
            });
        });
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, _cx| {
                this.selection_menu_anchor = Some(gpui::point(px(10.0), px(10.0)));
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_toolbar_breadcrumbs = Some(false);
                editor.editor_toolbar_quick_actions = Some(false);
                editor.editor_selections_menu = Some(false);
            });
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                assert!(this.selection_menu_anchor.is_none());
                assert!(!editor_toolbar_has_contribution(
                    this.breadcrumbs_visible(),
                    this.prefs.toolbar_quick_actions(),
                    this.prefs.selections_menu(),
                    this.prefs.toolbar_code_actions()
                        && this.language_service_snapshot.capabilities.code_actions,
                    false,
                ));
                assert!(editor_toolbar_has_contribution(
                    false, false, false, false, true
                ));
            });
        });
    }

    #[gpui::test]
    fn editor_controls_toggle_local_display_and_exclusive_modal_modes(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                assert!(this.line_numbers_enabled());
                this.toggle_editor_control(EditorControl::LineNumbers, cx);
                assert!(!this.display_config().show_line_numbers);

                assert!(!this.editor_control_enabled(EditorControl::GitBlame));
                this.toggle_editor_control(EditorControl::GitBlame, cx);
                assert!(this.editor_control_enabled(EditorControl::GitBlame));

                assert!(!this.minimap_enabled());
                this.toggle_editor_control(EditorControl::Minimap, cx);
                assert!(this.minimap_enabled());
                assert_eq!(this.minimap_visibility(), EditorMinimapVisibility::Always);

                this.toggle_editor_control(EditorControl::Diagnostics, cx);
                assert!(!this.diagnostics_enabled());
                this.toggle_editor_control(EditorControl::InlineDiagnostics, cx);
                assert!(this.editor_control_enabled(EditorControl::InlineDiagnostics));

                this.toggle_editor_control(EditorControl::VimMode, cx);
                assert!(this.vim.is_some());
                assert!(this.helix.is_none());
                this.toggle_editor_control(EditorControl::HelixMode, cx);
                assert!(this.vim.is_none());
                assert!(this.helix.is_some());
                this.toggle_editor_control(EditorControl::HelixMode, cx);
                assert!(this.helix.is_none());
            });
        });
    }

    #[gpui::test]
    fn editor_control_overrides_reconcile_with_global_settings(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.toggle_editor_control(EditorControl::LineNumbers, cx);
                assert!(!this.line_numbers_enabled());
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| editor.editor_line_numbers = Some(false));
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                assert_eq!(this.line_numbers_enabled_override, None);
                assert!(!this.line_numbers_enabled());

                this.toggle_editor_control(EditorControl::VimMode, cx);
                assert!(this.vim.is_some());
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_show_cursor_position = Some(false)
            });
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                assert_eq!(this.modal_mode_override, Some(EditorModalMode::Vim));
                assert!(this.vim.is_some());
            });
        });
        cx.update(|cx| {
            install_editor_prefs(cx, |editor| {
                editor.editor_modal_mode = Some(EditorModalMode::Helix)
            });
        });
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.apply_prefs(cx);
                assert_eq!(this.modal_mode_override, None);
                assert!(this.vim.is_none());
                assert!(this.helix.is_some());
            });
        });
    }

    #[gpui::test]
    fn toggle_breadcrumb_command_overrides_the_global_setting_for_this_editor(
        cx: &mut TestAppContext,
    ) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                assert!(this.breadcrumbs_visible());
                assert!(this.dispatch_registered_command(CommandId::EditorToggleBreadcrumb, cx));
                assert!(!this.breadcrumbs_visible());
                assert!(this.dispatch_registered_command(CommandId::EditorToggleBreadcrumb, cx));
                assert!(this.breadcrumbs_visible());
            });
        });
    }

    #[gpui::test]
    fn typed_brackets_close_skip_and_auto_indent_on_enter(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.rs".into(), "", 1);
                this.insert_printable("{", cx);
                assert_eq!(this.doc.text(), "{}");
                assert_eq!(this.doc.cursor, Position::new(0, 1));

                this.insert_newline(cx);
                assert_eq!(this.doc.text(), "{\n  }");
                assert_eq!(this.doc.cursor, Position::new(1, 2));

                this.insert_printable("}", cx);
                assert_eq!(this.doc.text(), "{\n  }");
                assert_eq!(this.doc.cursor, Position::new(1, 3));
            });
        });
    }

    #[gpui::test]
    fn ordinary_typing_and_multiline_paste_use_secondary_cursors(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.txt".into(), "a b", 1);
                this.doc.set_caret(Position::new(0, 1), false);
                this.doc
                    .execute_edit(EditIntent::AddCursor(Position::new(0, 3)));

                this.insert_printable("x", cx);
                assert_eq!(this.doc.text(), "ax bx");

                this.doc = Document::from_file("test.txt".into(), "a b", 1);
                this.doc.set_caret(Position::new(0, 1), false);
                this.doc
                    .execute_edit(EditIntent::AddCursor(Position::new(0, 3)));
                this.paste_text("left\nright", cx);
                assert_eq!(this.doc.text(), "aleft bright");

                this.doc = Document::from_file("test.txt".into(), "one two", 1);
                this.doc.set_selection_set(SelectionSet::from_selections(
                    vec![
                        Selection {
                            anchor: Anchor::new(0, AnchorAffinity::Before),
                            head: Anchor::new(3, AnchorAffinity::After),
                        },
                        Selection {
                            anchor: Anchor::new(4, AnchorAffinity::Before),
                            head: Anchor::new(7, AnchorAffinity::After),
                        },
                    ],
                    0,
                ));
                assert_eq!(this.clipboard_selection_text().as_deref(), Some("one\ntwo"));
            });
        });
    }

    #[gpui::test]
    fn kill_ring_commands_cut_and_yank_through_the_editor_registry(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.txt".into(), "one two", 1);
                this.doc.set_caret(Position::new(0, 4), false);

                assert!(this.dispatch_registered_command(CommandId::EditorKillRingCut, cx));
                assert_eq!(this.doc.text(), "one ");
                assert!(this.dispatch_registered_command(CommandId::EditorKillRingYank, cx));
                assert_eq!(this.doc.text(), "one two");
            });
        });
    }

    #[gpui::test]
    fn character_palette_inserts_through_an_undoable_editor_transaction(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.txt".into(), "abc", 1);
                this.doc.set_caret(Position::new(0, 1), false);

                assert!(this.dispatch_registered_command(
                    CommandId::EditorShowCharacterPalette,
                    cx
                ));
                assert!(this.character_palette_open);
                this.insert_character_from_palette("©", cx);
                assert!(!this.character_palette_open);
                assert_eq!(this.doc.text(), "a©bc");

                this.edit(cx, Document::undo);
                assert_eq!(this.doc.text(), "abc");
            });
        });
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
    fn scrollbar_visibility_and_diagnostic_severity_follow_settings() {
        use labonair_editor::language_services::DiagnosticSeverity as Severity;

        assert!(scrollbar_policy_visible(
            EditorScrollbarVisibility::Always,
            false
        ));
        assert!(!scrollbar_policy_visible(
            EditorScrollbarVisibility::Never,
            true
        ));
        assert!(!scrollbar_policy_visible(
            EditorScrollbarVisibility::Auto,
            false
        ));
        assert!(scrollbar_policy_visible(
            EditorScrollbarVisibility::System,
            true
        ));

        assert!(scrollbar_shows_diagnostic(
            Severity::Hint,
            EditorDiagnosticMaxSeverity::All
        ));
        assert!(!scrollbar_shows_diagnostic(
            Severity::Hint,
            EditorDiagnosticMaxSeverity::Information
        ));
        assert!(scrollbar_shows_diagnostic(
            Severity::Warning,
            EditorDiagnosticMaxSeverity::Warning
        ));
        assert!(!scrollbar_shows_diagnostic(
            Severity::Information,
            EditorDiagnosticMaxSeverity::Warning
        ));
        assert!(scrollbar_shows_diagnostic(
            Severity::Error,
            EditorDiagnosticMaxSeverity::Error
        ));
        assert!(!scrollbar_shows_diagnostic(
            Severity::Warning,
            EditorDiagnosticMaxSeverity::Error
        ));
    }

    #[test]
    fn diagnostic_hover_prefers_highest_severity_and_includes_source_details() {
        use labonair_editor::language_services::DiagnosticSeverity as Severity;

        let snapshot = labonair_editor::EditorBuffer::from_text("aλbc").snapshot();
        let range = |start, end| LspRange {
            start: LspPosition {
                line: 0,
                character: start,
            },
            end: LspPosition {
                line: 0,
                character: end,
            },
        };
        let diagnostics = vec![
            labonair_editor::Diagnostic {
                range: range(1, 3),
                severity: Severity::Warning,
                message: "review this expression".into(),
                source: labonair_editor::LanguageServerId::new("test-lsp"),
                code: None,
            },
            labonair_editor::Diagnostic {
                range: range(1, 2),
                severity: Severity::Error,
                message: "invalid token".into(),
                source: labonair_editor::LanguageServerId::new("test-lsp"),
                code: Some("E101".into()),
            },
        ];

        let found = diagnostic_at_position(
            &diagnostics,
            &snapshot,
            Position::new(0, 1),
            EditorDiagnosticMaxSeverity::All,
        )
        .expect("diagnostic at the cursor");
        assert_eq!(found.severity, Severity::Error);
        assert_eq!(
            diagnostic_hover_text(found),
            "Error (E101): invalid token\ntest-lsp"
        );
        assert!(diagnostic_at_position(
            &diagnostics,
            &snapshot,
            Position::new(0, 4),
            EditorDiagnosticMaxSeverity::All,
        )
        .is_none());
    }

    #[test]
    fn diagnostic_status_counts_only_errors_and_warnings() {
        use labonair_editor::language_services::DiagnosticSeverity as Severity;

        let diagnostics = [
            Severity::Error,
            Severity::Warning,
            Severity::Information,
            Severity::Hint,
        ]
        .into_iter()
        .map(|severity| labonair_editor::Diagnostic {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 0,
                },
                end: LspPosition {
                    line: 0,
                    character: 1,
                },
            },
            severity,
            message: "message".into(),
            source: labonair_editor::LanguageServerId::new("test-lsp"),
            code: None,
        })
        .collect::<Vec<_>>();

        assert_eq!(diagnostic_error_warning_counts(&diagnostics), (1, 1));
    }

    #[test]
    fn diagnostic_menu_labels_use_editor_columns_and_include_codes() {
        use labonair_editor::language_services::DiagnosticSeverity as Severity;

        let snapshot = labonair_editor::EditorBuffer::from_text("aλb").snapshot();
        let diagnostic = labonair_editor::Diagnostic {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 2,
                },
                end: LspPosition {
                    line: 0,
                    character: 3,
                },
            },
            severity: Severity::Error,
            message: "invalid token".into(),
            source: labonair_editor::LanguageServerId::new("test-lsp"),
            code: Some("E101".into()),
        };

        assert_eq!(
            diagnostic_menu_labels(&diagnostic, &snapshot),
            ("1:3 · Error".into(), "E101: invalid token".into())
        );
    }

    #[test]
    fn inlay_tooltip_lookup_uses_unicode_source_positions_and_visibility() {
        let snapshot = labonair_editor::EditorBuffer::from_text("aλb").snapshot();
        let hints = vec![InlayHint {
            position: LspPosition {
                line: 0,
                character: 2,
            },
            label: ": Result".into(),
            kind: InlayHintKind::Type,
            tooltip: Some("Inferred result type".into()),
            padding_left: true,
            padding_right: false,
        }];

        assert_eq!(
            inlay_hint_tooltip_at_position(&hints, &snapshot, Position::new(0, 2), |kind| kind
                == InlayHintKind::Type,),
            Some("Inferred result type")
        );
        assert_eq!(
            inlay_hint_tooltip_at_position(&hints, &snapshot, Position::new(0, 2), |_| false,),
            None
        );
    }

    #[test]
    fn document_link_hit_testing_uses_a_half_open_source_range() {
        let snapshot = labonair_editor::EditorBuffer::from_text("open here").snapshot();
        let links = vec![DocumentLink {
            range: LspRange {
                start: LspPosition {
                    line: 0,
                    character: 5,
                },
                end: LspPosition {
                    line: 0,
                    character: 9,
                },
            },
            target: Some("https://example.com".into()),
            tooltip: None,
            data: None,
        }];

        assert!(document_link_at_position(&links, &snapshot, Position::new(0, 5)).is_some());
        assert!(document_link_at_position(&links, &snapshot, Position::new(0, 8)).is_some());
        assert!(document_link_at_position(&links, &snapshot, Position::new(0, 9)).is_none());
    }

    #[test]
    fn error_lens_selects_the_highest_visible_diagnostic_for_a_line() {
        use labonair_editor::language_services::DiagnosticSeverity as Severity;

        let make = |line, severity, message: &str| labonair_editor::Diagnostic {
            range: LspRange {
                start: LspPosition { line, character: 0 },
                end: LspPosition { line, character: 1 },
            },
            severity,
            message: message.into(),
            source: labonair_editor::LanguageServerId::new("test-lsp"),
            code: None,
        };
        let diagnostics = vec![
            make(0, Severity::Warning, "  review   this value "),
            make(0, Severity::Error, "invalid value"),
            make(1, Severity::Hint, "a hint on another line"),
        ];

        assert_eq!(
            error_lens_for_line(&diagnostics, 0, EditorDiagnosticMaxSeverity::All),
            Some((Severity::Error, "invalid value".into()))
        );
        assert_eq!(
            error_lens_for_line(&diagnostics, 0, EditorDiagnosticMaxSeverity::Warning),
            Some((Severity::Error, "invalid value".into()))
        );
        assert_eq!(
            error_lens_for_line(&diagnostics, 1, EditorDiagnosticMaxSeverity::Warning),
            None
        );
        assert_eq!(
            error_lens_for_line(&diagnostics, 0, EditorDiagnosticMaxSeverity::Error),
            Some((Severity::Error, "invalid value".into()))
        );
    }

    #[gpui::test]
    fn diagnostic_messages_appear_when_hovering_the_source_range(cx: &mut TestAppContext) {
        use labonair_editor::language_services::DiagnosticSeverity;

        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.rs".into(), "abc", 1);
                this.metrics = (10.0, 18.0);
                this.gutter_width = 0.0;
                this.bounds = Some(gpui::Bounds {
                    origin: gpui::Point::default(),
                    size: gpui::Size {
                        width: px(100.0),
                        height: px(40.0),
                    },
                });
                let version = DocumentVersion::from_revision(this.editor_revision());
                let generation = this.language_service_generation;
                this.language_service_state
                    .set_document_generation(version, generation);
                assert!(this.language_service_state.apply(LanguageServiceResult {
                    server: labonair_editor::LanguageServerId::new("test-lsp"),
                    version,
                    generation,
                    response: Ok(LanguageServiceResponse::Diagnostics(vec![
                        labonair_editor::Diagnostic {
                            range: LspRange {
                                start: LspPosition {
                                    line: 0,
                                    character: 1,
                                },
                                end: LspPosition {
                                    line: 0,
                                    character: 2,
                                },
                            },
                            severity: DiagnosticSeverity::Warning,
                            message: "suspicious value".into(),
                            source: labonair_editor::LanguageServerId::new("test-lsp"),
                            code: Some("W17".into()),
                        },
                    ])),
                }));

                this.update_hover(gpui::point(px(5.0), px(0.0)), cx);
                assert_eq!(
                    this.hover.as_ref().map(|(_, text)| text.as_ref()),
                    Some("Warning (W17): suspicious value\ntest-lsp")
                );
            });
        });
    }

    #[test]
    fn minimap_slots_cover_lines_and_reject_invalid_coordinates() {
        assert_eq!(minimap_slot(0, 100, 10), Some(0));
        assert_eq!(minimap_slot(9, 100, 10), Some(0));
        assert_eq!(minimap_slot(10, 100, 10), Some(1));
        assert_eq!(minimap_slot(99, 100, 10), Some(9));
        assert_eq!(minimap_slot(100, 100, 10), None);
        assert_eq!(minimap_slot(0, 0, 10), None);
        assert_eq!(minimap_slot(0, 1, 0), None);

        let mut markers = vec![MinimapLineMarkers::default(); 10];
        mark_minimap_line_range(&mut markers, 100, 15, 35, |marker| {
            marker.search = true;
        });
        assert!(markers[1..=3].iter().all(|marker| marker.search));
        assert!(!markers[0].search);
        assert!(!markers[4].search);
    }

    #[test]
    fn minimap_pointer_scroll_is_centered_and_clamped() {
        assert_eq!(minimap_scroll_from_pointer(50.0, 0.0, 100.0, 100, 10), 45);
        assert_eq!(minimap_scroll_from_pointer(-10.0, 0.0, 100.0, 100, 10), 0);
        assert_eq!(minimap_scroll_from_pointer(110.0, 0.0, 100.0, 100, 10), 90);
        assert_eq!(minimap_scroll_from_pointer(50.0, 0.0, 100.0, 100, 100), 0);
        assert_eq!(minimap_scroll_from_pointer(50.0, 0.0, 0.0, 100, 10), 0);
    }

    #[test]
    fn minimap_visibility_and_width_follow_display_settings() {
        assert!(!minimap_policy_visible(
            EditorMinimapVisibility::Never,
            EditorMinimapDisplayIn::All,
            true,
            true
        ));
        assert!(!minimap_policy_visible(
            EditorMinimapVisibility::Auto,
            EditorMinimapDisplayIn::All,
            false,
            false
        ));
        assert!(minimap_policy_visible(
            EditorMinimapVisibility::Auto,
            EditorMinimapDisplayIn::All,
            false,
            true
        ));
        assert!(!minimap_policy_visible(
            EditorMinimapVisibility::Always,
            EditorMinimapDisplayIn::ActiveEditor,
            false,
            false
        ));
        assert!(minimap_policy_visible(
            EditorMinimapVisibility::Always,
            EditorMinimapDisplayIn::ActiveEditor,
            true,
            false
        ));
        assert_eq!(minimap_width_pixels(80), 240.0);
        assert_eq!(minimap_width_pixels(0), 76.0);
        assert_eq!(minimap_width_pixels(u32::MAX), 320.0);
    }

    #[test]
    fn scrollbar_marker_offsets_follow_visual_rows_and_stay_on_track() {
        assert_eq!(scrollbar_marker_offset(0, 100, 200.0), Some(0.0));
        assert_eq!(scrollbar_marker_offset(99, 100, 200.0), Some(198.0));
        assert_eq!(scrollbar_marker_offset(100, 100, 200.0), Some(198.0));
        assert_eq!(scrollbar_marker_offset(0, 0, 200.0), None);
        assert_eq!(scrollbar_marker_offset(0, 1, 0.0), None);
        assert_eq!(scrollbar_marker_offset(8, 10, 1.0), Some(0.0));
    }

    #[test]
    fn scrollbar_occurrence_markers_use_word_boundaries_and_pixel_density() {
        let buffer =
            labonair_editor::EditorBuffer::from_text("hit\nhitless\nx\nx\nx\nx\nx\nx\nx\nhit");
        let snapshot = buffer.snapshot();
        let display = DisplayMap::build(
            &snapshot,
            Viewport {
                height_rows: 10,
                ..Default::default()
            },
            DisplayConfig::default(),
            &[],
        );
        assert_eq!(
            find_scrollbar_marker_offsets(&snapshot, &display, 5.0, "hit", false),
            vec![0.0, 0.5, 3.0]
        );
        assert_eq!(
            find_scrollbar_marker_offsets(&snapshot, &display, 5.0, "hit", true),
            vec![0.0, 3.0]
        );
    }

    #[test]
    fn centered_search_scroll_line_centers_matches_and_clamps_at_document_edges() {
        let text = (0..40)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let snapshot = labonair_editor::EditorBuffer::from_text(&text).snapshot();
        let layout = DisplayMap::build(
            &snapshot,
            Viewport {
                height_rows: snapshot.byte_len() + 1,
                ..Default::default()
            },
            DisplayConfig::default(),
            &[],
        );

        assert_eq!(
            centered_search_scroll_line(&layout, Position::new(20, 0), 10),
            Some(15)
        );
        assert_eq!(
            centered_search_scroll_line(&layout, Position::new(2, 0), 10),
            Some(0)
        );
        assert_eq!(
            centered_search_scroll_line(&layout, Position::new(39, 0), 10),
            Some(30)
        );
    }

    #[test]
    fn minimap_occurrence_slots_collapse_matches_to_visible_marker_rows() {
        let buffer =
            labonair_editor::EditorBuffer::from_text("hit\nhitless\nx\nx\nx\nx\nx\nx\nx\nhit");
        let snapshot = buffer.snapshot();
        assert_eq!(
            find_minimap_occurrence_slots(&snapshot, 10, 10, "hit", false),
            vec![0, 1, 9]
        );
        assert_eq!(
            find_minimap_occurrence_slots(&snapshot, 10, 10, "hit", true),
            vec![0, 9]
        );
    }

    #[test]
    fn selected_symbol_lookup_handles_unicode_columns_and_word_edges() {
        let snapshot = labonair_editor::EditorBuffer::from_text("λfoo bar\n ").snapshot();
        assert_eq!(
            symbol_at_position(&snapshot, Position::new(0, 1)).as_deref(),
            Some("λfoo")
        );
        assert_eq!(
            symbol_at_position(&snapshot, Position::new(0, 4)).as_deref(),
            Some("λfoo")
        );
        assert_eq!(symbol_at_position(&snapshot, Position::new(1, 0)), None);
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
    fn fold_levels_follow_symbol_nesting_and_cursor_lookup_prefers_inner_ranges() {
        let outer = FoldRange::new(0, 10).expect("outer fold");
        let middle = FoldRange::new(2, 8).expect("middle fold");
        let inner = FoldRange::new(4, 5).expect("inner fold");
        let sibling = FoldRange::new(12, 14).expect("sibling fold");
        let candidates = vec![outer, middle, inner, sibling];

        assert_eq!(fold_ranges_at_level(&candidates, 1), vec![outer, sibling]);
        assert_eq!(fold_ranges_at_level(&candidates, 2), vec![middle]);
        assert_eq!(fold_ranges_at_level(&candidates, 3), vec![inner]);
        assert_eq!(fold_range_for_cursor(&candidates, 4), Some(inner));
        assert_eq!(
            fold_ranges_within(&candidates, outer),
            vec![outer, middle, inner]
        );
    }

    #[test]
    fn lsp_fold_ranges_are_validated_and_fall_back_to_symbols() {
        let snapshot = labonair_editor::EditorBuffer::from_text("abc\ndef\nghi").snapshot();
        let server_ranges = fold_ranges_from_lsp(
            &[
                FoldingRange {
                    start_line: 1,
                    end_line: 2,
                    kind: Some("region".into()),
                },
                FoldingRange {
                    start_line: 1,
                    end_line: 2,
                    kind: None,
                },
                FoldingRange {
                    start_line: 2,
                    end_line: 1,
                    kind: None,
                },
                FoldingRange {
                    start_line: 0,
                    end_line: 3,
                    kind: None,
                },
            ],
            &snapshot,
        );
        let server_range = FoldRange::new(1, 2).expect("valid server range");
        assert_eq!(server_ranges, vec![server_range]);

        let symbol_range = FoldRange::new(0, 2).expect("valid symbol range");
        let symbols = vec![labonair_editor::DocumentSymbol {
            name: "section".into(),
            kind: labonair_editor::symbols::SymbolKind::Function,
            line: 0,
            range: TextRange::new(0, 8),
            name_range: TextRange::new(0, 3),
        }];
        let syntax_range = FoldRange::new(1, 2).expect("valid syntax range");
        assert_eq!(
            fold_candidates(&[], &symbols, &[syntax_range], &snapshot),
            vec![symbol_range, syntax_range]
        );
        assert_eq!(
            fold_candidates(&server_ranges, &symbols, &[syntax_range], &snapshot),
            vec![server_range]
        );
    }

    #[gpui::test]
    fn lsp_folding_response_updates_editor_fold_candidates(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.doc = Document::from_file("test.rs".into(), "a\nb\nc", 1);
                this.folding_ranges_request_serial = Some(9);
                this.apply_language_service_response(
                    Ok(LanguageServiceResponse::FoldingRanges(vec![FoldingRange {
                        start_line: 0,
                        end_line: 2,
                        kind: Some("region".into()),
                    }])),
                    LanguageServiceRequestSlot::FoldingRanges,
                    cx,
                );
                assert_eq!(
                    this.server_fold_ranges,
                    vec![FoldRange::new(0, 2).expect("fold range")]
                );
                assert!(this.is_current_language_service_request(
                    LanguageServiceRequestSlot::FoldingRanges,
                    9,
                    None,
                ));
                this.folding_ranges_request_serial = Some(10);
                assert!(!this.is_current_language_service_request(
                    LanguageServiceRequestSlot::FoldingRanges,
                    9,
                    None,
                ));
            });
        });
    }

    #[gpui::test]
    fn unfold_all_command_clears_editor_owned_fold_state(cx: &mut TestAppContext) {
        let view = setup(cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.folds = vec![FoldRange::new(1, 3).expect("fold")];
                assert!(this.dispatch_registered_command(CommandId::EditorUnfoldAll, cx));
                assert!(this.folds.is_empty());
            });
        });
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

    #[test]
    fn git_blame_status_uses_author_and_a_compact_summary() {
        let blame = GitBlameSnapshot {
            path: "/repo/src/lib.rs".into(),
            line: 3,
            revision: Revision::new(7),
            commit: "0123456789abcdef".into(),
            author: "Ada Lovelace".into(),
            summary: "Improve parser error handling".into(),
            author_time: 1_740_000_000,
        };
        assert_eq!(
            git_blame_status_label(&blame),
            "Ada Lovelace · Improve parser error handling"
        );

        let long = GitBlameSnapshot {
            summary: "A very long commit subject that should not take over the editor status row"
                .into(),
            ..blame
        };
        assert!(git_blame_status_label(&long).ends_with('…'));
    }

    #[test]
    fn git_blame_age_uses_compact_relative_units() {
        assert_eq!(git_blame_age_label(1_000, 1_000), "just now");
        assert_eq!(git_blame_age_label(939, 1_000), "1 min ago");
        assert_eq!(git_blame_age_label(0, 90_000), "1 days ago");
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
