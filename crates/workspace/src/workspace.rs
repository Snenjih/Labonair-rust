//! Workspace: tab bar + split-pane content (T04-001, T04-002).
//!
//! [`Workspace`] owns the [`TabStore`], the shared [`TerminalRegistry`], the
//! per-workspace-tab [`WorkspaceLayout`] (its split-pane tree) and the content
//! view for every open pane. It renders the tab bar over the active tab's
//! split-pane tree; the window chrome around it (header, sidebar, status bar,
//! native titlebar) is composed by `AppShell` (in `labonair-ui`) (T04-003),
//! which queries this view for the data it surfaces (active cwd, tab label,
//! pane count) and forwards the header's inline search here.
//!
//! Panes / tabs / sessions are three distinct things, mirroring the reference:
//! a *session* is a PTY that lives in the [`TerminalRegistry`] and never pauses;
//! a *pane* is one slot in a workspace tab's [`WorkspaceLayout`] tree, bound to
//! one session; a *tab* selects which pane tree is on screen. Splitting a pane
//! (`Cmd-D` / `Cmd-Shift-D`) spawns a new session in the active pane's cwd;
//! closing the last pane of a tab closes the tab; closing any tab tears down
//! every session in its layout so no shell is orphaned.
//!
//! Crate root (`labonair-workspace`, extracted in T16-006). Besides [`Workspace`]
//! it hosts the pane tree, session snapshot/replay, the tab store, the AI live
//! bridge, the per-workspace stores (`agent_access`, `background`) and every
//! workspace-owned tab-content view under [`views`]. Transfer state and UI are
//! owned by `labonair-transfers` / `labonair-transfers-ui`; Workspace only
//! forwards transfer requests from SFTP views to the injected service.

pub mod agent_access;
pub mod bell;
pub mod command_provider;
pub mod context;
pub mod cwd_breadcrumb;
pub mod cwd_status_item;
pub mod dock;
pub mod dock_reveal;
pub mod dock_status_item;
pub mod editor_git_bridge;
pub mod file_finder;
pub mod layout;
pub mod live_bridge;
pub mod markdown;
pub mod mcp_event_bridge;
pub mod menu_slot;
pub mod modal_layer;
pub mod pane;
pub mod pane_actions;
pub(crate) mod pane_dnd;
pub(crate) mod pane_drop;
pub mod pane_group;
pub mod pane_group_naming;
pub mod pane_items;
pub mod pane_kind;
pub(crate) mod pane_menu;
pub mod pane_ops;
pub mod pane_view;
pub mod project_multibuffer;
pub mod project_search;
pub mod recent_projects;
pub mod search_overlay;
pub mod session;
pub mod session_io;
pub mod session_v3;
pub mod spaces;
pub mod spaces_ui;
pub mod ssh_connection;
pub mod ssh_event_bridge;
pub mod status_bar;
pub mod status_items;
pub mod status_placements;
pub(crate) mod tab_hover;
pub mod tab_strip;
pub mod tabs;
pub mod tabs_panel;
pub mod tiling;
pub mod views;
pub mod zoom;

pub use pane_view::{DraggedPane, PaneDragChip};

/// Re-export shim so `crate::theme::…` paths in the moved modules keep
/// resolving. The runtime theme store lives in `labonair_theme::store`
/// (relocated in T16-006).
pub mod theme {
    pub use labonair_theme::store::*;
}

gpui::actions!(
    labonair,
    [
        // "Ask about Selection" — dispatched from the terminal view's selection
        // context menu; handled by `menu` / `AppShell` in `crates/ui`. Defined
        // here (not in `labonair_ui::menu`) so `views::terminal` can name it
        // without a dependency cycle (T16-006).
        AskAboutSelection,
    ]
);

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::agent_access::AgentAccessStore;
use crate::mcp_event_bridge::McpEventBridge;
use crate::ssh_event_bridge::SshEventBridge;
use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, Context, DragMoveEvent, Entity, EventEmitter,
    ExternalPaths, FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Task, Window,
};
use labonair_command_palette_core::CommandId;
use labonair_git::{GitGraphService, GitService};
use labonair_mcp_core::{
    McpEvent, McpEventSource, McpSessionAccessService, McpTabOperationService, SessionGrantRequest,
    SessionKind, TabOpResult,
};
use labonair_sftp::{SftpBrowserService, SftpSessionService};
use labonair_ssh::{
    SshConnectRequest, SshConnectionEvent, SshConnectionService, SshEventSink, SshEventSource,
    SshPtyService, SshRemoteCommandService, SshRemoteFileService, SshSessionEvent, SshSessionId,
    SshTunnelService,
};
use labonair_terminal::{
    RemoteFeed, RemoteResizer, RemoteWriter, SessionHandle, SessionId, SessionOptions,
    TermDimensions, TerminalColors, TerminalRegistry,
};
use labonair_transfers::{TransferDirection, TransferRequest, TransferService};
use tokio::runtime::Handle as TokioHandle;

use crate::dock::{position_slug, RESIZE_HANDLE_SIZE};
use crate::editor_git_bridge::{EditorGitContext, WorkspaceGitBridge};
use crate::live_bridge::LiveCommand;
use crate::menu_slot::MenuSlot;
use crate::pane::{PaneId, SplitDirection, WorkspaceLayout};
use crate::project_search::root_safe_path;
use crate::spaces::{SpaceId, SpaceStore, DEFAULT_SPACE_ID};
use crate::ssh_connection::{
    ConnStage, ConnectionKind, ConnectionState, ConnectionStatusStore, StageStatus,
};
use crate::tabs::{Tab, TabData, TabDiagnosticStatus, TabGitStatus, TabKind, TabStore};
use crate::theme::ThemeStore;
use crate::views::preview::{PreviewImageAction, PreviewView};
use crate::views::sftp::{SftpEvent, SftpView};
use crate::views::terminal::TerminalView;
use labonair_background_host::BackgroundHost;
use labonair_editor::{
    FileFinderCandidate, FileFinderQuery, FileFinderResult, GitGutterSnapshot, GitGutterState,
    GitHunkStatus, LanguageServiceRuntime, MultiBuffer, MultiBufferSaveOutcome,
    MultiBufferSaveRequest, Position, ProjectDiagnosticRegistry, ProjectReplaceResult,
    ProjectSearchHit, ProjectSearchQuery, ProjectSearchSnapshot, Revision, SearchError,
    SearchQuery, TextRange,
};
use labonair_editor_ui::{EditorBufferSnapshot, EditorEvent, EditorSidePreviewContent, EditorView};
use labonair_hosts::{HostOpenMode, HostOpenRequest, HostPickerRow};
use labonair_hosts_host::{ActiveTunnelRow, HostStatus, HostView};
use labonair_hosts_ui::HostManagerView;
use labonair_keymap_ui::KeymapManagementView;
use labonair_panel_git_graph::GitGraphView;
use labonair_settings::content::general::StartupTab;
use labonair_settings::content::terminal::CursorStyle as PrefCursorStyle;
use labonair_settings::{ConnectionsSettings, GeneralSettings, Settings as _, TerminalSettings};
use labonair_ui_kit::{
    h_stack, indicator, BlinkCursor, ButtonSize, ButtonVariant, IndicatorSize, Palette,
};

fn editor_tab_git_status(
    snapshot: &GitGutterSnapshot,
    revision: Revision,
    path: &str,
) -> Option<TabGitStatus> {
    if snapshot.revision != revision
        || snapshot.path != path
        || !matches!(
            snapshot.state,
            GitGutterState::Ready | GitGutterState::Empty
        )
        || snapshot.hunks.is_empty()
    {
        return None;
    }

    let mut added = false;
    let mut modified = false;
    let mut deleted = false;
    for hunk in &snapshot.hunks {
        match hunk.status {
            GitHunkStatus::Added | GitHunkStatus::Untracked => added = true,
            GitHunkStatus::Modified => modified = true,
            GitHunkStatus::Deleted => deleted = true,
        }
    }

    if modified || (added && deleted) {
        Some(TabGitStatus::Modified)
    } else if deleted {
        Some(TabGitStatus::Deleted)
    } else if added {
        Some(TabGitStatus::Added)
    } else {
        None
    }
}

fn editor_tab_diagnostic_status(
    diagnostics: &[labonair_editor::Diagnostic],
) -> Option<TabDiagnosticStatus> {
    use labonair_editor::language_services::DiagnosticSeverity;

    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        Some(TabDiagnosticStatus::Error)
    } else if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    {
        Some(TabDiagnosticStatus::Warning)
    } else {
        None
    }
}

fn relative_path_under_root(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(relative.to_string_lossy().into_owned())
}

fn decode_local_file_uri(uri: &str) -> Result<std::path::PathBuf, String> {
    let encoded_path = uri
        .strip_prefix("file://")
        .ok_or_else(|| "Language server returned a non-file URI.".to_string())?;
    let local_path = if encoded_path.starts_with('/') {
        encoded_path.to_string()
    } else if let Some(path) = encoded_path.strip_prefix("localhost/") {
        format!("/{path}")
    } else {
        return Err("Language server returned a non-local file URI.".to_string());
    };
    if local_path.contains('?') || local_path.contains('#') {
        return Err("Language server returned an invalid file URI.".to_string());
    }

    fn hex_digit(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    let bytes = local_path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes
                .get(index + 1)
                .and_then(|byte| hex_digit(*byte))
                .ok_or_else(|| "Language server returned a malformed file URI.".to_string())?;
            let low = bytes
                .get(index + 2)
                .and_then(|byte| hex_digit(*byte))
                .ok_or_else(|| "Language server returned a malformed file URI.".to_string())?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let path = String::from_utf8(decoded)
        .map_err(|_| "Language server returned a non-UTF-8 file path.".to_string())?;
    if path.contains('\0') {
        return Err("Language server returned an invalid file path.".to_string());
    }
    Ok(std::path::PathBuf::from(path))
}

fn is_supported_external_document_link(uri: &str) -> bool {
    let Some((scheme, rest)) = uri.split_once(':') else {
        return false;
    };
    if uri.chars().any(char::is_whitespace) {
        return false;
    }
    if scheme.eq_ignore_ascii_case("mailto") {
        return !rest.is_empty();
    }
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        return rest
            .strip_prefix("//")
            .and_then(|authority| authority.split(['/', '?', '#']).next())
            .is_some_and(|authority| !authority.is_empty());
    }
    false
}

/// Interval for draining backend SSH events into the workspace.
const SSH_POLL_INTERVAL: Duration = Duration::from_millis(40);

/// Which surface the `Cmd+F` search overlay (T18-002) drives for the active tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTarget {
    /// A code editor tab — literal in-buffer search.
    Editor,
    /// A terminal pane — literal scrollback search.
    Terminal,
    /// SFTP list, git graph, host manager, … — search is not offered.
    Unavailable,
}

/// Outcome of resolving a command through the active-tab editor bridge.
/// Editor-specific unsupported operations are reported by EditorView through
/// the canonical notification center; this result covers the composition
/// boundary itself without duplicating editor state in Workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorCommandDispatchResult {
    Forwarded,
    NoActiveEditor,
    Unsupported(CommandId),
}

/// Build a working-tree style [`ProjectDiffRequest`](labonair_panel::ProjectDiffRequest)
/// for `source`. A `target` (path under or relative to the repo) is guaranteed
/// to be in the file list and becomes the initial selection.
async fn working_tree_diff_request(
    git: Arc<dyn GitService>,
    root: String,
    session: Option<String>,
    source: labonair_panel::DiffSource,
    target: Option<String>,
) -> Result<labonair_panel::ProjectDiffRequest, String> {
    let repo_root = git.repo_root(root, session.clone()).await?;
    let state = git
        .workspace_state(repo_root.clone(), session.clone())
        .await?;
    let mut files = editor_git_files(&state);
    match source {
        labonair_panel::DiffSource::Staged => files.retain(|file| file.staged),
        labonair_panel::DiffSource::Unstaged => files.retain(|file| !file.staged),
        _ => {}
    }
    let selected = target.map(|target| {
        let selected = crate::editor_git_bridge::relative_path(&repo_root, &target);
        if !files.iter().any(|file| file.path == selected) {
            files.push(labonair_panel::ProjectDiffFile {
                path: selected.clone(),
                staged: false,
                untracked: false,
            });
        }
        selected
    });
    Ok(labonair_panel::ProjectDiffRequest {
        repo_root,
        session_id: session,
        source,
        files,
        selected,
        mode: labonair_panel::ProjectDiffMode::Unified,
    })
}

fn editor_git_files(
    state: &labonair_git::WorkspaceGitState,
) -> Vec<labonair_panel::ProjectDiffFile> {
    let mut files = Vec::new();
    files.extend(
        state
            .status
            .staged
            .iter()
            .map(|file| labonair_panel::ProjectDiffFile {
                path: file.path.clone(),
                staged: true,
                untracked: false,
            }),
    );
    files.extend(
        state
            .status
            .unstaged
            .iter()
            .map(|file| labonair_panel::ProjectDiffFile {
                path: file.path.clone(),
                staged: false,
                untracked: false,
            }),
    );
    files.extend(
        state
            .status
            .untracked
            .iter()
            .map(|file| labonair_panel::ProjectDiffFile {
                path: file.path.clone(),
                staged: false,
                untracked: true,
            }),
    );
    files
}

/// One open SSH terminal tab: its backend session id, the host it targets and
/// the [`RemoteFeed`] used to push remote output / signal disconnects.
pub(crate) struct SshTab {
    pub(crate) ssh_id: String,
    pub(crate) host_id: String,
    pub(crate) feed: RemoteFeed,
    pub(crate) tab_id: u64,
    /// The terminal pane showing this session.
    pub(crate) pane_id: PaneId,
}

struct WorkspaceSshEventSink(RemoteFeed);

impl SshEventSink for WorkspaceSshEventSink {
    fn send(&self, event: SshSessionEvent) -> Result<(), String> {
        match event {
            SshSessionEvent::Output { data } => {
                self.0.feed(data.as_bytes());
                Ok(())
            }
        }
    }
}

/// A tab-lifecycle request from the MCP bridge (`labonair-mcp-server`), queued
/// off the event bus and drained in `render` where a `&mut Window` is available
/// — the bridge itself cannot touch tab state (tabs are pure UI), so it emits a
/// request event and waits on a `oneshot` for [`mcp_tab_op_response`].
/// `(session_id, label, kind, host_id)` for a tab that can be
/// granted MCP agent access — see [`Workspace::mcp_grant_target`].
type McpGrantTarget = (String, String, SessionKind, Option<String>);

enum McpTabOp {
    Open {
        request_id: String,
        host_id: String,
    },
    Close {
        request_id: String,
        session_id: String,
    },
}

/// Title for an SSH terminal tab, annotated with the bastion when the
/// connection is routed through a jump host (task T07-002, item 5).
fn ssh_tab_title(host_label: &str, jump_label: Option<&str>) -> String {
    match jump_label {
        Some(j) => format!("SSH \u{00b7} {host_label}  \u{2192} {j}"),
        None => format!("SSH \u{00b7} {host_label}"),
    }
}

/// Small action button for the SSH loading screen: the shared `button()`
/// primitive — `Action` for the primary choice, `Outlined` otherwise. Callers
/// add `.on_click(..)`.
fn loading_btn(
    id: &'static str,
    label: &'static str,
    c: Palette,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if primary {
        ButtonVariant::Action
    } else {
        ButtonVariant::Outlined
    };
    labonair_ui_kit::button(id, c, variant, ButtonSize::Xs).child(label)
}

/// A blocking prompt raised mid-connect by the SSH backend.
enum SshPrompt {
    Trust { ssh_id: String },
    Password { ssh_id: String, buffer: String },
    Passphrase { ssh_id: String, buffer: String },
}

impl SshPrompt {
    fn ssh_id(&self) -> &str {
        match self {
            SshPrompt::Trust { ssh_id, .. }
            | SshPrompt::Password { ssh_id, .. }
            | SshPrompt::Passphrase { ssh_id, .. } => ssh_id,
        }
    }
}

/// Interval for syncing terminal cwd/title into their tab labels.
const META_SYNC_INTERVAL: Duration = Duration::from_millis(400);

/// How often the session snapshot is re-written while the app runs (T14-001).
const SESSION_SAVE_INTERVAL: Duration = Duration::from_secs(30);

struct ScrollbackCapture {
    session_id: String,
    ansi: String,
    max_bytes: Option<usize>,
}

struct SessionCapture {
    snapshot: crate::session_v3::SpacesSnapshotV3,
    scrollbacks: Vec<ScrollbackCapture>,
}

fn persist_scrollback_captures(data_dir: &Path, captures: Vec<ScrollbackCapture>) {
    for capture in captures {
        let _ = labonair_terminal::scrollback::save(
            data_dir,
            &capture.session_id,
            &capture.ansi,
            capture.max_bytes,
        );
    }
}

/// The new-tab dropdown's two host submenus.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NewTabSubmenu {
    Ssh,
    Sftp,
}

/// The layered `terminal` settings slice, or its all-defaults value if the
/// `SettingsStore` global was never installed (headless test).
fn terminal_settings(cx: &App) -> TerminalSettings {
    TerminalSettings::try_get(cx).cloned().unwrap_or_else(|| {
        TerminalSettings::from_settings(&labonair_settings::SettingsContent::default())
    })
}

/// The layered `connections` settings slice, or its all-defaults value if the
/// `SettingsStore` global was never installed (headless test).
fn connections_settings(cx: &App) -> ConnectionsSettings {
    ConnectionsSettings::try_get(cx)
        .cloned()
        .unwrap_or_else(|| {
            ConnectionsSettings::from_settings(&labonair_settings::SettingsContent::default())
        })
}

/// The live-tunable emulator parameters derived from the `terminal` settings —
/// applied to new sessions at spawn and to open sessions on a settings change.
fn terminal_emulator_config(cx: &App) -> labonair_terminal::EmulatorConfig {
    let ts = terminal_settings(cx);
    labonair_terminal::EmulatorConfig {
        scrollback: ts.scrollback().max(1) as usize,
        cursor_shape: match ts.cursor_style() {
            PrefCursorStyle::Block => labonair_terminal::CursorShape::Block,
            PrefCursorStyle::Underline => labonair_terminal::CursorShape::Underline,
            PrefCursorStyle::Bar => labonair_terminal::CursorShape::Beam,
        },
        cursor_blink: ts.cursor_blink(),
        word_separators: ts.word_separator().to_string(),
    }
}

/// Value carried by a tab drag.
struct DraggedTab {
    id: u64,
    label: SharedString,
    icon: labonair_ui_kit::IconName,
}

/// Value carried while dragging a split divider: which axis, and which
/// boundary within it (between members `boundary` and `boundary + 1`).
struct PaneResize {
    axis_id: PaneId,
    boundary: usize,
}

/// Minimal drag preview for the resize handles (the cursor does the work).
struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorSidePreviewKind {
    Markdown,
    Svg,
    Tabular,
}

/// One pane's backing session + content view.
pub(crate) struct PaneEntry {
    pub(crate) session_id: SessionId,
    pub(crate) view: Entity<TerminalView>,
    /// Stable UUID keying this pane's persisted scrollback file (T14-002).
    pub(crate) scrollback_id: String,
}

/// A remote file staged into a local temp copy for editing (T08-001). Keyed
/// by the editor tab id; the temp file is uploaded back on save and removed
/// when the tab closes.
struct RemoteEdit {
    session_id: String,
    remote_path: String,
    temp_path: String,
    /// Last-seen dirty flag — a `true → false` transition means the editor
    /// just saved, so the temp copy is pushed back to the host.
    dirty: bool,
}

/// A file-open request queued by an [`SftpView`], drained in `render` where a
/// `&mut Window` is available.
enum PendingOpen {
    Local(String),
    RemoteEdit {
        session_id: String,
        remote_path: String,
        host_id: String,
        temp_path: String,
    },
    /// Open/focus an SSH terminal tab (SFTP remote pane's `>_ Term` action).
    SshTab(String),
    /// Open a new local terminal tab rooted at a directory (SFTP local
    /// pane's "Open Terminal Here" context action).
    LocalTerminal(String),
}

/// Events emitted by the workspace for actions owned by another surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceEvent {
    /// Ask the application shell to present the native project-folder picker,
    /// re-rooting the *current* space.
    OpenProject,
    /// Ask the application shell to present the native project-folder picker
    /// for a *new* space (T20-008 Spaces) — the current space's identity is
    /// left untouched.
    NewProjectSpace,
    /// Ask the application shell to open Settings on the Hosts page.
    OpenHosts,
    /// The DiffView cursor moved into this repo-relative file; the Source
    /// Control panel selects the matching entry.
    DiffCursorFile(String),
}

/// The tabbed, split-pane workspace shell.
pub struct Workspace {
    pub(crate) context: context::WorkspaceContext,
    pub(crate) registry: Arc<TerminalRegistry>,
    pub(crate) tabs: Entity<TabStore>,
    /// Named, switchable groups of tabs (T20-008 Spaces). Exactly one is
    /// visible in the tab strip at a time; the others' tabs are simply not
    /// the ones `TabStore` currently reports — their sessions/views stay
    /// alive in `layouts`/`panes`/`editors`/etc. unchanged.
    pub(crate) spaces: Entity<SpaceStore>,
    /// Anchor position of the open Spaces popover, if any (mirrors
    /// `new_tab_menu`).
    pub(crate) spaces_menu: MenuSlot<gpui::Point<gpui::Pixels>>,
    /// The per-space "..." actions menu: `(space id, anchor)`.
    pub(crate) space_actions_menu: MenuSlot<(SpaceId, gpui::Point<gpui::Pixels>)>,
    /// Which spaces are collapsed in the Spaces popover / Tabs sidebar.
    /// Shared by both presentations — one data model, two views.
    pub(crate) collapsed_spaces: std::collections::HashSet<SpaceId>,
    /// Space whose name is being edited inline: `(space id, buffer)`. Shares
    /// `rename_focus`/`rename_blink` with tab-rename — only one hand-rolled
    /// rename field is ever focused at a time (mutually exclusive with
    /// `rename_tab`), same invariant `caret`'s own doc comment documents.
    pub(crate) space_rename: Option<(SpaceId, String)>,
    /// MRU list of project folders opened via Spaces — independent of
    /// `spaces` itself so a closed Space's project is still one click away.
    pub(crate) recent_projects: recent_projects::RecentProjectsList,
    /// Whether the "Recent Projects" section is expanded in the Spaces
    /// popover / Tabs sidebar footer.
    pub(crate) recent_projects_open: bool,
    /// Anchor position of the Tabs sidebar's "New Space" dropdown (search +
    /// Recent Projects + New Space / Open Local Folder), if open.
    pub(crate) new_space_menu: MenuSlot<gpui::Point<gpui::Pixels>>,
    /// Filter text typed into the New Space dropdown's search field.
    pub(crate) new_space_search: String,
    pub(crate) new_space_search_focus: FocusHandle,
    pub(crate) new_space_search_blink: Entity<BlinkCursor>,
    pub(crate) theme: Entity<ThemeStore>,
    pub(crate) background: BackgroundHost,
    /// Split-pane tree per `Workspace` tab id — survives tab switches so the
    /// layout is never lost.
    pub(crate) layouts: HashMap<u64, WorkspaceLayout>,
    /// Content view + session per pane id (pane ids are process-unique).
    pub(crate) panes: HashMap<PaneId, PaneEntry>,
    /// Kind / title / dirty metadata of every pane (the pure half of the
    /// `PaneId`-keyed store; the views live in the maps below).
    pub(crate) pane_meta: crate::pane_kind::PaneMetaStore,
    /// Per-pane Explorer follow/pin override (runtime only; default comes from
    /// the space kind, see `context::resolve_explorer_root`).
    pub(crate) explorer_modes: HashMap<PaneId, context::ExplorerMode>,
    /// Last Explorer target a following pane reported, kept for panes that
    /// have no live cwd (editors).
    explorer_sticky: std::cell::RefCell<Option<context::ExplorerTarget>>,
    /// Editor panes that are not the first pane of their document.
    pub(crate) editor_aliases: HashMap<PaneId, crate::pane_items::EditorBinding>,
    /// Per-tab zoom overlay (runtime only).
    pub(crate) zoom: crate::zoom::ZoomState,
    /// The pane created by the last split: the only one that fades in (the
    /// split source must stay visible, it was fullscreen a frame ago).
    pub(crate) fresh_pane: Option<crate::pane::PaneId>,
    /// Drag-over state of tab / pane drags (drop zones, tab hints).
    pub(crate) pane_dnd: crate::pane_drop::PaneDnd,
    /// Hover list of a pane-group tab (one shared popover).
    pub(crate) tab_hover: crate::tab_hover::TabHover,
    /// Pane area of the active tab in window coordinates, measured during
    /// paint and read by split / focus / resize logic.
    pub(crate) pane_area: std::rc::Rc<std::cell::Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
    /// Editor view per `Editor` tab id.
    pub(crate) editors: HashMap<u64, Entity<EditorView>>,
    /// Editor tab whose Editor-owned project search session is currently shown.
    /// This stays pinned while opening results changes the active tab.
    pub(crate) project_search_owner: Option<u64>,
    /// The cancellable operation handle for the Editor-owned project search.
    /// Query/result/status state remains inside the search owner Editor entity.
    pub(crate) project_search_task: Option<Task<()>>,
    /// SFTP browser view per `Sftp` tab id (T08-001).
    pub(crate) sftp_views: HashMap<u64, Entity<SftpView>>,
    /// Native preview view per `Preview` tab id (T15-006 — WebView replacement).
    pub(crate) previews: HashMap<u64, Entity<PreviewView>>,
    /// Preview tabs that follow edits from their source editor tab.
    pub(crate) markdown_previews: HashMap<u64, u64>,
    /// SVG preview tabs that follow edits from their source editor tab.
    pub(crate) svg_previews: HashMap<u64, u64>,
    /// Preview pane opened beside an editor document (`Open Preview to the
    /// Side`), keyed by the editor slot; the view lives in `previews`.
    pub(crate) editor_side_previews: HashMap<u64, (PaneId, EditorSidePreviewKind)>,
    /// Single preview tab that follows whichever editor was last activated.
    pub(crate) following_markdown_preview: Option<u64>,
    pub(crate) following_markdown_editor: Option<u64>,
    /// Single SVG preview tab that follows whichever editor was last activated.
    pub(crate) following_svg_preview: Option<u64>,
    pub(crate) following_svg_editor: Option<u64>,
    /// The shared commit-graph view, lazily created for the `GitGraph` tab.
    pub(crate) git_graph: Option<Entity<GitGraphView>>,
    /// The keymap-management view, lazily created for the single `Keymap` tab.
    /// `labonair-keymap-ui` owns its rendering; the workspace owns its tab
    /// lifecycle (mirrors `git_graph`).
    pub(crate) keymap: Option<Entity<KeymapManagementView>>,
    /// The host-management view for the single `Hosts` tab. Unlike `keymap`,
    /// this is never `None`: the composition root constructs it eagerly
    /// because it also serves as the live host-data hub (tunnel status,
    /// picker rows) via `host_view`, independent of whether its tab is open.
    pub(crate) hosts: Entity<HostManagerView>,
    /// The single workspace Diff item, lazily created for the `Diff` tab.
    /// Source Control and the Git Graph emit a `ProjectDiffRequest`; this view
    /// is re-pointed (working tree ↔ commit) rather than duplicated.
    pub(crate) project_diff: Option<Entity<crate::views::project_diff::ProjectDiffView>>,
    /// Panel-type registry (T17-001) — populated once by
    /// `labonair_shell::register_builtin_panels`. The shell's dock rendering
    /// and status-bar toggles read it instead of a hard-coded `enum`. T17-002
    /// replaces the shell's ad-hoc per-side slot state with a real `Dock` that
    /// will also hang off the `Workspace`.
    pub(crate) panel_registry: labonair_panel::PanelRegistry,
    /// Status-bar item registry (T17-003) — populated once by
    /// `labonair_shell::register_builtin_status_items`. The `StatusBar`
    /// component reads it instead of a hard-coded `render_bar_item` match.
    pub(crate) status_item_registry: labonair_panel::StatusItemRegistry,
    /// Debounce for [`Workspace::persist_docks`].
    pub(crate) last_dock_save: Option<std::time::Instant>,
    /// Primary dock edge used when a panel has no current membership.
    pub(crate) primary_dock_position: labonair_panel::DockPosition,
    /// The three edge docks (T17-002). Empty at construction; populated by
    /// [`Workspace::init_docks`] once the shell has registered the builtin
    /// panels. Replaces the shell's former ad-hoc `left_slot`/`right_slot`.
    pub(crate) left_dock: crate::dock::Dock,
    pub(crate) right_dock: crate::dock::Dock,
    pub(crate) bottom_dock: crate::dock::Dock,
    /// Handle to the Tabs sidebar panel, kept so [`Workspace::sync_tabs_in_sidebar`]
    /// can add it back to a dock at runtime when `tabsLocation` flips to
    /// `"sidebar"`. Set once by [`Workspace::init_docks`].
    pub(crate) tabs_panel: Option<labonair_panel::AnyPanelHandle>,
    /// SFTP session id per `Sftp` tab id — kept alongside the view so the
    /// session can be torn down from `retire_tab` (which has no `cx`).
    pub(crate) sftp_sessions: HashMap<u64, String>,
    /// Active remote-edit temp copies, keyed by editor tab id.
    pub(crate) remote_edits: HashMap<u64, RemoteEdit>,
    /// SFTP tabs queued by the host manager, drained in `render`.
    pub(crate) pending_sftp: Vec<String>,
    /// File-open requests queued by SFTP views, drained in `render`.
    pub(crate) pending_open: Vec<PendingOpen>,
    pub(crate) pending_preview_focus: bool,
    /// Tab id whose close is awaiting unsaved-changes confirmation.
    pub(crate) confirm_close: Option<u64>,
    /// Remaining tabs in a confirmed close-left/right/others/all operation.
    pub(crate) pending_close: VecDeque<u64>,
    /// Open tab context menu: `(tab id, anchor position)`.
    pub(crate) context_menu: MenuSlot<(u64, gpui::Point<gpui::Pixels>)>,
    /// Anchor position of the open "+" new-tab dropdown, if any.
    pub(crate) new_tab_menu: MenuSlot<gpui::Point<gpui::Pixels>>,
    /// Strip scroll handles and the tab drag-over marker.
    pub(crate) tab_strip: crate::tab_strip::TabStripState,
    /// Open/close reveal state of the left, right and bottom docks.
    pub(crate) dock_reveal: [crate::dock_reveal::DockReveal; 3],
    /// Which of the new-tab dropdown's `SSH ▸` / `SFTP ▸` submenus is open.
    /// Held here (not as pure `:hover` state) so the two flyouts are mutually
    /// exclusive and can't overlap. Only meaningful while `new_tab_menu`.
    pub(crate) new_tab_submenu: Option<NewTabSubmenu>,
    /// Tab whose title is being edited inline: `(tab id, buffer)`.
    pub(crate) rename_tab: Option<(u64, String)>,
    pub(crate) rename_focus: FocusHandle,
    pub(crate) rename_blink: Entity<BlinkCursor>,
    pub(crate) _rename_blink_subs: Vec<Subscription>,
    pub(crate) focus_handle: FocusHandle,
    /// Cached active-terminal preview URL. Statusbar `is_empty` and render
    /// both read this value, so terminal grid-to-text conversion happens at
    /// most once per metadata refresh rather than twice per statusbar pass.
    pub(crate) preview_url_cache: Option<Option<String>>,
    pub(crate) _meta_sync: Task<()>,

