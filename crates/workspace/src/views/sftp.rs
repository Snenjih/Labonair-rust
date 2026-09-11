//! Dual-pane SFTP file browser (T08-001).
//!
//! Ported from `reference-src/src/modules/sftp/` (`SftpPane` / `SftpToolbar` /
//! `SftpContextMenu` / `PropertiesDialog` / `VirtualizedFileList`). The React
//! version is a resizable two-pane browser — local filesystem on the left,
//! the connected host's filesystem (over SFTP) on the right — with an address
//! bar + up/reload/hidden-toggle per pane, inline rename / new file / new
//! folder, a right-click context menu, a permissions (chmod/chown) dialog and
//! a properties dialog. The transfer *queue* UI lives in
//! `labonair-transfers-ui` (R01-003); this module only *triggers* transfers
//! (drag between panes + context-menu upload/download) via `SftpEvent`.
//!
//! Remote work is injected through the focused SSH/SFTP capability contracts.
//! This view does not know about russh handles, backend state, or host storage.
//!
//! Deviations from the reference:
//! * Rows are virtualised with GPUI's [`uniform_list`] (same primitive the
//!   explorer / SCM ports use) rather than `@tanstack/react-virtual`. The
//!   inline rename / new-file / new-folder text field is a pinned row above
//!   the list (not a virtualised row); during a rename the edited entry is
//!   hidden from the list so it isn't shown twice.
//! * The two panes are a draggable split (`split_ratio`, persisted) rather
//!   than a `ResizablePanelGroup`; multi-select uses Cmd/Shift-click plus an
//!   index-based marquee over the virtualised rows (no real geometric
//!   hit-testing, since only the on-screen row window is ever materialised)
//!   rather than `@tanstack/react-virtual`'s per-row geometry.
//! * The `Created` column has no SFTP equivalent — SFTPv3 (what `russh-sftp`
//!   and OpenSSH servers speak) exposes no birth/creation-time attribute, so
//!   it's local-pane-only and hidden for the remote pane rather than showing
//!   a permanent "—".
//! * Remote-edit conflict detection (remote file changed underneath the temp
//!   copy) is not implemented — the backend `save_remote_edit` is a plain
//!   overwrite. Documented as a follow-up.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, uniform_list, App, AppContext, ClickEvent, Context, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Render, ScrollStrategy, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Timer, UniformListScrollHandle, Window,
};
use tokio::runtime::Handle as TokioHandle;

use labonair_filesystem::{mutate, tree};
use labonair_notifications::{notification_center, Notification};
use labonair_settings::{Settings as _, SettingsStore, SftpBrowserSettings, SftpColumn};
use labonair_sftp::{RemoteEntry, SftpBrowserService, SftpSessionHandle, SftpSessionService};
use labonair_ssh::{
    SshConnectRequest, SshConnectionService, SshEventSink, SshRemoteCommandService,
    SshSessionEvent, SshSessionId,
};
use labonair_transfers::TransferDirection;

use crate::theme::ThemeStore;
use labonair_ui_kit::{
    button, caret, context_menu, divider, icon_for_path, icon_toggle_button, tree_row, Axis,
    BlinkCursor, ButtonSize, ButtonVariant, Density, IconName, MenuClick, MenuItem, Palette,
    Tooltip, TreeRowState,
};

/// A menu action against the SFTP view (wrapped into a [`MenuClick`]).
type SftpAct = Box<dyn Fn(&mut SftpView, &mut Context<SftpView>)>;

// ── pure helpers (unit-tested) ─────────────────────────────────────────────

/// Parent of a POSIX path. `/` and `""` map to `/`; a single-segment
/// absolute path maps to `/`. Mirrors the reference `parentPath`.
pub fn parent_path(p: &str) -> String {
    if p == "/" || p.is_empty() {
        return "/".to_string();
    }
    let trimmed = p.strip_suffix('/').unwrap_or(p);
    match trimmed.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

/// Joins `dir` and `name` with exactly one separator.
pub fn join_path(dir: &str, name: &str) -> String {
    if dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

/// Splits an absolute POSIX-style path into breadcrumb `(label, path)`
/// segments, root first — clicking a segment navigates to its `path`.
pub fn path_segments(path: &str) -> Vec<(String, String)> {
    let mut segments = vec![("/".to_string(), "/".to_string())];
    let mut acc = String::new();
    for part in path.split('/').filter(|s| !s.is_empty()) {
        acc.push('/');
        acc.push_str(part);
        segments.push((part.to_string(), acc.clone()));
    }
    segments
}

/// Trims and rejects empty / `.` / `..` / names containing a path separator.
/// Returns `None` when the name is invalid. Mirrors the reference
/// `sanitizeEntryName` (without the `allowNested` option — the SFTP panes
/// never create nested chains inline).
pub fn sanitize_entry_name(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() || t == "." || t == ".." {
        return None;
    }
    if t.contains('/') || t.contains('\\') {
        return None;
    }
    Some(t.to_string())
}

/// A 9-char `rwxr-xr-x`-style permission string → 3-digit octal string.
/// Mirrors the reference `permStringToOctal`.
pub fn perm_string_to_octal(perm: &str) -> String {
    let weights = [
        0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
    ];
    let chars: Vec<char> = perm.chars().take(9).collect();
    let mut n = 0u32;
    for (i, w) in weights.iter().enumerate() {
        if chars.get(i).is_some_and(|c| *c != '-') {
            n |= w;
        }
    }
    format!("{n:03o}")
}

/// Formats a UNIX epoch (seconds) as `YYYY-MM-DD HH:MM` UTC. `0` → `—`.
/// Self-contained (no `chrono`) via the well-known days→civil algorithm.
pub fn format_epoch(secs: i64) -> String {
    if secs <= 0 {
        return "\u{2014}".to_string();
    }
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    // days→civil (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{d:02} {h:02}:{m:02}")
}

/// Human-readable byte size.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.2} MB", b / (KB * KB))
    } else {
        format!("{:.2} GB", b / (KB * KB * KB))
    }
}

/// `secs` as a coarse relative age against `now` (`13d ago`, `just now`).
/// Falls back to [`format_epoch`] for anything older than ~30 days and to
/// `—` for `0`. Mirrors the reference `formatRelativeTime`.
pub fn format_relative_epoch(secs: i64, now: i64) -> String {
    if secs <= 0 {
        return "\u{2014}".to_string();
    }
    let diff = now - secs;
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3_600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86_400 {
        format!("{}h ago", diff / 3_600)
    } else if diff < 86_400 * 30 {
        format!("{}d ago", diff / 86_400)
    } else {
        format_epoch(secs)
    }
}

/// Current wall-clock time as UNIX seconds (for relative formatting).
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Whether `path` on `side` has no parent to walk up to.
fn is_filesystem_root(side: Side, path: &str) -> bool {
    match side {
        Side::Local => std::path::Path::new(path).parent().is_none(),
        Side::Remote => parent_path(path) == path,
    }
}

/// Ordinal of a column in [`SftpColumn::ALL`] — the index into
/// [`SftpView::col_widths`].
fn column_ord(col: SftpColumn) -> usize {
    SftpColumn::ALL.iter().position(|c| *c == col).unwrap_or(0)
}

/// The local-pane fraction of the split given a pointer x-position and the
/// container's total width, clamped so neither pane can be dragged fully
/// shut. `width <= 0.0` (not yet laid out) is a no-op fallback to 50/50.
pub fn split_ratio_from_drag(x: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.5;
    }
    (x / width).clamp(0.2, 0.8)
}

/// Default pixel width of a metadata column.
fn default_column_width(col: SftpColumn) -> f32 {
    match col {
        SftpColumn::Size => 84.0,
        SftpColumn::Modified => 120.0,
        SftpColumn::Created => 120.0,
        SftpColumn::Permissions => 96.0,
        SftpColumn::Type => 64.0,
    }
}

/// Snapshot of the `fileManager` settings the SFTP browser reads.
struct BrowserPrefs {
    columns: Vec<SftpColumn>,
    zebra: bool,
    show_up_folder: bool,
    relative_times: bool,
    show_hidden: bool,
    /// Draggable-split local-pane fraction, persisted from the last drag.
    split_ratio: f32,
    /// Per-column pixel width, indexed by [`column_ord`] — persisted values
    /// where set, [`default_column_width`] otherwise.
    col_widths: [f32; 5],
}

fn sftp_browser_settings(cx: &App) -> BrowserPrefs {
    let s = SftpBrowserSettings::try_get(cx).cloned().unwrap_or_else(|| {
        SftpBrowserSettings::from_settings(&labonair_settings::SettingsContent::default())
    });
    BrowserPrefs {
        columns: s.columns(),
        zebra: s.zebra_striping(),
        show_up_folder: s.show_up_folder(),
        relative_times: s.relative_times(),
        show_hidden: s.show_hidden_files(),
        split_ratio: s.split_ratio(),
        col_widths: std::array::from_fn(|i| {
            let col = SftpColumn::ALL[i];
            s.column_width(col).unwrap_or_else(|| default_column_width(col))
        }),
    }
}

// ── data model ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Local,
    Remote,
}

/// One directory entry, provider-agnostic (local or remote).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: i64,
    /// Creation / birth time (UNIX seconds); `0` when the source doesn't
    /// expose it — always `0` over SFTP, best-effort for local entries.
    pub created_at: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
    /// 9-char permission string (remote only; empty for local).
    pub permissions: String,
    /// Synthetic `..` parent-navigation row — not a real listing entry.
    pub is_parent_link: bool,
}

impl Entry {
    fn from_remote(n: RemoteEntry) -> Self {
        Self {
            name: n.name,
            path: n.path,
            size: n.size,
            modified_at: n.modified_at,
            created_at: 0,
            is_dir: n.is_dir,
            is_symlink: n.is_symlink,
            symlink_target: n.symlink_target,
            permissions: n.permissions,
            is_parent_link: false,
        }
    }

    /// The synthetic `..` row that walks one directory up.
    fn parent_link(parent: String) -> Self {
        Self {
            name: "..".to_string(),
            path: parent,
            size: 0,
            modified_at: 0,
            created_at: 0,
            is_dir: true,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
            is_parent_link: true,
        }
    }
}

/// The column a pane's row list is sorted by. Mirrors [`SftpColumn`] plus a
/// `Name` variant (Name isn't a toggleable metadata column, so it isn't part
/// of `SftpColumn`, but it's the default/most common sort key).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Modified,
    Created,
    Permissions,
    Type,
}

