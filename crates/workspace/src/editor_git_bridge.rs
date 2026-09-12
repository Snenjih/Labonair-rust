//! Workspace-owned live adapter for the editor's Git projection.
//!
//! The adapter is deliberately small: `labonair-git` remains the only owner
//! of repository/index state and mutations, while the editor receives only a
//! revision-bound projection. Review requests are queued as typed intents and
//! drained by [`crate::Workspace`] so opening Project Diff remains on its
//! canonical workspace path.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use labonair_editor::{
    GitDecorationProvider, GitDecorationRequest, GitGutterError, GitGutterSnapshot, GitHunkAction,
    GitProviderFuture, HunkActionSink, Revision,
};
use labonair_git::{FileStatus, GitService, WorkspaceGitState};

/// Context captured by one refresh or mutation. It is replaced when the
/// workspace changes its explicit project identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorGitContext {
    pub root: Option<String>,
    pub session_id: Option<String>,
}

/// A review intent waiting for Workspace to invoke the canonical Project Diff
/// request. Stage and unstage are executed directly by this adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedEditorGitAction {
    pub action: GitHunkAction,
    pub context: EditorGitContext,
}

struct BridgeState {
    context: EditorGitContext,
    /// Latest request per document identity. Actions created from an older
    /// projection are rejected before reaching GitService.
    latest_revisions: HashMap<(Option<String>, String), Revision>,
    latest_contexts: HashMap<(String, Revision), EditorGitContext>,
    review_actions: Vec<QueuedEditorGitAction>,
}

/// Typed Workspace adapter implementing both editor-side Git boundaries.
pub struct WorkspaceGitBridge {
    git: Arc<dyn GitService>,
    state: Mutex<BridgeState>,
}

impl WorkspaceGitBridge {
    pub fn new(git: Arc<dyn GitService>) -> Self {
        Self {
            git,
            state: Mutex::new(BridgeState {
                context: EditorGitContext::default(),
                latest_revisions: HashMap::new(),
                latest_contexts: HashMap::new(),
                review_actions: Vec::new(),
            }),
        }
    }

    pub fn set_context(&self, context: EditorGitContext) {
        if let Ok(mut state) = self.state.lock() {
            if state.context != context {
                state.context = context;
                state.latest_revisions.clear();
                state.latest_contexts.clear();
                state.review_actions.clear();
            }
        }
    }

    pub fn context(&self) -> EditorGitContext {
        self.state
            .lock()
            .map(|state| state.context.clone())
            .unwrap_or_default()
    }

    /// Drain only review intents. The returned actions are consumed by
    /// Workspace and never rendered as a second diff surface.
    pub fn take_review_actions(&self) -> Vec<QueuedEditorGitAction> {
        self.state
            .lock()
            .map(|mut state| std::mem::take(&mut state.review_actions))
            .unwrap_or_default()
    }

    fn context_and_record(
        &self,
        request: &GitDecorationRequest,
    ) -> Result<EditorGitContext, GitGutterError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| GitGutterError::Provider("Git bridge state was poisoned".into()))?;
        let context = state.context.clone();
        state.latest_revisions.insert(
            (context.session_id.clone(), request.path.clone()),
            request.revision,
        );
        state
            .latest_contexts
            .insert((request.path.clone(), request.revision), context.clone());
        Ok(context)
    }

    fn context_for_target(
        &self,
        target_path: &str,
        revision: Revision,
    ) -> Result<EditorGitContext, GitGutterError> {
        let state = self
            .state
            .lock()
            .map_err(|_| GitGutterError::Provider("Git bridge state was poisoned".into()))?;
        state
            .latest_contexts
            .get(&(target_path.to_string(), revision))
            .cloned()
            .ok_or(GitGutterError::StaleRevision {
                expected: Revision::default(),
                actual: revision,
            })
    }

    fn target_is_current(
        &self,
        target_path: &str,
        revision: Revision,
        context: &EditorGitContext,
    ) -> Result<(), GitGutterError> {
        let state = self
            .state
            .lock()
            .map_err(|_| GitGutterError::Provider("Git bridge state was poisoned".into()))?;
        let current = state
            .latest_revisions
            .get(&(context.session_id.clone(), target_path.to_string()))
            .copied();
        if current == Some(revision) && state.context == *context {
            Ok(())
        } else {
            Err(GitGutterError::StaleRevision {
                expected: current.unwrap_or_default(),
                actual: revision,
            })
        }
    }

    fn queue_review(
        &self,
        action: GitHunkAction,
        context: EditorGitContext,
    ) -> Result<(), GitGutterError> {
        let target = match &action {
            GitHunkAction::OpenProjectDiff { target } | GitHunkAction::ShowChange { target } => {
                target
            }
            GitHunkAction::Discard { preview } => &preview.target,
            GitHunkAction::Stage { target, .. } | GitHunkAction::Unstage { target, .. } => target,
        };
        self.target_is_current(&target.path, target.revision, &context)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| GitGutterError::Provider("Git bridge state was poisoned".into()))?;
        state
            .review_actions
            .push(QueuedEditorGitAction { action, context });
        Ok(())
    }
}

