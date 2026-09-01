//! Source-Control panel — Git status & staging (T09-001).
//!
//! [`GitPanelView`] is the GPUI-native port of the reference web app's
//! `src/modules/source-control/` sidebar panel. It polls the backend's
//! batched [`git_get_workspace_state`] for the repo at the active working
//! directory, renders changed files categorised into
//! Conflicts / Staged / Unstaged / Untracked, lets the user stage/unstage
//! whole files, whole sections and individual hunks, shows a unified-diff
//! preview for the selected file, and drives commit / pull / push / fetch.
//!
//! The Git logic itself lives entirely in `labonair_backend::modules::git`
//! (a `git` CLI wrapper — no `libgit2`). This view only wires it up: every
//! backend call is dispatched onto the tokio runtime and its result folded
//! back into the view on the GPUI thread.
//!
//! Polling uses a *generation guard* (`target_gen`, bumped only on a genuine
//! repo/session change) plus an in-flight flag so a slow response for a
//! previous target can never overwrite a newer target's state — mirrors
//! `useGitStatus.ts`'s `generationRef` / `isRefreshingRef`.

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window,
};
use labonair_backend::modules::git::{self, FileStatus, GitStatus, WorkspaceGitState};
use labonair_backend::App as Backend;
use tokio::runtime::Handle as TokioHandle;

use crate::notifications::notify_err;
use crate::theme::ThemeStore;

/// Local git status poll interval (matches the reference default
/// `gitStatusPollIntervalMs`). Remote targets back off (see
/// [`GitPanelView::poll_interval`]).
const POLL_INTERVAL: Duration = Duration::from_millis(2000);
const REMOTE_POLL_MULTIPLIER: u32 = 3;

// ─── Hunk parsing (port of source-control/lib/diffHunks.ts) ───────────────────

/// One `@@ … @@` block of a unified diff, body lines kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    /// The raw `@@ -a,b +c,d @@ …` line including trailing context.
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// Raw body lines (` `/`+`/`-`/`\ No newline…`), unmodified, in order.
    pub lines: Vec<String>,
}

/// Per-file view of a (possibly multi-file) unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    /// b-side path (post-rename for renames).
    pub path: String,
    /// Lines from `diff --git …` up to (excluding) the first hunk header.
    pub header_lines: Vec<String>,
    pub hunks: Vec<DiffHunk>,
    pub is_new_file: bool,
    pub is_deleted_file: bool,
}

fn parse_file_header_path(line: &str) -> Option<String> {
    // ^diff --git a/.+ b/(.+)$
    let rest = line.strip_prefix("diff --git a/")?;
    let idx = rest.find(" b/")?;
    Some(rest[idx + 3..].to_string())
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    // ^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@
    let rest = line.strip_prefix("@@ -")?;
    let end = rest.find(" @@")?;
    let spec = &rest[..end];
    let mut sides = spec.split(" +");
    let old = sides.next()?;
    let new = sides.next()?;
    let parse_pair = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (os, ol) = parse_pair(old)?;
    let (ns, nl) = parse_pair(new)?;
    Some((os, ol, ns, nl))
}

/// Parses a unified diff into per-file hunk structures. Returns `[]` for a
/// backend-truncated diff (a cut-off final hunk could corrupt the index).
pub fn parse_diff_hunks(diff: &str) -> Vec<FileDiff> {
    if diff.is_empty() || diff.contains("[diff truncated") || diff.contains("[diff too large]") {
        return Vec::new();
    }

    let all: Vec<&str> = diff.split('\n').collect();
    // Split into chunks at each "diff --git a/… b/…" line.
    let mut starts: Vec<usize> = Vec::new();
    for (i, l) in all.iter().enumerate() {
        if l.starts_with("diff --git a/") && l.contains(" b/") {
            starts.push(i);
        }
    }
    let mut files = Vec::new();
    for (si, &start) in starts.iter().enumerate() {
        let end = starts.get(si + 1).copied().unwrap_or(all.len());
        let mut lines: Vec<String> = all[start..end].iter().map(|s| s.to_string()).collect();
        // Drop the single trailing "" produced by the terminating "\n".
        if lines.last().map(|s| s.is_empty()).unwrap_or(false) {
            lines.pop();
        }
        let Some(path) = lines.first().and_then(|l| parse_file_header_path(l)) else {
            continue;
        };
        let first_hunk = lines.iter().position(|l| parse_hunk_header(l).is_some());
        let header_lines: Vec<String> = match first_hunk {
            Some(idx) => lines[..idx].to_vec(),
            None => lines.clone(),
        };
        let is_new_file = header_lines.iter().any(|l| l.starts_with("new file mode"));
        let is_deleted_file = header_lines
            .iter()
            .any(|l| l.starts_with("deleted file mode"));

        let mut hunks = Vec::new();
        if let Some(mut i) = first_hunk {
            while i < lines.len() {
                let Some((os, ol, ns, nl)) = parse_hunk_header(&lines[i]) else {
                    break;
                };
                let header = lines[i].clone();
                let body_start = i + 1;
                let mut body_end = body_start;
                while body_end < lines.len() && parse_hunk_header(&lines[body_end]).is_none() {
                    body_end += 1;
                }
                hunks.push(DiffHunk {
                    header,
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    lines: lines[body_start..body_end].to_vec(),
                });
                i = body_end;
            }
        }
        files.push(FileDiff {
            path,
            header_lines,
            hunks,
            is_new_file,
            is_deleted_file,
        });
    }
    files
}