impl SortKey {
    fn from_column(col: SftpColumn) -> Self {
        match col {
            SftpColumn::Size => SortKey::Size,
            SftpColumn::Modified => SortKey::Modified,
            SftpColumn::Created => SortKey::Created,
            SftpColumn::Permissions => SortKey::Permissions,
            SftpColumn::Type => SortKey::Type,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

impl SortDir {
    fn toggled(self) -> Self {
        match self {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        }
    }
}

/// The same classification the `Type` column cell displays: lowercased
/// extension, empty for directories/symlinks. Sorting uses this (not the
/// rendered "—" placeholder text) so the empty classification sorts first.
fn type_sort_key(e: &Entry) -> String {
    if e.is_dir || e.is_symlink {
        String::new()
    } else {
        e.name
            .rsplit_once('.')
            .map(|(_, ext)| ext.to_lowercase())
            .unwrap_or_default()
    }
}

/// Orders two entries by a single column/direction — the underlying value,
/// never the rendered display string (this matters for `Modified`/`Created`,
/// whose relative-time text like `"3m ago"` doesn't sort the way the epoch
/// does).
fn cmp_entries(a: &Entry, b: &Entry, col: SortKey, dir: SortDir) -> std::cmp::Ordering {
    let ord = match col {
        SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::Modified => a.modified_at.cmp(&b.modified_at),
        SortKey::Created => a.created_at.cmp(&b.created_at),
        SortKey::Permissions => {
            perm_string_to_octal(&a.permissions).cmp(&perm_string_to_octal(&b.permissions))
        }
        SortKey::Type => type_sort_key(a).cmp(&type_sort_key(b)),
    };
    match dir {
        SortDir::Asc => ord,
        SortDir::Desc => ord.reverse(),
    }
}

/// Sorts entries dirs-first, then by the given column/direction. Dirs-first
/// applies to every column, not only `Name` — folders would otherwise
/// scatter among files when sorting by size/date/type (and a folder's `Size`
/// cell is always blank, making a pure numeric size sort meaningless for
/// them); this also reproduces the pre-existing default ordering exactly
/// when `col == Name, dir == Asc`.
pub fn sort_entries_by(entries: &mut [Entry], col: SortKey, dir: SortDir) {
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| cmp_entries(a, b, col, dir)));
}

/// Sorts entries dirs-first, then case-insensitively by name — the default
/// ordering. Thin wrapper over [`sort_entries_by`] kept for the load-time
/// call sites and existing tests.
pub fn sort_entries(entries: &mut [Entry]) {
    sort_entries_by(entries, SortKey::Name, SortDir::Asc);
}

/// Paths between the indices of `anchor` and `target` in `visible`
/// (inclusive, order-independent). Falls back to `[target]` if either path
/// isn't present (e.g. filtered out by a search query since the anchor was
/// set). Used by Shift-click range selection.
pub fn path_range(visible: &[String], anchor: &str, target: &str) -> Vec<String> {
    let ai = visible.iter().position(|p| p == anchor);
    let ti = visible.iter().position(|p| p == target);
    match (ai, ti) {
        (Some(a), Some(t)) => {
            let (lo, hi) = (a.min(t), a.max(t));
            visible[lo..=hi].to_vec()
        }
        _ => vec![target.to_string()],
    }
}

/// Paths at indices `[a, b]` (inclusive, order-independent) of `rows`
/// (path, is-`..`-row) pairs, skipping the synthetic `..` row. Used by
/// marquee-drag selection over the virtualised (index-addressed) row list.
pub fn rows_in_range(rows: &[(String, bool)], a: usize, b: usize) -> Vec<String> {
    let (lo, hi) = (a.min(b), a.max(b));
    rows.iter()
        .enumerate()
        .filter(|(i, (_, is_up))| *i >= lo && *i <= hi && !*is_up)
        .map(|(_, (p, _))| p.clone())
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Rename,
    NewFile,
    NewDir,
}

struct EditSlot {
    kind: EditKind,
    /// Original path for a rename.
    orig: Option<String>,
}

/// Transient rubber-band (marquee) selection drag, started by a mouse-down
/// in the empty area below the last row. `base` is the selection to union
/// the swept range into (the pre-drag selection when Cmd/Ctrl is held,
/// otherwise empty).
struct MarqueeState {
    start_index: usize,
    current_index: usize,
    base: std::collections::BTreeSet<String>,
}

struct Pane {
    path: String,
    entries: Vec<Entry>,
    show_hidden: bool,
    loading: bool,
    selected: std::collections::BTreeSet<String>,
    /// Shift-click range base.
    anchor: Option<String>,
    /// In-progress marquee drag, if any.
    marquee: Option<MarqueeState>,
    /// Bumped on every load; a stale async response compares and bails.
    generation: u64,
    edit: Option<EditSlot>,
    edit_buffer: String,
    /// Address-bar edit in progress.
    path_editing: bool,
    path_buffer: String,
    /// Inline name-filter box (toolbar search) is open.
    search_open: bool,
    search_query: String,
    /// Virtualised row-list scroll position.
    scroll: UniformListScrollHandle,
    /// Column-header click sort — applied live in [`Pane::visible`], not at
    /// load time, so switching it never needs a re-list.
    sort_col: SortKey,
    sort_dir: SortDir,
    /// Destination paths of a just-completed enqueue — rows matching one of
    /// these get a brief landing highlight, cleared by a spawned timer.
    flashing: std::collections::BTreeSet<String>,
}

impl Pane {
    fn new(path: String) -> Self {
        Self {
            path,
            entries: Vec::new(),
            show_hidden: false,
            loading: false,
            selected: std::collections::BTreeSet::new(),
            anchor: None,
            marquee: None,
            generation: 0,
            edit: None,
            edit_buffer: String::new(),
            path_editing: false,
            path_buffer: String::new(),
            search_open: false,
            search_query: String::new(),
            scroll: UniformListScrollHandle::new(),
            sort_col: SortKey::default(),
            sort_dir: SortDir::default(),
            flashing: std::collections::BTreeSet::new(),
        }
    }

    fn visible(&self) -> Vec<&Entry> {
        let needle = self.search_query.trim().to_lowercase();
        let mut v: Vec<&Entry> = self
            .entries
            .iter()
            .filter(|e| self.show_hidden || !e.name.starts_with('.'))
            .filter(|e| needle.is_empty() || e.name.to_lowercase().contains(&needle))
            .collect();
        v.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| cmp_entries(a, b, self.sort_col, self.sort_dir))
        });
        v
    }
}

/// Connection lifecycle of the remote pane.
enum Conn {
    Connecting,
    Ready,
    Error(String),
}

/// A pending chmod/chown edit.
struct PermDialog {
    path: String,
    name: String,
    octal: String,
    owner: String,
    group: String,
    field: PermField,
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PermField {
    Octal,
    Owner,
    Group,
}

/// The properties dialog for a single entry.
struct PropsDialog {
    entry: Entry,
    calculated_size: Option<String>,
    calculating: bool,
}

/// Open right-click menu: `(side, entry path)`.
struct Menu {
    side: Side,
    path: String,
    is_dir: bool,
    /// Cursor anchor in window coordinates.
    pos: gpui::Point<gpui::Pixels>,
    /// Two-click delete arm state (mirrors the reference).
    confirming_delete: bool,
}

/// SFTP view → workspace notifications.
pub enum SftpEvent {
    /// Open a local file in the editor.
    OpenLocalFile(String),
    /// Download a remote file to a temp copy and open it for editing.
    OpenRemoteFile {
        session_id: String,
        remote_path: String,
        host_id: String,
    },
    /// Queue a transfer (T08-002); folders are handled recursively by the
    /// injected transfer worker.
    Enqueue {
        session_id: String,
        src_path: String,
        dest_path: String,
        direction: TransferDirection,
    },
    /// Result of the remote SSH/SFTP connection attempt — drives the shared
    /// connecting screen (`ConnectionStatusStore`). `error` is `None` on success.
    ConnResult {
        session_id: String,
        error: Option<String>,
    },
    /// Open (or focus) an SSH terminal tab for this host — the remote pane's
    /// `>_ Term` action.
    OpenRemoteTerminal { host_id: String },
    /// Open a local terminal tab with the given working directory — the
    /// local pane's context-menu "Open Terminal Here" action.
    OpenLocalTerminal { cwd: String },
}

/// Payload of a pointer-drag of one or more rows from one pane to the other
/// (T08-002). Dropping on the opposite pane queues an upload/download.
#[derive(Clone)]
pub struct SftpDrag {
    pub from: Side,
    pub paths: Vec<String>,
}

struct QuietSshEventSink;

impl SshEventSink for QuietSshEventSink {
    fn send(&self, _event: SshSessionEvent) -> Result<(), String> {
        Ok(())
    }
}

pub struct SftpView {
    ssh: std::sync::Arc<dyn SshConnectionService>,
    ssh_remote: std::sync::Arc<dyn SshRemoteCommandService>,
    sftp_session: std::sync::Arc<dyn SftpSessionService>,
    sftp_browser: std::sync::Arc<dyn SftpBrowserService>,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    /// The SSH/SFTP session id (shared registry key).
    session_id: String,
    host_id: String,
    host_label: SharedString,
    conn: Conn,
    sftp_handle: Option<SftpSessionHandle>,
    local: Pane,
    remote: Pane,
    /// Visible metadata columns, in display order (from `fileManager` settings,
    /// reorderable by dragging the column headers).
    columns: Vec<SftpColumn>,
    /// Per-column pixel width, indexed by [`column_ord`]. Persisted to
    /// `fileManager.sftpColumnWidths` once a resize drag ends.
    col_widths: [f32; 5],
    /// Local-pane fraction (`0.2..0.8`) of the draggable split between the
    /// local and remote panes. Persisted to `fileManager.sftpSplitRatio`
    /// once a split-drag ends.
    split_ratio: f32,
    /// Which resize gesture (if any) is currently in progress — set at the
    /// drag's `on_drag` start, read and cleared by the shared left
    /// mouse-up handler, which persists the final value exactly once
    /// rather than on every `on_drag_move` frame (each settings write is a
    /// synchronous file read+patch+rename on the UI thread).
    resizing: Option<ResizeKind>,
    /// Alternating row background.
    zebra: bool,
    /// Show the synthetic `..` row at the top of non-root directories.
    show_up_folder: bool,
    /// Relative (`13d ago`) vs absolute timestamps in the list.
    relative_times: bool,
    menu: Option<Menu>,
    perm: Option<PermDialog>,
    props: Option<PropsDialog>,
    /// Which pane currently owns keyboard focus — drives the visible focus
    /// ring and which pane arrow/Enter/F2/Delete/Backspace act on.
    active_pane: Side,
    local_focus: FocusHandle,
    remote_focus: FocusHandle,
    /// The pane a row-drag currently originates from, if any — dims that
    /// pane's body for the duration of the drag. Set from the dragged row's
    /// `on_drag` constructor (GPUI's drag-start hook); cleared on the next
    /// left mouse-up, which GPUI guarantees fires whether the drag lands on
    /// a valid target, an invalid one, or empty space.
    dragging_side: Option<Side>,
    edit_focus: FocusHandle,
    dialog_focus: FocusHandle,
    /// Drives the caret blink for every hand-rolled field on this view (path
    /// edit, name filter, rename, chmod) — only one is ever focused at once.
    blink: Entity<BlinkCursor>,
    _blink_obs: Subscription,
    /// `cx.on_focus`/`cx.on_blur` need a `Window`, which `new()` doesn't
    /// have — wired lazily on first render instead.
    blink_focus_wired: bool,
    _blink_focus_subs: Vec<Subscription>,
    edit_focused: bool,
    dialog_focused: bool,
}

impl EventEmitter<SftpEvent> for SftpView {}

impl Focusable for SftpView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        match self.active_pane {
            Side::Local => self.local_focus.clone(),
            Side::Remote => self.remote_focus.clone(),
        }
    }
}

