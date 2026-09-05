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
//!
//! Crate root (T16-008): this file is the `labonair-panel-scm` lib root. The
//! `theme` shim keeps the pre-split `crate::theme::…` paths resolving against
//! `labonair_theme::store`.

pub(crate) mod theme {
    pub use labonair_theme::store::*;
}

use std::time::Duration;

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, uniform_list, App, AppContext, ClickEvent, Context, Entity, EventEmitter, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseDownEvent, ParentElement,
    Pixels, Point, Render, SharedString, StatefulInteractiveElement, Styled, Window,
};
use labonair_backend::modules::git::{
    self, Branch, CommitInfo, FileStatus, GitStatus, WorkspaceGitState,
};
use labonair_backend::App as Backend;
use labonair_panel::{ProjectDiffFile, ProjectDiffMode, ProjectDiffRequest};
use tokio::runtime::Handle as TokioHandle;

use crate::theme::ThemeStore;
use labonair_notifications::notify_err;
use labonair_ui_kit::{
    button, checkbox, context_menu, disclosure, field_input, git_change_row, h_stack,
    segmented_control, ButtonSize, ButtonVariant, IconName, InputEvent, InputState, ListItem,
    MenuItem, Palette, SegmentSize, SegmentVariant, StageState,
};

// Unified-diff parsing moved to `labonair-editor` in the Zed-parity Phase 4
// redesign so the panel and the workspace Project Diff item share one
// implementation. Re-exported for the (pre-existing) public API surface.
pub use labonair_editor::unified::{
    build_hunk_patch, is_whole_file_single_hunk, parse_diff_hunks, DiffHunk, FileDiff,
};

/// A source-control file-menu action, wrapped into a click handler by
/// `render_file_menu`.
type GitFileAct = Box<dyn Fn(&mut GitPanelView, &mut Context<GitPanelView>)>;

/// Local git status poll interval (matches the reference default
/// `gitStatusPollIntervalMs`). Remote targets back off (see
/// [`GitPanelView::poll_interval`]).
const POLL_INTERVAL: Duration = Duration::from_millis(2000);
const REMOTE_POLL_MULTIPLIER: u32 = 3;

/// Current `scmFileTree` setting (`false` when settings are not yet loaded, e.g.
/// in headless tests).
fn scm_file_tree_setting(cx: &App) -> bool {
    use labonair_settings::Settings as _;
    labonair_settings::ScmSettings::try_get(cx)
        .map(|s| s.file_tree())
        .unwrap_or(false)
}

// ─── Branch / stash helpers (port of BranchDropdown / StashPanel) ─────────────

/// Case-insensitive substring filter over branches of one kind (local or
/// remote). An empty/whitespace filter returns every branch of that kind.
pub fn filter_branches<'a>(branches: &'a [Branch], filter: &str, remote: bool) -> Vec<&'a Branch> {
    let f = filter.trim().to_lowercase();
    branches
        .iter()
        .filter(|b| b.is_remote == remote)
        .filter(|b| f.is_empty() || b.name.to_lowercase().contains(&f))
        .collect()
}

/// Maps a raw `git checkout` failure to the user-facing message the reference
/// `BranchDropdown.handleCheckout` shows.
pub fn map_checkout_error(err: &str) -> String {
    if err.contains("overwritten") {
        "Could not checkout: uncommitted changes would be overwritten. Stash your changes first."
            .to_string()
    } else {
        format!("Could not checkout: {err}")
    }
}

/// A `git branch -d` refusal because the branch still has unmerged commits —
/// the reference escalates this to the "force delete?" confirm.
pub fn is_unmerged_branch_error(err: &str) -> bool {
    err.contains("not fully merged")
}

/// Display label for a stash entry (`StashPanel` shows `"WIP"` for a blank
/// message).
pub fn stash_display_message(msg: &str) -> String {
    let t = msg.trim();
    if t.is_empty() {
        "WIP".to_string()
    } else {
        t.to_string()
    }
}

/// `git stash pop`/`apply` that hit merge conflicts — the entry is kept (pop
/// behaves like apply) and the working tree carries the conflicts.
pub fn is_stash_conflict_error(err: &str) -> bool {
    err.contains("conflict") || err.contains("CONFLICT")
}

/// Message shown for a conflicting stash pop/apply (mirrors `StashPanel`).
pub fn stash_conflict_message(err: &str) -> String {
    if is_stash_conflict_error(err) {
        "Conflicts after stash apply \u{2014} the stash was kept; resolve the conflicts in the working tree".to_string()
    } else {
        err.to_string()
    }
}

/// Detached-HEAD label produced by the backend; `NewBranchDialog` unwraps it
/// to the bare short hash so `git branch <name> <ref>` can resolve it.
pub fn resolve_default_from_ref(current_branch: &str) -> String {
    const PREFIX: &str = "HEAD detached at ";
    match current_branch.strip_prefix(PREFIX) {
        Some(hash) => hash.trim().to_string(),
        None => current_branch.to_string(),
    }
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    Conflicts,
    Staged,
    Unstaged,
    Untracked,
}

/// Top-level panel information mode (Zed-parity Phase 4, §9.5 "Panel
/// navigation"). Two *real* modes — a commit log is not a decoration.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum PanelMode {
    #[default]
    Changes,
    History,
}

/// What a commit click will actually do, derived from repository + staging
/// state (Zed-parity Phase 4, §12.5). Drives the adaptive commit-button label.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CommitMode {
    /// Commit exactly what is staged.
    CommitStaged,
    /// Nothing is staged but tracked files are dirty — `git commit -a`.
    CommitTracked,
    /// Replace the last commit (`--amend`).
    Amend,
}

impl CommitMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            CommitMode::CommitStaged => "Commit",
            CommitMode::CommitTracked => "Commit Tracked",
            CommitMode::Amend => "Amend",
        }
    }
}

/// A repository-wide async operation currently in flight. Carries enough
/// identity to show progress / disable the affected action without freezing
/// the whole panel (Zed-parity Phase 4, §12.5 / Critical Rule 5).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum RepoOperation {
    #[default]
    Idle,
    Fetching,
    Pulling,
    Pushing,
    /// Staging / discarding / branch / stash / tag mutation.
    Mutating,
}

impl RepoOperation {
    fn is_busy(self) -> bool {
        !matches!(self, RepoOperation::Idle)
    }
}

/// Why the commit action is unavailable — surfaced in the button tooltip and
/// accessibility description (Zed-parity Phase 4, §12.5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DisabledReason {
    NoMessage,
    NoChanges,
    Conflict,
    OperationInProgress,
}

impl DisabledReason {
    pub(crate) fn describe(self) -> &'static str {
        match self {
            DisabledReason::NoMessage => "Enter a commit message",
            DisabledReason::NoChanges => "Nothing to commit",
            DisabledReason::Conflict => "Resolve merge conflicts before committing",
            DisabledReason::OperationInProgress => "A Git operation is in progress",
        }
    }
}

/// Pure derivation of the commit mode. `amend` is the explicit user toggle;
/// otherwise a non-empty stage commits the index, and a clean index with dirty
/// tracked files commits those.
pub(crate) fn derive_commit_mode(
    amend: bool,
    staged_count: usize,
    tracked_dirty: bool,
) -> CommitMode {
    if amend {
        CommitMode::Amend
    } else if staged_count > 0 {
        CommitMode::CommitStaged
    } else if tracked_dirty {
        CommitMode::CommitTracked
    } else {
        CommitMode::CommitStaged
    }
}

/// Pure derivation of the disabled reason (`None` → the commit action is
/// enabled). Order matters: an in-flight op and unresolved conflicts block
/// regardless of message/stage state.
pub(crate) fn derive_disabled_reason(
    message_empty: bool,
    staged_count: usize,
    tracked_dirty: bool,
    has_conflicts: bool,
    op: RepoOperation,
    amend: bool,
) -> Option<DisabledReason> {
    if op.is_busy() {
        return Some(DisabledReason::OperationInProgress);
    }
    if has_conflicts {
        return Some(DisabledReason::Conflict);
    }
    if message_empty {
        return Some(DisabledReason::NoMessage);
    }
    // Amend can re-commit with nothing new staged; every other mode needs
    // something to commit.
    if !amend && staged_count == 0 && !tracked_dirty {
        return Some(DisabledReason::NoChanges);
    }
    None
}

/// One line of the flattened Source-Control presentation list (Zed-parity
/// §12.5). Built by [`flatten_git`] (flat) or [`flatten_git_tree`] (tree)
/// independently of the nested section render loops, then virtualised.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum GitListEntry {
    SectionHeader {
        section: Section,
        title: &'static str,
        count: usize,
        /// Aggregate staging state of the section's files.
        stage: StageState,
        collapsed: bool,
    },
    /// A directory grouping node (tree presentation only). `paths` are the
    /// descendant file paths it aggregates for section/folder-level staging.
    Directory {
        section: Section,
        /// Repo-relative directory path, no trailing slash.
        path: String,
        /// Last segment, for display.
        name: String,
        depth: usize,
        stage: StageState,
        collapsed: bool,
        paths: Vec<String>,
    },
    File {
        section: Section,
        path: String,
        depth: usize,
        letter: char,
        staged: bool,
        untracked: bool,
        selected: bool,
    },
    EmptyState,
}

/// Fold a set of per-file staged flags into an aggregate tri-state.
pub(crate) fn aggregate_stage(staged_flags: &[bool]) -> StageState {
    if staged_flags.is_empty() || staged_flags.iter().all(|s| !s) {
        StageState::Unstaged
    } else if staged_flags.iter().all(|s| *s) {
        StageState::Staged
    } else {
        StageState::PartiallyStaged
    }
}

/// The four status sections, as `(Section, title, files)` tuples. Shared by
/// both flatteners so the file set is identical between tree and flat views.
pub(crate) type Sections<'a> = [(Section, &'static str, &'a [FileStatus]); 4];

