pub mod connection;
pub mod contract;
pub(crate) mod net_error;
pub mod worker;

// `state.rs` (the old dedicated `SftpState`/`SftpSession`/`SftpSessionInner`
// types, backed by the previous synchronous SSH library) is deleted per the
// russh migration's session-model decision: SFTP sessions are now stored
// per-`session_id` in the unified `crate::modules::ssh::SshState` registry
// (see `connection.rs`, `ssh/sftp.rs`, `worker.rs`, `git/executor.rs`).
