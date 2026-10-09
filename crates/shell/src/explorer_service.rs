//! Composition adapter for Explorer's remote-directory contract.
//!
//! The Explorer panel only needs a transport-neutral directory listing. The
//! shell owns the concrete SSH/SFTP services and adapts them at startup so the
//! panel does not depend directly on either remote capability.

use std::sync::Arc;

use labonair_explorer_host::{RemoteExplorerEntry, RemoteExplorerFuture, RemoteExplorerService};
use labonair_sftp::{SftpBrowserService, SftpSessionService};
use labonair_ssh::SshSessionId;

pub(crate) struct SftpExplorerService {
    sessions: Arc<dyn SftpSessionService>,
    browser: Arc<dyn SftpBrowserService>,
}

impl SftpExplorerService {
    pub(crate) fn new(
        sessions: Arc<dyn SftpSessionService>,
        browser: Arc<dyn SftpBrowserService>,
    ) -> Self {
        Self { sessions, browser }
    }
}

impl RemoteExplorerService for SftpExplorerService {
    fn read_dir(
        &self,
        session_id: String,
        path: String,
    ) -> RemoteExplorerFuture<Vec<RemoteExplorerEntry>> {
        let sessions = self.sessions.clone();
        let browser = self.browser.clone();

        Box::pin(async move {
            let handle = sessions
                .open(SshSessionId::new(session_id))
                .await
                .map_err(|error| error.to_string())?;
            let entries = browser
                .read_dir(handle, path)
                .await
                .map_err(|error| error.to_string())?;

            Ok(entries
                .into_iter()
                .map(|entry| RemoteExplorerEntry {
                    name: entry.name,
                    path: entry.path,
                    size: entry.size,
                    modified_at: entry.modified_at,
                    is_dir: entry.is_dir,
                    is_symlink: entry.is_symlink,
                    symlink_target: entry.symlink_target,
                    permissions: entry.permissions,
                })
                .collect())
        })
    }
}
