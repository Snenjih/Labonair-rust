//! Workspace-level DiffView (`docs/parity/diff-view-spec.md`).
//!
//! One scrolling document lists every changed file: a file header (fold,
//! stage checkbox, icon, name, directory, `+N −M`, Open File), then its hunks
//! with line numbers, row tints, word diff and — on hover — the hunk controls
//! (Stage / Unstage / Restore). A toolbar carries the diff stat, hunk
//! navigation, stage actions, Commit and the Unified / Split switch.
//!
//! The Source-Control panel emits a [`ProjectDiffRequest`] whose
//! [`DiffSource`] decides where the base text comes from:
//!
//! * [`DiffSource::WorkingTree`] — per file the *staged* (index↔HEAD) and
//!   *unstaged* (worktree↔index) patches are merged into one section, each hunk
//!   tagged with its stage state; hunks can be staged, unstaged and restored.
//! * [`DiffSource::Commit`] / [`DiffSource::Stash`] / [`DiffSource::Branch`] —
//!   one whole patch, split per file, rendered read-only.
//!
//! Presentation data (ordering, numbering, word diff, split alignment, hunk
//! navigation, patch building) lives in `labonair_editor::diff_document`; this
//! file only loads, paints and routes actions.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, list, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, Hsla,
    InteractiveElement, IntoElement, ListAlignment, ListOffset, ListSizingBehavior, ListState,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_editor::diff_document::{
    DiffDocument, DiffLine, DiffStyle, FileInput, FileKind, FlatRow, HunkId, HunkOperation,
    HunkStage, HunkView, LineKind,
};
use labonair_editor::unified::{parse_diff_hunks, FileDiff};
use labonair_git::GitService;
use labonair_notifications::{notification_center, notify_err, Notification};
use labonair_panel::{DiffSource, ProjectDiffFile, ProjectDiffRequest};
use labonair_settings::{
    enqueue_user_settings_write, EditorSettings, Settings as _, SettingsStore,
};
use labonair_ui_kit::{
    button, button_disabled, checkbox, file_icon_path, icon_button_builder, svg_path, ButtonSize,
    ButtonVariant, IconName, Palette, Tooltip,
};
use tokio::runtime::Handle as TokioHandle;

use crate::theme::ThemeStore;

type OpenFileHook = Rc<dyn Fn(String, &mut Window, &mut App)>;
type CommitHook = Rc<dyn Fn(&mut Window, &mut App)>;
type ToolbarAction = Box<dyn Fn(&mut ProjectDiffView, &mut Context<ProjectDiffView>)>;

/// Height of one code row.
const LINE_H: f32 = 20.0;
/// Height of the hunk separator row (carries the hunk controls).
const HUNK_H: f32 = 24.0;
/// Height of a file header block.
const FILE_H: f32 = 32.0;
/// Width of one line-number column.
const NUM_W: f32 = 44.0;
/// Fallback for `editorMinimumSplitDiffWidth` when settings are unavailable.
const DEFAULT_MIN_SPLIT_COLUMNS: f32 = 100.0;

/// Resolve which file a [`ProjectDiffRequest`] should focus: the requested path
/// if it is in the set, otherwise the first file (so a bare "View Diff" always
/// lands somewhere). Pure — unit-tested below.
pub fn resolve_selection(files: &[ProjectDiffFile], requested: Option<&str>) -> Option<String> {
    requested
        .filter(|p| files.iter().any(|f| f.path == *p))
        .map(str::to_string)
        .or_else(|| files.first().map(|f| f.path.clone()))
}

#[derive(Clone, Copy)]
struct Colors {
    bg: Hsla,
    fg: Hsla,
    muted: Hsla,
    border: Hsla,
    card: Hsla,
    accent: Hsla,
    success: Hsla,
    error: Hsla,
    info: Hsla,
}

/// Everything a row needs to paint, shared by reference across the frame.
struct RowCtx {
    c: Colors,
    palette: Palette,
    doc: Rc<DiffDocument>,
    rows: Rc<Vec<FlatRow>>,
    folded: HashSet<String>,
    cursor: Option<HunkId>,
    confirm_restore: Option<HunkId>,
    busy: bool,
    word_diff: bool,
    can_stage: bool,
    icon_theme: labonair_theme::IconThemeContent,
    view: Entity<ProjectDiffView>,
}

pub struct ProjectDiffView {
    theme: Entity<ThemeStore>,
    git: Arc<dyn GitService>,
    tokio: TokioHandle,
    focus: FocusHandle,

    repo_root: Option<String>,
    session_id: Option<String>,
    source: DiffSource,
    files: Vec<ProjectDiffFile>,
    /// The user's chosen layout; see [`Self::effective_style`].
    style: DiffStyle,
    /// `true` while the container is narrower than the split minimum.
    too_narrow_for_split: bool,
    last_width: Option<f32>,
    last_em: f32,
    /// `editorMinimumSplitDiffWidth`, in editor columns (0 = never suspend).
    min_split_columns: f32,

    doc: Rc<DiffDocument>,
    rows: Rc<Vec<FlatRow>>,
    list: ListState,
    folded: HashSet<String>,
    /// Paths already seen, so only a file's *first* appearance auto-folds it.
    known_files: HashSet<String>,
    cursor: Option<HunkId>,
    confirm_restore: Option<HunkId>,
    pending_scroll: Option<String>,

    loading: bool,
    busy: bool,
    /// Generation guard so a slow response for a previous load cannot
    /// overwrite a newer one.
    gen: u64,

    on_open_file: Option<OpenFileHook>,
    on_commit: Option<CommitHook>,
}

