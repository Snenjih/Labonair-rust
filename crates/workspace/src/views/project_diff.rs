//! Workspace-level Diff item (Zed-parity redesign Phase 4,
//! `docs/ui-comparison-zed-sidebar-status-bar.md` §9.5 / §12.6, extended in the
//! Phase-1 diff-surface consolidation).
//!
//! One workspace Diff surface renders every kind of change: the Source-Control
//! panel emits a [`ProjectDiffRequest`] whose [`DiffSource`] decides where the
//! base text comes from —
//!
//! * [`DiffSource::WorkingTree`] — uncommitted work. The request's `files` list
//!   drives a compact rail; each file's `git diff` is fetched on demand and its
//!   hunks can be staged / unstaged individually (`git apply --cached`).
//! * [`DiffSource::Commit`] — a single committed change, fetched once as a whole
//!   patch (`git show`), split per file for the rail, rendered **read-only**.
//!
//! The hunk body is virtualised ([`uniform_list`]) so large diffs stay cheap.
//! Repeated `WorkingTree` requests re-point the selection instead of
//! duplicating; a `Commit` request re-targets the same item at the new commit.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, uniform_list, App, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_editor::unified::{
    build_hunk_patch, is_whole_file_single_hunk, parse_diff_hunks, DiffHunk, FileDiff,
};
use labonair_git::GitService;
use labonair_notifications::{notification_center, notify_err, Notification};
use labonair_panel::{DiffSource, ProjectDiffFile, ProjectDiffMode, ProjectDiffRequest};
use tokio::runtime::Handle as TokioHandle;

use crate::theme::ThemeStore;

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
    bg: gpui::Hsla,
    fg: gpui::Hsla,
    muted: gpui::Hsla,
    border: gpui::Hsla,
    card: gpui::Hsla,
    accent: gpui::Hsla,
    success: gpui::Hsla,
    error: gpui::Hsla,
    info: gpui::Hsla,
}

/// One virtualised row of the diff body.
#[derive(Clone)]
enum DiffRow {
    HunkHeader {
        text: SharedString,
        /// Index into the selected file's `hunks`, for hunk staging.
        hunk_idx: usize,
        /// `false` for whole-file (new / deleted) diffs — no per-hunk control.
        per_hunk: bool,
    },
    Unified(SharedString),
    Split {
        /// `(text, is_deletion)` — `is_deletion` tints the cell.
        left: Option<(SharedString, bool)>,
        /// `(text, is_addition)`.
        right: Option<(SharedString, bool)>,
    },
    /// Untracked file / non-`@@` text — shown verbatim.
    Raw(SharedString),
}

pub struct ProjectDiffView {
    theme: Entity<ThemeStore>,
    git: std::sync::Arc<dyn GitService>,
    tokio: TokioHandle,
    focus: FocusHandle,

    repo_root: Option<String>,
    session_id: Option<String>,
    source: DiffSource,
    files: Vec<ProjectDiffFile>,
    selected: Option<String>,
    mode: ProjectDiffMode,

    /// `WorkingTree`: loaded `git diff` text for `selected`. `Commit`: unused.
    diff_text: Option<String>,
    /// `Commit`: the whole commit patch, parsed once per file.
    commit_files: Vec<FileDiff>,
    /// Flattened body of the selected file, rebuilt on select / mode / reload.
    rows: Vec<DiffRow>,
    /// Generation guard so a slow response for a previous selection / commit
    /// cannot overwrite a newer one.
    gen: u64,
    op_in_progress: bool,
}

