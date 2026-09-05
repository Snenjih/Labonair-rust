//! Workspace-level Project Diff item (Zed-parity redesign Phase 4,
//! `docs/ui-comparison-zed-sidebar-status-bar.md` §9.5 / §12.6).
//!
//! Replaces the Source-Control panel's inline 280 px diff viewer. The panel
//! emits a [`ProjectDiffRequest`]; the workspace opens/focuses exactly one of
//! these views. It lists the changed files in a compact rail, renders the
//! selected file's `git diff` as unified or side-by-side hunks, and stages /
//! unstages individual hunks straight through the backend (`git apply
//! --cached`). Repeated requests re-point the selection instead of duplicating.

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::git;
use labonair_backend::App as Backend;
use labonair_editor::unified::{
    build_hunk_patch, is_whole_file_single_hunk, parse_diff_hunks, DiffHunk,
};
use labonair_notifications::notify_err;
use labonair_panel::{ProjectDiffFile, ProjectDiffMode, ProjectDiffRequest};
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

pub struct ProjectDiffView {
    theme: Entity<ThemeStore>,
    backend: Backend,
    tokio: TokioHandle,
    focus: FocusHandle,

    repo_root: Option<String>,
    session_id: Option<String>,
    files: Vec<ProjectDiffFile>,
    selected: Option<String>,
    mode: ProjectDiffMode,

    /// Loaded `git diff` text for `selected`, plus a generation guard so a slow
    /// response for a previous selection cannot overwrite a newer one.
    diff_text: Option<String>,
    diff_error: Option<String>,
    gen: u64,
    op_in_progress: bool,
}