impl ProjectDiffView {
    pub fn new(
        theme: Entity<ThemeStore>,
        git: Arc<dyn GitService>,
        tokio: TokioHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        if cx.has_global::<SettingsStore>() {
            cx.observe_global::<SettingsStore>(|this, cx| this.sync_settings(cx))
                .detach();
        }
        let mut view = Self {
            theme,
            git,
            tokio,
            focus: cx.focus_handle(),
            repo_root: None,
            session_id: None,
            source: DiffSource::WorkingTree,
            files: Vec::new(),
            style: DiffStyle::Unified,
            too_narrow_for_split: false,
            last_width: None,
            last_em: 7.2,
            min_split_columns: DEFAULT_MIN_SPLIT_COLUMNS,
            doc: Rc::new(DiffDocument::default()),
            rows: Rc::new(Vec::new()),
            list: ListState::new(0, ListAlignment::Top, px(600.0)),
            folded: HashSet::new(),
            known_files: HashSet::new(),
            cursor: None,
            confirm_restore: None,
            pending_scroll: None,
            loading: false,
            busy: false,
            gen: 0,
            on_open_file: None,
            on_commit: None,
        };
        view.sync_settings(cx);
        view
    }

    /// Follow `editorDiffViewStyle` and `editorMinimumSplitDiffWidth`.
    fn sync_settings(&mut self, cx: &mut Context<Self>) {
        let Some(settings) = EditorSettings::try_get(cx) else {
            return;
        };
        let style = if settings.diff_view_split() {
            DiffStyle::Split
        } else {
            DiffStyle::Unified
        };
        let min_columns = settings.minimum_split_diff_width() as f32;
        if style == self.style && min_columns == self.min_split_columns {
            return;
        }
        self.style = style;
        self.min_split_columns = min_columns;
        self.refresh_narrowness();
        self.rebuild_rows();
        cx.notify();
    }

    fn refresh_narrowness(&mut self) {
        self.too_narrow_for_split = self.last_width.is_some_and(|w| {
            self.min_split_columns > 0.0 && w < self.min_split_columns * self.last_em
        });
    }

    /// Hook for the header's file name / Open File button (absolute path).
    pub fn set_on_open_file(&mut self, f: impl Fn(String, &mut Window, &mut App) + 'static) {
        self.on_open_file = Some(Rc::new(f));
    }

    /// Hook for the toolbar's Commit button.
    pub fn set_on_commit(&mut self, f: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_commit = Some(Rc::new(f));
    }

    /// Point the item at a (possibly new) review. Repeated requests for the
    /// same repository and source reload in place and re-point the scroll.
    pub fn apply_request(&mut self, req: ProjectDiffRequest, cx: &mut Context<Self>) {
        let repository_changed = !same_repository_context(
            self.repo_root.as_deref(),
            self.session_id.as_deref(),
            &req.repo_root,
            req.session_id.as_deref(),
        );
        let same_source = !repository_changed && same_source(&self.source, &req.source);
        self.repo_root = Some(req.repo_root);
        self.session_id = req.session_id;
        self.pending_scroll = match &req.source {
            DiffSource::WorkingTree => resolve_selection(&req.files, req.selected.as_deref()),
            _ => req.selected.clone(),
        };
        if !same_source {
            self.known_files.clear();
            self.folded.clear();
            self.cursor = None;
        }
        self.source = req.source;
        self.files = req.files;
        self.reload(cx);
    }

    fn effective_style(&self) -> DiffStyle {
        if self.style == DiffStyle::Split && self.too_narrow_for_split {
            DiffStyle::Unified
        } else {
            self.style
        }
    }

    fn supports_staging(&self) -> bool {
        self.source.supports_staging()
    }

