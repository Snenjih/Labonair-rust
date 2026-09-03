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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, AppContext, ClickEvent, ClipboardItem, Context, Entity, ExternalPaths,
    FocusHandle, Focusable, Hsla, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, Render, SharedString, StatefulInteractiveElement,
    Styled, Task, Window,
};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, Debouncer};

use labonair_backend::modules::fs::{mutate, tree};

use crate::notifications::{notification_center, Notification};
use crate::theme::ThemeStore;
use crate::workspace::Workspace;
use labonair_ui_kit::{
    context_menu, file_icon, folder_icon, IconName, InputEvent, InputState, MenuClick, MenuItem,
};

/// A menu action expressed against the view + window (wrapped into a
/// [`MenuClick`] by `render_context_menu`).
type ExpAct = Box<dyn Fn(&mut ExplorerView, &mut Window, &mut Context<ExplorerView>)>;

const PAGE_LIMIT: usize = tree::DEFAULT_LOCAL_PAGE_LIMIT;
const INDENT: f32 = 12.0;
const WATCH_DEBOUNCE: Duration = Duration::from_millis(300);
const DRAIN_INTERVAL: Duration = Duration::from_millis(400);

/// Payload of an in-tree drag (T05-002). Pure-data drag, mirroring the
/// reference `explorerDrag` module singleton — carries the selected paths from
/// an explorer row to a drop target (a folder in the same tree, or a terminal
/// pane which inserts the quoted path).
#[derive(Clone)]
pub struct DraggedPaths {
    pub paths: Vec<PathBuf>,
}

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

