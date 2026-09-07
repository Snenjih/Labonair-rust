//! Backend adapter for the UI-free Git graph contract.

use labonair_git::{GitFuture, GitGraphService, GitService};

use crate::modules::ssh::SshState;
use labonair_events::EventBus;

use super::CommitInfo;

/// Binds the existing Git executor and SSH session registry to the narrow
/// contract consumed by the commit graph view.
pub struct BackendGitGraphService {
    ssh_state: SshState,
    events: EventBus,
}

/// Full source-control adapter. The UI receives this capability at the
/// composition root and never needs the backend facade or SSH executor.
pub struct BackendGitService {
    ssh_state: SshState,
    events: EventBus,
}

impl BackendGitService {
    pub fn new(ssh_state: SshState, events: EventBus) -> Self {
        Self { ssh_state, events }
    }
}

impl BackendGitGraphService {
    pub fn new(ssh_state: SshState, events: EventBus) -> Self {
        Self { ssh_state, events }
    }
}

impl GitGraphService for BackendGitGraphService {
    fn is_repo(&self, path: String, session_id: Option<String>) -> GitFuture<bool> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_is_repo(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn repo_root(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_repo_root(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn current_branch(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_current_branch(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn log(
        &self,
        path: String,
        limit: Option<u32>,
        all_branches: bool,
        session_id: Option<String>,
        skip: Option<usize>,
    ) -> GitFuture<Vec<CommitInfo>> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_log(
                path,
                limit,
                all_branches,
                session_id,
                skip,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn commit_numstat(
        &self,
        path: String,
        hash: String,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_commit_numstat(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn commit_diff(
        &self,
        path: String,
        hash: String,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_commit_diff(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn checkout(&self, path: String, branch: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_checkout_branch(path, branch, session_id, &ssh_state, events.clone()).await
        })
    }

    fn cherry_pick(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_cherry_pick(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn create_branch(
        &self,
        path: String,
        name: String,
        from_ref: Option<String>,
        checkout: bool,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_create_branch(
                path,
                name,
                from_ref,
                checkout,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }
}

impl GitService for BackendGitService {
    fn is_repo(&self, path: String, session_id: Option<String>) -> GitFuture<bool> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_is_repo(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn repo_root(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_repo_root(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn workspace_state(
        &self,
        path: String,
        session_id: Option<String>,
    ) -> GitFuture<labonair_git::WorkspaceGitState> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_workspace_state(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn log(
        &self,
        path: String,
        limit: Option<u32>,
        all_branches: bool,
        session_id: Option<String>,
        skip: Option<usize>,
    ) -> GitFuture<Vec<labonair_git::CommitInfo>> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_log(
                path,
                limit,
                all_branches,
                session_id,
                skip,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn diff(
        &self,
        path: String,
        file: String,
        staged: bool,
        ignore_whitespace: Option<bool>,
        is_untracked: Option<bool>,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_get_diff(
                path,
                file,
                staged,
                ignore_whitespace,
                is_untracked,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn stage_file(&self, path: String, file: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stage_file(path, file, session_id, &ssh_state, events.clone()).await
        })
    }

    fn unstage_file(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_unstage_file(path, file, session_id, &ssh_state, events.clone()).await
        })
    }

    fn stage_hunk(
        &self,
        path: String,
        file: String,
        patch: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stage_hunk(path, file, patch, session_id, &ssh_state, events.clone()).await
        })
    }

    fn unstage_hunk(
        &self,
        path: String,
        file: String,
        patch: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_unstage_hunk(path, file, patch, session_id, &ssh_state, events.clone()).await
        })
    }

    fn discard_file(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_discard_file(path, file, session_id, &ssh_state, events.clone()).await
        })
    }

    fn stage_all(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_stage_all(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn unstage_all(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_unstage_all(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn discard_all(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_discard_all(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn clean_untracked(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_clean_untracked(path, session_id, &ssh_state, events.clone()).await
        })
    }

    fn commit(
        &self,
        path: String,
        message: String,
        amend: bool,
        session_id: Option<String>,
    ) -> GitFuture<labonair_git::CommitResult> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_commit(path, message, amend, session_id, &ssh_state, events.clone()).await
        })
    }

    fn push(
        &self,
        path: String,
        remote: Option<String>,
        branch: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_push(path, remote, branch, session_id, &ssh_state, events.clone()).await
        })
    }

    fn pull(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move { super::git_pull(path, session_id, &ssh_state, events.clone()).await })
    }

    fn fetch(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_fetch(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn abort(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_abort(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn continue_operation(&self, path: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(
            async move { super::git_continue(path, session_id, &ssh_state, events.clone()).await },
        )
    }

    fn checkout_branch(
        &self,
        path: String,
        branch: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_checkout_branch(path, branch, session_id, &ssh_state, events.clone()).await
        })
    }

    fn create_branch(
        &self,
        path: String,
        name: String,
        from_ref: Option<String>,
        checkout: bool,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_create_branch(
                path,
                name,
                from_ref,
                checkout,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn delete_branch(
        &self,
        path: String,
        name: String,
        force: bool,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_delete_branch(path, name, force, session_id, &ssh_state, events.clone())
                .await
        })
    }

    fn rename_branch(
        &self,
        path: String,
        old_name: String,
        new_name: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_rename_branch(
                path,
                old_name,
                new_name,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn create_tag(
        &self,
        path: String,
        name: String,
        message: Option<String>,
        from_ref: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_create_tag(
                path,
                name,
                message,
                from_ref,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn delete_tag(&self, path: String, name: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_delete_tag(path, name, session_id, &ssh_state, events.clone()).await
        })
    }

    fn push_tag(
        &self,
        path: String,
        name: String,
        remote: Option<String>,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_push_tag(path, name, remote, session_id, &ssh_state, events.clone()).await
        })
    }

    fn stash_push(
        &self,
        path: String,
        message: Option<String>,
        include_untracked: Option<bool>,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stash_push(
                path,
                message,
                include_untracked,
                session_id,
                &ssh_state,
                events.clone(),
            )
            .await
        })
    }

    fn stash_pop(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stash_pop(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn stash_apply(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stash_apply(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn stash_drop(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_stash_drop(path, hash, session_id, &ssh_state, events.clone()).await
        })
    }

    fn add_to_gitignore(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_add_to_gitignore(path, file, session_id, &ssh_state, events.clone()).await
        })
    }

    fn add_to_exclude(
        &self,
        path: String,
        file: String,
        session_id: Option<String>,
    ) -> GitFuture<()> {
        let ssh_state = self.ssh_state.clone();
        let events = self.events.clone();
        Box::pin(async move {
            super::git_add_to_exclude(path, file, session_id, &ssh_state, events.clone()).await
        })
    }
}