    // ── SSH (T07-001) ──────────────────────────────────────────────────────
    pub(crate) ssh: Arc<dyn SshConnectionService>,
    pub(crate) ssh_pty: Arc<dyn SshPtyService>,
    pub(crate) ssh_remote: Arc<dyn SshRemoteCommandService>,
    pub(crate) ssh_remote_file: Arc<dyn SshRemoteFileService>,
    pub(crate) ssh_tunnels: Arc<dyn SshTunnelService>,
    pub(crate) sftp_session: Arc<dyn SftpSessionService>,
    pub(crate) sftp_browser: Arc<dyn SftpBrowserService>,
    pub(crate) git: Arc<dyn GitService>,
    /// Workspace-owned adapter for editor Git projections and typed hunk
    /// actions. Git remains the sole repository/index owner.
    pub(crate) editor_git_bridge: Arc<WorkspaceGitBridge>,
    /// Shared Editor-owned language-service composition runtime. Workspace
    /// stores only the injected handle; process/document state stays in the
    /// editor capability.
    pub(crate) editor_language_services: Arc<LanguageServiceRuntime>,
    /// Shared typed projection of diagnostics from the open Editor buffers.
    pub(crate) editor_diagnostics: HashMap<SpaceId, ProjectDiagnosticRegistry>,
    pub(crate) git_graph_service: Arc<dyn GitGraphService>,
    pub(crate) mcp_access: Arc<dyn McpSessionAccessService>,
    pub(crate) mcp_tab_operations: Arc<dyn McpTabOperationService>,
    pub(crate) tokio: TokioHandle,
    pub(crate) host_view: HostView,
    /// Live SSH terminal tabs, keyed by registry session id.
    pub(crate) ssh_tabs: HashMap<SessionId, SshTab>,
    /// Host ids queued by the host manager for connection, drained in `render`
    /// (where a `&mut Window` is available).
    pub(crate) pending_connect: Vec<String>,
    /// The active connect prompt (trust / password / passphrase), if any.
    pub(crate) ssh_prompt: Option<SshPrompt>,
    pub(crate) prompt_focus: FocusHandle,
    pub(crate) prompt_shown: bool,
    /// Per-session connection state machine + stage progress + live log,
    /// driving the full-pane `SshLoadingScreen` (T16-015).
    pub(crate) ssh_connection: Entity<ConnectionStatusStore>,
    /// Tab ids the loading screen asked to close (needs `&mut Window`).
    pub(crate) pending_tab_close: Vec<u64>,
    /// Injected SSH connection-event source → UI bridge (T17-008). The source
    /// owns transport translation; Workspace receives typed events only.
    pub(crate) _ssh_event_bridge: Entity<SshEventBridge>,
    /// Injected MCP event source → UI bridge. MCP requests and activity are
    /// delivered separately from the SSH connection lifecycle.
    pub(crate) _mcp_event_bridge: Entity<McpEventBridge>,
    /// Periodic tunnel-liveness refresh (state poll, not event-driven).
    pub(crate) _ssh_poll: Task<()>,
    /// Periodic session-snapshot writer (T14-001) — covers force-quit; the
    /// window-close hook in `AppShell` covers the normal path.
    pub(crate) _session_save: Task<()>,

    // ── SFTP transfer requests (R01-003) ──────────────────────────────────
    pub(crate) transfer_service: Arc<dyn TransferService>,

    // ── MCP bridge (T11-005) ──────────────────────────────────────────────
    /// Tab open/close requests from the MCP bridge, drained in `render`.
    pub(crate) pending_mcp: Vec<McpTabOp>,
    /// Client-side mirror of the bridge's per-tab agent-access grants
    /// (T11-006) — shared with the header badge in `AppShell`.
    pub(crate) agent_access: Entity<AgentAccessStore>,

    // ── Snippets (T12-001) ────────────────────────────────────────────────
    /// Snippet commands waiting to be typed into a freshly-opened SSH tab
    /// once its session is established, keyed by `ssh_id`.
    pub(crate) pending_snippet_ssh: HashMap<String, String>,

    // ── Project settings (T19-003) ──────────────────────────────────────
    /// Signature of the rejected project-settings keys already reported for
    /// the current explicit project transition. This is only notification
    /// deduplication; project settings are synchronized by transitions, never
    /// from the render path.
    pub(crate) last_project_settings_rejection_signature: Option<String>,
}