    /// Fetch every file's patches and rebuild the document.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.gen += 1;
        let generation = self.gen;
        self.loading = true;
        let session = self.session_id.clone();
        let git = self.git.clone();
        let source = self.source.clone();
        let files = self.files.clone();
        let tokio = self.tokio.clone();
        let task = self.tokio.spawn(async move {
            match source {
                DiffSource::WorkingTree => {
                    load_working_tree(git, tokio, root, session, files).await
                }
                DiffSource::Commit { hash, .. } => {
                    let text = git.commit_diff(root, hash, session).await?;
                    Ok(read_only_inputs(&text))
                }
                DiffSource::Stash { hash, .. } => {
                    let text = git.stash_diff(root, hash, session).await?;
                    Ok(read_only_inputs(&text))
                }
                DiffSource::Branch { base, head, .. } => {
                    let text = git.branch_diff(root, base, head, session).await?;
                    Ok(read_only_inputs(&text))
                }
            }
        });
        cx.spawn(async move |this, cx| {
            let res = task.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation {
                    return;
                }
                this.loading = false;
                match res {
                    Ok(inputs) => this.set_document(DiffDocument::build(inputs)),
                    Err(e) => {
                        this.set_document(DiffDocument::default());
                        notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Diff load failed",
                                    "Could not load the changes for review.",
                                )
                                .source("project-diff")
                                .details(e)
                                .dedupe_key("project-diff:load".to_string()),
                                cx,
                            );
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn set_document(&mut self, doc: DiffDocument) {
        for section in &doc.files {
            // Spec §2: only a file's first appearance auto-folds it.
            if self.known_files.insert(section.path.clone()) && section.kind == FileKind::Deleted {
                self.folded.insert(section.path.clone());
            }
        }
        self.doc = Rc::new(doc);
        let ids = self.doc.hunk_ids();
        self.cursor = self
            .cursor
            .filter(|id| ids.contains(id))
            .or_else(|| ids.first().copied());
        self.confirm_restore = None;
        self.rebuild_rows();
        if let Some(path) = self.pending_scroll.take() {
            self.scroll_to_file(&path);
        }
    }

    /// Re-flatten the document for the current fold set and style, keeping the
    /// scroll position.
    fn rebuild_rows(&mut self) {
        let top = self.list.logical_scroll_top();
        self.rows = Rc::new(self.doc.flatten(self.effective_style(), &self.folded));
        self.list.reset(self.rows.len());
        self.list.scroll_to(top);
    }

    fn scroll_to_file(&mut self, path: &str) {
        let Some(file) = self.doc.file_index(path) else {
            return;
        };
        if let Some(ix) = self.rows.iter().position(|r| *r == FlatRow::File(file)) {
            self.list.scroll_to(ListOffset {
                item_ix: ix,
                offset_in_item: px(0.0),
            });
        }
    }

    fn reveal_hunk(&mut self, id: HunkId) {
        if let Some(path) = self.doc.files.get(id.file).map(|f| f.path.clone()) {
            self.folded.remove(&path);
        }
        self.rebuild_rows();
        if let Some(ix) = self.rows.iter().position(|r| *r == FlatRow::HunkHeader(id)) {
            self.list.scroll_to_reveal_item(ix);
        }
    }

    fn set_cursor(&mut self, id: HunkId, cx: &mut Context<Self>) {
        if self.cursor != Some(id) || self.confirm_restore.is_some() {
            self.cursor = Some(id);
            self.confirm_restore = None;
            cx.notify();
        }
    }

    fn toggle_fold(&mut self, path: &str, all: bool, cx: &mut Context<Self>) {
        if all {
            // Fold everything unless everything is already folded.
            if self.folded.len() < self.doc.files.len() {
                self.folded = self.doc.files.iter().map(|f| f.path.clone()).collect();
            } else {
                self.folded.clear();
            }
        } else if !self.folded.remove(path) {
            self.folded.insert(path.to_string());
        }
        self.rebuild_rows();
        cx.notify();
    }

    fn set_style(&mut self, style: DiffStyle, cx: &mut Context<Self>) {
        if self.style == style {
            return;
        }
        self.style = style;
        self.rebuild_rows();
        self.queue_style_setting(style, cx);
        cx.notify();
    }

    /// Persist the chosen layout so every DiffView (and the next session)
    /// follows it.
    fn queue_style_setting(&mut self, style: DiffStyle, cx: &mut Context<Self>) {
        if !cx.has_global::<SettingsStore>() {
            return;
        }
        let value = match style {
            DiffStyle::Split => "split",
            DiffStyle::Unified => "unified",
        };
        let Ok(Some(request)) = cx
            .global_mut::<SettingsStore>()
            .update_user_settings_deferred(move |c| {
                c.editor.editor_diff_view_style = Some(value.to_string());
            })
        else {
            return;
        };
        let _ = enqueue_user_settings_write(request);
    }

    /// Spec §6: below the minimum width a Split style renders unified; the
    /// style itself is untouched, so widening restores the split.
    fn width_changed(&mut self, width: f32, em: f32, cx: &mut Context<Self>) {
        self.last_width = Some(width);
        self.last_em = em;
        let was_narrow = self.too_narrow_for_split;
        self.refresh_narrowness();
        if was_narrow != self.too_narrow_for_split {
            if self.style == DiffStyle::Split {
                self.rebuild_rows();
            }
            cx.notify();
        }
    }

    fn go_to_hunk(&mut self, forward: bool, cx: &mut Context<Self>) {
        if let Some(id) = self.doc.step_hunk(self.cursor, forward) {
            self.cursor = Some(id);
            self.confirm_restore = None;
            self.reveal_hunk(id);
            cx.notify();
        }
    }

    /// Run one git mutation, then reload so stage tags and counts refresh.
    fn run_git(
        &mut self,
        failure: &'static str,
        cx: &mut Context<Self>,
        op: impl FnOnce(Arc<dyn GitService>, String, Option<String>) -> labonair_git::GitFuture<()>
            + Send
            + 'static,
    ) {
        if self.busy {
            return;
        }
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.busy = true;
        cx.notify();
        let fut = op(self.git.clone(), root, self.session_id.clone());
        let jh = self.tokio.spawn(fut);
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                notify_err(failure, res, cx);
                this.reload(cx);
            });
        })
        .detach();
    }

    fn apply_hunk(&mut self, id: HunkId, cx: &mut Context<Self>) {
        let Some(op) = self.doc.hunk_operation(id) else {
            return;
        };
        match op {
            HunkOperation::WholeFile { path, stage } => {
                let unstage = stage == HunkStage::Staged;
                self.run_git(stage_failure(unstage), cx, move |git, root, session| {
                    if unstage {
                        git.unstage_file(root, path, session)
                    } else {
                        git.stage_file(root, path, session)
                    }
                });
            }
            HunkOperation::Patch { path, patch, stage } => {
                let unstage = stage == HunkStage::Staged;
                self.run_git(stage_failure(unstage), cx, move |git, root, session| {
                    if unstage {
                        git.unstage_hunk(root, path, patch, session)
                    } else {
                        git.stage_hunk(root, path, patch, session)
                    }
                });
            }
        }
    }

    /// Stage / unstage the cursor hunk and move on to the next one
    /// (`StageAndNext` / `UnstageAndNext`).
    fn apply_cursor_and_next(&mut self, want: HunkStage, cx: &mut Context<Self>) {
        let Some(id) = self.cursor else {
            return;
        };
        if self.doc.hunk(id).map(|h| h.stage) != Some(want) {
            return;
        }
        let next = self.doc.step_hunk(Some(id), true);
        self.apply_hunk(id, cx);
        if let Some(next) = next {
            self.cursor = Some(next);
            self.reveal_hunk(next);
        }
    }

    /// Restore (discard) one hunk. Two-step — the first click arms it — because
    /// the diff has no undo.
    fn restore_hunk(&mut self, id: HunkId, cx: &mut Context<Self>) {
        if self.confirm_restore != Some(id) {
            self.cursor = Some(id);
            self.confirm_restore = Some(id);
            cx.notify();
            return;
        }
        self.confirm_restore = None;
        let Some(HunkOperation::Patch { path, patch, stage }) = self.doc.hunk_operation(id) else {
            // A whole-file hunk restores through the file-level discard.
            if let Some(HunkOperation::WholeFile { path, .. }) = self.doc.hunk_operation(id) {
                self.run_git("Restore failed", cx, move |git, root, session| {
                    git.discard_file(root, path, session)
                });
            }
            return;
        };
        let staged = stage == HunkStage::Staged;
        self.run_git("Restore hunk failed", cx, move |git, root, session| {
            git.discard_hunk(root, path, patch, staged, session)
        });
    }

    fn stage_file(&mut self, path: String, stage: bool, cx: &mut Context<Self>) {
        self.run_git(stage_failure(!stage), cx, move |git, root, session| {
            if stage {
                git.stage_file(root, path, session)
            } else {
                git.unstage_file(root, path, session)
            }
        });
    }

    fn stage_all(&mut self, stage: bool, cx: &mut Context<Self>) {
        self.run_git(
            if stage {
                "Stage all failed"
            } else {
                "Unstage all failed"
            },
            cx,
            move |git, root, session| {
                if stage {
                    git.stage_all(root, session)
                } else {
                    git.unstage_all(root, session)
                }
            },
        );
    }

    fn colors(&self, cx: &App) -> Colors {
        let t = self.theme.read(cx);
        Colors {
            bg: t.background(),
            fg: t.foreground(),
            muted: t.muted_foreground(),
            border: t.border(),
            card: t.card(),
            accent: t.accent(),
            success: t.status_success(),
            error: t.status_error(),
            info: t.status_info(),
        }
    }

    fn header_title(&self) -> String {
        match &self.source {
            DiffSource::Commit { hash, subject } | DiffSource::Stash { hash, subject } => {
                let short: String = hash.chars().take(7).collect();
                if subject.is_empty() {
                    short
                } else {
                    format!("{short}  {subject}")
                }
            }
            DiffSource::Branch { subject, .. } => subject.clone(),
            DiffSource::WorkingTree => "Uncommitted Changes".to_string(),
        }
    }

    fn empty_label(&self) -> &'static str {
        match self.source {
            DiffSource::WorkingTree => "No uncommitted changes",
            _ => "No changes",
        }
    }

    fn absolute_path(&self, relative: &str) -> Option<String> {
        let root = self.repo_root.as_deref()?;
        Some(format!("{}/{}", root.trim_end_matches('/'), relative))
    }
}

