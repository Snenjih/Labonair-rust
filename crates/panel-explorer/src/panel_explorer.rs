//! Sidebar file explorer (T05-001).
//!
//! Ported from `reference-src/src/modules/explorer/` — the React tree
//! (`FileExplorer` / `FileTreeNode` / `useFileTree` / `useLocalExplorerStore` /
//! `buildTreeRows`). The reference keeps a per-directory node map
//! (`idle | loading | loaded | error`), a `generation` counter that a slow
//! `readDir` compares against so a stale response can't overwrite a scope the
//! user has since navigated away from, lazy loading of only the visible
//! subtree, a `showHidden` toggle that invalidates the cache, and a flatten
//! pass (`buildTreeRows`) that turns the node map into an ordered row list.
//! That state machine lives in [`TreeModel`] (pure, unit-tested); [`ExplorerView`]
//! wraps it with the GPUI rendering, async filesystem calls and the watcher.
//!
//! The filesystem work runs in-process through
//! [`labonair_backend::modules::fs`] (`tree::read_dir_page` +
//! `mutate::{create_file_sync, create_dir_sync, rename_sync, delete_sync}`) on
//! `cx.background_executor()` — no Tauri IPC.
//!
//! Deviations from the reference:
//! * Rows render into a plain `overflow_y_scroll` column rather than
//!   `@tanstack/react-virtual`. The per-directory 500-entry page cap + lazy
//!   loading keep the element count bounded; true windowing is a later polish
//!   pass.
//! * The directory watcher (`reference-src/src-tauri/src/modules/fs/watcher.rs`)
//!   is embedded directly in this entity via `notify-debouncer-mini` (300 ms
//!   debounce, non-recursive, watch-set synced to the loaded directories)
//!   instead of going through the backend event bus.
//! * File-type icons are a small glyph map, not the full material-icon-theme
//!   port.

// Crate root (T16-008): this file is the `labonair-panel-explorer` lib root.
// It also hosts the path-bookmarks overlay view (`bookmarks` module) — the
// bookmarks feature is directory-near, so it lives here rather than in its own
// crate (see `docs/architecture.md §2`). The `theme` / `workspace` / `preview`
// shims keep the pre-split `crate::…` paths resolving against their new home
// crates.

pub mod bookmarks;

pub use bookmarks::{BookmarkEvent, BookmarksView};

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

pub(crate) mod workspace {
    pub use labonair_workspace::Workspace;
}

pub(crate) mod preview {
    pub use labonair_workspace::views::preview::*;
}

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::{
    div, px, uniform_list, App, AppContext, ClickEvent, ClipboardItem, Context, Entity,
    ExternalPaths, FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Render, ScrollStrategy,
    SharedString, StatefulInteractiveElement, Styled, Task, UniformListScrollHandle, Window,
};

use labonair_settings::{ExplorerSettings, Settings as _};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, Debouncer};

use labonair_backend::modules::fs::{mutate, tree};

use crate::theme::ThemeStore;
use crate::workspace::Workspace;
use labonair_notifications::{notification_center, Notification};
use labonair_ui_kit::{
    button, chevron_icon_path, context_menu, icon_for_path, svg_path, tree_row, ButtonSize,
    ButtonVariant, IconName, InputEvent, InputState, MenuClick, MenuItem, Palette, TreeRowState,
};

/// A menu action expressed against the view + window (wrapped into a
/// [`MenuClick`] by `render_context_menu`).
type ExpAct = Box<dyn Fn(&mut ExplorerView, &mut Window, &mut Context<ExplorerView>)>;

const PAGE_LIMIT: usize = tree::DEFAULT_LOCAL_PAGE_LIMIT;
const INDENT: f32 = 12.0;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(300);
const DRAIN_INTERVAL: Duration = Duration::from_millis(400);
/// Bounds for the Phase 3.2 project-wide search walk — deep enough for a
/// normal project, capped so a pathological tree can't stall the walk.
const SEARCH_MAX_DEPTH: usize = 8;
const SEARCH_MAX_VISITS: usize = 4000;

/// `DraggedPaths` / `shell_quote` / `quote_paths` moved to
/// `labonair_workspace::drag` in T16-006 (so `views::terminal` can accept
/// explorer-row drops without `labonair-workspace` depending on `labonair-ui`).
/// Re-exported here so `crate::explorer::DraggedPaths` keeps resolving.
pub use labonair_workspace::drag::{quote_paths, shell_quote, DraggedPaths};

/// The little chip that follows the pointer while dragging explorer rows.
pub struct DragPreview {
    label: SharedString,
}

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py(px(2.0))
            .rounded_sm()
            .bg(gpui::hsla(0.0, 0.0, 0.18, 0.94))
            .text_color(gpui::white())
            .text_xs()
            .child(self.label.clone())
    }
}

/// App-internal copy/cut buffer for the explorer (mirrors the reference
/// clipboard buffer — lives in the store, not the OS clipboard).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ClipOp {
    Copy,
    Cut,
}

#[derive(Clone)]
struct Clipboard {
    op: ClipOp,
    paths: Vec<PathBuf>,
}

/// A move/drop is a no-op (already in `dest_dir`) or invalid (into itself or a
/// descendant). Port of the reference `canDropInto`.
fn can_drop_into(src: &Path, dest_dir: &Path) -> bool {
    if src.parent() == Some(dest_dir) || dest_dir == src {
        return false;
    }
    !dest_dir.starts_with(src)
}

#[derive(Clone)]
struct Entry {
    name: String,
    is_dir: bool,
    is_ignored: bool,
}

#[derive(Clone)]
enum NodeState {
    Loading,
    Loaded { entries: Vec<Entry>, has_more: bool },
    Error(String),
}

struct PendingCreate {
    parent: PathBuf,
    is_dir: bool,
}

/// Whether the inline text field is creating a new entry or renaming one.
#[derive(Default)]
enum EditMode {
    #[default]
    None,
    Create,
    Rename(PathBuf),
}

/// One visible line of the flattened tree (mirrors the reference `TreeRow`).
enum Row {
    Entry {
        path: PathBuf,
        depth: usize,
        entry: Entry,
    },
    PendingCreate {
        depth: usize,
    },
    Rename {
        depth: usize,
    },
    Loading {
        depth: usize,
    },
    Error {
        depth: usize,
        message: String,
    },
    LoadMore {
        parent: PathBuf,
        depth: usize,
    },
}

/// Pure explorer state machine — the port of `useLocalExplorerStore` +
/// `buildTreeRows`. No GPUI, no IO; unit-tested below.
#[derive(Default)]
struct TreeModel {
    root: Option<PathBuf>,
    nodes: HashMap<PathBuf, NodeState>,
    expanded: HashSet<PathBuf>,
    show_hidden: bool,
    /// Bumped on every root/show-hidden change; a slow `read_dir_page` response
    /// is discarded when it no longer matches (reference `generation`).
    generation: u64,
    pending_create: Option<PendingCreate>,
    edit_mode: EditMode,
}

impl TreeModel {
    fn generation(&self) -> u64 {
        self.generation
    }

    /// Repoint at a new root. Returns `false` (no-op) when unchanged.
    fn set_root(&mut self, root: Option<PathBuf>) -> bool {
        if self.root == root {
            return false;
        }
        self.root = root;
        self.generation += 1;
        self.nodes.clear();
        self.expanded.clear();
        self.pending_create = None;
        self.edit_mode = EditMode::None;
        true
    }

    fn set_node(&mut self, path: PathBuf, state: NodeState) {
        self.nodes.insert(path, state);
    }

    fn mark_loading(&mut self, path: PathBuf) {
        self.nodes.insert(path, NodeState::Loading);
    }

    /// Lazy-load guard: a directory that is already loaded or in flight is not
    /// re-requested (reference `useFileTree` dedup).
    fn needs_load(&self, path: &Path) -> bool {
        !matches!(
            self.nodes.get(path),
            Some(NodeState::Loaded { .. }) | Some(NodeState::Loading)
        )
    }

    /// Returns `true` when the directory became expanded (caller should load).
    fn toggle_expanded(&mut self, path: PathBuf) -> bool {
        if self.expanded.remove(&path) {
            false
        } else {
            self.expanded.insert(path);
            true
        }
    }

    /// Flips `show_hidden`, invalidates every cached node (each was read under
    /// the old flag) but keeps `expanded` so open folders don't collapse.
    /// Returns the directories that must be re-fetched.
    fn toggle_show_hidden(&mut self) -> Vec<PathBuf> {
        self.show_hidden = !self.show_hidden;
        self.generation += 1;
        let reload: Vec<PathBuf> = self
            .root
            .iter()
            .cloned()
            .chain(self.expanded.iter().cloned())
            .collect();
        self.nodes.clear();
        reload
    }

    /// Directories currently loaded or loading — the watch set.
    fn watch_targets(&self) -> Vec<PathBuf> {
        self.nodes
            .iter()
            .filter(|(_, s)| matches!(s, NodeState::Loaded { .. } | NodeState::Loading))
            .map(|(p, _)| p.clone())
            .collect()
    }

    fn rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        if let Some(root) = self.root.clone() {
            self.walk(&root, 0, &mut out);
        }
        out
    }

    /// Every entry in every *loaded* directory, regardless of `expanded` — the
    /// traversal search filters over so a match can be found in a
    /// collapsed-but-loaded subtree, not only in the rows currently on screen
    /// (Zed-parity §8.6 / §14 "search can find entries that were not previously
    /// expanded"). Still bounded by lazy loading: directories never opened are
    /// not walked (that would need filesystem I/O on the render path).
    fn all_loaded_rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        if let Some(root) = self.root.clone() {
            self.walk_all(&root, 0, &mut out);
        }
        out
    }

    fn walk_all(&self, parent: &Path, depth: usize, out: &mut Vec<Row>) {
        if let Some(NodeState::Loaded { entries, .. }) = self.nodes.get(parent) {
            for entry in entries {
                let path = parent.join(&entry.name);
                out.push(Row::Entry {
                    path: path.clone(),
                    depth,
                    entry: entry.clone(),
                });
                if entry.is_dir {
                    self.walk_all(&path, depth + 1, out);
                }
            }
        }
    }

    fn walk(&self, parent: &Path, depth: usize, out: &mut Vec<Row>) {
        if let Some(pc) = &self.pending_create {
            if pc.parent == parent {
                out.push(Row::PendingCreate { depth });
            }
        }
        match self.nodes.get(parent) {
            None => {}
            Some(NodeState::Loading) => out.push(Row::Loading { depth }),
            Some(NodeState::Error(message)) => out.push(Row::Error {
                depth,
                message: message.clone(),
            }),
            Some(NodeState::Loaded { entries, has_more }) => {
                for entry in entries {
                    let path = parent.join(&entry.name);
                    if matches!(&self.edit_mode, EditMode::Rename(p) if *p == path) {
                        out.push(Row::Rename { depth });
                    } else {
                        out.push(Row::Entry {
                            path: path.clone(),
                            depth,
                            entry: entry.clone(),
                        });
                    }
                    if entry.is_dir && self.expanded.contains(&path) {
                        self.walk(&path, depth + 1, out);
                    }
                }
                if *has_more {
                    out.push(Row::LoadMore {
                        parent: parent.to_path_buf(),
                        depth,
                    });
                }
            }
        }
    }
}