/// Flatten the status buckets into one flat presentation list. Pure — no
/// `self`, no GPUI, no IO; unit-tested below.
pub(crate) fn flatten_git(
    sections: &[(Section, &'static str, &[FileStatus])],
    collapsed: &std::collections::HashSet<u8>,
    selected: Option<&str>,
) -> Vec<GitListEntry> {
    let _span = tracing::trace_span!(
        target: "labonair::perf",
        "scm_flatten_flat",
        files = sections.iter().map(|(_, _, f)| f.len()).sum::<usize>()
    )
    .entered();
    let mut out = Vec::new();
    for (section, title, files) in sections {
        if files.is_empty() {
            continue;
        }
        let staged = matches!(section, Section::Staged);
        let untracked = matches!(section, Section::Untracked);
        let key = *section as u8;
        let is_collapsed = collapsed.contains(&key);
        let flags: Vec<bool> = files.iter().map(|_| staged).collect();
        out.push(GitListEntry::SectionHeader {
            section: *section,
            title,
            count: files.len(),
            stage: aggregate_stage(&flags),
            collapsed: is_collapsed,
        });
        if !is_collapsed {
            for f in *files {
                out.push(GitListEntry::File {
                    section: *section,
                    path: f.path.clone(),
                    depth: 0,
                    letter: status_letter(f, untracked),
                    staged,
                    untracked,
                    selected: selected == Some(f.path.as_str()),
                });
            }
        }
    }
    if out.is_empty() {
        out.push(GitListEntry::EmptyState);
    }
    out
}

/// Flatten the same status buckets into a directory *tree* presentation. Same
/// file set as [`flatten_git`] — directories are grouping nodes only. Pure.
pub(crate) fn flatten_git_tree(
    sections: &[(Section, &'static str, &[FileStatus])],
    collapsed: &std::collections::HashSet<u8>,
    dir_collapsed: &std::collections::HashSet<String>,
    selected: Option<&str>,
) -> Vec<GitListEntry> {
    let _span = tracing::trace_span!(
        target: "labonair::perf",
        "scm_flatten_tree",
        files = sections.iter().map(|(_, _, f)| f.len()).sum::<usize>()
    )
    .entered();
    let mut out = Vec::new();
    for (section, title, files) in sections {
        if files.is_empty() {
            continue;
        }
        let staged = matches!(section, Section::Staged);
        let untracked = matches!(section, Section::Untracked);
        let key = *section as u8;
        let flags: Vec<bool> = files.iter().map(|_| staged).collect();
        let sec_collapsed = collapsed.contains(&key);
        out.push(GitListEntry::SectionHeader {
            section: *section,
            title,
            count: files.len(),
            stage: aggregate_stage(&flags),
            collapsed: sec_collapsed,
        });
        if sec_collapsed {
            continue;
        }

        // Emit directory nodes + files in path order. A directory node appears
        // once, the first time its prefix is seen; its subtree is skipped when
        // it (or an ancestor) is collapsed.
        let mut sorted: Vec<&FileStatus> = files.iter().collect();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        let mut emitted_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for f in sorted {
            let segs: Vec<&str> = f.path.split('/').collect();
            let mut prefix = String::new();
            let mut hidden = false;
            for (i, seg) in segs.iter().enumerate() {
                let is_file = i + 1 == segs.len();
                if is_file {
                    if hidden {
                        break;
                    }
                    out.push(GitListEntry::File {
                        section: *section,
                        path: f.path.clone(),
                        depth: i,
                        letter: status_letter(f, untracked),
                        staged,
                        untracked,
                        selected: selected == Some(f.path.as_str()),
                    });
                    break;
                }
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(seg);
                if hidden {
                    continue;
                }
                if emitted_dirs.insert(prefix.clone()) {
                    let dir_paths: Vec<String> = files
                        .iter()
                        .filter(|x| x.path.starts_with(&format!("{prefix}/")) || x.path == prefix)
                        .map(|x| x.path.clone())
                        .collect();
                    let dir_flags: Vec<bool> = dir_paths.iter().map(|_| staged).collect();
                    let dcoll = dir_collapsed.contains(&prefix);
                    out.push(GitListEntry::Directory {
                        section: *section,
                        path: prefix.clone(),
                        name: (*seg).to_string(),
                        depth: i,
                        stage: aggregate_stage(&dir_flags),
                        collapsed: dcoll,
                        paths: dir_paths,
                    });
                    if dcoll {
                        hidden = true;
                    }
                } else if dir_collapsed.contains(&prefix) {
                    hidden = true;
                }
            }
        }
    }
    if out.is_empty() {
        out.push(GitListEntry::EmptyState);
    }
    out
}

/// Every file path across all sections, in section order — the ordered change
/// list handed to the Project Diff item.
pub(crate) fn all_change_files(
    sections: &[(Section, &'static str, &[FileStatus])],
) -> Vec<ProjectDiffFile> {
    let mut out = Vec::new();
    for (section, _title, files) in sections {
        let staged = matches!(section, Section::Staged);
        let untracked = matches!(section, Section::Untracked);
        for f in *files {
            out.push(ProjectDiffFile {
                path: f.path.clone(),
                staged,
                untracked,
            });
        }
    }
    out
}

/// Tint for a status letter (independent of the staging checkbox).
fn status_color(letter: char, c: Colors) -> gpui::Hsla {
    match letter {
        'A' | '?' => c.success,
        'D' => c.error,
        'M' | 'R' | 'C' => c.modified,
        'U' => c.warning,
        _ => c.muted,
    }
}

/// Render one [`GitListEntry`] into an element. Free function so it can run
/// inside the `uniform_list` render closure (only `&mut Window, &mut App`);
/// every handler goes through `view.update(..)`.
fn git_list_element(
    entry: &GitListEntry,
    c: Colors,
    row_h: Pixels,
    view: &Entity<GitPanelView>,
) -> gpui::AnyElement {
    match entry {
        GitListEntry::EmptyState => div()
            .flex()
            .items_center()
            .h(row_h)
            .px(px(12.0))
            .text_size(px(11.0))
            .text_color(c.muted)
            .child(SharedString::from("No changes"))
            .into_any_element(),
        GitListEntry::SectionHeader {
            section,
            title,
            count,
            stage,
            collapsed,
        } => {
            let section = *section;
            let key = section as u8;
            let v = view.clone();
            let toggle_v = view.clone();
            let label = format!("{} ({})", title.to_uppercase(), count);
            // Section-level tri-state staging checkbox (Zed-parity Phase 4,
            // §9.5 "Change rows"). Reuses the file row's staging control.
            let box_row = git_change_row(
                SharedString::from(format!("git-sec-cb-{title}")),
                c.palette,
                *stage,
                SharedString::default(),
            )
            .on_toggle_stage(move |want_staged: &bool, _w, cx| {
                let want_staged = *want_staged;
                toggle_v.update(cx, |this, cx| this.stage_section(section, want_staged, cx));
            });
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .h(row_h)
                .px(px(8.0))
                .child(
                    div()
                        .flex_none()
                        .w(px(24.0))
                        .overflow_hidden()
                        .child(box_row),
                )
                .child(
                    disclosure(
                        SharedString::from(format!("git-sec-{title}")),
                        SharedString::from(label),
                        *collapsed,
                        c.muted,
                        c.fg,
                    )
                    .text_size(px(10.0))
                    .on_click(move |_: &ClickEvent, _w, cx| {
                        v.update(cx, |this, cx| {
                            if this.collapsed.contains(&key) {
                                this.collapsed.remove(&key);
                            } else {
                                this.collapsed.insert(key);
                            }
                            cx.notify();
                        });
                    }),
                )
                .into_any_element()
        }
        GitListEntry::Directory {
            section,
            path,
            name,
            depth,
            stage,
            collapsed,
            paths,
        } => {
            let section = *section;
            let dir_path = path.clone();
            let toggle_v = view.clone();
            let toggle_paths = paths.clone();
            let coll_v = view.clone();
            git_change_row(
                SharedString::from(format!("git-dir-{}-{path}", section as u8)),
                c.palette,
                *stage,
                SharedString::from(format!("{name}/")),
            )
            .depth(*depth)
            .icon(if *collapsed {
                IconName::Folder
            } else {
                IconName::FolderOpen
            })
            .tooltip(SharedString::from(path.clone()))
            .on_toggle_stage(move |want_staged: &bool, _w, cx| {
                let want_staged = *want_staged;
                let paths = toggle_paths.clone();
                toggle_v.update(cx, |this, cx| {
                    if want_staged {
                        this.stage_paths(paths.clone(), cx);
                    } else {
                        this.unstage_paths(paths.clone(), cx);
                    }
                });
            })
            .on_click(move |_: &ClickEvent, _w, cx| {
                let dir_path = dir_path.clone();
                coll_v.update(cx, |this, cx| {
                    if this.dir_collapsed.contains(&dir_path) {
                        this.dir_collapsed.remove(&dir_path);
                    } else {
                        this.dir_collapsed.insert(dir_path.clone());
                    }
                    cx.notify();
                });
            })
            .into_any_element()
        }
        GitListEntry::File {
            section,
            path,
            depth,
            letter,
            staged,
            untracked,
            selected,
        } => {
            let section = *section;
            let letter = *letter;
            let lc = status_color(letter, c);
            let stage = if *staged {
                StageState::Staged
            } else {
                StageState::Unstaged
            };
            let id = format!("git-file-{}-{}", section as u8, path);
            let can_discard = matches!(section, Section::Unstaged | Section::Conflicts);

            let actions = can_discard.then(|| {
                let v = view.clone();
                let p = path.clone();
                labonair_ui_kit::button_no_hover(
                    SharedString::from(format!("discard-{id}")),
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .text_color(c.muted)
                .hover(|s| s.text_color(c.error))
                .child(IconName::Refresh.svg(c.muted).size(px(11.0)))
                .on_click(move |_: &ClickEvent, _w, cx| {
                    cx.stop_propagation();
                    v.update(cx, |this, cx| this.discard_file(p.clone(), cx));
                })
            });

            let sel = Selected {
                path: path.clone(),
                staged: *staged,
                untracked: *untracked,
            };
            let click_v = view.clone();
            let menu_v = view.clone();
            let menu_p = path.clone();
            let toggle_v = view.clone();
            let toggle_p = path.clone();

            let mut row = git_change_row(
                SharedString::from(id),
                c.palette,
                stage,
                SharedString::from(short_path(path)),
            )
            .depth(*depth)
            .status(letter.to_string(), lc)
            .selected(*selected)
            .secondary(SharedString::from(dir_prefix(path)))
            .tooltip(SharedString::from(path.clone()))
            .on_toggle_stage(move |want_staged: &bool, _w, cx| {
                let want_staged = *want_staged;
                toggle_v.update(cx, |this, cx| {
                    if want_staged {
                        this.stage_file(toggle_p.clone(), cx);
                    } else {
                        this.unstage_file(toggle_p.clone(), cx);
                    }
                });
            })
            .on_click(move |_: &ClickEvent, _w, cx| {
                click_v.update(cx, |this, cx| this.select_file(sel.clone(), cx));
            })
            .on_secondary_down(move |ev: &MouseDownEvent, _w, cx| {
                menu_v.update(cx, |this, cx| {
                    this.file_menu = Some((menu_p.clone(), section, ev.position));
                    cx.notify();
                });
            });
            if let Some(a) = actions {
                row = row.actions(a);
            }
            row.into_any_element()
        }
    }
}

/// Which hand-rolled text field currently receives key events (only one is
/// ever active — the panel routes `on_key_down` to it via [`GitPanelView::on_field_key`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    BranchFilter,
    NewBranchName,
    NewBranchFrom,
    TagName,
    TagMessage,
    TagFrom,
    StashMsg,
    Rename,
}

/// The currently selected file (highlighted in the list; focused in the
/// workspace Project Diff item).
#[derive(Clone, PartialEq, Eq)]
struct Selected {
    path: String,
    /// `true` → diff index↔HEAD; `false` → diff worktree↔index.
    staged: bool,
    untracked: bool,
}

/// Events the panel emits for the shell to translate into workspace actions —
/// keeps `labonair-workspace` free of a dependency on this crate (§12.6).
#[derive(Clone, Debug)]
pub enum ScmEvent {
    /// Open / focus the single workspace Project Diff item.
    OpenProjectDiff(ProjectDiffRequest),
}

/// Which transient header/footer popover menu is open, and where.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PanelMenu {
    ViewOptions,
    Overflow,
    Repo,
}

/// A pending destructive action awaiting an explicit in-panel confirmation
/// (Zed-parity Phase 4, §9.5 — destructive actions are never a primary button).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingConfirm {
    DiscardAll,
    Clean,
    ForcePush,
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
    palette: Palette,
}

pub struct GitPanelView {
    backend: Backend,
    tokio: TokioHandle,
    theme: Entity<ThemeStore>,
    focus: FocusHandle,

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
    /// Identity of the repo-wide op in flight (progress / disable derivation).
    repo_op: RepoOperation,

    /// Panel information mode — `Changes` list vs `History` commit log.
    mode: PanelMode,
    /// Tree vs flat presentation of the change list.
    file_tree: bool,

    collapsed: std::collections::HashSet<u8>,
    /// Collapsed directory nodes (tree presentation), keyed by dir path.
    dir_collapsed: std::collections::HashSet<String>,

    selected: Option<Selected>,
    /// Open source-control file right-click menu: `(path, section, cursor)`.
    file_menu: Option<(String, Section, Point<Pixels>)>,
    /// Open header/footer popover menu: `(which, anchor)`.
    panel_menu: Option<(PanelMenu, Point<Pixels>)>,
    /// A destructive action awaiting confirmation.
    pending_confirm: Option<PendingConfirm>,

    // ── commit composer (editor-backed, Zed-parity Phase 4) ──
    /// Real text input; created lazily in `render` (needs a `Window`).
    commit_input: Option<Entity<InputState>>,
    /// Seed text before the input exists (tests / first paint).
    commit_seed: String,
    commit_error: Option<String>,
    /// `--amend` toggle.
    amend: bool,
    /// Expanded (taller) composer.
    commit_expanded: bool,

    // ── history mode ──
    history: Vec<CommitInfo>,
    history_loading: bool,

    // ── branch picker (port of BranchDropdown) ──
    branch_picker_open: bool,
    branch_filter: String,
    checkout_error: Option<String>,
    remotes_collapsed: bool,
    /// Branch pending a plain delete confirmation.
    delete_confirm_branch: Option<String>,
    /// Branch pending a *force* delete confirmation (was not fully merged).
    force_delete_branch: Option<String>,
    /// Branch currently being renamed inline (its row shows a text field).
    rename_target: Option<String>,
    rename_buf: String,

    // ── new-branch form ──
    new_branch_open: bool,
    new_branch_name: String,
    new_branch_from: String,
    new_branch_checkout: bool,
    new_branch_error: Option<String>,

    // ── tags (port of TagSection) ──
    tags_collapsed: bool,
    new_tag_open: bool,
    new_tag_name: String,
    new_tag_message: String,
    new_tag_from: String,
    tag_error: Option<String>,
    delete_confirm_tag: Option<String>,

    // ── stash (port of StashPanel) ──
    stash_collapsed: bool,
    stash_form_open: bool,
    stash_msg: String,
    /// Stash entry (index, hash) pending a drop confirmation.
    drop_confirm_stash: Option<(u32, String)>,

    active_field: Option<Field>,
}

impl Focusable for GitPanelView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EventEmitter<ScmEvent> for GitPanelView {}

impl GitPanelView {
    pub fn new(
        backend: Backend,
        tokio: TokioHandle,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        // Pick up external edits to `scmFileTree`.
        if cx.has_global::<labonair_settings::SettingsStore>() {
            cx.observe_global::<labonair_settings::SettingsStore>(|this, cx| {
                this.file_tree = scm_file_tree_setting(cx);
                cx.notify();
            })
            .detach();
        }
        let file_tree = scm_file_tree_setting(cx);

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
            repo_op: RepoOperation::Idle,
            mode: PanelMode::Changes,
            file_tree,
            collapsed: std::collections::HashSet::new(),
            dir_collapsed: std::collections::HashSet::new(),
            selected: None,
            file_menu: None,
            panel_menu: None,
            pending_confirm: None,
            commit_input: None,
            commit_seed: String::new(),
            commit_error: None,
            amend: false,
            commit_expanded: false,
            history: Vec::new(),
            history_loading: false,
            branch_picker_open: false,
            branch_filter: String::new(),
            checkout_error: None,
            remotes_collapsed: true,
            delete_confirm_branch: None,
            force_delete_branch: None,
            rename_target: None,
            rename_buf: String::new(),
            new_branch_open: false,
            new_branch_name: String::new(),
            new_branch_from: String::new(),
            new_branch_checkout: true,
            new_branch_error: None,
            tags_collapsed: true,
            new_tag_open: false,
            new_tag_name: String::new(),
            new_tag_message: String::new(),
            new_tag_from: String::new(),
            tag_error: None,
            delete_confirm_tag: None,
            stash_collapsed: true,
            stash_form_open: false,
            stash_msg: String::new(),
            drop_confirm_stash: None,
            active_field: None,
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
            // Target changed — clear stale selection + history.
            self.selected = None;
            self.history.clear();
            self.dir_collapsed.clear();
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
                            this.history.clear();
                        }
                        if this.mode == PanelMode::History {
                            this.load_history(cx);
                        }
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

    // ── Project Diff (workspace item) ─────────────────────────────────────

    /// The ordered change list for the current status, as Project-Diff files.
    fn change_files(&self) -> Vec<ProjectDiffFile> {
        let Some(state) = &self.state else {
            return Vec::new();
        };
        let b = bucketize(&state.status);
        let sections: Sections<'_> = [
            (Section::Conflicts, "Conflicts", &b.conflicted),
            (Section::Staged, "Staged", &b.staged),
            (Section::Unstaged, "Changes", &b.unstaged),
            (Section::Untracked, "Untracked", &b.untracked),
        ];
        all_change_files(&sections)
    }

    /// Emit a [`ScmEvent::OpenProjectDiff`] for the current change set, focusing
    /// `selected` if given. The shell forwards it to
    /// `Workspace::open_project_diff` (idempotent — never a duplicate tab).
    fn emit_project_diff(&mut self, selected: Option<String>, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            return;
        };
        let files = self.change_files();
        if files.is_empty() {
            return;
        }
        cx.emit(ScmEvent::OpenProjectDiff(ProjectDiffRequest {
            repo_root,
            session_id: self.session_id.clone(),
            files,
            selected,
            mode: ProjectDiffMode::Unified,
        }));
    }

    fn select_file(&mut self, sel: Selected, cx: &mut Context<Self>) {
        if self.selected.as_ref() == Some(&sel) {
            self.selected = None;
        } else {
            self.selected = Some(sel.clone());
            self.emit_project_diff(Some(sel.path), cx);
        }
        cx.notify();
    }

    fn set_mode(&mut self, mode: PanelMode, cx: &mut Context<Self>) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        if mode == PanelMode::History && self.history.is_empty() {
            self.load_history(cx);
        }
        cx.notify();
    }

    fn load_history(&mut self, cx: &mut Context<Self>) {
        let Some(repo_root) = self.repo_root.clone() else {
            return;
        };
        if self.history_loading {
            return;
        }
        self.history_loading = true;
        let session = self.session_id.clone();
        let backend = self.backend.clone();
        let generation = self.target_gen;
        let jh = self.tokio.spawn(async move {
            git::git_get_log(
                repo_root,
                Some(100),
                false,
                session,
                None,
                &backend.ssh,
                backend.clone(),
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.history_loading = false;
                if this.target_gen != generation {
                    return;
                }
                match res {
                    Ok(commits) => this.history = commits,
                    Err(e) => {
                        notify_err::<()>("Load history failed", Err(e), cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    // ── generic backend-op dispatch ────────────────────────────────────────

    /// Runs `op` on the tokio runtime, toasts any error, then refreshes. The
    /// op is tagged with a [`RepoOperation`] identity so the affected control
    /// can show progress / be disabled without freezing the whole panel.
    fn run_op_kind<F>(
        &mut self,
        kind: RepoOperation,
        title: &'static str,
        op: F,
        cx: &mut Context<Self>,
    ) where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        if self.op_in_progress {
            return;
        }
        self.op_in_progress = true;
        self.repo_op = kind;
        cx.notify();
        let jh = self.tokio.spawn(op);
        cx.spawn(async move |this, cx| {
            let res = jh.await.unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |this, cx| {
                this.op_in_progress = false;
                this.repo_op = RepoOperation::Idle;
                notify_err(title, res, cx);
                this.refresh_soon(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// [`run_op_kind`] with the generic `Mutating` identity.
    fn run_op<F>(&mut self, title: &'static str, op: F, cx: &mut Context<Self>)
    where
        F: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        self.run_op_kind(RepoOperation::Mutating, title, op, cx);
    }

    /// Stage every path in `paths` in one sequential op (section / directory
    /// aggregate checkbox).
    fn stage_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Stage failed",
            async move {
                for p in paths {
                    git::git_stage_file(root.clone(), p, sid.clone(), &be.ssh, be.clone()).await?;
                }
                Ok(())
            },
            cx,
        );
    }

    /// Unstage every path in `paths` in one sequential op.
    fn unstage_paths(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Unstage failed",
            async move {
                for p in paths {
                    git::git_unstage_file(root.clone(), p, sid.clone(), &be.ssh, be.clone())
                        .await?;
                }
                Ok(())
            },
            cx,
        );
    }

    /// Toggle a whole section's staging (header checkbox).
    fn stage_section(&mut self, section: Section, want_staged: bool, cx: &mut Context<Self>) {
        let Some(state) = &self.state else {
            return;
        };
        let b = bucketize(&state.status);
        let files: &[FileStatus] = match section {
            Section::Conflicts => &b.conflicted,
            Section::Staged => &b.staged,
            Section::Unstaged => &b.unstaged,
            Section::Untracked => &b.untracked,
        };
        let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        if paths.is_empty() {
            return;
        }
        // The checkbox already reports the desired next state: `true` for the
        // Unstaged / Untracked / Conflicts sections, `false` for the Staged
        // section (whose box is "on").
        if want_staged {
            self.stage_paths(paths, cx);
        } else {
            self.unstage_paths(paths, cx);
        }
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

    // ── commit / sync ──────────────────────────────────────────────────────

    /// Aggregate staging counts + whether tracked files are dirty. Drives the
    /// adaptive commit mode / disabled reason.
    fn commit_state(&self) -> (usize, bool, bool) {
        let Some(s) = &self.state else {
            return (0, false, false);
        };
        let b = bucketize(&s.status);
        let staged = b.staged.len();
        let tracked_dirty = !b.unstaged.is_empty();
        let has_conflicts = !b.conflicted.is_empty();
        (staged, tracked_dirty, has_conflicts)
    }

    fn commit_mode(&self) -> CommitMode {
        let (staged, tracked_dirty, _) = self.commit_state();
        derive_commit_mode(self.amend, staged, tracked_dirty)
    }

    fn commit_disabled_reason(&self, message_empty: bool) -> Option<DisabledReason> {
        let (staged, tracked_dirty, has_conflicts) = self.commit_state();
        derive_disabled_reason(
            message_empty,
            staged,
            tracked_dirty,
            has_conflicts,
            self.repo_op,
            self.amend,
        )
    }

    fn do_commit(&mut self, msg: String, window: &mut Window, cx: &mut Context<Self>) {
        let msg = msg.trim().to_string();
        if self.commit_disabled_reason(msg.is_empty()).is_some() {
            return;
        }
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let mode = self.commit_mode();
        let amend = matches!(mode, CommitMode::Amend);
        // `CommitTracked` stages tracked-file changes first.
        let stage_tracked = matches!(mode, CommitMode::CommitTracked);
        self.commit_error = None;
        self.commit_seed.clear();
        if let Some(input) = self.commit_input.clone() {
            input.update(cx, |s, cx| s.set_value("", window, cx));
        }
        self.amend = false;
        self.run_op(
            "Commit failed",
            async move {
                if stage_tracked {
                    git::git_stage_all(root.clone(), sid.clone(), &be.ssh, be.clone()).await?;
                }
                git::git_commit(root, msg, amend, sid, &be.ssh, be.clone())
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
        self.run_op_kind(
            RepoOperation::Pulling,
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
            self.run_op_kind(
                RepoOperation::Pushing,
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
            self.run_op_kind(
                RepoOperation::Pushing,
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

    /// Force-push. Only reached after an explicit in-panel confirmation
    /// (`PendingConfirm::ForcePush`) — never a bare primary button.
    fn force_push(&mut self, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let branch = self
            .state
            .as_ref()
            .map(|s| s.current_branch.clone())
            .unwrap_or_default();
        self.run_op_kind(
            RepoOperation::Pushing,
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
        self.run_op_kind(
            RepoOperation::Fetching,
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

    // ── branch / tag / stash operations ────────────────────────────────────

    /// Like [`run_op`], but hands the raw `Result` to `done` instead of always
    /// toasting — lets callers surface inline errors / drive follow-up state.
    fn dispatch<F>(
        &mut self,
        op: F,
        done: impl FnOnce(&mut Self, Result<(), String>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) where
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
                done(this, res, cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// `(name, is_current, is_remote)` for every branch — for the command
    /// palette's `Git: Switch Branch…` sub-page.
    pub fn branch_choices(&self) -> Vec<(String, bool, bool)> {
        let Some(state) = &self.state else {
            return Vec::new();
        };
        state
            .branches
            .iter()
            .map(|b| (b.name.clone(), b.name == state.current_branch, b.is_remote))
            .collect()
    }

    /// Checkout a branch by name (palette entry point).
    pub fn checkout(&mut self, name: String, cx: &mut Context<Self>) {
        self.checkout_branch(name, cx);
    }

    fn checkout_branch(&mut self, name: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.checkout_error = None;
        let name2 = name.clone();
        self.dispatch(
            async move { git::git_checkout_branch(root, name2, sid, &be.ssh, be.clone()).await },
            move |this, res, cx| match res {
                Ok(()) => {
                    this.checkout_error = None;
                    this.branch_picker_open = false;
                    this.refresh_soon(cx);
                }
                Err(e) => {
                    let msg = map_checkout_error(&e);
                    this.checkout_error = Some(msg.clone());
                    notify_err::<()>("Checkout failed", Err(msg), cx);
                }
            },
            cx,
        );
    }

    fn create_branch(&mut self, cx: &mut Context<Self>) {
        let name = self.new_branch_name.trim().to_string();
        if name.is_empty() {
            self.new_branch_error = Some("Branch name is required.".to_string());
            cx.notify();
            return;
        }
        let from = {
            let f = self.new_branch_from.trim();
            if f.is_empty() {
                None
            } else {
                Some(f.to_string())
            }
        };
        let checkout = self.new_branch_checkout;
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.new_branch_error = None;
        self.dispatch(
            async move {
                git::git_create_branch(root, name, from, checkout, sid, &be.ssh, be.clone()).await
            },
            move |this, res, cx| match res {
                Ok(()) => {
                    this.new_branch_open = false;
                    this.new_branch_name.clear();
                    this.new_branch_from.clear();
                    this.active_field = None;
                    this.refresh_soon(cx);
                }
                Err(e) => this.new_branch_error = Some(e),
            },
            cx,
        );
    }

    fn delete_branch(&mut self, name: String, force: bool, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        let name2 = name.clone();
        self.dispatch(
            async move {
                git::git_delete_branch(root, name2, force, sid, &be.ssh, be.clone()).await
            },
            move |this, res, cx| match res {
                Ok(()) => {
                    this.delete_confirm_branch = None;
                    this.force_delete_branch = None;
                    this.refresh_soon(cx);
                }
                Err(e) if !force && is_unmerged_branch_error(&e) => {
                    this.delete_confirm_branch = None;
                    this.force_delete_branch = Some(name);
                }
                Err(e) => {
                    this.delete_confirm_branch = None;
                    this.force_delete_branch = None;
                    this.checkout_error = Some(format!("Could not delete branch: {e}"));
                    notify_err::<()>("Delete branch failed", Err(e), cx);
                }
            },
            cx,
        );
    }

    fn rename_branch(&mut self, cx: &mut Context<Self>) {
        let Some(old) = self.rename_target.clone() else {
            return;
        };
        let new_name = self.rename_buf.trim().to_string();
        if new_name.is_empty() || new_name == old {
            self.rename_target = None;
            self.active_field = None;
            cx.notify();
            return;
        }
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move {
                git::git_rename_branch(root, old, new_name, sid, &be.ssh, be.clone()).await
            },
            move |this, res, cx| {
                this.rename_target = None;
                this.active_field = None;
                notify_err("Rename branch failed", res, cx);
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    fn create_tag(&mut self, cx: &mut Context<Self>) {
        let name = self.new_tag_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let message = {
            let m = self.new_tag_message.trim();
            (!m.is_empty()).then(|| m.to_string())
        };
        let from = {
            let f = self.new_tag_from.trim();
            (!f.is_empty()).then(|| f.to_string())
        };
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.tag_error = None;
        self.dispatch(
            async move {
                git::git_create_tag(root, name, message, from, sid, &be.ssh, be.clone()).await
            },
            move |this, res, cx| match res {
                Ok(()) => {
                    this.new_tag_open = false;
                    this.new_tag_name.clear();
                    this.new_tag_message.clear();
                    this.new_tag_from.clear();
                    this.active_field = None;
                    this.refresh_soon(cx);
                }
                Err(e) => this.tag_error = Some(e),
            },
            cx,
        );
    }

    fn delete_tag(&mut self, name: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move { git::git_delete_tag(root, name, sid, &be.ssh, be.clone()).await },
            move |this, res, cx| {
                this.delete_confirm_tag = None;
                if let Err(e) = &res {
                    this.tag_error = Some(e.clone());
                }
                notify_err("Delete tag failed", res, cx);
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    fn push_tag(&mut self, name: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move {
                git::git_push_tag(root, name, None, sid, &be.ssh, be.clone())
                    .await
                    .map(|_| ())
            },
            move |this, res, cx| {
                if let Err(e) = &res {
                    this.tag_error = Some(e.clone());
                }
                notify_err("Push tag failed", res, cx);
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    fn stash_push(&mut self, cx: &mut Context<Self>) {
        let message = {
            let m = self.stash_msg.trim();
            (!m.is_empty()).then(|| m.to_string())
        };
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move { git::git_stash_push(root, message, None, sid, &be.ssh, be.clone()).await },
            move |this, res, cx| {
                this.stash_form_open = false;
                this.stash_msg.clear();
                this.active_field = None;
                notify_err("Stash failed", res, cx);
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    /// `pop == true` → `git stash pop`; otherwise `git stash apply`.
    fn stash_apply(&mut self, hash: String, pop: bool, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move {
                if pop {
                    git::git_stash_pop(root, hash, sid, &be.ssh, be.clone()).await
                } else {
                    git::git_stash_apply(root, hash, sid, &be.ssh, be.clone()).await
                }
            },
            move |this, res, cx| {
                if let Err(e) = res {
                    notify_err::<()>(
                        if pop {
                            "Stash pop failed"
                        } else {
                            "Stash apply failed"
                        },
                        Err(stash_conflict_message(&e)),
                        cx,
                    );
                }
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    fn stash_drop(&mut self, hash: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.dispatch(
            async move { git::git_stash_drop(root, hash, sid, &be.ssh, be.clone()).await },
            move |this, res, cx| {
                this.drop_confirm_stash = None;
                notify_err("Stash drop failed", res, cx);
                this.refresh_soon(cx);
            },
            cx,
        );
    }

    // ── hand-rolled text-field routing ─────────────────────────────────────

    fn field_buf_mut(&mut self, f: Field) -> &mut String {
        match f {
            Field::BranchFilter => &mut self.branch_filter,
            Field::NewBranchName => &mut self.new_branch_name,
            Field::NewBranchFrom => &mut self.new_branch_from,
            Field::TagName => &mut self.new_tag_name,
            Field::TagMessage => &mut self.new_tag_message,
            Field::TagFrom => &mut self.new_tag_from,
            Field::StashMsg => &mut self.stash_msg,
            Field::Rename => &mut self.rename_buf,
        }
    }

    fn field_value(&self, f: Field) -> &str {
        match f {
            Field::BranchFilter => &self.branch_filter,
            Field::NewBranchName => &self.new_branch_name,
            Field::NewBranchFrom => &self.new_branch_from,
            Field::TagName => &self.new_tag_name,
            Field::TagMessage => &self.new_tag_message,
            Field::TagFrom => &self.new_tag_from,
            Field::StashMsg => &self.stash_msg,
            Field::Rename => &self.rename_buf,
        }
    }

    fn submit_field(&mut self, f: Field, cx: &mut Context<Self>) {
        match f {
            Field::BranchFilter => {}
            Field::NewBranchName | Field::NewBranchFrom => self.create_branch(cx),
            Field::TagName | Field::TagMessage | Field::TagFrom => self.create_tag(cx),
            Field::StashMsg => self.stash_push(cx),
            Field::Rename => self.rename_branch(cx),
        }
    }

    fn cancel_field(&mut self, f: Field) {
        match f {
            Field::BranchFilter => self.branch_filter.clear(),
            Field::NewBranchName | Field::NewBranchFrom => {
                self.new_branch_open = false;
                self.new_branch_error = None;
            }
            Field::TagName | Field::TagMessage | Field::TagFrom => {
                self.new_tag_open = false;
                self.tag_error = None;
            }
            Field::StashMsg => {
                self.stash_form_open = false;
                self.stash_msg.clear();
            }
            Field::Rename => self.rename_target = None,
        }
        self.active_field = None;
    }

    fn on_field_key(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let Some(field) = self.active_field else {
            return;
        };
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => self.cancel_field(field),
            "enter" => self.submit_field(field, cx),
            "backspace" => {
                self.field_buf_mut(field).pop();
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
                    self.field_buf_mut(field).push_str(&ch);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    // ── commit composer (editor-backed) ───────────────────────────────────

    /// Create the `InputState` lazily (it needs a `Window`). Mirrors the AI
    /// composer's pattern: an `InputEvent::PressEnter { secondary }` commits on
    /// ⌘↵ / Ctrl↵.
    fn ensure_commit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.commit_input.is_some() {
            return;
        }
        let seed = std::mem::take(&mut self.commit_seed);
        let input = cx.new(|cx| {
            let mut s = InputState::new(window, cx)
                .multi_line(true)
                .auto_grow(2, 8)
                .placeholder("Message (\u{2318}\u{21A9} to commit)");
            if !seed.is_empty() {
                s.set_value(seed, window, cx);
            }
            s
        });
        let view = cx.entity();
        window
            .subscribe(&input, cx, move |input, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { secondary } = ev {
                    if !*secondary {
                        return;
                    }
                    let v = input.read(cx).value().to_string();
                    let trimmed = v.strip_suffix('\n').unwrap_or(&v).to_string();
                    view.update(cx, |this, cx| this.do_commit(trimmed, window, cx));
                }
            })
            .detach();
        self.commit_input = Some(input);
    }

    fn commit_text(&self, cx: &App) -> String {
        match &self.commit_input {
            Some(i) => i.read(cx).value().to_string(),
            None => self.commit_seed.clone(),
        }
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
            palette: Palette::from_theme(t),
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
        labonair_ui_kit::button_no_hover(id, c.palette, ButtonVariant::Ghost, ButtonSize::Xs)
            .text_color(c.muted)
            .hover(|s| s.bg(c.border).text_color(c.fg))
            .child(label.into())
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| on_click(this, w, cx)))
    }

    /// Consolidated repository footer (Zed-parity Phase 4, §9.5): branch,
    /// ahead/behind, and one compact repo menu (`⋯`) that holds Fetch / Pull /
    /// Push and — clearly separated and de-emphasised — Force Push, plus
    /// Stashes / Tags and any in-progress Continue / Abort.
    fn render_footer(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = &self.state else {
            return div().into_any_element();
        };
        let status = &state.status;
        let in_progress =
            status.merge_in_progress || status.rebase_in_progress || status.cherry_pick_in_progress;

        let mut bar = div().flex().flex_col().border_t_1().border_color(c.border);

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .h(px(28.0))
            .px(px(8.0))
            .text_size(px(11.0))
            .child(
                labonair_ui_kit::button_no_hover(
                    "git-branch-toggle",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::Xs,
                )
                .flex_1()
                .justify_start()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(c.fg)
                .hover(|s| s.text_color(c.accent))
                .child(SharedString::from(format!(
                    "\u{2325} {} \u{25BE}",
                    if state.current_branch.is_empty() {
                        "\u{2014}"
                    } else {
                        &state.current_branch
                    }
                )))
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.branch_picker_open = !this.branch_picker_open;
                    cx.notify();
                })),
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
        if self.repo_op.is_busy() {
            row = row.child(div().text_size(px(10.0)).text_color(c.muted).child(
                SharedString::from(match self.repo_op {
                    RepoOperation::Fetching => "fetching\u{2026}",
                    RepoOperation::Pulling => "pulling\u{2026}",
                    RepoOperation::Pushing => "pushing\u{2026}",
                    _ => "working\u{2026}",
                }),
            ));
        }
        row = row.child(
            labonair_ui_kit::button_no_hover(
                "git-repo-menu",
                c.palette,
                ButtonVariant::Ghost,
                ButtonSize::IconXs,
            )
            .text_color(c.muted)
            .hover(|s| s.text_color(c.fg))
            .child(SharedString::from("\u{22EF}"))
            .on_click(cx.listener(|this, ev: &ClickEvent, _w, cx| {
                this.panel_menu = Some((PanelMenu::Repo, ev.position()));
                cx.notify();
            })),
        );
        bar = bar.child(row);

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

    /// Editor-backed commit composer (Zed-parity Phase 4, §9.5). Compact by
    /// default, `⤢` grows it; adaptive `Commit` / `Commit Tracked` / `Amend`
    /// button with a tooltip explaining any disabled reason.
    fn render_commit_composer(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let text = self.commit_text(cx);
        let mode = self.commit_mode();
        let disabled = self.commit_disabled_reason(text.trim().is_empty());
        let disabled_desc = disabled.map(|d| d.describe());
        let title_len = text.lines().next().map(|l| l.chars().count()).unwrap_or(0);
        let over_title = title_len > 72;

        let input_el: gpui::AnyElement = match &self.commit_input {
            Some(input) => field_input(input).into_any_element(),
            None => div()
                .id("git-commit-input-seed")
                .min_h(px(48.0))
                .p(px(6.0))
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .bg(c.bg)
                .text_size(px(12.0))
                .text_color(c.muted)
                .child(SharedString::from("Message (\u{2318}\u{21A9} to commit)"))
                .into_any_element(),
        };

        let commit_label = SharedString::from(mode.label());
        let mut commit_btn = button(
            "git-commit-btn",
            c.palette,
            ButtonVariant::Default,
            ButtonSize::Sm,
        )
        .flex_1()
        .child(commit_label);
        if let Some(desc) = disabled_desc {
            // No `on_click` → inert; dimmed + tooltip explains why.
            let desc = SharedString::from(desc);
            commit_btn = commit_btn
                .opacity(0.5)
                .cursor_default()
                .tooltip(move |w, cx| labonair_ui_kit::Tooltip::new(desc.clone()).build(w, cx));
        } else {
            commit_btn = commit_btn.on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                let msg = this.commit_text(cx);
                this.do_commit(msg, w, cx);
            }));
        }

        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(8.0))
            .border_t_1()
            .border_color(c.border)
            .child(
                div()
                    .id("git-commit-box")
                    .when(self.commit_expanded, |d| d.min_h(px(160.0)))
                    .child(input_el),
            )
            .when(over_title, |d| {
                d.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.warning)
                        .child(SharedString::from(format!(
                            "Title is {title_len} chars (recommended \u{2264} 72)"
                        ))),
                )
            })
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
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        checkbox("git-amend", c.palette, self.amend)
                            .label("Amend")
                            .on_click(cx.listener(|this, _: &bool, _w, cx| {
                                this.amend = !this.amend;
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1())
                    .child(
                        labonair_ui_kit::button_no_hover(
                            "git-commit-expand",
                            c.palette,
                            ButtonVariant::Ghost,
                            ButtonSize::IconXs,
                        )
                        .text_color(c.muted)
                        .hover(|s| s.text_color(c.fg))
                        .child(SharedString::from("\u{2922}"))
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.commit_expanded = !this.commit_expanded;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(div().flex().child(commit_btn))
            .into_any_element()
    }

    // ── branch picker / stash rendering ────────────────────────────────────

    /// A hand-rolled single-line text field (GPUI has no built-in text input
    /// here — the panel routes key events to the [`Field`] marked active).
    fn text_field(
        &self,
        id: &'static str,
        field: Field,
        placeholder: &'static str,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_field == Some(field);
        let value = self.field_value(field).to_string();
        let empty = value.is_empty();
        div()
            .id(id)
            .h(px(22.0))
            .px(px(6.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .whitespace_nowrap()
            .rounded_sm()
            .border_1()
            .border_color(if active { c.accent } else { c.border })
            .bg(c.bg)
            .text_size(px(11.0))
            .text_color(if empty { c.muted } else { c.fg })
            .child(SharedString::from(if empty {
                placeholder.to_string()
            } else {
                value
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                cx.stop_propagation();
                this.active_field = Some(field);
                w.focus(&this.focus);
                cx.notify();
            }))
    }

    fn confirm_bar(
        &self,
        text: String,
        c: Colors,
        actions: Vec<(&'static str, &'static str, gpui::Hsla)>,
        cx: &mut Context<Self>,
        on_action: impl Fn(&mut Self, &'static str, &mut Context<Self>) + 'static + Clone,
    ) -> gpui::AnyElement {
        let mut row = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(4.0))
            .bg(c.warning.opacity(0.10))
            .text_size(px(10.0))
            .text_color(c.warning)
            .child(div().flex_1().child(SharedString::from(text)));
        for (id, label, color) in actions {
            let cb = on_action.clone();
            row = row.child(
                labonair_ui_kit::button_no_hover(
                    id,
                    c.palette,
                    ButtonVariant::Outline,
                    ButtonSize::Xs,
                )
                .h(px(18.0))
                .border_color(color)
                .text_color(color)
                .hover(|s| s.bg(color.opacity(0.15)))
                .child(SharedString::from(label))
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    cx.stop_propagation();
                    cb(this, id, cx);
                })),
            );
        }
        row.into_any_element()
    }

    fn render_branch_picker(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        if !self.branch_picker_open {
            return div().into_any_element();
        }
        let Some(state) = &self.state else {
            return div().into_any_element();
        };
        let current = state.current_branch.clone();
        let locals = filter_branches(&state.branches, &self.branch_filter, false);
        let remotes = filter_branches(&state.branches, &self.branch_filter, true);
        let filtering = !self.branch_filter.trim().is_empty();

        let mut body = div()
            .id("git-branch-picker")
            .flex()
            .flex_col()
            .max_h(px(320.0))
            .overflow_y_scroll()
            .border_t_1()
            .border_color(c.border)
            .bg(c.card);

        // Filter + New Branch.
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(6.0))
                .border_b_1()
                .border_color(c.border)
                .child(self.text_field(
                    "git-branch-filter",
                    Field::BranchFilter,
                    "Filter branches\u{2026}",
                    c,
                    cx,
                ))
                .child(self.tool_btn(
                    "git-new-branch-toggle",
                    "+ New Branch\u{2026}",
                    c,
                    cx,
                    |this, w, cx| {
                        this.new_branch_open = !this.new_branch_open;
                        if this.new_branch_open {
                            this.new_branch_from = resolve_default_from_ref(
                                &this
                                    .state
                                    .as_ref()
                                    .map(|s| s.current_branch.clone())
                                    .unwrap_or_default(),
                            );
                            this.active_field = Some(Field::NewBranchName);
                            w.focus(&this.focus);
                        }
                        cx.notify();
                    },
                )),
        );

        if let Some(err) = &self.checkout_error {
            body = body.child(
                div()
                    .px(px(8.0))
                    .py(px(4.0))
                    .bg(c.error.opacity(0.10))
                    .text_size(px(10.0))
                    .text_color(c.error)
                    .child(SharedString::from(err.clone())),
            );
        }

        if self.new_branch_open {
            body = body.child(self.render_new_branch_form(c, cx));
        }

        // Plain / force delete confirmations.
        if let Some(name) = self.delete_confirm_branch.clone() {
            body = body.child(self.confirm_bar(
                format!("Delete branch '{name}'?"),
                c,
                vec![
                    ("del-yes", "Delete", c.error),
                    ("del-no", "Cancel", c.muted),
                ],
                cx,
                move |this, id, cx| {
                    if id == "del-yes" {
                        this.delete_branch(name.clone(), false, cx);
                    } else {
                        this.delete_confirm_branch = None;
                        cx.notify();
                    }
                },
            ));
        }
        if let Some(name) = self.force_delete_branch.clone() {
            body = body.child(self.confirm_bar(
                format!(
                    "'{name}' is not fully merged \u{2014} force delete (discards its commits)?"
                ),
                c,
                vec![
                    ("fdel-yes", "Force delete", c.error),
                    ("fdel-no", "Cancel", c.muted),
                ],
                cx,
                move |this, id, cx| {
                    if id == "fdel-yes" {
                        this.delete_branch(name.clone(), true, cx);
                    } else {
                        this.force_delete_branch = None;
                        cx.notify();
                    }
                },
            ));
        }

        // Local branches.
        body = body.child(section_label("Local Branches", c));
        if locals.is_empty() {
            body = body.child(picker_empty(
                if filtering {
                    "No matching branches"
                } else {
                    "No local branches"
                },
                c,
            ));
        } else {
            for b in &locals {
                body = body.child(self.render_branch_row(b, &current, c, cx));
            }
        }

        // Remote branches.
        if !remotes.is_empty() {
            let expanded = filtering || !self.remotes_collapsed;
            body = body.child(
                div()
                    .h(px(22.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(c.border)
                    .child(
                        disclosure(
                            "git-remotes-hdr",
                            SharedString::from(format!("REMOTE ({})", remotes.len())),
                            !expanded,
                            c.muted,
                            c.fg,
                        )
                        .text_size(px(10.0))
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _w, cx| {
                                this.remotes_collapsed = !this.remotes_collapsed;
                                cx.notify();
                            },
                        )),
                    ),
            );
            if expanded {
                for b in &remotes {
                    body = body.child(self.render_branch_row(b, &current, c, cx));
                }
            }
        }

        // Tags.
        body = body.child(self.render_tag_section(c, cx));

        body.into_any_element()
    }

    fn render_new_branch_form(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .m(px(6.0))
            .p(px(6.0))
            .rounded_sm()
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .child(self.text_field(
                "git-nb-name",
                Field::NewBranchName,
                "Branch name (required)",
                c,
                cx,
            ))
            .child(self.text_field(
                "git-nb-from",
                Field::NewBranchFrom,
                "From (default HEAD)",
                c,
                cx,
            ))
            .child(
                checkbox("git-nb-checkout", c.palette, self.new_branch_checkout)
                    .label("Checkout after create")
                    .on_click(cx.listener(|this, _: &bool, _w, cx| {
                        this.new_branch_checkout = !this.new_branch_checkout;
                        cx.notify();
                    })),
            )
            .when_some(self.new_branch_error.clone(), |d, err| {
                d.child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.error)
                        .child(SharedString::from(err)),
                )
            })
            .child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .child(
                        self.tool_btn("git-nb-create", "Create", c, cx, |this, _w, cx| {
                            this.create_branch(cx)
                        }),
                    )
                    .child(
                        self.tool_btn("git-nb-cancel", "Cancel", c, cx, |this, _w, cx| {
                            this.new_branch_open = false;
                            this.new_branch_error = None;
                            this.active_field = None;
                            cx.notify();
                        }),
                    ),
            )
            .into_any_element()
    }

    fn render_branch_row(
        &self,
        b: &Branch,
        current: &str,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let is_current = !b.is_remote && b.name == current;
        let name = b.name.clone();

        if self.rename_target.as_deref() == Some(name.as_str()) {
            return div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .px(px(8.0))
                .py(px(3.0))
                .child(div().flex_1().child(self.text_field(
                    "git-rename",
                    Field::Rename,
                    "New name",
                    c,
                    cx,
                )))
                .child(
                    self.tool_btn("git-rename-ok", "Rename", c, cx, |this, _w, cx| {
                        this.rename_branch(cx)
                    }),
                )
                .child(
                    self.tool_btn("git-rename-x", "Cancel", c, cx, |this, _w, cx| {
                        this.rename_target = None;
                        this.active_field = None;
                        cx.notify();
                    }),
                )
                .into_any_element();
        }

        let meta = [b.author.clone(), b.committed_relative.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" \u{2022} ");
        let co_name = name.clone();
        let rn_name = name.clone();
        let del_name = name.clone();

        let mut trailing = h_stack().gap(px(4.0));
        if !b.is_remote {
            trailing = trailing.child(
                labonair_ui_kit::button_no_hover(
                    SharedString::from(format!("git-branch-rn-{}", name)),
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .text_color(c.muted)
                .hover(|s| s.text_color(c.fg))
                .child(IconName::Pencil.svg(c.muted).size(px(11.0)))
                .on_click(cx.listener(move |this, _: &ClickEvent, w, cx| {
                    cx.stop_propagation();
                    this.rename_target = Some(rn_name.clone());
                    this.rename_buf = rn_name.clone();
                    this.active_field = Some(Field::Rename);
                    w.focus(&this.focus);
                    cx.notify();
                })),
            );
            if !is_current {
                trailing = trailing.child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-branch-del-{}", name)),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.error))
                    .child(SharedString::from("\u{2715}"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            cx.stop_propagation();
                            this.delete_confirm_branch = Some(del_name.clone());
                            cx.notify();
                        },
                    )),
                );
            }
        }

        let on_click = cx.listener(move |this, _: &ClickEvent, _w, cx| {
            if !is_current {
                this.checkout_branch(co_name.clone(), cx);
            }
        });

        ListItem::new(
            SharedString::from(format!("git-branch-{}", name)),
            c.fg,
            c.muted,
            c.border,
        )
        .child(
            div()
                .w(px(10.0))
                .flex_shrink_0()
                .text_color(c.success)
                .text_size(px(10.0))
                .child(SharedString::from(if is_current { "\u{2713}" } else { "" })),
        )
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .whitespace_nowrap()
                        .child(SharedString::from(name.clone()))
                        .when(b.ahead > 0, |d| {
                            d.child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(c.success)
                                    .child(SharedString::from(format!("\u{2191}{}", b.ahead))),
                            )
                        })
                        .when(b.behind > 0, |d| {
                            d.child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(c.error)
                                    .child(SharedString::from(format!("\u{2193}{}", b.behind))),
                            )
                        }),
                )
                .when(!meta.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.muted)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(meta.clone())),
                    )
                }),
        )
        .trailing(trailing)
        .extra(move |mut row| {
            row = row.px(px(8.0)).py(px(3.0)).text_size(px(12.0));
            if is_current {
                row = row.bg(c.accent);
            }
            row.on_click(on_click)
        })
        .into_any_element()
    }

    fn render_tag_section(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = &self.state else {
            return div().into_any_element();
        };
        let tags = state.tags.clone();
        let mut wrap = div().flex().flex_col().border_t_1().border_color(c.border);

        wrap = wrap.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(px(22.0))
                .px(px(8.0))
                .child(
                    disclosure(
                        "git-tags-hdr",
                        SharedString::from(format!("TAGS ({})", tags.len())),
                        self.tags_collapsed,
                        c.muted,
                        c.fg,
                    )
                    .text_size(px(10.0))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.tags_collapsed = !this.tags_collapsed;
                        cx.notify();
                    })),
                )
                .child(
                    labonair_ui_kit::button_no_hover(
                        "git-tags-new",
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("+"))
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.new_tag_open = true;
                        this.tags_collapsed = false;
                        this.active_field = Some(Field::TagName);
                        w.focus(&this.focus);
                        cx.notify();
                    })),
                ),
        );

        if self.tags_collapsed {
            return wrap.into_any_element();
        }

        if let Some(err) = &self.tag_error {
            wrap = wrap.child(
                div()
                    .px(px(8.0))
                    .py(px(3.0))
                    .bg(c.error.opacity(0.10))
                    .text_size(px(10.0))
                    .text_color(c.error)
                    .child(SharedString::from(err.clone())),
            );
        }

        if self.new_tag_open {
            wrap = wrap.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .m(px(6.0))
                    .p(px(6.0))
                    .rounded_sm()
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .child(self.text_field(
                        "git-tag-name",
                        Field::TagName,
                        "Tag name (required)",
                        c,
                        cx,
                    ))
                    .child(self.text_field(
                        "git-tag-msg",
                        Field::TagMessage,
                        "Message (optional, annotated)",
                        c,
                        cx,
                    ))
                    .child(self.text_field(
                        "git-tag-from",
                        Field::TagFrom,
                        "From (optional, default HEAD)",
                        c,
                        cx,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(self.tool_btn(
                                "git-tag-create",
                                "Create",
                                c,
                                cx,
                                |this, _w, cx| this.create_tag(cx),
                            ))
                            .child(self.tool_btn(
                                "git-tag-cancel",
                                "Cancel",
                                c,
                                cx,
                                |this, _w, cx| {
                                    this.new_tag_open = false;
                                    this.tag_error = None;
                                    this.active_field = None;
                                    cx.notify();
                                },
                            )),
                    ),
            );
        }

        if tags.is_empty() && !self.new_tag_open {
            wrap = wrap.child(picker_empty("No tags", c));
        }

        if let Some(name) = self.delete_confirm_tag.clone() {
            wrap = wrap.child(self.confirm_bar(
                format!("Delete tag '{name}'?"),
                c,
                vec![
                    ("tdel-yes", "Delete", c.error),
                    ("tdel-no", "Cancel", c.muted),
                ],
                cx,
                move |this, id, cx| {
                    if id == "tdel-yes" {
                        this.delete_tag(name.clone(), cx);
                    } else {
                        this.delete_confirm_tag = None;
                        cx.notify();
                    }
                },
            ));
        }

        for tag in &tags {
            let push_tag = tag.clone();
            let del_tag = tag.clone();
            let trailing = h_stack()
                .gap(px(2.0))
                .child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-tag-push-{tag}")),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("\u{2191}"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.push_tag(push_tag.clone(), cx);
                        },
                    )),
                )
                .child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-tag-del-{tag}")),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.error))
                    .child(SharedString::from("\u{2715}"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.delete_confirm_tag = Some(del_tag.clone());
                            cx.notify();
                        },
                    )),
                );
            wrap = wrap.child(
                ListItem::new(
                    SharedString::from(format!("git-tag-{tag}")),
                    c.fg,
                    c.muted,
                    c.border,
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(SharedString::from(tag.clone())),
                )
                .trailing(trailing)
                .extra(|row| row.h(px(20.0)).text_size(px(11.0))),
            );
        }

        wrap.into_any_element()
    }

    fn render_stash_panel(&self, c: Colors, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(state) = &self.state else {
            return div().into_any_element();
        };
        let entries = state.stash.clone();
        let mut wrap = div().flex().flex_col().border_b_1().border_color(c.border);

        wrap = wrap.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .h(px(22.0))
                .px(px(8.0))
                .child(
                    disclosure(
                        "git-stash-hdr",
                        SharedString::from(format!("STASHES ({})", entries.len())),
                        self.stash_collapsed,
                        c.muted,
                        c.fg,
                    )
                    .text_size(px(10.0))
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.stash_collapsed = !this.stash_collapsed;
                        cx.notify();
                    })),
                )
                .child(
                    labonair_ui_kit::button_no_hover(
                        "git-stash-new",
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("+"))
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.stash_form_open = true;
                        this.stash_collapsed = false;
                        this.active_field = Some(Field::StashMsg);
                        w.focus(&this.focus);
                        cx.notify();
                    })),
                ),
        );

        if self.stash_collapsed {
            return wrap.into_any_element();
        }

        if self.stash_form_open {
            wrap = wrap.child(
                div()
                    .flex()
                    .gap(px(4.0))
                    .px(px(8.0))
                    .pb(px(4.0))
                    .child(div().flex_1().child(self.text_field(
                        "git-stash-msg",
                        Field::StashMsg,
                        "Stash message (optional)",
                        c,
                        cx,
                    )))
                    .child(
                        self.tool_btn("git-stash-do", "Stash", c, cx, |this, _w, cx| {
                            this.stash_push(cx)
                        }),
                    )
                    .child(
                        self.tool_btn("git-stash-cancel", "Cancel", c, cx, |this, _w, cx| {
                            this.stash_form_open = false;
                            this.stash_msg.clear();
                            this.active_field = None;
                            cx.notify();
                        }),
                    ),
            );
        }

        if entries.is_empty() {
            return wrap.child(picker_empty("No stashes", c)).into_any_element();
        }

        if let Some((idx, _)) = self.drop_confirm_stash.clone() {
            wrap = wrap.child(self.confirm_bar(
                format!("Drop stash@{{{idx}}}?"),
                c,
                vec![
                    ("sdrop-yes", "Drop", c.error),
                    ("sdrop-no", "Cancel", c.muted),
                ],
                cx,
                move |this, id, cx| {
                    if id == "sdrop-yes" {
                        if let Some((_, hash)) = this.drop_confirm_stash.clone() {
                            this.stash_drop(hash, cx);
                        }
                    } else {
                        this.drop_confirm_stash = None;
                        cx.notify();
                    }
                },
            ));
        }

        for e in &entries {
            let apply_hash = e.hash.clone();
            let pop_hash = e.hash.clone();
            let drop_idx = e.index;
            let drop_hash = e.hash.clone();
            let trailing = h_stack()
                .gap(px(4.0))
                .child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-stash-apply-{}", e.index)),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("A"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.stash_apply(apply_hash.clone(), false, cx);
                        },
                    )),
                )
                .child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-stash-pop-{}", e.index)),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child(SharedString::from("P"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.stash_apply(pop_hash.clone(), true, cx);
                        },
                    )),
                )
                .child(
                    labonair_ui_kit::button_no_hover(
                        SharedString::from(format!("git-stash-drop-{}", e.index)),
                        c.palette,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.error))
                    .child(SharedString::from("\u{2715}"))
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _w, cx| {
                            this.drop_confirm_stash = Some((drop_idx, drop_hash.clone()));
                            cx.notify();
                        },
                    )),
                );
            wrap = wrap.child(
                ListItem::new(
                    SharedString::from(format!("git-stash-{}", e.index)),
                    c.fg,
                    c.muted,
                    c.border,
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(9.0))
                        .text_color(c.muted)
                        .child(SharedString::from(e.index.to_string())),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(SharedString::from(stash_display_message(&e.message))),
                )
                .when(!e.branch.is_empty(), |d| {
                    d.child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(9.0))
                            .text_color(c.muted)
                            .child(SharedString::from(e.branch.clone())),
                    )
                })
                .trailing(trailing)
                .extra(|row| row.h(px(20.0)).text_size(px(11.0))),
            );
        }

        wrap.into_any_element()
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

    /// Adaptive Changes header (Zed-parity Phase 4, §9.5): repo-level tri-state
    /// checkbox, `View Diff` (+ file count), a view-options menu, an adaptive
    /// Stage All / Unstage All action and an overflow menu for the rare /
    /// destructive repo-wide operations.
    fn render_changes_header(
        &self,
        buckets: &Buckets,
        c: Colors,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let total = buckets.conflicted.len()
            + buckets.staged.len()
            + buckets.unstaged.len()
            + buckets.untracked.len();
        let unstaged_total =
            buckets.conflicted.len() + buckets.unstaged.len() + buckets.untracked.len();
        let mut flags: Vec<bool> = vec![true; buckets.staged.len()];
        flags.extend(std::iter::repeat_n(false, unstaged_total));
        let repo_stage = aggregate_stage(&flags);
        let has_unstaged = !buckets.unstaged.is_empty() || !buckets.untracked.is_empty();

        let repo_v = cx.entity();
        let repo_box = git_change_row(
            "git-repo-cb",
            c.palette,
            repo_stage,
            SharedString::default(),
        )
        .on_toggle_stage(move |want: &bool, _w, cx| {
            let want = *want;
            repo_v.update(cx, |this, cx| {
                if want {
                    this.stage_all(cx);
                } else {
                    this.unstage_all(cx);
                }
            });
        });

        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .h(px(28.0))
            .px(px(6.0))
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .flex_none()
                    .w(px(24.0))
                    .overflow_hidden()
                    .child(repo_box),
            )
            .child(
                self.tool_btn("git-view-diff", "View Diff", c, cx, |this, _w, cx| {
                    this.emit_project_diff(None, cx)
                }),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!("{total} files"))),
            )
            .child(div().flex_1())
            .child(
                labonair_ui_kit::button_no_hover(
                    "git-view-options",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .text_color(c.muted)
                .hover(|s| s.text_color(c.fg))
                .child(SharedString::from("\u{22EE}"))
                .on_click(cx.listener(|this, ev: &ClickEvent, _w, cx| {
                    this.panel_menu = Some((PanelMenu::ViewOptions, ev.position()));
                    cx.notify();
                })),
            )
            .child(if has_unstaged {
                self.tool_btn("git-stage-all", "Stage All", c, cx, |this, _w, cx| {
                    this.stage_all(cx)
                })
                .into_any_element()
            } else {
                self.tool_btn("git-unstage-all", "Unstage All", c, cx, |this, _w, cx| {
                    this.unstage_all(cx)
                })
                .into_any_element()
            })
            .child(
                labonair_ui_kit::button_no_hover(
                    "git-overflow",
                    c.palette,
                    ButtonVariant::Ghost,
                    ButtonSize::IconXs,
                )
                .text_color(c.muted)
                .hover(|s| s.text_color(c.fg))
                .child(SharedString::from("\u{25BE}"))
                .on_click(cx.listener(|this, ev: &ClickEvent, _w, cx| {
                    this.panel_menu = Some((PanelMenu::Overflow, ev.position()));
                    cx.notify();
                })),
            )
            .into_any_element()
    }

    fn render_confirm(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let pending = self.pending_confirm?;
        let (text, yes) = match pending {
            PendingConfirm::DiscardAll => (
                "Discard ALL changes in tracked files? This cannot be undone.".to_string(),
                "Discard All",
            ),
            PendingConfirm::Clean => (
                "Delete ALL untracked files? This cannot be undone.".to_string(),
                "Clean",
            ),
            PendingConfirm::ForcePush => (
                "Force-push with lease? This can overwrite the remote branch.".to_string(),
                "Force Push",
            ),
        };
        Some(self.confirm_bar(
            text,
            c,
            vec![
                ("confirm-yes", yes, c.error),
                ("confirm-no", "Cancel", c.muted),
            ],
            cx,
            move |this, id, cx| {
                if id == "confirm-yes" {
                    match pending {
                        PendingConfirm::DiscardAll => this.discard_all(cx),
                        PendingConfirm::Clean => this.clean_untracked(cx),
                        PendingConfirm::ForcePush => this.force_push(cx),
                    }
                }
                this.pending_confirm = None;
                cx.notify();
            },
        ))
    }

    fn render_history(&self, c: Colors, _cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.history.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.0))
                .text_color(c.muted)
                .child(SharedString::from(if self.history_loading {
                    "Loading history\u{2026}"
                } else {
                    "No commits"
                }))
                .into_any_element();
        }
        let commits = self.history.clone();
        let row_h = px(40.0);
        uniform_list(
            "git-history-list",
            commits.len(),
            move |range, _win, _cx| {
                range
                    .map(|i| {
                        let cm = &commits[i];
                        div()
                            .flex()
                            .flex_col()
                            .h(row_h)
                            .justify_center()
                            .px(px(10.0))
                            .border_b_1()
                            .border_color(c.border.opacity(0.5))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.0))
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_size(px(12.0))
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_color(c.info)
                                            .child(SharedString::from(cm.short_hash.clone())),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .text_color(c.fg)
                                            .child(SharedString::from(cm.subject.clone())),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(c.muted)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(format!(
                                        "{}  \u{2022}  +{} \u{2212}{}",
                                        cm.author_name, cm.insertions, cm.deletions
                                    ))),
                            )
                            .into_any_element()
                    })
                    .collect::<Vec<_>>()
            },
        )
        .flex_1()
        .into_any_element()
    }

    fn render_panel_menu(&self, c: Colors, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (which, pos) = self.panel_menu?;
        let view = cx.entity();
        let close = {
            let v = view.clone();
            move |cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.panel_menu = None;
                    cx.notify();
                });
            }
        };
        let act = |f: GitFileAct| {
            let v = view.clone();
            move |_: &ClickEvent, _w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.panel_menu = None;
                    f(this, cx);
                });
            }
        };

        let items: Vec<MenuItem> = match which {
            PanelMenu::ViewOptions => {
                let tree = self.file_tree;
                vec![
                    MenuItem::new("vo-flat", "Flat view")
                        .checked(!tree)
                        .on_click(act(Box::new(|this, cx| {
                            this.file_tree = false;
                            cx.notify();
                        }))),
                    MenuItem::new("vo-tree", "Tree view")
                        .checked(tree)
                        .on_click(act(Box::new(|this, cx| {
                            this.file_tree = true;
                            cx.notify();
                        }))),
                    MenuItem::separator(),
                    MenuItem::new("vo-collapse", "Collapse all").on_click(act(Box::new(
                        |this, cx| {
                            for s in [
                                Section::Conflicts,
                                Section::Staged,
                                Section::Unstaged,
                                Section::Untracked,
                            ] {
                                this.collapsed.insert(s as u8);
                            }
                            cx.notify();
                        },
                    ))),
                    MenuItem::new("vo-expand", "Expand all").on_click(act(Box::new(|this, cx| {
                        this.collapsed.clear();
                        this.dir_collapsed.clear();
                        cx.notify();
                    }))),
                ]
            }
            PanelMenu::Overflow => vec![
                MenuItem::new("of-refresh", "Refresh")
                    .icon(IconName::Refresh)
                    .on_click(act(Box::new(|this, cx| this.refresh_soon(cx)))),
                MenuItem::new("of-stash", "Stashes\u{2026}").on_click(act(Box::new(|this, cx| {
                    this.stash_collapsed = false;
                    cx.notify();
                }))),
                MenuItem::separator(),
                MenuItem::new("of-discard", "Discard All Changes\u{2026}")
                    .icon(IconName::Trash)
                    .destructive()
                    .on_click(act(Box::new(|this, cx| {
                        this.pending_confirm = Some(PendingConfirm::DiscardAll);
                        cx.notify();
                    }))),
                MenuItem::new("of-clean", "Clean Untracked Files\u{2026}")
                    .icon(IconName::Trash)
                    .destructive()
                    .on_click(act(Box::new(|this, cx| {
                        this.pending_confirm = Some(PendingConfirm::Clean);
                        cx.notify();
                    }))),
            ],
            PanelMenu::Repo => {
                let has_upstream = self.current_branch_has_upstream();
                vec![
                    MenuItem::new("rp-fetch", "Fetch")
                        .on_click(act(Box::new(|this, cx| this.fetch(cx)))),
                    MenuItem::new("rp-pull", "Pull")
                        .on_click(act(Box::new(|this, cx| this.pull(cx)))),
                    MenuItem::new("rp-push", if has_upstream { "Push" } else { "Publish" })
                        .on_click(act(Box::new(|this, cx| this.push(cx)))),
                    MenuItem::separator(),
                    MenuItem::new("rp-force", "Force Push\u{2026}")
                        .destructive()
                        .on_click(act(Box::new(|this, cx| {
                            this.pending_confirm = Some(PendingConfirm::ForcePush);
                            cx.notify();
                        }))),
                    MenuItem::separator(),
                    MenuItem::new("rp-branches", "Branches & Tags\u{2026}").on_click(act(
                        Box::new(|this, cx| {
                            this.branch_picker_open = true;
                            cx.notify();
                        }),
                    )),
                    MenuItem::new("rp-stashes", "Stashes\u{2026}").on_click(act(Box::new(
                        |this, cx| {
                            this.stash_collapsed = false;
                            cx.notify();
                        },
                    ))),
                ]
            }
        };

        Some(context_menu(pos, c.palette, move |_w, cx| close(cx), items))
    }
}