impl GitDecorationProvider for WorkspaceGitBridge {
    fn refresh(&self, request: GitDecorationRequest) -> GitProviderFuture<GitGutterSnapshot> {
        let context = match self.context_and_record(&request) {
            Ok(context) => context,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let Some(root) = context.root.clone() else {
            return Box::pin(async move {
                Ok(GitGutterSnapshot::no_repository(
                    request.path,
                    request.revision,
                ))
            });
        };
        let git = self.git.clone();
        Box::pin(async move {
            let is_repo = git
                .is_repo(root.clone(), context.session_id.clone())
                .await
                .map_err(GitGutterError::Provider)?;
            if !is_repo {
                return Ok(GitGutterSnapshot::no_repository(
                    request.path,
                    request.revision,
                ));
            }
            let repo_root = git
                .repo_root(root, context.session_id.clone())
                .await
                .map_err(GitGutterError::Provider)?;
            let state = git
                .workspace_state(repo_root.clone(), context.session_id.clone())
                .await
                .map_err(GitGutterError::Provider)?;
            let file = relative_path(&repo_root, &request.path);
            let (staged, untracked) = diff_mode(&state, &file);
            let diff = git
                .diff(
                    repo_root,
                    file,
                    staged,
                    Some(false),
                    Some(untracked),
                    context.session_id,
                )
                .await
                .map_err(GitGutterError::Provider)?;
            if diff.is_empty() {
                return Ok(
                    GitGutterSnapshot::empty(request.path, request.revision).with_staged(staged)
                );
            }
            GitGutterSnapshot::from_unified_diff(request.path, request.revision, &diff)
                .map(|snapshot| snapshot.with_staged(staged))
                .map_err(|error| match error {
                    GitGutterError::EmptyDiff => {
                        GitGutterError::Provider("Git returned an empty diff".into())
                    }
                    other => other,
                })
        })
    }
}

impl HunkActionSink for WorkspaceGitBridge {
    fn submit(&self, action: GitHunkAction) -> GitProviderFuture<()> {
        let target = match &action {
            GitHunkAction::Stage { target, .. }
            | GitHunkAction::Unstage { target, .. }
            | GitHunkAction::OpenProjectDiff { target }
            | GitHunkAction::ShowChange { target } => target,
            GitHunkAction::Discard { preview } => &preview.target,
        };
        let context = match self.context_for_target(&target.path, target.revision) {
            Ok(context) => context,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        if let Err(error) = self.target_is_current(&target.path, target.revision, &context) {
            return Box::pin(async move { Err(error) });
        }

        match action {
            GitHunkAction::OpenProjectDiff { .. } | GitHunkAction::ShowChange { .. } => {
                let result = self.queue_review(action, context);
                Box::pin(async move { result })
            }
            GitHunkAction::Discard { .. } => Box::pin(async {
                Err(GitGutterError::Provider(
                    "Discarding an individual hunk is not supported by the Git capability; review the whole-file operation in Project Diff.".into(),
                ))
            }),
            GitHunkAction::Stage { target, patch } => {
                self.mutate_hunk(context, target.path, patch, true)
            }
            GitHunkAction::Unstage { target, patch } => {
                self.mutate_hunk(context, target.path, patch, false)
            }
        }
    }
}

impl WorkspaceGitBridge {
    fn mutate_hunk(
        &self,
        context: EditorGitContext,
        path: String,
        patch: String,
        stage: bool,
    ) -> GitProviderFuture<()> {
        let git = self.git.clone();
        Box::pin(async move {
            let Some(root) = context.root else {
                return Err(GitGutterError::Provider(
                    "Git actions require an explicit repository workspace".into(),
                ));
            };
            let repo_root = git
                .repo_root(root, context.session_id.clone())
                .await
                .map_err(GitGutterError::Provider)?;
            let file = relative_path(&repo_root, &path);
            let result = if stage {
                git.stage_hunk(repo_root, file, patch, context.session_id)
                    .await
            } else {
                git.unstage_hunk(repo_root, file, patch, context.session_id)
                    .await
            };
            result.map_err(GitGutterError::Provider)
        })
    }
}

pub(crate) fn relative_path(root: &str, path: &str) -> String {
    Path::new(path)
        .strip_prefix(root)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

fn same_path(left: &str, right: &str) -> bool {
    let left = left.strip_prefix("./").unwrap_or(left);
    let right = right.strip_prefix("./").unwrap_or(right);
    left == right || Path::new(left).file_name() == Path::new(right).file_name()
}

fn contains_path(files: &[FileStatus], path: &str) -> bool {
    files.iter().any(|file| same_path(&file.path, path))
}

/// Select the comparison side for the current editor document. Unstaged
/// changes win when both sides exist, matching the normal editor view; staged
/// changes are shown only when the worktree side is clean.
fn diff_mode(state: &WorkspaceGitState, path: &str) -> (bool, bool) {
    let untracked = contains_path(&state.status.untracked, path);
    if untracked {
        return (false, true);
    }
    let has_worktree = contains_path(&state.status.unstaged, path);
    let has_staged = contains_path(&state.status.staged, path);
    (!has_worktree && has_staged, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_repo_scoped() {
        assert_eq!(relative_path("/repo", "/repo/src/lib.rs"), "src/lib.rs");
        assert_eq!(relative_path("/repo", "src/lib.rs"), "src/lib.rs");
    }

    #[test]
    fn unstaged_changes_win_over_staged_changes() {
        let status = labonair_git::GitStatus {
            staged: vec![status("src/lib.rs", 'M', '.')],
            unstaged: vec![status("src/lib.rs", '.', 'M')],
            untracked: Vec::new(),
            has_conflicts: false,
            merge_in_progress: false,
            rebase_in_progress: false,
            cherry_pick_in_progress: false,
            ahead: 0,
            behind: 0,
        };
        let state = WorkspaceGitState {
            status,
            branches: Vec::new(),
            current_branch: String::new(),
            stash: Vec::new(),
            tags: Vec::new(),
            diff_stats: Vec::new(),
            submodules: Vec::new(),
        };
        assert_eq!(diff_mode(&state, "src/lib.rs"), (false, false));
    }

    fn status(path: &str, index_status: char, worktree_status: char) -> FileStatus {
        FileStatus {
            path: path.to_string(),
            original_path: None,
            index_status,
            worktree_status,
            submodule: None,
            conflicted: false,
        }
    }
}