fn stage_failure(unstage: bool) -> &'static str {
    if unstage {
        "Unstage failed"
    } else {
        "Stage failed"
    }
}

fn same_repository_context(
    left_root: Option<&str>,
    left_session: Option<&str>,
    right_root: &str,
    right_session: Option<&str>,
) -> bool {
    left_root == Some(right_root) && left_session == right_session
}

/// Whether two requests review the same thing (a re-request then keeps fold
/// state and cursor instead of resetting them).
fn same_source(left: &DiffSource, right: &DiffSource) -> bool {
    match (left, right) {
        (DiffSource::WorkingTree, DiffSource::WorkingTree) => true,
        (DiffSource::Commit { hash: l, .. }, DiffSource::Commit { hash: r, .. })
        | (DiffSource::Stash { hash: l, .. }, DiffSource::Stash { hash: r, .. }) => l == r,
        (
            DiffSource::Branch {
                base: lb, head: lh, ..
            },
            DiffSource::Branch {
                base: rb, head: rh, ..
            },
        ) => lb == rb && lh == rh,
        _ => false,
    }
}

/// Splits a whole read-only patch into per-file inputs.
fn read_only_inputs(text: &str) -> Vec<FileInput> {
    parse_diff_hunks(text)
        .into_iter()
        .map(|diff: FileDiff| FileInput {
            path: diff.path.clone(),
            untracked: false,
            diffs: vec![(HunkStage::ReadOnly, diff)],
            raw: Vec::new(),
        })
        .collect()
}

/// Fetches the staged / unstaged / untracked patch of every requested file
/// concurrently and groups them per path.
async fn load_working_tree(
    git: Arc<dyn GitService>,
    tokio: TokioHandle,
    root: String,
    session: Option<String>,
    files: Vec<ProjectDiffFile>,
) -> Result<Vec<FileInput>, String> {
    let mut jobs = Vec::new();
    for file in files {
        let (git, root, session) = (git.clone(), root.clone(), session.clone());
        jobs.push((
            file.clone(),
            tokio.spawn(async move {
                git.diff(
                    root,
                    file.path,
                    file.staged,
                    Some(false),
                    Some(file.untracked),
                    session,
                )
                .await
            }),
        ));
    }
    let mut inputs: Vec<FileInput> = Vec::new();
    let mut first_error = None;
    for (file, job) in jobs {
        let text = match job.await.unwrap_or_else(|e| Err(e.to_string())) {
            Ok(text) => text,
            Err(e) => {
                first_error.get_or_insert(e);
                continue;
            }
        };
        let stage = if file.staged {
            HunkStage::Staged
        } else {
            HunkStage::Unstaged
        };
        let idx = match inputs.iter().position(|i| i.path == file.path) {
            Some(idx) => idx,
            None => {
                inputs.push(FileInput {
                    path: file.path.clone(),
                    untracked: file.untracked,
                    diffs: Vec::new(),
                    raw: Vec::new(),
                });
                inputs.len() - 1
            }
        };
        match parse_diff_hunks(&text).into_iter().next() {
            Some(diff) if !diff.hunks.is_empty() => inputs[idx].diffs.push((stage, diff)),
            _ => inputs[idx]
                .raw
                .extend(text.lines().take(20_000).map(str::to_string)),
        }
    }
    match (inputs.is_empty(), first_error) {
        (true, Some(e)) => Err(e),
        _ => Ok(inputs),
    }
}