impl EventEmitter<WorkspaceEvent> for Workspace {}

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        registry: Arc<TerminalRegistry>,
        theme: Entity<ThemeStore>,
        background: BackgroundHost,
        ssh: Arc<dyn SshConnectionService>,
        ssh_pty: Arc<dyn SshPtyService>,
        ssh_remote: Arc<dyn SshRemoteCommandService>,
        ssh_remote_file: Arc<dyn SshRemoteFileService>,
        ssh_tunnels: Arc<dyn SshTunnelService>,
        sftp_session: Arc<dyn SftpSessionService>,
        sftp_browser: Arc<dyn SftpBrowserService>,
        git: Arc<dyn GitService>,
        git_graph_service: Arc<dyn GitGraphService>,
        ssh_event_source: Arc<dyn SshEventSource>,
        mcp_event_source: Arc<dyn McpEventSource>,
        mcp_access: Arc<dyn McpSessionAccessService>,
        mcp_tab_operations: Arc<dyn McpTabOperationService>,
        transfer_service: Arc<dyn TransferService>,
        tokio: TokioHandle,
        agent_access: Entity<AgentAccessStore>,
        host_view: HostView,
        hosts: Entity<HostManagerView>,
        restore: Option<crate::session_v3::SpacesSnapshotV3>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tabs = cx.new(|_| TabStore::new());
        let spaces = cx.new(|_| SpaceStore::new());
        cx.observe(&tabs, |_, _, cx| cx.notify()).detach();
        cx.observe(&spaces, |_, _, cx| cx.notify()).detach();
        if cx.has_global::<labonair_settings::SettingsStore>() {
            cx.observe_global::<labonair_settings::SettingsStore>(|_, cx| cx.notify())
                .detach();
        }
        cx.subscribe(&tabs, |this, _, ev: &crate::tabs::ActiveTabChanged, cx| {
            if let Some(editor) = this.editors.get(&this.editor_slot_for_tab(ev.0)).cloned() {
                editor.update(cx, |e, cx| e.check_external(cx));
                if this.following_markdown_preview.is_some() {
                    this.update_following_markdown_preview_for_editor(ev.0, cx);
                }
                if this.following_svg_preview.is_some() {
                    this.update_following_svg_preview_for_editor(ev.0, cx);
                }
            }
            // Keep the active space's "resume here" tab current live, so a
            // later `switch_space` away and back lands on the right tab
            // without a separate save step (T20-008 Spaces).
            let active_space = this.spaces.read(cx).active_id();
            let active_tab = this
                .tabs
                .read(cx)
                .get(ev.0)
                .filter(|tab| tab.space_id == active_space)
                .map(|tab| tab.id);
            this.spaces.update(cx, |s, _| {
                s.record_last_active_tab(active_space, active_tab);
            });
        })
        .detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(background.pulse(), |_, _, cx| cx.notify())
            .detach();

        let meta_sync = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(META_SYNC_INTERVAL).await;
            if this.update(cx, |this, cx| this.sync_meta(cx)).is_err() {
                break;
            }
        });

        // SSH connection events → UI (T17-008): one injected source and one
        // foreground bridge. No backend event bus reaches Workspace.
        let ssh_event_bridge = {
            let workspace = cx.entity().downgrade();
            cx.new(|cx| SshEventBridge::new(ssh_event_source, workspace, cx))
        };
        let mcp_event_bridge = {
            let workspace = cx.entity().downgrade();
            cx.new(|cx| McpEventBridge::new(mcp_event_source, workspace, cx))
        };

        let ssh_poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(SSH_POLL_INTERVAL).await;
            let ok = this
                .update(cx, |this, cx| this.refresh_active_tunnels(cx))
                .is_ok();
            if !ok {
                break;
            }
        });

        let session_save = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(SESSION_SAVE_INTERVAL).await;
            if this
                .update(cx, |this, cx| this.maybe_save_session(cx))
                .is_err()
            {
                break;
            }
        });

        let rename_focus = cx.focus_handle();
        let rename_blink = cx.new(|_| BlinkCursor::new());
        let mut _rename_blink_subs = vec![
            cx.observe(&rename_blink, |_, _, cx| cx.notify()),
            cx.on_focus(&rename_focus, window, |this, _w, cx| {
                this.rename_blink.update(cx, |b, cx| b.start(cx));
            }),
            cx.on_blur(&rename_focus, window, |this, _w, cx| {
                this.rename_blink.update(cx, |b, cx| b.stop(cx));
                // Losing focus (click-away, popover dismissed, space switched)
                // must end the rename the same way Enter would — otherwise the
                // edit box is left stuck in-place until the app restarts,
                // since nothing else ever clears `rename_tab`/`space_rename`.
                if this.rename_tab.is_some() {
                    this.commit_tab_rename(cx);
                }
                if this.space_rename.is_some() {
                    this.commit_space_rename(cx);
                }
            }),
        ];

        let new_space_search_focus = cx.focus_handle();
        let new_space_search_blink = cx.new(|_| BlinkCursor::new());
        _rename_blink_subs.extend([
            cx.observe(&new_space_search_blink, |_, _, cx| cx.notify()),
            cx.on_focus(&new_space_search_focus, window, |this, _w, cx| {
                this.new_space_search_blink.update(cx, |b, cx| b.start(cx));
            }),
            cx.on_blur(&new_space_search_focus, window, |this, _w, cx| {
                this.new_space_search_blink.update(cx, |b, cx| b.stop(cx));
            }),
        ]);

        let editor_git_bridge = Arc::new(WorkspaceGitBridge::new(git.clone(), tokio.clone()));
        let editor_language_services = Arc::new(LanguageServiceRuntime::default());
        editor_language_services.set_tokio_handle(tokio.clone());
        let editor_diagnostics = HashMap::new();
        let mut this = Self {
            registry,
            tabs,
            spaces,
            spaces_menu: MenuSlot::new(),
            space_actions_menu: MenuSlot::new(),
            collapsed_spaces: std::collections::HashSet::new(),
            space_rename: None,
            recent_projects: recent_projects::load(),
            recent_projects_open: false,
            new_space_menu: MenuSlot::new(),
            new_space_search: String::new(),
            new_space_search_focus,
            new_space_search_blink,
            theme,
            background,
            layouts: HashMap::new(),
            panes: HashMap::new(),
            editors: HashMap::new(),
            project_search_owner: None,
            project_search_task: None,
            sftp_views: HashMap::new(),
            previews: HashMap::new(),
            markdown_previews: HashMap::new(),
            svg_previews: HashMap::new(),
            editor_side_previews: HashMap::new(),
            pane_meta: crate::pane_kind::PaneMetaStore::new(),
            explorer_modes: HashMap::new(),
            explorer_sticky: std::cell::RefCell::new(None),
            editor_aliases: HashMap::new(),
            zoom: crate::zoom::ZoomState::default(),
            fresh_pane: None,
            pane_dnd: crate::pane_drop::PaneDnd::default(),
            tab_hover: crate::tab_hover::TabHover::default(),
            pane_area: std::rc::Rc::new(std::cell::Cell::new(None)),
            following_markdown_preview: None,
            following_markdown_editor: None,
            following_svg_preview: None,
            following_svg_editor: None,
            git_graph: None,
            keymap: None,
            hosts,
            project_diff: None,
            panel_registry: labonair_panel::PanelRegistry::new(),
            status_item_registry: labonair_panel::StatusItemRegistry::new(),
            context: context::WorkspaceContext::standalone(),
            last_dock_save: None,
            primary_dock_position: labonair_panel::DockPosition::Left,
            left_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Left),
            right_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Right),
            bottom_dock: crate::dock::Dock::new(labonair_panel::DockPosition::Bottom),
            tabs_panel: None,
            sftp_sessions: HashMap::new(),
            remote_edits: HashMap::new(),
            pending_sftp: Vec::new(),
            pending_open: Vec::new(),
            pending_preview_focus: false,
            confirm_close: None,
            pending_close: VecDeque::new(),
            context_menu: MenuSlot::new(),
            new_tab_menu: MenuSlot::new(),
            tab_strip: Default::default(),
            dock_reveal: Default::default(),
            new_tab_submenu: None,
            rename_tab: None,
            rename_focus,
            rename_blink,
            _rename_blink_subs,
            focus_handle: cx.focus_handle(),
            preview_url_cache: None,
            _meta_sync: meta_sync,
            _session_save: session_save,
            git,
            editor_git_bridge,
            editor_language_services,
            editor_diagnostics,
            git_graph_service,
            mcp_access,
            mcp_tab_operations,
            ssh,
            ssh_pty,
            ssh_remote,
            ssh_remote_file,
            ssh_tunnels,
            sftp_session,
            sftp_browser,
            tokio,
            host_view,
            ssh_tabs: HashMap::new(),
            pending_connect: Vec::new(),
            ssh_prompt: None,
            prompt_focus: cx.focus_handle(),
            prompt_shown: false,
            ssh_connection: cx.new(|_| ConnectionStatusStore::new()),
            pending_tab_close: Vec::new(),
            _ssh_event_bridge: ssh_event_bridge,
            _mcp_event_bridge: mcp_event_bridge,
            _ssh_poll: ssh_poll,
            transfer_service,
            pending_mcp: Vec::new(),
            agent_access,
            pending_snippet_ssh: HashMap::new(),
            last_project_settings_rejection_signature: None,
        };
        cx.observe(&this.agent_access, |_, _, cx| cx.notify())
            .detach();
        cx.observe(&this.ssh_connection, |_, _, cx| cx.notify())
            .detach();

        // Startup (T14-001 / T17-009 / T20-008 Spaces). A passed-in snapshot
        // means session restore is on *and* a snapshot was found — replay
        // every space it carried, even if a space restores zero tabs (an
        // empty space is a valid state). With no snapshot, honour the
        // `startup_tab` preference in the single seeded Default space:
        // `terminal` opens one local terminal, `empty` opens nothing. There
        // is no automatic host-manager tab any more.
        match restore {
            Some(snapshot) => {
                let mut space_ids = Vec::with_capacity(snapshot.spaces.len());
                for (index, entry) in snapshot.spaces.iter().enumerate() {
                    let id = this.spaces.update(cx, |s, cx| {
                        s.restore_entry(
                            index == 0,
                            entry.name.clone(),
                            entry.session.identity.clone(),
                            cx,
                        )
                    });
                    space_ids.push(id);
                    this.tabs.update(cx, |s, _| s.set_current_space(id));

                    let result = this.restore_session(&entry.session, window, cx);
                    for (title, reason) in &result.failed {
                        tracing::warn!(title, reason, "session tab not restored");
                    }

                    // Record this space's "resume here" tab now, while
                    // `current_space` is still pinned to it — the very next
                    // space's restore would otherwise overwrite
                    // `TabStore::active_id` before we get a chance to read it.
                    let last_tab = this.tabs.read(cx).active_id();
                    let belongs = this.tabs.read(cx).get(last_tab).map(|t| t.space_id) == Some(id);
                    this.spaces.update(cx, |s, _| {
                        s.record_last_active_tab(id, belongs.then_some(last_tab));
                    });
                }

                let active_space_id = space_ids
                    .get(snapshot.active_space_index)
                    .copied()
                    .unwrap_or(DEFAULT_SPACE_ID);
                this.apply_space_switch(active_space_id, cx);
            }
            None => {
                let startup = GeneralSettings::try_get(cx)
                    .map(|s| s.default_startup_tab())
                    .unwrap_or_default();
                match startup {
                    StartupTab::Terminal => this.open_terminal_tab(window, cx),
                    StartupTab::Empty => {}
                }
            }
        }
        // Scrollback cleanup (T14-002): drop files for panes that no longer
        // exist plus anything past the retention window. Runs once on startup,
        // after restore has re-registered the panes it could bring back.
        this.cleanup_scrollback(cx);
        this
    }

    // ── Session persistence (T14-001) ─────────────────────────────────────

    /// Snapshot every Space, each with its persistable tabs + every workspace
    /// tab's split-pane tree (T20-008 Spaces).
    pub fn session_snapshot(&self, cx: &App) -> crate::session_v3::SpacesSnapshotV3 {
        let capture = self.session_capture(cx);
        persist_scrollback_captures(&labonair_filesystem::paths::data_dir(), capture.scrollbacks);
        capture.snapshot
    }

    fn session_capture(&self, cx: &App) -> SessionCapture {
        let mut scrollbacks = Vec::new();
        let snapshot = self.session_snapshot_collect(cx, &mut scrollbacks);
        SessionCapture {
            snapshot,
            scrollbacks,
        }
    }

    /// Scrollback-file UUIDs of every currently-open local pane.
    fn known_scrollback_ids(&self) -> Vec<String> {
        self.panes
            .values()
            .map(|e| e.scrollback_id.clone())
            .collect()
    }

    /// Remove orphaned / stale persisted scrollback files (T14-002).
    pub fn cleanup_scrollback(&self, cx: &App) {
        let retention = terminal_settings(cx).scrollback_retention_secs();
        labonair_terminal::scrollback::cleanup(
            &labonair_filesystem::paths::data_dir(),
            &self.known_scrollback_ids(),
            retention,
        );
    }

    /// Persist the snapshot now if the `sessionRestore` preference is on;
    /// otherwise remove any stale snapshot. Returns nothing — best effort.
    fn maybe_save_session(&self, cx: &App) {
        if GeneralSettings::try_get(cx)
            .map(|s| s.session_restore())
            .unwrap_or(false)
        {
            let capture = self.session_capture(cx);
            let data_dir = labonair_filesystem::paths::data_dir();
            cx.background_executor()
                .spawn(async move {
                    crate::session::save_snapshot(&capture.snapshot);
                    persist_scrollback_captures(&data_dir, capture.scrollbacks);
                })
                .detach();
        }
    }

    /// The tab store (for later phases / command palette wiring).
    pub fn tab_store(&self) -> &Entity<TabStore> {
        &self.tabs
    }

    /// The current project/standalone identity plus tab activity. This is the
    /// only workspace state contract that permanent shell surfaces should use
    /// when deciding whether the workspace is empty.
    pub fn state(&self, cx: &App) -> context::WorkspaceState {
        context::WorkspaceState::from_parts(
            self.context.identity().clone(),
            self.tabs.read(cx).len(),
        )
    }

    /// Return the filesystem root for workspace-owned file surfaces.
    ///
    /// The workspace owns the project/standalone precedence rule. Shell
    /// composition roots should consume this contract instead of rebuilding
    /// it for individual panels.
    pub fn filesystem_root(&self, cx: &App) -> Option<String> {
        context::resolve_filesystem_root(
            self.context
                .identity()
                .project_root()
                .map(std::path::Path::to_path_buf),
            self.active_cwd(cx),
            dirs::home_dir(),
        )
    }

    /// The `(path, ssh_session_id)` pair the sidebar Explorer should browse.
    /// `ssh_session_id` is `Some` while the focused pane is a connected SSH
    /// terminal, so Explorer lists the remote host over SFTP instead of the
    /// local disk. Whether it follows the pane's cwd or stays put is decided
    /// per pane (`context::resolve_explorer_root`).
    pub fn explorer_root(&self, cx: &App) -> Option<context::ExplorerTarget> {
        self.explorer_resolve(cx).0
    }

    /// Whether the Explorer currently follows the focused pane's cwd.
    pub fn explorer_follows(&self, cx: &App) -> bool {
        self.explorer_resolve(cx).1
    }

    /// Flip the focused pane between following and staying at its current root.
    pub fn toggle_explorer_follow(&mut self, cx: &mut Context<Self>) {
        let Some((_, pane)) = self.active_pane(cx) else {
            return;
        };
        let (target, follows) = self.explorer_resolve(cx);
        let mode = if follows {
            context::ExplorerMode::Pinned(target)
        } else {
            context::ExplorerMode::Follow
        };
        self.explorer_modes.insert(pane, mode);
        cx.notify();
    }

    fn explorer_resolve(&self, cx: &App) -> (Option<context::ExplorerTarget>, bool) {
        let mode = self
            .active_pane(cx)
            .and_then(|(_, pane)| self.explorer_modes.get(&pane));
        let live = self
            .active_cwd(cx)
            .map(|cwd| (cwd, self.active_ssh_session_id(cx)));
        let (target, follows) = context::resolve_explorer_root(
            mode,
            self.context
                .identity()
                .project_root()
                .map(std::path::Path::to_path_buf),
            live.clone(),
            self.explorer_sticky.borrow().clone(),
            dirs::home_dir(),
        );
        if follows && live.is_some() {
            *self.explorer_sticky.borrow_mut() = live;
        }
        (target, follows)
    }

    /// The live SSH connection id backing the active tab's terminal, if any.
    fn active_ssh_session_id(&self, cx: &App) -> Option<String> {
        let active = self.tabs.read(cx).active()?;
        self.ssh_tabs
            .values()
            .find(|t| t.tab_id == active.id)
            .map(|t| t.ssh_id.clone())
    }

    /// Spawn the Workspace filesystem adapter for one Editor-owned finder
    /// generation. The root is passed explicitly and all traversal and
    /// canonicalization happen on the blocking runtime, never on GPUI's
    /// foreground thread.
    pub fn file_finder_listing(
        &self,
        filesystem_root: std::path::PathBuf,
        query: FileFinderQuery,
        generation: u64,
    ) -> tokio::task::JoinHandle<Result<FileFinderResult, String>> {
        crate::file_finder::spawn_listing(&self.tokio, filesystem_root, query, generation)
    }

    /// Return the repository root for workspace-owned Git surfaces.
    pub fn git_root(&self, cx: &App) -> Option<String> {
        context::resolve_git_root(
            self.context
                .identity()
                .project_root()
                .map(std::path::Path::to_path_buf),
            self.active_cwd(cx),
        )
    }

    /// Apply one explicit project/standalone identity transition.
    ///
    /// Also writes the resulting identity back onto the currently active
    /// Space. `apply_space_switch` already left the active Space's stored
    /// identity in sync before calling this, so that write is a harmless
    /// no-op there; for the direct "Open Project…" / "Return to Standalone"
    /// commands (which retarget the active Space in place rather than
    /// creating a new one) it is the fix that keeps `SpaceStore` from going
    /// stale — without it, switching away and back to that Space would
    /// silently revert the retarget.
    pub fn apply_transition(
        &mut self,
        transition: context::WorkspaceTransition,
        cx: &mut Context<Self>,
    ) {
        if !self.context.apply_transition(&transition) {
            return;
        }
        self.sync_editor_git_context();
        let active_space = self.spaces.read(cx).active_id();
        match transition {
            context::WorkspaceTransition::OpenProject { root } => {
                self.last_project_settings_rejection_signature = None;
                labonair_settings::set_active_project_root(cx, Some(root.clone()));
                self.spaces.update(cx, |s, cx| {
                    s.set_identity(active_space, context::WorkspaceIdentity::project(root), cx)
                });
                self.notify_project_settings_rejections(cx);
                cx.notify();
            }
            context::WorkspaceTransition::ReturnToStandalone => {
                self.last_project_settings_rejection_signature = None;
                labonair_settings::set_active_project_root(cx, None);
                self.spaces.update(cx, |s, cx| {
                    s.set_identity(active_space, context::WorkspaceIdentity::Standalone, cx)
                });
                cx.notify();
            }
        }
    }

    /// Explicit "Open Project…" action: re-roots the *active* Space at
    /// `root` in place (unlike `create_space_from_project`, which opens a
    /// new Space for it) and records it in Recent Projects. The shell's
    /// project-folder picker calls this once a folder is chosen, rather than
    /// `apply_transition` directly, so a bare internal resync (Space switch)
    /// never bumps Recent Projects on its own.
    pub fn open_project_in_place(&mut self, root: std::path::PathBuf, cx: &mut Context<Self>) {
        self.apply_transition(
            context::WorkspaceTransition::OpenProject { root: root.clone() },
            cx,
        );
        self.record_recent_project(root);
    }

    // ── Spaces (T20-008) ───────────────────────────────────────────────────

    /// Switch which Space is visible. No-op if `target` is already active or
    /// unknown. Reuses `apply_transition` for Explorer/Git/settings-root
    /// retargeting — the same path "Open Project…" already uses — so a Space
    /// switch changes both which tabs are shown *and* which project scope is
    /// live, in one step.
    pub fn switch_space(&mut self, target: SpaceId, cx: &mut Context<Self>) {
        if target == self.spaces.read(cx).active_id() {
            return;
        }
        self.apply_space_switch(target, cx);
    }

    /// The actual switch, without the "already active" guard — also used to
    /// land on the right space/tab once startup restore has recreated every
    /// space's tabs (see `Workspace::new`), where the freshly-seeded
    /// `SpaceStore` may already *report* `DEFAULT_SPACE_ID` as active even
    /// though its identity/active-tab still need to be applied.
    fn apply_space_switch(&mut self, target: SpaceId, cx: &mut Context<Self>) {
        let Some(target_space) = self.spaces.read(cx).get(target).cloned() else {
            return;
        };
        let outgoing = self.spaces.read(cx).active_id();
        let current_tab = self.tabs.read(cx).active_id();
        let current_belongs =
            self.tabs.read(cx).get(current_tab).map(|t| t.space_id) == Some(outgoing);
        self.spaces.update(cx, |s, cx| {
            s.record_last_active_tab(outgoing, current_belongs.then_some(current_tab));
            s.set_active(target, cx);
        });
        self.tabs.update(cx, |s, _| s.set_current_space(target));

        // Edge case: the target space's project root was deleted from disk
        // since it was last active. Don't silently point Explorer/Git at a
        // dead root — switch the tabs but leave the current identity as-is
        // and tell the user why.
        match &target_space.identity {
            context::WorkspaceIdentity::Project { root } if root.is_dir() => {
                self.apply_transition(
                    context::WorkspaceTransition::OpenProject { root: root.clone() },
                    cx,
                );
            }
            context::WorkspaceIdentity::Project { root } => {
                labonair_notifications::notify_err::<()>(
                    "Switch Space",
                    Err(format!(
                        "\"{}\"'s project folder no longer exists ({}).",
                        target_space.name,
                        root.display()
                    )),
                    cx,
                );
            }
            context::WorkspaceIdentity::Standalone => {
                self.apply_transition(context::WorkspaceTransition::ReturnToStandalone, cx);
            }
        }

        let next_tab = target_space
            .last_active_tab
            .filter(|id| self.tabs.read(cx).get(*id).map(|t| t.space_id) == Some(target))
            .or_else(|| {
                self.tabs
                    .read(cx)
                    .tabs_in_space(target)
                    .first()
                    .map(|t| t.id)
            });
        if let Some(id) = next_tab {
            self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        } else {
            self.tabs.update(cx, |s, cx| s.clear_active(cx));
        }
        cx.notify();
    }

    /// Create a new Standalone space (auto-numbered name) and switch to it.
    pub fn create_space_standalone(&mut self, cx: &mut Context<Self>) -> SpaceId {
        let n = self.spaces.read(cx).len() + 1;
        let id = self.spaces.update(cx, |s, cx| {
            s.create(
                format!("Space {n}"),
                context::WorkspaceIdentity::Standalone,
                cx,
            )
        });
        self.switch_space(id, cx);
        id
    }

    /// Create a new Project-backed space rooted at `root` and switch to it.
    /// Called by the shell once the native folder picker resolves (mirrors
    /// `request_open_project`'s split: Workspace owns the identity, the shell
    /// owns the platform picker).
    pub fn create_space_from_project(
        &mut self,
        root: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) -> SpaceId {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let id = self.spaces.update(cx, |s, cx| {
            s.create(name, context::WorkspaceIdentity::project(root.clone()), cx)
        });
        self.switch_space(id, cx);
        self.record_recent_project(root);
        id
    }

    /// Reopen a project from "Recent Projects": switches to the existing
    /// Space already rooted there if one is open, otherwise creates a new
    /// one — avoids piling up duplicate Spaces for the same folder.
    pub fn open_or_create_project_space(
        &mut self,
        root: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) -> SpaceId {
        let existing = self
            .spaces
            .read(cx)
            .spaces()
            .iter()
            .find(|s| s.identity.project_root() == Some(root.as_path()))
            .map(|s| s.id);
        if let Some(id) = existing {
            self.switch_space(id, cx);
            self.record_recent_project(root);
            id
        } else {
            self.create_space_from_project(root, cx)
        }
    }

    /// Record `root` in the persisted Recent Projects list (best-effort disk
    /// write) — shared by every path that roots a Space at a project folder.
    fn record_recent_project(&mut self, root: std::path::PathBuf) {
        self.recent_projects.record(root);
        recent_projects::save(&self.recent_projects);
    }

    /// Recent project folders opened via Spaces, most-recent first — feeds
    /// the "Recent Projects" section of the Spaces popover / Tabs sidebar.
    pub fn recent_projects(&self) -> &[recent_projects::RecentProject] {
        self.recent_projects.entries()
    }

    /// Ask the application shell to present the native folder picker for a
    /// *new* space (as opposed to `request_open_project`, which re-roots the
    /// current space).
    pub fn request_new_project_space(&self, cx: &mut Context<Self>) {
        cx.emit(WorkspaceEvent::NewProjectSpace);
    }

    /// Close a space and every tab in it. Rejected for the permanent Default
    /// space. Forced, without a per-tab dirty confirmation — mirrors the
    /// existing "Close All {kind}" precedent (`Workspace::close_by_kind`,
    /// tab context menu), not the sequential single-tab `close_all_tabs`
    /// path, since a bulk/scoped close is the closer existing analogue.
    pub fn close_space(&mut self, id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        if id == DEFAULT_SPACE_ID {
            return;
        }
        let was_active = self.spaces.read(cx).active_id() == id;
        for tab in self.tabs.update(cx, |s, cx| s.close_by_space(id, cx)) {
            self.retire_tab(&tab, cx);
        }
        self.editor_diagnostics.remove(&id);
        self.spaces.update(cx, |s, cx| {
            s.close(id, cx);
        });
        if was_active {
            self.apply_space_switch(DEFAULT_SPACE_ID, cx);
        }
        self.focus_active(window, cx);
        cx.notify();
    }

    fn sync_editor_git_context(&self) {
        self.editor_git_bridge.set_context(EditorGitContext {
            root: self
                .context
                .identity()
                .project_root()
                .map(|root| root.to_string_lossy().into_owned()),
            session_id: None,
        });
    }

    /// Request an explicit project-folder selection from the application
    /// composition root. Workspace owns the resulting identity; the shell
    /// only supplies the platform picker because it owns the application
    /// window and platform services.
    pub fn request_open_project(&self, cx: &mut Context<Self>) {
        cx.emit(WorkspaceEvent::OpenProject);
    }

    /// The working directory of the active pane's shell, if known — feeds the
    /// status-bar cwd breadcrumb (T04-003).
    pub fn active_cwd(&self, cx: &App) -> Option<String> {
        self.active_pane_view(cx).and_then(|v| v.read(cx).cwd())
    }

    /// The working directory to seed a freshly spawned local terminal with.
    /// Prefers the active pane's live cwd, so splits/new tabs keep following
    /// wherever the user already `cd`-ed to; falls back to the active space's
    /// project root when there is no live cwd yet (e.g. the first terminal
    /// opened in a brand-new Project space), and finally to the shell's own
    /// default (home) when there is no project scope either.
    fn terminal_spawn_cwd(&self, cx: &App) -> Option<String> {
        context::resolve_terminal_cwd(
            self.active_cwd(cx),
            self.context
                .identity()
                .project_root()
                .map(std::path::Path::to_path_buf),
        )
    }

    /// Report project-settings keys rejected by the project whitelist once per
    /// explicit project transition. This is deliberately called after a
    /// transition or an explicit project-settings refresh, never during
    /// rendering.
    fn notify_project_settings_rejections(&mut self, cx: &mut Context<Self>) {
        let rejected = cx
            .global::<labonair_settings::SettingsStore>()
            .project_rejected_keys();
        if !rejected.is_empty() {
            let signature = rejected.join("\u{1f}");
            if self.last_project_settings_rejection_signature.as_deref() == Some(&signature) {
                return;
            }
            self.last_project_settings_rejection_signature = Some(signature);
            let body = format!(
                "Ignored (not allowed from a project settings file): {}",
                rejected.join(", ")
            );
            labonair_notifications::notify_err::<()>("Project settings", Err(body), cx);
        }
    }

    /// Command: create (if missing) `<project root>/.labonair/settings.json`
    /// from the commented scaffold and open it as an editor tab (T19-003
    /// Anweisung #6). The root comes from the explicit workspace identity;
    /// terminal cwd is intentionally not used as an implicit project choice.
    pub fn open_or_create_project_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(root) = self
            .context
            .identity()
            .project_root()
            .map(std::path::Path::to_path_buf)
        else {
            labonair_notifications::notify_err::<()>(
                "Project settings",
                Err("Open a project before opening project settings.".to_string()),
                cx,
            );
            return false;
        };
        let path = match labonair_settings::ensure_project_settings_file(&root) {
            Ok(path) => path,
            Err(err) => {
                labonair_notifications::notify_err::<()>("Project settings", Err(err), cx);
                return false;
            }
        };
        // The directory may not have existed (and thus not been watchable)
        // before `ensure_project_settings_file` created it — force a
        // reload + fresh watch rather than relying on the next render.
        labonair_settings::refresh_project_watch(cx);
        self.notify_project_settings_rejections(cx);
        self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
        true
    }

    /// Command: "Open Settings (JSON)" (T19-005 Anweisung #4) — create (if
    /// missing) `~/.config/labonair/config.json` from the
    /// commented scaffold and open it as an editor tab. Unlike the project
    /// settings command above, this always succeeds (the user settings path
    /// doesn't depend on an active pane's cwd).
    pub fn open_or_create_user_settings_json(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = match labonair_settings::ensure_user_settings_file() {
            Ok(path) => path,
            Err(err) => {
                labonair_notifications::notify_err::<()>("Settings", Err(err), cx);
                return;
            }
        };
        self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
    }

    /// The keymap tab's "Edit keymap.json" escape hatch (T19-008) — create (if
    /// missing) `~/.config/labonair/keymap.json` and open it as an editor tab,
    /// same pattern as [`Self::open_or_create_user_settings_json`]. Injected
    /// into [`KeymapManagementView`] by [`Self::open_keymap_tab`].
    pub fn open_or_create_user_keymap_json(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = match labonair_keymap::file::ensure_user_keymap_file() {
            Ok(path) => path,
            Err(err) => {
                labonair_notifications::notify_err::<()>("Keymap", Err(err), cx);
                return;
            }
        };
        self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
    }

    /// The file path of the active editor tab, if the active tab is an editor
    /// with a saved file — feeds the breadcrumb's "file mode" + the
    /// `cursorPosition` bar item.
    pub fn active_file_path(&self, cx: &App) -> Option<String> {
        let active = self.tabs.read(cx).active()?.clone();
        if active.kind != TabKind::Editor {
            return None;
        }
        self.editors
            .get(&self.editor_slot_for_tab(active.id))?
            .read(cx)
            .path()
            .map(|p| p.to_string_lossy().into_owned())
    }

    /// The active Editor view, if the current tab is owned by Editor. This is
    /// the typed statusbar contribution seam; Workspace exposes the view while
    /// all status controls and behavior stay in the Editor owner.
    pub fn active_editor_view(&self, cx: &App) -> Option<Entity<EditorView>> {
        let active = self.tabs.read(cx).active()?;
        if active.kind != TabKind::Editor {
            return None;
        }
        self.editors
            .get(&self.editor_slot_for_tab(active.id))
            .cloned()
    }

    /// Symbols from every editor tab in the active space, for the Editor-owned
    /// Go to Symbol palette. This is a live open-buffer projection; it does
    /// not scan closed files or build a second project index.
    pub fn editor_symbol_picker_rows(
        &self,
        cx: &App,
    ) -> Vec<labonair_editor::command_provider::EditorSymbolPickerRow> {
        let active_space = self.spaces.read(cx).active_id();
        let editors = self
            .tabs
            .read(cx)
            .tabs_in_space(active_space)
            .into_iter()
            .filter(|tab| tab.kind == TabKind::Editor)
            .map(|tab| (tab.id, tab.label()))
            .collect::<Vec<_>>();

        let mut rows = Vec::new();
        for (tab_id, file_label) in editors {
            let Some(editor) = self.editors.get(&self.editor_slot_for_tab(tab_id)) else {
                continue;
            };
            rows.extend(
                editor
                    .read(cx)
                    .document_symbols()
                    .into_iter()
                    .map(
                        |symbol| labonair_editor::command_provider::EditorSymbolPickerRow {
                            tab_id,
                            file_label: file_label.clone(),
                            symbol,
                        },
                    ),
            );
        }
        rows
    }

    /// Current diagnostics from each editor tab in the active space, for the
    /// Editor-owned diagnostics palette page.
    pub fn editor_diagnostic_picker_rows(
        &self,
        cx: &App,
    ) -> Vec<labonair_editor::command_provider::EditorDiagnosticPickerRow> {
        let active_space = self.spaces.read(cx).active_id();
        let editors = self
            .tabs
            .read(cx)
            .tabs_in_space(active_space)
            .into_iter()
            .filter(|tab| tab.kind == TabKind::Editor)
            .map(|tab| (tab.id, tab.label()))
            .collect::<Vec<_>>();

        let mut rows = Vec::new();
        for (tab_id, file_label) in editors {
            let Some(editor) = self.editors.get(&self.editor_slot_for_tab(tab_id)) else {
                continue;
            };
            rows.extend(editor.read(cx).diagnostics().into_iter().map(|diagnostic| {
                labonair_editor::command_provider::EditorDiagnosticPickerRow {
                    tab_id,
                    file_label: file_label.clone(),
                    diagnostic,
                }
            }));
        }
        rows
    }

    /// Activate an editor tab from the current space and move its caret to a
    /// symbol definition selected from the Go to Symbol palette.
    pub fn goto_editor_line(
        &mut self,
        tab_id: u64,
        line0: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let active_space = self.spaces.read(cx).active_id();
        let valid_target = self
            .tabs
            .read(cx)
            .get(tab_id)
            .is_some_and(|tab| tab.kind == TabKind::Editor && tab.space_id == active_space);
        if !valid_target {
            return;
        }

        self.reveal_tab(tab_id, window, cx);
        if let Some(editor) = self.editors.get(&self.editor_slot_for_tab(tab_id)) {
            editor.update(cx, |editor, cx| editor.goto_line(line0, cx));
        }
    }

    /// `{hostId, sessionId}` of the SSH session backing the active pane, when
    /// it's remote — lets the breadcrumb browse through the same session.
    pub fn active_remote_target(&self, cx: &App) -> Option<(String, String)> {
        let active = self.tabs.read(cx).active()?.clone();
        let host_id = active.data.host_id.clone()?;
        let session_id = active.data.session_id.map(|s| s.to_string())?;
        Some((host_id, session_id))
    }

    /// Send a `cd <path>` into the active pane's shell (breadcrumb navigation).
    pub fn send_cd(&self, path: &str, cx: &App) {
        let cmd = format!("cd {}\n", shell_quote(path));
        self.inject_into_active_terminal(&cmd, cx);
    }

    /// Open a new local terminal tab rooted at `path` (breadcrumb "open in new
    /// terminal").
    pub fn cd_in_new_tab(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        self.run_snippet_local(Some(path), String::new(), window, cx);
    }

    /// The active tab's display label.
    pub fn active_tab_label(&self, cx: &App) -> String {
        self.tabs
            .read(cx)
            .active()
            .map(Tab::label)
            .unwrap_or_default()
    }

    /// Number of panes in the active workspace tab (0 for non-workspace tabs).
    pub fn active_pane_count(&self, cx: &App) -> usize {
        self.active_layout(cx)
            .map(WorkspaceLayout::len)
            .unwrap_or(0)
    }

    /// Whether the active tab targets a terminal (vs. an editor / other) —
    /// drives which surface the header's inline search dispatches to.
    pub fn active_is_terminal(&self, cx: &App) -> bool {
        self.tabs
            .read(cx)
            .active()
            .map(|t| t.kind == TabKind::Workspace)
            .unwrap_or(false)
    }

    /// Which surface the `Cmd+F` search overlay (T18-002) targets for the
    /// active tab.
    pub fn active_search_target(&self, cx: &App) -> SearchTarget {
        if self.active_editor_slot(cx).is_some() {
            SearchTarget::Editor
        } else if self.active_pane_view(cx).is_some() {
            SearchTarget::Terminal
        } else {
            SearchTarget::Unavailable
        }
    }

    /// One-line pre-fill for the search overlay (editor selection only).
    pub fn search_seed(&self, cx: &App) -> Option<String> {
        let id = self.tabs.read(cx).active_id();
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .and_then(|e| e.read(cx).search_seed())
    }

    /// Original active-buffer selection captured when the search overlay opens.
    pub fn search_selection_range(&self, cx: &App) -> Option<TextRange> {
        let id = self.tabs.read(cx).active_id();
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .and_then(|editor| editor.read(cx).search_selection_range())
    }

    /// Active range for the Editor-owned buffer-search selection toggle.
    pub fn search_selection_scope(&self, cx: &App) -> Option<TextRange> {
        let id = self.tabs.read(cx).active_id();
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .and_then(|editor| editor.read(cx).search_selection_scope())
    }

    /// Current `(match_index, total)` for the active Editor buffer query.
    pub fn buffer_search_count(&self, cx: &App) -> Option<(usize, usize)> {
        let id = self.tabs.read(cx).active_id();
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .map(|editor| editor.read(cx).search_count())
    }

    /// Toggle the Editor-owned buffer-search selection scope.
    pub fn toggle_search_selection_scope(&mut self, cx: &mut Context<Self>) -> bool {
        let id = self.tabs.read(cx).active_id();
        let Some(editor) = self.editors.get(&self.editor_slot_for_tab(id)).cloned() else {
            return false;
        };
        editor.update(cx, |editor, cx| editor.toggle_search_selection_scope(cx))
    }

    /// Start / update the search against whichever surface the active tab owns.
    /// Returns `(current_1_based, total)`; `None` when the tab is not searchable.
    pub fn search_set(
        &mut self,
        query: SearchQuery,
        cx: &mut Context<Self>,
    ) -> Option<Result<(usize, usize), SearchError>> {
        self.project_search_task.take();
        let id = self.tabs.read(cx).active_id();
        if let Some(e) = self.editors.get(&self.editor_slot_for_tab(id)).cloned() {
            return Some(e.update(cx, |e, cx| e.search_set(query, cx)));
        }
        let view = self.active_pane_view(cx)?;
        Some(Ok(view.update(cx, |v, cx| {
            v.search_set(&query.text, query.case_sensitive_for_matching(), cx)
        })))
    }

    /// Saved editor file paths in the active space for the open-files search
    /// filter. Unsaved buffers have no filesystem path to pass to the provider.
    fn open_editor_paths_in_active_space(&self, cx: &App) -> Vec<std::path::PathBuf> {
        let active_space = self.spaces.read(cx).active_id();
        self.tabs
            .read(cx)
            .tabs_in_space(active_space)
            .into_iter()
            .filter(|tab| tab.kind == TabKind::Editor)
            .filter_map(|tab| self.editors.get(&self.editor_slot_for_tab(tab.id)))
            .filter_map(|editor| editor.read(cx).path())
            .collect()
    }

    /// Start the Editor-owned project search through the explicit workspace
    /// filesystem root. Workspace owns only this async adapter and operation
    /// handle; query/result/status state stays in the Editor entity that
    /// started the search, even when opening a result activates another tab.
    pub fn project_search_set(
        &mut self,
        query: ProjectSearchQuery,
        cx: &mut Context<Self>,
    ) -> Option<()> {
        self.project_search_task.take();
        let tab_id = self.tabs.read(cx).active_id();
        let editor = self
            .editors
            .get(&self.editor_slot_for_tab(tab_id))
            .cloned()?;
        self.project_search_owner = Some(tab_id);
        let request = editor.update(cx, |editor, cx| editor.begin_project_search(query, cx));
        if request.query.text.is_empty() {
            return Some(());
        }
        let generation = request.generation;
        let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) else {
            let _ = editor.update(cx, |editor, cx| {
                editor.fail_project_search(
                    request.generation,
                    "No filesystem root is available for this workspace.",
                    cx,
                )
            });
            return Some(());
        };
        let open_file_paths = if request.query.options.only_open_files {
            crate::project_search::relative_open_paths(
                &root,
                self.open_editor_paths_in_active_space(cx),
            )
        } else {
            Vec::new()
        };

        let job = self.tokio.spawn_blocking(move || {
            crate::project_search::search_blocking(root, request, open_file_paths)
        });
        self.project_search_task = Some(cx.spawn(async move |this, cx| {
            let result = job.await.map_err(|error| error.to_string()).and_then(|r| r);
            let _ = this.update(cx, |workspace, cx| {
                if workspace.project_search_owner != Some(tab_id)
                    || !workspace.editors.contains_key(&tab_id)
                {
                    return;
                }
                workspace.project_search_task = None;
                match result {
                    Ok(result) => {
                        let _ = editor
                            .update(cx, |editor, cx| editor.accept_project_search(result, cx));
                    }
                    Err(error) => {
                        let _ = editor.update(cx, |editor, cx| {
                            editor.fail_project_search(generation, error.clone(), cx)
                        });
                        labonair_notifications::notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                labonair_notifications::Notification::error(
                                    "Project search failed",
                                    "The project search could not be completed.",
                                )
                                .source("editor")
                                .details(error.clone())
                                .dedupe_key(format!("editor:project-search:{error}")),
                                cx,
                            );
                        });
                    }
                }
            });
        }));
        Some(())
    }

    /// Load the source files for project-search hits into the Editor-owned
    /// Multi-Buffer model. Path checks and file reads run off the foreground
    /// thread; this method is only the typed Workspace filesystem adapter.
    pub fn load_project_multibuffer(
        &mut self,
        hits: Vec<ProjectSearchHit>,
        callback: impl FnOnce(Result<MultiBuffer, String>, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) else {
            callback(
                Err("No filesystem root is available for this workspace.".to_string()),
                cx,
            );
            return;
        };
        let job = self
            .tokio
            .spawn_blocking(move || crate::project_multibuffer::load(&root, &hits, 3));
        cx.spawn(async move |this, cx| {
            let result = job.await.map_err(|error| error.to_string()).and_then(|r| r);
            let _ = this.update(cx, move |_workspace, cx| callback(result, cx));
        })
        .detach();
    }

    /// Save dirty Multi-Buffer files through the filesystem lifecycle adapter.
    /// All disk identities are checked before the first write; a later I/O
    /// failure returns already saved files so Editor can accept those revisions.
    pub fn save_project_multibuffer(
        &self,
        requests: Vec<MultiBufferSaveRequest>,
        callback: impl FnOnce(Result<MultiBufferSaveOutcome, String>, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) {
        if requests.is_empty() {
            callback(Ok(MultiBufferSaveOutcome::default()), cx);
            return;
        }
        let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) else {
            callback(
                Err("No filesystem root is available for this workspace.".to_string()),
                cx,
            );
            return;
        };
        let job = self
            .tokio
            .spawn_blocking(move || crate::project_multibuffer::save(&root, requests));
        cx.spawn(async move |this, cx| {
            let result = job.await.map_err(|error| error.to_string()).and_then(|r| r);
            let _ = this.update(cx, move |_workspace, cx| callback(result, cx));
        })
        .detach();
    }

    pub fn project_search_snapshot(&self, cx: &App) -> Option<ProjectSearchSnapshot> {
        let id = self.project_search_owner?;
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .map(|editor| editor.read(cx).project_search_snapshot())
    }

    pub fn project_search_move(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> Option<ProjectSearchHit> {
        let id = self.project_search_owner?;
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .cloned()
            .and_then(|editor| {
                let result =
                    editor.update(cx, |editor, _| editor.move_project_search_selection(delta));
                if result.is_some() {
                    cx.notify();
                }
                result
            })
    }

    /// Open a selected project result through the canonical file route and
    /// move the Editor caret to its reported line/column.
    pub fn open_project_search_hit(
        &mut self,
        hit: ProjectSearchHit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) else {
            labonair_notifications::notify_err::<()>(
                "Open search result",
                Err("No filesystem root is available for this workspace.".to_string()),
                cx,
            );
            return;
        };
        let path = match root_safe_path(&root, std::path::Path::new(&hit.path)) {
            Ok(path) => path,
            Err(error) => {
                labonair_notifications::notify_err::<()>("Open search result", Err(error), cx);
                return;
            }
        };
        self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
        let id = self.tabs.read(cx).active_id();
        if let Some(editor) = self.editors.get(&self.editor_slot_for_tab(id)).cloned() {
            editor.update(cx, |editor, cx| {
                editor.goto_position(
                    Position::new(hit.line.saturating_sub(1) as usize, hit.start_column),
                    cx,
                )
            });
        }
    }

    /// Step to the next / previous match. Returns `(current, total)`.
    pub fn search_step(
        &mut self,
        forward: bool,
        cx: &mut Context<Self>,
    ) -> Option<Result<(usize, usize), SearchError>> {
        let id = self.tabs.read(cx).active_id();
        if let Some(e) = self.editors.get(&self.editor_slot_for_tab(id)).cloned() {
            return Some(e.update(cx, |e, cx| e.search_step(forward, cx)));
        }
        let view = self.active_pane_view(cx)?;
        Some(Ok(view.update(cx, |v, cx| v.search_step(forward, cx))))
    }

    /// Forward Editor-owned replacement actions without exposing the document
    /// or its mutable structures to Workspace.
    pub fn search_replace_one(
        &mut self,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Option<Result<bool, SearchError>> {
        let id = self.tabs.read(cx).active_id();
        let editor = self.editors.get(&self.editor_slot_for_tab(id)).cloned()?;
        Some(editor.update(cx, |editor, cx| editor.search_replace_one(replacement, cx)))
    }

    pub fn search_replace_all(
        &mut self,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Option<Result<usize, SearchError>> {
        let id = self.tabs.read(cx).active_id();
        let editor = self.editors.get(&self.editor_slot_for_tab(id)).cloned()?;
        Some(editor.update(cx, |editor, cx| editor.search_replace_all(replacement, cx)))
    }

    /// Replace matches from the active project search in matching open editor
    /// buffers. Each buffer change is one Editor-owned undoable transaction;
    /// closed files remain untouched until the project result view can review
    /// and save cross-file changes safely.
    pub fn project_replace_all(
        &mut self,
        replacement: &str,
        cx: &mut Context<Self>,
    ) -> Option<Result<ProjectReplaceResult, SearchError>> {
        let search_owner = self.project_search_owner?;
        let active_editor = self
            .editors
            .get(&self.editor_slot_for_tab(search_owner))
            .cloned()?;
        let request = match active_editor.update(cx, |editor, cx| {
            editor.begin_project_replace(replacement.to_string(), cx)
        }) {
            Ok(request) => request,
            Err(error) => return Some(Err(error)),
        };
        let mut buffers = Vec::with_capacity(self.editors.len());
        let mut editors_by_path = std::collections::HashMap::new();
        for editor in self.editors.values() {
            if let Some(candidate) = editor.read(cx).project_buffer_candidate() {
                editors_by_path.insert(candidate.path.clone(), editor.clone());
                buffers.push(candidate);
            }
        }
        let plan = match active_editor
            .read(cx)
            .plan_project_replace(&request, &buffers)
        {
            Ok(plan) => plan,
            Err(error) => {
                active_editor.update(cx, |editor, cx| {
                    editor.fail_project_replace(request.generation, error.message.clone(), cx);
                });
                return Some(Err(error));
            }
        };

        let mut replacements = 0;
        for target in &plan.targets {
            let Some(editor) = editors_by_path.get(&target.path).cloned() else {
                let error = SearchError {
                    message: format!("{} is no longer open", target.path),
                };
                active_editor.update(cx, |editor, cx| {
                    editor.fail_project_replace(request.generation, error.message.clone(), cx);
                });
                return Some(Err(error));
            };
            match editor.update(cx, |editor, cx| {
                editor.apply_project_replace_target(&request, target, cx)
            }) {
                Ok(count) => replacements += count,
                Err(error) => {
                    active_editor.update(cx, |editor, cx| {
                        editor.fail_project_replace(request.generation, error.message.clone(), cx);
                    });
                    return Some(Err(error));
                }
            }
        }
        let result = match active_editor.update(cx, |editor, cx| {
            editor.complete_project_replace(&plan, replacements, cx)
        }) {
            Ok(result) => result,
            Err(error) => return Some(Err(error)),
        };
        cx.notify();
        Some(Ok(result))
    }

    pub fn search_can_replace(&self, cx: &App) -> bool {
        let id = self.tabs.read(cx).active_id();
        self.editors
            .get(&self.editor_slot_for_tab(id))
            .is_some_and(|editor| editor.read(cx).search_can_replace())
    }

    /// Clear search state on the active surface and pinned project-search owner.
    pub fn search_end(&mut self, cx: &mut Context<Self>) {
        self.project_search_task.take();
        let active_id = self.tabs.read(cx).active_id();
        let search_owner = self.project_search_owner.take();
        if let Some(e) =
            search_owner.and_then(|id| self.editors.get(&self.editor_slot_for_tab(id)).cloned())
        {
            e.update(cx, |e, cx| e.search_close(cx));
        }
        if search_owner != Some(active_id) {
            if let Some(e) = self
                .editors
                .get(&self.editor_slot_for_tab(active_id))
                .cloned()
            {
                e.update(cx, |e, cx| e.search_close(cx));
            }
        }
        if let Some(view) = self.active_pane_view(cx) {
            view.update(cx, |v, cx| v.search_end(cx));
        }
    }

    /// Focus the active pane (called by the app shell after closing an overlay).
    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_active(window, cx);
    }

    // ── Menu / shortcut entry points (T04-005) ──────────────────────────────
    // Thin `pub` wrappers so the native menu and keyboard shortcuts drive the
    // exact same code path.

    /// Open a new local terminal tab.
    pub fn new_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_terminal_tab(window, cx);
    }

    /// Open a new local terminal tab rooted at `cwd` — the Explorer's
    /// "Open in Terminal" context action.
    pub fn new_terminal_tab_in(
        &mut self,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
    }

    // ── Snippets (T12-001) ────────────────────────────────────────────────

    /// Type `command` (without executing beyond the trailing newline the caller
    /// includes, if any) into the active terminal pane — snippet "inject" mode.
    /// No-op when the active pane is not a terminal.
    /// Local dev-server URL detected in the active terminal's output, if any
    /// (statusbar `previewUrl` item).
    pub fn active_preview_url(&self, cx: &App) -> Option<String> {
        self.preview_url_cache
            .clone()
            .unwrap_or_else(|| self.active_pane_view(cx)?.read(cx).preview_url())
    }

    /// Last `n` lines of the active terminal's buffer (AI live bridge).
    pub fn active_terminal_lines(&self, n: usize, cx: &App) -> Vec<String> {
        self.active_pane_view(cx)
            .map(|v| v.read(cx).recent_lines(n))
            .unwrap_or_default()
    }

    pub fn inject_into_active_terminal(&self, text: &str, cx: &App) {
        if let Some(view) = self.active_pane_view(cx) {
            let _ = view.read(cx).handle().write(text.as_bytes());
        }
    }

    /// Execute `command` in the active terminal (appends a newline). If no
    /// terminal is focused, opens a new local tab in the active cwd and runs it
    /// there. Used by the AI panel's "Run" affordances (command snippets,
    /// AI⇄Shell composer mode).
    pub fn run_in_active_terminal(
        &mut self,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cmd = command.trim_end().to_string();
        if cmd.is_empty() {
            return;
        }
        if let Some(view) = self.active_pane_view(cx) {
            let _ = view.read(cx).handle().write(format!("{cmd}\n").as_bytes());
            self.focus_active(window, cx);
        } else {
            let cwd = self.terminal_spawn_cwd(cx);
            self.run_snippet_local(cwd, cmd, window, cx);
        }
    }

    /// Open a new local terminal tab in `cwd` and run `command` in it —
    /// snippet "terminal" mode, local target.
    pub fn run_snippet_local(
        &mut self,
        cwd: Option<String>,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle.clone(), window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
        let _ = handle.write(format!("{}\n", command.trim_end()).as_bytes());
        cx.notify();
    }

    /// Open (or reuse a connection to) an SSH tab for `host_id` and run
    /// `command` in it once the session is established — snippet "terminal"
    /// mode, SSH target.
    pub fn run_snippet_ssh_terminal(
        &mut self,
        host_id: String,
        command: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(ssh_id) = self.connect_host(host_id, window, cx) {
            self.pending_snippet_ssh.insert(ssh_id, command);
        }
    }

    /// The `ssh_id` of a live SSH session for `host_id`, if one is open — used
    /// by snippet "silent" mode against SSH targets.
    pub fn ssh_session_for_host(&self, host_id: &str) -> Option<String> {
        self.ssh_tabs
            .values()
            .find(|t| t.host_id == host_id)
            .map(|t| t.ssh_id.clone())
    }

    /// Open a file from the Explorer in the code editor. `peek` opens it as a
    /// reusable preview tab (single click); a non-peek call (double click, or a
    /// file already open) makes/keeps it permanent. Clicking a different file
    /// while a peek tab is open replaces that tab's content instead of piling
    /// up tabs — the VS Code / Labonair "peek" behaviour.
    pub fn open_file(
        &mut self,
        path: String,
        peek: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pathbuf = std::path::PathBuf::from(&path);
        let title = pathbuf
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path.as_str())
            .to_string();

        // Already open (permanent) → just activate it.
        let existing = self.editors.iter().find_map(|(tab_id, view)| {
            (view.read(cx).path().as_deref() == Some(pathbuf.as_path())).then_some(*tab_id)
        });
        if let Some(slot) = existing {
            let pane = self
                .layouts
                .values()
                .flat_map(|l| l.leaves())
                .find(|p| self.editor_slot_of_pane(*p) == Some(slot))
                .unwrap_or(slot);
            if let Some(tab_id) = self.tab_of_pane(pane).or(Some(slot)) {
                self.tabs.update(cx, |s, cx| {
                    if !peek {
                        s.set_peek(tab_id, false, cx);
                    }
                });
            }
            self.activate_pane(pane, window, cx);
            return;
        }

        // Reuse an existing peek tab if present.
        let peek_tab = self
            .tabs
            .read(cx)
            .tabs_by_kind(TabKind::Editor)
            .iter()
            .find(|t| t.peek)
            .map(|t| t.id);

        let tab_id = if let Some(tab_id) = peek_tab {
            self.tabs.update(cx, |s, cx| {
                s.set_path(tab_id, Some(path.clone()), cx);
                s.set_custom_title(tab_id, Some(title.clone()), cx);
                s.set_peek(tab_id, peek, cx);
                s.set_active(tab_id, cx);
            });
            tab_id
        } else {
            let tab_id = self.tabs.update(cx, |s, cx| {
                let id = s.open(
                    TabKind::Editor,
                    TabData {
                        path: Some(path.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                s.set_custom_title(id, Some(title.clone()), cx);
                s.set_peek(id, peek, cx);
                id
            });
            let view = self.new_editor_view(tab_id, cx);
            self.watch_editor(tab_id, &view, cx);
            self.editors.insert(tab_id, view);
            tab_id
        };

        let view = self.editors.get(&self.editor_slot_for_tab(tab_id)).cloned();
        if let Some(view) = view {
            view.update(cx, |e, cx| e.open_path(pathbuf, cx));
        }
        self.focus_active(window, cx);
    }

    fn open_document_link(&mut self, uri: String, window: &mut Window, cx: &mut Context<Self>) {
        if uri.starts_with("file://") {
            let path = match decode_local_file_uri(&uri) {
                Ok(path) => path,
                Err(error) => {
                    labonair_notifications::notify_err::<()>("Open document link", Err(error), cx);
                    return;
                }
            };
            let path = if let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) {
                match root_safe_path(&root, &path) {
                    Ok(path) => path,
                    Err(error) => {
                        labonair_notifications::notify_err::<()>(
                            "Open document link",
                            Err(error),
                            cx,
                        );
                        return;
                    }
                }
            } else {
                match std::fs::canonicalize(&path) {
                    Ok(path) => path,
                    Err(error) => {
                        labonair_notifications::notify_err::<()>(
                            "Open document link",
                            Err(format!("cannot resolve file link: {error}")),
                            cx,
                        );
                        return;
                    }
                }
            };
            if !path.is_file() {
                labonair_notifications::notify_err::<()>(
                    "Open document link",
                    Err("The linked path is not a file.".to_string()),
                    cx,
                );
                return;
            }
            self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
        } else if is_supported_external_document_link(&uri) {
            cx.open_url(&uri);
        } else {
            labonair_notifications::notify_err::<()>(
                "Open document link",
                Err("The linked URI scheme is not supported.".to_string()),
                cx,
            );
        }
    }

    fn open_call_hierarchy_location(
        &mut self,
        uri: String,
        position: labonair_editor::LspPosition,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = match decode_local_file_uri(&uri) {
            Ok(path) => path,
            Err(error) => {
                labonair_notifications::notify_err::<()>(
                    "Call hierarchy navigation",
                    Err(error),
                    cx,
                );
                return;
            }
        };
        let path = if let Some(root) = self.filesystem_root(cx).map(std::path::PathBuf::from) {
            match root_safe_path(&root, &path) {
                Ok(path) => path,
                Err(error) => {
                    labonair_notifications::notify_err::<()>("Language navigation", Err(error), cx);
                    return;
                }
            }
        } else {
            path
        };
        let path_string = path.to_string_lossy().into_owned();
        self.open_file(path_string, false, window, cx);
        if let Some(editor) = self
            .editors
            .iter()
            .find(|(_, editor)| editor.read(cx).path().as_deref() == Some(path.as_path()))
            .map(|(_, editor)| editor.clone())
        {
            editor.update(cx, |editor, cx| editor.goto_lsp_position(position, cx));
        }
    }

    /// Open one Finder result only after re-validating it against the current
    /// explicit workspace root. Finder results may outlive a root transition
    /// or a file deletion, so the adapter never trusts stale UI metadata.
    pub fn open_file_finder_candidate(
        &mut self,
        candidate: FileFinderCandidate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let root = self
            .filesystem_root(cx)
            .map(std::path::PathBuf::from)
            .ok_or_else(|| "No filesystem root is available for this workspace.".to_string())?;
        let path = root_safe_path(&root, &candidate.path)?;
        if !path.is_file() {
            return Err(format!(
                "File finder result is not a file: {}",
                path.display()
            ));
        }
        self.open_file(path.to_string_lossy().into_owned(), false, window, cx);
        Ok(())
    }

    /// `Cmd-E` / File ▸ New Editor Tab — an empty, pathless editor.
    pub fn new_editor_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open(TabKind::Editor, TabData::default(), cx));
        let view = self.new_editor_view(tab_id, cx);
        self.watch_editor(tab_id, &view, cx);
        self.editors.insert(tab_id, view);
        self.focus_active(window, cx);
    }

    /// File ▸ New Preview Tab — an empty native preview pane (the WebView
    /// replacement). Opens the address bar ready for a path or URL.
    pub fn new_preview_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open(TabKind::Preview, TabData::default(), cx));
        let theme = self.theme.clone();
        let view = cx.new(|cx| PreviewView::new(theme, cx));
        self.previews.insert(tab_id, view);
        self.focus_active(window, cx);
    }

    /// Open `target` (a local path or URL) in a preview tab, reusing the active
    /// preview tab if one is already open.
    pub fn open_preview(&mut self, target: String, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_preview_target(target, cx);
        self.focus_active(window, cx);
    }

    fn activate_preview_target(&mut self, target: String, cx: &mut Context<Self>) {
        let title = std::path::Path::new(&target)
            .file_name()
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| target.clone());

        let existing = self
            .tabs
            .read(cx)
            .tabs_by_kind(TabKind::Preview)
            .first()
            .map(|t| t.id);

        let tab_id = if let Some(tab_id) = existing {
            self.tabs.update(cx, |s, cx| {
                s.set_custom_title(tab_id, Some(title.clone()), cx);
                s.set_url(tab_id, Some(target.clone()), cx);
                s.set_active(tab_id, cx);
            });
            tab_id
        } else {
            let tab_id = self.tabs.update(cx, |s, cx| {
                let id = s.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                s.set_custom_title(id, Some(title.clone()), cx);
                id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(tab_id, view);
            tab_id
        };

        self.markdown_previews
            .retain(|_, preview_tab_id| *preview_tab_id != tab_id);
        if let Some(view) = self.previews.get(&tab_id).cloned() {
            view.update(cx, |p, cx| p.set_url(target, cx));
        }
    }

    /// Apply a registry-backed image-viewer command to the active preview tab.
    /// Other preview content and tab kinds ignore image-only commands.
    pub fn apply_active_preview_image_action(
        &mut self,
        action: PreviewImageAction,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab_id) = self
            .tabs
            .read(cx)
            .active()
            .filter(|tab| tab.kind == TabKind::Preview)
            .map(|tab| tab.id)
        else {
            return;
        };
        let Some(preview) = self.previews.get(&tab_id).cloned() else {
            return;
        };
        let viewport = window.viewport_size();
        preview.update(cx, |preview, cx| {
            preview.perform_image_action(
                action,
                f32::from(viewport.width),
                f32::from(viewport.height),
                cx,
            )
        });
    }

    /// Open the Editor-owned Markdown buffer in a Workspace preview tab. The
    /// preview keeps following that editor tab through subsequent revisions.
    fn open_markdown_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.md".to_string());
        let name = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let title = format!("{name} Preview");
        let existing = self
            .markdown_previews
            .get(&editor_tab_id)
            .copied()
            .filter(|preview_tab_id| self.previews.contains_key(preview_tab_id));

        let preview_tab_id = if let Some(preview_tab_id) = existing {
            self.tabs.update(cx, |tabs, cx| {
                tabs.set_custom_title(preview_tab_id, Some(title.clone()), cx);
                tabs.set_url(preview_tab_id, Some(target.clone()), cx);
                tabs.set_active(preview_tab_id, cx);
            });
            preview_tab_id
        } else {
            let preview_tab_id = self.tabs.update(cx, |tabs, cx| {
                let preview_tab_id = tabs.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                tabs.set_custom_title(preview_tab_id, Some(title), cx);
                tabs.set_active(preview_tab_id, cx);
                preview_tab_id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(preview_tab_id, view);
            self.markdown_previews.insert(editor_tab_id, preview_tab_id);
            preview_tab_id
        };

        if let Some(view) = self.previews.get(&preview_tab_id).cloned() {
            let language = source.language;
            let path = source.path;
            let snapshot = source.snapshot;
            view.update(cx, |preview, cx| {
                preview.set_editor_source(path, language, snapshot, cx)
            });
        }
        self.pending_preview_focus = true;
        cx.notify();
    }

    fn open_svg_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.svg".to_string());
        let name = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let title = format!("{name} Preview");
        let existing = self
            .svg_previews
            .get(&editor_tab_id)
            .copied()
            .filter(|preview_tab_id| self.previews.contains_key(preview_tab_id));

        let preview_tab_id = if let Some(preview_tab_id) = existing {
            self.tabs.update(cx, |tabs, cx| {
                tabs.set_custom_title(preview_tab_id, Some(title.clone()), cx);
                tabs.set_url(preview_tab_id, Some(target.clone()), cx);
                tabs.set_active(preview_tab_id, cx);
            });
            preview_tab_id
        } else {
            let preview_tab_id = self.tabs.update(cx, |tabs, cx| {
                let preview_tab_id = tabs.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target.clone()),
                        ..TabData::default()
                    },
                    cx,
                );
                tabs.set_custom_title(preview_tab_id, Some(title), cx);
                tabs.set_active(preview_tab_id, cx);
                preview_tab_id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(preview_tab_id, view);
            self.svg_previews.insert(editor_tab_id, preview_tab_id);
            preview_tab_id
        };

        if let Some(view) = self.previews.get(&preview_tab_id).cloned() {
            view.update(cx, |preview, cx| {
                preview.set_editor_svg_source(source.path, source.snapshot, cx)
            });
        }
        self.pending_preview_focus = true;
        cx.notify();
    }

    fn set_editor_side_preview_content(
        preview: &Entity<PreviewView>,
        content: EditorSidePreviewContent,
        cx: &mut Context<Self>,
    ) {
        preview.update(cx, |preview, cx| match content {
            EditorSidePreviewContent::Markdown(source) => {
                preview.set_editor_source(source.path, source.language, source.snapshot, cx)
            }
            EditorSidePreviewContent::Svg(source) => {
                preview.set_editor_svg_source(source.path, source.snapshot, cx)
            }
            EditorSidePreviewContent::Tabular(source) => {
                preview.set_editor_tabular_source(source.path, source.snapshot, cx)
            }
        });
    }

    fn update_editor_side_preview_for_source(
        preview: &Entity<PreviewView>,
        kind: EditorSidePreviewKind,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        preview.update(cx, |preview, cx| match kind {
            EditorSidePreviewKind::Markdown => {
                preview.set_editor_source(source.path, source.language, source.snapshot, cx)
            }
            EditorSidePreviewKind::Svg => {
                preview.set_editor_svg_source(source.path, source.snapshot, cx)
            }
            EditorSidePreviewKind::Tabular => {
                preview.set_editor_tabular_source(source.path, source.snapshot, cx)
            }
        });
    }

    fn open_following_svg_preview(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.svg".to_string());
        let name = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let title = format!("{name} Following Preview");
        let existing = self
            .following_svg_preview
            .filter(|preview_tab_id| self.previews.contains_key(preview_tab_id));
        let preview_tab_id = if let Some(preview_tab_id) = existing {
            self.tabs.update(cx, |tabs, cx| {
                tabs.set_custom_title(preview_tab_id, Some(title.clone()), cx);
                tabs.set_url(preview_tab_id, Some(target.clone()), cx);
                tabs.set_active(preview_tab_id, cx);
            });
            preview_tab_id
        } else {
            let preview_tab_id = self.tabs.update(cx, |tabs, cx| {
                let preview_tab_id = tabs.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target),
                        ..TabData::default()
                    },
                    cx,
                );
                tabs.set_custom_title(preview_tab_id, Some(title), cx);
                tabs.set_active(preview_tab_id, cx);
                preview_tab_id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(preview_tab_id, view);
            preview_tab_id
        };

        self.following_svg_preview = Some(preview_tab_id);
        self.following_svg_editor = Some(editor_tab_id);
        self.svg_previews
            .retain(|_, mapped_preview_id| *mapped_preview_id != preview_tab_id);
        self.update_following_svg_preview_source(source, cx);
        self.pending_preview_focus = true;
        cx.notify();
    }

    fn update_following_svg_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self
            .editors
            .get(&self.editor_slot_for_tab(editor_tab_id))
            .cloned()
        else {
            return;
        };
        self.following_svg_editor = Some(editor_tab_id);
        self.update_following_svg_preview_source(editor.read(cx).buffer_snapshot(), cx);
    }

    fn update_following_svg_preview_source(
        &mut self,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_tab_id) = self.following_svg_preview else {
            return;
        };
        let Some(preview) = self.previews.get(&preview_tab_id).cloned() else {
            self.following_svg_preview = None;
            self.following_svg_editor = None;
            return;
        };
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.svg".to_string());
        let title = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| format!("{name} Following Preview"))
            .unwrap_or_else(|| "Untitled Following Preview".to_string());
        self.tabs.update(cx, |tabs, cx| {
            tabs.set_custom_title(preview_tab_id, Some(title), cx);
            tabs.set_url(preview_tab_id, Some(target), cx);
        });
        preview.update(cx, |preview, cx| {
            preview.set_editor_svg_source(source.path, source.snapshot, cx)
        });
    }

    fn update_svg_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_tab_id) = self.svg_previews.get(&editor_tab_id).copied() else {
            return;
        };
        let Some(preview) = self.previews.get(&preview_tab_id).cloned() else {
            self.svg_previews.remove(&editor_tab_id);
            return;
        };
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.svg".to_string());
        let title = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| format!("{name} Preview"));
        self.tabs.update(cx, |tabs, cx| {
            tabs.set_custom_title(preview_tab_id, title, cx);
            tabs.set_url(preview_tab_id, Some(target), cx);
        });
        preview.update(cx, |preview, cx| {
            preview.set_editor_svg_source(source.path, source.snapshot, cx)
        });
    }

    fn open_following_markdown_preview(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.md".to_string());
        let name = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let title = format!("{name} Following Preview");
        let existing = self
            .following_markdown_preview
            .filter(|preview_tab_id| self.previews.contains_key(preview_tab_id));
        let preview_tab_id = if let Some(preview_tab_id) = existing {
            self.tabs.update(cx, |tabs, cx| {
                tabs.set_custom_title(preview_tab_id, Some(title.clone()), cx);
                tabs.set_url(preview_tab_id, Some(target), cx);
                tabs.set_active(preview_tab_id, cx);
            });
            preview_tab_id
        } else {
            let preview_tab_id = self.tabs.update(cx, |tabs, cx| {
                let preview_tab_id = tabs.open(
                    TabKind::Preview,
                    TabData {
                        url: Some(target),
                        ..TabData::default()
                    },
                    cx,
                );
                tabs.set_custom_title(preview_tab_id, Some(title), cx);
                tabs.set_active(preview_tab_id, cx);
                preview_tab_id
            });
            let theme = self.theme.clone();
            let view = cx.new(|cx| PreviewView::new(theme, cx));
            self.previews.insert(preview_tab_id, view);
            preview_tab_id
        };

        self.following_markdown_preview = Some(preview_tab_id);
        self.following_markdown_editor = Some(editor_tab_id);
        self.markdown_previews
            .retain(|_, mapped_preview_id| *mapped_preview_id != preview_tab_id);
        self.update_following_markdown_preview_source(source, cx);
        self.pending_preview_focus = true;
        cx.notify();
    }

    fn update_following_markdown_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self
            .editors
            .get(&self.editor_slot_for_tab(editor_tab_id))
            .cloned()
        else {
            return;
        };
        let source = editor.read(cx).buffer_snapshot();
        self.following_markdown_editor = Some(editor_tab_id);
        self.update_following_markdown_preview_source(source, cx);
    }

    fn update_following_markdown_preview_source(
        &mut self,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_tab_id) = self.following_markdown_preview else {
            return;
        };
        let Some(preview) = self.previews.get(&preview_tab_id).cloned() else {
            self.following_markdown_preview = None;
            self.following_markdown_editor = None;
            return;
        };
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.md".to_string());
        let title = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| format!("{name} Following Preview"))
            .unwrap_or_else(|| "Untitled Following Preview".to_string());
        self.tabs.update(cx, |tabs, cx| {
            tabs.set_custom_title(preview_tab_id, Some(title), cx);
            tabs.set_url(preview_tab_id, Some(target), cx);
        });
        preview.update(cx, |preview, cx| {
            preview.set_editor_source(source.path, source.language, source.snapshot, cx)
        });
    }

    fn update_markdown_preview_for_editor(
        &mut self,
        editor_tab_id: u64,
        source: EditorBufferSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_tab_id) = self.markdown_previews.get(&editor_tab_id).copied() else {
            return;
        };
        let Some(view) = self.previews.get(&preview_tab_id).cloned() else {
            self.markdown_previews.remove(&editor_tab_id);
            return;
        };
        let title = source
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| format!("{name} Preview"));
        let target = source
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.md".to_string());
        self.tabs.update(cx, |tabs, cx| {
            tabs.set_custom_title(preview_tab_id, title, cx);
            tabs.set_url(preview_tab_id, Some(target), cx);
        });
        let language = source.language;
        let path = source.path;
        let snapshot = source.snapshot;
        view.update(cx, |preview, cx| {
            preview.set_editor_source(path, language, snapshot, cx)
        });
    }

    /// Save the active editor tab (`Cmd-S`).
    pub fn save_active(&mut self, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        if let Some(view) = self.editors.get(&self.editor_slot_for_tab(id)).cloned() {
            view.update(cx, |e, cx| e.save(cx));
        }
    }

    /// Resolve the canonical command id to the active Editor owner. Workspace
    /// supplies only the active-view lookup; command meaning and execution
    /// remain in `EditorView`/`labonair-editor`.
    pub(crate) fn dispatch_active_editor_command(
        &mut self,
        id: CommandId,
        cx: &mut Context<Self>,
    ) -> EditorCommandDispatchResult {
        let tab_id = self.tabs.read(cx).active_id();
        let Some(editor) = self.editors.get(&self.editor_slot_for_tab(tab_id)).cloned() else {
            return EditorCommandDispatchResult::NoActiveEditor;
        };
        if editor.update(cx, |editor, cx| editor.dispatch_registered_command(id, cx)) {
            EditorCommandDispatchResult::Forwarded
        } else {
            EditorCommandDispatchResult::Unsupported(id)
        }
    }

    fn new_editor_view(&mut self, tab_id: u64, cx: &mut Context<Self>) -> Entity<EditorView> {
        let theme = self.theme.clone();
        let bridge = self.editor_git_bridge.clone();
        let language_services = self.editor_language_services.clone();
        let space_id = self
            .tabs
            .read(cx)
            .get(tab_id)
            .map(|tab| tab.space_id)
            .unwrap_or_else(|| self.spaces.read(cx).active_id());
        let diagnostics = self.editor_diagnostics.entry(space_id).or_default().clone();
        let workspace = cx.entity();
        cx.new(|cx| {
            let mut view =
                EditorView::new_with_project_diagnostics(theme, language_services, diagnostics, cx);
            view.set_git_bridge(bridge.clone(), bridge.clone());
            let call_hierarchy_workspace = workspace.clone();
            view.set_call_hierarchy_navigator(move |uri, position, window, app| {
                call_hierarchy_workspace.update(app, |this, cx| {
                    this.open_call_hierarchy_location(uri, position, window, cx);
                });
            });
            let document_link_workspace = workspace.clone();
            view.set_document_link_navigator(move |uri, window, app| {
                document_link_workspace.update(app, |this, cx| {
                    this.open_document_link(uri, window, cx);
                });
            });
            view
        })
    }

    /// Composition seam for the app/shell to inject an explicitly configured
    /// local language-service runtime. Workspace stores only the handle and
    /// forwards it to existing editor views; it owns no LSP state.
    pub fn set_editor_language_services(
        &mut self,
        runtime: Arc<LanguageServiceRuntime>,
        cx: &mut Context<Self>,
    ) {
        runtime.set_tokio_handle(self.tokio.clone());
        self.editor_language_services = runtime.clone();
        for view in self.editors.values() {
            let runtime = runtime.clone();
            view.update(cx, |view, _cx| view.set_language_services(runtime));
        }
    }

    pub fn editor_language_services(&self) -> Arc<LanguageServiceRuntime> {
        self.editor_language_services.clone()
    }

    /// Drain editor review intents and resolve them through the one canonical
    /// Project Diff item. Git state discovery runs on the injected service's
    /// async boundary; no Git I/O occurs while rendering.
    fn drain_editor_git_actions(&mut self, cx: &mut Context<Self>) {
        for queued in self.editor_git_bridge.take_review_actions() {
            let action = queued.action;
            let target = match &action {
                labonair_editor::GitHunkAction::OpenProjectDiff { target }
                | labonair_editor::GitHunkAction::ShowChange { target } => target.clone(),
                _ => continue,
            };
            let context = queued.context;
            let Some(root) = context.root.clone() else {
                labonair_notifications::notify_err::<()>(
                    "Open Project Diff",
                    Err("Git actions require an explicit repository workspace.".to_string()),
                    cx,
                );
                continue;
            };
            let git = self.git.clone();
            let session = context.session_id.clone();
            let relative_target = target.path.clone();
            let task = self.tokio.spawn(async move {
                working_tree_diff_request(
                    git,
                    root,
                    session,
                    labonair_panel::DiffSource::WorkingTree,
                    Some(relative_target),
                )
                .await
            });
            cx.spawn(async move |this, cx| {
                let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
                let _ = this.update(cx, |this, cx| {
                    if this.editor_git_bridge.context() != context {
                        return;
                    }
                    match result {
                        Ok(request) => this.open_project_diff(request, cx),
                        Err(error) => {
                            labonair_notifications::notify_err::<()>(
                                "Open Project Diff",
                                Err(error),
                                cx,
                            );
                        }
                    }
                });
            })
            .detach();
        }
    }

    fn watch_editor(&self, tab_id: u64, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        let tabs = self.tabs.clone();
        cx.observe(view, move |_this, editor, cx| {
            let editor = editor.read(cx);
            let path = editor
                .path()
                .map(|path| path.to_string_lossy().into_owned());
            let status = path.as_deref().and_then(|path| {
                editor_tab_git_status(editor.git_gutter_snapshot(), editor.editor_revision(), path)
            });
            let diagnostic_status = path
                .as_ref()
                .and_then(|_| editor_tab_diagnostic_status(&editor.diagnostics()));
            tabs.update(cx, |tabs, cx| {
                tabs.set_git_status(tab_id, status, cx);
                tabs.set_diagnostic_status(tab_id, diagnostic_status, cx);
            });
            cx.notify();
        })
        .detach();

        cx.subscribe(view, move |this, view, ev: &EditorEvent, cx| {
            if matches!(ev, EditorEvent::CloseRequested) {
                // Vim `:q` / `:wq` — close the pane that shows this editor
                // (the tab when it is the last pane). Needs a window, so it is
                // drained from `render`.
                let pane = this
                    .layouts
                    .values()
                    .flat_map(|l| l.leaves())
                    .find(|p| this.editor_slot_of_pane(*p) == Some(tab_id))
                    .unwrap_or(tab_id);
                this.pending_tab_close.push(pane);
                cx.notify();
                return;
            }
            match ev {
                EditorEvent::MarkdownPreviewRequested(source) => {
                    this.open_markdown_preview_for_editor(tab_id, source.clone(), cx);
                }
                EditorEvent::MarkdownFollowingPreviewRequested(source) => {
                    this.open_following_markdown_preview(tab_id, source.clone(), cx);
                }
                EditorEvent::SvgPreviewRequested(source) => {
                    this.open_svg_preview_for_editor(tab_id, source.clone(), cx);
                }
                EditorEvent::SvgFollowingPreviewRequested(source) => {
                    this.open_following_svg_preview(tab_id, source.clone(), cx);
                }
                EditorEvent::PreviewToSideRequested { content } => {
                    this.open_editor_preview_to_side(tab_id, content.clone(), cx);
                }
                EditorEvent::GroupFocused(group) => {
                    this.on_editor_group_focused(tab_id, *group, cx);
                }
                EditorEvent::TabularPreviewRequested(path) => {
                    this.activate_preview_target(path.to_string_lossy().into_owned(), cx);
                    this.pending_preview_focus = true;
                    cx.notify();
                }
                EditorEvent::ProjectDiagnosticsChanged => cx.notify(),
                EditorEvent::ProjectDiagnosticsRefreshRequested => {
                    let source_space = this.tabs.read(cx).get(tab_id).map(|tab| tab.space_id);
                    let editors = this
                        .editors
                        .iter()
                        .filter_map(|(editor_tab_id, editor)| {
                            let editor_space = this
                                .tabs
                                .read(cx)
                                .get(*editor_tab_id)
                                .map(|tab| tab.space_id);
                            (source_space.is_some() && source_space == editor_space)
                                .then(|| editor.clone())
                        })
                        .collect::<Vec<_>>();
                    for editor in editors {
                        let _ =
                            editor.update(cx, |editor, cx| editor.refresh_project_diagnostics(cx));
                    }
                }
                EditorEvent::BufferChanged(source) => {
                    this.update_markdown_preview_for_editor(tab_id, source.clone(), cx);
                    this.update_svg_preview_for_editor(tab_id, source.clone(), cx);
                    if this.following_markdown_editor == Some(tab_id) {
                        this.update_following_markdown_preview_source(source.clone(), cx);
                    }
                    if this.following_svg_editor == Some(tab_id) {
                        this.update_following_svg_preview_source(source.clone(), cx);
                    }
                    if let Some((preview_pane, kind)) =
                        this.editor_side_previews.get(&tab_id).copied()
                    {
                        if let Some(preview) = this.previews.get(&preview_pane).cloned() {
                            Self::update_editor_side_preview_for_source(
                                &preview,
                                kind,
                                source.clone(),
                                cx,
                            );
                        }
                    }
                }
                _ => {}
            }
            let dirty = view.read(cx).is_dirty();
            let showing: Vec<u64> = this
                .layouts
                .iter()
                .filter(|(_, l)| {
                    l.leaves()
                        .iter()
                        .any(|p| this.editor_slot_of_pane(*p) == Some(tab_id))
                })
                .map(|(tab, _)| *tab)
                .collect();
            for tab in showing {
                this.sync_tab_dirty(tab, cx);
            }
            let is_remote_edit = this.remote_edits.contains_key(&tab_id);
            this.tabs.update(cx, |s, cx| {
                s.set_dirty(tab_id, dirty, cx);
                if matches!(ev, EditorEvent::Edited) {
                    s.set_peek(tab_id, false, cx);
                }
                if !is_remote_edit {
                    if let Some(title) = view
                        .read(cx)
                        .path()
                        .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_string))
                    {
                        s.set_custom_title(tab_id, Some(title), cx);
                    }
                }
            });

            // Remote-edit tab: a `dirty → clean` transition means the editor
            // just saved the local temp copy — push it back to the host.
            if let Some(re) = this.remote_edits.get_mut(&tab_id) {
                let saved = re.dirty && !dirty;
                re.dirty = dirty;
                if saved {
                    let service = this.ssh_remote_file.clone();
                    let (sid, rpath, tpath) = (
                        re.session_id.clone(),
                        re.remote_path.clone(),
                        re.temp_path.clone(),
                    );
                    let jh = this.tokio.spawn(async move {
                        service
                            .save_remote_edit(SshSessionId::new(sid), rpath, tpath)
                            .await
                    });
                    cx.spawn(async move |_this, _cx| {
                        if let Ok(Err(e)) = jh.await {
                            tracing::warn!(%e, "save_remote_edit failed");
                        }
                    })
                    .detach();
                }
            }
            cx.notify();
        })
        .detach();
    }

    /// Split the focused pane of the active tab in `dir` (`pane::Split*`).
    pub fn split(&mut self, dir: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        self.split_focused(dir, window, cx);
    }

    /// Evenly redistribute every divider in the active tab's pane tree.
    pub fn reset_pane_sizes(&mut self, cx: &mut Context<Self>) {
        let tab_id = self.tabs.read(cx).active_id();
        if self
            .layouts
            .get_mut(&tab_id)
            .is_some_and(WorkspaceLayout::reset_sizes)
        {
            cx.notify();
        }
    }

    /// `Close Tab`: close the focused pane if the tab is split, else the tab.
    pub fn close_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_active_pane_or_tab(window, cx);
    }

    /// `Close Pane`: close just the focused pane.
    pub fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_focused_pane(window, cx);
    }

    /// Cycle to the next (`forward`) or previous tab.
    pub fn cycle(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_tab(forward, window, cx);
    }

    fn active_editor_history_state(&self, cx: &App) -> Option<(bool, bool)> {
        let tab = self.tabs.read(cx).active()?;
        if tab.kind != TabKind::Editor {
            return None;
        }
        Some(
            self.editors
                .get(&self.editor_slot_for_tab(tab.id))
                .map_or((false, false), |editor| {
                    let editor = editor.read(cx);
                    (
                        editor.can_navigate_history_back(),
                        editor.can_navigate_history_forward(),
                    )
                }),
        )
    }

    fn navigate_tab_bar(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let editor_id = self
            .tabs
            .read(cx)
            .active()
            .filter(|tab| tab.kind == TabKind::Editor)
            .map(|tab| tab.id);
        if let Some(editor) =
            editor_id.and_then(|id| self.editors.get(&self.editor_slot_for_tab(id)).cloned())
        {
            let command = if forward {
                CommandId::EditorGoForward
            } else {
                CommandId::EditorGoBack
            };
            editor.update(cx, |editor, cx| {
                editor.dispatch_registered_command(command, cx);
            });
            return;
        }
        self.cycle(forward, window, cx);
    }

    fn cycle_from_tab(
        &mut self,
        source_id: u64,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(target_id) = self.tabs.read(cx).adjacent_in_space(source_id, forward) {
            self.select_tab(target_id, window, cx);
        }
    }

    /// Jump to the tab at position `index` (0-based). No-op when no tab holds
    /// that slot. Port of `useTabsStore.selectByIndex` (T13-005).
    pub fn select_tab_by_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.tabs.read(cx).tabs().get(index).map(|t| t.id) {
            self.select_tab(id, window, cx);
        }
    }

    /// Cycle focus to the next pane of the active tab (`pane::FocusNext`).
    pub fn focus_next_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_pane_cycle(true, window, cx);
    }

    /// Whether the active tab holds two or more panes.
    pub fn active_has_split(&self, cx: &App) -> bool {
        self.active_layout(cx).map(|l| l.len() > 1).unwrap_or(false)
    }

    /// Whether the focused pane can be split (gates the split menu items).
    pub fn active_can_split(&self, cx: &App) -> bool {
        self.active_pane(cx)
            .is_some_and(|(_, pane)| self.pane_can_split(pane, cx))
    }

    /// Whether the focused pane is a terminal.
    pub fn active_focus_is_terminal(&self, cx: &App) -> bool {
        self.active_pane_view(cx).is_some()
    }

    fn theme_colors(&self, cx: &App) -> TerminalColors {
        TerminalColors::from_theme(self.theme.read(cx).theme())
    }

    fn alloc_pane(&mut self) -> PaneId {
        crate::pane::next_id()
    }

    /// Spawn a local terminal session in `cwd` and return its id + handle +
    /// scrollback-file UUID. When `replay_scrollback_id` is `Some`, that id is
    /// reused (session restore) and its persisted scrollback — if any — is fed
    /// into the emulator before the shell starts (T14-002); otherwise a fresh
    /// id is minted.
    fn spawn_session(
        &self,
        cwd: Option<String>,
        replay_scrollback_id: Option<&str>,
        cx: &App,
    ) -> Option<(SessionId, SessionHandle, String)> {
        let ts = terminal_settings(cx);
        let shell = (!ts.shell().trim().is_empty()).then(|| ts.shell().to_string());
        let cursor_shape = Some(match ts.cursor_style() {
            PrefCursorStyle::Block => labonair_terminal::CursorShape::Block,
            PrefCursorStyle::Underline => labonair_terminal::CursorShape::Underline,
            PrefCursorStyle::Bar => labonair_terminal::CursorShape::Beam,
        });
        let scrollback_id = replay_scrollback_id
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let replay_scrollback = replay_scrollback_id.and_then(|id| {
            labonair_terminal::scrollback::load(
                &labonair_filesystem::paths::data_dir(),
                id,
                Some(ts.scrollback_max_bytes()),
            )
        });
        let options = SessionOptions {
            working_directory: cwd,
            shell,
            args: ts.shell_args(),
            env: ts.environment_variables(),
            scrollback: Some(ts.scrollback().max(1) as usize),
            replay_scrollback,
            cursor_shape,
            cursor_blink: Some(ts.cursor_blink()),
            word_separators: Some(ts.word_separator().to_string()),
            ..SessionOptions::default()
        };
        let session_id =
            match self
                .registry
                .create(self.theme_colors(cx), TermDimensions::new(80, 24), options)
            {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!(%err, "failed to spawn terminal session");
                    return None;
                }
            };
        let handle = self.registry.handle(session_id)?;
        Some((session_id, handle, scrollback_id))
    }

    fn new_terminal_view(
        &self,
        handle: SessionHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        let theme = self.theme.clone();
        let background = self.background.clone();
        cx.new(|cx| TerminalView::new(handle, theme, background, window, cx))
    }

    /// Spawn a new local terminal session and open a workspace tab for it.
    fn open_terminal_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.terminal_spawn_cwd(cx);
        let Some((session_id, handle, scrollback_id)) = self.spawn_session(cwd.clone(), None, cx)
        else {
            return;
        };
        let pane_id = self.alloc_pane();
        let tab_id = self
            .tabs
            .update(cx, |s, cx| s.open_workspace(session_id, cwd, cx));
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                scrollback_id,
            },
        );
        self.register_pane(pane_id, crate::pane_kind::PaneKind::Terminal);
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.focus_active(window, cx);
    }

    /// `Cmd-W`: close the focused pane if the tab is split, otherwise close
    /// the whole tab.
    fn close_active_pane_or_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_has_split(cx) {
            self.close_focused_pane(window, cx);
            return;
        }
        let id = self.tabs.read(cx).active_id();
        self.request_close(id, window, cx);
    }

    pub(crate) fn reset_axis(&mut self, axis_id: PaneId, cx: &mut Context<Self>) {
        let tab_id = self.tabs.read(cx).active_id();
        if let Some(layout) = self.layouts.get_mut(&tab_id) {
            if layout.reset_axis(axis_id) {
                cx.notify();
            }
        }
    }

    // ── Lookups ─────────────────────────────────────────────────────────────

    fn active_layout<'a>(&'a self, cx: &App) -> Option<&'a WorkspaceLayout> {
        let (tab, _) = self.active_pane(cx)?;
        self.layouts.get(&tab)
    }

    /// The terminal view of the focused pane, when the focused pane is one.
    fn active_pane_view(&self, cx: &App) -> Option<Entity<TerminalView>> {
        let (_, pane) = self.active_pane(cx)?;
        self.panes.get(&pane).map(|e| e.view.clone())
    }

    /// The active editor / terminal selection, tagged with its source label,
    /// for "Ask AI about Selection".
    pub fn active_selection(&self, cx: &App) -> Option<(&'static str, String)> {
        if let Some(editor) = self
            .active_editor_slot(cx)
            .and_then(|slot| self.editors.get(&slot))
        {
            return editor.read(cx).selected_text().map(|t| ("editor", t));
        }
        self.active_pane_view(cx)
            .and_then(|v| v.read(cx).selection_text())
            .map(|t| ("terminal", t))
    }

    fn select_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename_tab.is_some() {
            self.commit_tab_rename(cx);
        }
        self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        self.focus_active(window, cx);
    }

    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active_pane(cx) {
            Some((_, pane)) => self.focus_pane(pane, window, cx),
            None => window.focus(&self.focus_handle),
        }
    }

    fn cycle_tab(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.cycle(forward, cx));
        self.focus_active(window, cx);
    }

    /// Request closing a tab. Editor tabs with unsaved changes first ask for
    /// confirmation; everything else closes immediately, sessions torn down.
    fn request_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_requires_confirmation(id, cx) && self.confirm_close != Some(id) {
            self.confirm_close = Some(id);
            window.focus(&self.focus_handle);
            cx.notify();
            return;
        }
        self.do_close(id, window, cx);
    }

    fn close_requires_confirmation(&self, id: u64, cx: &App) -> bool {
        // Unsaved edits are per document: any editor document shown by this
        // tab that no pane outside the tab keeps alive asks first.
        let panes = self.tab_pane_ids(id);
        let dirty_document = panes.iter().any(|pane| {
            self.editor_slot_of_pane(*pane).is_some_and(|slot| {
                self.editors
                    .get(&slot)
                    .is_some_and(|e| e.read(cx).is_dirty())
                    && !self
                        .layouts
                        .iter()
                        .filter(|(tab, _)| **tab != id)
                        .flat_map(|(_, l)| l.leaves())
                        .any(|p| self.editor_slot_of_pane(p) == Some(slot))
            })
        });
        dirty_document
            || (terminal_settings(cx).confirm_close_terminal_tab()
                && self.tab_has_running_shell(id, cx))
    }

    fn request_close_many(&mut self, ids: Vec<u64>, window: &mut Window, cx: &mut Context<Self>) {
        self.pending_close = ids.into_iter().collect();
        self.advance_pending_close(window, cx);
    }

    fn advance_pending_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while let Some(id) = self.pending_close.pop_front() {
            if self.tabs.read(cx).get(id).is_none() {
                continue;
            }
            if self.close_requires_confirmation(id, cx) {
                self.confirm_close = Some(id);
                window.focus(&self.focus_handle);
                cx.notify();
                return;
            }
            self.do_close(id, window, cx);
        }
    }

    fn confirm_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.read(cx).get(id).is_some() {
            self.do_close(id, window, cx);
        } else {
            self.do_close_pane(id, window, cx);
        }
        self.advance_pending_close(window, cx);
    }

    fn cancel_pending_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_close = None;
        self.pending_close.clear();
        self.focus_active(window, cx);
        cx.notify();
    }

    /// Whether *any* open pane — terminal or SSH — still has a live shell.
    /// Gates `confirmQuitWithActiveSessions` at the window-close boundary.
    pub fn has_running_shells(&self, cx: &App) -> bool {
        self.panes.values().any(|entry| {
            matches!(
                entry.view.read(cx).handle().status(),
                labonair_terminal::SessionStatus::Running
            )
        })
    }

    /// Whether a workspace (terminal) tab has at least one local pane whose
    /// shell process is still alive — gates `confirmCloseTerminalTab`.
    fn tab_has_running_shell(&self, id: u64, cx: &App) -> bool {
        let Some(layout) = self.layouts.get(&id) else {
            return false;
        };
        layout.leaves().iter().any(|leaf| {
            self.panes.get(leaf).is_some_and(|entry| {
                matches!(
                    entry.view.read(cx).handle().status(),
                    labonair_terminal::SessionStatus::Running
                )
            })
        })
    }

    /// Re-apply the live-tunable terminal settings (scrollback depth, cursor
    /// shape/blink, word separators) to every open terminal pane, and repaint
    /// them so a `terminalOpacity` / `terminalCursorBlinkInterval` change is
    /// visible immediately. Called from the shell's `SettingsStore` observer.
    pub fn reapply_terminal_settings(&mut self, cx: &mut Context<Self>) {
        let cfg = terminal_emulator_config(cx);
        let views: Vec<Entity<TerminalView>> =
            self.panes.values().map(|e| e.view.clone()).collect();
        for view in views {
            let handle = view.read(cx).handle().clone();
            let _ = handle.with(|s| s.apply_runtime_config(cfg.clone()));
            view.update(cx, |_, cx| cx.notify());
        }
    }

    fn do_close(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_close == Some(id) {
            self.confirm_close = None;
        }
        if let Some(removed) = self.tabs.update(cx, |s, cx| s.close(id, cx)) {
            self.retire_tab(&removed, cx);
        }
        self.focus_active(window, cx);
    }

    fn close_others(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(space_id) = self.tabs.read(cx).get(id).map(|tab| tab.space_id) else {
            return;
        };
        let ids = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .filter(|tab| tab.id != id)
            .map(|tab| tab.id)
            .collect();
        self.request_close_many(ids, window, cx);
    }

    /// Close tabs in one Space sequentially so each dirty editor can be
    /// confirmed before the next item is removed.
    fn close_all_tabs(&mut self, space_id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .map(|tab| tab.id)
            .collect();
        self.request_close_many(ids, window, cx);
    }

    fn close_by_kind(
        &mut self,
        space_id: SpaceId,
        kind: TabKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .filter(|tab| tab.kind == kind)
            .map(|tab| tab.id)
            .collect();
        self.request_close_many(ids, window, cx);
    }

    fn close_tabs_to_side(
        &mut self,
        id: u64,
        close_right: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(space_id) = self.tabs.read(cx).get(id).map(|tab| tab.space_id) else {
            return;
        };
        let tabs = self.tabs.read(cx).tabs_in_space(space_id);
        let Some(index) = tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let ids = if close_right {
            tabs.iter().skip(index + 1).map(|tab| tab.id).collect()
        } else {
            tabs.iter().take(index).map(|tab| tab.id).collect()
        };
        self.request_close_many(ids, window, cx);
    }

    fn close_clean_tabs(&mut self, space_id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let ids = self
            .tabs
            .read(cx)
            .tabs_in_space(space_id)
            .into_iter()
            .filter(|tab| !tab.dirty)
            .map(|tab| tab.id)
            .collect();
        self.request_close_many(ids, window, cx);
    }

    /// Activate a tab by id and move focus into it — used by the header
    /// agent-access badge's "jump to tab".
    pub fn reveal_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.update(cx, |s, cx| s.set_active(id, cx));
        self.focus_active(window, cx);
    }

    // ── Command-palette entry points (T12-002) ─────────────────────────────

    /// The `CommandContext` the active tab exposes, driving which
    /// context-scoped palette commands are offered.
    pub fn active_context(&self, cx: &App) -> Option<labonair_command_palette::CommandContext> {
        let (_, pane) = self.active_pane(cx)?;
        let kind = self.pane_kind(pane, cx).tab_kind();
        let is_ssh = self
            .panes
            .get(&pane)
            .is_some_and(|entry| self.ssh_tabs.contains_key(&entry.session_id));
        labonair_command_palette::context_of(palette_tab_kind(kind), is_ssh)
    }

    /// Canonical rows for every known host — feeds palette host choices.
    pub fn host_picker_rows(&self, cx: &App) -> Vec<HostPickerRow> {
        self.host_view.picker_rows(cx)
    }

    /// Recent hosts for palette providers; ordering and labels belong to Hosts.
    pub fn recent_host_picker_rows(&self, cx: &App, n: usize) -> Vec<HostPickerRow> {
        self.host_view.recent_picker_rows(n, cx)
    }

    /// The host the active tab targets (SSH terminal or SFTP browser), if any.
    pub fn active_host_id(&self, cx: &App) -> Option<String> {
        let active = self.tabs.read(cx).active()?;
        if let Some(t) = self.ssh_tabs.values().find(|t| t.tab_id == active.id) {
            return Some(t.host_id.clone());
        }
        if active.kind == TabKind::Sftp {
            return active.data.host_id.clone();
        }
        None
    }

    /// Open (or re-focus) an SFTP browser tab for `host_id`.
    pub fn open_sftp_tab(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_sftp(host_id, window, cx);
    }

    /// Open a saved host through the typed Hosts request contract.
    pub fn open_host_request(
        &mut self,
        request: HostOpenRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match request.mode {
            HostOpenMode::Ssh => self.open_ssh_tab(request.host_id, window, cx),
            HostOpenMode::Sftp => self.open_sftp_tab(request.host_id, window, cx),
        }
    }

    /// Queue a Hosts request emitted by the management window, which has no
    /// access to the workspace window handle.
    pub fn enqueue_host_request(&mut self, request: HostOpenRequest, cx: &mut Context<Self>) {
        match request.mode {
            HostOpenMode::Ssh => self.pending_connect.push(request.host_id),
            HostOpenMode::Sftp => self.pending_sftp.push(request.host_id),
        }
        cx.notify();
    }

    /// Open (or reuse) an SSH terminal tab for `host_id` (`+` dropdown / menu).
    pub fn open_ssh_tab(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.connect_host(host_id, window, cx);
    }

    /// Open (or focus) the commit-graph tab (`+` dropdown / command palette).
    /// Share the app-shell's `GitGraphView` entity so the Git Graph tab and
    /// the shell's CWD feed operate on the same view.
    pub fn set_git_graph(&mut self, view: Entity<GitGraphView>) {
        self.git_graph = Some(view);
    }

    fn ensure_git_graph_view(&mut self, cx: &mut Context<Self>) -> Entity<GitGraphView> {
        if let Some(view) = &self.git_graph {
            return view.clone();
        }
        let view = cx.new(|cx| {
            GitGraphView::new(
                self.git_graph_service.clone(),
                self.tokio.clone(),
                self.theme.clone(),
                cx,
            )
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        self.git_graph = Some(view.clone());
        view
    }

    /// The app's [`PanelRegistry`](labonair_panel::PanelRegistry). Populated
    /// once by `labonair_shell::register_builtin_panels` right after the
    /// `Workspace` is constructed; read by the shell's dock rendering and
    /// status-bar toggles so adding a panel never touches a `match`.
    pub fn panel_registry(&self) -> &labonair_panel::PanelRegistry {
        &self.panel_registry
    }

    /// Mutable access, for the one-time builtin-panel registration in the shell.
    pub fn panel_registry_mut(&mut self) -> &mut labonair_panel::PanelRegistry {
        &mut self.panel_registry
    }

    /// The app's [`StatusItemRegistry`](labonair_panel::StatusItemRegistry).
    /// Populated once by `labonair_shell::register_builtin_status_items`; read
    /// by the [`StatusBar`](crate::status_bar::StatusBar) component.
    pub fn status_item_registry(&self) -> &labonair_panel::StatusItemRegistry {
        &self.status_item_registry
    }

    /// Mutable access, for the one-time builtin status-item registration.
    pub fn status_item_registry_mut(&mut self) -> &mut labonair_panel::StatusItemRegistry {
        &mut self.status_item_registry
    }

    /// Hidden status-bar items exposed to owner registries such as the
    /// command palette. The workspace owns visibility state and labels; the
    /// shell only composes the resulting snapshot.
    pub fn hidden_status_bar_items(&self) -> Vec<(String, String)> {
        self.status_item_registry
            .iter()
            .filter(|registration| self.status_item_registry.is_hidden(registration.id))
            .map(|registration| {
                (
                    registration.id.to_string(),
                    crate::command_provider::status_item_label(registration.id).to_string(),
                )
            })
            .collect()
    }

    /// Re-reads the persisted `statusBarItemPlacements` blob from disk and
    /// applies it to the [`StatusItemRegistry`](labonair_panel::StatusItemRegistry)
    /// overrides (T18-005). Called once at startup and whenever another
    /// window bumps [`status_placements::StatusBarLayoutTick`].
    pub fn reload_status_bar_placements(&mut self) {
        let blob = status_placements::status_bar_item_placements_load();
        let overrides = status_placements::overrides_from_blob(&blob);
        self.status_item_registry.set_overrides(overrides);
    }

    /// The right-click "move left/right" / "hide" action on a status-bar item
    /// (T18-005): applies the change to the local registry immediately (so
    /// this window's `StatusBar` re-renders without waiting on the write),
    /// then persists it through the workspace owner's atomic read-merge-write and, on
    /// completion, bumps [`status_placements::StatusBarLayoutTick`] so every
    /// *other* window's `StatusBar` reloads the blob and picks up the change
    /// too. The tick bump is deliberately deferred until after the write
    /// lands — bumping it eagerly would race the still-in-flight write, and
    /// this window's own tick observer would reload the pre-write blob from
    /// disk and clobber the override just set above.
    pub fn set_status_bar_placement(
        &mut self,
        id: &'static str,
        side: Option<labonair_panel::StatusSide>,
        hidden: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        self.status_item_registry.set_override(id, side, hidden);
        cx.notify();

        let patch = status_placements::placement_patch(side, hidden);
        let item_id = id.to_string();
        let jh = self.tokio.spawn(async move {
            status_placements::set_status_bar_item_placement(item_id, patch).await
        });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// Show a status-bar item selected from the dynamic command palette.
    /// Resolution from the persisted string to the registry's stable static
    /// id belongs to Workspace, not to the shell composition layer.
    pub fn show_status_bar_item(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(item_id) = self.status_item_registry().get(id).map(|item| item.id) {
            self.set_status_bar_placement(item_id, None, Some(false), cx);
        }
    }

    /// "Reset to default" on the Personalization settings pane (T18-007):
    /// clears every user override in the local registry immediately, then
    /// deletes the persisted `statusBarItemPlacements` blob and bumps
    /// [`status_placements::StatusBarLayoutTick`] so every window (including
    /// this one) re-reads the now-empty blob. Only the statusbar layout is
    /// reset — panel-toggle visibility is untouched (Notizen: scoped to this
    /// pane's own data, not a global reset).
    pub fn reset_status_bar_placements(&mut self, cx: &mut Context<Self>) {
        self.status_item_registry.set_overrides(HashMap::new());
        cx.notify();

        let jh = self
            .tokio
            .spawn(async move { status_placements::clear_status_bar_item_placements().await });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// The persisted `panelToggleVisibility` blob (T18-007): whether `name`'s
    /// toggle shows in the status bar's fixed-left panel-toggle cluster. A
    /// panel absent from the blob defaults to visible. Does not affect the
    /// panel's dock position or whether it can still be opened from the
    /// command palette.
    pub fn panel_toggle_visible(name: &str) -> bool {
        status_placements::panel_toggle_visibility_load()
            .get(name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
    }

    /// The single write path for panel-toggle visibility (T18-007): both the
    /// status bar's own "Hide from toggle bar" right-click action and the
    /// Personalization settings pane's per-panel switch call this. Persists
    /// through the workspace owner's atomic read-merge-write and bumps
    /// [`status_placements::StatusBarLayoutTick`] so every window's panel
    /// toggle cluster (which observes that global) re-reads the blob and
    /// reflects the change live.
    pub fn set_panel_toggle_visible(
        &mut self,
        name: String,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        let panel_name = name.clone();
        let jh = self.tokio.spawn(async move {
            status_placements::set_panel_toggle_visibility(panel_name, visible).await
        });
        cx.spawn(async move |_this, cx| {
            let _ = jh.await;
            let _ = cx.update(|app| {
                app.default_global::<status_placements::StatusBarLayoutTick>()
                    .0 += 1;
            });
        })
        .detach();
    }

    /// Set the primary edge restored from the workspace-owned layout file.
    pub fn set_primary_dock(&mut self, position: labonair_panel::DockPosition) {
        self.primary_dock_position = position;
    }

    /// The primary edge restored from workspace layout state.
    pub fn primary_dock(&self) -> labonair_panel::DockPosition {
        self.primary_dock_position
    }

    /// Which dock hosts `name` — live membership, falling back to the primary
    /// edge when the panel is somehow unregistered.
    pub fn dock_for_panel(&self, name: &str) -> labonair_panel::DockPosition {
        self.dock_of_panel(name)
            .unwrap_or_else(|| self.primary_dock())
    }

    /// Whether `name` is the active panel of an open dock.
    pub fn panel_is_active(&self, name: &str) -> bool {
        self.docks()
            .iter()
            .any(|d| d.is_open() && d.active_name() == Some(name))
    }

    /// Status-bar-toggle intent: open + activate `name`, or close its dock if it
    /// is already the active panel there. Persists the layout.
    pub fn select_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        let pos = self.dock_for_panel(name);
        self.dock_mut(pos).toggle_panel(name);
        self.persist_docks(cx);
        cx.notify();
    }

    /// "show me X" — never closes the dock (palette / menu intent).
    pub fn open_panel(&mut self, name: &str, cx: &mut Context<Self>) {
        let pos = self.dock_for_panel(name);
        {
            let dock = self.dock_mut(pos);
            dock.activate_panel(name);
            dock.set_open(true);
        }
        self.persist_docks(cx);
        cx.notify();
    }

    /// Reconcile the Tabs sidebar panel's dock membership with the
    /// `tabsLocation` setting. When it is `"sidebar"` the panel is added to the
    /// primary dock (activated + opened); otherwise it is removed from every
    /// dock. A no-op when membership already matches the setting, so it is
    /// cheap to call from the `SettingsStore` observer on every change.
    pub fn sync_tabs_in_sidebar(&mut self, cx: &mut Context<Self>) {
        let want = labonair_settings::ThemeSettings::try_get(cx)
            .map(|s| s.tabs_location() == "sidebar")
            .unwrap_or(false);
        let present = self.dock_of_panel("tabs").is_some();
        if want == present {
            return;
        }

        if want {
            let Some(handle) = self.tabs_panel.clone() else {
                return;
            };
            let pos = self.primary_dock();
            let dock = self.dock_mut(pos);
            dock.add_panel(handle);
            dock.activate_panel("tabs");
            dock.set_open(true);
        } else {
            for pos in labonair_panel::DockPosition::ALL {
                let dock = self.dock_mut(pos);
                if dock.remove_panel("tabs").is_some() && dock.is_empty() {
                    dock.set_open(false);
                }
            }
        }

        self.persist_docks(cx);
        cx.notify();
    }

    /// Debounced write of the full dock layout through the workspace-owned
    /// persistence file (mirrors the reference `onLayoutChanged` 300ms persist).
    pub fn persist_docks(&mut self, cx: &mut Context<Self>) {
        let now = std::time::Instant::now();
        if let Some(last) = self.last_dock_save {
            if now.duration_since(last) < Duration::from_millis(300) {
                return;
            }
        }
        self.last_dock_save = Some(now);
        let data: Vec<crate::dock::DockData> = self.docks().iter().map(|d| d.to_data()).collect();
        let snapshot = crate::layout::WorkspaceLayoutSnapshot::new(self.primary_dock(), data);
        crate::layout::save(&snapshot);
        let _ = cx;
    }

    /// One of the three edge docks (T17-002).
    pub fn dock(&self, position: labonair_panel::DockPosition) -> &crate::dock::Dock {
        use labonair_panel::DockPosition::*;
        match position {
            Left => &self.left_dock,
            Right => &self.right_dock,
            Bottom => &self.bottom_dock,
        }
    }

    /// Mutable access to one of the three edge docks.
    pub fn dock_mut(&mut self, position: labonair_panel::DockPosition) -> &mut crate::dock::Dock {
        use labonair_panel::DockPosition::*;
        match position {
            Left => &mut self.left_dock,
            Right => &mut self.right_dock,
            Bottom => &mut self.bottom_dock,
        }
    }

    /// `[left, right, bottom]`, for read-only iteration (status-bar toggles,
    /// persistence).
    pub fn docks(&self) -> [&crate::dock::Dock; 3] {
        [&self.left_dock, &self.right_dock, &self.bottom_dock]
    }

    /// Which dock currently holds the panel named `name`, if any.
    pub fn dock_of_panel(&self, name: &str) -> Option<labonair_panel::DockPosition> {
        labonair_panel::DockPosition::ALL
            .into_iter()
            .find(|p| self.dock(*p).has_panel(name))
    }

    /// Populate the docks from the [`PanelRegistry`](labonair_panel::PanelRegistry)
    /// and an optional persisted layout (`layout_json` is a JSON array of
    /// [`dock::DockData`](crate::dock::DockData); empty / invalid = first run).
    ///
    /// Each registered panel is built once here and placed in the dock its
    /// persisted `panel_order` names, else its registry `default_position`.
    pub fn init_docks(&mut self, layout_json: &str, window: &mut Window, cx: &mut App) {
        use crate::dock::{position_from_slug, DockData};

        let parsed: Vec<DockData> = serde_json::from_str(layout_json).unwrap_or_default();

        let regs: Vec<(
            &'static str,
            labonair_panel::DockPosition,
            labonair_panel::PanelConstructor,
        )> = self
            .panel_registry
            .iter()
            .map(|r| (r.persistent_name, r.default_position, r.build.clone()))
            .collect();

        for (name, default_pos, build) in regs {
            let handle = build(window, cx);
            let persisted = parsed
                .iter()
                .find(|d| d.panel_order.iter().any(|n| n == name))
                .and_then(|d| position_from_slug(&d.position))
                .filter(|pos| handle.position_is_valid(*pos, cx));
            // The Tabs panel is only docked when `tabsLocation == "sidebar"`;
            // its presence is driven by `sync_tabs_in_sidebar`, called right
            // after `init_docks` and on every settings change. Keep the handle
            // so it can be re-added at runtime, and only auto-place it here if
            // a persisted layout already had it in a dock.
            if name == "tabs" {
                self.tabs_panel = Some(handle.clone());
                if let Some(pos) = persisted {
                    self.dock_mut(pos).add_panel(handle);
                }
                continue;
            }
            let target = persisted.unwrap_or(default_pos);
            self.dock_mut(target).add_panel(handle);
        }

        for pos in labonair_panel::DockPosition::ALL {
            if let Some(data) = parsed
                .iter()
                .find(|d| position_from_slug(&d.position) == Some(pos))
            {
                let dock = self.dock_mut(pos);
                dock.apply_order(&data.panel_order);
                dock.apply_scalars(data);
            }
        }

        // First run (no persisted layout at all): open the left dock so the
        // explorer is visible, matching the pre-T17-002 default.
        if parsed.is_empty() {
            self.left_dock.set_open(true);
        }
    }

    /// Move the panel `name` to the dock at `to`, if the panel allows that
    /// position. Returns `true` when the move happened. (T17-002 — the UI that
    /// calls this lands in T18-007; a debug shortcut exercises it meanwhile.)
    pub fn move_panel(&mut self, name: &str, to: labonair_panel::DockPosition, cx: &App) -> bool {
        let Some(from) = self.dock_of_panel(name) else {
            return false;
        };
        if from == to {
            return true;
        }
        if !self.dock(from).panel_allows(name, to, cx) {
            return false;
        }
        let Some(handle) = self.dock_mut(from).remove_panel(name) else {
            return false;
        };
        let dest = self.dock_mut(to);
        dest.add_panel(handle);
        dest.activate_panel(name);
        dest.set_open(true);
        true
    }

    pub fn open_git_graph_tab(&mut self, cx: &mut Context<Self>) {
        let view = self.ensure_git_graph_view(cx);
        view.update(cx, |graph, cx| graph.clear_file_history(cx));
        let existing = self.find_singleton_tab(crate::pane_kind::PaneKind::GitGraph, cx);
        self.tabs.update(cx, |s, cx| {
            let id = match existing {
                Some(id) => {
                    s.set_active(id, cx);
                    id
                }
                None => s.open(TabKind::GitGraph, TabData::default(), cx),
            };
            s.set_custom_title(id, None, cx);
        });
    }

    /// Open the Git Graph tab scoped to the history of one local editor file.
    pub fn open_file_history_tab(&mut self, path: String, cx: &mut Context<Self>) {
        let graph = self.ensure_git_graph_view(cx);
        if let Err(error) = graph.update(cx, |graph, cx| graph.set_file_history(path.clone(), cx)) {
            labonair_notifications::notify_err::<()>("File History", Err(error), cx);
            return;
        }
        let title = Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| format!("History · {name}"))
            .unwrap_or_else(|| "File History".to_string());
        let existing = self.find_singleton_tab(crate::pane_kind::PaneKind::GitGraph, cx);
        self.tabs.update(cx, |tabs, cx| {
            let id = match existing {
                Some(id) => {
                    tabs.set_active(id, cx);
                    id
                }
                None => tabs.open(TabKind::GitGraph, TabData::default(), cx),
            };
            tabs.set_custom_title(id, Some(title), cx);
        });
    }

    /// Open (or focus) the single keymap-management tab. The keymap tab
    /// presents the full command catalog, so composition supplies the
    /// descriptor snapshot; `labonair-keymap-ui` renders it while the
    /// workspace owns the tab lifecycle (mirrors [`Self::open_git_graph_tab`]).
    pub fn open_keymap_tab(
        &mut self,
        descriptors: Vec<labonair_command_palette_core::CommandDescriptor>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.keymap.is_none() {
            let theme = self.theme.clone();
            let weak = cx.entity().downgrade();
            let open_raw: labonair_keymap_ui::OpenRawCallback = Box::new(move |window, cx| {
                let _ = weak.update(cx, |workspace, cx| {
                    workspace.open_or_create_user_keymap_json(window, cx);
                });
            });
            let view =
                cx.new(|cx| KeymapManagementView::new(theme, descriptors, open_raw, window, cx));
            cx.observe(&view, |_, _, cx| cx.notify()).detach();
            // Keep the surface current when `keymap.json` is edited outside it
            // (the raw editor tab, another editor, a sync). The keymap module
            // owns loading/merging; this only nudges the view to re-read.
            let view_for_watch = view.downgrade();
            labonair_settings::watch_file(
                cx,
                labonair_keymap::file::user_keymap_path(),
                move |cx| {
                    let _ = view_for_watch.update(cx, |view, cx| view.reload(cx));
                },
            );
            self.keymap = Some(view);
        }
        let existing = self.find_singleton_tab(crate::pane_kind::PaneKind::Keymap, cx);
        self.tabs.update(cx, |s, cx| match existing {
            Some(id) => s.set_active(id, cx),
            None => {
                s.open(TabKind::Keymap, TabData::default(), cx);
            }
        });
    }

    /// Ask the shell to open Settings on the Hosts page. Host management
    /// lives there; `self.hosts` is only the live host-data hub here.
    pub fn open_hosts_settings(&mut self, cx: &mut Context<Self>) {
        cx.emit(WorkspaceEvent::OpenHosts);
    }

    /// The host manager entity, which Settings embeds as its Hosts page.
    pub fn hosts_view(&self) -> Entity<HostManagerView> {
        self.hosts.clone()
    }

    /// Open — or, if it already exists, focus and re-point — the single
    /// workspace Project Diff item (Zed-parity Phase 4, §12.6). Idempotent:
    /// repeated requests never open a duplicate tab; a request carrying a new
    /// `selected` just moves the selection inside the existing item.
    pub fn open_project_diff(
        &mut self,
        req: labonair_panel::ProjectDiffRequest,
        cx: &mut Context<Self>,
    ) {
        let view = match &self.project_diff {
            Some(v) => v.clone(),
            None => {
                let theme = self.theme.clone();
                let git = self.git.clone();
                let tokio = self.tokio.clone();
                let v = cx.new(|cx| {
                    crate::views::project_diff::ProjectDiffView::new(theme, git, tokio, cx)
                });
                let workspace = cx.weak_entity();
                v.update(cx, |view, _| {
                    let open = workspace.clone();
                    view.set_on_open_file(move |path, window, cx| {
                        open.update(cx, |ws, cx| ws.open_file(path, false, window, cx))
                            .ok();
                    });
                    let close = workspace.clone();
                    view.set_on_close(move |window, cx| {
                        close
                            .update(cx, |ws, cx| ws.close_active(window, cx))
                            .ok();
                    });
                    view.set_on_commit(move |_window, cx| {
                        workspace
                            .update(cx, |ws, cx| ws.open_panel("source-control", cx))
                            .ok();
                    });
                });
                cx.observe(&v, |_, _, cx| cx.notify()).detach();
                cx.subscribe(&v, |_, _, event, cx| match event {
                    crate::views::project_diff::ProjectDiffEvent::CursorFile(path) => {
                        cx.emit(WorkspaceEvent::DiffCursorFile(path.clone()));
                    }
                })
                .detach();
                self.project_diff = Some(v.clone());
                v
            }
        };
        view.update(cx, |v, cx| v.apply_request(req, cx));

        let existing = self.find_singleton_tab(crate::pane_kind::PaneKind::Diff, cx);
        self.tabs.update(cx, |s, cx| match existing {
            Some(id) => s.set_active(id, cx),
            None => {
                s.open(TabKind::Diff, TabData::default(), cx);
            }
        });
    }

    /// Open the DiffView for `source` over the repository of the active
    /// workspace (`git: View Uncommitted / Unstaged / Staged Changes`).
    pub fn open_diff_source(&mut self, source: labonair_panel::DiffSource, cx: &mut Context<Self>) {
        let context = self.editor_git_bridge.context();
        let Some(root) = context.root.clone() else {
            labonair_notifications::notify_err::<()>(
                "Open Diff",
                Err("Git actions require an explicit repository workspace.".to_string()),
                cx,
            );
            return;
        };
        let git = self.git.clone();
        let session = context.session_id.clone();
        let task = self.tokio.spawn(async move {
            working_tree_diff_request(git, root, session, source, None).await
        });
        cx.spawn(async move |this, cx| {
            let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
            let _ = this.update(cx, |this, cx| match result {
                Ok(request) => this.open_project_diff(request, cx),
                Err(error) => {
                    labonair_notifications::notify_err::<()>("Open Diff", Err(error), cx);
                }
            });
        })
        .detach();
    }

    /// Compare the clipboard text (old side) with the editor selection (new
    /// side) in the DiffView.
    pub fn diff_clipboard_with_selection(&mut self, cx: &mut Context<Self>) {
        let Some(selection) = self
            .active_selection(cx)
            .filter(|(kind, _)| *kind == "editor")
            .map(|(_, text)| text)
        else {
            return;
        };
        let clipboard = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        self.open_project_diff(
            labonair_panel::ProjectDiffRequest {
                repo_root: String::new(),
                session_id: None,
                source: labonair_panel::DiffSource::Text {
                    title: "Clipboard \u{2194} Selection".to_string(),
                    old: clipboard,
                    new: selection,
                },
                files: Vec::new(),
                selected: None,
                mode: labonair_panel::ProjectDiffMode::Unified,
            },
            cx,
        );
    }

    /// Open the single-file diff of the active editor's file.
    pub fn open_active_file_diff(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self
            .tabs
            .read(cx)
            .active()
            .filter(|tab| tab.kind == TabKind::Editor)
            .and_then(|tab| tab.data.path.clone())
        else {
            return;
        };
        let context = self.editor_git_bridge.context();
        let Some(root) = context.root.clone() else {
            labonair_notifications::notify_err::<()>(
                "Open File Diff",
                Err("Git actions require an explicit repository workspace.".to_string()),
                cx,
            );
            return;
        };
        let git = self.git.clone();
        let session = context.session_id.clone();
        let task = self.tokio.spawn(async move {
            let mut request = working_tree_diff_request(
                git,
                root,
                session,
                labonair_panel::DiffSource::WorkingTree,
                Some(path),
            )
            .await?;
            let selected = request.selected.clone().unwrap_or_default();
            request.files.retain(|file| file.path == selected);
            request.source = labonair_panel::DiffSource::File { path: selected };
            Ok::<_, String>(request)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await.unwrap_or_else(|error| Err(error.to_string()));
            let _ = this.update(cx, |this, cx| match result {
                Ok(request) => this.open_project_diff(request, cx),
                Err(error) => {
                    labonair_notifications::notify_err::<()>("Open File Diff", Err(error), cx);
                }
            });
        })
        .detach();
    }

    /// Forward a keyboard / palette operation to the DiffView.
    pub fn perform_project_diff_action(
        &mut self,
        action: crate::views::project_diff::DiffAction,
        cx: &mut Context<Self>,
    ) {
        if let Some(view) = self.project_diff.clone() {
            view.update(cx, |view, cx| view.perform(action, cx));
        }
    }

    /// Close every tab except the active one (palette "Close Other Tabs").
    pub fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.tabs.read(cx).active_id();
        self.close_others(id, window, cx);
    }

    /// Open a fresh copy of the active tab (palette "Duplicate Tab").
    /// Terminal tabs re-spawn (inheriting cwd); editor tabs re-open the file.
    pub fn duplicate_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(active) = self.tabs.read(cx).active().cloned() else {
            return;
        };
        match active.kind {
            TabKind::Workspace => self.open_terminal_tab(window, cx),
            TabKind::Editor => {
                if let Some(path) = active.data.path.clone() {
                    self.open_file(path, false, window, cx);
                }
            }
            _ => {}
        }
    }

    /// Send the ANSI clear-screen sequence to the active terminal pane
    /// (palette "Clear Terminal").
    pub fn clear_active_terminal(&self, cx: &App) {
        self.inject_into_active_terminal("\x1b[2J\x1b[H", cx);
    }

    /// Tear down one pane's session + content view.
    fn retire_pane(&mut self, pane_id: PaneId) {
        if let Some(entry) = self.panes.remove(&pane_id) {
            self.registry.close(entry.session_id);
            // Drop this pane's persisted scrollback — a closed pane is never
            // restored (T14-002).
            labonair_terminal::scrollback::delete(
                &labonair_filesystem::paths::data_dir(),
                &entry.scrollback_id,
            );
        }
    }

    /// Closing a tab revokes any agent-access grant that followed it —
    /// backend (so the bridge stops exposing the session) and local mirror.
    fn revoke_tab_grant(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        if self.agent_access.read(cx).is_granted(tab_id) {
            self.agent_access.update(cx, |s, cx| {
                s.set_grant(
                    tab_id,
                    String::new(),
                    false,
                    String::new(),
                    SessionKind::Ssh,
                    None,
                    cx,
                );
            });
        }
    }

    /// Tear down a removed tab's whole pane tree: every pane retires its own
    /// session, view, document binding and connection.
    fn retire_tab(&mut self, tab: &Tab, cx: &mut Context<Self>) {
        self.revoke_tab_grant(tab.id, cx);
        self.zoom.clear(tab.id);
        let panes = self
            .layouts
            .remove(&tab.id)
            .map(|layout| layout.leaves())
            .unwrap_or_else(|| vec![tab.id]);
        for pane in panes {
            self.retire_any_pane(pane, cx);
        }
    }

    /// Disconnect the SSH connection that backs terminal session `session`.
    pub(crate) fn disconnect_ssh_session(&mut self, session: SessionId, cx: &mut Context<Self>) {
        let Some(t) = self.ssh_tabs.remove(&session) else {
            return;
        };
        self.ssh_connection
            .update(cx, |s, cx| s.remove(&t.ssh_id, cx));
        let mcp_access = self.mcp_access.clone();
        let ssh = self.ssh.clone();
        let tunnels = self.ssh_tunnels.clone();
        let ssh_id = t.ssh_id.clone();
        let host_id = t.host_id.clone();
        let tab_key = t.tab_id.to_string();
        self.tokio.spawn(async move {
            // Closing a tab revokes any MCP bridge grant that followed it.
            let _ = mcp_access
                .set_session_grant(SessionGrantRequest {
                    tab_id: tab_key,
                    session_id: String::new(),
                    granted: false,
                    label: String::new(),
                    kind: SessionKind::Ssh,
                    host_id: None,
                })
                .await;
            let _ = ssh.disconnect(SshSessionId::new(ssh_id)).await;
            let _ = tunnels.stop(host_id).await;
        });
        if let Some(p) = &self.ssh_prompt {
            if p.ssh_id() == t.ssh_id {
                self.ssh_prompt = None;
            }
        }
    }

    // ── SFTP browser (T08-001) ────────────────────────────────────────────

    /// Open (or re-focus) a dual-pane SFTP browser tab for `host_id`.
    fn open_sftp(&mut self, host_id: String, window: &mut Window, cx: &mut Context<Self>) {
        let existing = self.sftp_views.keys().copied().find(|id| {
            self.tabs
                .read(cx)
                .get(*id)
                .and_then(|t| t.data.host_id.clone())
                == Some(host_id.clone())
        });
        if let Some(tab_id) = existing {
            self.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
            self.focus_active(window, cx);
            return;
        }

        let session_id = uuid::Uuid::new_v4().to_string();
        let label = self
            .host_view
            .host_name(&host_id, cx)
            .unwrap_or_else(|| host_id.clone());
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Sftp,
                TabData {
                    host_id: Some(host_id.clone()),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(format!("SFTP \u{00b7} {label}")), cx);
            id
        });
        self.ssh_connection.update(cx, |s, cx| {
            s.begin(
                session_id.clone(),
                host_id.clone(),
                label.clone(),
                ConnectionKind::Sftp,
                None,
                cx,
            );
        });
        let view = self.new_sftp_view(session_id.clone(), host_id, label, cx);
        self.sftp_views.insert(tab_id, view);
        self.sftp_sessions.insert(tab_id, session_id);
        self.focus_active(window, cx);
        cx.notify();
    }

    fn on_sftp_event(&mut self, ev: &SftpEvent, cx: &mut Context<Self>) {
        match ev {
            SftpEvent::ConnResult { session_id, error } => {
                self.ssh_connection.update(cx, |s, cx| match error {
                    None => s.set_state(session_id, ConnectionState::Connected, cx),
                    Some(msg) => {
                        // Only surface the error screen if we never connected.
                        let connected = s
                            .get(session_id)
                            .map(|e| e.state == ConnectionState::Connected)
                            .unwrap_or(false);
                        if !connected {
                            s.set_error(session_id, msg.clone(), cx);
                        }
                    }
                });
            }
            SftpEvent::OpenLocalFile(path) => {
                self.pending_open.push(PendingOpen::Local(path.clone()));
                cx.notify();
            }
            SftpEvent::OpenRemoteFile {
                session_id,
                remote_path,
                host_id,
            } => {
                let service = self.ssh_remote_file.clone();
                let (sid, rpath, hid) = (session_id.clone(), remote_path.clone(), host_id.clone());
                let (jh_sid, jh_rpath) = (sid.clone(), rpath.clone());
                let jh = self.tokio.spawn(async move {
                    service
                        .prepare_remote_edit(SshSessionId::new(jh_sid), jh_rpath, None)
                        .await
                });
                cx.spawn(async move |this, cx| {
                    let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
                    let _ = this.update(cx, |this, cx| match res {
                        Ok(temp_path) => {
                            this.pending_open.push(PendingOpen::RemoteEdit {
                                session_id: sid,
                                remote_path: rpath,
                                host_id: hid,
                                temp_path,
                            });
                            cx.notify();
                        }
                        Err(e) => tracing::warn!(%e, "prepare_remote_edit failed"),
                    });
                })
                .detach();
            }
            SftpEvent::Enqueue {
                session_id,
                src_path,
                dest_path,
                direction,
            } => {
                let service = self.transfer_service.clone();
                let request = TransferRequest {
                    session_id: session_id.clone(),
                    src_path: src_path.clone(),
                    dest_path: dest_path.clone(),
                    direction: *direction,
                };
                self.tokio.spawn(async move {
                    if let Err(e) = service.enqueue(request).await {
                        tracing::warn!(%e, "enqueue_transfer failed");
                    }
                });
            }
            SftpEvent::OpenRemoteTerminal { host_id } => {
                self.pending_open.push(PendingOpen::SshTab(host_id.clone()));
                cx.notify();
            }
            SftpEvent::OpenLocalTerminal { cwd } => {
                self.pending_open
                    .push(PendingOpen::LocalTerminal(cwd.clone()));
                cx.notify();
            }
        }
    }

    /// Open an editor tab on a remote-edit temp copy and register it so a
    /// save uploads it back to the host.
    fn open_remote_edit(
        &mut self,
        session_id: String,
        remote_path: String,
        host_id: String,
        temp_path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Already open → just activate.
        if let Some(tab_id) = self
            .remote_edits
            .iter()
            .find(|(_, re)| re.temp_path == temp_path)
            .map(|(id, _)| *id)
        {
            self.tabs.update(cx, |s, cx| s.set_active(tab_id, cx));
            self.focus_active(window, cx);
            return;
        }

        let base = std::path::Path::new(&remote_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("file")
            .to_string();
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Editor,
                TabData {
                    path: Some(temp_path.clone()),
                    host_id: Some(host_id),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(format!("{base} (remote)")), cx);
            id
        });
        let view = self.new_editor_view(tab_id, cx);
        self.watch_editor(tab_id, &view, cx);
        view.update(cx, |e, cx| {
            e.open_path(std::path::PathBuf::from(&temp_path), cx)
        });
        self.editors.insert(tab_id, view);
        self.remote_edits.insert(
            tab_id,
            RemoteEdit {
                session_id,
                remote_path,
                temp_path,
                dirty: false,
            },
        );
        self.focus_active(window, cx);
        cx.notify();
    }

    /// Refresh the SFTP pane affected by a completed transfer. Transfer
    /// lifecycle state belongs to the transfers module; Workspace only owns
    /// the tab-local view refresh because it owns SFTP tab entities.
    pub fn refresh_sftp_after_transfer(
        &mut self,
        session_id: &str,
        direction: TransferDirection,
        cx: &mut Context<Self>,
    ) {
        let remote = matches!(direction, TransferDirection::Upload);
        let view = self
            .sftp_views
            .values()
            .find(|view| view.read(cx).session_id() == session_id)
            .cloned();
        if let Some(view) = view {
            view.update(cx, |view, cx| view.reload_side(remote, cx));
        }
    }

    // ── SSH connection flow (T07-001) ──────────────────────────────────────

    fn set_host_status(&mut self, host_id: &str, status: HostStatus, cx: &mut Context<Self>) {
        self.host_view.set_status(host_id, status, cx);
    }

    /// Push a completed [`TabOpResult`] back to a pending MCP `open_tab` /
    /// `close_tab` tool call waiting on its `oneshot` in `labonair-mcp-server`.
    fn respond_mcp_tab_op(&self, request_id: String, result: TabOpResult) {
        let mcp_tab_operations = self.mcp_tab_operations.clone();
        self.tokio.spawn(async move {
            let _ = mcp_tab_operations
                .respond_tab_operation(request_id, result)
                .await;
        });
    }

    /// Handle an MCP `open_tab` request: open a real SSH tab to `host_id`,
    /// auto-grant the bridge access to it (the agent explicitly asked for it),
    /// and answer the pending tool call.
    fn mcp_open_tab(
        &mut self,
        request_id: String,
        host_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let label = match self.host_view.host_name(&host_id, cx) {
            Some(name) => name,
            None => {
                self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some(format!("host '{host_id}' not found")),
                        ..Default::default()
                    },
                );
                return;
            }
        };
        let tab = self
            .connect_host(host_id.clone(), window, cx)
            .and_then(|ssh_id| {
                self.ssh_tabs
                    .values()
                    .find(|t| t.ssh_id == ssh_id)
                    .map(|t| (ssh_id.clone(), t.tab_id))
            });
        let Some((ssh_id, tab_id)) = tab else {
            self.respond_mcp_tab_op(
                request_id,
                TabOpResult {
                    ok: false,
                    error: Some("failed to open tab".to_string()),
                    ..Default::default()
                },
            );
            return;
        };
        let tab_id_str = tab_id.to_string();
        let mcp_access = self.mcp_access.clone();
        let mcp_tab_operations = self.mcp_tab_operations.clone();
        self.tokio.spawn(async move {
            let result = match mcp_access
                .set_session_grant(SessionGrantRequest {
                    tab_id: tab_id_str.clone(),
                    session_id: ssh_id.clone(),
                    granted: true,
                    label,
                    kind: SessionKind::Ssh,
                    host_id: Some(host_id),
                })
                .await
            {
                Ok(()) => TabOpResult {
                    ok: true,
                    session_id: Some(ssh_id),
                    tab_id: Some(tab_id_str),
                    error: None,
                },
                Err(error) => TabOpResult {
                    ok: false,
                    error: Some(error),
                    ..Default::default()
                },
            };
            let _ = mcp_tab_operations
                .respond_tab_operation(request_id, result)
                .await;
        });
    }

    /// Handle an MCP `close_tab` request: close the SSH tab whose transport
    /// session matches `session_id` (its grant is revoked by `retire_tab`).
    fn mcp_close_tab(
        &mut self,
        request_id: String,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(tab_id) = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == session_id)
            .map(|t| t.tab_id)
        else {
            self.respond_mcp_tab_op(
                request_id,
                TabOpResult {
                    ok: false,
                    error: Some("no tab found for that session_id".to_string()),
                    ..Default::default()
                },
            );
            return;
        };
        self.do_close(tab_id, window, cx);
        let result = if self.tabs.read(cx).get(tab_id).is_some() {
            TabOpResult {
                ok: false,
                error: Some("tab could not be closed".to_string()),
                ..Default::default()
            }
        } else {
            TabOpResult {
                ok: true,
                ..Default::default()
            }
        };
        self.respond_mcp_tab_op(request_id, result);
    }

    /// Open an SSH terminal tab for `host_id` and start the connection. Returns
    /// the backend SSH session id (`ssh_id`) of the new tab, or `None` if the
    /// terminal session could not be created.
    fn connect_host(
        &mut self,
        host_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let (ssh_id, session_id, feed, handle) = self.new_ssh_session(&host_id, cx)?;
        let (host_label, jump_label) = (
            self.host_view
                .host_name(&host_id, cx)
                .unwrap_or_else(|| host_id.clone()),
            self.host_view.jump_host_label(&host_id, cx),
        );
        let tab_title = ssh_tab_title(&host_label, jump_label.as_deref());
        let pane_id = self.alloc_pane();
        let tab_id = self.tabs.update(cx, |s, cx| {
            let id = s.open(
                TabKind::Workspace,
                TabData {
                    session_id: Some(session_id),
                    host_id: Some(host_id.clone()),
                    ..TabData::default()
                },
                cx,
            );
            s.set_custom_title(id, Some(tab_title.clone()), cx);
            id
        });
        self.layouts.insert(tab_id, WorkspaceLayout::new(pane_id));
        self.attach_ssh_terminal(
            host_id,
            ssh_id.clone(),
            session_id,
            feed,
            handle,
            pane_id,
            tab_id,
            window,
            cx,
        );
        Some(ssh_id)
    }

    /// Create the remote terminal session for `host_id` (registry session plus
    /// the PTY writer / resizer bridges). Nothing is connected yet.
    pub(crate) fn new_ssh_session(
        &mut self,
        _host_id: &str,
        cx: &mut Context<Self>,
    ) -> Option<(String, SessionId, RemoteFeed, SessionHandle)> {
        let ssh_id = uuid::Uuid::new_v4().to_string();
        let colors = self.theme_colors(cx);
        let dims = TermDimensions::new(80, 24);

        let writer: RemoteWriter = {
            let (id, service, tk) = (ssh_id.clone(), self.ssh_pty.clone(), self.tokio.clone());
            Arc::new(move |bytes: Vec<u8>| {
                let (id, service) = (id.clone(), service.clone());
                tk.spawn(async move {
                    let _ = service
                        .write(
                            SshSessionId::new(id),
                            String::from_utf8_lossy(&bytes).into_owned(),
                        )
                        .await;
                });
            })
        };
        let resizer: RemoteResizer = {
            let (id, service, tk) = (ssh_id.clone(), self.ssh_pty.clone(), self.tokio.clone());
            Arc::new(move |cols: u16, rows: u16| {
                let (id, service) = (id.clone(), service.clone());
                tk.spawn(async move {
                    let _ = service
                        .resize(SshSessionId::new(id), cols as u32, rows as u32)
                        .await;
                });
            })
        };

        let (session_id, feed) = self.registry.create_remote(colors, dims, writer, resizer);
        let handle = self.registry.handle(session_id)?;
        Some((ssh_id, session_id, feed, handle))
    }

    /// Show `session_id` in terminal pane `pane_id` of `tab_id` and start the
    /// connection (loading screen, prompts and tunnels follow from there).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn attach_ssh_terminal(
        &mut self,
        host_id: String,
        ssh_id: String,
        session_id: SessionId,
        feed: RemoteFeed,
        handle: SessionHandle,
        pane_id: PaneId,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (host_label, jump_label) = (
            self.host_view
                .host_name(&host_id, cx)
                .unwrap_or_else(|| host_id.clone()),
            self.host_view.jump_host_label(&host_id, cx),
        );
        let view = self.new_terminal_view(handle, window, cx);
        self.panes.insert(
            pane_id,
            PaneEntry {
                session_id,
                view,
                // SSH panes are not persisted with scrollback (T14-002).
                scrollback_id: uuid::Uuid::new_v4().to_string(),
            },
        );
        self.ssh_tabs.insert(
            session_id,
            SshTab {
                ssh_id: ssh_id.clone(),
                host_id: host_id.clone(),
                feed: feed.clone(),
                tab_id,
                pane_id,
            },
        );
        self.set_host_status(&host_id, HostStatus::Connecting, cx);
        self.ssh_connection.update(cx, |s, cx| {
            s.begin(
                ssh_id.clone(),
                host_id.clone(),
                host_label.clone(),
                ConnectionKind::Terminal,
                jump_label.clone(),
                cx,
            );
        });
        self.spawn_ssh_connect(ssh_id, host_id, None, None, feed, cx);
        self.focus_active(window, cx);
        cx.notify();
    }

    /// (Re)run `ssh_connect` for `ssh_id`, streaming remote output into `feed`.
    fn spawn_ssh_connect(
        &self,
        ssh_id: String,
        host_id: String,
        passphrase: Option<String>,
        password: Option<String>,
        feed: RemoteFeed,
        cx: &mut Context<Self>,
    ) {
        let ssh = self.ssh.clone();
        let event_sink = Arc::new(WorkspaceSshEventSink(feed));
        let connect_id = ssh_id.clone();
        let conns = connections_settings(cx);
        let (connect_timeout_secs, keepalive_interval_secs, keepalive_max_failures) = (
            conns.connect_timeout_secs(),
            conns.keepalive_interval_secs(),
            conns.keepalive_max_failures(),
        );
        let jh = self.tokio.spawn(async move {
            ssh.connect(
                SshConnectRequest {
                    session_id: SshSessionId::new(connect_id),
                    host_id,
                    passphrase,
                    password,
                    initial_cols: Some(80),
                    initial_rows: Some(24),
                    blocks: false,
                    connect_timeout_secs: Some(connect_timeout_secs),
                    keepalive_interval_secs: Some(keepalive_interval_secs),
                    keepalive_max_failures: Some(keepalive_max_failures),
                },
                event_sink,
            )
            .await
            .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(err)) = jh.await {
                let low = err.to_lowercase();
                let expected_prompt = low.contains("passphrase")
                    || low.contains("auth")
                    || low.contains("trust")
                    || low.contains("host key");
                if !expected_prompt {
                    let _ = this.update(cx, |this, cx| {
                        labonair_notifications::notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                labonair_notifications::Notification::error(
                                    "SSH connection failed",
                                    "The SSH session could not be established.",
                                )
                                .source("ssh")
                                .details(err.clone())
                                .dedupe_key(format!("ssh:connect:{ssh_id}")),
                                cx,
                            );
                        });
                        this.ssh_connection
                            .update(cx, |s, cx| s.set_error(&ssh_id, err.clone(), cx));
                        if let Some(host) = this
                            .ssh_tabs
                            .values()
                            .find(|t| t.ssh_id == ssh_id)
                            .map(|t| t.host_id.clone())
                        {
                            this.set_host_status(&host, HostStatus::Failed, cx);
                        }
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Bring up this host's configured local port-forwards on a dedicated
    /// background SSH connection (ref-counted per host by the backend). Mirrors
    /// the reference app's `ssh_start_tunnels` call on `session_established`.
    fn start_tunnels(&self, host_id: &str, cx: &mut Context<Self>) {
        let tunnels = self.ssh_tunnels.clone();
        let hid = host_id.to_string();
        let jh = self.tokio.spawn(async move { tunnels.start(hid).await });
        cx.spawn(async move |this, cx| {
            if let Ok(Err(e)) = jh.await {
                tracing::warn!(%e, "failed to start SSH tunnels");
            }
            let _ = this.update(cx, |this, cx| this.refresh_active_tunnels(cx));
        })
        .detach();
    }

    /// Push the current set of running forwards into the host manager panel.
    fn refresh_active_tunnels(&self, cx: &mut Context<Self>) {
        let raw = self.ssh_tunnels.active();
        let rows: Vec<ActiveTunnelRow> = raw
            .into_iter()
            .map(|t| ActiveTunnelRow {
                host_label: self
                    .host_view
                    .host_name(&t.host_id, cx)
                    .unwrap_or(t.host_id),
                local_port: t.local_port,
                remote_host: t.remote_host,
                remote_port: t.remote_port,
            })
            .collect();
        self.host_view.set_active_tunnels(rows, cx);
    }

    fn retry_ssh(
        &mut self,
        ssh_id: &str,
        passphrase: Option<String>,
        password: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some((host_id, feed)) = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == ssh_id)
            .map(|t| (t.host_id.clone(), t.feed.clone()))
        else {
            return;
        };
        self.ssh_connection.update(cx, |s, cx| s.resume(ssh_id, cx));
        self.spawn_ssh_connect(ssh_id.to_string(), host_id, passphrase, password, feed, cx);
    }

    pub(crate) fn handle_ssh_event(&mut self, ev: SshConnectionEvent, cx: &mut Context<Self>) {
        match ev {
            SshConnectionEvent::ConnectLog {
                session_id,
                message,
            } => {
                self.ssh_connection
                    .update(cx, |s, cx| s.push_log(&session_id, &message, cx));
            }
            SshConnectionEvent::KnownHostsWarning {
                session_id,
                fingerprint,
                host,
                is_mismatch,
            } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_trust(&session_id, fingerprint.clone(), is_mismatch, cx)
                });
                let _ = (host, fingerprint, is_mismatch);
                self.ssh_prompt = Some(SshPrompt::Trust { ssh_id: session_id });
            }
            SshConnectionEvent::AuthRequired {
                session_id,
                prompt_message,
                is_2fa,
            } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_auth_prompt(&session_id, prompt_message.clone(), is_2fa, cx)
                });
                self.ssh_prompt = Some(SshPrompt::Password {
                    ssh_id: session_id,
                    buffer: String::new(),
                });
            }
            SshConnectionEvent::PassphraseRequired { session_id } => {
                self.ssh_connection
                    .update(cx, |s, cx| s.set_passphrase(&session_id, cx));
                self.ssh_prompt = Some(SshPrompt::Passphrase {
                    ssh_id: session_id,
                    buffer: String::new(),
                });
            }
            SshConnectionEvent::SessionEstablished { session_id, .. } => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_state(&session_id, ConnectionState::Connected, cx)
                });
                if let Some(host) = self
                    .ssh_tabs
                    .values()
                    .find(|t| t.ssh_id == session_id)
                    .map(|t| t.host_id.clone())
                {
                    self.set_host_status(&host, HostStatus::Connected, cx);
                    self.start_tunnels(&host, cx);
                }
                if self.ssh_prompt.as_ref().map(|p| p.ssh_id()) == Some(session_id.as_str()) {
                    self.ssh_prompt = None;
                }
                if let Some(command) = self.pending_snippet_ssh.remove(&session_id) {
                    if let Some(sid) = self
                        .ssh_tabs
                        .iter()
                        .find(|(_, t)| t.ssh_id == session_id)
                        .map(|(sid, _)| *sid)
                    {
                        if let Some(handle) = self.registry.handle(sid) {
                            let _ = handle.write(format!("{}\n", command.trim_end()).as_bytes());
                        }
                    }
                }
            }
            SshConnectionEvent::ConnectionLost { session_id } => {
                let known = self.ssh_connection.read(cx).get(&session_id).is_some();
                if let Some(host) =
                    self.ssh_tabs
                        .values()
                        .find(|t| t.ssh_id == session_id)
                        .map(|t| {
                            t.feed.mark_disconnected();
                            t.host_id.clone()
                        })
                {
                    self.set_host_status(&host, HostStatus::Failed, cx);
                }
                // Only surface the error screen if the connection never reached
                // the shell — a drop after `Connected` belongs in the terminal.
                if known {
                    self.ssh_connection.update(cx, |s, cx| {
                        let was_connected = s
                            .get(&session_id)
                            .map(|e| e.state == ConnectionState::Connected)
                            .unwrap_or(false);
                        if !was_connected {
                            labonair_notifications::notification_center(cx).update(
                                cx,
                                |center, cx| {
                                    center.push(
                                        labonair_notifications::Notification::error(
                                            "SSH connection lost",
                                            "The session ended before the shell became ready.",
                                        )
                                        .source("ssh")
                                        .details(
                                            "The connection was lost before the remote shell was ready.",
                                        )
                                        .dedupe_key(format!("ssh:lost:{session_id}")),
                                        cx,
                                    );
                                },
                            );
                            s.set_error(
                                &session_id,
                                "Connection lost before the session was ready.",
                                cx,
                            );
                        }
                    });
                }
            }
        }
        cx.notify();
    }

    pub(crate) fn handle_mcp_event(&mut self, ev: McpEvent, cx: &mut Context<Self>) {
        match ev {
            McpEvent::OpenTabRequest {
                request_id,
                host_id,
                ..
            } => match host_id {
                Some(host_id) => self.pending_mcp.push(McpTabOp::Open {
                    request_id,
                    host_id,
                }),
                None => self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some("open_tab requires a host_id".to_string()),
                        ..Default::default()
                    },
                ),
            },
            McpEvent::CloseTabRequest {
                request_id,
                session_id,
            } => match session_id {
                Some(session_id) => self.pending_mcp.push(McpTabOp::Close {
                    request_id,
                    session_id,
                }),
                None => self.respond_mcp_tab_op(
                    request_id,
                    TabOpResult {
                        ok: false,
                        error: Some("close_tab requires a session_id".to_string()),
                        ..Default::default()
                    },
                ),
            },
            McpEvent::ServerError { message } => {
                let center = labonair_notifications::notification_center(cx);
                center.update(cx, |c, cx| {
                    c.push_action_result(
                        labonair_notifications::Notification::error(
                            "AI Agent Bridge failed to start",
                            message,
                        ),
                        cx,
                    );
                });
            }
            McpEvent::GrantExpired { tab_id } => {
                // Auto-revoke sweep or a host's "Block AI Agent Access" flag
                // being switched on: Rust already dropped the grant, clear the
                // local mirror so the badge / context-menu checkbox catch up.
                if let Ok(id) = tab_id.parse::<u64>() {
                    self.agent_access.update(cx, |s, cx| s.clear_local(id, cx));
                }
            }
            McpEvent::Activity {
                label,
                action,
                detail,
            } => {
                tracing::debug!(%label, %action, %detail, "mcp agent activity");
                if self.agent_access.read(cx).notify_on_activity() {
                    let center = labonair_notifications::notification_center(cx);
                    center.update(cx, |c, cx| {
                        c.push(
                            labonair_notifications::Notification::info(
                                format!("Agent: {action} \u{2014} {label}"),
                                detail.clone(),
                            ),
                            cx,
                        );
                    });
                }
            }
        }
        cx.notify();
    }

    fn submit_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(prompt) = self.ssh_prompt.take() else {
            return;
        };
        match prompt {
            SshPrompt::Trust { ssh_id, .. } => {
                self.ssh_connection
                    .update(cx, |s, cx| s.resume(&ssh_id, cx));
                let ssh = self.ssh.clone();
                self.tokio.spawn(async move {
                    let _ = ssh.trust_host(SshSessionId::new(ssh_id), true).await;
                });
            }
            SshPrompt::Password { ssh_id, buffer, .. } => {
                self.retry_ssh(&ssh_id, None, Some(buffer), cx);
            }
            SshPrompt::Passphrase { ssh_id, buffer } => {
                self.retry_ssh(&ssh_id, Some(buffer), None, cx);
            }
        }
        cx.notify();
    }

    fn cancel_prompt(&mut self, cx: &mut Context<Self>) {
        match self.ssh_prompt.take() {
            Some(SshPrompt::Trust { ssh_id, .. }) => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_error(&ssh_id, "Host key was not trusted.", cx)
                });
                let ssh = self.ssh.clone();
                self.tokio.spawn(async move {
                    let _ = ssh.trust_host(SshSessionId::new(ssh_id), false).await;
                });
            }
            Some(SshPrompt::Password { ssh_id, .. } | SshPrompt::Passphrase { ssh_id, .. }) => {
                self.ssh_connection.update(cx, |s, cx| {
                    s.set_error(&ssh_id, "Authentication cancelled.", cx)
                });
            }
            None => {}
        }
        cx.notify();
    }

    fn on_prompt_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cancel_prompt(cx),
            "enter" => self.submit_prompt(cx),
            "backspace" => {
                if let Some(
                    SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. },
                ) = self.ssh_prompt.as_mut()
                {
                    buffer.pop();
                }
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                let ch = ks
                    .key_char
                    .clone()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                    .or_else(|| (key.chars().count() == 1).then(|| key.to_string()));
                if let (
                    Some(ch),
                    Some(SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. }),
                ) = (ch, self.ssh_prompt.as_mut())
                {
                    buffer.push_str(&ch);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// Full-pane SSH connecting screen (T16-015) — port of
    /// `reference-src/src/modules/terminal/SshLoadingScreen.tsx`. Shown in the
    /// tab body in place of the terminal until `session_established`, driving:
    /// the 4-stage progress indicator, the live connection-log panel, and the
    /// state-specific card (quick-connect password / trust / auth / passphrase
    /// / error with retry+edit-host).
    fn render_ssh_loading(
        &mut self,
        entry: crate::ssh_connection::ConnectionEntry,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let theme = self.theme.read(cx);
        let core = theme.theme().core.clone();
        let (bg, card, fg, border, accent, muted) = (
            theme.background(),
            theme.card(),
            theme.foreground(),
            theme.border(),
            theme.accent(),
            theme.muted_foreground(),
        );
        let c = Palette::from_theme(theme);
        let destructive = core.destructive;
        let ssh_id = entry.session_id.clone();
        let is_sftp = matches!(entry.kind, ConnectionKind::Sftp);
        let tab_id = self
            .ssh_tabs
            .values()
            .find(|t| t.ssh_id == ssh_id)
            .map(|t| t.pane_id)
            .or_else(|| {
                self.sftp_sessions
                    .iter()
                    .find(|(_, sid)| **sid == ssh_id)
                    .map(|(tid, _)| *tid)
            });
        let host_id = entry.host_id.clone();

        // ── 4-stage progress row ──────────────────────────────────────────
        let stage_count = ConnStage::ORDER.len();
        let stages =
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(ConnStage::ORDER.iter().enumerate().map(|(i, &stage)| {
                    let status = entry.stage_status(stage);
                    let (dot_bg, dot_fg, txt) = match status {
                        StageStatus::Done => (accent, bg, accent),
                        StageStatus::Active => (accent.opacity(0.2), accent, fg),
                        StageStatus::Pending => (gpui::transparent_black(), muted, muted),
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .size(px(18.0))
                                .rounded_full()
                                .border_1()
                                .border_color(if status == StageStatus::Pending {
                                    border
                                } else {
                                    accent
                                })
                                .bg(dot_bg)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(dot_fg)
                                .child(if status == StageStatus::Done {
                                    "\u{2713}".to_string()
                                } else {
                                    (i + 1).to_string()
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(txt)
                                .child(SharedString::from(stage.label(entry.kind))),
                        )
                        .when(i + 1 < stage_count, |d| {
                            d.child(div().w(px(16.0)).h(px(1.0)).bg(border))
                        })
                }));

        // ── state card ────────────────────────────────────────────────────
        let is_prompt = matches!(
            entry.state,
            ConnectionState::WaitingAuth
                | ConnectionState::WaitingPassphrase
                | ConnectionState::QuickConnectPassword
        );
        let buffer_dots = self
            .ssh_prompt
            .as_ref()
            .and_then(|p| match p {
                SshPrompt::Password { buffer, .. } | SshPrompt::Passphrase { buffer, .. } => {
                    Some("\u{2022}".repeat(buffer.chars().count()))
                }
                _ => None,
            })
            .unwrap_or_default();

        let (card_title, card_body): (String, String) = match &entry.state {
            ConnectionState::Error => (
                "Connection failed".to_string(),
                "See Notifications for connection details.".to_string(),
            ),
            ConnectionState::WaitingTrust => (
                if entry.trust_mismatch {
                    "Host key CHANGED".to_string()
                } else {
                    "Unknown host key".to_string()
                },
                format!(
                    "Fingerprint: {}\n\n{}",
                    entry.trust_fingerprint.as_deref().unwrap_or("(unknown)"),
                    if entry.trust_mismatch {
                        "The key differs from the one on record. Only continue if you know why."
                    } else {
                        "This host is not yet in known_hosts."
                    }
                ),
            ),
            ConnectionState::WaitingAuth => (
                if entry.is_2fa {
                    "Two-factor code required".to_string()
                } else {
                    "Password required".to_string()
                },
                format!(
                    "{}\n{}",
                    entry
                        .prompt_message
                        .as_deref()
                        .unwrap_or("Enter your password"),
                    buffer_dots
                ),
            ),
            ConnectionState::WaitingPassphrase | ConnectionState::QuickConnectPassword => (
                "Key passphrase".to_string(),
                format!("Enter the passphrase for the private key.\n{buffer_dots}"),
            ),
            _ => (
                format!("Connecting to {}\u{2026}", entry.host_label),
                "Negotiating a secure channel.".to_string(),
            ),
        };

        let mut actions = div().flex().gap_2().justify_end().pt_1();
        match entry.state {
            ConnectionState::Error => {
                actions = actions
                    .child(
                        loading_btn("ssh-l-close", "Close", c, false).on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, _cx| {
                                if let Some(id) = tab_id {
                                    this.pending_tab_close.push(id);
                                }
                            },
                        )),
                    )
                    .child(
                        loading_btn("ssh-l-edit", "Edit Host", c, false).on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| this.open_hosts_settings(cx),
                        )),
                    )
                    .child(
                        loading_btn("ssh-l-retry", "Retry", c, true).on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| {
                                this.ssh_prompt = None;
                                if is_sftp {
                                    this.ssh_connection
                                        .update(cx, |s, cx| s.resume(&ssh_id, cx));
                                    if let Some(tid) = tab_id {
                                        if let Some(view) = this.sftp_views.get(&tid).cloned() {
                                            view.update(cx, |v, cx| v.reconnect(cx));
                                        }
                                    }
                                } else {
                                    this.retry_ssh(&ssh_id, None, None, cx);
                                }
                            },
                        )),
                    );
                let _ = host_id;
            }
            ConnectionState::WaitingTrust => {
                actions = actions
                    .child(loading_btn("ssh-l-abort", "Abort", c, false).on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.cancel_prompt(cx)),
                    ))
                    .child(
                        loading_btn(
                            "ssh-l-trust",
                            if entry.trust_mismatch {
                                "Accept anyway"
                            } else {
                                "Trust & Connect"
                            },
                            c,
                            true,
                        )
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_prompt(cx)),
                        ),
                    );
            }
            _ if is_prompt => {
                actions = actions
                    .child(loading_btn("ssh-l-cancel", "Cancel", c, false).on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.cancel_prompt(cx)),
                    ))
                    .child(loading_btn("ssh-l-submit", "Submit", c, true).on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.submit_prompt(cx)),
                    ));
            }
            _ => {
                actions = actions.child(loading_btn("ssh-l-cancel2", "Cancel", c, false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.ssh_connection
                            .update(cx, |s, cx| s.set_error(&ssh_id, "Cancelled by user.", cx));
                    }),
                ));
            }
        }

        let card_border = if matches!(entry.state, ConnectionState::Error)
            || entry.trust_mismatch && entry.state == ConnectionState::WaitingTrust
        {
            destructive
        } else {
            border
        };

        let state_card = div()
            .track_focus(&self.prompt_focus)
            .key_context("SshPrompt")
            .on_key_down(cx.listener(Self::on_prompt_key))
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_4()
            .rounded_lg()
            .bg(card)
            .border_1()
            .border_color(card_border)
            .child(
                div()
                    .text_sm()
                    .text_color(fg)
                    .child(SharedString::from(card_title)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .whitespace_normal()
                    .child(SharedString::from(card_body)),
            )
            .child(actions);

        // ── live connection log ───────────────────────────────────────────
        let log_lines = entry.log.clone();
        let log_panel = div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            // T20-001: shared `h_stack` + `Indicator` primitives.
            .child(
                h_stack()
                    .gap_2()
                    .child(indicator(IndicatorSize::Xs, accent))
                    .child(div().text_xs().text_color(muted).child("Connection Log")),
            )
            .child(
                div()
                    .id("ssh-connect-log")
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .w_full()
                    .max_h(px(180.0))
                    .overflow_y_scroll()
                    .p_2()
                    .rounded_md()
                    .bg(c.card)
                    .border_1()
                    .border_color(border)
                    .font_family("monospace")
                    .text_size(px(11.0))
                    .text_color(muted)
                    .children(if log_lines.is_empty() {
                        vec![div()
                            .child("Waiting for the backend\u{2026}")
                            .into_any_element()]
                    } else {
                        log_lines
                            .iter()
                            .map(|l| {
                                div()
                                    .child(SharedString::from(l.clone()))
                                    .into_any_element()
                            })
                            .collect()
                    }),
            );

        let jump_badge = entry.jump_host_name.clone().map(|j| {
            div()
                .px_2()
                .py_0p5()
                .rounded_md()
                .border_1()
                .border_color(border)
                .text_xs()
                .text_color(muted)
                .child(SharedString::from(format!("via {j}")))
        });

        div()
            .size_full()
            .bg(bg)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .w(px(520.0))
                    .max_w_full()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(fg)
                                    .child(SharedString::from(entry.host_label.clone())),
                            )
                            .children(jump_badge),
                    )
                    .child(stages)
                    .child(state_card)
                    .child(log_panel),
            )
            .into_any_element()
    }

    fn sync_meta(&mut self, cx: &mut Context<Self>) {
        let updates: Vec<(u64, Option<String>, Option<String>)> = self
            .layouts
            .iter()
            .filter_map(|(tab_id, layout)| {
                let v = self.panes.get(&layout.active?)?.view.read(cx);
                let (cwd, title) = v.shell_meta();
                Some((*tab_id, cwd, title))
            })
            .collect();
        let preview_url = self
            .active_pane_view(cx)
            .and_then(|view| view.read(cx).preview_url());
        let preview_changed = self.preview_url_cache.as_ref() != Some(&preview_url);
        self.preview_url_cache = Some(preview_url);
        self.tabs.update(cx, |store, cx| {
            for (id, cwd, title) in updates {
                store.sync_workspace_meta(id, cwd, title, cx);
            }
        });
        self.sync_pane_meta(cx);
        if preview_changed {
            cx.notify();
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let active_space = self.spaces.read(cx).active_id();
        let active = self
            .tabs
            .read(cx)
            .active()
            .filter(|tab| tab.space_id == active_space)
            .cloned();
        let Some(active) = active else {
            return self.render_empty_surface(cx).into_any_element();
        };
        self.render_pane_area(active.id, window, cx)
    }

    fn placeholder(&self, title: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.background())
            .text_color(theme.muted_foreground())
            .text_sm()
            .child(SharedString::from(format!(
                "{title} — coming in a later phase"
            )))
    }

    /// The empty-workspace surface, shown when no tabs are open. Double-click
    /// opens a local terminal; dropping files opens an editor tab for each.
    fn render_empty_surface(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = self.theme.read(cx).background();

        div()
            .id("empty-workspace")
            .size_full()
            .bg(bg)
            .on_click(cx.listener(|this, ev: &ClickEvent, window, cx| {
                if ev.click_count() >= 2 {
                    this.new_terminal_tab(window, cx);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                for path in paths.paths() {
                    if let Some(p) = path.to_str() {
                        this.open_file(p.to_string(), false, window, cx);
                    }
                }
            }))
    }

    fn render_confirm(&mut self, id: u64, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme.read(cx);
        let (card, fg, border) = (theme.card(), theme.foreground(), theme.border());
        let c = Palette::from_theme(theme);
        let label = self
            .tabs
            .read(cx)
            .get(id)
            .map(Tab::label)
            .unwrap_or_default();

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
                    .p_4()
                    .rounded_lg()
                    .bg(card)
                    .border_1()
                    .border_color(border)
                    .text_color(fg)
                    .child(SharedString::from(format!(
                        "Discard unsaved changes to \u{201c}{label}\u{201d}?"
                    )))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                labonair_ui_kit::button(
                                    "confirm-cancel",
                                    c,
                                    ButtonVariant::Subtle,
                                    ButtonSize::Sm,
                                )
                                .child("Cancel")
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, window, cx| {
                                        this.cancel_pending_close(window, cx);
                                    },
                                )),
                            )
                            .child(
                                labonair_ui_kit::button(
                                    "confirm-discard",
                                    c,
                                    ButtonVariant::TintedError,
                                    ButtonSize::Sm,
                                )
                                .child("Discard")
                                .on_click(cx.listener(
                                    move |this, _: &ClickEvent, window, cx| {
                                        this.confirm_close(id, window, cx);
                                    },
                                )),
                            ),
                    ),
            )
    }

    /// Start editing a tab's title inline (from the tab context menu).
    fn begin_tab_rename(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tab_menu(cx);
        let current = self
            .tabs
            .read(cx)
            .get(id)
            .map(|t| t.label().to_string())
            .unwrap_or_default();
        self.space_rename = None;
        self.rename_tab = Some((id, current));
        window.focus(&self.rename_focus);
        cx.notify();
    }

    fn commit_tab_rename(&mut self, cx: &mut Context<Self>) {
        if let Some((id, buf)) = self.rename_tab.take() {
            let trimmed = buf.trim();
            let title = (!trimmed.is_empty()).then(|| trimmed.to_string());
            self.tabs
                .update(cx, |s, cx| s.set_custom_title(id, title, cx));
        }
        cx.notify();
    }

    fn on_rename_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, buf)) = self.rename_tab.as_mut() else {
            return;
        };
        match ev.keystroke.key.as_str() {
            "enter" => self.commit_tab_rename(cx),
            "escape" => {
                self.rename_tab = None;
                cx.notify();
            }
            "backspace" => {
                buf.pop();
                self.rename_blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    buf.push_str(ch);
                    self.rename_blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
    }

    /// Start editing a space's name inline (Spaces popover / Tabs sidebar
    /// header). Mutually exclusive with tab-rename — both share
    /// `rename_focus`/`rename_blink`.
    fn begin_space_rename(&mut self, id: SpaceId, window: &mut Window, cx: &mut Context<Self>) {
        let current = self
            .spaces
            .read(cx)
            .get(id)
            .map(|s| s.name.clone())
            .unwrap_or_default();
        self.rename_tab = None;
        self.space_rename = Some((id, current));
        window.focus(&self.rename_focus);
        cx.notify();
    }

    fn commit_space_rename(&mut self, cx: &mut Context<Self>) {
        if let Some((id, buf)) = self.space_rename.take() {
            let trimmed = buf.trim();
            if !trimmed.is_empty() {
                self.spaces.update(cx, |s, cx| s.rename(id, trimmed, cx));
            }
        }
        cx.notify();
    }

    fn on_space_rename_key(
        &mut self,
        ev: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, buf)) = self.space_rename.as_mut() else {
            return;
        };
        match ev.keystroke.key.as_str() {
            "enter" => self.commit_space_rename(cx),
            "escape" => {
                // Cancel the rename only; the popover hosting the field stays.
                self.space_rename = None;
                cx.stop_propagation();
                cx.notify();
            }
            "backspace" => {
                buf.pop();
                self.rename_blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    buf.push_str(ch);
                    self.rename_blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
    }

    /// `Recent Projects` entries matching the New Space dropdown's search
    /// buffer (case-insensitive substring on the display name; empty buffer
    /// matches everything).
    fn filtered_recent_projects(&self) -> Vec<recent_projects::RecentProject> {
        let query = self.new_space_search.trim().to_lowercase();
        self.recent_projects
            .entries()
            .iter()
            .filter(|e| query.is_empty() || e.name.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }

    fn on_new_space_search_key(
        &mut self,
        ev: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.new_space_menu.is_open() {
            return;
        }
        match ev.keystroke.key.as_str() {
            "escape" => self.close_new_space_menu(cx),
            "enter" => {
                if let Some(first) = self.filtered_recent_projects().into_iter().next() {
                    self.close_new_space_menu(cx);
                    self.open_or_create_project_space(first.root, cx);
                }
            }
            "backspace" => {
                self.new_space_search.pop();
                self.new_space_search_blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            _ => {
                if let Some(ch) = ev
                    .keystroke
                    .key_char
                    .as_ref()
                    .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
                {
                    self.new_space_search.push_str(ch);
                    self.new_space_search_blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
    }

    /// Resolve the MCP agent-access grant target for a tab, when it's an SSH or
    /// local terminal tab and the bridge is enabled.
    fn mcp_grant_target(&self, tab_id: u64, cx: &mut Context<Self>) -> Option<McpGrantTarget> {
        if !self.agent_access.read(cx).bridge_enabled() {
            return None;
        }
        let tab = self.tabs.read(cx).get(tab_id)?.clone();
        let label = tab.label();
        if let Some(ssh) = self.ssh_tabs.values().find(|t| t.tab_id == tab_id) {
            return Some((
                ssh.ssh_id.clone(),
                label,
                SessionKind::Ssh,
                Some(ssh.host_id.clone()),
            ));
        }
        if tab.kind == TabKind::Workspace {
            if let Some(sid) = tab.data.session_id {
                return Some((sid.to_string(), label, SessionKind::Local, None));
            }
        }
        None
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Value carried while dragging a dock's edge handle — the position tells the
/// drop handler which dock to resize and along which axis. Moved off `AppShell`
/// in T17-006 together with [`Workspace::render_dock`].
#[derive(Clone, Copy)]
struct DockResize(labonair_panel::DockPosition);

impl Workspace {
    /// Resize the dock at `pos`, clamped by the active panel's `min_size` and
    /// the dock's own bounds, then persist. Ported verbatim off `AppShell`
    /// (T17-006).
    fn set_dock_size(
        &mut self,
        pos: labonair_panel::DockPosition,
        size: f32,
        cx: &mut Context<Self>,
    ) {
        let floor = self.dock(pos).active_panel().and_then(|p| p.min_size(cx));
        let changed = {
            let dock = self.dock_mut(pos);
            let before = dock.size();
            dock.set_size(px(size), floor);
            (f32::from(dock.size()) - f32::from(before)).abs() > 0.5
        };
        if changed {
            self.persist_docks(cx);
            cx.notify();
        }
    }

    /// Apply one queued [`LiveCommand`] from the AI live-bridge to the active
    /// terminal (T17-006 — replaces `AppShell::sync_live_bridge`'s per-frame
    /// `drain_commands` loop). Queued commands only ever exist while a terminal
    /// is active (the bridge gates `enqueue` on `has_terminal`), so no
    /// new-tab / `&mut Window` path is needed here.
    pub fn apply_live_command(&mut self, cmd: LiveCommand, cx: &mut Context<Self>) {
        let Some(view) = self.active_pane_view(cx) else {
            return;
        };
        let payload = if cmd.execute {
            format!("{}\n", cmd.text.trim_end())
        } else {
            cmd.text.clone()
        };
        let _ = view.read(cx).handle().write(payload.as_bytes());
        cx.notify();
    }

    /// Reset the dock at `pos` to its default size (bound double-click on the
    /// resize handle — `docs/ui-comparison-zed-sidebar-status-bar.md` §7.4).
    fn reset_dock_size(&mut self, pos: labonair_panel::DockPosition, cx: &mut Context<Self>) {
        let floor = self.dock(pos).active_panel().and_then(|p| p.min_size(cx));
        let default = crate::dock::default_size(pos);
        let changed = {
            let dock = self.dock_mut(pos);
            let before = dock.size();
            dock.set_size(default, floor);
            (f32::from(dock.size()) - f32::from(before)).abs() > 0.5
        };
        if changed {
            self.persist_docks(cx);
            cx.notify();
        }
    }

    /// Render one edge dock (T17-002): the active panel's body filling the dock,
    /// a 1px structural border on the workspace-facing edge, and an *overlaid*
    /// resize hit target straddling that border (no generic title/move header —
    /// panel movement lives on the status-bar button's context menu). Left/right
    /// docks are width-resizable; the bottom dock is height-resizable.
    /// Double-clicking the handle restores the default size. A zoomed dock fills
    /// its axis and drops the handle.
    /// (`docs/ui-comparison-zed-sidebar-status-bar.md` §7.4.)
    fn render_dock(
        &mut self,
        pos: labonair_panel::DockPosition,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        use labonair_panel::DockPosition;

        let (sidebar_bg, sidebar_fg, sidebar_border) = {
            let theme = self.theme.read(cx);
            (
                theme.sidebar_bg(),
                theme.sidebar_fg(),
                theme.sidebar_border(),
            )
        };

        let is_bottom = pos == DockPosition::Bottom;
        let (size, zoomed, body) = {
            let dock = self.dock(pos);
            let body: Option<gpui::AnyElement> = dock
                .active_panel()
                .map(|handle| handle.to_any().into_any_element());
            (f32::from(dock.size()), dock.is_zoomed(), body)
        };

        // 6px transparent hit target, absolutely positioned so it does not
        // consume layout width/height; it straddles the 1px structural border.
        let handle = (!zoomed).then(|| {
            div()
                .id(SharedString::from(format!(
                    "dock-handle-{}",
                    position_slug(pos)
                )))
                .absolute()
                .when(!is_bottom, |d| {
                    d.top(px(0.0))
                        .bottom(px(0.0))
                        .w(RESIZE_HANDLE_SIZE)
                        .cursor_col_resize()
                })
                .when(is_bottom, |d| {
                    d.left(px(0.0))
                        .right(px(0.0))
                        .h(RESIZE_HANDLE_SIZE)
                        .cursor_row_resize()
                })
                .when(pos == DockPosition::Left, |d| d.right(px(-3.0)))
                .when(pos == DockPosition::Right, |d| d.left(px(-3.0)))
                .when(is_bottom, |d| d.top(px(-3.0)))
                .on_drag(DockResize(pos), |_, _, _, cx| cx.new(|_| DragGhost))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                        if ev.click_count >= 2 {
                            this.reset_dock_size(pos, cx);
                        }
                        cx.stop_propagation();
                    }),
                )
        });

        div()
            .relative()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .min_h_0()
            .when(!zoomed && !is_bottom, |d| d.w(px(size)))
            .when(!zoomed && is_bottom, |d| d.h(px(size)))
            .when(zoomed, |d| d.flex_1())
            .when(!is_bottom, |d| d.h_full())
            .when(is_bottom, |d| d.w_full())
            .bg(sidebar_bg)
            .text_color(sidebar_fg)
            .when(pos == DockPosition::Left, |d| {
                d.border_r_1().border_color(sidebar_border)
            })
            .when(pos == DockPosition::Right, |d| {
                d.border_l_1().border_color(sidebar_border)
            })
            .when(is_bottom, |d| d.border_t_1().border_color(sidebar_border))
            .children(body)
            .children(handle)
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span =
            tracing::trace_span!(target: "labonair::perf", "render", view = "workspace").entered();
        self.drain_editor_git_actions(cx);
        // T19-003: keep `labonair-settings`'s active project root in sync
        // with the explicit workspace identity (cheap no-op unless it
        // changed — see the method doc).
        // Drain host-manager connect requests here — `connect_host` needs a
        // `&mut Window` and `cx.subscribe` does not provide one.
        if !self.pending_connect.is_empty() {
            for host_id in std::mem::take(&mut self.pending_connect) {
                let _ = self.connect_host(host_id, window, cx);
            }
        }
        if !self.pending_mcp.is_empty() {
            for op in std::mem::take(&mut self.pending_mcp) {
                match op {
                    McpTabOp::Open {
                        request_id,
                        host_id,
                    } => self.mcp_open_tab(request_id, host_id, window, cx),
                    McpTabOp::Close {
                        request_id,
                        session_id,
                    } => self.mcp_close_tab(request_id, session_id, window, cx),
                }
            }
        }
        if !self.pending_sftp.is_empty() {
            for host_id in std::mem::take(&mut self.pending_sftp) {
                self.open_sftp(host_id, window, cx);
            }
        }
        if !self.pending_tab_close.is_empty() {
            for id in std::mem::take(&mut self.pending_tab_close) {
                if self.tabs.read(cx).get(id).is_some() {
                    self.request_close(id, window, cx);
                } else {
                    self.close_pane_id(id, window, cx);
                }
            }
        }
        if !self.pending_open.is_empty() {
            for open in std::mem::take(&mut self.pending_open) {
                match open {
                    PendingOpen::Local(path) => self.open_file(path, false, window, cx),
                    PendingOpen::RemoteEdit {
                        session_id,
                        remote_path,
                        host_id,
                        temp_path,
                    } => self.open_remote_edit(
                        session_id,
                        remote_path,
                        host_id,
                        temp_path,
                        window,
                        cx,
                    ),
                    PendingOpen::SshTab(host_id) => self.open_ssh_tab(host_id, window, cx),
                    PendingOpen::LocalTerminal(cwd) => {
                        self.new_terminal_tab_in(Some(cwd), window, cx)
                    }
                }
            }
        }
        if std::mem::take(&mut self.pending_preview_focus) {
            self.focus_active(window, cx);
        }
        let want_prompt = self.ssh_prompt.is_some();
        if want_prompt && !self.prompt_shown {
            window.focus(&self.prompt_focus);
        }
        self.prompt_shown = want_prompt;

        let content = self.render_content(window, cx);
        let confirm = self
            .confirm_close
            .map(|id| self.render_confirm(id, cx).into_any_element());
        self.tick_menus(window, cx);
        let context_menu = self
            .context_menu
            .mounted()
            .map(|(&(id, pos), closing)| (id, pos, closing))
            .map(|(id, pos, closing)| {
                self.render_context_menu(id, pos, closing, cx)
                    .into_any_element()
            });
        let new_tab_menu = self
            .new_tab_menu
            .mounted()
            .map(|(&pos, closing)| (pos, closing))
            .map(|(pos, closing)| {
                self.render_new_tab_menu(pos, closing, cx)
                    .into_any_element()
            });
        let tab_hover = self.render_tab_hover(cx);
        let spaces_menu = self
            .spaces_menu
            .mounted()
            .map(|(&pos, closing)| (pos, closing))
            .map(|(pos, closing)| self.render_spaces_popover(pos, closing, cx));
        let space_actions_menu = self
            .space_actions_menu
            .mounted()
            .map(|(&(id, pos), closing)| (id, pos, closing))
            .map(|(id, pos, closing)| self.render_space_actions_menu(id, pos, closing, cx));
        let new_space_menu = self
            .new_space_menu
            .mounted()
            .map(|(&pos, closing)| (pos, closing))
            .map(|(pos, closing)| self.render_new_space_menu(pos, closing, cx));

        // T17-006: the three edge docks + the drag-to-resize handler used to
        // live in `AppShell::render`; they compose here now so the shell only
        // has to `.child(workspace.clone())`.
        use labonair_panel::DockPosition;
        let left_dock = self.render_dock_slot(DockPosition::Left, window, cx);
        let right_dock = self.render_dock_slot(DockPosition::Right, window, cx);
        let bottom_dock = self.render_dock_slot(DockPosition::Bottom, window, cx);
        let center = div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().min_w_0().child(content))
            .children(bottom_dock);
        let docked = div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_row()
            .children(left_dock)
            .child(center)
            .children(right_dock)
            .on_drag_move(
                cx.listener(|this, ev: &DragMoveEvent<DockResize>, _window, cx| {
                    let pos = ev.drag(cx).0;
                    let b = ev.bounds;
                    let p = ev.event.position;
                    let size = match pos {
                        DockPosition::Left => f32::from(p.x - b.origin.x),
                        DockPosition::Right => f32::from(b.origin.x + b.size.width - p.x),
                        DockPosition::Bottom => f32::from(b.origin.y + b.size.height - p.y),
                    };
                    this.set_dock_size(pos, size, cx);
                }),
            );

        div()
            .track_focus(&self.focus_handle)
            .key_context("Workspace")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .on_key_down(cx.listener(Self::on_key_down))
            .capture_key_down(cx.listener(Self::cancel_drag_on_escape))
            .child(docked)
            .children(confirm)
            .children(context_menu)
            .children(new_tab_menu)
            .children(tab_hover)
            .children(spaces_menu)
            .children(space_actions_menu)
            .children(new_space_menu)
    }
}

impl Workspace {
    fn on_key_down(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        if self.confirm_close.is_some() && ks.key == "escape" {
            self.cancel_pending_close(window, cx);
            cx.stop_propagation();
        }
    }
}

/// POSIX single-quote a path for a `cd` command (breadcrumb navigation).
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Maps a [`TabKind`] onto the palette crate's owned tab-kind enum so
/// `crates/ui` never passes its `TabKind` across the crate boundary (T16-004).
fn palette_tab_kind(kind: TabKind) -> labonair_command_palette::PaletteTabKind {
    use labonair_command_palette::PaletteTabKind as K;
    match kind {
        TabKind::Workspace => K::Workspace,
        TabKind::Editor => K::Editor,
        TabKind::Preview => K::Preview,
        TabKind::Diff => K::Diff,
        TabKind::Sftp => K::Sftp,
        _ => K::Other,
    }
}

/// Bridges [`Workspace`] to the `labonair-command-palette` view's
/// [`PaletteWorkspace`](labonair_command_palette::PaletteWorkspace) contract
/// (T16-004 decoupling). Both accessors mirror what the palette view read
/// directly from `Workspace` before the split.
impl labonair_command_palette::PaletteWorkspace for Workspace {
    fn palette_active_context(&self, cx: &App) -> Option<labonair_command_palette::CommandContext> {
        self.active_context(cx)
    }

    fn palette_tab_rows(&self, cx: &App) -> Vec<labonair_command_palette::PaletteTabRow> {
        self.tabs
            .read(cx)
            .tabs()
            .iter()
            .map(|t| labonair_command_palette::PaletteTabRow {
                id: t.id,
                label: t.label(),
                kind_title: t.kind.default_title().to_string(),
                is_ssh: self.ssh_tabs.values().any(|s| s.tab_id == t.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_local_file_uri, editor_tab_diagnostic_status, editor_tab_git_status,
        is_supported_external_document_link, relative_path_under_root, shell_quote, ssh_tab_title,
        TabDiagnosticStatus, TabGitStatus,
    };
    use labonair_editor::language_services::{
        Diagnostic, DiagnosticSeverity, LanguageServerId, LspPosition, LspRange,
    };
    use labonair_editor::{GitGutterSnapshot, GitGutterState, GitHunk, GitHunkStatus, Revision};
    use std::path::Path;

    fn git_hunk(status: GitHunkStatus) -> GitHunk {
        GitHunk {
            id: "hunk".into(),
            status,
            header: "@@ -1 +1 @@".into(),
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 1,
            changed_lines: Vec::new(),
            patch: String::new(),
            inline_decorations: Vec::new(),
        }
    }

    #[test]
    fn editor_tab_git_status_requires_a_current_snapshot_and_summarizes_hunks() {
        let revision = Revision::new(1);
        let mut snapshot = GitGutterSnapshot::empty("src/lib.rs", revision);
        snapshot.state = GitGutterState::Ready;
        snapshot.hunks = vec![git_hunk(GitHunkStatus::Added)];
        assert_eq!(
            editor_tab_git_status(&snapshot, revision, "src/lib.rs"),
            Some(TabGitStatus::Added)
        );

        snapshot.hunks.push(git_hunk(GitHunkStatus::Deleted));
        assert_eq!(
            editor_tab_git_status(&snapshot, revision, "src/lib.rs"),
            Some(TabGitStatus::Modified)
        );
        assert_eq!(
            editor_tab_git_status(&snapshot, Revision::new(2), "src/lib.rs"),
            None
        );
    }

    #[test]
    fn editor_tab_diagnostic_status_prefers_errors_then_warnings() {
        let diagnostic = |severity| Diagnostic {
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
            message: "diagnostic".into(),
            source: LanguageServerId::new("test"),
            code: None,
        };

        assert_eq!(
            editor_tab_diagnostic_status(&[diagnostic(DiagnosticSeverity::Warning)]),
            Some(TabDiagnosticStatus::Warning)
        );
        assert_eq!(
            editor_tab_diagnostic_status(&[
                diagnostic(DiagnosticSeverity::Warning),
                diagnostic(DiagnosticSeverity::Error),
            ]),
            Some(TabDiagnosticStatus::Error)
        );
        assert_eq!(
            editor_tab_diagnostic_status(&[diagnostic(DiagnosticSeverity::Information)]),
            None
        );
    }

    #[test]
    fn tab_file_relative_path_requires_a_path_inside_the_project_root() {
        assert_eq!(
            relative_path_under_root(Path::new("/repo/src/main.rs"), Path::new("/repo")),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            relative_path_under_root(Path::new("/other/main.rs"), Path::new("/repo")),
            None
        );
        assert_eq!(
            relative_path_under_root(Path::new("/repo/../other/main.rs"), Path::new("/repo")),
            None
        );
    }

    #[test]
    fn language_server_file_uri_decodes_local_paths() {
        assert_eq!(
            decode_local_file_uri("file:///repo/src/my%20file.rs").unwrap(),
            Path::new("/repo/src/my file.rs")
        );
        assert_eq!(
            decode_local_file_uri("file://localhost/repo/src/file.rs").unwrap(),
            Path::new("/repo/src/file.rs")
        );
        assert!(decode_local_file_uri("https://example.test/file.rs").is_err());
        assert!(decode_local_file_uri("file://remote-host/repo/file.rs").is_err());
        assert!(decode_local_file_uri("file:///repo/bad%2.rs").is_err());
    }

    #[test]
    fn document_link_external_schemes_are_allowlisted() {
        assert!(is_supported_external_document_link(
            "https://example.test/path"
        ));
        assert!(is_supported_external_document_link(
            "HTTP://example.test/path"
        ));
        assert!(is_supported_external_document_link(
            "mailto:dev@example.test"
        ));
        assert!(!is_supported_external_document_link("javascript:alert(1)"));
        assert!(!is_supported_external_document_link(
            "https:///missing-host"
        ));
        assert!(!is_supported_external_document_link(
            "https://example.test/a b"
        ));
    }

    #[test]
    fn shell_quote_wraps_and_escapes() {
        assert_eq!(shell_quote("/a/b"), "'/a/b'");
        assert_eq!(shell_quote("/it's here"), "'/it'\\''s here'");
    }

    #[test]
    fn ssh_tab_title_annotates_jump_route() {
        assert_eq!(ssh_tab_title("prod-web", None), "SSH \u{00b7} prod-web");
        assert_eq!(
            ssh_tab_title("prod-web", Some("bastion")),
            "SSH \u{00b7} prod-web  \u{2192} bastion"
        );
    }
}