/// Builds a standalone one-hunk unified-diff patch for `git apply --cached`.
pub fn build_hunk_patch(file: &FileDiff, hunk: &DiffHunk) -> String {
    let mut parts: Vec<&str> = file.header_lines.iter().map(|s| s.as_str()).collect();
    parts.push(hunk.header.as_str());
    for l in &hunk.lines {
        parts.push(l.as_str());
    }
    format!("{}\n", parts.join("\n"))
}

/// A brand-new / fully-deleted file collapses to one whole-file hunk — the
/// plain `git add` / `git restore --staged` path is far more robust than
/// applying a synthetic patch, so callers prefer it when this is true.
pub fn is_whole_file_single_hunk(file: &FileDiff) -> bool {
    (file.is_new_file || file.is_deleted_file) && file.hunks.len() == 1
}

// ─── Commit-message validation (port of CommitForm.tsx) ───────────────────────

/// `Ok(trimmed message)` or `Err(reason)`.
pub fn validate_commit_message(raw: &str, staged_count: usize) -> Result<String, String> {
    let msg = raw.trim();
    if msg.is_empty() {
        return Err("Enter a commit message".to_string());
    }
    if staged_count == 0 {
        return Err("Nothing staged to commit".to_string());
    }
    Ok(msg.to_string())
}

// ─── File status presentation ────────────────────────────────────────────────

/// Single-letter badge for a file entry (`M`, `A`, `D`, `R`, `C`, `U`, `?`).
pub fn status_letter(f: &FileStatus, untracked: bool) -> char {
    if untracked {
        return '?';
    }
    if f.conflicted {
        return 'U';
    }
    let s = if f.index_status != '.' {
        f.index_status
    } else {
        f.worktree_status
    };
    if s == '.' {
        'M'
    } else {
        s
    }
}

/// Conflicted entries can be filed into both `staged` and `unstaged` by the
/// porcelain parser — dedupe them into their own bucket so a conflicted file
/// renders exactly once (mirrors `SourceControlPanel.tsx`).
struct Buckets {
    conflicted: Vec<FileStatus>,
    staged: Vec<FileStatus>,
    unstaged: Vec<FileStatus>,
    untracked: Vec<FileStatus>,
}

fn bucketize(status: &GitStatus) -> Buckets {
    let mut conflicted: Vec<FileStatus> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for f in status.staged.iter().chain(status.unstaged.iter()) {
        if f.conflicted && seen.insert(f.path.clone()) {
            conflicted.push(f.clone());
        }
    }
    Buckets {
        staged: status
            .staged
            .iter()
            .filter(|f| !f.conflicted)
            .cloned()
            .collect(),
        unstaged: status
            .unstaged
            .iter()
            .filter(|f| !f.conflicted)
            .cloned()
            .collect(),
        untracked: status.untracked.clone(),
        conflicted,
    }
}

// ─── View ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Conflicts,
    Staged,
    Unstaged,
    Untracked,
}

/// The currently previewed file.
#[derive(Clone, PartialEq, Eq)]
struct Selected {
    path: String,
    /// `true` → diff index↔HEAD; `false` → diff worktree↔index.
    staged: bool,
    untracked: bool,
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
    warning: gpui::Hsla,
    info: gpui::Hsla,
    modified: gpui::Hsla,
}

pub struct GitPanelView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    focus: FocusHandle,
    commit_focus: FocusHandle,

    /// Directory the panel points at (the active terminal cwd).
    root: Option<String>,
    /// Remote SSH session id, if the target is remote (local repos: `None`).
    session_id: Option<String>,
    /// Resolved repository root (`git rev-parse --show-toplevel`).
    repo_root: Option<String>,
    is_repo: bool,
    poll_error: Option<String>,

    state: Option<WorkspaceGitState>,

    target_gen: u64,
    last_target: Option<(Option<String>, Option<String>)>,
    refreshing: bool,
    op_in_progress: bool,

    collapsed: std::collections::HashSet<u8>,

    selected: Option<Selected>,
    diff_text: Option<String>,
    diff_error: Option<String>,

    commit_msg: String,
    commit_error: Option<String>,
    /// Second-click confirmation latch for force-push.
    force_push_armed: bool,
}