impl Focusable for ProjectDiffView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ProjectDiffView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span = tracing::trace_span!(
            target: "labonair::perf",
            "render",
            view = "project_diff",
            files = self.doc.files.len()
        )
        .entered();
        let c = self.colors(cx);
        let theme = self.theme.read(cx);
        let font = theme.buffer_font();
        let palette = Palette::from_theme(theme);
        let icon_theme = theme.icon_theme().clone();
        let view = cx.entity();
        let word_diff = EditorSettings::try_get(cx)
            .map(|settings| settings.git_word_diff())
            .unwrap_or(true);

        let body: gpui::AnyElement = if self.rows.is_empty() {
            self.render_empty(c, palette).into_any_element()
        } else {
            let ctx = Rc::new(RowCtx {
                c,
                palette,
                doc: self.doc.clone(),
                rows: self.rows.clone(),
                folded: self.folded.clone(),
                cursor: self.cursor,
                confirm_restore: self.confirm_restore,
                busy: self.busy,
                word_diff,
                can_stage: self.supports_staging(),
                icon_theme,
                view: view.clone(),
            });
            list(self.list.clone(), move |ix, _window, cx| {
                render_row(&ctx, ix, cx)
            })
            .with_sizing_behavior(ListSizingBehavior::Auto)
            .size_full()
            .font(font.clone())
            .text_size(px(12.0))
            .into_any_element()
        };

        // Width probe: the split suspends below `min_split_columns` columns.
        let probe_view = view.clone();
        let last_width = self.last_width;
        let probe = canvas(
            move |bounds, window, cx| {
                let width: f32 = bounds.size.width.into();
                if last_width == Some(width) {
                    return;
                }
                let font_size = px(12.0);
                let text_system = window.text_system().clone();
                let font_id = text_system.resolve_font(&font);
                let em: f32 = text_system
                    .em_advance(font_id, font_size)
                    .unwrap_or(font_size)
                    .into();
                let view = probe_view.clone();
                window.defer(cx, move |_window, cx| {
                    view.update(cx, |this, cx| this.width_changed(width, em, cx));
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        div()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .child(self.render_toolbar(c, palette, cx))
            .child(div().relative().flex_1().min_h_0().child(body).child(probe))
    }
}

impl ProjectDiffView {
    fn render_empty(&self, c: Colors, palette: Palette) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .text_size(px(13.0))
            .text_color(c.muted)
            .when(self.loading, |d| {
                d.child(IconName::LoadCircle.svg(palette.fg).size(px(20.0)))
                    .child(SharedString::from("Loading changes\u{2026}"))
            })
            .when(!self.loading, |d| {
                d.child(SharedString::from(self.empty_label()))
            })
    }

    fn render_toolbar(&self, c: Colors, palette: Palette, cx: &mut Context<Self>) -> gpui::Div {
        let (added, removed) = self.doc.totals();
        let has_hunks = !self.doc.hunk_ids().is_empty();
        let can_stage = self.supports_staging();
        let cursor_stage = self
            .cursor
            .and_then(|id| self.doc.hunk(id))
            .map(|h| h.stage);
        let has_unstaged = self.doc.has_stage(HunkStage::Unstaged);
        let has_staged = self.doc.has_stage(HunkStage::Staged);
        // Spec §7: "Unstage All" replaces "Stage All" once everything is staged.
        let show_unstage_all = has_staged && !has_unstaged;
        let style = self.style;
        let suspended = style == DiffStyle::Split && self.too_narrow_for_split;

        let sep = || div().w(px(1.0)).h(px(16.0)).bg(c.border).mx(px(2.0));
        let text_button = |id: &'static str,
                           label: &'static str,
                           tip: &'static str,
                           enabled: bool,
                           on_click: ToolbarAction| {
            if enabled {
                button(id, palette, ButtonVariant::Outlined, ButtonSize::Xs)
                    .child(SharedString::from(label))
                    .tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| on_click(this, cx)))
                    .into_any_element()
            } else {
                button_disabled(id, palette, ButtonVariant::Outlined, ButtonSize::Xs)
                    .child(SharedString::from(label))
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_none()
            .flex_wrap()
            .items_center()
            .gap(px(6.0))
            .px(px(10.0))
            .py(px(4.0))
            .min_h(px(36.0))
            .border_b_1()
            .border_color(c.border)
            .bg(c.card)
            .text_size(px(12.0))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(SharedString::from(self.header_title())),
            )
            .when(!can_stage, |d| {
                d.child(
                    div()
                        .px(px(6.0))
                        .text_size(px(10.0))
                        .text_color(c.muted)
                        .child(SharedString::from("read-only")),
                )
            })
            .when(!self.doc.is_empty(), |d| {
                d.child(diff_stat(added, removed, c)).child(sep())
            })
            .child(
                icon_button_builder("pd-prev-hunk", palette, IconName::ArrowUp)
                    .variant(ButtonVariant::Subtle)
                    .size(ButtonSize::Xs)
                    .disabled(!has_hunks)
                    .tooltip("Go to Previous Hunk")
                    .render()
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.go_to_hunk(false, cx)),
                    ),
            )
            .child(
                icon_button_builder("pd-next-hunk", palette, IconName::ArrowDown)
                    .variant(ButtonVariant::Subtle)
                    .size(ButtonSize::Xs)
                    .disabled(!has_hunks)
                    .tooltip("Go to Next Hunk")
                    .render()
                    .on_click(
                        cx.listener(|this, _: &ClickEvent, _w, cx| this.go_to_hunk(true, cx)),
                    ),
            )
            .when(can_stage, |d| {
                d.child(sep())
                    .child(text_button(
                        "pd-stage",
                        "Stage",
                        "Stage and Go to Next Hunk",
                        cursor_stage == Some(HunkStage::Unstaged) && !self.busy,
                        Box::new(|this, cx| this.apply_cursor_and_next(HunkStage::Unstaged, cx)),
                    ))
                    .child(text_button(
                        "pd-unstage",
                        "Unstage",
                        "Unstage and Go to Next Hunk",
                        cursor_stage == Some(HunkStage::Staged) && !self.busy,
                        Box::new(|this, cx| this.apply_cursor_and_next(HunkStage::Staged, cx)),
                    ))
                    .child(sep())
                    .child(if show_unstage_all {
                        text_button(
                            "pd-unstage-all",
                            "Unstage All",
                            "Unstage All Changes",
                            !self.busy,
                            Box::new(|this, cx| this.stage_all(false, cx)),
                        )
                    } else {
                        text_button(
                            "pd-stage-all",
                            "Stage All",
                            "Stage All Changes",
                            has_unstaged && !self.busy,
                            Box::new(|this, cx| this.stage_all(true, cx)),
                        )
                    })
                    .child(sep())
                    .child(
                        button(
                            "pd-commit",
                            palette,
                            ButtonVariant::Outlined,
                            ButtonSize::Xs,
                        )
                        .child(SharedString::from("Commit"))
                        .tooltip(|window, cx| Tooltip::new("Commit").build(window, cx))
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                if let Some(f) = this.on_commit.clone() {
                                    f(window, cx);
                                }
                            },
                        )),
                    )
            })
            .child(sep())
            .child(
                icon_button_builder("pd-style-unified", palette, IconName::DiffUnified)
                    .variant(ButtonVariant::Subtle)
                    .size(ButtonSize::Xs)
                    .selected(style == DiffStyle::Unified)
                    .tooltip("Unified")
                    .render()
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.set_style(DiffStyle::Unified, cx)
                    })),
            )
            .child(
                icon_button_builder(
                    "pd-style-split",
                    palette,
                    if suspended {
                        IconName::DiffSplitAuto
                    } else {
                        IconName::DiffSplit
                    },
                )
                .variant(ButtonVariant::Subtle)
                .size(ButtonSize::Xs)
                .selected(style == DiffStyle::Split)
                .tooltip(if suspended {
                    format!("Split when wider than {} columns", self.min_split_columns)
                } else {
                    "Split".to_string()
                })
                .render()
                .on_click(
                    cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.set_style(DiffStyle::Split, cx)
                    }),
                ),
            )
    }
}