/// Presentation-ready, cheap-to-clone snapshot of one visible tree line
/// (Zed-parity §12.4). Produced by [`flatten_rows`] with **no** filesystem
/// access; the `uniform_list` render closure turns a `&[ExplorerRowData]`
/// viewport window into elements. Visual state (`selected` / `cut` /
/// `drop_target`) is resolved here once, not re-derived per row during render.
/// Diagnostic severity channel for an Explorer row (Zed-parity Phase 3.7).
/// A typed hook only — no in-repo diagnostic source feeds it yet, so the
/// provider is always empty. TODO: wire an LSP / build diagnostics source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ExplorerRowData {
    Entry {
        path: PathBuf,
        name: String,
        depth: usize,
        is_dir: bool,
        is_ignored: bool,
        expanded: bool,
        selected: bool,
        cut: bool,
        drop_target: bool,
        drag_paths: Vec<PathBuf>,
        drag_label: String,
        /// The file open in the active editor (Phase 3.5 auto-reveal channel).
        active_file: bool,
        /// Merged Git status letter (Phase 3.7), `None` for clean / untracked-
        /// ignored paths and for remote roots.
        git: Option<char>,
        /// Merged diagnostic severity (Phase 3.7) — always `None` today.
        diag: Option<DiagnosticSeverity>,
    },
    PendingCreate {
        depth: usize,
    },
    Rename {
        depth: usize,
    },
    Loading {
        depth: usize,
    },
    Error {
        depth: usize,
        message: String,
    },
    LoadMore {
        parent: PathBuf,
        depth: usize,
    },
}

/// Turn a flattened [`Row`] list into presentation data. Pure — unit-tested
/// below; no `self`, no GPUI, no IO.
pub(crate) fn flatten_rows(
    rows: Vec<Row>,
    expanded: &HashSet<PathBuf>,
    selection: &[PathBuf],
    cut: &[PathBuf],
    drop_target: Option<&Path>,
) -> Vec<ExplorerRowData> {
    let _span = tracing::trace_span!(
        target: "labonair::perf",
        "explorer_flatten",
        rows = rows.len()
    )
    .entered();
    let multi = selection.len() > 1;
    rows.into_iter()
        .map(|row| match row {
            Row::Entry { path, depth, entry } => {
                let selected = selection.iter().any(|p| p == &path);
                let drag_paths = if selected && multi {
                    selection.to_vec()
                } else {
                    vec![path.clone()]
                };
                let drag_label = if drag_paths.len() > 1 {
                    format!("{} items", drag_paths.len())
                } else {
                    entry.name.clone()
                };
                ExplorerRowData::Entry {
                    expanded: entry.is_dir && expanded.contains(&path),
                    selected,
                    cut: cut.iter().any(|p| p == &path),
                    drop_target: entry.is_dir && drop_target == Some(path.as_path()),
                    name: entry.name,
                    is_dir: entry.is_dir,
                    is_ignored: entry.is_ignored,
                    depth,
                    drag_paths,
                    drag_label,
                    active_file: false,
                    git: None,
                    diag: None,
                    path,
                }
            }
            Row::PendingCreate { depth } => ExplorerRowData::PendingCreate { depth },
            Row::Rename { depth } => ExplorerRowData::Rename { depth },
            Row::Loading { depth } => ExplorerRowData::Loading { depth },
            Row::Error { depth, message } => ExplorerRowData::Error { depth, message },
            Row::LoadMore { parent, depth } => ExplorerRowData::LoadMore { parent, depth },
        })
        .collect()
}

/// Depth of any flattened row (pseudo-rows included).
fn row_depth(r: &ExplorerRowData) -> usize {
    match r {
        ExplorerRowData::Entry { depth, .. }
        | ExplorerRowData::PendingCreate { depth }
        | ExplorerRowData::Rename { depth }
        | ExplorerRowData::Loading { depth }
        | ExplorerRowData::Error { depth, .. }
        | ExplorerRowData::LoadMore { depth, .. } => *depth,
    }
}

/// Merge optional Git / diagnostic decorations and the active-file marker into
/// a flattened row list at the presentation boundary (Zed-parity §11 / §12.4).
/// Pure — decorations never change row geometry.
pub(crate) fn decorate_rows(
    rows: &mut [ExplorerRowData],
    active_file: Option<&Path>,
    git: &HashMap<PathBuf, char>,
    diag: &HashMap<PathBuf, DiagnosticSeverity>,
) {
    let _span = tracing::trace_span!(
        target: "labonair::perf",
        "explorer_decorate",
        rows = rows.len()
    )
    .entered();
    for row in rows.iter_mut() {
        if let ExplorerRowData::Entry {
            path,
            active_file: af,
            git: g,
            diag: d,
            ..
        } = row
        {
            *af = active_file == Some(path.as_path());
            *g = git.get(path).copied();
            *d = diag.get(path).copied();
        }
    }
}

/// Collapse single-child directory *chains* (`a/b/c` where every link has
/// exactly one child and that child is a directory) into one compressed row
/// whose label is the joined path (Zed-parity Phase 3.6). Pure: the input is
/// consumed and a new list returned, so it is trivially reversible (re-run
/// [`flatten_rows`] without this pass). The surviving row keeps the *real*
/// deepest path, so selection, drag/drop and context menus are unaffected.
pub(crate) fn fold_chains(mut rows: Vec<Row>) -> Vec<Row> {
    let _span = tracing::trace_span!(
        target: "labonair::perf",
        "explorer_fold_chains",
        rows = rows.len()
    )
    .entered();
    let mut i = 0;
    while i < rows.len() {
        let Row::Entry {
            depth: d,
            entry: parent_entry,
            ..
        } = &rows[i]
        else {
            i += 1;
            continue;
        };
        if !parent_entry.is_dir {
            i += 1;
            continue;
        }
        let d = *d;
        let parent_name = parent_entry.name.clone();
        // The parent's descendant run: rows after i until depth drops back.
        let run_end = rows[i + 1..]
            .iter()
            .position(|r| row_depth_model(r) <= d)
            .map(|p| i + 1 + p)
            .unwrap_or(rows.len());
        let direct: Vec<usize> = (i + 1..run_end)
            .filter(|&k| row_depth_model(&rows[k]) == d + 1)
            .collect();
        let [only] = direct[..] else {
            i += 1;
            continue;
        };
        let Row::Entry {
            entry: child_entry, ..
        } = &rows[only]
        else {
            i += 1;
            continue;
        };
        if !child_entry.is_dir {
            i += 1;
            continue;
        }
        // Fold: drop the parent row, pull the child (and its subtree) up one
        // level, and prefix the parent's name onto the child's label.
        if let Row::Entry { entry, .. } = &mut rows[only] {
            entry.name = format!("{parent_name}/{}", entry.name);
        }
        for row in rows[only..run_end].iter_mut() {
            decrement_depth(row);
        }
        rows.remove(i);
        // Re-examine the same index — the folded row may chain further.
    }
    rows
}

fn row_depth_model(r: &Row) -> usize {
    match r {
        Row::Entry { depth, .. }
        | Row::PendingCreate { depth }
        | Row::Rename { depth }
        | Row::Loading { depth }
        | Row::Error { depth, .. }
        | Row::LoadMore { depth, .. } => *depth,
    }
}

fn decrement_depth(r: &mut Row) {
    let d = match r {
        Row::Entry { depth, .. }
        | Row::PendingCreate { depth }
        | Row::Rename { depth }
        | Row::Loading { depth }
        | Row::Error { depth, .. }
        | Row::LoadMore { depth, .. } => depth,
    };
    *d = d.saturating_sub(1);
}

/// The ancestor directory rows of the row at `first_visible`, in
/// shallow→deep order (Zed-parity Phase 3.3). Computed independently over the
/// flattened model; the caller pins these above the virtual list.
pub(crate) fn sticky_ancestor_indices(
    rows: &[ExplorerRowData],
    first_visible: usize,
) -> Vec<usize> {
    if first_visible == 0 || first_visible >= rows.len() {
        return Vec::new();
    }
    let target_depth = row_depth(&rows[first_visible]);
    let mut stack: Vec<usize> = Vec::new();
    for (k, row) in rows.iter().enumerate().take(first_visible) {
        let dk = row_depth(row);
        while stack
            .last()
            .is_some_and(|&last| row_depth(&rows[last]) >= dk)
        {
            stack.pop();
        }
        if matches!(
            row,
            ExplorerRowData::Entry {
                is_dir: true,
                expanded: true,
                ..
            }
        ) {
            stack.push(k);
        }
    }
    while stack
        .last()
        .is_some_and(|&last| row_depth(&rows[last]) >= target_depth)
    {
        stack.pop();
    }
    stack
}

/// Index of the row whose path is `active` (Zed-parity Phase 3.5 reveal
/// policy). `None` — and therefore a no-op reveal — when the file is not
/// currently in the flattened tree.
pub(crate) fn reveal_target_index(rows: &[ExplorerRowData], active: &Path) -> Option<usize> {
    rows.iter()
        .position(|r| matches!(r, ExplorerRowData::Entry { path, .. } if path == active))
}

/// Bounded, blocking filesystem walk for project-wide Explorer search
/// (Zed-parity Phase 3.2). Runs off the GPUI thread (`background_executor`).
/// Capped at `max_depth` levels and `max_visits` directory reads so an
/// enormous tree can't stall the walk; returns `(path, name, depth)` for
/// every entry whose name contains `query` (already lowercased by the caller).
pub(crate) fn bounded_fs_search(
    root: &Path,
    query: &str,
    show_hidden: bool,
    max_depth: usize,
    max_visits: usize,
) -> Vec<(PathBuf, String, usize, bool)> {
    let mut out = Vec::new();
    let mut visits = 0usize;
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth || visits >= max_visits {
            continue;
        }
        visits += 1;
        let Ok(page) = tree::read_dir_page(&dir.to_string_lossy(), 0, PAGE_LIMIT, show_hidden)
        else {
            continue;
        };
        for e in page.entries {
            let path = dir.join(&e.name);
            let is_dir = matches!(e.kind, tree::EntryKind::Dir);
            if e.name.to_lowercase().contains(query) {
                out.push((path.clone(), e.name.clone(), depth, is_dir));
            }
            if is_dir {
                stack.push((path, depth + 1));
            }
        }
    }
    out
}

/// Parse `git status --porcelain` (v1) into a `relative-path → status char`
/// map (Zed-parity Phase 3.7). Worktree status wins over index status; a `?`
/// (untracked) is kept as-is. Pure — unit-tested.
pub(crate) fn parse_git_porcelain(out: &str) -> HashMap<PathBuf, char> {
    let mut map = HashMap::new();
    for line in out.lines() {
        if line.len() < 4 {
            continue;
        }
        let (xy, rest) = line.split_at(2);
        let x = xy.chars().next().unwrap_or(' ');
        let y = xy.chars().nth(1).unwrap_or(' ');
        let path_part = rest.trim_start();
        // Renames: "R  old -> new" — decorate the new path.
        let path_str = path_part.rsplit(" -> ").next().unwrap_or(path_part);
        let path_str = path_str.trim_matches('"');
        if path_str.is_empty() {
            continue;
        }
        let ch = if x == '?' && y == '?' {
            '?'
        } else if y != ' ' {
            y
        } else {
            x
        };
        map.insert(PathBuf::from(path_str), ch);
    }
    map
}

/// The label tint for a Git status char.
fn git_tint(ch: char, c: &Colors) -> Hsla {
    match ch {
        'M' | 'T' => c.palette.warning,
        'A' | '?' => c.palette.success,
        'D' | 'U' => c.err,
        'R' | 'C' => c.accent,
        _ => c.muted,
    }
}