impl SftpView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ssh: std::sync::Arc<dyn SshConnectionService>,
        ssh_remote: std::sync::Arc<dyn SshRemoteCommandService>,
        sftp_session: std::sync::Arc<dyn SftpSessionService>,
        sftp_browser: std::sync::Arc<dyn SftpBrowserService>,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        session_id: String,
        host_id: String,
        host_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let s = sftp_browser_settings(cx);
        let mut local = Pane::new(home);
        let mut remote = Pane::new("/".to_string());
        local.show_hidden = s.show_hidden;
        remote.show_hidden = s.show_hidden;
        let blink = cx.new(|_| BlinkCursor::new());
        let _blink_obs = cx.observe(&blink, |_, _, cx| cx.notify());
        let mut this = Self {
            ssh,
            ssh_remote,
            sftp_session,
            sftp_browser,
            tokio,
            theme,
            session_id,
            host_id,
            host_label: host_label.into(),
            conn: Conn::Connecting,
            sftp_handle: None,
            local,
            remote,
            columns: s.columns,
            col_widths: s.col_widths,
            split_ratio: s.split_ratio,
            resizing: None,
            zebra: s.zebra,
            show_up_folder: s.show_up_folder,
            relative_times: s.relative_times,
            menu: None,
            perm: None,
            props: None,
            active_pane: Side::Local,
            local_focus: cx.focus_handle(),
            remote_focus: cx.focus_handle(),
            dragging_side: None,
            edit_focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
            blink,
            _blink_obs,
            blink_focus_wired: false,
            _blink_focus_subs: Vec::new(),
            edit_focused: false,
            dialog_focused: false,
        };
        cx.observe_global::<SettingsStore>(Self::apply_settings).detach();
        this.load_local(cx);
        this.connect(cx);
        this
    }

    /// Re-read the `fileManager` settings that drive the browser chrome
    /// (columns, zebra, `..` row, relative times, default hidden-files).
    fn apply_settings(&mut self, cx: &mut Context<Self>) {
        let s = sftp_browser_settings(cx);
        self.columns = s.columns;
        self.zebra = s.zebra;
        self.show_up_folder = s.show_up_folder;
        self.relative_times = s.relative_times;
        cx.notify();
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn sftp_handle(&self) -> Option<SftpSessionHandle> {
        self.sftp_handle.clone()
    }

    fn notify_error(
        &self,
        title: &'static str,
        details: String,
        dedupe_key: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        notification_center(cx).update(cx, |center, cx| {
            center.push(
                Notification::error(title, "The SFTP operation could not be completed.")
                    .source("sftp")
                    .details(details)
                    .dedupe_key(dedupe_key.into()),
                cx,
            );
        });
    }

    fn pane(&mut self, side: Side) -> &mut Pane {
        match side {
            Side::Local => &mut self.local,
            Side::Remote => &mut self.remote,
        }
    }

    /// Column-header click: switches the pane's sort column, or toggles
    /// direction when the same column is clicked again.
    fn toggle_sort(&mut self, side: Side, col: SortKey, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        if pane.sort_col == col {
            pane.sort_dir = pane.sort_dir.toggled();
        } else {
            pane.sort_col = col;
            pane.sort_dir = SortDir::Asc;
        }
        cx.notify();
    }

    // ── connection ─────────────────────────────────────────────────────────

    /// Re-run the remote connection (used by the connecting screen's Retry).
    pub fn reconnect(&mut self, cx: &mut Context<Self>) {
        self.connect(cx);
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        self.conn = Conn::Connecting;
        let ssh = self.ssh.clone();
        let sftp = self.sftp_session.clone();
        let (sid, hid) = (self.session_id.clone(), self.host_id.clone());
        let jh = self.tokio.spawn(async move {
            ssh.connect(
                SshConnectRequest {
                    session_id: SshSessionId::new(sid.clone()),
                    host_id: hid,
                    passphrase: None,
                    password: None,
                    initial_cols: None,
                    initial_rows: None,
                    blocks: false,
                    connect_timeout_secs: None,
                },
                std::sync::Arc::new(QuietSshEventSink),
            )
            .await
            .map_err(|e| e.to_string())?;
            match sftp.open(SshSessionId::new(sid.clone())).await {
                Ok(handle) => Ok(handle),
                Err(error) => {
                    let _ = ssh.disconnect(SshSessionId::new(sid)).await;
                    Err(error.to_string())
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                let sid = this.session_id.clone();
                match res {
                    Ok(handle) => {
                        this.sftp_handle = Some(handle);
                        this.conn = Conn::Ready;
                        this.load_remote(cx);
                        cx.emit(SftpEvent::ConnResult {
                            session_id: sid,
                            error: None,
                        });
                    }
                    Err(e) => {
                        this.conn = Conn::Error(e.clone());
                        this.notify_error(
                            "SFTP connection failed",
                            e.clone(),
                            format!("sftp:connect:{}", this.session_id),
                            cx,
                        );
                        cx.emit(SftpEvent::ConnResult {
                            session_id: sid,
                            error: Some(e),
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── loading ────────────────────────────────────────────────────────────

    fn load_local(&mut self, cx: &mut Context<Self>) {
        self.local.generation += 1;
        let generation = self.local.generation;
        self.local.loading = true;
        let path = self.local.path.clone();
        let show_hidden = self.local.show_hidden;
        cx.spawn(async move |this, cx| {
            let path2 = path.clone();
            let res = cx
                .background_executor()
                .spawn(async move { tree::list_dir_entries_sync(&path2, true) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.local.generation != generation {
                    return;
                }
                this.local.loading = false;
                match res {
                    Ok(list) => {
                        let mut entries: Vec<Entry> = list
                            .into_iter()
                            .map(|d| Entry {
                                name: d.name.clone(),
                                path: join_path(&path, &d.name),
                                size: d.size,
                                modified_at: (d.mtime / 1000) as i64,
                                created_at: (d.created / 1000) as i64,
                                is_dir: matches!(d.kind, tree::EntryKind::Dir),
                                is_symlink: matches!(d.kind, tree::EntryKind::Symlink),
                                symlink_target: None,
                                permissions: String::new(),
                                is_parent_link: false,
                            })
                            .collect();
                        sort_entries(&mut entries);
                        this.local.entries = entries;
                    }
                    Err(e) => {
                        this.local.entries.clear();
                        this.notify_error(
                            "Local directory load failed",
                            e,
                            format!("sftp:local-load:{}", this.local.path),
                            cx,
                        );
                    }
                }
                let _ = show_hidden;
                cx.notify();
            });
        })
        .detach();
    }

    fn load_remote(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.conn, Conn::Ready) {
            return;
        }
        self.remote.generation += 1;
        let generation = self.remote.generation;
        self.remote.loading = true;
        let Some(handle) = self.sftp_handle.clone() else {
            return;
        };
        let browser = self.sftp_browser.clone();
        let path = self.remote.path.clone();
        let jh = self.tokio.spawn(async move {
            browser
                .read_dir(handle, path)
                .await
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.remote.generation != generation {
                    return;
                }
                this.remote.loading = false;
                match res {
                    Ok(list) => {
                        let mut entries: Vec<Entry> =
                            list.into_iter().map(Entry::from_remote).collect();
                        sort_entries(&mut entries);
                        this.remote.entries = entries;
                    }
                    Err(e) => {
                        this.remote.entries.clear();
                        this.notify_error(
                            "Remote directory load failed",
                            e,
                            format!("sftp:remote-load:{}:{}", this.session_id, this.remote.path),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reload(&mut self, side: Side, cx: &mut Context<Self>) {
        match side {
            Side::Local => self.load_local(cx),
            Side::Remote => self.load_remote(cx),
        }
    }

    // ── navigation ─────────────────────────────────────────────────────────

    fn navigate(&mut self, side: Side, path: String, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.path = path;
        pane.selected.clear();
        pane.anchor = None;
        pane.edit = None;
        pane.path_editing = false;
        self.reload(side, cx);
    }

    fn go_up(&mut self, side: Side, cx: &mut Context<Self>) {
        let up = match side {
            Side::Local => {
                let p = std::path::Path::new(&self.local.path);
                p.parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.local.path.clone())
            }
            Side::Remote => parent_path(&self.remote.path),
        };
        self.navigate(side, up, cx);
    }

    /// Whether the synthetic `..` row is currently shown above `side`'s
    /// entries (mirrors the condition in [`Self::render_list`]) — needed so
    /// arrow-key scroll-into-view targets the right virtualised row index.
    fn shows_up_row(&self, side: Side) -> bool {
        let pane = match side {
            Side::Local => &self.local,
            Side::Remote => &self.remote,
        };
        self.show_up_folder
            && pane.search_query.trim().is_empty()
            && !is_filesystem_root(side, &pane.path)
    }

    /// Move the single-row keyboard selection by `delta` within `side`'s
    /// visible entries, clamping at the ends, and scroll it into view.
    fn move_list_selection(&mut self, side: Side, delta: isize, cx: &mut Context<Self>) {
        let up_offset = usize::from(self.shows_up_row(side));
        let pane = self.pane(side);
        let visible: Vec<String> = pane.visible().into_iter().map(|e| e.path.clone()).collect();
        if visible.is_empty() {
            return;
        }
        let current = pane.selected.iter().next().cloned();
        let cur_idx = current.and_then(|p| visible.iter().position(|v| *v == p));
        let next_idx = match cur_idx {
            Some(i) => {
                let n = i as isize + delta;
                if n < 0 || n as usize >= visible.len() {
                    return;
                }
                n as usize
            }
            None => {
                if delta > 0 {
                    0
                } else {
                    visible.len() - 1
                }
            }
        };
        let path = visible[next_idx].clone();
        pane.selected.clear();
        pane.selected.insert(path.clone());
        pane.anchor = Some(path);
        pane.scroll
            .scroll_to_item(next_idx + up_offset, ScrollStrategy::Center);
        cx.notify();
    }

    /// `Enter` on the keyboard-selected row — same activation path as a
    /// double-click.
    fn activate_selected(&mut self, side: Side, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        let Some(path) = pane.selected.iter().next().cloned() else {
            return;
        };
        let Some(entry) = pane.entries.iter().find(|e| e.path == path).cloned() else {
            return;
        };
        self.activate(side, &entry, true, cx);
    }

    /// Keyboard navigation for the file list itself (arrows / Enter / F2 /
    /// Delete / Backspace / Tab) — a no-op whenever an inline edit, the
    /// address bar, the search box, or a dialog/menu currently owns input.
    fn on_list_key(
        &mut self,
        side: Side,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.menu.is_some() || self.perm.is_some() || self.props.is_some() {
            return;
        }
        {
            let pane = self.pane(side);
            if pane.edit.is_some() || pane.path_editing || pane.search_open {
                return;
            }
        }
        match ev.keystroke.key.as_str() {
            "up" => self.move_list_selection(side, -1, cx),
            "down" => self.move_list_selection(side, 1, cx),
            "enter" => self.activate_selected(side, cx),
            "backspace" => self.go_up(side, cx),
            "f2" => self.start_edit(side, EditKind::Rename, cx),
            "delete" => {
                let pane = self.pane(side);
                let Some(path) = pane.selected.iter().next().cloned() else {
                    return;
                };
                self.delete(side, path, cx);
            }
            "tab" => {
                let next = match side {
                    Side::Local => Side::Remote,
                    Side::Remote => Side::Local,
                };
                self.active_pane = next;
                match next {
                    Side::Local => window.focus(&self.local_focus),
                    Side::Remote => window.focus(&self.remote_focus),
                }
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn toggle_hidden(&mut self, side: Side, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.show_hidden = !pane.show_hidden;
        cx.notify();
    }

    fn toggle_search(&mut self, side: Side, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.search_open = !pane.search_open;
        if !pane.search_open {
            pane.search_query.clear();
        }
        cx.notify();
    }

    /// Move `moved` so it sits where `target` is in the visible-column order,
    /// then persist the new order to `fileManager` settings.
    fn reorder_column(&mut self, moved: SftpColumn, target: SftpColumn, cx: &mut Context<Self>) {
        if moved == target {
            return;
        }
        let mut cols = self.columns.clone();
        let Some(from) = cols.iter().position(|c| *c == moved) else {
            return;
        };
        cols.remove(from);
        let to = cols.iter().position(|c| *c == target).unwrap_or(cols.len());
        cols.insert(to, moved);
        self.columns = cols.clone();
        if cx.has_global::<SettingsStore>() {
            let _ = cx.global_mut::<SettingsStore>().update_user(move |c| {
                c.file_manager.sftp_columns = Some(cols.clone());
            });
        }
        cx.notify();
    }

    fn resize_column(&mut self, col: SftpColumn, delta_px: f32, cx: &mut Context<Self>) {
        let ord = column_ord(col);
        self.col_widths[ord] = (self.col_widths[ord] + delta_px).clamp(48.0, 320.0);
        self.resizing = Some(ResizeKind::Column(col));
        cx.notify();
    }

    /// Set the local-pane fraction of the pane split. `ratio` is expected
    /// pre-clamped (see [`split_ratio_from_drag`]) but is clamped again
    /// defensively.
    fn resize_split(&mut self, ratio: f32, cx: &mut Context<Self>) {
        self.split_ratio = ratio.clamp(0.2, 0.8);
        self.resizing = Some(ResizeKind::Split);
        cx.notify();
    }

    /// Persist whichever resize gesture just ended (column width or split
    /// ratio) to `fileManager` settings — called once from the shared
    /// left-mouse-up handler, not per `on_drag_move` frame: each
    /// `update_user` call is a synchronous settings-file read+patch+rename
    /// on the UI thread, cheap once per gesture but not once per pixel.
    fn persist_resize(&mut self, cx: &mut Context<Self>) {
        let Some(kind) = self.resizing.take() else {
            return;
        };
        if !cx.has_global::<SettingsStore>() {
            return;
        }
        match kind {
            ResizeKind::Column(col) => {
                let width = self.col_widths[column_ord(col)];
                let token = col.token().to_string();
                let _ = cx.global_mut::<SettingsStore>().update_user(move |c| {
                    let widths = c.file_manager.sftp_column_widths.get_or_insert_with(Default::default);
                    widths.insert(token.clone(), width);
                });
            }
            ResizeKind::Split => {
                let ratio = self.split_ratio;
                let _ = cx.global_mut::<SettingsStore>().update_user(move |c| {
                    c.file_manager.sftp_split_ratio = Some(ratio);
                });
            }
        }
    }

    fn activate(&mut self, side: Side, entry: &Entry, dbl: bool, cx: &mut Context<Self>) {
        let pane = self.pane(side);
        pane.selected.clear();
        pane.selected.insert(entry.path.clone());
        pane.anchor = Some(entry.path.clone());
        if entry.is_dir {
            if dbl {
                let target = entry
                    .symlink_target
                    .clone()
                    .filter(|_| entry.is_symlink)
                    .unwrap_or_else(|| entry.path.clone());
                self.navigate(side, target, cx);
            }
        } else if dbl {
            match side {
                Side::Local => cx.emit(SftpEvent::OpenLocalFile(entry.path.clone())),
                Side::Remote => cx.emit(SftpEvent::OpenRemoteFile {
                    session_id: self.session_id.clone(),
                    remote_path: entry.path.clone(),
                    host_id: self.host_id.clone(),
                }),
            }
        }
        cx.notify();
    }

    // ── mutations ──────────────────────────────────────────────────────────

    fn start_edit(&mut self, side: Side, kind: EditKind, cx: &mut Context<Self>) {
        self.menu = None;
        let (buffer, orig) = match kind {
            EditKind::Rename => {
                let sel = self.pane(side).selected.iter().next().cloned();
                let Some(sel) = sel else { return };
                let name = std::path::Path::new(&sel)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (name, Some(sel))
            }
            _ => (String::new(), None),
        };
        let pane = self.pane(side);
        pane.edit_buffer = buffer;
        pane.edit = Some(EditSlot { kind, orig });
        cx.notify();
    }

    fn cancel_edit(&mut self, side: Side, cx: &mut Context<Self>) {
        self.pane(side).edit = None;
        cx.notify();
    }

    fn commit_edit(&mut self, side: Side, cx: &mut Context<Self>) {
        let (kind, orig, raw, dir) = {
            let pane = self.pane(side);
            let Some(slot) = pane.edit.as_ref() else {
                return;
            };
            (
                slot.kind,
                slot.orig.clone(),
                pane.edit_buffer.clone(),
                pane.path.clone(),
            )
        };
        let Some(name) = sanitize_entry_name(&raw) else {
            self.notify_error(
                "Invalid SFTP name",
                "Names cannot be empty, '.', '..', or contain path separators.".to_string(),
                format!("sftp:invalid-name:{side:?}"),
                cx,
            );
            cx.notify();
            return;
        };
        self.pane(side).edit = None;

        let Some(handle) = self.sftp_handle.clone() else {
            return;
        };
        let browser = self.sftp_browser.clone();
        match (side, kind) {
            (Side::Remote, EditKind::Rename) => {
                let Some(old) = orig else { return };
                let new = join_path(&parent_path(&old), &name);
                let jh = self.tokio.spawn(async move {
                    browser
                        .rename(handle, old, new)
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Remote, EditKind::NewFile) => {
                let path = join_path(&dir, &name);
                let jh = self.tokio.spawn(async move {
                    browser
                        .create_file(handle, path)
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Remote, EditKind::NewDir) => {
                let path = join_path(&dir, &name);
                let jh = self.tokio.spawn(async move {
                    browser
                        .mkdir(handle, path, false)
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            (Side::Local, kind) => {
                let dir2 = dir.clone();
                let name2 = name.clone();
                let orig2 = orig.clone();
                cx.spawn(async move |this, cx| {
                    let res = cx
                        .background_executor()
                        .spawn(async move {
                            match kind {
                                EditKind::Rename => {
                                    let old = orig2.unwrap_or_default();
                                    let new = join_path(&parent_path(&old), &name2);
                                    mutate::rename_sync(&old, &new)
                                }
                                EditKind::NewFile => {
                                    mutate::create_file_sync(&join_path(&dir2, &name2))
                                }
                                EditKind::NewDir => {
                                    mutate::create_dir_sync(&join_path(&dir2, &name2))
                                }
                            }
                        })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Err(e) = res {
                            this.notify_error(
                                "Local file operation failed",
                                e,
                                format!("sftp:local-operation:{}", this.local.path),
                                cx,
                            );
                        }
                        this.load_local(cx);
                    });
                })
                .detach();
            }
        }
    }

    fn after_remote_op(
        &mut self,
        jh: tokio::task::JoinHandle<Result<(), String>>,
        side: Side,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = res {
                    this.notify_error(
                        "Remote file operation failed",
                        e,
                        format!("sftp:remote-operation:{side:?}"),
                        cx,
                    );
                }
                this.reload(side, cx);
            });
        })
        .detach();
    }

    fn delete(&mut self, side: Side, path: String, cx: &mut Context<Self>) {
        self.menu = None;
        let Some(handle) = self.sftp_handle.clone() else {
            return;
        };
        let browser = self.sftp_browser.clone();
        match side {
            Side::Remote => {
                let jh = self.tokio.spawn(async move {
                    browser
                        .delete(handle, vec![path])
                        .await
                        .map_err(|e| e.to_string())
                });
                self.after_remote_op(jh, side, cx);
            }
            Side::Local => {
                cx.spawn(async move |this, cx| {
                    let res = cx
                        .background_executor()
                        .spawn(async move { mutate::delete_sync(&path) })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if let Err(e) = res {
                            this.notify_error(
                                "Local delete failed",
                                e,
                                format!("sftp:local-delete:{}", this.local.path),
                                cx,
                            );
                        }
                        this.load_local(cx);
                    });
                })
                .detach();
            }
        }
    }

    // ── permissions dialog ─────────────────────────────────────────────────

    fn open_perm_dialog(&mut self, entry: &Entry, cx: &mut Context<Self>) {
        self.menu = None;
        let octal = if entry.permissions.is_empty() {
            "755".to_string()
        } else {
            perm_string_to_octal(&entry.permissions)
        };
        self.perm = Some(PermDialog {
            path: entry.path.clone(),
            name: entry.name.clone(),
            octal,
            owner: String::new(),
            group: String::new(),
            field: PermField::Octal,
            error: None,
        });
        cx.notify();
    }

    fn apply_perm(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.perm.as_ref() else { return };
        let path = d.path.clone();
        let (owner, group) = (d.owner.trim().to_string(), d.group.trim().to_string());
        let octal = u32::from_str_radix(d.octal.trim(), 8).ok();
        if octal.is_none() {
            if let Some(d) = self.perm.as_mut() {
                d.error = Some("Invalid octal value".to_string());
            }
            cx.notify();
            return;
        }
        self.perm = None;
        let mode = octal.unwrap();
        let Some(handle) = self.sftp_handle.clone() else {
            return;
        };
        let browser = self.sftp_browser.clone();
        let ssh_remote = self.ssh_remote.clone();
        let sid = self.session_id.clone();
        let do_chown = !owner.is_empty() || !group.is_empty();
        let jh = self.tokio.spawn(async move {
            let sid_clone = sid.clone();
            browser
                .chmod(handle, path.clone(), mode)
                .await
                .map_err(|e| e.to_string())?;
            if do_chown {
                ssh_remote
                    .chown(SshSessionId::new(sid_clone), path, owner, group)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            Ok::<(), String>(())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if let Err(e) = res {
                    this.notify_error(
                        "Permission update failed",
                        e,
                        format!("sftp:permissions:{}", this.session_id),
                        cx,
                    );
                }
                this.load_remote(cx);
            });
        })
        .detach();
    }

    // ── properties dialog ──────────────────────────────────────────────────

    fn open_props(&mut self, entry: &Entry, cx: &mut Context<Self>) {
        self.menu = None;
        self.props = Some(PropsDialog {
            entry: entry.clone(),
            calculated_size: None,
            calculating: false,
        });
        cx.notify();
    }

    fn calc_size(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.props.as_mut() else { return };
        d.calculating = true;
        let path = d.entry.path.clone();
        let Some(handle) = self.sftp_handle.clone() else {
            return;
        };
        let ssh_remote = self.ssh_remote.clone();
        let sid = self.session_id.clone();
        let jh = self.tokio.spawn(async move {
            let _ = handle;
            ssh_remote
                .calculate_size(SshSessionId::new(sid), path)
                .await
                .map_err(|e| e.to_string())
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                match res {
                    Ok(s) => {
                        if let Some(d) = this.props.as_mut() {
                            d.calculating = false;
                            d.calculated_size = Some(s);
                        }
                    }
                    Err(e) => {
                        let path = this
                            .props
                            .as_ref()
                            .map(|dialog| dialog.entry.path.clone())
                            .unwrap_or_default();
                        this.notify_error(
                            "Remote size calculation failed",
                            e,
                            format!("sftp:size:{path}"),
                            cx,
                        );
                        if let Some(d) = this.props.as_mut() {
                            d.calculating = false;
                            d.calculated_size = None;
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── transfers (T08-002) ────────────────────────────────────────────────

    /// Queue upload/download of `src_paths` (which live on the `from` side)
    /// into the opposite pane's current directory. Folders transfer
    /// recursively (handled by the backend worker).
    fn enqueue(&mut self, from: Side, src_paths: Vec<String>, cx: &mut Context<Self>) {
        self.menu = None;
        let (dest_dir, direction) = match from {
            Side::Local => (self.remote.path.clone(), TransferDirection::Upload),
            Side::Remote => (self.local.path.clone(), TransferDirection::Download),
        };
        let target_side = match from {
            Side::Local => Side::Remote,
            Side::Remote => Side::Local,
        };
        let session_id = self.session_id.clone();
        let mut landed = Vec::new();
        for src in src_paths {
            let name = src
                .rsplit(['/', '\\'])
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("file")
                .to_string();
            let dest_path = join_path(&dest_dir, &name);
            landed.push(dest_path.clone());
            cx.emit(SftpEvent::Enqueue {
                session_id: session_id.clone(),
                src_path: src,
                dest_path,
                direction,
            });
        }
        if !landed.is_empty() {
            for p in &landed {
                self.pane(target_side).flashing.insert(p.clone());
            }
            cx.spawn(async move |this, cx| {
                Timer::after(std::time::Duration::from_millis(900)).await;
                this.update(cx, |this, cx| {
                    let pane = this.pane(target_side);
                    for p in &landed {
                        pane.flashing.remove(p);
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        cx.notify();
    }

    /// Reload one pane after a transfer landed a file in it.
    pub fn reload_side(&mut self, remote: bool, cx: &mut Context<Self>) {
        self.reload(if remote { Side::Remote } else { Side::Local }, cx);
    }

    fn menu_entry(&self) -> Option<(Side, Entry)> {
        let m = self.menu.as_ref()?;
        let pane = match m.side {
            Side::Local => &self.local,
            Side::Remote => &self.remote,
        };
        pane.entries
            .iter()
            .find(|e| e.path == m.path)
            .map(|e| (m.side, e.clone()))
    }
}

// ── keyboard for inline edit / dialogs ─────────────────────────────────────

impl SftpView {
    fn on_edit_key(&mut self, side: Side, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cancel_edit(side, cx),
            "enter" => self.commit_edit(side, cx),
            "backspace" => {
                self.pane(side).edit_buffer.pop();
                self.blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    self.pane(side).edit_buffer.push_str(&ch);
                    self.blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    fn on_path_key(&mut self, side: Side, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                self.pane(side).path_editing = false;
                cx.notify();
            }
            "enter" => {
                let p = self.pane(side).path_buffer.clone();
                if !p.trim().is_empty() {
                    self.navigate(side, p.trim().to_string(), cx);
                }
            }
            "backspace" => {
                self.pane(side).path_buffer.pop();
                self.blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    self.pane(side).path_buffer.push_str(&ch);
                    self.blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    fn on_search_key(&mut self, side: Side, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                let pane = self.pane(side);
                pane.search_open = false;
                pane.search_query.clear();
                cx.notify();
            }
            "backspace" => {
                self.pane(side).search_query.pop();
                self.blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    self.pane(side).search_query.push_str(&ch);
                    self.blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    fn on_perm_key(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                self.perm = None;
                cx.notify();
            }
            "enter" => self.apply_perm(cx),
            "tab" => {
                if let Some(d) = self.perm.as_mut() {
                    d.field = match d.field {
                        PermField::Octal => PermField::Owner,
                        PermField::Owner => PermField::Group,
                        PermField::Group => PermField::Octal,
                    };
                }
                cx.notify();
            }
            "backspace" => {
                if let Some(d) = self.perm.as_mut() {
                    match d.field {
                        PermField::Octal => d.octal.pop(),
                        PermField::Owner => d.owner.pop(),
                        PermField::Group => d.group.pop(),
                    };
                }
                self.blink.update(cx, |b, cx| b.pause(cx));
                cx.notify();
            }
            key => {
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                    return;
                }
                if let Some(ch) = printable(ks, key) {
                    if let Some(d) = self.perm.as_mut() {
                        match d.field {
                            PermField::Octal => {
                                if ch.chars().all(|c| c.is_ascii_digit()) && d.octal.len() < 4 {
                                    d.octal.push_str(&ch);
                                }
                            }
                            PermField::Owner => d.owner.push_str(&ch),
                            PermField::Group => d.group.push_str(&ch),
                        }
                    }
                    self.blink.update(cx, |b, cx| b.pause(cx));
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }
}

fn printable(ks: &gpui::Keystroke, key: &str) -> Option<String> {
    ks.key_char
        .clone()
        .filter(|s| !s.is_empty() && !s.chars().any(|c| c.is_control()))
        .or_else(|| (key.chars().count() == 1).then(|| key.to_string()))
}

// ── rendering ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Colors {
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    accent: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    bg: gpui::Hsla,
    err: gpui::Hsla,
    /// Subtle alternating-row fill (zebra striping).
    zebra: gpui::Hsla,
    /// Directional drop-target tint.
    drop: gpui::Hsla,
    /// The full token snapshot the ui-kit primitives (`button`, `banner`, …)
    /// are styled from.
    palette: Palette,
}

impl Render for SftpView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.blink_focus_wired {
            self.blink_focus_wired = true;
            self._blink_focus_subs.push(cx.on_focus(
                &self.edit_focus.clone(),
                window,
                |this, _w, cx| {
                    this.edit_focused = true;
                    this.blink.update(cx, |b, cx| b.start(cx));
                },
            ));
            self._blink_focus_subs.push(cx.on_blur(
                &self.edit_focus.clone(),
                window,
                |this, _w, cx| {
                    this.edit_focused = false;
                    this.blink.update(cx, |b, cx| b.stop(cx));
                },
            ));
            self._blink_focus_subs.push(cx.on_focus(
                &self.dialog_focus.clone(),
                window,
                |this, _w, cx| {
                    this.dialog_focused = true;
                    this.blink.update(cx, |b, cx| b.start(cx));
                },
            ));
            self._blink_focus_subs.push(cx.on_blur(
                &self.dialog_focus.clone(),
                window,
                |this, _w, cx| {
                    this.dialog_focused = false;
                    this.blink.update(cx, |b, cx| b.stop(cx));
                },
            ));
            // Keep `active_pane` in sync with real keyboard focus so the
            // pane-body click-to-focus behaviour (from `track_focus`) also
            // drives the focus ring / arrow-key target, not just `Tab`.
            self._blink_focus_subs.push(cx.on_focus(
                &self.local_focus.clone(),
                window,
                |this, _w, cx| {
                    this.active_pane = Side::Local;
                    cx.notify();
                },
            ));
            self._blink_focus_subs.push(cx.on_focus(
                &self.remote_focus.clone(),
                window,
                |this, _w, cx| {
                    this.active_pane = Side::Remote;
                    cx.notify();
                },
            ));
        }
        let c = {
            let t = self.theme.read(cx);
            Colors {
                fg: t.foreground(),
                muted: t.muted_foreground(),
                accent: t.accent(),
                border: t.border(),
                card: t.card(),
                bg: t.background(),
                err: t.status_error(),
                zebra: {
                    let mut h = t.hover_fill();
                    h.a *= 0.5;
                    h
                },
                drop: {
                    let mut a = t.accent();
                    a.a = 0.12;
                    a
                },
                palette: Palette::from_theme(t),
            }
        };

        let local = self.render_pane(Side::Local, c, window, cx);
        let remote = self.render_pane(Side::Remote, c, window, cx);

        let mut root = div()
            .id("sftp")
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(c.bg)
            .text_color(c.fg)
            // GPUI clears `cx.active_drag` on every left mouse-up while a
            // drag is live, whether it lands on a valid target, an invalid
            // one, or empty space — mirroring that here reliably clears the
            // source-pane dim in all three cases, not just a successful
            // drop, and doubles as the "drag end" signal for the debounced
            // column-width/split-ratio settings persist (see
            // `SftpView::persist_resize` — writing on every `on_drag_move`
            // frame would mean a synchronous settings-file rewrite tens of
            // times a second while a resize is in progress).
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _w, cx| {
                    let mut changed = this.dragging_side.take().is_some();
                    if this.resizing.is_some() {
                        this.persist_resize(cx);
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                }),
            )
            // Ratio math lives on this outer container (not the grip
            // itself) so `ev.bounds` is the whole local+grip+remote row —
            // mirrors the dock-resize precedent in `workspace.rs`, where
            // the handle only starts the drag and the ancestor computes
            // the new size from its own bounds.
            .on_drag_move(cx.listener(
                |this, ev: &gpui::DragMoveEvent<SplitDrag>, _w, cx| {
                    let b = ev.bounds;
                    let width = f32::from(b.size.width);
                    let x = f32::from(ev.event.position.x - b.origin.x);
                    this.resize_split(split_ratio_from_drag(x, width), cx);
                },
            ))
            .child(
                div()
                    .w(gpui::relative(self.split_ratio))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_full()
                    .child(local),
            )
            .child(self.render_split_grip(c))
            .child(
                div()
                    .w(gpui::relative(1.0 - self.split_ratio))
                    .flex_shrink_0()
                    .min_w_0()
                    .h_full()
                    .child(remote),
            );

        if self.menu.is_some() {
            root = root.child(self.render_menu(c, cx));
        }
        if self.perm.is_some() {
            root = root.child(self.render_perm_dialog(c, cx));
        }
        if self.props.is_some() {
            root = root.child(self.render_props_dialog(c, cx));
        }
        root
    }
}

impl SftpView {
    /// Thin draggable grip between the two panes. The drag itself only
    /// starts here; the ratio math runs on the outer `root` container in
    /// `render()` (see the `on_drag_move` comment there).
    fn render_split_grip(&self, c: Colors) -> gpui::AnyElement {
        div()
            .relative()
            .w(px(1.0))
            .h_full()
            .flex_shrink_0()
            .child(divider(Axis::Vertical, c.border))
            .child(
                div()
                    .id("sftp-split-grip")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(-3.0))
                    .w(px(6.0))
                    .cursor_col_resize()
                    .hover(|s| s.bg(c.accent))
                    .on_drag(SplitDrag, |_, _, _, cx| cx.new(|_| DragGhost)),
            )
            .into_any_element()
    }

    fn render_pane(
        &self,
        side: Side,
        c: Colors,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let pane = match side {
            Side::Local => &self.local,
            Side::Remote => &self.remote,
        };
        let (label, count_label) = match side {
            Side::Local => ("LOCAL", format!("{} items", pane.visible().len())),
            Side::Remote => (
                "REMOTE",
                match &self.conn {
                    Conn::Ready => format!(
                        "{} \u{00b7} {} items",
                        self.host_label,
                        pane.visible().len()
                    ),
                    Conn::Connecting => format!("{} \u{00b7} connecting\u{2026}", self.host_label),
                    Conn::Error(_) => format!("{} \u{00b7} offline", self.host_label),
                },
            ),
        };

        // Row 1: pane label + item count.
        let header = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(22.0))
            .child(
                div()
                    .text_size(px(10.5))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(c.fg)
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(c.muted)
                    .child(SharedString::from(count_label)),
            );

        // Row 2: up · path · search · refresh · hidden · (remote) term.
        let mut toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .h(px(30.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                self.tool_btn(side, "up", IconName::ArrowUp.svg(c.muted), c, cx, |this, side, cx| {
                    this.go_up(side, cx)
                }),
            )
            .child(self.render_path_bar(side, pane, c, cx))
            .child(
                icon_toggle_button(
                    match side {
                        Side::Local => "sftp-local-search",
                        Side::Remote => "sftp-remote-search",
                    },
                    c.palette,
                    IconName::Search,
                    pane.search_open,
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.toggle_search(side, cx);
                    if this.pane(side).search_open {
                        window.focus(&this.edit_focus);
                    }
                })),
            )
            .child(self.tool_btn(
                side,
                "reload",
                IconName::Refresh.svg(c.muted),
                c,
                cx,
                |this, side, cx| this.reload(side, cx),
            ))
            .child(
                icon_toggle_button(
                    match side {
                        Side::Local => "sftp-local-hidden",
                        Side::Remote => "sftp-remote-hidden",
                    },
                    c.palette,
                    if pane.show_hidden {
                        IconName::Eye
                    } else {
                        IconName::EyeOff
                    },
                    pane.show_hidden,
                )
                .on_click(
                    cx.listener(move |this, _: &ClickEvent, _w, cx| this.toggle_hidden(side, cx)),
                ),
            );
        if side == Side::Remote {
            let hid = self.host_id.clone();
            toolbar = toolbar.child(
                button("sftp-remote-term", c.palette, ButtonVariant::Ghost, ButtonSize::Xs)
                    .child(IconName::Terminal.svg(c.muted).size(px(13.0)))
                    .child("Term")
                    .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                        cx.emit(SftpEvent::OpenRemoteTerminal {
                            host_id: hid.clone(),
                        });
                    })),
            );
        }

        let toolbar = div()
            .flex()
            .flex_col()
            .child(header)
            .child(toolbar)
            .when(pane.search_open, |d| {
                d.child(self.render_search_row(side, pane, c, window, cx))
            });

        // Body
        let body: gpui::AnyElement = if side == Side::Remote {
            match &self.conn {
                Conn::Connecting => text_center("Connecting\u{2026}", c.muted),
                Conn::Error(e) => self.render_conn_error(e, c, cx),
                Conn::Ready => self.render_list(side, pane, c, cx),
            }
        } else {
            self.render_list(side, pane, c, cx)
        };

        let bhandler_side = side;
        let drop_fill = c.drop;
        let pane_focus = match side {
            Side::Local => &self.local_focus,
            Side::Remote => &self.remote_focus,
        };
        // Phase 4 drag feedback: the pane a row-drag originates from dims
        // slightly; the *other* pane (the only valid drop target) shows a
        // persistent "Upload/Download here" hint for the whole drag, not
        // just on pixel-precise hover — `drag_over` below still supplies the
        // extra hover tint on top of this for the exact target feedback.
        let is_drag_source = self.dragging_side == Some(side);
        let drop_hint = match self.dragging_side {
            Some(from) if from != side => Some(match side {
                Side::Remote => ("Upload here", IconName::ArrowUp),
                Side::Local => ("Download here", IconName::Download),
            }),
            _ => None,
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .when(is_drag_source, |d| d.opacity(0.6))
            .child(toolbar)
            .child(
                div()
                    .id(match side {
                        Side::Local => "sftp-local-body",
                        Side::Remote => "sftp-remote-body",
                    })
                    .track_focus(pane_focus)
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| {
                        this.on_list_key(side, ev, window, cx)
                    }))
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                            // Right-click on empty space → menu anchored to the
                            // directory itself (new file / new folder only).
                            let path = match bhandler_side {
                                Side::Local => this.local.path.clone(),
                                Side::Remote => this.remote.path.clone(),
                            };
                            this.menu = Some(Menu {
                                side: bhandler_side,
                                path,
                                is_dir: true,
                                pos: ev.position,
                                confirming_delete: false,
                            });
                            cx.notify();
                        }),
                    )
                    // Directional drop-target highlight: the opposite pane
                    // tints + rings while a row-drag hovers it (T08-002 gap).
                    .drag_over::<SftpDrag>(move |style, drag, _w, _cx| {
                        if drag.from != bhandler_side {
                            style.bg(drop_fill)
                        } else {
                            style
                        }
                    })
                    .on_drop(cx.listener(move |this, d: &SftpDrag, _w, cx| {
                        if d.from != bhandler_side {
                            this.enqueue(d.from, d.paths.clone(), cx);
                        }
                    }))
                    .child(body)
                    .when_some(drop_hint, |el, (label, icon)| {
                        el.child(
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap_1()
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .bg(c.card.opacity(0.92))
                                        .border_1()
                                        .border_color(c.palette.primary)
                                        .child(icon.svg(c.palette.primary).size(px(20.0)))
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .font_weight(gpui::FontWeight::MEDIUM)
                                                .text_color(c.fg)
                                                .child(label),
                                        ),
                                ),
                        )
                    }),
            )
            .into_any_element()
    }

    // T20-003: a click-to-edit path field (view + a `KeyDownEvent`-driven
    // edit box on click) — same shape as `panel-scm`'s `text_field` and the
    // settings-ui click-to-edit triggers; no `ui-kit` text-input primitive
    // fits this focus-toggling pattern, documented exception.
    fn render_path_bar(
        &self,
        side: Side,
        pane: &Pane,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if pane.path_editing {
            let show_caret = self.edit_focused && self.blink.read(cx).visible();
            div()
                .id(match side {
                    Side::Local => "sftp-local-path",
                    Side::Remote => "sftp-remote-path",
                })
                .track_focus(&self.edit_focus)
                .flex_1()
                .flex()
                .items_center()
                .px_1()
                .text_xs()
                .font_family("monospace")
                .rounded_sm()
                .border_1()
                .border_color(c.accent)
                .bg(c.card)
                .child(SharedString::from(pane.path_buffer.clone()))
                .when(show_caret, |d| d.child(caret(c.fg, 12.0)))
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                    this.on_path_key(side, ev, cx)
                }))
                .into_any_element()
        } else {
            // Clickable breadcrumb segments (one per path component) instead
            // of a single-line text field — the free-text editor above
            // remains reachable via the trailing pencil button.
            let segments = path_segments(&pane.path);
            let last_idx = segments.len().saturating_sub(1);
            let crumbs = div()
                .id(match side {
                    Side::Local => "sftp-local-pathview",
                    Side::Remote => "sftp-remote-pathview",
                })
                .flex_1()
                .flex()
                .items_center()
                .px_1()
                .overflow_hidden()
                .children(segments.into_iter().enumerate().map(|(i, (label, seg_path))| {
                    let is_last = i == last_idx;
                    let mut crumb = div()
                        .id(SharedString::from(format!("sftp-crumb-{}-{}", side_key(side), i)))
                        .flex_none()
                        .px(px(2.0))
                        .text_xs()
                        .font_family("monospace")
                        .rounded_sm()
                        .text_color(if is_last { c.fg } else { c.muted })
                        .child(SharedString::from(label));
                    if !is_last {
                        crumb = crumb.cursor_pointer().hover(|s| s.bg(c.card)).on_click(
                            cx.listener(move |this, _: &ClickEvent, _window, cx| {
                                this.navigate(side, seg_path.clone(), cx);
                            }),
                        );
                    }
                    if is_last {
                        crumb.into_any_element()
                    } else {
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .child(crumb)
                            .child(IconName::ChevronRight.svg(c.muted).size(px(10.0)))
                            .into_any_element()
                    }
                }))
                .into_any_element();

            div()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(crumbs)
                .child(
                    div()
                        .id(match side {
                            Side::Local => "sftp-local-path-edit",
                            Side::Remote => "sftp-remote-path-edit",
                        })
                        .flex_none()
                        .cursor_pointer()
                        .rounded_sm()
                        .p(px(2.0))
                        .hover(|s| s.bg(c.card))
                        .child(IconName::Pencil.svg(c.muted).size(px(12.0)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            let p = match side {
                                Side::Local => &mut this.local,
                                Side::Remote => &mut this.remote,
                            };
                            p.path_buffer = p.path.clone();
                            p.path_editing = true;
                            window.focus(&this.edit_focus);
                            cx.notify();
                        })),
                )
                .into_any_element()
        }
    }

    /// Inline name-filter box shown under the toolbar when search is toggled.
    fn render_search_row(
        &self,
        side: Side,
        pane: &Pane,
        c: Colors,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let q = pane.search_query.clone();
        let focused = self.edit_focus.is_focused(window);
        let show_caret = focused && self.blink.read(cx).visible();
        let mut field = div()
            .id(match side {
                Side::Local => "sftp-local-searchbox",
                Side::Remote => "sftp-remote-searchbox",
            })
            .track_focus(&self.edit_focus)
            .flex_1()
            .flex()
            .items_center()
            .px_1()
            .text_xs()
            .rounded_sm()
            .border_1()
            .border_color(c.accent)
            .bg(c.card)
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                this.on_search_key(side, ev, cx)
            }));
        if !q.is_empty() {
            field = field.child(div().text_color(c.fg).child(SharedString::from(q.clone())));
        }
        if show_caret {
            field = field.child(caret(c.fg, 12.0));
        }
        if q.is_empty() {
            field = field.child(
                div()
                    .when(focused, |d| d.pl(px(4.0)))
                    .text_color(c.muted)
                    .child("Filter by name\u{2026}"),
            );
        }
        div()
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .py(px(2.0))
            .border_b_1()
            .border_color(c.border)
            .child(field)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn tool_btn(
        &self,
        side: Side,
        id: &'static str,
        glyph: impl IntoElement,
        c: Colors,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, Side, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        button(id, c.palette, ButtonVariant::Ghost, ButtonSize::IconXs)
            .child(glyph)
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| handler(this, side, cx)))
    }

    fn render_list(
        &self,
        side: Side,
        pane: &Pane,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if pane.loading && pane.entries.is_empty() {
            return text_center("Loading\u{2026}", c.muted);
        }

        // Uniform row height (driven by `TreeRow`'s density tokens) so
        // `uniform_list` can virtualise — only the on-screen `[range]` window
        // is turned into elements, keeping large directories cheap (mirrors
        // panel-explorer / panel-scm).

        // The inline rename / new-file / new-folder text field is rendered as
        // a pinned row *above* the virtualised list, never inside the
        // `uniform_list` closure: that closure runs in a deferred layout pass
        // (and again while measuring row 0), and `track_focus` must not run
        // there. During a rename the edited entry is dropped from the list so
        // it isn't shown twice.
        let editing = pane.edit.is_some();
        let rename_orig = pane
            .edit
            .as_ref()
            .filter(|s| s.kind == EditKind::Rename)
            .and_then(|s| s.orig.clone());

        // Per-frame owned snapshot: the `uniform_list` closure is `'static`
        // and cannot borrow `pane`.
        let mut entries: Vec<Entry> = pane
            .visible()
            .into_iter()
            .filter(|e| rename_orig.as_deref() != Some(e.path.as_str()))
            .cloned()
            .collect();

        // Synthetic `..` row at the top of every non-root directory (unless a
        // name filter is active — `..` never matches a query).
        let show_up = self.shows_up_row(side);
        if show_up {
            let parent = match side {
                Side::Local => std::path::Path::new(&pane.path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| pane.path.clone()),
                Side::Remote => parent_path(&pane.path),
            };
            entries.insert(0, Entry::parent_link(parent));
        }

        let list_id = match side {
            Side::Local => "sftp-list-local",
            Side::Remote => "sftp-list-remote",
        };
        let selected = pane.selected.clone();
        let view = cx.entity();

        // SFTP (russh-sftp, SFTPv3) has no birth/creation-time attribute —
        // `Created` is only ever meaningful for the local pane (see
        // `Entry::from_remote`, which always sets `created_at: 0`). Filter
        // it out of the remote pane's columns instead of silently rendering
        // a permanent "—".
        let cols: Vec<(SftpColumn, gpui::Pixels)> = self
            .columns
            .iter()
            .filter(|col| !(side == Side::Remote && **col == SftpColumn::Created))
            .map(|col| (*col, px(self.col_widths[column_ord(*col)])))
            .collect();
        let rr = RowRender {
            side,
            zebra: self.zebra,
            relative_times: self.relative_times,
            now: now_secs(),
            columns: cols.clone(),
            flashing: pane.flashing.clone(),
        };

        if entries.is_empty() {
            return div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .when(editing, |d| d.child(self.render_inline_input(side, c, cx)))
                .child(self.render_col_header(side, &cols, c, cx))
                .child(text_center("Empty directory", c.muted))
                .into_any_element();
        }

        let is_active_pane = self.active_pane == side;

        // Marquee (rubber-band) selection: the virtualised list has no
        // per-row geometry to hit-test against, so a window-relative mouse Y
        // is converted to a row index via the list's own `ScrollHandle`
        // bounds/offset (public API — `logical_scroll_top_index()` is
        // test-only and unusable here) and the fixed `TreeRow` height.
        let total_rows = entries.len();
        let row_paths: Vec<(String, bool)> = entries
            .iter()
            .map(|e| (e.path.clone(), e.is_parent_link))
            .collect();
        let row_h = f32::from(Density::from_palette(&c.palette).tree_row_height());
        let scroll_handle = pane.scroll.clone();

        let list = uniform_list(list_id, entries.len(), move |range, _win, cx| {
            range
                .map(|i| {
                    let entry = &entries[i];
                    let row_selected = selected.contains(entry.path.as_str());
                    sftp_row_element(
                        entry,
                        &rr,
                        i,
                        row_selected,
                        is_active_pane && row_selected,
                        c,
                        &view,
                        &selected,
                        cx,
                    )
                })
                .collect::<Vec<_>>()
        })
        .on_mouse_down(MouseButton::Left, {
            let scroll_handle = scroll_handle.clone();
            cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                let raw = row_index_raw(&scroll_handle, row_h, ev.position.y);
                if raw >= 0 && (raw as usize) < total_rows {
                    // Landed on a real row — its own click/drag handlers own
                    // this gesture.
                    return;
                }
                let start = raw.clamp(0, total_rows.saturating_sub(1) as isize) as usize;
                let pane = this.pane(side);
                let base = if ev.modifiers.secondary() {
                    pane.selected.clone()
                } else {
                    std::collections::BTreeSet::new()
                };
                pane.selected = base.clone();
                pane.marquee = Some(MarqueeState {
                    start_index: start,
                    current_index: start,
                    base,
                });
                cx.notify();
            })
        })
        .on_mouse_move({
            let scroll_handle = scroll_handle.clone();
            let row_paths = row_paths.clone();
            cx.listener(move |this, ev: &MouseMoveEvent, _window, cx| {
                if ev.pressed_button != Some(MouseButton::Left) {
                    return;
                }
                let pane = this.pane(side);
                let Some(marquee) = pane.marquee.as_mut() else {
                    return;
                };
                let raw = row_index_raw(&scroll_handle, row_h, ev.position.y);
                let cur = raw.clamp(0, total_rows.saturating_sub(1) as isize) as usize;
                if cur == marquee.current_index {
                    return;
                }
                marquee.current_index = cur;
                let start = marquee.start_index;
                let mut sel = marquee.base.clone();
                sel.extend(rows_in_range(&row_paths, start, cur));
                pane.selected = sel;
                cx.notify();
            })
        })
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _ev: &MouseUpEvent, _window, cx| {
                let pane = this.pane(side);
                if pane.marquee.take().is_some() {
                    cx.notify();
                }
            }),
        )
        .track_scroll(pane.scroll.clone())
        .flex_1();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .when(editing, |d| d.child(self.render_inline_input(side, c, cx)))
            .child(self.render_col_header(side, &cols, c, cx))
            .child(list)
            .into_any_element()
    }

    /// Sticky column-header row above the file list. Each metadata header is
    /// a drag source + drop target (reorder, persisted to settings) with a
    /// right-edge resize handle.
    fn render_col_header(
        &self,
        side: Side,
        cols: &[(SftpColumn, gpui::Pixels)],
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (sort_col, sort_dir) = match side {
            Side::Local => (self.local.sort_col, self.local.sort_dir),
            Side::Remote => (self.remote.sort_col, self.remote.sort_dir),
        };
        let arrow = move |active: bool| -> Option<gpui::AnyElement> {
            if !active {
                return None;
            }
            let icon = if sort_dir == SortDir::Asc {
                IconName::ChevronUp
            } else {
                IconName::ChevronDown
            };
            Some(icon.svg(c.muted).size(px(10.0)).flex_none().into_any_element())
        };
        let head = move |text: &str, active: bool| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_size(px(9.5))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(c.muted)
                .child(SharedString::from(text.to_string()))
                .children(arrow(active))
        };

        let mut row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(22.0))
            .border_b_1()
            .border_color(c.border)
            .bg(c.card)
            // icon gutter + name column
            .child(div().w(px(20.0)).flex_shrink_0())
            .child(
                div()
                    .id(SharedString::from(format!("sftp-colh-{}-name", side_key(side))))
                    .flex_1()
                    .min_w_0()
                    .cursor_pointer()
                    .child(head("NAME", sort_col == SortKey::Name))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.toggle_sort(side, SortKey::Name, cx);
                    })),
            );

        for (col, width) in cols.iter().copied() {
            let moved = col;
            let is_sorted = sort_col == SortKey::from_column(col);
            row = row.child(
                div()
                    .id(SharedString::from(format!("sftp-colh-{}-{}", side_key(side), col.token())))
                    .relative()
                    .flex_shrink_0()
                    .w(width)
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .child(head(col.header(), is_sorted))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.toggle_sort(side, SortKey::from_column(moved), cx);
                    }))
                    .on_drag(ColHeaderDrag { col }, |_, _, _, cx| cx.new(|_| DragGhost))
                    .drag_over::<ColHeaderDrag>(move |s, drag, _w, _cx| {
                        if drag.col != moved {
                            s.border_l_2().border_color(c.accent)
                        } else {
                            s
                        }
                    })
                    .on_drop(cx.listener(move |this, drag: &ColHeaderDrag, _w, cx| {
                        this.reorder_column(drag.col, moved, cx);
                    }))
                    // Width follows the pointer while the right-edge grip is
                    // dragged (`ev.bounds` is this cell).
                    .on_drag_move(cx.listener(
                        move |this, ev: &gpui::DragMoveEvent<ColResizeDrag>, _w, cx| {
                            if ev.drag(cx).col != moved {
                                return;
                            }
                            let want = f32::from(ev.event.position.x - ev.bounds.origin.x);
                            let cur = this.col_widths[column_ord(moved)];
                            this.resize_column(moved, want - cur, cx);
                        },
                    ))
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "sftp-colh-grip-{}-{}",
                                side_key(side),
                                col.token()
                            )))
                            .absolute()
                            .right_0()
                            .top_0()
                            .h_full()
                            .w(px(4.0))
                            .cursor_col_resize()
                            .hover(|s| s.bg(c.accent))
                            .on_drag(ColResizeDrag { col }, |_, _, _, cx| cx.new(|_| DragGhost)),
                    ),
            );
        }
        row.into_any_element()
    }

    fn render_inline_input(
        &self,
        side: Side,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let buffer = match side {
            Side::Local => self.local.edit_buffer.clone(),
            Side::Remote => self.remote.edit_buffer.clone(),
        };
        let show_caret = self.edit_focused && self.blink.read(cx).visible();
        div()
            .flex()
            .flex_row()
            .items_center()
            .px_2()
            .py(px(1.0))
            .child(
                div()
                    .id("sftp-inline-input")
                    .track_focus(&self.edit_focus)
                    .flex_1()
                    .flex()
                    .items_center()
                    .px_1()
                    .text_sm()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.accent)
                    .bg(c.card)
                    .child(SharedString::from(buffer))
                    .when(show_caret, |d| d.child(caret(c.fg, 14.0)))
                    .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _w, cx| {
                        this.on_edit_key(side, ev, cx)
                    })),
            )
    }

    fn render_conn_error(&self, _msg: &str, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_color(c.muted)
                    .child("SFTP connection unavailable"),
            )
            .child(
                button(
                    "sftp-retry",
                    c.palette,
                    ButtonVariant::Outline,
                    ButtonSize::Xs,
                )
                .child("Retry")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.connect(cx))),
            )
            .into_any_element()
    }

    fn render_menu(&self, _c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(menu) = self.menu.as_ref() else {
            return div().into_any_element();
        };
        let side = menu.side;
        let pos = menu.pos;
        let has_entry = self.menu_entry().is_some();
        let (is_dir, is_remote) = (menu.is_dir, side == Side::Remote);
        let confirming = menu.confirming_delete;
        let entry = self.menu_entry().map(|(_, e)| e);
        let view = cx.entity();

        let run = |f: SftpAct| -> MenuClick {
            let v = view.clone();
            Box::new(move |_ev, _w, cx| {
                v.update(cx, |this, cx| f(this, cx));
            })
        };

        let mut items: Vec<MenuItem> = Vec::new();
        if has_entry {
            let e = entry.clone().unwrap();
            items.push(MenuItem::new("sftp-cm-open", "Open").on_click(run(Box::new(
                move |this, cx| {
                    this.menu = None;
                    this.activate(side, &e, true, cx);
                },
            ))));
            items.push(MenuItem::separator());
        }
        items.push(
            MenuItem::new("sftp-cm-newdir", "New Folder")
                .icon(IconName::Folder)
                .on_click(run(Box::new(move |this, cx| {
                    this.start_edit(side, EditKind::NewDir, cx)
                }))),
        );
        items.push(
            MenuItem::new("sftp-cm-newfile", "New File")
                .icon(IconName::File)
                .on_click(run(Box::new(move |this, cx| {
                    this.start_edit(side, EditKind::NewFile, cx)
                }))),
        );
        if has_entry {
            items.push(
                MenuItem::new("sftp-cm-rename", "Rename")
                    .icon(IconName::Pencil)
                    .on_click(run(Box::new(move |this, cx| {
                        this.start_edit(side, EditKind::Rename, cx)
                    }))),
            );
            let transfer_label = if is_remote {
                "Download to Local\u{2026}"
            } else {
                "Upload to Remote\u{2026}"
            };
            items.push(
                MenuItem::new("sftp-cm-transfer", transfer_label)
                    .icon(IconName::Download)
                    .on_click(run(Box::new(move |this, cx| {
                        if let Some((s, e)) = this.menu_entry() {
                            this.enqueue(s, vec![e.path], cx);
                        }
                    }))),
            );
            items.push(MenuItem::separator());
            items.push(
                MenuItem::new("sftp-cm-copypath", "Copy Path")
                    .icon(IconName::Copy)
                    .on_click(run(Box::new(move |this, cx| {
                        if let Some((_, e)) = this.menu_entry() {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(e.path));
                        }
                        this.menu = None;
                        cx.notify();
                    }))),
            );
            items.push(
                MenuItem::new("sftp-cm-copyname", "Copy Name")
                    .icon(IconName::Copy)
                    .on_click(run(Box::new(move |this, cx| {
                        if let Some((_, e)) = this.menu_entry() {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(e.name));
                        }
                        this.menu = None;
                        cx.notify();
                    }))),
            );
        }
        if is_remote && has_entry {
            let e = entry.clone().unwrap();
            let ep = e.clone();
            items.push(
                MenuItem::new("sftp-cm-perm", "Permissions\u{2026}").on_click(run(Box::new(
                    move |this, cx| this.open_perm_dialog(&ep, cx),
                ))),
            );
            let ep = e.clone();
            items.push(
                MenuItem::new("sftp-cm-props", "Properties\u{2026}")
                    .icon(IconName::Info)
                    .on_click(run(Box::new(move |this, cx| this.open_props(&ep, cx)))),
            );
            if !is_dir {
                let ep = e.clone();
                items.push(
                    MenuItem::new("sftp-cm-editremote", "Edit Remote File")
                        .icon(IconName::Pencil)
                        .on_click(run(Box::new(move |this, cx| {
                            cx.emit(SftpEvent::OpenRemoteFile {
                                session_id: this.session_id.clone(),
                                remote_path: ep.path.clone(),
                                host_id: this.host_id.clone(),
                            });
                            this.menu = None;
                            cx.notify();
                        }))),
                );
            }
        }
        if !is_remote {
            // The target is the right-clicked entry, or — for a background
            // right-click (`!has_entry`) — the pane's current directory
            // itself (`menu.path`, set to `pane.path` in that case).
            let reveal_path = entry
                .as_ref()
                .map(|e| e.path.clone())
                .unwrap_or_else(|| menu.path.clone());
            let cwd = match &entry {
                Some(e) if e.is_dir => e.path.clone(),
                Some(e) => std::path::Path::new(&e.path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| menu.path.clone()),
                None => menu.path.clone(),
            };
            items.push(MenuItem::separator());
            {
                let v = view.clone();
                items.push(
                    MenuItem::new("sftp-cm-reveal", "Reveal in Finder")
                        .icon(IconName::FolderOpen)
                        .on_click(move |_ev, _w, cx| {
                            cx.reveal_path(std::path::Path::new(&reveal_path));
                            v.update(cx, |this, cx| {
                                this.menu = None;
                                cx.notify();
                            });
                        }),
                );
            }
            items.push(
                MenuItem::new("sftp-cm-terminal", "Open Terminal Here")
                    .icon(IconName::Terminal)
                    .on_click(run(Box::new(move |this, cx| {
                        this.menu = None;
                        cx.emit(SftpEvent::OpenLocalTerminal { cwd: cwd.clone() });
                        cx.notify();
                    }))),
            );
        }
        items.push(MenuItem::separator());
        items.push(
            MenuItem::new("sftp-cm-refresh", "Refresh")
                .icon(IconName::Refresh)
                .on_click(run(Box::new(move |this, cx| {
                    this.menu = None;
                    this.reload(side, cx);
                }))),
        );
        if has_entry {
            let label = if confirming {
                "Click again to delete"
            } else {
                "Delete\u{2026}"
            };
            items.push(
                MenuItem::new("sftp-cm-delete", label)
                    .icon(IconName::Trash)
                    .destructive()
                    .on_click({
                        let v = view.clone();
                        move |_ev, _w, cx| {
                            v.update(cx, |this, cx| {
                                let Some(menu) = this.menu.as_mut() else {
                                    return;
                                };
                                if menu.confirming_delete {
                                    let (side, path) = (menu.side, menu.path.clone());
                                    this.delete(side, path, cx);
                                } else {
                                    menu.confirming_delete = true;
                                    cx.notify();
                                }
                            });
                        }
                    }),
            );
        }

        let v = view.clone();
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            v.update(cx, |this, cx| {
                this.menu = None;
                cx.notify()
            });
        };
        context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        )
    }

    fn render_perm_dialog(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(d) = self.perm.as_ref() else {
            return div().into_any_element();
        };
        let dialog_focused = self.dialog_focused && self.blink.read(cx).visible();
        let row = |label: &str, value: String, active: bool| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(56.0))
                        .text_xs()
                        .text_color(c.muted)
                        .child(SharedString::from(label.to_string())),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .px_1()
                        .text_sm()
                        .font_family("monospace")
                        .rounded_sm()
                        .border_1()
                        .border_color(if active { c.accent } else { c.border })
                        .bg(c.card)
                        .child(SharedString::from(value))
                        .when(active && dialog_focused, |d| d.child(caret(c.fg, 14.0))),
                )
        };
        overlay()
            .child(
                dialog_card("sftp-perm-dialog", c)
                    .track_focus(&self.dialog_focus)
                    .on_key_down(
                        cx.listener(|this, ev: &KeyDownEvent, _w, cx| this.on_perm_key(ev, cx)),
                    )
                    .child(div().text_sm().child(SharedString::from(format!(
                        "Permissions \u{2014} {}",
                        d.name
                    ))))
                    .child(row("Octal", d.octal.clone(), d.field == PermField::Octal))
                    .child(row("Owner", d.owner.clone(), d.field == PermField::Owner))
                    .child(row("Group", d.group.clone(), d.field == PermField::Group))
                    .child(
                        div().text_xs().text_color(c.muted).child(
                            "Tab switches field \u{00b7} Enter applies \u{00b7} Esc cancels",
                        ),
                    )
                    .when_some(d.error.clone(), |el, e| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(c.err)
                                .child(SharedString::from(e)),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .child(dialog_btn("perm-cancel", "Cancel", c, false).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| {
                                    this.perm = None;
                                    cx.notify();
                                }),
                            ))
                            .child(dialog_btn("perm-apply", "Apply", c, true).on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.apply_perm(cx)),
                            )),
                    ),
            )
            .into_any_element()
    }

    fn render_props_dialog(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(d) = self.props.as_ref() else {
            return div().into_any_element();
        };
        let e = &d.entry;
        let kind = if e.is_dir {
            "Directory".to_string()
        } else if e.is_symlink {
            format!(
                "Symlink \u{2192} {}",
                e.symlink_target.as_deref().unwrap_or("?")
            )
        } else {
            "File".to_string()
        };
        let size = if e.is_dir {
            d.calculated_size
                .clone()
                .unwrap_or_else(|| "\u{2014}".to_string())
        } else {
            format_bytes(e.size)
        };
        let info =
            |label: &str, value: String| {
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .w(px(72.0))
                            .text_xs()
                            .text_color(c.muted)
                            .child(SharedString::from(label.to_string())),
                    )
                    .child(div().flex_1().text_xs().child(SharedString::from(
                        if value.is_empty() {
                            "\u{2014}".to_string()
                        } else {
                            value
                        },
                    )))
            };
        overlay()
            .child(
                dialog_card("sftp-props-dialog", c)
                    .child(div().text_sm().child(SharedString::from(format!(
                        "Properties \u{2014} {}",
                        e.name
                    ))))
                    .child(info("Name", e.name.clone()))
                    .child(info("Path", e.path.clone()))
                    .child(info("Type", kind))
                    .child(info("Size", size))
                    .child(info("Permissions", e.permissions.clone()))
                    .child(info("Modified", format_epoch(e.modified_at)))
                    .when(e.is_dir, |el| {
                        el.child(
                            dialog_btn(
                                "props-calc",
                                if d.calculating {
                                    "Calculating\u{2026}"
                                } else {
                                    "Calculate size"
                                },
                                c,
                                false,
                            )
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.calc_size(cx)),
                            ),
                        )
                    })
                    .child(
                        dialog_btn("props-close", "Close", c, true).on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.props = None;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .into_any_element()
    }
}