/// Shell-quote a single path for insertion into a terminal (single-quote wrap
/// unless it is entirely "safe" characters).
pub fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || "-_./=:@%+,".contains(c));
    if safe {
        s.into_owned()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Space-joined shell-quoted paths (drag-into-terminal payload).
pub fn quote_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| shell_quote(p))
        .collect::<Vec<_>>()
        .join(" ")
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

    fn collapse_all(&mut self) {
        self.expanded.clear();
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
    context_menu: Option<(PathBuf, Point<Pixels>)>,
    confirm_delete: Option<PathBuf>,
    focus: FocusHandle,
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
            context_menu: None,
            confirm_delete: None,
            focus: cx.focus_handle(),
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
        self.confirm_delete = None;
        self.edit_buffer.clear();
        if let Some(root) = self.model.root.clone() {
            self.load_dir(root, false, cx);
        }
        self.sync_watchers();
        cx.notify();
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

    fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.model.collapse_all();
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

    fn is_cut(&self, path: &Path) -> bool {
        matches!(&self.clipboard, Some(c) if c.op == ClipOp::Cut && c.paths.iter().any(|p| p == path))
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
        for dir in dirs {
            if self.model.nodes.contains_key(&dir) {
                self.load_dir(dir, true, cx);
            }
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
}

impl Render for ExplorerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = {
            let t = self.theme.read(cx);
            Colors {
                fg: t.foreground(),
                muted: t.muted_foreground(),
                accent: t.accent(),
                border: t.border(),
                card: t.card(),
                err: t.status_error(),
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

        let root_file = root.clone();
        let root_dir = root.clone();
        let root_refresh = root.clone();
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
                    .text_xs()
                    .text_color(c.muted)
                    .child(SharedString::from(root_name)),
            )
            .child(self.icon_btn(
                "new-file",
                IconName::Plus,
                c,
                cx,
                move |this, window, cx| this.begin_create(root_file.clone(), false, window, cx),
            ))
            .child(self.icon_btn(
                "new-dir",
                IconName::FolderOpen,
                c,
                cx,
                move |this, window, cx| this.begin_create(root_dir.clone(), true, window, cx),
            ))
            .child(self.icon_btn(
                "refresh",
                IconName::Refresh,
                c,
                cx,
                move |this, _window, cx| this.load_dir(root_refresh.clone(), true, cx),
            ))
            .child(self.icon_btn(
                "toggle-hidden",
                if self.model.show_hidden {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                },
                c,
                cx,
                move |this, _window, cx| this.toggle_show_hidden(cx),
            ))
            .child(self.icon_btn(
                "collapse",
                IconName::Minus,
                c,
                cx,
                move |this, _window, cx| this.collapse_all(cx),
            ));

        let root_drop = root.clone();
        let root_ext = root.clone();
        let root_over = root.clone();
        let list = div()
            .id("explorer-list")
            .flex_1()
            .overflow_y_scroll()
            .py_1()
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
            .children(
                self.model
                    .rows()
                    .into_iter()
                    .map(|row| self.render_row(row, c, cx))
                    .collect::<Vec<_>>(),
            );

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
            .children(self.render_clip_banner(c, cx))
            .child(list);

        if let Some((target, pos)) = self.context_menu.clone() {
            container = container.child(self.render_context_menu(target, pos, c, cx));
        }
        if let Some(target) = self.confirm_delete.clone() {
            container = container.child(self.render_delete_confirm(target, c, cx));
        }

        container.into_any_element()
    }
}

impl ExplorerView {
    fn icon_btn(
        &self,
        id: &'static str,
        icon: IconName,
        c: Colors,
        cx: &mut Context<Self>,
        handler: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id)
            .h(px(20.0))
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .text_xs()
            .text_color(c.muted)
            .hover(|s| s.bg(c.border))
            .child(icon.svg(c.muted))
            .on_click(
                cx.listener(move |this, _: &ClickEvent, window, cx| handler(this, window, cx)),
            )
    }

    /// Explorer-level keyboard: copy / cut / paste buffer + clear.
    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
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
                    div()
                        .id("clip-paste")
                        .px_1()
                        .rounded_sm()
                        .text_color(c.accent)
                        .hover(|s| s.bg(c.border))
                        .child("Paste")
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            if let Some(dir) = this.paste_dir() {
                                this.paste_into(dir, cx);
                            }
                        })),
                )
                .child(
                    div()
                        .id("clip-clear")
                        .px_1()
                        .rounded_sm()
                        .text_color(c.muted)
                        .hover(|s| s.bg(c.border))
                        .child("Clear")
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.clip_clear(cx))),
                )
                .into_any_element(),
        )
    }

    fn on_edit_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" {
            self.cancel_edit(cx);
            cx.stop_propagation();
        }
    }

    fn render_inline_input(
        &self,
        depth: usize,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut row = div()
            .flex()
            .flex_row()
            .items_center()
            .pl(px(8.0 + depth as f32 * INDENT))
            .pr_2()
            .py(px(1.0))
            .on_key_down(cx.listener(Self::on_edit_key));
        if let Some(field) = &self.edit_field {
            row = row.child(
                div()
                    .flex_1()
                    .text_sm()
                    .rounded_sm()
                    .border_1()
                    .border_color(c.accent)
                    .child(labonair_ui_kit::field_input(field)),
            );
        }
        row
    }

    fn render_row(&self, row: Row, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        match row {
            Row::PendingCreate { depth } => self
                .render_inline_input(depth + 1, c, cx)
                .into_any_element(),
            Row::Rename { depth } => self.render_inline_input(depth, c, cx).into_any_element(),
            Row::Loading { depth } => text_row(depth, "Loading\u{2026}", c.muted),
            Row::Error { depth, message } => text_row(depth, &message, c.err),
            Row::LoadMore { parent, depth } => {
                let id: SharedString = format!("more:{}", parent.display()).into();
                div()
                    .id(id)
                    .pl(px(8.0 + (depth as f32 + 1.0) * INDENT))
                    .py(px(2.0))
                    .text_xs()
                    .text_color(c.accent)
                    .hover(|s| s.underline())
                    .child("Load more\u{2026}")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.load_dir(parent.clone(), true, cx);
                    }))
                    .into_any_element()
            }
            Row::Entry { path, depth, entry } => {
                let is_selected = self.is_selected(&path);
                let is_cut = self.is_cut(&path);
                let is_drop_target =
                    entry.is_dir && self.drop_target.as_deref() == Some(path.as_path());
                let is_expanded = entry.is_dir && self.model.expanded.contains(&path);
                let chevron = if entry.is_dir {
                    Some(if is_expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                } else {
                    None
                };
                let glyph = if entry.is_dir {
                    folder_icon(is_expanded)
                } else {
                    file_icon(&entry.name)
                };
                let id: SharedString = format!("row:{}", path.display()).into();
                let click_path = path.clone();
                let menu_path = path.clone();
                let over_path = path.clone();
                let drop_path = path.clone();
                let is_dir = entry.is_dir;
                let drag_paths = self.action_paths(&path);
                let drag_label: SharedString = if drag_paths.len() > 1 {
                    format!("{} items", drag_paths.len()).into()
                } else {
                    entry.name.clone().into()
                };
                let mut row = div()
                    .id(id)
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .pl(px(8.0 + depth as f32 * INDENT))
                    .pr_2()
                    .py(px(2.0))
                    .text_sm()
                    .when(is_drop_target, |d| d.bg(c.accent))
                    .when(is_selected && !is_drop_target, |d| d.bg(c.border))
                    .when(!is_selected && !is_drop_target, |d| {
                        d.hover(|s| s.bg(c.card))
                    })
                    .when(entry.is_ignored, |d| d.text_color(c.muted))
                    .when(is_cut, |d| d.opacity(0.5).text_color(c.err))
                    .child(
                        div()
                            .w(px(10.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .children(chevron.map(|ch| ch.svg(c.muted).size(px(12.0)))),
                    )
                    .child(div().child(glyph.svg(c.muted)))
                    .child(div().flex_1().child(SharedString::from(entry.name.clone())))
                    .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                        let additive = ev.modifiers().secondary() || ev.modifiers().shift;
                        this.select(click_path.clone(), additive, cx);
                        if additive {
                            return;
                        }
                        if is_dir {
                            this.toggle_expanded(click_path.clone(), cx);
                        } else {
                            this.open_file(&click_path, ev.click_count() < 2, window, cx);
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, ev: &MouseDownEvent, _window, cx| {
                            this.context_menu = Some((menu_path.clone(), ev.position));
                            cx.notify();
                        }),
                    )
                    .on_drag(DraggedPaths { paths: drag_paths }, move |_, _, _, cx| {
                        cx.new(|_| DragPreview {
                            label: drag_label.clone(),
                        })
                    });

                if is_dir {
                    row = row
                        .on_drag_move(cx.listener(
                            move |this, _: &gpui::DragMoveEvent<DraggedPaths>, _w, cx| {
                                if this.drop_target.as_deref() != Some(over_path.as_path()) {
                                    this.drop_target = Some(over_path.clone());
                                    cx.notify();
                                }
                            },
                        ))
                        .on_drop(cx.listener(move |this, d: &DraggedPaths, _w, cx| {
                            this.drop_move(d.paths.clone(), drop_path.clone(), cx);
                        }))
                        .on_drop(cx.listener({
                            let drop_path = path.clone();
                            move |this, d: &ExternalPaths, _w, cx| {
                                this.drop_external(d.paths().to_vec(), drop_path.clone(), cx);
                            }
                        }));
                }
                row.into_any_element()
            }
        }
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
        context_menu(pos, self.theme.read(cx), dismiss, items)
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
                                div()
                                    .id("del-cancel")
                                    .px_2()
                                    .py_1()
                                    .text_sm()
                                    .rounded_sm()
                                    .hover(|s| s.bg(c.border))
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                        this.confirm_delete = None;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("del-ok")
                                    .px_2()
                                    .py_1()
                                    .text_sm()
                                    .rounded_sm()
                                    .bg(c.err)
                                    .text_color(c.card)
                                    .hover(|s| s.opacity(0.9))
                                    .child("Delete")
                                    .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
                                        this.confirm_delete_now(cx);
                                    })),
                            ),
                    ),
            )
    }
}

fn text_row(depth: usize, text: &str, color: Hsla) -> gpui::AnyElement {
    div()
        .pl(px(8.0 + depth as f32 * INDENT))
        .py(px(2.0))
        .text_xs()
        .text_color(color)
        .child(SharedString::from(text.to_string()))
        .into_any_element()
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
    fn file_icon_maps_known_extensions() {
        assert_eq!(file_icon("main.rs"), IconName::FileCode);
        assert_eq!(file_icon("data.json"), IconName::FileJson);
        assert_eq!(file_icon("weird.unknownext"), IconName::File);
    }
}
