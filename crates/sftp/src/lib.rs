//! UI-free SFTP capability contracts and remote filesystem value types.
//!
//! SFTP never authenticates a host or reads application storage. The SSH
//! capability creates an authenticated session first; this crate only opens
//! an SFTP subsystem on that session and exposes remote filesystem behavior.

use labonair_errors::LabonairError;
use labonair_ssh::SshSessionId;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SftpSessionHandle(String);

impl SftpSessionHandle {
    pub fn from_ssh_session(session_id: SshSessionId) -> Self {
        Self(session_id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: i64,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<String>,
    pub permissions: String,
}

pub trait SftpSessionService: Send + Sync {
    fn open<'a>(
        &'a self,
        ssh_session: SshSessionId,
    ) -> BoxFuture<'a, Result<SftpSessionHandle, LabonairError>>;

    fn close<'a>(&'a self, session: SftpSessionHandle) -> BoxFuture<'a, Result<(), String>>;
}

pub trait SftpBrowserService: Send + Sync {
    fn read_dir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> BoxFuture<'a, Result<Vec<RemoteEntry>, LabonairError>>;

    fn rename<'a>(
        &'a self,
        session: SftpSessionHandle,
        old_path: String,
        new_path: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;

    fn delete<'a>(
        &'a self,
        session: SftpSessionHandle,
        paths: Vec<String>,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;

    fn mkdir<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        recursive: bool,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;

    fn create_file<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;

    fn chmod<'a>(
        &'a self,
        session: SftpSessionHandle,
        path: String,
        mode: u32,
    ) -> BoxFuture<'a, Result<(), LabonairError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_handles_are_opaque_and_stable() {
        let handle = SftpSessionHandle::from_ssh_session(SshSessionId::new("tab-1"));
        assert_eq!(handle.as_str(), "tab-1");
    }

    #[test]
    fn remote_entry_is_transport_free() {
        let entry = RemoteEntry {
            name: "README.md".into(),
            path: "/README.md".into(),
            size: 10,
            modified_at: 1,
            is_dir: false,
            is_symlink: false,
            symlink_target: None,
            permissions: "rw-r--r--".into(),
        };
        assert_eq!(entry.path, "/README.md");
    }
}