/// Minimal drag preview — the cursor + drop-target highlighting do the work.
struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Pointer-drag of a column header onto another header → reorder.
#[derive(Clone)]
struct ColHeaderDrag {
    col: SftpColumn,
}

/// Pointer-drag of a column header's right-edge grip → resize.
#[derive(Clone)]
struct ColResizeDrag {
    col: SftpColumn,
}

/// Pointer-drag of the grip between the local and remote panes → resize.
#[derive(Clone)]
struct SplitDrag;

/// Which resize gesture is live between drag-start and the next mouse-up —
/// drives the one-shot settings persist at drag end (see [`SftpView::resizing`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeKind {
    Column(SftpColumn),
    Split,
}

/// Per-frame render parameters shared by every row (the `uniform_list`
/// closure is `'static` and can't borrow the view).
#[derive(Clone)]
struct RowRender {
    side: Side,
    zebra: bool,
    relative_times: bool,
    now: i64,
    columns: Vec<(SftpColumn, gpui::Pixels)>,
    /// Destination paths of a just-completed drop — rendered with a brief
    /// landing highlight (Phase 4 drop feedback).
    flashing: std::collections::BTreeSet<String>,
}

fn side_key(side: Side) -> &'static str {
    match side {
        Side::Local => "L",
        Side::Remote => "R",
    }
}