impl ProjectDiffView {
    pub fn new(
        theme: Entity<ThemeStore>,
        git: std::sync::Arc<dyn GitService>,
        tokio: TokioHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            git,
            tokio,
            focus: cx.focus_handle(),
            repo_root: None,
            session_id: None,
            source: DiffSource::WorkingTree,
            files: Vec::new(),
            selected: None,
            mode: ProjectDiffMode::Unified,
            diff_text: None,
            commit_files: Vec::new(),
            rows: Vec::new(),
            gen: 0,
            op_in_progress: false,
        }
    }

    /// Point the item at a (possibly new) review. Idempotent for the working
    /// tree — repeated calls just re-point the selection.
    pub fn apply_request(&mut self, req: ProjectDiffRequest, cx: &mut Context<Self>) {
        self.repo_root = Some(req.repo_root);
        self.session_id = req.session_id;
        self.mode = req.mode;

        match req.source.clone() {
            DiffSource::WorkingTree => {
                let source_changed = self.source != DiffSource::WorkingTree;
                self.source = DiffSource::WorkingTree;
                self.commit_files.clear();
                self.files = req.files;
                let want = resolve_selection(&self.files, req.selected.as_deref());
                if want != self.selected || source_changed {
                    self.selected = want;
                    self.reload(cx);
                }
            }
            DiffSource::Commit { hash, .. } => {
                let same_commit = matches!(
                    &self.source,
                    DiffSource::Commit { hash: h, .. } if *h == hash
                );
                self.source = req.source;
                if !same_commit {
                    self.load_commit(hash, cx);
                } else if let Some(sel) = req.selected {
                    self.selected = Some(sel);
                    self.rebuild_rows();
                }
            }
        }
        cx.notify();
    }

    fn select(&mut self, path: String, cx: &mut Context<Self>) {
        if self.selected.as_deref() == Some(path.as_str()) {
            return;
        }
        self.selected = Some(path);
        match self.source {
            DiffSource::WorkingTree => self.reload(cx),
            DiffSource::Commit { .. } => self.rebuild_rows(),
        }
        cx.notify();
    }

    fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = match self.mode {
            ProjectDiffMode::Unified => ProjectDiffMode::Split,
            ProjectDiffMode::Split => ProjectDiffMode::Unified,
        };
        self.rebuild_rows();
        cx.notify();
    }

    fn current_file(&self) -> Option<&ProjectDiffFile> {
        let sel = self.selected.as_deref()?;
        self.files.iter().find(|f| f.path == sel)
    }

    /// `WorkingTree`: fetch the selected file's `git diff`.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let (Some(root), Some(file)) = (self.repo_root.clone(), self.current_file().cloned())
        else {
            self.diff_text = None;
            self.rows.clear();
            return;
        };
        self.diff_text = None;
        self.rows.clear();
        self.gen += 1;
        let generation = self.gen;
        let session = self.session_id.clone();
        let git = self.git.clone();
        let jh = self.tokio.spawn(async move {
            git.diff(
                root,
                file.path,
                file.staged,
                Some(false),
                Some(file.untracked),
                session,
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation {
                    return;
                }
                match res {
                    Ok(text) => {
                        this.diff_text = Some(text);
                        this.rebuild_rows();
                    }
                    Err(e) => {
                        this.diff_text = None;
                        this.rows.clear();
                        let path = this.selected.clone().unwrap_or_default();
                        notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Diff load failed",
                                    "Could not load the selected file diff.",
                                )
                                .source("project-diff")
                                .details(e.clone())
                                .dedupe_key(format!("project-diff:load:{path}")),
                                cx,
                            );
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// `Commit`: fetch the whole commit patch once, split per file for the rail.
    fn load_commit(&mut self, hash: String, cx: &mut Context<Self>) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.diff_text = None;
        self.commit_files.clear();
        self.files.clear();
        self.selected = None;
        self.rows.clear();
        self.gen += 1;
        let generation = self.gen;
        let session = self.session_id.clone();
        let git = self.git.clone();
        let jh = self
            .tokio
            .spawn(async move { git.commit_diff(root, hash, session).await });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.gen != generation {
                    return;
                }
                match res {
                    Ok(text) => {
                        let parsed = parse_diff_hunks(&text);
                        this.files = parsed
                            .iter()
                            .map(|f| ProjectDiffFile {
                                path: f.path.clone(),
                                staged: false,
                                untracked: false,
                            })
                            .collect();
                        this.commit_files = parsed;
                        this.selected = this.files.first().map(|f| f.path.clone());
                        this.rebuild_rows();
                    }
                    Err(e) => {
                        this.commit_files.clear();
                        this.files.clear();
                        this.rows.clear();
                        notification_center(cx).update(cx, |center, cx| {
                            center.push(
                                Notification::error(
                                    "Commit diff load failed",
                                    "Could not load the selected commit.",
                                )
                                .source("project-diff")
                                .details(e.clone())
                                .dedupe_key("project-diff:commit".to_string()),
                                cx,
                            );
                        });
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Rebuild [`Self::rows`] for the current selection + layout from whichever
    /// source is active. Pure re-derivation — no IO, no theme access.
    fn rebuild_rows(&mut self) {
        let is_split = matches!(self.mode, ProjectDiffMode::Split);
        let mut rows: Vec<DiffRow> = Vec::new();

        match &self.source {
            DiffSource::WorkingTree => {
                let Some(text) = &self.diff_text else {
                    self.rows = rows;
                    return;
                };
                let untracked = self.current_file().map(|f| f.untracked).unwrap_or(false);
                if untracked || !text.contains("@@ ") {
                    for line in text.lines().take(20_000) {
                        rows.push(DiffRow::Raw(SharedString::from(line.to_string())));
                    }
                    self.rows = rows;
                    return;
                }
                let parsed = parse_diff_hunks(text);
                if let Some(file) = parsed.first() {
                    let per_hunk = !is_whole_file_single_hunk(file);
                    push_file_rows(&mut rows, file, is_split, per_hunk);
                }
            }
            DiffSource::Commit { .. } => {
                let selected_file = self
                    .selected
                    .as_deref()
                    .and_then(|s| self.commit_files.iter().find(|f| f.path == s));
                if let Some(file) = selected_file {
                    // Committed changes are read-only — never a per-hunk control.
                    push_file_rows(&mut rows, file, is_split, false);
                }
            }
        }
        self.rows = rows;
    }

    /// Stage (or, if `reverse`, unstage) one hunk of the loaded diff.
    /// Working-tree source only.
    fn apply_hunk(&mut self, hunk_idx: usize, reverse: bool, cx: &mut Context<Self>) {
        if self.op_in_progress || !self.source.supports_staging() {
            return;
        }
        let (Some(root), Some(file), Some(diff)) = (
            self.repo_root.clone(),
            self.current_file().cloned(),
            self.diff_text.clone(),
        ) else {
            return;
        };
        let session = self.session_id.clone();
        let git = self.git.clone();
        let files = parse_diff_hunks(&diff);
        let Some(parsed) = files.into_iter().next() else {
            notify_err::<()>(
                "Hunk staging unavailable",
                Err("Diff could not be parsed for hunk staging.".to_string()),
                cx,
            );
            return;
        };
        let path = file.path.clone();
        let whole = is_whole_file_single_hunk(&parsed);
        let patch = if whole {
            None
        } else {
            let Some(hunk): Option<&DiffHunk> = parsed.hunks.get(hunk_idx) else {
                return;
            };
            Some(build_hunk_patch(&parsed, hunk))
        };
        self.op_in_progress = true;
        cx.notify();
        let jh = self.tokio.spawn(async move {
            match (whole, reverse, patch) {
                (true, false, _) => git.stage_file(root, path, session).await,
                (true, true, _) => git.unstage_file(root, path, session).await,
                (false, false, Some(p)) => git.stage_hunk(root, path, p, session).await,
                (false, true, Some(p)) => git.unstage_hunk(root, path, p, session).await,
                _ => Ok(()),
            }
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.op_in_progress = false;
                notify_err(
                    if reverse {
                        "Unstage hunk failed"
                    } else {
                        "Stage hunk failed"
                    },
                    res,
                    cx,
                );
                this.reload(cx);
                cx.notify();
            });
        })
        .detach();
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
            DiffSource::Commit { hash, subject } => {
                let short: String = hash.chars().take(7).collect();
                if subject.is_empty() {
                    short
                } else {
                    format!("{short}  {subject}")
                }
            }
            DiffSource::WorkingTree => self
                .selected
                .clone()
                .unwrap_or_else(|| "Project Diff".to_string()),
        }
    }
}

fn push_file_rows(rows: &mut Vec<DiffRow>, file: &FileDiff, is_split: bool, per_hunk: bool) {
    for (i, hunk) in file.hunks.iter().enumerate() {
        rows.push(DiffRow::HunkHeader {
            text: SharedString::from(hunk.header.clone()),
            hunk_idx: i,
            per_hunk,
        });
        if is_split {
            split_hunk_rows_into(rows, &hunk.lines);
        } else {
            for l in &hunk.lines {
                rows.push(DiffRow::Unified(SharedString::from(if l.is_empty() {
                    " ".to_string()
                } else {
                    l.clone()
                })));
            }
        }
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
            files = self.files.len()
        )
        .entered();
        let c = self.colors(cx);
        let font = self.theme.read(cx).buffer_font();
        let is_split = matches!(self.mode, ProjectDiffMode::Split);
        let read_only = !self.source.supports_staging();
        let reverse = self.current_file().map(|f| f.staged).unwrap_or(false);
        let selected = self.selected.clone();
        let view = cx.entity();

        // Compact file rail.
        let mut rail = div()
            .id("project-diff-rail")
            .flex()
            .flex_col()
            .w(px(220.0))
            .flex_none()
            .overflow_y_scroll()
            .border_r_1()
            .border_color(c.border)
            .bg(c.card)
            .text_size(px(12.0));
        for f in &self.files {
            let is_sel = selected.as_deref() == Some(f.path.as_str());
            let p = f.path.clone();
            let v = view.clone();
            rail = rail.child(
                div()
                    .id(SharedString::from(format!("pd-file-{}", f.path)))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .h(px(24.0))
                    .px(px(8.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .when(is_sel, |d| d.bg(c.accent))
                    .when(!is_sel, |d| d.hover(|s| s.bg(c.accent.opacity(0.4))))
                    .child(
                        div()
                            .w(px(10.0))
                            .flex_none()
                            .text_size(px(10.0))
                            .text_color(if f.staged { c.success } else { c.muted })
                            .child(SharedString::from(if f.staged { "\u{25CF}" } else { "" })),
                    )
                    .child(div().flex_1().overflow_hidden().child(SharedString::from(
                        f.path.rsplit('/').next().unwrap_or(&f.path).to_string(),
                    )))
                    .on_click(move |_: &ClickEvent, _w, cx| {
                        v.update(cx, |this, cx| this.select(p.clone(), cx));
                    }),
            );
        }

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(28.0))
            .px(px(10.0))
            .border_b_1()
            .border_color(c.border)
            .bg(c.card)
            .text_size(px(12.0))
            .text_color(c.fg)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(SharedString::from(self.header_title())),
            )
            .when(read_only, |d| {
                d.child(
                    div()
                        .px(px(6.0))
                        .text_size(px(10.0))
                        .text_color(c.muted)
                        .child(SharedString::from("read-only")),
                )
            })
            .child(
                div()
                    .id("project-diff-mode")
                    .px(px(6.0))
                    .py(px(1.0))
                    .rounded_sm()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from(if is_split {
                        "Unified"
                    } else {
                        "Split"
                    }))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.toggle_mode(cx))),
            );

        let body: gpui::AnyElement = if self.rows.is_empty() {
            let msg = if self.selected.is_some() {
                "Loading diff\u{2026}"
            } else {
                "No changes to review"
            };
            div()
                .id("project-diff-body")
                .flex_1()
                .p(px(10.0))
                .text_color(c.muted)
                .font(font)
                .text_size(px(12.0))
                .child(SharedString::from(msg))
                .into_any_element()
        } else {
            let rows = self.rows.clone();
            let list_view = view.clone();
            uniform_list("project-diff-body", rows.len(), move |range, _win, _cx| {
                range
                    .map(|i| {
                        diff_row_element(&rows[i], c, is_split, read_only, reverse, &list_view)
                    })
                    .collect::<Vec<_>>()
            })
            .flex_1()
            .font(font)
            .text_size(px(12.0))
            .into_any_element()
        };

        div()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .child(header)
            .child(div().flex().flex_1().min_h_0().child(rail).child(body))
    }
}