impl Render for GitPanelView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span =
            tracing::trace_span!(target: "labonair::perf", "render", view = "scm_panel").entered();
        let c = self.colors(cx);
        self.ensure_commit_input(window, cx);

        let mut root = div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_field_key))
            .flex()
            .flex_col()
            .size_full()
            .bg(c.bg)
            .text_color(c.fg);

        if !self.is_repo {
            return root.child(self.render_no_repo(c, cx));
        }

        // Panel-owned tab bar: Changes | History (two real information modes).
        let mode_key = match self.mode {
            PanelMode::Changes => "changes",
            PanelMode::History => "history",
        };
        root = root.child(
            div()
                .flex()
                .items_center()
                .h(px(28.0))
                .px(px(6.0))
                .border_b_1()
                .border_color(c.border)
                .child(
                    segmented_control("git-tabs", c.palette, mode_key)
                        .variant(SegmentVariant::Solid)
                        .size(SegmentSize::Xs)
                        .segment("changes", "Changes")
                        .segment("history", "History")
                        .on_select(cx.listener(|this, key: &SharedString, _w, cx| {
                            let mode = if key.as_ref() == "history" {
                                PanelMode::History
                            } else {
                                PanelMode::Changes
                            };
                            this.set_mode(mode, cx);
                        })),
                ),
        );

        if self.mode == PanelMode::History {
            return root.child(self.render_history(c, cx));
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

        root = root.child(self.render_changes_header(&buckets, c, cx));

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

        if let Some(bar) = self.render_confirm(c, cx) {
            root = root.child(bar);
        }

        if !self.stash_collapsed {
            root = root.child(self.render_stash_panel(c, cx));
        }

        // Flattened, virtualised change list (§12.5) — tree or flat over one
        // change model.
        let sections: Sections<'_> = [
            (Section::Conflicts, "Conflicts", &buckets.conflicted),
            (Section::Staged, "Staged", &buckets.staged),
            (Section::Unstaged, "Changes", &buckets.unstaged),
            (Section::Untracked, "Untracked", &buckets.untracked),
        ];
        let sel = self.selected.as_ref().map(|s| s.path.as_str());
        let entries = if self.file_tree {
            flatten_git_tree(&sections, &self.collapsed, &self.dir_collapsed, sel)
        } else {
            flatten_git(&sections, &self.collapsed, sel)
        };
        let row_h = c.palette.density_tokens().tree_row_height();
        let view = cx.entity();
        let list = uniform_list("git-file-list", entries.len(), move |range, _win, _cx| {
            let _span = tracing::trace_span!(
                target: "labonair::perf",
                "scm_viewport_build",
                built = range.len(),
                total = entries.len()
            )
            .entered();
            range
                .map(|i| git_list_element(&entries[i], c, row_h, &view))
                .collect::<Vec<_>>()
        })
        .flex_1()
        .py(px(2.0));

        root.child(list)
            .child(self.render_branch_picker(c, cx))
            .child(self.render_footer(c, cx))
            .child(self.render_commit_composer(c, cx))
            .children(self.render_file_menu(cx))
            .children(self.render_panel_menu(c, cx))
    }
}

