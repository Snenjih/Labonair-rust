//! Backend adapter for the UI-free Git graph contract.

use labonair_git::{GitFuture, GitGraphService};

use super::CommitInfo;

/// Binds the existing Git executor and SSH session registry to the narrow
/// contract consumed by the commit graph view.
pub struct BackendGitGraphService {
    app: crate::App,
}

impl BackendGitGraphService {
    pub fn new(app: crate::App) -> Self {
        Self { app }
    }
}

impl GitGraphService for BackendGitGraphService {
    fn is_repo(&self, path: String, session_id: Option<String>) -> GitFuture<bool> {
        let app = self.app.clone();
        Box::pin(async move { super::git_is_repo(path, session_id, &app.ssh, app.clone()).await })
    }

    fn repo_root(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let app = self.app.clone();
        Box::pin(
            async move { super::git_get_repo_root(path, session_id, &app.ssh, app.clone()).await },
        )
    }

    fn current_branch(&self, path: String, session_id: Option<String>) -> GitFuture<String> {
        let app = self.app.clone();
        Box::pin(async move {
            super::git_get_current_branch(path, session_id, &app.ssh, app.clone()).await
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
        let app = self.app.clone();
        Box::pin(async move {
            super::git_get_log(
                path,
                limit,
                all_branches,
                session_id,
                skip,
                &app.ssh,
                app.clone(),
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
        let app = self.app.clone();
        Box::pin(async move {
            super::git_get_commit_numstat(path, hash, session_id, &app.ssh, app.clone()).await
        })
    }

    fn commit_diff(
        &self,
        path: String,
        hash: String,
        session_id: Option<String>,
    ) -> GitFuture<String> {
        let app = self.app.clone();
        Box::pin(async move {
            super::git_get_commit_diff(path, hash, session_id, &app.ssh, app.clone()).await
        })
    }

    fn checkout(&self, path: String, branch: String, session_id: Option<String>) -> GitFuture<()> {
        let app = self.app.clone();
        Box::pin(async move {
            super::git_checkout_branch(path, branch, session_id, &app.ssh, app.clone()).await
        })
    }

    fn cherry_pick(&self, path: String, hash: String, session_id: Option<String>) -> GitFuture<()> {
        let app = self.app.clone();
        Box::pin(async move {
            super::git_cherry_pick(path, hash, session_id, &app.ssh, app.clone()).await
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
        let app = self.app.clone();
        Box::pin(async move {
            super::git_create_branch(
                path,
                name,
                from_ref,
                checkout,
                session_id,
                &app.ssh,
                app.clone(),
            )
            .await
        })
    }
}