fn diff_row_element(
    row: &DiffRow,
    c: Colors,
    _is_split: bool,
    read_only: bool,
    reverse: bool,
    view: &Entity<ProjectDiffView>,
) -> gpui::AnyElement {
    match row {
        DiffRow::HunkHeader {
            text,
            hunk_idx,
            per_hunk,
        } => {
            let show_action = *per_hunk && !read_only;
            let idx = *hunk_idx;
            let v = view.clone();
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(8.0))
                .bg(c.info.opacity(0.10))
                .text_color(c.info)
                .child(text.clone())
                .when(show_action, |d| {
                    d.child(
                        div()
                            .id(SharedString::from(format!("pd-hunk-{idx}")))
                            .px(px(6.0))
                            .rounded_sm()
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
                            .child(SharedString::from(if reverse {
                                "Unstage hunk"
                            } else {
                                "Stage hunk"
                            }))
                            .on_click(move |_: &ClickEvent, _w, cx| {
                                v.update(cx, |this, cx| this.apply_hunk(idx, reverse, cx));
                            }),
                    )
                })
                .into_any_element()
        }
        DiffRow::Unified(text) => {
            let color = match text.chars().next() {
                Some('+') => c.success,
                Some('-') => c.error,
                _ => c.fg,
            };
            div()
                .px(px(8.0))
                .whitespace_nowrap()
                .text_color(color)
                .child(text.clone())
                .into_any_element()
        }
        DiffRow::Raw(text) => {
            let color = match text.chars().next() {
                Some('+') => c.success,
                Some('-') => c.error,
                _ => c.fg,
            };
            div()
                .px(px(8.0))
                .whitespace_nowrap()
                .text_color(color)
                .child(text.clone())
                .into_any_element()
        }
        DiffRow::Split { left, right } => {
            let cell = |slot: &Option<(SharedString, bool)>, tint: gpui::Hsla| {
                let (text, active) = match slot {
                    Some((t, active)) => (t.clone(), *active),
                    None => (SharedString::from(" "), false),
                };
                let mut d = div()
                    .flex_1()
                    .min_w_0()
                    .px(px(8.0))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_color(if active { tint } else { c.fg });
                if active {
                    d = d.bg(tint.opacity(0.10));
                }
                d.child(text)
            };
            div()
                .flex()
                .gap(px(1.0))
                .child(cell(left, c.error))
                .child(cell(right, c.success))
                .into_any_element()
        }
    }
}