impl Focusable for GitPanelView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl GitPanelView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        // Poll loop — stops when the view is dropped.
        cx.spawn(async move |this, cx| loop {
            let interval = this
                .read_with(cx, |this, _| this.poll_interval())
                .unwrap_or(POLL_INTERVAL);
            cx.background_executor().timer(interval).await;
            if this.update(cx, |this, cx| this.refresh(cx)).is_err() {
                break;
            }
        })
        .detach();

        Self {
            backend,
            tokio,
            theme,
            focus: cx.focus_handle(),
            commit_focus: cx.focus_handle(),
            root: None,
            session_id: None,
            repo_root: None,
            is_repo: false,
            poll_error: None,
            state: None,
            target_gen: 0,
            last_target: None,
            refreshing: false,
            op_in_progress: false,
            collapsed: std::collections::HashSet::new(),
            selected: None,
            diff_text: None,
            diff_error: None,
            commit_msg: String::new(),
            commit_error: None,
            force_push_armed: false,
        }
    }

    fn poll_interval(&self) -> Duration {
        if self.session_id.is_some() {
            POLL_INTERVAL * REMOTE_POLL_MULTIPLIER
        } else {
            POLL_INTERVAL
        }
    }

    /// Points the panel at a new working directory (called from the app shell
    /// when the active terminal's cwd changes).
    pub fn set_root(&mut self, root: Option<String>, cx: &mut Context<Self>) {
        if self.root == root {
            return;
        }
        self.root = root;
        self.refresh(cx);
        cx.notify();
    }

    /// Points the panel at a remote SSH session (or back to local with `None`).
    pub fn set_session(&mut self, session_id: Option<String>, cx: &mut Context<Self>) {
        if self.session_id == session_id {
            return;
        }
        self.session_id = session_id;
        self.refresh(cx);
        cx.notify();
    }

    // ── polling ────────────────────────────────────────────────────────────

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let target = (self.root.clone(), self.session_id.clone());
        if self.last_target.as_ref() != Some(&target) {
            self.last_target = Some(target.clone());
            self.target_gen += 1;
            // Target changed — clear stale preview.
            self.selected = None;
            self.diff_text = None;
            self.diff_error = None;
        }
        let Some(root) = self.root.clone() else {
            self.is_repo = false;
            self.state = None;
            return;
        };
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let generation = self.target_gen;
        let session = self.session_id.clone();
        let backend = self.backend.clone();

        let jh = self.tokio.spawn(async move {
            let is_repo =
                git::git_is_repo(root.clone(), session.clone(), &backend.ssh, backend.clone())
                    .await?;
            if !is_repo {
                return Ok::<_, String>(None);
            }
            let repo_root = git::git_get_repo_root(
                root.clone(),
                session.clone(),
                &backend.ssh,
                backend.clone(),
            )
            .await?;
            let state = git::git_get_workspace_state(
                repo_root.clone(),
                session.clone(),
                &backend.ssh,
                backend.clone(),
            )
            .await?;
            Ok(Some((repo_root, state)))
        });

        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.refreshing = false;
                if this.target_gen != generation {
                    return;
                }
                match res {
                    Ok(Some((repo_root, state))) => {
                        let root_changed = this.repo_root.as_deref() != Some(repo_root.as_str());
                        this.is_repo = true;
                        this.repo_root = Some(repo_root);
                        this.state = Some(state);
                        this.poll_error = None;
                        if root_changed {
                            this.selected = None;
                            this.diff_text = None;
                        }
                        this.reload_diff(cx);
                    }
                    Ok(None) => {
                        this.is_repo = false;
                        this.repo_root = None;
                        this.state = None;
                        this.poll_error = None;
                    }
                    Err(e) => {
                        this.poll_error = Some(e);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh_soon(&mut self, cx: &mut Context<Self>) {
        // Force a fresh poll on the next tick regardless of the in-flight flag.
        self.refreshing = false;
        self.refresh(cx);
    }

    // ── diff preview ───────────────────────────────────────────────────────

    fn select_file(&mut self, sel: Selected, cx: &mut Context<Self>) {
        if self.selected.as_ref() == Some(&sel) {
            self.selected = None;
            self.diff_text = None;
            self.diff_error = None;
        } else {
            self.selected = Some(sel);
            self.reload_diff(cx);
        }
        cx.notify();
    }

    fn reload_diff(&mut self, cx: &mut Context<Self>) {
        let (Some(repo_root), Some(sel)) = (self.repo_root.clone(), self.selected.clone()) else {
            return;
        };
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let generation = self.target_gen;
        let jh = self.tokio.spawn(async move {
            git::git_get_diff(
                repo_root,
                sel.path,
                sel.staged,
                Some(false),
                Some(sel.untracked),
                session,
                &backend.ssh,
                backend.clone(),
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                if this.target_gen != generation {
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

    // ── generic backend-op dispatch ────────────────────────────────────────

    /// Runs `op` on the tokio runtime, toasts any error, then refreshes.
    fn run_op<F>(&mut self, title: &'static str, op: F, cx: &mut Context<Self>)
    where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        if self.op_in_progress {
            return;
        }
        self.op_in_progress = true;
        cx.notify();
        let jh = self.tokio.spawn(op);
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.op_in_progress = false;
                notify_err(title, res, cx);
                this.refresh_soon(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn ctx(&self) -> Option<(String, Option<String>, Backend)> {
        Some((
            self.repo_root.clone()?,
            self.session_id.clone(),
            self.backend.clone(),
        ))
    }

    fn stage_file(&mut self, path: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Stage failed",
            async move { git::git_stage_file(root, path, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn unstage_file(&mut self, path: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Unstage failed",
            async move { git::git_unstage_file(root, path, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn discard_file(&mut self, path: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Discard failed",
            async move { git::git_discard_file(root, path, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn stage_all(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Stage all failed",
            async move { git::git_stage_all(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn unstage_all(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Unstage all failed",
            async move { git::git_unstage_all(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn discard_all(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Discard all failed",
            async move { git::git_discard_all(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn clean_untracked(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Clean failed",
            async move { git::git_clean_untracked(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    /// Stage (or, if `reverse`, unstage) the hunk at `hunk_idx` of the loaded
    /// diff. Falls back to whole-file stage/unstage for new/deleted files.
    fn apply_hunk(&mut self, hunk_idx: usize, reverse: bool, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let Some(diff) = self.diff_text.clone() else {
            return;
        };
        let files = parse_diff_hunks(&diff);
        let Some(file) = files.into_iter().next() else {
            notify_err::<()>(
                "Hunk staging unavailable",
                Err("Diff could not be parsed for hunk staging.".to_string()),
                cx,
            );
            return;
        };
        let path = file.path.clone();
        if is_whole_file_single_hunk(&file) {
            if reverse {
                self.unstage_file(path, cx);
            } else {
                self.stage_file(path, cx);
            }
            return;
        }
        let Some(hunk) = file.hunks.get(hunk_idx) else {
            return;
        };
        let patch = build_hunk_patch(&file, hunk);
        let title = if reverse {
            "Unstage hunk failed"
        } else {
            "Stage hunk failed"
        };
        self.run_op(
            title,
            async move {
                if reverse {
                    git::git_unstage_hunk(root, path, patch, sid, &be.ssh, be.clone()).await
                } else {
                    git::git_stage_hunk(root, path, patch, sid, &be.ssh, be.clone()).await
                }
            },
            cx,
        );
    }

    // ── commit / sync ──────────────────────────────────────────────────────

    fn do_commit(&mut self, cx: &mut Context<Self>) {
        let staged_count = self
            .state
            .as_ref()
            .map(|s| bucketize(&s.status).staged.len() + bucketize(&s.status).conflicted.len())
            .unwrap_or(0);
        let msg = match validate_commit_message(&self.commit_msg, staged_count) {
            Ok(m) => m,
            Err(e) => {
                self.commit_error = Some(e);
                cx.notify();
                return;
            }
        };
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.commit_error = None;
        self.commit_msg.clear();
        self.run_op(
            "Commit failed",
            async move {
                git::git_commit(root, msg, false, sid, &be.ssh, be.clone())
                    .await
                    .map(|_| ())
            },
            cx,
        );
    }

    fn pull(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Pull failed",
            async move {
                git::git_pull(root, sid, &be.ssh, be.clone())
                    .await
                    .map(|_| ())
            },
            cx,
        );
    }

    fn push(&mut self, cx: &mut Context<Self>) {
        let has_upstream = self.current_branch_has_upstream();
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let branch = self
            .state
            .as_ref()
            .map(|s| s.current_branch.clone())
            .unwrap_or_default();
        if has_upstream {
            self.run_op(
                "Push failed",
                async move {
                    git::git_push(root, None, None, sid, &be.ssh, be.clone())
                        .await
                        .map(|_| ())
                },
                cx,
            );
        } else {
            // New branch — publish with --set-upstream to origin.
            self.run_op(
                "Publish failed",
                async move {
                    git::git_push_set_upstream(
                        root,
                        "origin".to_string(),
                        branch,
                        sid,
                        &be.ssh,
                        be.clone(),
                    )
                    .await
                    .map(|_| ())
                },
                cx,
            );
        }
    }

    fn force_push(&mut self, cx: &mut Context<Self>) {
        // Requires an explicit second click (`force_push_armed`).
        if !self.force_push_armed {
            self.force_push_armed = true;
            cx.notify();
            return;
        }
        self.force_push_armed = false;
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let branch = self
            .state
            .as_ref()
            .map(|s| s.current_branch.clone())
            .unwrap_or_default();
        self.run_op(
            "Force push failed",
            async move {
                git::git_push_force_with_lease(
                    root,
                    Some("origin".to_string()),
                    Some(branch),
                    sid,
                    &be.ssh,
                    be.clone(),
                )
                .await
                .map(|_| ())
            },
            cx,
        );
    }

    fn fetch(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Fetch failed",
            async move {
                git::git_fetch(root, sid, &be.ssh, be.clone())
                    .await
                    .map(|_| ())
            },
            cx,
        );
    }

    fn abort(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Abort failed",
            async move { git::git_abort(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn git_continue(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Continue failed",
            async move { git::git_continue(root, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn current_branch_has_upstream(&self) -> bool {
        let Some(state) = &self.state else {
            return false;
        };
        state
            .branches
            .iter()
            .any(|b| b.name == state.current_branch && !b.is_remote && b.upstream.is_some())
    }

    // ── commit-message text input ──────────────────────────────────────────

    fn on_commit_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "enter" => {
                if ks.modifiers.platform || ks.modifiers.control {
                    self.do_commit(cx);
                } else {
                    self.commit_msg.push('\n');
                }
                cx.notify();
            }
            "backspace" => {
                self.commit_msg.pop();
                cx.notify();
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
                if let Some(ch) = ch {
                    self.commit_msg.push_str(&ch);
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }

    // ── rendering ──────────────────────────────────────────────────────────

    fn colors(&self, cx: &App) -> Colors {
        let t = self.theme.read(cx);
        Colors {
            bg: t.sidebar_bg(),
            fg: t.sidebar_fg(),
            muted: t.muted_foreground(),
            border: t.sidebar_border(),
            card: t.card(),
            accent: t.accent(),
            success: t.status_success(),
            error: t.status_error(),
            warning: t.status_warning(),
            info: t.status_info(),
            modified: t.status_modified(),
        }
    }

    fn status_color(&self, letter: char, c: Colors) -> gpui::Hsla {
        match letter {
            'A' | '?' => c.success,
            'D' => c.error,
            'M' | 'R' | 'C' => c.modified,
            'U' => c.warning,
            _ => c.muted,
        }
    }

    fn tool_btn(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        c: Colors,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex()
            .items_center()
            .h(px(22.0))
            .px(px(6.0))
            .rounded_sm()
            .text_size(px(11.0))
            .text_color(c.muted)
            .hover(|s| s.bg(c.border).text_color(c.fg))
            .child(label.into())
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| on_click(this, w, cx)))
    }

    fn render_section(
        &self,
        section: Section,
        title: &'static str,
        files: &[FileStatus],
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if files.is_empty() {
            return div().into_any_element();
        }
        let key = section as u8;
        let collapsed = self.collapsed.contains(&key);
        let untracked = matches!(section, Section::Untracked);
        let staged = matches!(section, Section::Staged);

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(22.0))
            .px(px(8.0))
            .text_size(px(10.0))
            .text_color(c.muted)
            .child(
                div()
                    .id(SharedString::from(format!("git-sec-{title}")))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(SharedString::from(if collapsed {
                        "\u{25B8}"
                    } else {
                        "\u{25BE}"
                    }))
                    .child(SharedString::from(format!(
                        "{} ({})",
                        title.to_uppercase(),
                        files.len()
                    )))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if this.collapsed.contains(&key) {
                            this.collapsed.remove(&key);
                        } else {
                            this.collapsed.insert(key);
                        }
                        cx.notify();
                    })),
            );

        let mut list = div().flex().flex_col();
        if !collapsed {
            for f in files {
                let letter = status_letter(f, untracked);
                let lc = self.status_color(letter, c);
                let path = f.path.clone();
                let selected = self
                    .selected
                    .as_ref()
                    .map(|s| s.path == path)
                    .unwrap_or(false);
                let sel = Selected {
                    path: path.clone(),
                    staged,
                    untracked,
                };
                let action_path = path.clone();
                let row = div()
                    .id(SharedString::from(format!("git-file-{}-{}", key, path)))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .h(px(22.0))
                    .px(px(8.0))
                    .text_size(px(12.0))
                    .text_color(c.fg)
                    .when(selected, |d| d.bg(c.accent))
                    .hover(|s| s.bg(c.border))
                    .child(
                        div()
                            .w(px(12.0))
                            .flex_shrink_0()
                            .text_color(lc)
                            .text_size(px(11.0))
                            .child(SharedString::from(letter.to_string())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(short_path(&path))),
                    )
                    .when(
                        matches!(section, Section::Unstaged | Section::Conflicts),
                        |d| {
                            let dp = path.clone();
                            d.child(
                                div()
                                    .id(SharedString::from(format!("discard-{key}-{path}")))
                                    .flex_shrink_0()
                                    .w(px(16.0))
                                    .text_color(c.muted)
                                    .text_size(px(12.0))
                                    .hover(|s| s.text_color(c.error))
                                    .child(SharedString::from("\u{21BA}"))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        cx.stop_propagation();
                                        this.discard_file(dp.clone(), cx);
                                    })),
                            )
                        },
                    )
                    .child(self.row_action(section, action_path, c, cx))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        this.select_file(sel.clone(), cx);
                    }));
                list = list.child(row);
            }
        }

        div()
            .flex()
            .flex_col()
            .child(header)
            .child(list)
            .into_any_element()
    }

    fn row_action(
        &self,
        section: Section,
        path: String,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let (glyph, id): (&str, String) = match section {
            Section::Staged => ("\u{2212}", format!("unstage-{path}")),
            _ => ("+", format!("stage-{path}")),
        };
        div()
            .id(SharedString::from(id))
            .flex_shrink_0()
            .w(px(16.0))
            .text_color(c.muted)
            .text_size(px(13.0))
            .hover(|s| s.text_color(c.fg))
            .child(SharedString::from(glyph))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                cx.stop_propagation();
                let _ = ev;
                match section {
                    Section::Staged => this.unstage_file(path.clone(), cx),
                    _ => this.stage_file(path.clone(), cx),
                }
            }))
            .into_any_element()
    }

    fn render_diff(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(sel) = self.selected.clone() else {
            return div().into_any_element();
        };
        let mut body = div()
            .id("git-diff")
            .flex()
            .flex_col()
            .max_h(px(280.0))
            .overflow_y_scroll()
            .border_b_1()
            .border_color(c.border)
            .font(self.theme.read(cx).buffer_font())
            .text_size(px(11.0));

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(22.0))
            .px(px(8.0))
            .bg(c.card)
            .text_size(px(11.0))
            .text_color(c.fg)
            .child(SharedString::from(short_path(&sel.path)))
            .child(
                div()
                    .id("git-diff-close")
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("\u{2715}"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.selected = None;
                        this.diff_text = None;
                        cx.notify();
                    })),
            );

        if let Some(err) = &self.diff_error {
            body = body.child(
                div()
                    .p(px(8.0))
                    .text_color(c.error)
                    .child(SharedString::from(err.clone())),
            );
        } else if let Some(text) = &self.diff_text {
            if sel.untracked || !text.contains("@@ ") {
                // Untracked / no-hunk: show raw lines only.
                for line in text.lines().take(400) {
                    body = body.child(diff_line(line, c));
                }
            } else {
                let files = parse_diff_hunks(text);
                if let Some(file) = files.first() {
                    let whole = is_whole_file_single_hunk(file);
                    for (i, hunk) in file.hunks.iter().enumerate() {
                        let reverse = sel.staged;
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
                                            .id(SharedString::from(format!("hunk-{i}")))
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
                        for l in &hunk.lines {
                            body = body.child(diff_line(l, c));
                        }
                    }
                }
            }
        } else {
            body = body.child(
                div()
                    .p(px(8.0))
                    .text_color(c.muted)
                    .child(SharedString::from("Loading diff\u{2026}")),
            );
        }

        div()
            .flex()
            .flex_col()
            .child(header)
            .child(body)
            .into_any_element()
    }

    fn render_branch_bar(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = &self.state else {
            return div().into_any_element();
        };
        let status = &state.status;
        let has_upstream = self.current_branch_has_upstream();
        let push_label = if has_upstream { "Push" } else { "Publish" };

        let mut bar = div().flex().flex_col().border_t_1().border_color(c.border);

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .h(px(28.0))
            .px(px(8.0))
            .text_size(px(11.0))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(c.fg)
                    .child(SharedString::from(format!(
                        "\u{2325} {}",
                        if state.current_branch.is_empty() {
                            "\u{2014}"
                        } else {
                            &state.current_branch
                        }
                    ))),
            );
        if status.behind > 0 {
            row = row.child(
                div()
                    .text_color(c.error)
                    .child(SharedString::from(format!("\u{2193}{}", status.behind))),
            );
        }
        if status.ahead > 0 {
            row = row.child(
                div()
                    .text_color(c.info)
                    .child(SharedString::from(format!("\u{2191}{}", status.ahead))),
            );
        }
        row = row
            .child(self.tool_btn("git-fetch", "Fetch", c, cx, |this, _w, cx| this.fetch(cx)))
            .child(self.tool_btn("git-pull", "Pull", c, cx, |this, _w, cx| this.pull(cx)))
            .child(self.tool_btn("git-push", push_label, c, cx, |this, _w, cx| this.push(cx)))
            .child(self.tool_btn(
                "git-forcepush",
                if self.force_push_armed {
                    "Confirm force"
                } else {
                    "Force"
                },
                c,
                cx,
                |this, _w, cx| this.force_push(cx),
            ));
        bar = bar.child(row);

        // In-progress banners (merge / rebase / cherry-pick).
        let in_progress =
            status.merge_in_progress || status.rebase_in_progress || status.cherry_pick_in_progress;
        if in_progress {
            let text = if status.merge_in_progress {
                "Merge in progress \u{2014} resolve conflicts, then commit or Abort"
            } else if status.rebase_in_progress {
                "Rebase in progress \u{2014} resolve conflicts, then Continue or Abort"
            } else {
                "Cherry-pick in progress"
            };
            bar = bar.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(8.0))
                    .py(px(4.0))
                    .bg(c.warning.opacity(0.12))
                    .text_size(px(10.0))
                    .text_color(c.warning)
                    .child(SharedString::from(text))
                    .child(
                        self.tool_btn("git-continue", "Continue", c, cx, |this, _w, cx| {
                            this.git_continue(cx)
                        }),
                    )
                    .child(
                        self.tool_btn("git-abort", "Abort", c, cx, |this, _w, cx| this.abort(cx)),
                    ),
            );
        }

        bar.into_any_element()
    }

    fn render_commit_form(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let placeholder = self.commit_msg.is_empty();
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(8.0))
            .border_t_1()
            .border_color(c.border)
            .child(
                div()
                    .id("git-commit-input")
                    .track_focus(&self.commit_focus)
                    .min_h(px(48.0))
                    .p(px(6.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .text_size(px(12.0))
                    .text_color(if placeholder { c.muted } else { c.fg })
                    .whitespace_normal()
                    .child(SharedString::from(if placeholder {
                        "Message (\u{2318}Enter to commit)".to_string()
                    } else {
                        self.commit_msg.clone()
                    }))
                    .on_key_down(cx.listener(Self::on_commit_key))
                    .on_click(cx.listener(|this, _: &ClickEvent, w, _cx| {
                        w.focus(&this.commit_focus);
                    })),
            )
            .when_some(self.commit_error.clone(), |d, err| {
                d.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.error)
                        .child(SharedString::from(err)),
                )
            })
            .child(
                div()
                    .id("git-commit-btn")
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(px(26.0))
                    .rounded_sm()
                    .bg(c.accent)
                    .text_size(px(12.0))
                    .text_color(c.fg)
                    .hover(|s| s.opacity(0.85))
                    .child(SharedString::from("Commit"))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.do_commit(cx))),
            )
            .into_any_element()
    }

    fn render_no_repo(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .flex_1()
            .gap(px(8.0))
            .p(px(12.0))
            .text_center()
            .text_size(px(11.0))
            .text_color(c.muted)
            .child(SharedString::from(match &self.poll_error {
                Some(e) => e.clone(),
                None => "No Git repository at the current folder".to_string(),
            }))
            .child(
                self.tool_btn("git-retry", "Refresh", c, cx, |this, _w, cx| {
                    this.refresh_soon(cx)
                }),
            )
            .into_any_element()
    }
}

impl Render for GitPanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = self.colors(cx);

        let mut root = div()
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg);

        if !self.is_repo {
            return root.child(self.render_no_repo(c, cx));
        }

        let buckets = self
            .state
            .as_ref()
            .map(|s| bucketize(&s.status))
            .unwrap_or(Buckets {
                conflicted: vec![],
                staged: vec![],
                unstaged: vec![],
                untracked: vec![],
            });

        let total_changes = buckets.conflicted.len()
            + buckets.staged.len()
            + buckets.unstaged.len()
            + buckets.untracked.len();
        let has_unstaged = !buckets.unstaged.is_empty() || !buckets.untracked.is_empty();

        // Action bar.
        let action_bar = div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .h(px(28.0))
            .px(px(6.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                self.tool_btn("git-refresh", "\u{21BB} Refresh", c, cx, |this, _w, cx| {
                    this.refresh_soon(cx)
                }),
            )
            .child(if has_unstaged {
                self.tool_btn("git-stage-all", "Stage all", c, cx, |this, _w, cx| {
                    this.stage_all(cx)
                })
                .into_any_element()
            } else {
                self.tool_btn("git-unstage-all", "Unstage all", c, cx, |this, _w, cx| {
                    this.unstage_all(cx)
                })
                .into_any_element()
            })
            .child(
                self.tool_btn("git-discard-all", "Discard", c, cx, |this, _w, cx| {
                    this.discard_all(cx)
                }),
            )
            .child(self.tool_btn("git-clean", "Clean", c, cx, |this, _w, cx| {
                this.clean_untracked(cx)
            }));

        root = root.child(action_bar);

        if let Some(err) = &self.poll_error {
            root = root.child(
                div()
                    .mx(px(8.0))
                    .my(px(4.0))
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded_sm()
                    .bg(c.error.opacity(0.10))
                    .text_size(px(10.0))
                    .text_color(c.error)
                    .child(SharedString::from(err.clone())),
            );
        }

        root = root.child(self.render_diff(c, cx));

        let mut list = div()
            .id("git-file-list")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .py(px(2.0));

        if total_changes == 0 {
            list = list.child(
                div()
                    .p(px(12.0))
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from("No changes")),
            );
        } else {
            list = list
                .child(self.render_section(
                    Section::Conflicts,
                    "Conflicts",
                    &buckets.conflicted,
                    c,
                    cx,
                ))
                .child(self.render_section(Section::Staged, "Staged", &buckets.staged, c, cx))
                .child(self.render_section(Section::Unstaged, "Changes", &buckets.unstaged, c, cx))
                .child(self.render_section(
                    Section::Untracked,
                    "Untracked",
                    &buckets.untracked,
                    c,
                    cx,
                ));
        }

        root.child(list)
            .child(self.render_branch_bar(c, cx))
            .child(self.render_commit_form(c, cx))
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

/// Trims a repo-relative path to its last two segments for display.
fn short_path(path: &str) -> String {
    let segs: Vec<&str> = path.split('/').collect();
    if segs.len() <= 2 {
        path.to_string()
    } else {
        format!("\u{2026}/{}", segs[segs.len() - 2..].join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures are joined explicitly (not via `\` line-continuation, which
    // would strip the leading space that marks a diff context line).
    fn two_hunk_diff() -> String {
        [
            "diff --git a/tracked.txt b/tracked.txt",
            "index fe7fa38..e0b0b1c 100644",
            "--- a/tracked.txt",
            "+++ b/tracked.txt",
            "@@ -1,5 +1,5 @@",
            " a1",
            "-a2",
            "+a2_CHANGED",
            " a3",
            " a4",
            " a5",
            "@@ -11,5 +11,5 @@ a10",
            " a11",
            " a12",
            " a13",
            "-a14",
            "+a14_CHANGED",
            " a15",
            "",
        ]
        .join("\n")
    }

    fn new_file_diff() -> String {
        [
            "diff --git a/newfile.txt b/newfile.txt",
            "new file mode 100644",
            "index 0000000..71ac1b5",
            "--- /dev/null",
            "+++ b/newfile.txt",
            "@@ -0,0 +1,3 @@",
            "+a",
            "+b",
            "+c",
            "",
        ]
        .join("\n")
    }

    fn deleted_file_diff() -> String {
        [
            "diff --git a/newfile.txt b/newfile.txt",
            "deleted file mode 100644",
            "index 71ac1b5..0000000",
            "--- a/newfile.txt",
            "+++ /dev/null",
            "@@ -1,3 +0,0 @@",
            "-a",
            "-b",
            "-c",
            "",
        ]
        .join("\n")
    }

    fn crlf_diff() -> String {
        [
            "diff --git a/crlf.txt b/crlf.txt",
            "index 46b21fa..f146c25 100644",
            "--- a/crlf.txt",
            "+++ b/crlf.txt",
            "@@ -1,3 +1,3 @@",
            " x1\r",
            "-x2\r",
            "+X2_CHANGED\r",
            " x3\r",
            "",
        ]
        .join("\n")
    }

    #[test]
    fn splits_two_hunks_with_line_numbers() {
        let files = parse_diff_hunks(&two_hunk_diff());
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.path, "tracked.txt");
        assert!(!f.is_new_file && !f.is_deleted_file);
        assert_eq!(f.hunks.len(), 2);
        assert_eq!(
            (
                f.hunks[0].old_start,
                f.hunks[0].old_lines,
                f.hunks[0].new_start,
                f.hunks[0].new_lines
            ),
            (1, 5, 1, 5)
        );
        assert_eq!(f.hunks[1].header, "@@ -11,5 +11,5 @@ a10");
    }

    #[test]
    fn detects_new_and_deleted_files() {
        let n = &parse_diff_hunks(&new_file_diff())[0];
        assert!(n.is_new_file && !n.is_deleted_file);
        assert!(is_whole_file_single_hunk(n));
        let d = &parse_diff_hunks(&deleted_file_diff())[0];
        assert!(d.is_deleted_file && !d.is_new_file);
        assert!(is_whole_file_single_hunk(d));
    }

    #[test]
    fn multi_hunk_is_not_whole_file() {
        assert!(!is_whole_file_single_hunk(
            &parse_diff_hunks(&two_hunk_diff())[0]
        ));
    }

    #[test]
    fn preserves_crlf_content_bytes() {
        let f = &parse_diff_hunks(&crlf_diff())[0];
        assert!(f.hunks[0].lines.contains(&" x1\r".to_string()));
        assert!(f.hunks[0].lines.contains(&"-x2\r".to_string()));
        assert!(f.hunks[0].lines.contains(&"+X2_CHANGED\r".to_string()));
    }

    #[test]
    fn truncated_and_empty_return_nothing() {
        let truncated = format!(
            "{}\n\n[diff truncated \u{2014} output exceeded 200 KB]",
            two_hunk_diff()
        );
        assert!(parse_diff_hunks(&truncated).is_empty());
        assert!(parse_diff_hunks("").is_empty());
    }

    #[test]
    fn parses_multiple_files_independently() {
        let combined = format!("{}\n{}", two_hunk_diff(), new_file_diff());
        let files = parse_diff_hunks(&combined);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "tracked.txt");
        assert_eq!(files[1].path, "newfile.txt");
        assert!(files[1].is_new_file);
    }

    #[test]
    fn builds_standalone_hunk_patch() {
        let f = &parse_diff_hunks(&two_hunk_diff())[0];
        let patch = build_hunk_patch(f, &f.hunks[0]);
        let expected = [
            "diff --git a/tracked.txt b/tracked.txt",
            "index fe7fa38..e0b0b1c 100644",
            "--- a/tracked.txt",
            "+++ b/tracked.txt",
            "@@ -1,5 +1,5 @@",
            " a1",
            "-a2",
            "+a2_CHANGED",
            " a3",
            " a4",
            " a5",
            "",
        ]
        .join("\n");
        assert_eq!(patch, expected);
    }

    #[test]
    fn hunk_patch_round_trips_crlf() {
        let f = &parse_diff_hunks(&crlf_diff())[0];
        let patch = build_hunk_patch(f, &f.hunks[0]);
        assert!(patch.contains(" x1\r\n"));
        assert!(patch.contains("-x2\r\n"));
        assert!(patch.contains("+X2_CHANGED\r\n"));
    }

    #[test]
    fn commit_message_validation() {
        assert!(validate_commit_message("   ", 3).is_err());
        assert!(validate_commit_message("msg", 0).is_err());
        assert_eq!(validate_commit_message("  hi ", 1).unwrap(), "hi");
    }

    #[test]
    fn status_letter_cases() {
        let mk = |i: char, w: char, conflicted: bool| FileStatus {
            path: "f".into(),
            original_path: None,
            index_status: i,
            worktree_status: w,
            submodule: None,
            conflicted,
        };
        assert_eq!(status_letter(&mk('.', 'M', false), false), 'M');
        assert_eq!(status_letter(&mk('A', '.', false), false), 'A');
        assert_eq!(status_letter(&mk('U', 'U', true), false), 'U');
        assert_eq!(status_letter(&mk('.', '.', false), true), '?');
    }

    #[test]
    fn bucketize_dedupes_conflicts() {
        let mk = |path: &str, conflicted: bool| FileStatus {
            path: path.into(),
            original_path: None,
            index_status: if conflicted { 'U' } else { 'M' },
            worktree_status: if conflicted { 'U' } else { '.' },
            submodule: None,
            conflicted,
        };
        let status = GitStatus {
            staged: vec![mk("a", true), mk("b", false)],
            unstaged: vec![mk("a", true), mk("c", false)],
            untracked: vec![mk("d", false)],
            has_conflicts: true,
            merge_in_progress: true,
            rebase_in_progress: false,
            cherry_pick_in_progress: false,
            ahead: 0,
            behind: 0,
        };
        let b = bucketize(&status);
        assert_eq!(b.conflicted.len(), 1);
        assert_eq!(b.conflicted[0].path, "a");
        assert_eq!(b.staged.len(), 1);
        assert_eq!(b.staged[0].path, "b");
        assert_eq!(b.unstaged.len(), 1);
        assert_eq!(b.unstaged[0].path, "c");
        assert_eq!(b.untracked.len(), 1);
    }

    #[test]
    fn short_path_trims_to_two_segments() {
        assert_eq!(short_path("a.txt"), "a.txt");
        assert_eq!(short_path("src/a.txt"), "src/a.txt");
        assert_eq!(short_path("a/b/c/d.txt"), "\u{2026}/c/d.txt");
    }
}