fn diff_stat(added: u32, removed: u32, c: Colors) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .text_color(c.success)
                .child(SharedString::from(format!("+{added}"))),
        )
        .child(
            div()
                .text_color(c.error)
                .child(SharedString::from(format!("\u{2212}{removed}"))),
        )
}

fn render_row(ctx: &Rc<RowCtx>, ix: usize, cx: &mut App) -> gpui::AnyElement {
    let Some(row) = ctx.rows.get(ix) else {
        return div().into_any_element();
    };
    match row {
        FlatRow::File(file) => render_file_header(ctx, *file, ix, cx),
        FlatRow::HunkHeader(id) => render_hunk_header(ctx, *id, ix),
        FlatRow::Line { id, line } => {
            let Some(hunk) = ctx.doc.hunk(*id) else {
                return div().into_any_element();
            };
            let Some(l) = hunk.lines.get(*line) else {
                return div().into_any_element();
            };
            let tint = line_tint(ctx, hunk, l);
            let view = ctx.view.clone();
            let id = *id;
            div()
                .id(("pd-line", ix))
                .flex()
                .h(px(LINE_H))
                .w_full()
                .when_some(tint, |d, (bg, _)| d.bg(bg))
                .child(num_cell(l.old_no, ctx.c))
                .child(num_cell(l.new_no, ctx.c))
                .child(marker_bar(l.kind, hunk.stage, ctx.c))
                .child(code_text(ctx, l))
                .on_click(move |_: &ClickEvent, _w, cx| {
                    view.update(cx, |this, cx| this.set_cursor(id, cx));
                })
                .into_any_element()
        }
        FlatRow::Split { id, left, right } => {
            let Some(hunk) = ctx.doc.hunk(*id) else {
                return div().into_any_element();
            };
            let view = ctx.view.clone();
            let id = *id;
            div()
                .id(("pd-split", ix))
                .flex()
                .h(px(LINE_H))
                .w_full()
                .child(split_cell(
                    ctx,
                    hunk,
                    left.and_then(|i| hunk.lines.get(i)),
                    true,
                ))
                .child(div().w(px(1.0)).h_full().bg(ctx.c.border))
                .child(split_cell(
                    ctx,
                    hunk,
                    right.and_then(|i| hunk.lines.get(i)),
                    false,
                ))
                .on_click(move |_: &ClickEvent, _w, cx| {
                    view.update(cx, |this, cx| this.set_cursor(id, cx));
                })
                .into_any_element()
        }
        FlatRow::Raw { file, line } => {
            let text = ctx
                .doc
                .files
                .get(*file)
                .and_then(|f| f.raw.get(*line))
                .cloned()
                .unwrap_or_default();
            let color = match text.chars().next() {
                Some('+') => ctx.c.success,
                Some('-') => ctx.c.error,
                _ => ctx.c.fg,
            };
            div()
                .h(px(LINE_H))
                .px(px(8.0))
                .whitespace_nowrap()
                .text_color(color)
                .child(SharedString::from(text))
                .into_any_element()
        }
    }
}

/// `(row background, gutter-bar alpha)` for a changed line, or `None` for
/// context. Staged hunks are hollow (spec §4, `staged_hollow`).
fn line_tint(ctx: &RowCtx, hunk: &HunkView, line: &DiffLine) -> Option<(Hsla, f32)> {
    let base = match line.kind {
        LineKind::Added => ctx.c.success,
        LineKind::Deleted => ctx.c.error,
        LineKind::Context => return None,
    };
    let hollow = hunk.stage == HunkStage::Staged;
    Some(if hollow {
        (base.opacity(0.05), 0.55)
    } else {
        (base.opacity(0.14), 1.0)
    })
}

fn num_cell(n: Option<u32>, c: Colors) -> gpui::Div {
    div()
        .w(px(NUM_W))
        .flex_none()
        .pr(px(8.0))
        .flex()
        .justify_end()
        .text_color(c.muted)
        .child(SharedString::from(
            n.map(|n| n.to_string()).unwrap_or_default(),
        ))
}