impl GitPanelView {
    fn add_to_gitignore(&mut self, path: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Add to .gitignore failed",
            async move { git::git_add_to_gitignore(root, path, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn add_to_exclude(&mut self, path: String, cx: &mut Context<Self>) {
        let Some((root, sid, be)) = self.ctx() else {
            return;
        };
        self.run_op(
            "Add to exclude failed",
            async move { git::git_add_to_exclude(root, path, sid, &be.ssh, be.clone()).await },
            cx,
        );
    }

    fn render_file_menu(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let (path, section, pos) = self.file_menu.clone()?;
        let staged = matches!(section, Section::Staged);
        let can_discard = matches!(section, Section::Unstaged | Section::Conflicts);
        let untracked = matches!(section, Section::Untracked);
        let view = cx.entity();
        let act = |f: GitFileAct| {
            let v = view.clone();
            move |_: &ClickEvent, _w: &mut Window, cx: &mut App| {
                v.update(cx, |this, cx| {
                    this.file_menu = None;
                    f(this, cx);
                });
            }
        };

        let mut items: Vec<MenuItem> = Vec::new();
        let p = path.clone();
        if staged {
            items.push(
                MenuItem::new("gitm-unstage", "Unstage").on_click(act(Box::new(
                    move |this, cx| this.unstage_file(p.clone(), cx),
                ))),
            );
        } else {
            items.push(
                MenuItem::new("gitm-stage", "Stage")
                    .icon(IconName::Plus)
                    .on_click(act(Box::new(move |this, cx| {
                        this.stage_file(p.clone(), cx)
                    }))),
            );
        }
        if can_discard {
            let p = path.clone();
            items.push(
                MenuItem::new("gitm-discard", "Discard Changes")
                    .icon(IconName::Trash)
                    .destructive()
                    .on_click(act(Box::new(move |this, cx| {
                        this.discard_file(p.clone(), cx)
                    }))),
            );
        }
        items.push(MenuItem::separator());
        let p = path.clone();
        items.push(
            MenuItem::new("gitm-ignore", "Add to .gitignore").on_click(act(Box::new(
                move |this, cx| this.add_to_gitignore(p.clone(), cx),
            ))),
        );
        let p = path.clone();
        items.push(
            MenuItem::new("gitm-exclude", "Add to .git/info/exclude").on_click(act(Box::new(
                move |this, cx| this.add_to_exclude(p.clone(), cx),
            ))),
        );
        {
            items.push(MenuItem::separator());
            let sel = Selected {
                path: path.clone(),
                staged,
                untracked,
            };
            items.push(
                MenuItem::new("gitm-diff", "Open Diff").on_click(act(Box::new(move |this, cx| {
                    this.select_file(sel.clone(), cx)
                }))),
            );
        }

        let v = view.clone();
        let dismiss = move |_w: &mut Window, cx: &mut App| {
            v.update(cx, |this, cx| {
                this.file_menu = None;
                cx.notify();
            });
        };
        Some(context_menu(
            pos,
            Palette::from_theme(self.theme.read(cx)),
            dismiss,
            items,
        ))
    }
}

fn section_label(text: &'static str, c: Colors) -> impl IntoElement {
    div()
        .h(px(20.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .text_size(px(10.0))
        .text_color(c.muted)
        .child(SharedString::from(text.to_uppercase()))
}

fn picker_empty(text: &'static str, c: Colors) -> impl IntoElement {
    div()
        .px(px(12.0))
        .py(px(4.0))
        .text_size(px(11.0))
        .text_color(c.muted)
        .child(SharedString::from(text))
}

/// The file's base name — used as the primary label; the full path is kept in
/// the row's `secondary` muted text and its tooltip so identity is never
/// reduced to an ambiguous tail (Zed-parity Phase 4, §9.5 "Change rows").
fn short_path(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// The directory portion of a repo-relative path (`""` for a top-level file) —
/// the muted secondary line that keeps the complete path visible in flat view.
fn dir_prefix(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// [`Panel`](labonair_panel::Panel) wiring (T17-001).
///
/// Source Control docks on the **left** at **300 px** — the reference opens the
/// staging/diff panel in the left sidebar next to the file tree; 300 px gives
/// the two-column status list + inline diff room without crowding. It is a
/// vertical list, so only side docks are valid. Dock move/persistence is
/// T17-002; [`set_position`] is a no-op until then.
impl labonair_panel::Panel for GitPanelView {
    fn persistent_name() -> &'static str {
        "source-control"
    }

    fn title(&self, _cx: &App) -> SharedString {
        "Source Control".into()
    }

    fn icon(&self) -> labonair_panel::PanelIcon {
        labonair_panel::PanelIcon::SourceControl
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

    fn default_size(&self, _cx: &App) -> Pixels {
        px(300.0)
    }

    fn min_size(&self) -> Option<Pixels> {
        Some(px(200.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn mk_branch(name: &str, remote: bool) -> Branch {
        Branch {
            name: name.into(),
            is_current: false,
            is_remote: remote,
            upstream: None,
            ahead: 0,
            behind: 0,
            author: None,
            committed_relative: None,
            subject: None,
        }
    }

    #[test]
    fn filter_branches_splits_by_kind_and_matches_case_insensitively() {
        let bs = vec![
            mk_branch("main", false),
            mk_branch("feature/Login", false),
            mk_branch("origin/main", true),
        ];
        let local = filter_branches(&bs, "", false);
        assert_eq!(local.len(), 2);
        let remote = filter_branches(&bs, "", true);
        assert_eq!(remote.len(), 1);
        let filtered = filter_branches(&bs, "login", false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "feature/Login");
        assert!(filter_branches(&bs, "  ", false).len() == 2);
    }

    #[test]
    fn checkout_error_mapping() {
        assert!(
            map_checkout_error("error: Your local changes would be overwritten by checkout")
                .contains("Stash your changes first")
        );
        assert_eq!(
            map_checkout_error("boom"),
            "Could not checkout: boom".to_string()
        );
        assert!(is_unmerged_branch_error(
            "error: The branch 'x' is not fully merged."
        ));
        assert!(!is_unmerged_branch_error("some other error"));
    }

    #[test]
    fn stash_message_and_conflict_helpers() {
        assert_eq!(stash_display_message("   "), "WIP");
        assert_eq!(stash_display_message("  fix bug "), "fix bug");
        assert!(is_stash_conflict_error(
            "CONFLICT (content): Merge conflict in a.txt"
        ));
        assert!(stash_conflict_message("CONFLICT in a.txt").contains("stash was kept"));
        assert_eq!(stash_conflict_message("plain error"), "plain error");
    }

    #[test]
    fn from_ref_unwraps_detached_head_label() {
        assert_eq!(resolve_default_from_ref("main"), "main");
        assert_eq!(
            resolve_default_from_ref("HEAD detached at a1b2c3d"),
            "a1b2c3d"
        );
    }

    #[test]
    fn short_path_is_basename_and_dir_prefix_is_the_rest() {
        assert_eq!(short_path("a.txt"), "a.txt");
        assert_eq!(short_path("src/a.txt"), "a.txt");
        assert_eq!(short_path("a/b/c/d.txt"), "d.txt");
        assert_eq!(dir_prefix("a.txt"), "");
        assert_eq!(dir_prefix("src/a.txt"), "src");
        assert_eq!(dir_prefix("a/b/c/d.txt"), "a/b/c");
    }

    fn fstat(path: &str) -> FileStatus {
        FileStatus {
            path: path.into(),
            original_path: None,
            index_status: 'M',
            worktree_status: '.',
            submodule: None,
            conflicted: false,
        }
    }

    #[test]
    fn aggregate_stage_is_tri_state() {
        assert_eq!(aggregate_stage(&[]), StageState::Unstaged);
        assert_eq!(aggregate_stage(&[false, false]), StageState::Unstaged);
        assert_eq!(aggregate_stage(&[true, true]), StageState::Staged);
        assert_eq!(aggregate_stage(&[true, false]), StageState::PartiallyStaged);
    }

    #[test]
    fn flatten_git_preserves_headers_files_and_aggregate_staging() {
        let staged = vec![fstat("src/a.rs"), fstat("src/b.rs")];
        let unstaged = vec![fstat("c.rs")];
        let empty: Vec<FileStatus> = vec![];
        let sections: [(Section, &'static str, &[FileStatus]); 4] = [
            (Section::Conflicts, "Conflicts", &empty),
            (Section::Staged, "Staged", &staged),
            (Section::Unstaged, "Changes", &unstaged),
            (Section::Untracked, "Untracked", &empty),
        ];
        let mut collapsed = std::collections::HashSet::new();
        let list = flatten_git(&sections, &collapsed, Some("c.rs"));

        // header(Staged) + 2 files + header(Changes) + 1 file
        assert_eq!(list.len(), 5);
        match &list[0] {
            GitListEntry::SectionHeader {
                section,
                count,
                stage,
                ..
            } => {
                assert_eq!(*section, Section::Staged);
                assert_eq!(*count, 2);
                assert_eq!(*stage, StageState::Staged);
            }
            other => panic!("expected staged header, got {other:?}"),
        }
        assert!(matches!(&list[1], GitListEntry::File { staged: true, .. }));
        match &list[3] {
            GitListEntry::SectionHeader { section, stage, .. } => {
                assert_eq!(*section, Section::Unstaged);
                assert_eq!(*stage, StageState::Unstaged);
            }
            other => panic!("expected changes header, got {other:?}"),
        }
        assert!(matches!(
            &list[4],
            GitListEntry::File {
                staged: false,
                selected: true,
                ..
            }
        ));

        // Collapsing the Staged section drops its file rows but keeps its
        // header (and its aggregate staging state).
        collapsed.insert(Section::Staged as u8);
        let collapsed_list = flatten_git(&sections, &collapsed, None);
        assert_eq!(collapsed_list.len(), 3); // staged header + changes header + 1 file
        assert!(matches!(
            &collapsed_list[0],
            GitListEntry::SectionHeader {
                section: Section::Staged,
                collapsed: true,
                stage: StageState::Staged,
                ..
            }
        ));
    }

    #[test]
    fn flatten_git_emits_empty_state_when_clean() {
        let empty: Vec<FileStatus> = vec![];
        let sections: [(Section, &'static str, &[FileStatus]); 1] =
            [(Section::Unstaged, "Changes", &empty)];
        let list = flatten_git(&sections, &std::collections::HashSet::new(), None);
        assert_eq!(list, vec![GitListEntry::EmptyState]);
    }

    fn file_paths(entries: &[GitListEntry]) -> Vec<String> {
        entries
            .iter()
            .filter_map(|e| match e {
                GitListEntry::File { path, .. } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tree_and_flat_flatteners_yield_the_same_file_set() {
        let staged = vec![fstat("src/app/main.rs"), fstat("src/app/util.rs")];
        let unstaged = vec![fstat("README.md"), fstat("src/lib.rs")];
        let empty: Vec<FileStatus> = vec![];
        let sections: [(Section, &'static str, &[FileStatus]); 4] = [
            (Section::Conflicts, "Conflicts", &empty),
            (Section::Staged, "Staged", &staged),
            (Section::Unstaged, "Changes", &unstaged),
            (Section::Untracked, "Untracked", &empty),
        ];
        let collapsed = std::collections::HashSet::new();
        let dcoll = std::collections::HashSet::new();
        let flat = flatten_git(&sections, &collapsed, None);
        let tree = flatten_git_tree(&sections, &collapsed, &dcoll, None);
        let mut a = file_paths(&flat);
        let mut b = file_paths(&tree);
        a.sort();
        b.sort();
        assert_eq!(a, b);

        // The tree emits directory grouping nodes; the flat view never does.
        assert!(flat
            .iter()
            .all(|e| !matches!(e, GitListEntry::Directory { .. })));
        assert!(tree
            .iter()
            .any(|e| matches!(e, GitListEntry::Directory { path, .. } if path == "src/app")));
    }

    #[test]
    fn tree_directory_node_reports_aggregate_tri_state_and_collapse_hides_children() {
        // One staged + one unstaged file under the same directory → the
        // directory node aggregates to PartiallyStaged.
        let staged = vec![fstat("src/a.rs")];
        let unstaged = vec![fstat("src/b.rs")];
        let empty: Vec<FileStatus> = vec![];
        let sections: [(Section, &'static str, &[FileStatus]); 4] = [
            (Section::Conflicts, "Conflicts", &empty),
            (Section::Staged, "Staged", &staged),
            (Section::Unstaged, "Changes", &unstaged),
            (Section::Untracked, "Untracked", &empty),
        ];
        let collapsed = std::collections::HashSet::new();
        let mut dcoll = std::collections::HashSet::new();
        let tree = flatten_git_tree(&sections, &collapsed, &dcoll, None);
        // Each section has its own `src` directory node (per-section staging).
        assert!(tree.iter().any(|e| matches!(
            e,
            GitListEntry::Directory { path, stage: StageState::Staged, .. } if path == "src"
        )));
        assert!(tree.iter().any(|e| matches!(
            e,
            GitListEntry::Directory { path, stage: StageState::Unstaged, .. } if path == "src"
        )));

        // Collapsing `src` hides every `src/*` file row (in every section) while
        // keeping the directory nodes visible.
        dcoll.insert("src".to_string());
        let collapsed_tree = flatten_git_tree(&sections, &collapsed, &dcoll, None);
        let files = file_paths(&collapsed_tree);
        assert!(files.is_empty());
        assert!(collapsed_tree.iter().any(|e| matches!(
            e,
            GitListEntry::Directory { path, collapsed: true, .. } if path == "src"
        )));
    }

    #[test]
    fn commit_mode_derivation_covers_every_case() {
        // staged-only
        assert_eq!(
            derive_commit_mode(false, 3, false),
            CommitMode::CommitStaged
        );
        // tracked-only (nothing staged, tracked files dirty)
        assert_eq!(
            derive_commit_mode(false, 0, true),
            CommitMode::CommitTracked
        );
        // amend flag always wins
        assert_eq!(derive_commit_mode(true, 3, true), CommitMode::Amend);
        assert_eq!(derive_commit_mode(true, 0, false), CommitMode::Amend);
        // clean repo, no amend → still labelled Commit (button is disabled
        // separately via the disabled reason)
        assert_eq!(
            derive_commit_mode(false, 0, false),
            CommitMode::CommitStaged
        );
    }

    #[test]
    fn disabled_reason_derivation_covers_every_case() {
        use DisabledReason::*;
        // in-flight op blocks everything
        assert_eq!(
            derive_disabled_reason(false, 3, false, false, RepoOperation::Pushing, false),
            Some(OperationInProgress)
        );
        // conflicts block a commit
        assert_eq!(
            derive_disabled_reason(false, 3, false, true, RepoOperation::Idle, false),
            Some(Conflict)
        );
        // empty message
        assert_eq!(
            derive_disabled_reason(true, 3, false, false, RepoOperation::Idle, false),
            Some(NoMessage)
        );
        // empty repo — nothing staged, nothing tracked-dirty, not amending
        assert_eq!(
            derive_disabled_reason(false, 0, false, false, RepoOperation::Idle, false),
            Some(NoChanges)
        );
        // staged-only, message present → enabled
        assert_eq!(
            derive_disabled_reason(false, 2, false, false, RepoOperation::Idle, false),
            None
        );
        // tracked-only, message present → enabled
        assert_eq!(
            derive_disabled_reason(false, 0, true, false, RepoOperation::Idle, false),
            None
        );
        // amend with nothing new staged → still enabled (re-commit message)
        assert_eq!(
            derive_disabled_reason(false, 0, false, false, RepoOperation::Idle, true),
            None
        );
    }

    // ── Zed-parity Phase 5: large change-set render/latency evidence ──────────
    //
    // §14 "no row-count-linear render regression" for the Git panel. `flatten_*`
    // is linear in the change set (it *is* the change set); the virtual list's
    // per-frame `git_list_element` work must be viewport-bounded. Live trace:
    //   RUST_LOG=labonair::perf=trace cargo run
    // then open Source Control on a big repo — `scm_flatten_flat` /
    // `scm_flatten_tree` fire once per refresh, `scm_viewport_build` per frame
    // with `built` == visible rows.

    fn many_changes(n: usize) -> Vec<FileStatus> {
        (0..n)
            .map(|i| FileStatus {
                path: format!("src/mod{}/file{i}.rs", i % 128),
                original_path: None,
                index_status: if i % 2 == 0 { 'M' } else { '.' },
                worktree_status: if i % 2 == 0 { '.' } else { 'M' },
                submodule: None,
                conflicted: false,
            })
            .collect()
    }

    #[test]
    fn flatten_git_on_a_large_change_set_is_bounded_and_viewport_render_is_not() {
        const N: usize = 5_000;
        let unstaged = many_changes(N);
        let empty: Vec<FileStatus> = vec![];
        let sections: [(Section, &'static str, &[FileStatus]); 4] = [
            (Section::Conflicts, "Conflicts", &empty),
            (Section::Staged, "Staged", &empty),
            (Section::Unstaged, "Changes", &unstaged),
            (Section::Untracked, "Untracked", &empty),
        ];
        let collapsed = std::collections::HashSet::new();

        let flat = flatten_git(&sections, &collapsed, None);
        // one section header + N file rows, nothing quadratic.
        assert_eq!(flat.len(), N + 1);

        let tree = flatten_git_tree(
            &sections,
            &collapsed,
            &std::collections::HashSet::new(),
            None,
        );
        // tree adds grouping nodes but every file still appears exactly once.
        assert_eq!(file_paths(&flat).len(), N);
        assert_eq!(file_paths(&tree).len(), N);

        // The work `uniform_list("git-file-list", …)` does per frame: map a
        // viewport `Range` -> elements. Touched rows track the viewport only.
        const VIEWPORT: usize = 40;
        for start in [0usize, 2_500, flat.len() - VIEWPORT] {
            let mut touched = 0usize;
            let _built: Vec<&GitListEntry> = (start..start + VIEWPORT)
                .map(|i| {
                    touched += 1;
                    &flat[i]
                })
                .collect();
            assert_eq!(touched, VIEWPORT);
        }
    }
}