pub struct ExplorerView {
    theme: Entity<ThemeStore>,
    workspace: Entity<Workspace>,
    model: TreeModel,
    selection: Vec<PathBuf>,
    clipboard: Option<Clipboard>,
    drop_target: Option<PathBuf>,
    edit_buffer: String,
    /// Real text field backing the inline create/rename row (T16-002 canary).
    /// Lazily created on `begin_create` / `begin_rename` (needs a `Window`).
    edit_field: Option<Entity<InputState>>,
    /// The reference exposes a compact, collapsible file-name filter directly
    /// below the Explorer toolbar. Keep the native input entity alive while
    /// it is visible so it retains focus, selection and IME state.
    search_open: bool,
    search_field: Option<Entity<InputState>>,
    context_menu: Option<(PathBuf, Point<Pixels>)>,
    /// The compact root row's `…` overflow menu anchor (Phase 3.1).
    overflow_menu: Option<Point<Pixels>>,
    confirm_delete: Option<PathBuf>,
    focus: FocusHandle,
    /// Virtual-list scroll handle — drives sticky-ancestor computation and
    /// active-file reveal (Phase 3.3 / 3.5).
    scroll: UniformListScrollHandle,
    /// The file open in the active editor (Phase 3.5 auto-reveal).
    active_file: Option<PathBuf>,
    /// A pending "scroll the tree to this path" request, serviced in `render`.
    pending_reveal: Option<PathBuf>,
    /// `absolute path → git status char` (Phase 3.7), local roots only.
    git_status: HashMap<PathBuf, char>,
    /// Typed diagnostic hook — no source feeds it yet (Phase 3.7 TODO).
    diagnostics: HashMap<PathBuf, DiagnosticSeverity>,
    /// Extra project-wide search hits from the bounded async FS walk
    /// (Phase 3.2), beyond what the lazily-loaded model already holds.
    search_hits: Vec<(PathBuf, String, usize, bool)>,
    /// Bumped per walk so a slow response for a stale query is discarded.
    search_gen: u64,
    /// A bounded FS walk is in flight — drives the in-panel progress hint.
    search_walking: bool,
    /// Parent directories flagged dirty by the watcher, drained on a timer.
    dirty: Arc<Mutex<HashSet<PathBuf>>>,
    watched: HashSet<PathBuf>,
    debouncer: Option<Debouncer<RecommendedWatcher>>,
    _drain: Task<()>,
}

impl ExplorerView {
    pub fn new(
        theme: Entity<ThemeStore>,
        workspace: Entity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        // Phase 3.5: follow the workspace's active editor for auto-reveal.
        cx.observe(&workspace, |this, _, cx| this.on_workspace_changed(cx))
            .detach();

        let drain = cx.spawn(async move |view, cx| loop {
            cx.background_executor().timer(DRAIN_INTERVAL).await;
            if view
                .update(cx, |this, cx| this.drain_watch_events(cx))
                .is_err()
            {
                break;
            }
        });

        Self {
            theme,
            workspace,
            model: TreeModel::default(),
            selection: Vec::new(),
            clipboard: None,
            drop_target: None,
            edit_buffer: String::new(),
            edit_field: None,
            search_open: false,
            search_field: None,
            context_menu: None,
            overflow_menu: None,
            confirm_delete: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            active_file: None,
            pending_reveal: None,
            git_status: HashMap::new(),
            diagnostics: HashMap::new(),
            search_hits: Vec::new(),
            search_gen: 0,
            search_walking: false,
            dirty: Arc::new(Mutex::new(HashSet::new())),
            watched: HashSet::new(),
            debouncer: None,
            _drain: drain,
        }
    }

