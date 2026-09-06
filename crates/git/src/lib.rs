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
