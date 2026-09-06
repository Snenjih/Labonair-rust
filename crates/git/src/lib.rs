//! UI-free Git capability contracts.
//!
//! The crate deliberately contains no Git implementation. Local and remote
//! execution is supplied by an adapter at the composition root, while views
//! depend only on these stable value types and operations.

use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

/// A commit record used by graph and history views.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub parent_hashes: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub subject: String,
    pub refs: Vec<String>,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleState {
    pub commit_changed: bool,
    pub modified: bool,
    pub untracked: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    pub path: String,
    pub original_path: Option<String>,
    pub index_status: char,
    pub worktree_status: char,
    pub submodule: Option<SubmoduleState>,
    pub conflicted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SubmoduleSyncState {
    Uninitialized,
    PointerChanged,
    Conflict,
    Clean,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubmoduleStatus {
    pub path: String,
    pub commit: String,
    pub state: SubmoduleSyncState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<FileStatus>,
    pub has_conflicts: bool,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
    pub cherry_pick_in_progress: bool,
    pub ahead: u32,
    pub behind: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub author: Option<String>,
    pub committed_relative: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitResult {
    pub hash: String,
    pub subject: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StashEntry {
    pub index: u32,
    pub message: String,
    pub branch: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffStat {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub staged: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitState {
    pub status: GitStatus,
    pub branches: Vec<Branch>,
    pub current_branch: String,
    pub stash: Vec<StashEntry>,
    pub tags: Vec<String>,
    pub diff_stats: Vec<FileDiffStat>,
    pub submodules: Vec<SubmoduleStatus>,
}

/// Full source-control capability used by SCM and project-diff surfaces.
/// Implementations own local/remote execution; consumers do not know about
/// SSH sessions, subprocesses, or the application backend.
pub trait GitService: Send + Sync {
    fn is_repo(&self, path: String, session_id: Option<String>) -> GitFuture<bool>;
    fn repo_root(&self, path: String, session_id: Option<String>) -> GitFuture<String>;
    fn workspace_state(
        &self,
        path: String,
        session_id: Option<String>,
    ) -> GitFuture<WorkspaceGitState>;
    fn log(
        &self,
        path: String,
        limit: Option<u32>,
        all_branches: bool,
        session_id: Option<String>,
        skip: Option<usize>,
    ) -> GitFuture<Vec<CommitInfo>>;
    fn diff(
        &self,
        path: String,
        file: String,
        staged: bool,
        ignore_whitespace: Option<bool>,
        is_untracked: Option<bool>,
        session_id: Option<String>,
    ) -> GitFuture<String>;
    fn stage_file(&self, path: String, file: String, session_id: Option<String>) -> GitFuture<()>;
    fn unstage_file(&self, path: String, file: String, session_id: Option<String>)
        -> GitFuture<()>;
    fn stage_hunk(
        &self,
        path: String,
        file: String,
        patch: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn unstage_hunk(
        &self,
        path: String,
        file: String,
        patch: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn discard_file(&self, path: String, file: String, session_id: Option<String>)
        -> GitFuture<()>;
    fn stage_all(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn unstage_all(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn discard_all(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn clean_untracked(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn commit(
        &self,
        path: String,
        message: String,
        amend: bool,
        session_id: Option<String>,
    ) -> GitFuture<CommitResult>;
    fn push(
        &self,
        path: String,
        remote: Option<String>,
        branch: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<String>;
    fn pull(&self, path: String, session_id: Option<String>) -> GitFuture<String>;
    fn fetch(&self, path: String, session_id: Option<String>) -> GitFuture<String>;
    fn abort(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn continue_operation(&self, path: String, session_id: Option<String>) -> GitFuture<()>;
    fn checkout_branch(
        &self,
        path: String,
        branch: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn create_branch(
        &self,
        path: String,
        name: String,
        from_ref: Option<String>,
        checkout: bool,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn delete_branch(
        &self,
        path: String,
        name: String,
        force: bool,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn rename_branch(
        &self,
        path: String,
        old_name: String,
        new_name: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn create_tag(
        &self,
        path: String,
        name: String,
        message: Option<String>,
        from_ref: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn delete_tag(&self, path: String, name: String, session_id: Option<String>) -> GitFuture<()>;
    fn push_tag(
        &self,
        path: String,
        name: String,
        remote: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<String>;
    fn stash_push(
        &self,
        path: String,
        message: Option<String>,
        include_untracked: Option<bool>,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn stash_pop(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()>;
    fn stash_apply(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()>;
    fn stash_drop(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()>;
    fn add_to_gitignore(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
    fn add_to_exclude(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()>;
}

/// Boxed asynchronous Git operation returned by a capability adapter.
pub type GitFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;

/// Minimal Git contract required by the commit graph view.
pub trait GitGraphService: Send + Sync {
    fn is_repo(&self, path: String, session_id: Option<String>) -> GitFuture<bool>;
    fn repo_root(&self, path: String, session_id: Option<String>) -> GitFuture<String>;
    fn current_branch(&self, path: String, session_id: Option<String>) -> GitFuture<String>;
    fn log(
        &self,
        path: String,
        limit: Option<u32>,
        all_branches: bool,
        session_id: Option<String>,
        skip: Option<usize>,
    ) -> GitFuture<Vec<CommitInfo>>;
    fn commit_numstat(
        &self,
        path: String,
        hash: String,
        session_id: Option<String>,
    ) -> GitFuture<String>;
    fn commit_diff(
        &self,
        path: String,
        hash: String,
        session_id: Option<String>,
    ) -> GitFuture<String>;
    fn checkout(&self, path: String, branch: String, session_id: Option<String>) -> GitFuture<()>;
    fn cherry_pick(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()>;
    fn create_branch(
        &self,
        path: String,
        name: String,
        from_ref: Option<String>,
        checkout: bool,
        session_id: Option<String>,
    ) -> GitFuture<()>;
}