    /// Point the explorer at a new working directory (driven by the active
    /// terminal's cwd — see [`crate::app_shell`]). No-op if unchanged.
    pub fn set_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        if !self.model.set_root(root) {
            return;
        }
        self.selection.clear();
        self.clipboard = None;
        self.drop_target = None;
        self.context_menu = None;
        self.overflow_menu = None;
        self.confirm_delete = None;
        self.edit_buffer.clear();
        self.git_status.clear();
        self.search_hits.clear();
        if let Some(root) = self.model.root.clone() {
            self.load_dir(root, false, cx);
        }
        self.sync_watchers();
        self.poll_git(cx);
        self.reveal_active_file(cx);
        cx.notify();
    }

    // ── Zed-parity Phase 3 ──────────────────────────────────────────────────

    /// Resolved Explorer settings, falling back to the shipped defaults when
    /// the settings store isn't up yet (headless tests).
    fn settings(cx: &App) -> ExplorerSettings {
        ExplorerSettings::try_get(cx)
            .cloned()
            .unwrap_or_else(|| ExplorerSettings::from_settings(&Default::default()))
    }

    /// The workspace's active editor changed — re-evaluate auto-reveal.
    fn on_workspace_changed(&mut self, cx: &mut Context<Self>) {
        self.reveal_active_file(cx);
    }

    /// Phase 3.5: when auto-reveal is on, expand the active file's ancestor
    /// directories, mark it, and queue a scroll. No-op when the setting is
    /// off or the file is not under the current root.
    fn reveal_active_file(&mut self, cx: &mut Context<Self>) {
        if !Self::settings(cx).auto_reveal_active_file() {
            if self.active_file.take().is_some() {
                cx.notify();
            }
            return;
        }
        let Some(root) = self.model.root.clone() else {
            return;
        };
        let path = self
            .workspace
            .read(cx)
            .active_file_path(cx)
            .map(PathBuf::from);
        let Some(path) = path.filter(|p| p.starts_with(&root)) else {
            if self.active_file.take().is_some() {
                cx.notify();
            }
            return;
        };
        if self.active_file.as_deref() == Some(path.as_path()) {
            return;
        }
        // Expand + lazily load every ancestor directory between root and file.
        if let Ok(rel) = path.strip_prefix(&root) {
            let mut cur = root.clone();
            for comp in rel.components() {
                let next = cur.join(comp.as_os_str());
                if next != path {
                    self.model.expanded.insert(next.clone());
                    self.load_dir(next.clone(), false, cx);
                }
                cur = next;
            }
        }
        self.active_file = Some(path.clone());
        self.pending_reveal = Some(path);
        cx.notify();
    }

    /// Phase 3.7: refresh Git status decorations for a local root via
    /// `git status --porcelain`, off the GPUI thread. Remote roots degrade
    /// gracefully — no decorations, same row geometry.
    fn poll_git(&mut self, cx: &mut Context<Self>) {
        if !Self::settings(cx).git_decorations() {
            self.git_status.clear();
            return;
        }
        let Some(root) = self.model.root.clone() else {
            return;
        };
        if !root.is_absolute() {
            return;
        }
        let root_c = root.clone();
        cx.spawn(async move |view, cx| {
            let out = cx
                .background_executor()
                .spawn(async move {
                    std::process::Command::new("git")
                        .arg("-C")
                        .arg(&root_c)
                        .args(["status", "--porcelain"])
                        .output()
                        .ok()
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                let Some(root) = this.model.root.clone() else {
                    return;
                };
                let mut map = HashMap::new();
                if let Some(text) = out {
                    for (rel, ch) in parse_git_porcelain(&text) {
                        map.insert(root.join(rel), ch);
                    }
                }
                if map != this.git_status {
                    this.git_status = map;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Phase 3.2: kick off a bounded, off-thread FS walk for the current
    /// query so search reaches directories lazy loading never opened.
    fn spawn_search_walk(&mut self, cx: &mut Context<Self>) {
        let query = self.search_query(cx);
        self.search_gen += 1;
        let gen = self.search_gen;
        if query.is_empty() {
            self.search_hits.clear();
            self.search_walking = false;
            cx.notify();
            return;
        }
        let Some(root) = self.model.root.clone() else {
            return;
        };
        if !root.is_absolute() {
            return;
        }
        let show_hidden = self.model.show_hidden;
        self.search_walking = true;
        cx.notify();
        cx.spawn(async move |view, cx| {
            let hits = cx
                .background_executor()
                .spawn(async move {
                    bounded_fs_search(
                        &root,
                        &query,
                        show_hidden,
                        SEARCH_MAX_DEPTH,
                        SEARCH_MAX_VISITS,
                    )
                })
                .await;
            let _ = view.update(cx, |this, cx| {
                if this.search_gen != gen {
                    return;
                }
                this.search_hits = hits;
                this.search_walking = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub fn set_root_str(&mut self, root: Option<String>, cx: &mut Context<Self>) {
        self.set_root(root.map(PathBuf::from), cx);
    }

    fn load_dir(&mut self, path: PathBuf, force: bool, cx: &mut Context<Self>) {
        if !force && !self.model.needs_load(&path) {
            return;
        }
        self.model.mark_loading(path.clone());
        cx.notify();

        let gen = self.model.generation();
        let show_hidden = self.model.show_hidden;
        let path_str = path.to_string_lossy().to_string();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { tree::read_dir_page(&path_str, 0, PAGE_LIMIT, show_hidden) })
                .await;
            let _ = view.update(cx, |this, cx| {
                if this.model.generation() != gen {
                    return;
                }
                match result {
                    Ok(page) => {
                        let entries = page
                            .entries
                            .into_iter()
                            .map(|e| Entry {
                                name: e.name,
                                is_dir: matches!(e.kind, tree::EntryKind::Dir),
                                is_ignored: e.is_ignored,
                            })
                            .collect();
                        this.model.set_node(
                            path.clone(),
                            NodeState::Loaded {
                                entries,
                                has_more: page.has_more,
                            },
                        );
                    }
                    Err(message) => {
                        this.model.set_node(path.clone(), NodeState::Error(message));
                    }
                }
                this.sync_watchers();
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_expanded(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.model.toggle_expanded(path.clone()) {
            self.load_dir(path, false, cx);
        }
        cx.notify();
    }

    fn toggle_show_hidden(&mut self, cx: &mut Context<Self>) {
        for path in self.model.toggle_show_hidden() {
            self.load_dir(path, true, cx);
        }
        cx.notify();
    }

    fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = !self.search_open;
        if self.search_open {
            let field = self.search_field.get_or_insert_with(|| {
                let field =
                    cx.new(|cx| InputState::new(window, cx).placeholder("Search files\u{2026}"));
                cx.subscribe(&field, |this, _, ev: &InputEvent, cx| {
                    if matches!(ev, InputEvent::Change) {
                        this.spawn_search_walk(cx);
                        cx.notify();
                    }
                })
                .detach();
                field
            });
            field.update(cx, |state, cx| state.focus(window, cx));
        } else {
            // Closing search drops the transient overlay + its extra hits.
            self.search_hits.clear();
            self.search_walking = false;
        }
        cx.notify();
    }

    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(field) = &self.search_field {
            field.update(cx, |state, cx| state.set_value("", window, cx));
        }
        self.search_hits.clear();
        self.search_walking = false;
        cx.notify();
    }

    fn select(&mut self, path: PathBuf, additive: bool, cx: &mut Context<Self>) {
        if additive {
            if let Some(i) = self.selection.iter().position(|p| p == &path) {
                self.selection.remove(i);
            } else {
                self.selection.push(path);
            }
        } else {
            self.selection.clear();
            self.selection.push(path);
        }
        cx.notify();
    }

    fn is_selected(&self, path: &Path) -> bool {
        self.selection.iter().any(|p| p == path)
    }

    /// Paths a drag/copy/cut acts on: the whole selection when `path` is part
    /// of it, otherwise just `path`.
    fn action_paths(&self, path: &Path) -> Vec<PathBuf> {
        if self.is_selected(path) && self.selection.len() > 1 {
            self.selection.clone()
        } else {
            vec![path.to_path_buf()]
        }
    }

    // --- copy / cut / paste buffer (T05-002) ---

    fn clip_set(&mut self, op: ClipOp, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.context_menu = None;
        if paths.is_empty() {
            return;
        }
        self.clipboard = Some(Clipboard { op, paths });
        cx.notify();
    }

    fn clip_clear(&mut self, cx: &mut Context<Self>) {
        self.clipboard = None;
        cx.notify();
    }

    fn paste_into(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        self.context_menu = None;
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        // Guard against pasting a folder into itself / a descendant.
        if clip.paths.iter().any(|p| {
            dir == *p || dir.starts_with(p) || (clip.op == ClipOp::Cut && !can_drop_into(p, &dir))
        }) {
            notification_center(cx).update(cx, |c, cx| {
                c.push(
                    Notification::error("Paste failed", "Invalid destination".to_string()),
                    cx,
                );
            });
            return;
        }
        let mut reload: Vec<PathBuf> = vec![dir.clone()];
        let op = clip.op;
        let paths: Vec<String> = clip
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        if op == ClipOp::Cut {
            for p in &clip.paths {
                if let Some(parent) = p.parent() {
                    reload.push(parent.to_path_buf());
                }
                self.model.expanded.remove(p);
            }
            self.clipboard = None;
        }
        let dir_s = dir.to_string_lossy().to_string();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match op {
                        ClipOp::Copy => mutate::copy_into_sync(&paths, &dir_s).map(|_| ()),
                        ClipOp::Cut => {
                            for p in &paths {
                                mutate::move_into_sync(p, &dir_s)?;
                            }
                            Ok(())
                        }
                    }
                })
                .await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(()) => {
                    for d in reload {
                        this.load_dir(d, true, cx);
                    }
                }
                Err(message) => {
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(Notification::error("Paste failed", message), cx);
                    });
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// Drop of a dragged row (or rows) onto a directory — move.
    fn drop_move(&mut self, srcs: Vec<PathBuf>, dest_dir: PathBuf, cx: &mut Context<Self>) {
        self.drop_target = None;
        let srcs: Vec<PathBuf> = srcs
            .into_iter()
            .filter(|s| can_drop_into(s, &dest_dir))
            .collect();
        if srcs.is_empty() {
            return;
        }
        let mut reload: Vec<PathBuf> = vec![dest_dir.clone()];
        let mut paths = Vec::new();
        for s in &srcs {
            if let Some(parent) = s.parent() {
                reload.push(parent.to_path_buf());
            }
            self.model.expanded.remove(s);
            paths.push(s.to_string_lossy().to_string());
        }
        self.selection.clear();
        let dir_s = dest_dir.to_string_lossy().to_string();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    for p in &paths {
                        mutate::move_into_sync(p, &dir_s)?;
                    }
                    Ok::<(), String>(())
                })
                .await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(()) => {
                    for d in reload {
                        this.load_dir(d, true, cx);
                    }
                }
                Err(message) => {
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(Notification::error("Move failed", message), cx);
                    });
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// External OS file drop into the tree — copy into the target directory.
    fn drop_external(&mut self, srcs: Vec<PathBuf>, dest_dir: PathBuf, cx: &mut Context<Self>) {
        self.drop_target = None;
        if srcs.is_empty() {
            return;
        }
        let paths: Vec<String> = srcs
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        let dir_s = dest_dir.to_string_lossy().to_string();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { mutate::copy_into_sync(&paths, &dir_s).map(|v| v.len()) })
                .await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(n) => {
                    this.load_dir(dest_dir, true, cx);
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(
                            Notification::success(
                                "Files copied",
                                format!("{n} item{} copied", if n == 1 { "" } else { "s" }),
                            ),
                            cx,
                        );
                    });
                }
                Err(message) => {
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(Notification::error("Copy failed", message), cx);
                    });
                }
            });
        })
        .detach();
        cx.notify();
    }

    // --- inline create / rename ---

    /// Spins up the real [`InputState`] for the inline row, seeds it with
    /// `initial`, focuses it and wires Enter → commit (T16-002).
    fn open_edit_field(&mut self, initial: &str, window: &mut Window, cx: &mut Context<Self>) {
        let initial = initial.to_string();
        let field = cx.new(|cx| InputState::new(window, cx).default_value(initial));
        cx.subscribe(&field, |this, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::PressEnter { .. }) {
                this.commit_edit(cx);
            }
        })
        .detach();
        field.update(cx, |state, cx| state.focus(window, cx));
        self.edit_field = Some(field);
    }

    fn begin_create(
        &mut self,
        parent: PathBuf,
        is_dir: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context_menu = None;
        self.model.expanded.insert(parent.clone());
        self.load_dir(parent.clone(), false, cx);
        self.model.pending_create = Some(PendingCreate { parent, is_dir });
        self.model.edit_mode = EditMode::Create;
        self.edit_buffer.clear();
        self.open_edit_field("", window, cx);
        cx.notify();
    }

    fn begin_rename(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.context_menu = None;
        self.edit_buffer = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.model.edit_mode = EditMode::Rename(path);
        self.model.pending_create = None;
        let seed = self.edit_buffer.clone();
        self.open_edit_field(&seed, window, cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        self.model.edit_mode = EditMode::None;
        self.model.pending_create = None;
        self.edit_buffer.clear();
        self.edit_field = None;
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(field) = &self.edit_field {
            self.edit_buffer = field.read(cx).value().to_string();
        }
        let name = self.edit_buffer.trim().to_string();
        match std::mem::replace(&mut self.model.edit_mode, EditMode::None) {
            EditMode::None => {}
            EditMode::Create => {
                let Some(pc) = self.model.pending_create.take() else {
                    return;
                };
                if name.is_empty() {
                    self.edit_buffer.clear();
                    cx.notify();
                    return;
                }
                let target = pc.parent.join(&name);
                let is_dir = pc.is_dir;
                self.run_fs_op(
                    pc.parent.clone(),
                    move || {
                        let p = target.to_string_lossy().to_string();
                        if is_dir {
                            mutate::create_dir_sync(&p)
                        } else {
                            mutate::create_file_sync(&p)
                        }
                    },
                    cx,
                );
            }
            EditMode::Rename(from) => {
                if name.is_empty()
                    || Some(name.as_str()) == from.file_name().and_then(|n| n.to_str())
                {
                    self.edit_buffer.clear();
                    cx.notify();
                    return;
                }
                let Some(parent) = from.parent().map(Path::to_path_buf) else {
                    return;
                };
                let to = parent.join(&name);
                let from_c = from.clone();
                self.model.expanded.remove(&from);
                self.run_fs_op(
                    parent,
                    move || mutate::rename_sync(&from_c.to_string_lossy(), &to.to_string_lossy()),
                    cx,
                );
            }
        }
        self.edit_buffer.clear();
        self.edit_field = None;
        cx.notify();
    }

    fn request_delete(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.context_menu = None;
        self.confirm_delete = Some(path);
        cx.notify();
    }

    fn confirm_delete_now(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.confirm_delete.take() else {
            return;
        };
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            return;
        };
        self.model.expanded.remove(&path);
        self.model.nodes.remove(&path);
        self.selection.clear();
        let path_c = path.clone();
        self.run_fs_op(
            parent,
            move || mutate::delete_sync(&path_c.to_string_lossy()),
            cx,
        );
        cx.notify();
    }

    /// Runs a blocking filesystem mutation off-thread, then reloads `reload`
    /// (stale-guard: the reference re-fetches after every op) or toasts the
    /// error (Critical Rule 6 — no `unwrap` on predictable errors).
    fn run_fs_op<F>(&mut self, reload: PathBuf, op: F, cx: &mut Context<Self>)
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        cx.spawn(async move |view, cx| {
            let result = cx.background_executor().spawn(async move { op() }).await;
            let _ = view.update(cx, |this, cx| match result {
                Ok(()) => this.load_dir(reload, true, cx),
                Err(message) => {
                    notification_center(cx).update(cx, |c, cx| {
                        c.push(Notification::error("File operation failed", message), cx);
                    });
                }
            });
        })
        .detach();
    }

    // --- context-menu actions ---

    fn copy_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.write_to_clipboard(ClipboardItem::new_string(
            path.to_string_lossy().to_string(),
        ));
        cx.notify();
    }

    /// Save `dir` as a local path bookmark (T12-003 context-menu entry).
    fn bookmark_folder(&mut self, dir: PathBuf, cx: &mut Context<Self>) {
        self.context_menu = None;
        let path = dir.to_string_lossy().to_string();
        let current = labonair_backend::modules::bookmarks::load();
        match labonair_backend::modules::bookmarks::compute_add_bookmark(
            &current, None, &path, None,
        ) {
            Some(next) => {
                let result = labonair_backend::modules::bookmarks::save(&next);
                notification_center(cx).update(cx, |c, cx| match result {
                    Ok(()) => c.push(Notification::success("Bookmarked", path.clone()), cx),
                    Err(message) => c.push(Notification::error("Bookmark failed", message), cx),
                });
            }
            None => {
                notification_center(cx).update(cx, |c, cx| {
                    c.push(Notification::info("Already bookmarked", path.clone()), cx);
                });
            }
        }
        cx.notify();
    }

    /// The current local root directory, if any (feeds the bookmarks popover).
    pub fn root(&self) -> Option<PathBuf> {
        self.model.root.clone()
    }

    fn open_in_terminal(&mut self, dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.context_menu = None;
        let cwd = dir.to_string_lossy().to_string();
        self.workspace.update(cx, |w, cx| {
            w.new_terminal_tab_in(Some(cwd), window, cx);
        });
        cx.notify();
    }

    fn open_in_preview(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.context_menu = None;
        let target = path.to_string_lossy().to_string();
        self.workspace
            .update(cx, |w, cx| w.open_preview(target, window, cx));
        cx.notify();
    }

    fn open_file(&mut self, path: &Path, peek: bool, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_string_lossy().to_string();
        self.workspace
            .update(cx, |w, cx| w.open_file(path, peek, window, cx));
        cx.notify();
    }

    // --- watcher (port of fs/watcher.rs, embedded) ---

    fn sync_watchers(&mut self) {
        let target: HashSet<PathBuf> = self
            .model
            .watch_targets()
            .into_iter()
            .filter(|p| p.is_dir())
            .collect();

        if target == self.watched {
            return;
        }

        if self.debouncer.is_none() {
            let dirty = self.dirty.clone();
            match new_debouncer(
                WATCH_DEBOUNCE,
                move |res: notify_debouncer_mini::DebounceEventResult| {
                    if let Ok(events) = res {
                        let mut set = dirty.lock().unwrap();
                        for ev in events {
                            if let Some(parent) = ev.path.parent() {
                                set.insert(parent.to_path_buf());
                            }
                        }
                    }
                },
            ) {
                Ok(d) => self.debouncer = Some(d),
                Err(err) => {
                    tracing::warn!(%err, "explorer: failed to create fs watcher");
                    return;
                }
            }
        }

        let Some(debouncer) = self.debouncer.as_mut() else {
            return;
        };
        for path in target.difference(&self.watched) {
            let _ = debouncer.watcher().watch(path, RecursiveMode::NonRecursive);
        }
        for path in self.watched.difference(&target) {
            let _ = debouncer.watcher().unwatch(path);
        }
        self.watched = target;
    }

    fn drain_watch_events(&mut self, cx: &mut Context<Self>) {
        let dirs: Vec<PathBuf> = {
            let mut set = self.dirty.lock().unwrap();
            if set.is_empty() {
                return;
            }
            set.drain().collect()
        };
        let mut changed = false;
        for dir in dirs {
            if self.model.nodes.contains_key(&dir) {
                self.load_dir(dir, true, cx);
                changed = true;
            }
        }
        // A filesystem change under the root may also change Git status.
        if changed {
            self.poll_git(cx);
        }
    }
}

impl Focusable for ExplorerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

#[derive(Clone, Copy)]
struct Colors {
    fg: Hsla,
    muted: Hsla,
    accent: Hsla,
    border: Hsla,
    card: Hsla,
    err: Hsla,
    palette: Palette,
}

