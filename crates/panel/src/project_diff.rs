//! Neutral Project-Diff request contract (Zed-parity redesign Phase 4,
//! `docs/ui-comparison-zed-sidebar-status-bar.md` §9.5 / §12.6).
//!
//! The Source-Control panel (`labonair-panel-scm`) no longer owns an inline
//! diff viewer. Instead it emits a [`ProjectDiffRequest`] describing *what* to
//! review; the workspace opens or focuses exactly one Project Diff item and
//! renders the code + hunk actions. This type lives in the contracts crate so
//! neither `labonair-workspace` nor `labonair-panel-scm` has to depend on the
//! other's concrete view types — the request flows
//! `panel-scm → shell → workspace`.

/// How the Project Diff lays a file's hunks out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectDiffMode {
    /// One column, `+`/`-` prefixed.
    Unified,
    /// Two columns: old on the left, new on the right.
    Split,
}

/// One changed file in a [`ProjectDiffRequest`], carrying the identity the diff
/// item needs to fetch the right `git diff` for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiffFile {
    /// Repo-relative path (b-side for renames).
    pub path: String,
    /// `true` → diff index↔HEAD (a staged change); `false` → worktree↔index.
    pub staged: bool,
    /// `true` → the file is untracked (`git diff --no-index /dev/null <file>`).
    pub untracked: bool,
}

/// A request to open / focus the single workspace Project Diff item.
///
/// Repeated requests for the same repository are **idempotent**: the workspace
/// focuses the existing item and re-points its selection, never opening a
/// duplicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiffRequest {
    /// Resolved repository root (`git rev-parse --show-toplevel`).
    pub repo_root: String,
    /// Remote SSH session id, or `None` for a local repo.
    pub session_id: Option<String>,
    /// Ordered changed files to review.
    pub files: Vec<ProjectDiffFile>,
    /// Which file to focus initially / on a follow-up selection request. Must be
    /// one of `files`' paths; ignored otherwise.
    pub selected: Option<String>,
    /// Initial layout.
    pub mode: ProjectDiffMode,
}