/// Flatten one hunk's body lines into side-by-side [`DiffRow::Split`] rows
/// (old left, new right).
fn split_hunk_rows_into(out: &mut Vec<DiffRow>, lines: &[String]) {
    let mut dels: Vec<String> = Vec::new();
    let mut adds: Vec<String> = Vec::new();
    let flush = |out: &mut Vec<DiffRow>, dels: &mut Vec<String>, adds: &mut Vec<String>| {
        let n = dels.len().max(adds.len());
        for i in 0..n {
            out.push(DiffRow::Split {
                left: dels.get(i).map(|s| (SharedString::from(s.clone()), true)),
                right: adds.get(i).map(|s| (SharedString::from(s.clone()), true)),
            });
        }
        dels.clear();
        adds.clear();
    };
    for line in lines {
        match line.chars().next() {
            Some('-') => dels.push(line.get(1..).unwrap_or("").to_string()),
            Some('+') => adds.push(line.get(1..).unwrap_or("").to_string()),
            _ => {
                flush(out, &mut dels, &mut adds);
                let text = line.strip_prefix(' ').unwrap_or(line).to_string();
                out.push(DiffRow::Split {
                    left: Some((SharedString::from(text.clone()), false)),
                    right: Some((SharedString::from(text), false)),
                });
            }
        }
    }
    flush(out, &mut dels, &mut adds);
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
        // bare request → first file
        assert_eq!(resolve_selection(&files, None).as_deref(), Some("a.rs"));
        // explicit, in-set → that file (a follow-up "focus this file" request)
        assert_eq!(
            resolve_selection(&files, Some("b.rs")).as_deref(),
            Some("b.rs")
        );
        // explicit, not in set → falls back to first, never nothing
        assert_eq!(
            resolve_selection(&files, Some("zzz")).as_deref(),
            Some("a.rs")
        );
        // empty change set → nothing
        assert_eq!(resolve_selection(&[], Some("a.rs")), None);
    }
}