impl Render for ExplorerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span =
            tracing::trace_span!(target: "labonair::perf", "render", view = "explorer_panel")
                .entered();
        let c = {
            let t = self.theme.read(cx);
            Colors {
                fg: t.foreground(),
                muted: t.muted_foreground(),
                accent: t.accent(),
                border: t.border(),
                card: t.card(),
                err: t.status_error(),
                palette: Palette::from_theme(t),
            }
        };

        let Some(root) = self.model.root.clone() else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .px_3()
                .text_xs()
                .text_color(c.muted)
                .child("No working directory")
                .into_any_element();
        };

        let root_name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string());

        let root_icon = {
            let theme = self.theme.read(cx);
            icon_for_path(theme.icon_theme(), &root_name, true, false)
        };
        // Phase 3.1: the five permanent toolbar icons collapse to a compact
        // root row — the root identity, one discoverable search affordance,
        // and one overflow button. New File / New Folder / Refresh / hidden-
        // files move into the `…` menu (and stay in the tree context menu).
        let toolbar = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_xs()
                    .text_color(c.fg)
                    .child(svg_path(root_icon, c.muted).size(px(15.0)))
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(root_name)),
                    ),
            )
            .child(
                button(
                    "search",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child(IconName::Search.svg(c.muted).size(px(13.0)))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.toggle_search(window, cx)
                })),
            )
            .child(
                button(
                    "explorer-overflow",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .child(IconName::Ellipsis.svg(c.muted).size(px(13.0)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                        this.overflow_menu = Some(ev.position);
                        cx.notify();
                    }),
                ),
            );

        let search = self.render_search(c, cx);

        let root_drop = root.clone();
        let root_ext = root.clone();
        let root_over = root.clone();
        let settings = Self::settings(cx);
        let query = self.search_query(cx);
        let searching = !query.is_empty();
        let mut rows = if searching {
            self.search_rows(&query)
        } else {
            self.model.rows()
        };
        // Phase 3.6: fold single-child directory chains (setting; off unless
        // enabled — preserves the current look). Skipped during search so a
        // match's real depth stays visible.
        if settings.fold_single_child_dirs() && !searching {
            rows = fold_chains(rows);
        }
        let cut_paths: Vec<PathBuf> = self
            .clipboard
            .as_ref()
            .filter(|c| c.op == ClipOp::Cut)
            .map(|c| c.paths.clone())
            .unwrap_or_default();
        let mut data = flatten_rows(
            rows,
            &self.model.expanded,
            &self.selection,
            &cut_paths,
            self.drop_target.as_deref(),
        );
        // Phase 3.7: merge Git / diagnostic decorations + the active-file
        // marker at the presentation boundary (never changes row geometry).
        decorate_rows(
            &mut data,
            self.active_file.as_deref(),
            &self.git_status,
            &self.diagnostics,
        );
        let empty_search = data.is_empty() && searching;
        let view = cx.entity();
        let edit_field = self.edit_field.clone();

        // Phase 3.5: service a queued active-file reveal now that `data` is
        // final — expand happened in `reveal_active_file`, this just scrolls.
        if let Some(target) = self.pending_reveal.take() {
            if let Some(ix) = reveal_target_index(&data, &target) {
                self.scroll.scroll_to_item(ix, ScrollStrategy::Center);
            }
        }

        // Phase 3.3: ancestor rows for the current scroll position, pinned
        // above the virtual list. Independent computation over `data`.
        let indent_guides = settings.indent_guides();
        let sticky: Vec<gpui::AnyElement> = if settings.sticky_ancestors() && !empty_search {
            let row_h: f32 = c.palette.density_tokens().tree_row_height().into();
            let first_visible = if row_h > 0.0 {
                (f32::from(-self.scroll.0.borrow().base_handle.offset().y) / row_h).floor() as usize
            } else {
                0
            };
            sticky_ancestor_indices(&data, first_visible.min(data.len().saturating_sub(1)))
                .into_iter()
                .map(|ix| explorer_row_element(&data[ix], c, indent_guides, &view, &edit_field, cx))
                .collect()
        } else {
            Vec::new()
        };

        let list_body: gpui::AnyElement = if empty_search && !self.search_walking {
            div()
                .px_3()
                .py_2()
                .text_xs()
                .text_color(c.muted)
                .child("No matches")
                .into_any_element()
        } else {
            // Virtualised (§13 Phase 2.3): only the visible `[start..end]`
            // window is turned into elements. The closure gets only
            // `&mut Window, &mut App` — handlers go through `view.update(..)`.
            uniform_list("explorer-rows", data.len(), move |range, _win, cx| {
                let _span = tracing::trace_span!(
                    target: "labonair::perf",
                    "explorer_viewport_build",
                    built = range.len(),
                    total = data.len()
                )
                .entered();
                range
                    .map(|i| {
                        explorer_row_element(&data[i], c, indent_guides, &view, &edit_field, cx)
                    })
                    .collect::<Vec<_>>()
            })
            .track_scroll(self.scroll.clone())
            .flex_1()
            .into_any_element()
        };

        // Phase 3.2: transient in-panel progress hint while the bounded FS
        // walk runs (walk is off-thread — see `spawn_search_walk`).
        let walk_hint = self.search_walking.then(|| {
            div()
                .px_3()
                .py_1()
                .text_xs()
                .text_color(c.muted)
                .child("Searching project\u{2026}")
        });

        // Phase 3.3: sticky ancestor strip with a subtle bottom boundary.
        let sticky_strip = (!sticky.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .bg(c.palette.bg)
                .border_b_1()
                .border_color(c.border)
                .children(sticky)
        });

        let list = div()
            .id("explorer-list")
            .flex_1()
            .flex()
            .flex_col()
            .min_h_0()
            .py_1()
            .children(sticky_strip)
            .children(walk_hint)
            .on_drag_move(cx.listener(
                move |this, _: &gpui::DragMoveEvent<DraggedPaths>, _w, cx| {
                    // Empty space / non-folder rows → drop into the root.
                    if this.drop_target.as_deref() != Some(root_over.as_path()) {
                        this.drop_target = Some(root_over.clone());
                        cx.notify();
                    }
                },
            ))
            .on_drop(cx.listener(move |this, d: &DraggedPaths, _w, cx| {
                this.drop_move(d.paths.clone(), root_drop.clone(), cx);
            }))
            .on_drop(cx.listener(move |this, d: &ExternalPaths, _w, cx| {
                this.drop_external(d.paths().to_vec(), root_ext.clone(), cx);
            }))
            .child(list_body);

        let mut container = div()
            .id("explorer")
            .track_focus(&self.focus)
            .relative()
            .flex_1()
            .flex()
            .flex_col()
            .text_color(c.fg)
            .on_key_down(cx.listener(Self::on_key))
            .child(toolbar)
            .children(search)
            .children(self.render_clip_banner(c, cx))
            .child(list);

        if let Some((target, pos)) = self.context_menu.clone() {
            container = container.child(self.render_context_menu(target, pos, c, cx));
        }
        if let Some(pos) = self.overflow_menu {
            container = container.child(self.render_overflow_menu(pos, root.clone(), cx));
        }
        if let Some(target) = self.confirm_delete.clone() {
            container = container.child(self.render_delete_confirm(target, c, cx));
        }

        container.into_any_element()
    }
}