/// One metadata cell's text for `entry`.
fn column_cell_value(col: SftpColumn, entry: &Entry, rr: &RowRender) -> String {
    if entry.is_parent_link {
        return String::new();
    }
    match col {
        SftpColumn::Size => {
            if entry.is_dir {
                String::new()
            } else {
                format_bytes(entry.size)
            }
        }
        SftpColumn::Modified => {
            if rr.relative_times {
                format_relative_epoch(entry.modified_at, rr.now)
            } else {
                format_epoch(entry.modified_at)
            }
        }
        SftpColumn::Created => {
            if rr.relative_times {
                format_relative_epoch(entry.created_at, rr.now)
            } else {
                format_epoch(entry.created_at)
            }
        }
        SftpColumn::Permissions => entry.permissions.clone(),
        SftpColumn::Type => {
            if entry.is_dir || entry.is_symlink {
                "\u{2014}".to_string()
            } else {
                entry
                    .name
                    .rsplit_once('.')
                    .map(|(_, ext)| ext.to_lowercase())
                    .unwrap_or_else(|| "\u{2014}".to_string())
            }
        }
    }
}

/// Window-relative Y → row index (unclamped) for the marquee-select hit
/// test, via the `uniform_list`'s own `ScrollHandle` bounds/offset. Not
/// unit-tested: `ScrollHandle` bounds/offset are only populated by GPUI's
/// layout pass, not constructible in a plain unit test.
fn row_index_raw(scroll: &UniformListScrollHandle, row_h: f32, y: gpui::Pixels) -> isize {
    let base = scroll.0.borrow().base_handle.clone();
    let local_y = f32::from(y) - f32::from(base.bounds().top()) - f32::from(base.offset().y);
    (local_y / row_h).floor() as isize
}

