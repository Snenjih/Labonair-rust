pub mod connection;
pub mod contract;

// `state.rs` (the old dedicated `SftpState`/`SftpSession`/`SftpSessionInner`
// types, backed by the previous synchronous SSH library) is deleted per the
// russh migration's session-model decision: SFTP sessions are now stored
// per-`session_id` in the unified SSH transport session registry
// (see `connection.rs`, `ssh/sftp.rs`, `worker.rs`, `git/executor.rs`).