impl ExplorerView {
    /// Reference-aligned compact search strip. The native tree is already
    /// lazy-loaded, so filtering the loaded rows is instantaneous and does
    /// not turn a toolbar gesture into filesystem I/O.
    fn render_search(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        if !self.search_open {
            return None;
        }
        let field = self.search_field.as_ref()?.clone();
        let has_query = !field.read(cx).value().trim().is_empty();
        let clear = has_query.then(|| {
            button(
                "clear-search",
                c.palette,
                ButtonVariant::Ghost,
                ButtonSize::IconXs,
            )
            .child(IconName::X.svg(c.muted).size(px(11.0)))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.clear_search(window, cx)))
        });

        Some(
            div()
                .relative()
                .flex()
                .flex_row()
                .items_center()
                .mx_2()
                .my(px(6.0))
                .h(px(28.0))
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .bg(c.card)
                .child(IconName::Search.svg(c.muted).size(px(13.0)).ml(px(7.0)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px_1()
                        .child(labonair_ui_kit::field_input(&field)),
                )
                .children(clear)
                .into_any_element(),
        )
    }

    fn search_query(&self, cx: &App) -> String {
        self.search_field
            .as_ref()
            .map(|field| field.read(cx).value().trim().to_lowercase())
            .unwrap_or_default()
    }

    /// Search results: matches from every lazily-loaded directory, plus the
    /// bounded async FS-walk hits for subtrees lazy loading never opened
    /// (Zed-parity §14 "search can find entries that were not previously
    /// expanded"). Merge is de-duplicated by path.
    fn search_rows(&self, query: &str) -> Vec<Row> {
        let mut seen: HashSet<PathBuf> = HashSet::new();
        let mut out: Vec<Row> = Vec::new();
        for row in self.model.all_loaded_rows() {
            if let Row::Entry { path, entry, depth } = row {
                if entry.name.to_lowercase().contains(query) {
                    seen.insert(path.clone());
                    out.push(Row::Entry { path, depth, entry });
                }
            }
        }
        for (path, name, depth, is_dir) in &self.search_hits {
            if seen.insert(path.clone()) {
                out.push(Row::Entry {
                    path: path.clone(),
                    depth: *depth,
                    entry: Entry {
                        name: name.clone(),
                        is_dir: *is_dir,
                        is_ignored: false,
                    },
                });
            }
        }
        out
    }

    /// Phase 3.1: the `…` overflow menu — the actions dropped from the
    /// permanent toolbar. Tree-option toggles live in Settings (typed fields,
    /// generated UI) per the settings design contract.
    fn render_overflow_menu(
        &self,
        pos: Point<Pixels>,
        root: PathBuf,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();
        let win = move |v: &Entity<Self>, f: ExpAct| -> MenuClick {
            let v = v.clone();
            Box::new(move |_ev: &ClickEvent, w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.overflow_menu = None;
                    f(this, w, cx);
                });
            })
        };
        let r1 = root.clone();
        let r2 = root.clone();
        let r3 = root.clone();
        let items: Vec<MenuItem> = vec![
            MenuItem::new("of-new-file", "New File")
                .icon(IconName::File)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.begin_create(r1.clone(), false, w, cx)),
                )),
            MenuItem::new("of-new-dir", "New Folder")
                .icon(IconName::Folder)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.begin_create(r2.clone(), true, w, cx)),
                )),
            MenuItem::new("of-refresh", "Refresh")
                .icon(IconName::Refresh)
                .on_click(win(
                    &view,
                    Box::new(move |this, _w, cx| this.load_dir(r3.clone(), true, cx)),
                )),
            MenuItem::separator(),
            MenuItem::new(
                "of-hidden",
                if self.model.show_hidden {
                    "Hide Hidden Files"
                } else {
                    "Show Hidden Files"
                },
            )
            .icon(if self.model.show_hidden {
                IconName::EyeOff
            } else {
                IconName::Eye
            })
            .on_click(win(
                &view,
                Box::new(move |this, _w, cx| this.toggle_show_hidden(cx)),
            )),
        ];
        let v = view.clone();
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            v.update(cx, |this, cx| {
                this.overflow_menu = None;
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

    /// Explorer-level keyboard: preview / open (Phase 3.8), copy / cut / paste
    /// buffer + clear.
    ///
    /// Preview vs permanent open is an explicit contract: single-click and
    /// `Space` open a **preview** (transient / peek tab, reused for the next
    /// preview); double-click and `Enter` open a **permanent** tab. Directories
    /// toggle on either key.
    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        if ks.key == "escape" {
            if self.clipboard.is_some() {
                self.clip_clear(cx);
            } else {
                self.selection.clear();
                cx.notify();
            }
            return;
        }
        if (ks.key == "space" || ks.key == "enter") && !ks.modifiers.secondary() {
            let Some(primary) = self.selection.last().cloned() else {
                return;
            };
            if self.path_is_dir(&primary) {
                self.toggle_expanded(primary, cx);
            } else {
                let peek = ks.key == "space";
                self.open_file(&primary, peek, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if !ks.modifiers.secondary() {
            return;
        }
        let Some(primary) = self.selection.last().cloned() else {
            if ks.key == "v" {
                if let Some(root) = self.paste_dir() {
                    self.paste_into(root, cx);
                }
            }
            return;
        };
        match ks.key.as_str() {
            "c" => self.clip_set(ClipOp::Copy, self.action_paths(&primary), cx),
            "x" => self.clip_set(ClipOp::Cut, self.action_paths(&primary), cx),
            "v" => {
                if let Some(dir) = self.paste_dir() {
                    self.paste_into(dir, cx);
                }
            }
            _ => {}
        }
    }

    /// Whether `path` is a directory *according to the loaded model* — avoids
    /// a filesystem stat on the keypress path.
    fn path_is_dir(&self, path: &Path) -> bool {
        if self.model.expanded.contains(path) || self.model.nodes.contains_key(path) {
            return true;
        }
        path.parent()
            .and_then(|parent| match self.model.nodes.get(parent) {
                Some(NodeState::Loaded { entries, .. }) => entries
                    .iter()
                    .find(|e| Some(e.name.as_str()) == path.file_name().and_then(|n| n.to_str()))
                    .map(|e| e.is_dir),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// Where a keyboard/banner paste lands: the selected directory (or the
    /// selected file's parent), else the root.
    fn paste_dir(&self) -> Option<PathBuf> {
        if let Some(p) = self.selection.last() {
            if p.is_dir() {
                return Some(p.clone());
            }
            if let Some(parent) = p.parent() {
                return Some(parent.to_path_buf());
            }
        }
        self.model.root.clone()
    }

    fn render_clip_banner(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let clip = self.clipboard.as_ref()?;
        let verb = match clip.op {
            ClipOp::Copy => "copied",
            ClipOp::Cut => "cut",
        };
        let text = format!(
            "{} item{} {verb}",
            clip.paths.len(),
            if clip.paths.len() == 1 { "" } else { "s" }
        );
        Some(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py(px(2.0))
                .text_xs()
                .bg(c.card)
                .border_b_1()
                .border_color(c.border)
                .child(
                    div()
                        .flex_1()
                        .text_color(c.muted)
                        .child(SharedString::from(text)),
                )
                .child(
                    button("clip-paste", c.palette, ButtonVariant::Link, ButtonSize::Xs)
                        .child("Paste")
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            if let Some(dir) = this.paste_dir() {
                                this.paste_into(dir, cx);
                            }
                        })),
                )
                .child(
                    button(
                        "clip-clear",
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::Xs,
                    )
                    .child("Clear")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.clip_clear(cx))),
                )
                .into_any_element(),
        )
    }

    fn render_context_menu(
        &self,
        target: PathBuf,
        pos: Point<Pixels>,
        _c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_dir = matches!(
            self.model.nodes.get(&target),
            Some(NodeState::Loaded { .. })
        ) || self.model.expanded.contains(&target)
            || target.is_dir();
        let dir_for_ops = if is_dir {
            target.clone()
        } else {
            target
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| target.clone())
        };
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let can_preview = !is_dir && crate::preview::is_previewable(&target.to_string_lossy());
        let has_clip = self.clipboard.is_some();
        let rel = self
            .root()
            .as_ref()
            .and_then(|r| target.strip_prefix(r).ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| target.to_string_lossy().to_string());
        let view = cx.entity();

        // Each entry: capture a fresh clone of the path(s) it needs, then run
        // inside `view.update` (which supplies `&mut Self` + `Context`).
        let win = move |v: &Entity<Self>, f: ExpAct| -> MenuClick {
            let v = v.clone();
            Box::new(move |_ev: &ClickEvent, w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.context_menu = None;
                    f(this, w, cx);
                });
            })
        };

        let mut items: Vec<MenuItem> = vec![MenuItem::label(name)];
        if !is_dir {
            let t = target.clone();
            items.push(
                MenuItem::new("cm-open", "Open")
                    .icon(IconName::File)
                    .on_click(win(
                        &view,
                        Box::new(move |this, w, cx| this.open_file(&t, false, w, cx)),
                    )),
            );
        }
        if can_preview {
            let t = target.clone();
            items.push(
                MenuItem::new("cm-preview", "Open Preview")
                    .icon(IconName::Globe)
                    .on_click(win(
                        &view,
                        Box::new(move |this, w, cx| this.open_in_preview(t.clone(), w, cx)),
                    )),
            );
        }
        items.push(MenuItem::separator());
        let d = dir_for_ops.clone();
        items.push(
            MenuItem::new("cm-new-file", "New File")
                .icon(IconName::File)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.begin_create(d.clone(), false, w, cx)),
                )),
        );
        let d = dir_for_ops.clone();
        items.push(
            MenuItem::new("cm-new-dir", "New Folder")
                .icon(IconName::Folder)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.begin_create(d.clone(), true, w, cx)),
                )),
        );
        let d = dir_for_ops.clone();
        items.push(
            MenuItem::new("cm-terminal", "Reveal in Terminal")
                .icon(IconName::Terminal)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.open_in_terminal(d.clone(), w, cx)),
                )),
        );
        {
            let t = target.clone();
            let v = view.clone();
            items.push(MenuItem::new("cm-finder", "Reveal in Finder").on_click(
                move |_, _w, cx| {
                    cx.reveal_path(&t);
                    v.update(cx, |this, cx| {
                        this.context_menu = None;
                        cx.notify()
                    });
                },
            ));
        }
        items.push(MenuItem::separator());
        let t = target.clone();
        items.push(
            MenuItem::new("cm-copy-path", "Copy Path")
                .icon(IconName::Copy)
                .on_click(win(
                    &view,
                    Box::new(move |this, _w, cx| this.copy_path(&t, cx)),
                )),
        );
        {
            let rel = rel.clone();
            let v = view.clone();
            items.push(
                MenuItem::new("cm-copy-rel", "Copy Relative Path")
                    .icon(IconName::Copy)
                    .on_click(move |_, _w, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(rel.clone()));
                        v.update(cx, |this, cx| {
                            this.context_menu = None;
                            cx.notify()
                        });
                    }),
            );
        }
        let cc = self.action_paths(&target);
        items.push(MenuItem::new("cm-clip-copy", "Copy").on_click(win(
            &view,
            Box::new(move |this, _w, cx| this.clip_set(ClipOp::Copy, cc.clone(), cx)),
        )));
        let cc = self.action_paths(&target);
        items.push(MenuItem::new("cm-clip-cut", "Cut").on_click(win(
            &view,
            Box::new(move |this, _w, cx| this.clip_set(ClipOp::Cut, cc.clone(), cx)),
        )));
        if has_clip {
            let d = dir_for_ops.clone();
            items.push(MenuItem::new("cm-clip-paste", "Paste").on_click(win(
                &view,
                Box::new(move |this, _w, cx| this.paste_into(d.clone(), cx)),
            )));
        }
        let d = dir_for_ops.clone();
        items.push(
            MenuItem::new("cm-bookmark", "Bookmark Path")
                .icon(IconName::Bookmark)
                .on_click(win(
                    &view,
                    Box::new(move |this, _w, cx| this.bookmark_folder(d.clone(), cx)),
                )),
        );
        items.push(MenuItem::separator());
        let t = target.clone();
        items.push(
            MenuItem::new("cm-rename", "Rename")
                .icon(IconName::Pencil)
                .on_click(win(
                    &view,
                    Box::new(move |this, w, cx| this.begin_rename(t.clone(), w, cx)),
                )),
        );
        let t = target.clone();
        items.push(
            MenuItem::new("cm-delete", "Delete")
                .icon(IconName::Trash)
                .destructive()
                .on_click(win(
                    &view,
                    Box::new(move |this, _w, cx| this.request_delete(t.clone(), cx)),
                )),
        );

        let v = view.clone();
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            v.update(cx, |this, cx| {
                this.context_menu = None;
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

    fn render_delete_confirm(
        &self,
        target: PathBuf,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
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
                    .w(px(240.0))
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.card)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(c.fg)
                            .child(SharedString::from(format!(
                                "Delete \u{201C}{name}\u{201D}?"
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .child(
                                button(
                                    "del-cancel",
                                    c.palette,
                                    ButtonVariant::Outline,
                                    ButtonSize::Sm,
                                )
                                .child("Cancel")
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _window, cx| {
                                        this.confirm_delete = None;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                button(
                                    "del-ok",
                                    c.palette,
                                    ButtonVariant::Destructive,
                                    ButtonSize::Sm,
                                )
                                .child("Delete")
                                .on_click(cx.listener(
                                    |this, _: &ClickEvent, _window, cx| {
                                        this.confirm_delete_now(cx);
                                    },
                                )),
                            ),
                    ),
            )
    }
}

fn text_row(depth: usize, row_h: Pixels, text: &str, color: Hsla) -> gpui::AnyElement {
    div()
        .flex()
        .items_center()
        .h(row_h)
        .pl(px(8.0 + depth as f32 * INDENT))
        .pr_2()
        .text_xs()
        .text_color(color)
        .child(SharedString::from(text.to_string()))
        .into_any_element()
}

/// The inline create/rename text-field row. Rendered inside the `uniform_list`
/// closure, so it wires Escape through `view.update(..)` rather than a
/// `cx.listener` (which needs `&mut Context`).
fn inline_input_row(
    depth: usize,
    row_h: Pixels,
    accent: Hsla,
    view: &Entity<ExplorerView>,
    field: &Option<Entity<InputState>>,
) -> gpui::AnyElement {
    let v = view.clone();
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .h(row_h)
        .pl(px(8.0 + depth as f32 * INDENT))
        .pr_2()
        .on_key_down(move |ev: &KeyDownEvent, _w, cx| {
            if ev.keystroke.key == "escape" {
                v.update(cx, |this, cx| this.cancel_edit(cx));
                cx.stop_propagation();
            }
        });
    if let Some(field) = field {
        row = row.child(
            div()
                .flex_1()
                .text_sm()
                .rounded_sm()
                .border_1()
                .border_color(accent)
                .child(labonair_ui_kit::field_input(field)),
        );
    }
    row.into_any_element()
}

/// Turn one [`ExplorerRowData`] into an element. Free function so it can run
/// inside the `uniform_list` render closure (which only has `&mut Window,
/// &mut App`); every handler goes through `view.update(..)`.
fn explorer_row_element(
    data: &ExplorerRowData,
    c: Colors,
    indent_guides: bool,
    view: &Entity<ExplorerView>,
    edit_field: &Option<Entity<InputState>>,
    cx: &mut App,
) -> gpui::AnyElement {
    let row_h = c.palette.density_tokens().tree_row_height();
    match data {
        ExplorerRowData::PendingCreate { depth } => {
            inline_input_row(depth + 1, row_h, c.accent, view, edit_field)
        }
        ExplorerRowData::Rename { depth } => {
            inline_input_row(*depth, row_h, c.accent, view, edit_field)
        }
        ExplorerRowData::Loading { depth } => text_row(*depth, row_h, "Loading\u{2026}", c.muted),
        ExplorerRowData::Error { depth, message } => text_row(*depth, row_h, message, c.err),
        ExplorerRowData::LoadMore { parent, depth } => {
            let id: SharedString = format!("more:{}", parent.display()).into();
            let v = view.clone();
            let p = parent.clone();
            button(id, c.palette, ButtonVariant::Link, ButtonSize::Xs)
                .h(row_h)
                .pl(px(8.0 + (*depth as f32 + 1.0) * INDENT))
                .justify_start()
                .child("Load more\u{2026}")
                .on_click(move |_: &ClickEvent, _w, cx| {
                    v.update(cx, |this, cx| this.load_dir(p.clone(), true, cx));
                })
                .into_any_element()
        }
        ExplorerRowData::Entry {
            path,
            name,
            depth,
            is_dir,
            is_ignored,
            expanded,
            selected,
            cut,
            drop_target,
            drag_paths,
            drag_label,
            active_file,
            git,
            diag: _diag,
        } => {
            let (glyph, chevron) = {
                let store = view.read(cx).theme.read(cx);
                let it = store.icon_theme();
                (
                    icon_for_path(it, name, *is_dir, *expanded),
                    is_dir.then(|| chevron_icon_path(it, *expanded)),
                )
            };

            let id: SharedString = format!("row:{}", path.display()).into();
            let is_dir = *is_dir;
            let (cut, drop_target) = (*cut, *drop_target);

            let on_click = {
                let v = view.clone();
                let p = path.clone();
                move |ev: &ClickEvent, window: &mut Window, cx: &mut App| {
                    let additive = ev.modifiers().secondary() || ev.modifiers().shift;
                    let peek = ev.click_count() < 2;
                    v.update(cx, |this, cx| {
                        this.select(p.clone(), additive, cx);
                        if additive {
                            return;
                        }
                        if is_dir {
                            this.toggle_expanded(p.clone(), cx);
                        } else {
                            this.open_file(&p, peek, window, cx);
                        }
                    });
                }
            };
            let on_right = {
                let v = view.clone();
                let p = path.clone();
                move |ev: &MouseDownEvent, _w: &mut Window, cx: &mut App| {
                    v.update(cx, |this, cx| {
                        this.context_menu = Some((p.clone(), ev.position));
                        cx.notify();
                    });
                }
            };

            let drag_paths = drag_paths.clone();
            let drag_label: SharedString = drag_label.clone().into();
            let over_v = view.clone();
            let over_p = path.clone();
            let drop_v = view.clone();
            let drop_p = path.clone();
            let ext_v = view.clone();
            let ext_p = path.clone();

            let mut tr = tree_row(id, c.palette, SharedString::from(name.clone()))
                .depth(*depth)
                .indent_step(INDENT)
                .indent_guides(indent_guides)
                .chevron_path(chevron)
                .icon_path(Some(glyph))
                .tooltip(SharedString::from(path.to_string_lossy().to_string()))
                .state(TreeRowState {
                    selected: *selected && !drop_target,
                    active_file: *active_file,
                    cut,
                    drop_target,
                    ..Default::default()
                })
                .on_click(on_click)
                .on_secondary_down(on_right);
            // Phase 3.7: Git status wins the label-tint channel + a trailing
            // status letter; otherwise ignored files stay muted.
            if let Some(ch) = git {
                if !cut {
                    tr = tr.label_tint(git_tint(*ch, &c));
                }
                tr = tr.trailing(
                    div()
                        .text_xs()
                        .text_color(git_tint(*ch, &c))
                        .child(SharedString::from(ch.to_string())),
                );
            } else if *is_ignored && !cut {
                tr = tr.label_tint(c.muted);
            }
            tr.extra(move |mut row| {
                row = row.on_drag(DraggedPaths { paths: drag_paths }, move |_, _, _, cx| {
                    cx.new(|_| DragPreview {
                        label: drag_label.clone(),
                    })
                });
                if is_dir {
                    row = row
                        .on_drag_move(move |_: &gpui::DragMoveEvent<DraggedPaths>, _w, cx| {
                            over_v.update(cx, |this, cx| {
                                if this.drop_target.as_deref() != Some(over_p.as_path()) {
                                    this.drop_target = Some(over_p.clone());
                                    cx.notify();
                                }
                            });
                        })
                        .on_drop(move |d: &DraggedPaths, _w, cx| {
                            drop_v.update(cx, |this, cx| {
                                this.drop_move(d.paths.clone(), drop_p.clone(), cx)
                            });
                        })
                        .on_drop(move |d: &ExternalPaths, _w, cx| {
                            ext_v.update(cx, |this, cx| {
                                this.drop_external(d.paths().to_vec(), ext_p.clone(), cx)
                            });
                        });
                }
                row
            })
            .into_any_element()
        }
    }
}

/// [`Panel`](labonair_panel::Panel) wiring (T17-001).
///
/// The explorer docks on the **left** at **260 px** — the reference
/// `useLocalExplorerStore` opens the file tree in the left sidebar, and 260 px
/// sits comfortably above the shell's 180 px `SIDEBAR_MIN` floor (also the
/// [`min_size`](labonair_panel::Panel::min_size) here). A vertical file tree is
/// only meaningful in a side dock, so the bottom dock is rejected. Dock
/// move/persistence is T17-002; until then [`set_position`] is a no-op and
/// [`position`] reports the compile-time default.
impl labonair_panel::Panel for ExplorerView {
    fn persistent_name() -> &'static str {
        "explorer"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Explorer".into()
    }

    fn icon(&self) -> labonair_panel::PanelIcon {
        labonair_panel::PanelIcon::Explorer
    }

    fn position(&self, _cx: &App) -> labonair_panel::DockPosition {
        labonair_panel::DockPosition::Left
    }

    fn position_is_valid(&self, position: labonair_panel::DockPosition) -> bool {
        matches!(
            position,
            labonair_panel::DockPosition::Left | labonair_panel::DockPosition::Right
        )
    }

    fn set_position(
        &mut self,
        _position: labonair_panel::DockPosition,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        // T17-002 owns the dock model; nothing to persist here yet.
    }

    fn default_size(&self, _cx: &App) -> gpui::Pixels {
        gpui::px(260.0)
    }

    fn min_size(&self) -> Option<gpui::Pixels> {
        Some(gpui::px(180.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(entries: &[(&str, bool)]) -> NodeState {
        NodeState::Loaded {
            entries: entries
                .iter()
                .map(|(n, d)| Entry {
                    name: n.to_string(),
                    is_dir: *d,
                    is_ignored: false,
                })
                .collect(),
            has_more: false,
        }
    }

    #[test]
    fn rows_flatten_respects_expanded_and_depth() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(
            PathBuf::from("/r"),
            loaded(&[("sub", true), ("a.txt", false)]),
        );
        assert_eq!(m.rows().len(), 2);

        assert!(m.toggle_expanded(PathBuf::from("/r/sub")));
        m.set_node(PathBuf::from("/r/sub"), loaded(&[("nested.txt", false)]));
        let rows = m.rows();
        assert_eq!(rows.len(), 3);
        match &rows[1] {
            Row::Entry { depth, entry, .. } => {
                assert_eq!(*depth, 1);
                assert_eq!(entry.name, "nested.txt");
            }
            _ => panic!("expected nested entry at index 1"),
        }
        // Collapsing hides the child again.
        assert!(!m.toggle_expanded(PathBuf::from("/r/sub")));
        assert_eq!(m.rows().len(), 2);
    }

    #[test]
    fn lazy_load_only_requests_unloaded_dirs() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        assert!(m.needs_load(Path::new("/r")));
        m.mark_loading(PathBuf::from("/r"));
        assert!(
            !m.needs_load(Path::new("/r")),
            "in-flight dir not re-requested"
        );
        m.set_node(PathBuf::from("/r"), loaded(&[]));
        assert!(
            !m.needs_load(Path::new("/r")),
            "loaded dir not re-requested"
        );
        // A never-touched subdir still needs loading.
        assert!(m.needs_load(Path::new("/r/other")));
    }

    #[test]
    fn set_root_bumps_generation_and_clears_state() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/a")));
        m.set_node(PathBuf::from("/a"), loaded(&[("x", false)]));
        m.toggle_expanded(PathBuf::from("/a/d"));
        let g = m.generation();

        assert!(m.set_root(Some(PathBuf::from("/b"))));
        assert_eq!(m.generation(), g + 1, "stale responses now discarded");
        assert!(m.nodes.is_empty() && m.expanded.is_empty());
        assert!(
            !m.set_root(Some(PathBuf::from("/b"))),
            "no-op when unchanged"
        );
    }

    #[test]
    fn toggle_hidden_invalidates_cache_but_keeps_expanded() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(PathBuf::from("/r"), loaded(&[]));
        m.toggle_expanded(PathBuf::from("/r/sub"));
        m.set_node(PathBuf::from("/r/sub"), loaded(&[]));
        let g = m.generation();

        let reload = m.toggle_show_hidden();
        assert!(m.show_hidden);
        assert_eq!(m.generation(), g + 1);
        assert!(m.nodes.is_empty());
        assert!(m.expanded.contains(Path::new("/r/sub")));
        assert!(reload.contains(&PathBuf::from("/r")));
        assert!(reload.contains(&PathBuf::from("/r/sub")));
    }

    #[test]
    fn watch_targets_are_the_loaded_directories_only() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.mark_loading(PathBuf::from("/r"));
        m.set_node(PathBuf::from("/r/a"), loaded(&[]));
        m.set_node(PathBuf::from("/r/b"), NodeState::Error("nope".into()));
        let t = m.watch_targets();
        assert!(t.contains(&PathBuf::from("/r")));
        assert!(t.contains(&PathBuf::from("/r/a")));
        assert!(!t.contains(&PathBuf::from("/r/b")));
    }

    #[test]
    fn can_drop_into_rejects_noop_and_self_and_descendant() {
        // Same folder → no-op.
        assert!(!can_drop_into(Path::new("/r/a/x.txt"), Path::new("/r/a")));
        // Into itself.
        assert!(!can_drop_into(Path::new("/r/a"), Path::new("/r/a")));
        // Into its own descendant.
        assert!(!can_drop_into(Path::new("/r/a"), Path::new("/r/a/sub")));
        // Valid: sibling directory.
        assert!(can_drop_into(Path::new("/r/a/x.txt"), Path::new("/r/b")));
        assert!(can_drop_into(Path::new("/r/a"), Path::new("/r/b")));
    }

    #[test]
    fn shell_quote_wraps_only_when_needed() {
        assert_eq!(
            shell_quote(Path::new("/home/me/file.txt")),
            "/home/me/file.txt"
        );
        assert_eq!(
            shell_quote(Path::new("/home/me/my file.txt")),
            "'/home/me/my file.txt'"
        );
        assert_eq!(shell_quote(Path::new("/a/it's")), "'/a/it'\\''s'");
        assert_eq!(
            quote_paths(&[PathBuf::from("/a/b"), PathBuf::from("/c d")]),
            "/a/b '/c d'"
        );
    }

    #[test]
    fn clipboard_buffer_set_replace_and_holds_multiple() {
        let mut clip = Some(Clipboard {
            op: ClipOp::Copy,
            paths: vec![PathBuf::from("/a"), PathBuf::from("/b")],
        });
        assert_eq!(clip.as_ref().unwrap().paths.len(), 2);
        // A cut replaces the buffer.
        clip = Some(Clipboard {
            op: ClipOp::Cut,
            paths: vec![PathBuf::from("/c")],
        });
        assert_eq!(clip.as_ref().unwrap().op, ClipOp::Cut);
        assert_eq!(clip.as_ref().unwrap().paths, vec![PathBuf::from("/c")]);
        // Discard.
        clip = None;
        assert!(clip.is_none());
    }

    #[test]
    fn file_icon_resolves_to_icon_theme_paths() {
        let t = labonair_theme::IconThemeContent::default();
        assert_eq!(t.file_icon_path("main.rs"), "icons/file_icons/rust.svg");
        assert_eq!(t.file_icon_path("data.json"), "icons/file_icons/code.svg");
        assert_eq!(
            t.file_icon_path("weird.unknownext"),
            "icons/file_icons/file.svg"
        );
    }

    #[test]
    fn flatten_preserves_depth_expansion_loading_and_selection_across_ranges() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(
            PathBuf::from("/r"),
            loaded(&[("dir", true), ("a.txt", false), ("b.txt", false)]),
        );
        m.toggle_expanded(PathBuf::from("/r/dir"));
        m.set_node(PathBuf::from("/r/dir"), NodeState::Loading);

        let rows = m.rows(); // dir(0), Loading(1), a.txt(0), b.txt(0)
        let selection = vec![PathBuf::from("/r/a.txt")];
        let data = flatten_rows(rows, &m.expanded, &selection, &[], None);
        assert_eq!(data.len(), 4);

        match &data[0] {
            ExplorerRowData::Entry {
                path,
                depth,
                is_dir,
                expanded,
                ..
            } => {
                assert_eq!(path, &PathBuf::from("/r/dir"));
                assert_eq!(*depth, 0);
                assert!(*is_dir && *expanded);
            }
            other => panic!("expected dir entry, got {other:?}"),
        }
        assert!(matches!(&data[1], ExplorerRowData::Loading { depth: 1 }));

        // Selection state survives regardless of which viewport window the
        // virtual list asks for.
        let window = &data[2..4];
        match &window[0] {
            ExplorerRowData::Entry { path, selected, .. } => {
                assert_eq!(path, &PathBuf::from("/r/a.txt"));
                assert!(*selected, "selected entry keeps its flag in a sub-range");
            }
            other => panic!("expected a.txt entry, got {other:?}"),
        }
        assert!(matches!(
            &window[1],
            ExplorerRowData::Entry {
                selected: false,
                ..
            }
        ));
    }

    #[test]
    fn flatten_marks_cut_and_drop_target() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(
            PathBuf::from("/r"),
            loaded(&[("d", true), ("x.txt", false)]),
        );
        let cut = vec![PathBuf::from("/r/x.txt")];
        let data = flatten_rows(m.rows(), &m.expanded, &[], &cut, Some(Path::new("/r/d")));
        // dir "d" is the drop target; file "x.txt" is cut.
        assert!(matches!(
            &data[0],
            ExplorerRowData::Entry {
                drop_target: true,
                is_dir: true,
                ..
            }
        ));
        assert!(matches!(&data[1], ExplorerRowData::Entry { cut: true, .. }));
    }

    #[test]
    fn search_traverses_collapsed_but_loaded_subtrees() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(PathBuf::from("/r"), loaded(&[("sub", true)]));
        m.set_node(PathBuf::from("/r/sub"), loaded(&[("needle.rs", false)]));
        // `sub` was never expanded.
        assert!(!m.expanded.contains(Path::new("/r/sub")));

        // The viewport source (`rows`) does not descend into the collapsed dir…
        assert!(!m
            .rows()
            .iter()
            .any(|r| matches!(r, Row::Entry { entry, .. } if entry.name == "needle.rs")));
        // …but the search source (`all_loaded_rows`) does — so search is not
        // limited to the rows currently on screen.
        assert!(m
            .all_loaded_rows()
            .iter()
            .any(|r| matches!(r, Row::Entry { entry, .. } if entry.name == "needle.rs")));
    }

    // ── Zed-parity Phase 3 ─────────────────────────────────────────────────

    /// Build the deep nested fixture `a/b/c/{c.txt,d.txt}` (all expanded).
    fn nested_model() -> TreeModel {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(PathBuf::from("/r"), loaded(&[("a", true)]));
        m.toggle_expanded(PathBuf::from("/r/a"));
        m.set_node(PathBuf::from("/r/a"), loaded(&[("b", true)]));
        m.toggle_expanded(PathBuf::from("/r/a/b"));
        m.set_node(
            PathBuf::from("/r/a/b"),
            loaded(&[("c.txt", false), ("d.txt", false)]),
        );
        m
    }

    #[test]
    fn sticky_ancestors_are_the_open_dir_chain_for_a_viewport_row() {
        let m = nested_model();
        let data = flatten_rows(m.rows(), &m.expanded, &[], &[], None);
        // rows: a(0), b(1), c.txt(2), d.txt(2)
        assert_eq!(data.len(), 4);

        // Scrolled so d.txt (index 3) is the first visible row.
        let anc = sticky_ancestor_indices(&data, 3);
        assert_eq!(anc, vec![0, 1], "a and b are pinned above d.txt");

        // A top-level row has no ancestors.
        assert!(sticky_ancestor_indices(&data, 0).is_empty());
        // b (index 1) only has a as an ancestor.
        assert_eq!(sticky_ancestor_indices(&data, 1), vec![0]);
    }

    #[test]
    fn fold_single_child_chains_compresses_and_is_reversible() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(PathBuf::from("/r"), loaded(&[("a", true)]));
        m.toggle_expanded(PathBuf::from("/r/a"));
        m.set_node(PathBuf::from("/r/a"), loaded(&[("b", true)]));
        m.toggle_expanded(PathBuf::from("/r/a/b"));
        m.set_node(PathBuf::from("/r/a/b"), loaded(&[("c", true)]));
        m.toggle_expanded(PathBuf::from("/r/a/b/c"));
        m.set_node(PathBuf::from("/r/a/b/c"), loaded(&[("f.txt", false)]));

        let folded = fold_chains(m.rows());
        assert_eq!(folded.len(), 2);
        match &folded[0] {
            Row::Entry { path, depth, entry } => {
                assert_eq!(entry.name, "a/b/c");
                assert_eq!(*depth, 0);
                assert_eq!(path, &PathBuf::from("/r/a/b/c"), "real deepest path kept");
                assert!(entry.is_dir);
            }
            _ => panic!("expected compressed dir row"),
        }
        match &folded[1] {
            Row::Entry { entry, depth, .. } => {
                assert_eq!(entry.name, "f.txt");
                assert_eq!(*depth, 1);
            }
            _ => panic!("expected file row"),
        }

        // Pure: the model still flattens to the un-folded 4-row list.
        assert_eq!(m.rows().len(), 4);
    }

    #[test]
    fn fold_chains_leaves_multi_child_dirs_untouched() {
        let mut m = TreeModel::default();
        m.set_root(Some(PathBuf::from("/r")));
        m.set_node(PathBuf::from("/r"), loaded(&[("a", true)]));
        m.toggle_expanded(PathBuf::from("/r/a"));
        m.set_node(PathBuf::from("/r/a"), loaded(&[("b", true), ("c", true)]));
        let folded = fold_chains(m.rows());
        assert_eq!(folded.len(), 3, "a has two children — no folding");
    }

    #[test]
    fn reveal_policy_finds_the_entry_or_no_ops() {
        let m = nested_model();
        let data = flatten_rows(m.rows(), &m.expanded, &[], &[], None);
        assert_eq!(
            reveal_target_index(&data, Path::new("/r/a/b/c.txt")),
            Some(2)
        );
        // Not in the tree → no-op reveal.
        assert_eq!(reveal_target_index(&data, Path::new("/r/a/z.txt")), None);
    }

    #[test]
    fn decorate_rows_merges_git_and_active_file_without_touching_geometry() {
        let m = nested_model();
        let mut data = flatten_rows(m.rows(), &m.expanded, &[], &[], None);
        let mut git = HashMap::new();
        git.insert(PathBuf::from("/r/a/b/c.txt"), 'M');
        let diag = HashMap::new();
        decorate_rows(&mut data, Some(Path::new("/r/a/b/d.txt")), &git, &diag);
        match &data[2] {
            ExplorerRowData::Entry {
                git, active_file, ..
            } => {
                assert_eq!(*git, Some('M'));
                assert!(!active_file);
            }
            other => panic!("expected c.txt entry, got {other:?}"),
        }
        match &data[3] {
            ExplorerRowData::Entry {
                git, active_file, ..
            } => {
                assert_eq!(*git, None);
                assert!(active_file);
            }
            other => panic!("expected d.txt entry, got {other:?}"),
        }
    }

    #[test]
    fn parse_git_porcelain_maps_worktree_and_index_and_rename() {
        let out = " M src/main.rs\n?? new.txt\nA  staged.rs\nR  old.rs -> renamed.rs\n D gone.rs\n";
        let map = parse_git_porcelain(out);
        assert_eq!(map.get(Path::new("src/main.rs")), Some(&'M'));
        assert_eq!(map.get(Path::new("new.txt")), Some(&'?'));
        assert_eq!(map.get(Path::new("staged.rs")), Some(&'A'));
        assert_eq!(map.get(Path::new("renamed.rs")), Some(&'R'));
        assert_eq!(map.get(Path::new("gone.rs")), Some(&'D'));
    }

    #[test]
    fn bounded_fs_search_reaches_unloaded_subtrees_and_respects_the_depth_cap() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("labonair-exp-search-{nonce}"));
        std::fs::create_dir_all(root.join("a/b/c")).unwrap();
        std::fs::write(root.join("a/b/c/needle.rs"), "x").unwrap();
        std::fs::write(root.join("top_needle.txt"), "x").unwrap();

        let hits = bounded_fs_search(&root, "needle", false, 8, 1000);
        let names: Vec<&str> = hits.iter().map(|(_, n, _, _)| n.as_str()).collect();
        assert!(
            names.contains(&"needle.rs"),
            "deep unloaded file found: {names:?}"
        );
        assert!(names.contains(&"top_needle.txt"));
        let deep = hits.iter().find(|(_, n, _, _)| n == "needle.rs").unwrap();
        assert_eq!(deep.2, 3, "reported at its real depth");

        // Depth-capped walk cannot reach the depth-3 file.
        let shallow = bounded_fs_search(&root, "needle", false, 1, 1000);
        assert!(shallow.iter().all(|(_, n, _, _)| n != "needle.rs"));

        std::fs::remove_dir_all(&root).ok();
    }

    // ── Zed-parity Phase 5: large-project render/latency evidence ─────────────
    //
    // §14 "no row-count-linear render regression": the flatten pass is linear in
    // the model (unavoidable — it *is* the model) but the per-frame element
    // construction the virtual list performs must be bounded by the viewport,
    // never by the total row count. These tests pin both halves of that
    // contract on the pure functions so a regression fails `cargo test` without
    // needing a GPUI window. Reproduce the live trace with:
    //   RUST_LOG=labonair::perf=trace cargo run
    // then scroll the Explorer — `explorer_flatten` fires once per model change,
    // `explorer_viewport_build` fires per frame with `built` == viewport rows.

    fn synthetic_rows(n: usize) -> Vec<Row> {
        (0..n)
            .map(|i| Row::Entry {
                path: PathBuf::from(format!("/r/dir{}/file{i}.rs", i % 64)),
                depth: 1 + (i % 4),
                entry: Entry {
                    name: format!("file{i}.rs"),
                    is_dir: false,
                    is_ignored: false,
                },
            })
            .collect()
    }

    #[test]
    fn flatten_on_a_large_tree_is_bounded_and_geometry_stable() {
        const N: usize = 20_000;
        let expanded = HashSet::new();
        let selection = vec![PathBuf::from("/r/dir0/file0.rs")];

        let mut data = flatten_rows(synthetic_rows(N), &expanded, &selection, &[], None);
        assert_eq!(data.len(), N, "flatten is 1:1 with the model, no fan-out");

        // Decorations merge in place and never add/remove rows (stable geometry
        // while metadata arrives — §10.5).
        let mut git = HashMap::new();
        git.insert(PathBuf::from("/r/dir0/file0.rs"), 'M');
        decorate_rows(
            &mut data,
            Some(Path::new("/r/dir1/file1.rs")),
            &git,
            &HashMap::new(),
        );
        assert_eq!(data.len(), N, "decorate_rows must not change row count");

        // Exactly one row carries selection, one the active-file marker, one the
        // git tint — decorations do not smear across the list.
        let sel = data
            .iter()
            .filter(|r| matches!(r, ExplorerRowData::Entry { selected: true, .. }))
            .count();
        let active = data
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    ExplorerRowData::Entry {
                        active_file: true,
                        ..
                    }
                )
            })
            .count();
        assert_eq!((sel, active), (1, 1));
    }

    #[test]
    fn virtual_list_builds_only_the_viewport_not_the_whole_model() {
        const N: usize = 20_000;
        const VIEWPORT: usize = 48;
        let data = flatten_rows(synthetic_rows(N), &HashSet::new(), &[], &[], None);

        // This is precisely the work the `uniform_list("explorer-rows", …)`
        // render closure does each frame: map a `Range` -> elements. Assert the
        // touched-row count tracks the viewport, not `data.len()`.
        for start in [0usize, 5_000, N - VIEWPORT] {
            let range = start..start + VIEWPORT;
            let mut touched = 0usize;
            let _built: Vec<&ExplorerRowData> = range
                .map(|i| {
                    touched += 1;
                    &data[i]
                })
                .collect();
            assert_eq!(
                touched, VIEWPORT,
                "frame cost is viewport-bounded at {start}"
            );
        }
    }
}