impl ProjectDiffView {
    pub fn new(
        theme: Entity<ThemeStore>,
        backend: Backend,
        tokio: TokioHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            backend,
            tokio,
            focus: cx.focus_handle(),
            repo_root: None,
            session_id: None,
            files: Vec::new(),
            selected: None,
            mode: ProjectDiffMode::Unified,
            diff_text: None,
            diff_error: None,
            gen: 0,
            op_in_progress: false,
        }
    }

    /// Point the item at a (possibly new) review set. Idempotent — repeated
    /// calls for the same repo just re-point the selection.
    pub fn apply_request(&mut self, req: ProjectDiffRequest, cx: &mut Context<Self>) {
        self.repo_root = Some(req.repo_root);
        self.session_id = req.session_id;
        self.files = req.files;
        self.mode = req.mode;

        let want = resolve_selection(&self.files, req.selected.as_deref());
        if want != self.selected {
            self.selected = want;
            self.reload(cx);
        }
        cx.notify();
    }

    fn select(&mut self, path: String, cx: &mut Context<Self>) {
        if self.selected.as_deref() == Some(path.as_str()) {
            return;
        }
        self.selected = Some(path);
        self.reload(cx);
        cx.notify();
    }

    fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = match self.mode {
            ProjectDiffMode::Unified => ProjectDiffMode::Split,
            ProjectDiffMode::Split => ProjectDiffMode::Unified,
        };
        cx.notify();
    }

    fn current_file(&self) -> Option<&ProjectDiffFile> {
        let sel = self.selected.as_deref()?;
        self.files.iter().find(|f| f.path == sel)
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let (Some(root), Some(file)) = (self.repo_root.clone(), self.current_file().cloned())
        else {
            self.diff_text = None;
            return;
        };
        self.diff_text = None;
        self.diff_error = None;
        self.gen += 1;
        let generation = self.gen;
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let jh = self.tokio.spawn(async move {
            git::git_get_diff(
                root,
                file.path,
                file.staged,
                Some(false),
                Some(file.untracked),
                session,
                &backend.ssh,
                backend.clone(),
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
                        this.diff_error = None;
                    }
                    Err(e) => {
                        this.diff_text = None;
                        this.diff_error = Some(e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Stage (or, if `reverse`, unstage) one hunk of the loaded diff.
    fn apply_hunk(&mut self, hunk_idx: usize, reverse: bool, cx: &mut Context<Self>) {
        if self.op_in_progress {
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
        let backend = self.backend.clone();
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
                (true, false, _) => {
                    git::git_stage_file(root, path, session, &backend.ssh, backend.clone()).await
                }
                (true, true, _) => {
                    git::git_unstage_file(root, path, session, &backend.ssh, backend.clone()).await
                }
                (false, false, Some(p)) => {
                    git::git_stage_hunk(root, path, p, session, &backend.ssh, backend.clone()).await
                }
                (false, true, Some(p)) => {
                    git::git_unstage_hunk(root, path, p, session, &backend.ssh, backend.clone())
                        .await
                }
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
            .child(SharedString::from(
                selected
                    .clone()
                    .unwrap_or_else(|| "Project Diff".to_string()),
            ))
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

        let mut body = div()
            .id("project-diff-body")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_scroll()
            .font(font)
            .text_size(px(12.0));

        if let Some(err) = &self.diff_error {
            body = body.child(
                div()
                    .p(px(10.0))
                    .text_color(c.error)
                    .child(SharedString::from(err.clone())),
            );
        } else if let Some(text) = &self.diff_text {
            let untracked = self.current_file().map(|f| f.untracked).unwrap_or(false);
            if untracked || !text.contains("@@ ") {
                for line in text.lines().take(2000) {
                    body = body.child(diff_line(line, c));
                }
            } else {
                let parsed = parse_diff_hunks(text);
                if let Some(file) = parsed.first() {
                    let whole = is_whole_file_single_hunk(file);
                    let reverse = self.current_file().map(|f| f.staged).unwrap_or(false);
                    for (i, hunk) in file.hunks.iter().enumerate() {
                        body = body.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .px(px(8.0))
                                .bg(c.info.opacity(0.10))
                                .text_color(c.info)
                                .child(SharedString::from(hunk.header.clone()))
                                .when(!whole, |d| {
                                    d.child(
                                        div()
                                            .id(SharedString::from(format!("pd-hunk-{i}")))
                                            .px(px(6.0))
                                            .rounded_sm()
                                            .text_color(c.muted)
                                            .hover(|s| s.text_color(c.fg))
                                            .child(SharedString::from(if reverse {
                                                "Unstage hunk"
                                            } else {
                                                "Stage hunk"
                                            }))
                                            .on_click(cx.listener(
                                                move |this, _: &ClickEvent, _w, cx| {
                                                    this.apply_hunk(i, reverse, cx);
                                                },
                                            )),
                                    )
                                }),
                        );
                        if is_split {
                            for row in split_hunk_rows(&hunk.lines, c) {
                                body = body.child(row);
                            }
                        } else {
                            for l in &hunk.lines {
                                body = body.child(diff_line(l, c));
                            }
                        }
                    }
                }
            }
        } else if self.selected.is_some() {
            body = body.child(
                div()
                    .p(px(10.0))
                    .text_color(c.muted)
                    .child(SharedString::from("Loading diff\u{2026}")),
            );
        } else {
            body = body.child(
                div()
                    .p(px(10.0))
                    .text_color(c.muted)
                    .child(SharedString::from("No changes to review")),
            );
        }

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

fn diff_line(line: &str, c: Colors) -> impl IntoElement {
    let color = match line.chars().next() {
        Some('+') => c.success,
        Some('-') => c.error,
        _ => c.fg,
    };
    div()
        .px(px(8.0))
        .whitespace_nowrap()
        .text_color(color)
        .child(SharedString::from(if line.is_empty() {
            " ".to_string()
        } else {
            line.to_string()
        }))
}

/// Side-by-side rows for one hunk's body lines (old left, new right).
fn split_hunk_rows(lines: &[String], c: Colors) -> Vec<gpui::AnyElement> {
    let cell = |text: &str, color: gpui::Hsla, tint: Option<gpui::Hsla>| {
        let mut d = div()
            .flex_1()
            .min_w_0()
            .px(px(8.0))
            .whitespace_nowrap()
            .overflow_hidden()
            .text_color(color);
        if let Some(t) = tint {
            d = d.bg(t.opacity(0.10));
        }
        d.child(SharedString::from(if text.is_empty() {
            " ".to_string()
        } else {
            text.to_string()
        }))
    };
    let row = |left: gpui::AnyElement, right: gpui::AnyElement| {
        div()
            .flex()
            .gap(px(1.0))
            .child(left)
            .child(right)
            .into_any_element()
    };
    let mut out: Vec<gpui::AnyElement> = Vec::new();
    let mut dels: Vec<&str> = Vec::new();
    let mut adds: Vec<&str> = Vec::new();
    let flush = |out: &mut Vec<gpui::AnyElement>, dels: &mut Vec<&str>, adds: &mut Vec<&str>| {
        let n = dels.len().max(adds.len());
        for i in 0..n {
            let l = dels
                .get(i)
                .map(|s| cell(s, c.error, Some(c.error)).into_any_element())
                .unwrap_or_else(|| cell("", c.fg, None).into_any_element());
            let r = adds
                .get(i)
                .map(|s| cell(s, c.success, Some(c.success)).into_any_element())
                .unwrap_or_else(|| cell("", c.fg, None).into_any_element());
            out.push(row(l, r));
        }
        dels.clear();
        adds.clear();
    };
    for line in lines {
        match line.chars().next() {
            Some('-') => dels.push(line.get(1..).unwrap_or("")),
            Some('+') => adds.push(line.get(1..).unwrap_or("")),
            _ => {
                flush(&mut out, &mut dels, &mut adds);
                let text = line.strip_prefix(' ').unwrap_or(line);
                out.push(row(
                    cell(text, c.fg, None).into_any_element(),
                    cell(text, c.fg, None).into_any_element(),
                ));
            }
        }
    }
    flush(&mut out, &mut dels, &mut adds);
    out
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
