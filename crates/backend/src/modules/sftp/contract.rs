//! Backend adapters for the UI-free SFTP capability contracts.

use super::connection;
use crate::modules::ssh::sftp as remote;
use crate::App;
use labonair_errors::LabonairError;
use labonair_sftp::{
    BoxFuture, RemoteEntry, SftpBrowserService, SftpSessionHandle, SftpSessionService,
};
use labonair_ssh::SshSessionId;

#[derive(Clone)]
pub struct BackendSftpService {
    app: App,
}

impl BackendSftpService {
    pub fn new(app: App) -> Self {
        Self { app }
    }
}

impl SftpSessionService for BackendSftpService {
    fn open<'a>(
        &'a self,
        ssh_session: SshSessionId,
    ) -> BoxFuture<'a, Result<SftpSessionHandle, LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            connection::sftp_open_session(ssh_session.clone().into(), &state, app).await?;
            Ok(SftpSessionHandle::from_ssh_session(ssh_session))
        })
    }

    fn close<'a>(&'a self, session: SftpSessionHandle) -> BoxFuture<'a, Result<(), String>> {
        let state = self.app.ssh.clone();
        Box::pin(async move {
            connection::sftp_disconnect(session.as_str().to_string(), &state)
                .map_err(|error| error.to_string())
        })
    }
}

impl SftpBrowserService for BackendSftpService {
    fn read_dir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> BoxFuture<'a, Result<Vec<RemoteEntry>, LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_read_dir(session.as_str().to_string(), path, &state, app)
                .await
                .map(|entries| {
                    entries
                        .into_iter()
                        .map(|entry| RemoteEntry {
                            name: entry.name,
                            path: entry.path,
                            size: entry.size,
                            modified_at: entry.modified_at,
                            is_dir: entry.is_dir,
                            is_symlink: entry.is_symlink,
                            symlink_target: entry.symlink_target,
                            permissions: entry.permissions,
                        })
                        .collect()
                })
        })
    }

    fn rename<'a>(
        &'a self,
        session: SftpSessionHandle,
        old_path: String,
        new_path: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_rename(
                session.as_str().to_string(),
                old_path,
                new_path,
                &state,
                app,
            )
            .await
        })
    }

    fn delete<'a>(
        &'a self,
        session: SftpSessionHandle,
        paths: Vec<String>,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_delete(session.as_str().to_string(), paths, &state, app).await
        })
    }

    fn mkdir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        recursive: bool,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_mkdir(
                session.as_str().to_string(),
                path,
                Some(recursive),
                &state,
                app,
            )
            .await
        })
    }

    fn create_file<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_create_file(session.as_str().to_string(), path, &state, app).await
        })
    }

    fn chmod<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        mode: u32,
    ) -> BoxFuture<'a, Result<(), LabonairError>> {
        let app = self.app.clone();
        Box::pin(async move {
            let state = app.ssh.clone();
            remote::sftp_chmod(session.as_str().to_string(), path, mode, &state, app).await
        })
    }
}