/// One virtualised file/folder row. Free function (not a `&self` method) so it
/// can be built inside the `uniform_list` render closure, which only gets
/// `&mut App`; handlers reach the view through `view.update(..)` — the same
/// shape as `panel-explorer`'s `explorer_row_element`.
#[allow(clippy::too_many_arguments)]
fn sftp_row_element(
    entry: &Entry,
    rr: &RowRender,
    index: usize,
    selected: bool,
    focused: bool,
    c: Colors,
    view: &Entity<SftpView>,
    all_selected: &std::collections::BTreeSet<String>,
    cx: &mut App,
) -> gpui::AnyElement {
    let side = rr.side;
    let is_up = entry.is_parent_link;

    let glyph: SharedString = if entry.is_symlink && !is_up {
        IconName::Link.path().into()
    } else {
        let store = view.read(cx).theme.read(cx);
        // Same icon-theme resolution the sidebar Explorer uses.
        icon_for_path(
            store.icon_theme(),
            if entry.is_dir { "" } else { &entry.name },
            entry.is_dir,
            false,
        )
    };
    let id: SharedString = format!("row:{}:{}", side_key(side), entry.path).into();

    // Zebra: tint every other *data* row (index 0 = first entry).
    let zebra_fill =
        (rr.zebra && !selected && index % 2 == 1).then_some(c.zebra);
    // Phase 4: a row whose path just received a dropped transfer gets a
    // brief landing highlight (cleared by `enqueue`'s spawned timer).
    let just_landed = !selected && rr.flashing.contains(&entry.path);

    let on_click = {
        let v = view.clone();
        let entry = entry.clone();
        move |ev: &ClickEvent, _w: &mut Window, cx: &mut App| {
            v.update(cx, |this, cx| {
                // `..` navigates on a single click; a real double-click always
                // opens — neither takes the Cmd/Shift multi-select branches.
                let dbl = entry.is_parent_link || ev.click_count() >= 2;
                if !dbl {
                    let mods = ev.modifiers();
                    if mods.secondary() {
                        // Cmd (macOS) / Ctrl (elsewhere): toggle membership.
                        let pane = this.pane(side);
                        if !pane.selected.remove(&entry.path) {
                            pane.selected.insert(entry.path.clone());
                            pane.anchor = Some(entry.path.clone());
                        }
                        cx.notify();
                        return;
                    }
                    if mods.shift {
                        let pane = this.pane(side);
                        let anchor = pane.anchor.clone().unwrap_or_else(|| entry.path.clone());
                        let visible: Vec<String> =
                            pane.visible().into_iter().map(|e| e.path.clone()).collect();
                        for p in path_range(&visible, &anchor, &entry.path) {
                            pane.selected.insert(p);
                        }
                        cx.notify();
                        return;
                    }
                }
                this.activate(side, &entry, dbl, cx);
            });
        }
    };
    let on_right_click = {
        let v = view.clone();
        let path = entry.path.clone();
        let is_dir = entry.is_dir;
        move |ev: &MouseDownEvent, _w: &mut Window, cx: &mut App| {
            if is_up {
                return;
            }
            v.update(cx, |this, cx| {
                let pane = this.pane(side);
                pane.selected.clear();
                pane.selected.insert(path.clone());
                pane.anchor = Some(path.clone());
                this.menu = Some(Menu {
                    side,
                    path: path.clone(),
                    is_dir,
                    pos: ev.position,
                    confirming_delete: false,
                });
                cx.notify();
            });
        }
    };
    // Dragging a row that's part of the current multi-selection drags the
    // whole selection; dragging an unselected row drags just that row
    // (matches Finder/Explorer: dragging outside the selection replaces it
    // implicitly for the purpose of the drag).
    let drag_paths: Vec<String> = if selected && all_selected.len() > 1 {
        all_selected.iter().cloned().collect()
    } else {
        vec![entry.path.clone()]
    };
    // Cloned at function scope (owned `Entity<SftpView>`, not the borrowed
    // `&Entity<SftpView>` parameter) so the `'static` `.extra()`/`.on_drag`
    // closures below can move it in without borrowing past this call.
    let drag_view = view.clone();

    let cells_row = div()
        .flex()
        .items_center()
        .children(rr.columns.iter().enumerate().map(|(ci, (col, width))| {
            let text = column_cell_value(*col, entry, rr);
            // `.tooltip()` is a `StatefulInteractiveElement` method — needs
            // `.id()` first to become a `Stateful<Div>`.
            let mut cell = div()
                .id(SharedString::from(format!(
                    "sftp-cell-{}-{}-{}",
                    side_key(side),
                    entry.path,
                    ci
                )))
                .flex_shrink_0()
                .w(*width)
                .text_xs()
                .text_color(c.muted);
            if *col == SftpColumn::Permissions {
                cell = cell.font_family("monospace");
            }
            // Relative time is a compact display value; the exact absolute
            // timestamp is a hover away rather than dropped entirely.
            if rr.relative_times && !is_up {
                let abs = match col {
                    SftpColumn::Modified => Some(format_epoch(entry.modified_at)),
                    SftpColumn::Created => Some(format_epoch(entry.created_at)),
                    _ => None,
                };
                if let Some(abs) = abs {
                    let abs = SharedString::from(abs);
                    cell = cell.tooltip(move |window, cx| Tooltip::new(abs.clone()).build(window, cx));
                }
            }
            cell.child(SharedString::from(text)).into_any_element()
        }));

    // `TreeRow` only carries a single label colour (no italic channel) — the
    // symlink/`..` muted tint survives the migration, the italic styling
    // doesn't (accepted, tracked as a minor visual regression).
    let label_tint = (entry.is_symlink || is_up).then_some(c.muted);
    // Dotfiles are only ever rendered when "show hidden" is on — dim the
    // whole row so they stay visually marked as hidden instead of blending
    // in with regular entries.
    let is_hidden = !is_up && entry.name.starts_with('.');

    tree_row(id, c.palette, SharedString::from(entry.name.clone()))
        .icon_path(Some(glyph))
        .chevron(None)
        .trailing(cells_row)
        .when_some(label_tint, |row, tint| row.label_tint(tint))
        .state(TreeRowState {
            selected,
            focused,
            ..Default::default()
        })
        .on_click(on_click)
        .on_secondary_down(on_right_click)
        .extra(move |row| {
            let mut row = row;
            if let Some(fill) = zebra_fill {
                row = row.bg(fill);
            } else if just_landed {
                row = row.bg(c.palette.selected_accent.opacity(0.28));
            }
            if is_hidden {
                row = row.opacity(0.55);
            }
            if !is_up {
                row = row.on_drag(
                    SftpDrag {
                        from: side,
                        paths: drag_paths,
                    },
                    move |_, _, _, cx| {
                        drag_view.update(cx, |this, cx| {
                            this.dragging_side = Some(side);
                            cx.notify();
                        });
                        cx.new(|_| DragGhost)
                    },
                );
            }
            row
        })
        .into_any_element()
}