fn marker_bar(kind: LineKind, stage: HunkStage, c: Colors) -> gpui::Div {
    let color = match kind {
        LineKind::Added => Some(c.success),
        LineKind::Deleted => Some(c.error),
        LineKind::Context => None,
    };
    let alpha = if stage == HunkStage::Staged {
        0.55
    } else {
        1.0
    };
    div()
        .w(px(3.0))
        .h_full()
        .flex_none()
        .mr(px(8.0))
        .when_some(color, |d, color| d.bg(color.opacity(alpha)))
}

/// Line text with the matched word range emphasized.
fn code_text(ctx: &RowCtx, line: &DiffLine) -> gpui::Div {
    let text = line.text.replace('\t', "    ");
    let tint = match line.kind {
        LineKind::Added => ctx.c.success,
        LineKind::Deleted => ctx.c.error,
        LineKind::Context => ctx.c.fg,
    };
    let base = div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .whitespace_nowrap();
    // Tabs were widened, so the byte range only applies to tab-free text.
    let range = line
        .word
        .clone()
        .filter(|r| ctx.word_diff && !line.text.contains('\t') && r.end <= text.len());
    match range {
        Some(range) => base
            .flex()
            .child(SharedString::from(text[..range.start].to_string()))
            .child(
                div()
                    .bg(tint.opacity(0.35))
                    .child(SharedString::from(text[range.clone()].to_string())),
            )
            .child(SharedString::from(text[range.end..].to_string())),
        None => base.child(SharedString::from(text)),
    }
}

fn split_cell(ctx: &RowCtx, hunk: &HunkView, line: Option<&DiffLine>, left: bool) -> gpui::Div {
    let cell = div().flex().flex_1().min_w_0().h_full().overflow_hidden();
    let Some(line) = line else {
        // Filler row: keeps both sides aligned.
        return cell.bg(ctx.c.muted.opacity(0.06));
    };
    let number = if left { line.old_no } else { line.new_no };
    cell.when_some(line_tint(ctx, hunk, line), |d, (bg, _)| d.bg(bg))
        .child(num_cell(number, ctx.c))
        .child(marker_bar(line.kind, hunk.stage, ctx.c))
        .child(code_text(ctx, line))
}

fn render_hunk_header(ctx: &Rc<RowCtx>, id: HunkId, ix: usize) -> gpui::AnyElement {
    let Some(hunk) = ctx.doc.hunk(id) else {
        return div().into_any_element();
    };
    let c = ctx.c;
    let is_cursor = ctx.cursor == Some(id);
    let group = SharedString::from(format!("pd-hunk-{ix}"));
    let alpha = if ctx.busy { 0.66 } else { 1.0 };
    let armed = ctx.confirm_restore == Some(id);
    let (view_a, view_b, view_c) = (ctx.view.clone(), ctx.view.clone(), ctx.view.clone());
    let can_act = ctx.can_stage && hunk.stage != HunkStage::ReadOnly;

    let controls = div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .opacity(alpha)
        .when(!is_cursor, |d| d.invisible())
        .group_hover(group.clone(), |d| d.visible())
        .when(can_act, |d| {
            let staged = hunk.stage == HunkStage::Staged;
            d.child(
                button(
                    ("pd-hunk-stage", ix),
                    ctx.palette,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child(SharedString::from(if staged { "Unstage" } else { "Stage" }))
                .tooltip(move |window, cx| {
                    Tooltip::new(if staged { "Unstage Hunk" } else { "Stage Hunk" })
                        .build(window, cx)
                })
                .on_click(move |_: &ClickEvent, _w, cx| {
                    view_a.update(cx, |this, cx| this.apply_hunk(id, cx));
                }),
            )
            .child(
                button(
                    ("pd-hunk-restore", ix),
                    ctx.palette,
                    if armed {
                        ButtonVariant::TintedError
                    } else {
                        ButtonVariant::Outlined
                    },
                    ButtonSize::Xs,
                )
                .child(SharedString::from(if armed {
                    "Confirm Restore"
                } else {
                    "Restore"
                }))
                .tooltip(|window, cx| Tooltip::new("Restore Hunk").build(window, cx))
                .on_click(move |_: &ClickEvent, _w, cx| {
                    view_b.update(cx, |this, cx| this.restore_hunk(id, cx));
                }),
            )
        });

    div()
        .id(("pd-hunk", ix))
        .group(group)
        .flex()
        .items_center()
        .justify_between()
        .h(px(HUNK_H))
        .w_full()
        .px(px(8.0))
        .text_size(px(11.0))
        .text_color(c.muted)
        .bg(if is_cursor {
            c.accent
        } else {
            c.info.opacity(0.07)
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .child(SharedString::from(hunk.header.clone()))
                .when(hunk.stage == HunkStage::Staged, |d| {
                    d.child(
                        div()
                            .text_color(c.success)
                            .child(SharedString::from("staged")),
                    )
                }),
        )
        .child(controls)
        .on_click(move |_: &ClickEvent, _w, cx| {
            view_c.update(cx, |this, cx| this.set_cursor(id, cx));
        })
        .into_any_element()
}

fn render_file_header(
    ctx: &Rc<RowCtx>,
    file_ix: usize,
    ix: usize,
    _cx: &mut App,
) -> gpui::AnyElement {
    let Some(file) = ctx.doc.files.get(file_ix) else {
        return div().into_any_element();
    };
    let c = ctx.c;
    let folded = ctx.folded.contains(&file.path);
    let (dir, name) = match file.path.rsplit_once('/') {
        Some((dir, name)) => (Some(format!("{dir}/")), name.to_string()),
        None => (None, file.path.clone()),
    };
    let name_color = match file.kind {
        FileKind::Added | FileKind::Untracked => c.success,
        FileKind::Deleted => c.error,
        FileKind::Modified => c.fg,
    };
    let all_staged =
        !file.hunks.is_empty() && file.hunks.iter().all(|h| h.stage == HunkStage::Staged);
    let icon = file_icon_path(&ctx.icon_theme, &name);
    let path = file.path.clone();
    let (fold_view, check_view, open_view) = (ctx.view.clone(), ctx.view.clone(), ctx.view.clone());
    let (fold_path, check_path, open_path) = (path.clone(), path.clone(), path.clone());
    let chevron = if folded {
        IconName::ChevronRight
    } else {
        IconName::ChevronDown
    };

    div()
        .w_full()
        .pt(px(if file_ix == 0 { 0.0 } else { 8.0 }))
        .child(
            div()
                .id(("pd-file", ix))
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(FILE_H))
                .px(px(6.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(c.border)
                .bg(c.card)
                .hover(|s| s.bg(c.accent.opacity(0.5)))
                .child(
                    icon_button_builder(("pd-fold", ix), ctx.palette, chevron)
                        .variant(ButtonVariant::Transparent)
                        .size(ButtonSize::Xs)
                        .tooltip(if folded {
                            "Unfold Excerpt"
                        } else {
                            "Fold Excerpt (\u{2325} click toggles all)"
                        })
                        .render()
                        .on_click(move |event: &ClickEvent, _w, cx| {
                            let all = event.modifiers().alt;
                            fold_view.update(cx, |this, cx| this.toggle_fold(&fold_path, all, cx));
                        }),
                )
                .when(ctx.can_stage && !file.hunks.is_empty(), |d| {
                    d.child(
                        checkbox(("pd-stage-file", ix), ctx.palette, all_staged)
                            .disabled(ctx.busy)
                            .on_click(move |checked: &bool, _w, cx| {
                                let stage = *checked;
                                check_view.update(cx, |this, cx| {
                                    this.stage_file(check_path.clone(), stage, cx)
                                });
                            }),
                    )
                })
                .child(svg_path(icon, c.muted))
                .child(
                    div()
                        .id(("pd-file-name", ix))
                        .cursor_pointer()
                        .whitespace_nowrap()
                        .text_color(name_color)
                        .when(file.kind == FileKind::Deleted, |d| d.line_through())
                        .child(SharedString::from(name))
                        .on_click(move |_: &ClickEvent, window, cx| {
                            let path = open_path.clone();
                            let handler = open_view.update(cx, |this, _| {
                                this.absolute_path(&path).zip(this.on_open_file.clone())
                            });
                            if let Some((abs, f)) = handler {
                                f(abs, window, cx);
                            }
                        }),
                )
                .when_some(dir, |d, dir| {
                    d.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_color(c.muted)
                            .child(SharedString::from(dir)),
                    )
                })
                .when(file.added > 0 || file.removed > 0, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .child(diff_stat(file.added, file.removed, c)),
                    )
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str) -> ProjectDiffFile {
        ProjectDiffFile {
            path: path.into(),
            staged: false,
            untracked: false,
        }
    }

    #[test]
    fn resolve_selection_prefers_requested_then_first() {
        let files = vec![f("a.rs"), f("b.rs"), f("c.rs")];
        assert_eq!(resolve_selection(&files, None).as_deref(), Some("a.rs"));
        assert_eq!(
            resolve_selection(&files, Some("b.rs")).as_deref(),
            Some("b.rs")
        );
        assert_eq!(
            resolve_selection(&files, Some("zzz")).as_deref(),
            Some("a.rs")
        );
        assert_eq!(resolve_selection(&[], Some("a.rs")), None);
    }

    #[test]
    fn source_identity_includes_kind_and_hash() {
        let commit = DiffSource::Commit {
            hash: "abc".into(),
            subject: "subject changed in place".into(),
        };
        let same_commit = DiffSource::Commit {
            hash: "abc".into(),
            subject: "updated subject".into(),
        };
        let other_commit = DiffSource::Commit {
            hash: "def".into(),
            subject: "subject".into(),
        };
        let stash = DiffSource::Stash {
            hash: "abc".into(),
            subject: "Stash".into(),
        };
        let branch = DiffSource::Branch {
            base: "main".into(),
            head: "feature".into(),
            subject: "main → feature".into(),
        };
        let same_branch = DiffSource::Branch {
            base: "main".into(),
            head: "feature".into(),
            subject: "updated subject".into(),
        };

        assert!(same_source(&commit, &same_commit));
        assert!(!same_source(&commit, &other_commit));
        assert!(!same_source(&commit, &stash));
        assert!(same_source(&stash, &stash));
        assert!(same_source(&branch, &same_branch));
        assert!(!same_source(&branch, &commit));
        assert!(same_source(
            &DiffSource::WorkingTree,
            &DiffSource::WorkingTree
        ));
    }

    #[test]
    fn repository_context_identity_includes_root_and_remote_session() {
        assert!(same_repository_context(
            Some("/work/project"),
            Some("ssh-session"),
            "/work/project",
            Some("ssh-session"),
        ));
        assert!(!same_repository_context(
            Some("/work/project"),
            Some("ssh-session"),
            "/work/other",
            Some("ssh-session"),
        ));
        assert!(!same_repository_context(
            Some("/work/project"),
            Some("ssh-session"),
            "/work/project",
            Some("other-session"),
        ));
    }

    #[test]
    fn read_only_patch_splits_into_one_input_per_file() {
        let patch = [
            "diff --git a/a.rs b/a.rs",
            "--- a/a.rs",
            "+++ b/a.rs",
            "@@ -1 +1 @@",
            "-x",
            "+y",
            "diff --git a/b.rs b/b.rs",
            "--- a/b.rs",
            "+++ b/b.rs",
            "@@ -1 +1 @@",
            "-p",
            "+q",
            "",
        ]
        .join("\n");
        let inputs = read_only_inputs(&patch);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].path, "a.rs");
        assert_eq!(inputs[0].diffs[0].0, HunkStage::ReadOnly);
    }
}

#[cfg(test)]
mod render_tests;