fn text_center(msg: &str, color: gpui::Hsla) -> gpui::AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(color)
        .child(SharedString::from(msg.to_string()))
        .into_any_element()
}

fn overlay() -> gpui::Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(crate::theme::modal_scrim())
}

fn dialog_card(id: &'static str, c: Colors) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .w(px(360.0))
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(c.border)
        .bg(c.card)
        .text_color(c.fg)
        .shadow_lg()
}

fn dialog_btn(
    id: &'static str,
    label: &'static str,
    c: Colors,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    let variant = if primary {
        ButtonVariant::Default
    } else {
        ButtonVariant::Outline
    };
    button(id, c.palette, variant, ButtonSize::Xs).child(label)
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_path_cases() {
        assert_eq!(parent_path("/"), "/");
        assert_eq!(parent_path(""), "/");
        assert_eq!(parent_path("/a"), "/");
        assert_eq!(parent_path("/a/b/c"), "/a/b");
        assert_eq!(parent_path("/a/b/"), "/a");
    }

    #[test]
    fn split_ratio_from_drag_divides_and_clamps() {
        assert_eq!(split_ratio_from_drag(400.0, 800.0), 0.5);
        assert_eq!(
            split_ratio_from_drag(10.0, 800.0),
            0.2,
            "clamps below the allowed range"
        );
        assert_eq!(
            split_ratio_from_drag(790.0, 800.0),
            0.8,
            "clamps above the allowed range"
        );
        assert_eq!(
            split_ratio_from_drag(400.0, 0.0),
            0.5,
            "not-yet-laid-out width falls back to 50/50"
        );
    }

    #[test]
    fn join_path_single_separator() {
        assert_eq!(join_path("/a", "b"), "/a/b");
        assert_eq!(join_path("/a/", "b"), "/a/b");
        assert_eq!(join_path("/", "b"), "/b");
    }

    #[test]
    fn path_segments_builds_breadcrumb_chain() {
        assert_eq!(path_segments("/"), vec![("/".to_string(), "/".to_string())]);
        assert_eq!(
            path_segments("/Users/foo"),
            vec![
                ("/".to_string(), "/".to_string()),
                ("Users".to_string(), "/Users".to_string()),
                ("foo".to_string(), "/Users/foo".to_string()),
            ]
        );
        assert_eq!(
            path_segments("/a/b/"),
            vec![
                ("/".to_string(), "/".to_string()),
                ("a".to_string(), "/a".to_string()),
                ("b".to_string(), "/a/b".to_string()),
            ],
            "trailing slash doesn't produce an empty trailing segment"
        );
    }

    #[test]
    fn sanitize_rejects_bad_names() {
        assert_eq!(sanitize_entry_name("  foo "), Some("foo".to_string()));
        assert_eq!(sanitize_entry_name(""), None);
        assert_eq!(sanitize_entry_name("."), None);
        assert_eq!(sanitize_entry_name(".."), None);
        assert_eq!(sanitize_entry_name("a/b"), None);
        assert_eq!(sanitize_entry_name("a\\b"), None);
    }

    #[test]
    fn perm_string_to_octal_matches_reference() {
        assert_eq!(perm_string_to_octal("rwxr-xr-x"), "755");
        assert_eq!(perm_string_to_octal("rw-r--r--"), "644");
        assert_eq!(perm_string_to_octal("---------"), "000");
        assert_eq!(perm_string_to_octal(""), "000");
        assert_eq!(perm_string_to_octal("rwxrwxrwx"), "777");
    }

    #[test]
    fn sort_entries_dirs_first_then_case_insensitive() {
        let mk = |name: &str, is_dir: bool| Entry {
            name: name.to_string(),
            path: format!("/{name}"),
            size: 0,
            modified_at: 0,
            created_at: 0,
            is_dir,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
            is_parent_link: false,
        };
        let mut v = vec![
            mk("Zebra", false),
            mk("apple", false),
            mk("Mango", true),
            mk("banana", true),
        ];
        sort_entries(&mut v);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["banana", "Mango", "apple", "Zebra"]);
    }

    #[test]
    fn sort_entries_by_size_keeps_dirs_first_and_respects_direction() {
        let mk = |name: &str, is_dir: bool, size: u64| Entry {
            name: name.to_string(),
            path: format!("/{name}"),
            size,
            modified_at: 0,
            created_at: 0,
            is_dir,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
            is_parent_link: false,
        };
        let mut v = vec![
            mk("big.txt", false, 300),
            mk("folder", true, 0),
            mk("small.txt", false, 10),
        ];
        sort_entries_by(&mut v, SortKey::Size, SortDir::Asc);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        // Dirs-first still wins regardless of the sort column.
        assert_eq!(names, vec!["folder", "small.txt", "big.txt"]);

        sort_entries_by(&mut v, SortKey::Size, SortDir::Desc);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["folder", "big.txt", "small.txt"]);
    }

    #[test]
    fn sort_entries_by_type_groups_extensions_and_dirs_have_no_extension() {
        let mk = |name: &str, is_dir: bool| Entry {
            name: name.to_string(),
            path: format!("/{name}"),
            size: 0,
            modified_at: 0,
            created_at: 0,
            is_dir,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
            is_parent_link: false,
        };
        let mut v = vec![mk("b.rs", false), mk("dir", true), mk("a.md", false)];
        sort_entries_by(&mut v, SortKey::Type, SortDir::Asc);
        let names: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["dir", "a.md", "b.rs"]);
    }

    #[test]
    fn path_range_spans_either_direction_and_falls_back() {
        let v = vec!["/a".to_string(), "/b".to_string(), "/c".to_string(), "/d".to_string()];
        assert_eq!(path_range(&v, "/b", "/d"), vec!["/b", "/c", "/d"]);
        // Shift-clicking back toward the anchor covers the same range.
        assert_eq!(path_range(&v, "/d", "/b"), vec!["/b", "/c", "/d"]);
        assert_eq!(path_range(&v, "/a", "/a"), vec!["/a"]);
        // Anchor no longer visible (e.g. filtered out) — fall back to just the target.
        assert_eq!(path_range(&v, "/missing", "/c"), vec!["/c"]);
    }

    #[test]
    fn rows_in_range_skips_the_up_row_and_spans_either_direction() {
        let rows = vec![
            ("/..".to_string(), true),
            ("/a".to_string(), false),
            ("/b".to_string(), false),
            ("/c".to_string(), false),
        ];
        assert_eq!(rows_in_range(&rows, 0, 2), vec!["/a", "/b"]);
        assert_eq!(rows_in_range(&rows, 2, 0), vec!["/a", "/b"]);
        assert_eq!(rows_in_range(&rows, 1, 1), vec!["/a"]);
    }

    #[test]
    fn format_epoch_known_value() {
        // 2021-01-01 00:00:00 UTC
        assert_eq!(format_epoch(1_609_459_200), "2021-01-01 00:00");
        assert_eq!(format_epoch(0), "\u{2014}");
    }

    #[test]
    fn format_bytes_scales() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    #[test]
    fn pane_visible_respects_hidden_toggle() {
        let mut p = Pane::new("/".to_string());
        let mk = |name: &str| Entry {
            name: name.to_string(),
            path: format!("/{name}"),
            size: 0,
            modified_at: 0,
            created_at: 0,
            is_dir: false,
            is_symlink: false,
            symlink_target: None,
            permissions: String::new(),
            is_parent_link: false,
        };
        p.entries = vec![mk(".hidden"), mk("visible"), mk("other")];
        assert_eq!(p.visible().len(), 2);
        p.show_hidden = true;
        assert_eq!(p.visible().len(), 3);
        p.search_query = "oth".to_string();
        assert_eq!(p.visible().len(), 1);
    }

    #[test]
    fn relative_epoch_buckets() {
        let now = 1_000_000_000;
        assert_eq!(format_relative_epoch(0, now), "\u{2014}");
        assert_eq!(format_relative_epoch(now - 30, now), "just now");
        assert_eq!(format_relative_epoch(now - 120, now), "2m ago");
        assert_eq!(format_relative_epoch(now - 7_200, now), "2h ago");
        assert_eq!(format_relative_epoch(now - 3 * 86_400, now), "3d ago");
        // older than ~30d falls back to an absolute date
        assert_eq!(
            format_relative_epoch(now - 60 * 86_400, now),
            format_epoch(now - 60 * 86_400)
        );
    }

    #[test]
    fn filesystem_root_detection() {
        assert!(is_filesystem_root(Side::Remote, "/"));
        assert!(!is_filesystem_root(Side::Remote, "/etc"));
        assert!(is_filesystem_root(Side::Local, "/"));
        assert!(!is_filesystem_root(Side::Local, "/Users/x"));
    }

    #[test]
    fn type_column_uses_extension() {
        let rr = RowRender {
            side: Side::Local,
            zebra: false,
            relative_times: false,
            now: 0,
            columns: vec![],
            flashing: std::collections::BTreeSet::new(),
        };
        let mut e = Entry::parent_link("/".to_string());
        assert_eq!(column_cell_value(SftpColumn::Type, &e, &rr), "");
        e = Entry {
            is_parent_link: false,
            is_dir: false,
            ..e
        };
        e.name = "notes.MD".to_string();
        assert_eq!(column_cell_value(SftpColumn::Type, &e, &rr), "md");
        e.name = "README".to_string();
        assert_eq!(column_cell_value(SftpColumn::Type, &e, &rr), "\u{2014}");
    }
}
